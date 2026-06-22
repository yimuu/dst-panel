//! Scheduled task compatibility handlers for `/api/task`.
//!
//! This slice migrates the HTTP contract and persistence of Go's timed-task
//! API. The background runtime lives in `domain::scheduler::runtime`; this
//! handler keeps the list/delete payload shape Go-compatible and exposes a
//! stable `jobId` backed by the persisted row id.

use axum::{
    Json,
    body::Bytes,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    domain::scheduler::model::{JobTaskRecord, SaveJobTask},
    domain::scheduler::repository::tasks::JobTaskRepository,
    dst::DstConfig,
    web::app::AppState,
    web::error::{AppError, AppResult},
    web::handlers::{legacy_empty_success, legacy_success, repository_error},
    web::response::LoginResponse,
};

const ALLOWED_TASK_CATEGORIES: &[&str] = &[
    "backup",
    "update",
    "start",
    "stop",
    "restart",
    "regenerate",
    "startGame",
    "stopGame",
    "none",
];

/// Query parameters accepted by `DELETE /api/task`.
#[derive(Debug, Deserialize)]
pub(crate) struct DeleteTaskQuery {
    #[serde(rename = "jobId", default)]
    job_id: Option<String>,
}

/// Go-compatible scheduled task list item.
#[derive(Debug, Serialize)]
pub(crate) struct TaskListItem {
    /// DST cluster name.
    #[serde(rename = "clusterName")]
    cluster_name: String,
    /// Human-facing level name.
    #[serde(rename = "levelName")]
    level_name: String,
    /// Level folder/uuid passed to the strategy.
    uuid: String,
    /// Exposed runtime job id. Currently mapped to the persisted row id.
    #[serde(rename = "jobId")]
    job_id: i64,
    /// Next scheduled execution time when a runtime scheduler is active.
    next: Option<DateTime<Utc>>,
    /// Previous execution time when a runtime scheduler is active.
    prev: Option<DateTime<Utc>>,
    /// Whether the task entry is valid.
    valid: bool,
    /// Cron expression.
    cron: String,
    /// UI comment.
    comment: String,
    /// Task category.
    category: String,
    /// Announcement sent before task execution.
    announcement: String,
}

impl From<JobTaskRecord> for TaskListItem {
    fn from(record: JobTaskRecord) -> Self {
        Self {
            cluster_name: record.cluster_name,
            level_name: record.level_name,
            uuid: record.uuid,
            job_id: record.id,
            next: None,
            prev: None,
            valid: true,
            cron: record.cron,
            comment: record.comment,
            category: record.category,
            announcement: record.announcement,
        }
    }
}

/// Lists active scheduled tasks with Go's response shape.
pub(crate) async fn list_handler(
    State(state): State<AppState>,
) -> AppResult<Json<LoginResponse<Vec<TaskListItem>>>> {
    let repository = JobTaskRepository::new(state.db);
    let records = repository
        .list_active()
        .await
        .map_err(|error| repository_error("list scheduled tasks", error))?;
    let tasks = records
        .into_iter()
        .map(TaskListItem::from)
        .collect::<Vec<_>>();
    tracing::debug!(count = tasks.len(), "listed scheduled tasks");
    Ok(Json(legacy_success(tasks)))
}

/// Creates a persisted scheduled task after Go-compatible cron validation.
pub(crate) async fn create_handler(
    State(state): State<AppState>,
    body: Bytes,
) -> AppResult<Response> {
    let mut request = match serde_json::from_slice::<SaveJobTask>(&body) {
        Ok(request) => request,
        Err(error) => {
            tracing::warn!(error = %error, "rejected malformed scheduled task body");
            return Ok(bad_task_request(format!("请求参数错误: {error}")));
        }
    };

    if request.cluster_name.is_empty() {
        let config = DstConfig::load(&state.root_path).map_err(file_error("load dst_config"))?;
        request.cluster_name = config.cluster;
    }
    if request.cron.trim().is_empty() {
        tracing::warn!("rejected scheduled task with empty cron expression");
        return Ok(bad_task_request("cron 表达式不能为空"));
    }
    if let Err(error) = validate_standard_cron(&request.cron) {
        tracing::warn!(error = %error, "rejected malformed scheduled task body");
        return Ok(bad_task_request(format!("cron 表达式格式错误: {error}")));
    }
    if !ALLOWED_TASK_CATEGORIES.contains(&request.category.as_str()) {
        tracing::warn!(
            category = %request.category,
            "rejected scheduled task with unsupported category"
        );
        return Ok(bad_task_request(format!(
            "请求参数错误: unsupported category {}",
            request.category
        )));
    }

    let repository = JobTaskRepository::new(state.db);
    let created = repository
        .create(request)
        .await
        .map_err(|error| repository_error("create scheduled task", error))?;
    tracing::info!(
        id = created.id,
        cluster_name = %created.cluster_name,
        category = %created.category,
        "created scheduled task"
    );
    Ok(Json(legacy_empty_success()).into_response())
}

/// Deletes a scheduled task by the `jobId` query parameter.
pub(crate) async fn delete_handler(
    State(state): State<AppState>,
    Query(query): Query<DeleteTaskQuery>,
) -> AppResult<Json<LoginResponse<Value>>> {
    let job_id = query
        .job_id
        .as_deref()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or_default();

    let repository = JobTaskRepository::new(state.db);
    repository
        .delete_by_job_id(job_id)
        .await
        .map_err(|error| repository_error("delete scheduled task", error))?;
    Ok(Json(legacy_empty_success()))
}

/// Returns the static instruction list hard-coded by Go's scheduler.
pub(crate) async fn instruct_handler() -> Json<LoginResponse<Value>> {
    Json(legacy_success(
        json!([{"backup": "备份"}, {"update": "更新"}]),
    ))
}

fn validate_standard_cron(expression: &str) -> Result<(), String> {
    let expression = strip_timezone_prefix(expression.trim())?;
    if is_supported_descriptor(expression) {
        return Ok(());
    }

    let parts = expression.split_whitespace().collect::<Vec<_>>();
    if parts.len() != 5 {
        return Err(format!("expected exactly 5 fields, found {}", parts.len()));
    }

    validate_cron_field("minute", parts[0], 0, 59, &[], false)?;
    validate_cron_field("hour", parts[1], 0, 23, &[], false)?;
    validate_cron_field("day-of-month", parts[2], 1, 31, &[], true)?;
    validate_cron_field("month", parts[3], 1, 12, MONTH_NAMES, false)?;
    validate_cron_field("day-of-week", parts[4], 0, 6, DOW_NAMES, true)?;
    Ok(())
}

fn is_supported_descriptor(expression: &str) -> bool {
    matches!(
        expression,
        "@yearly" | "@annually" | "@monthly" | "@weekly" | "@daily" | "@midnight" | "@hourly"
    ) || expression
        .strip_prefix("@every ")
        .is_some_and(|duration| parse_go_duration(duration.trim()).is_ok())
}

fn validate_cron_field(
    name: &str,
    field: &str,
    min: u32,
    max: u32,
    names: &[(&str, u32)],
    allow_question: bool,
) -> Result<(), String> {
    if field.is_empty() {
        return Err(format!("{name} field is empty"));
    }
    for item in field.split(',') {
        validate_cron_item(name, item, min, max, names, allow_question)?;
    }
    Ok(())
}

fn validate_cron_item(
    name: &str,
    item: &str,
    min: u32,
    max: u32,
    names: &[(&str, u32)],
    allow_question: bool,
) -> Result<(), String> {
    if item.is_empty() {
        return Err(format!("{name} field contains an empty list item"));
    }

    let mut split = item.split('/');
    let base = split.next().unwrap_or_default();
    let step = split.next();
    if split.next().is_some() {
        return Err(format!("{name} field contains too many step separators"));
    }
    if let Some(step) = step {
        let step = step
            .parse::<u32>()
            .map_err(|_| format!("{name} step is not numeric"))?;
        if step == 0 {
            return Err(format!("{name} step must be greater than zero"));
        }
    }

    validate_cron_base(name, base, min, max, names, allow_question)
}

fn validate_cron_base(
    name: &str,
    base: &str,
    min: u32,
    max: u32,
    names: &[(&str, u32)],
    allow_question: bool,
) -> Result<(), String> {
    if base == "*" || (allow_question && base == "?") {
        return Ok(());
    }
    if base == "?" {
        return Err(format!("{name} field does not allow ?"));
    }

    let mut range = base.split('-');
    let start = range.next().unwrap_or_default();
    let maybe_end = range.next();
    if range.next().is_some() {
        return Err(format!("{name} field contains too many range separators"));
    }

    let start = parse_cron_value(name, start, min, max, names)?;
    if let Some(end) = maybe_end {
        let end = parse_cron_value(name, end, min, max, names)?;
        if start > end {
            return Err(format!("{name} range start is greater than end"));
        }
    }

    Ok(())
}

fn parse_cron_value(
    name: &str,
    value: &str,
    min: u32,
    max: u32,
    names: &[(&str, u32)],
) -> Result<u32, String> {
    if value.is_empty() {
        return Err(format!("{name} value is empty"));
    }
    if let Ok(number) = value.parse::<u32>() {
        if (min..=max).contains(&number) {
            return Ok(number);
        }
        return Err(format!("{name} value {number} is outside {min}-{max}"));
    }

    let lowercase = value.to_ascii_lowercase();
    names
        .iter()
        .find_map(|(label, number)| (*label == lowercase).then_some(*number))
        .ok_or_else(|| format!("{name} value {value} is not recognized"))
}

const MONTH_NAMES: &[(&str, u32)] = &[
    ("jan", 1),
    ("feb", 2),
    ("mar", 3),
    ("apr", 4),
    ("may", 5),
    ("jun", 6),
    ("jul", 7),
    ("aug", 8),
    ("sep", 9),
    ("oct", 10),
    ("nov", 11),
    ("dec", 12),
];

const DOW_NAMES: &[(&str, u32)] = &[
    ("sun", 0),
    ("mon", 1),
    ("tue", 2),
    ("wed", 3),
    ("thu", 4),
    ("fri", 5),
    ("sat", 6),
];

fn strip_timezone_prefix(expression: &str) -> Result<&str, String> {
    for prefix in ["TZ=", "CRON_TZ="] {
        if let Some(rest) = expression.strip_prefix(prefix) {
            let Some((zone, cron)) = rest.split_once(char::is_whitespace) else {
                return Err(format!("{prefix} prefix must be followed by a schedule"));
            };
            if !is_supported_timezone_name(zone) {
                return Err(format!("{prefix} timezone is not recognized"));
            }
            let cron = cron.trim();
            if cron.is_empty() {
                return Err(format!("{prefix} prefix must be followed by a schedule"));
            }
            return Ok(cron);
        }
    }
    Ok(expression)
}

fn is_supported_timezone_name(zone: &str) -> bool {
    if matches!(zone, "UTC" | "GMT" | "Local" | "Etc/UTC") {
        return true;
    }
    zone.contains('/')
        && zone
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'_' | b'-' | b'+'))
}

fn parse_go_duration(duration: &str) -> Result<(), String> {
    if duration.is_empty() {
        return Err("duration is empty".to_owned());
    }

    let mut rest = duration;
    while !rest.is_empty() {
        let number_len = duration_number_prefix_len(rest);
        if number_len == 0 {
            return Err("duration segment is missing a number".to_owned());
        }
        rest = &rest[number_len..];
        let unit_len = duration_unit_prefix_len(rest)
            .ok_or_else(|| "duration segment has an unsupported unit".to_owned())?;
        rest = &rest[unit_len..];
    }
    Ok(())
}

fn duration_number_prefix_len(value: &str) -> usize {
    let mut seen_digit = false;
    let mut seen_dot = false;
    let mut last_index = 0;
    for (index, character) in value.char_indices() {
        if character.is_ascii_digit() {
            seen_digit = true;
            last_index = index + character.len_utf8();
        } else if character == '.' && !seen_dot {
            seen_dot = true;
            last_index = index + character.len_utf8();
        } else {
            break;
        }
    }
    if seen_digit { last_index } else { 0 }
}

fn duration_unit_prefix_len(value: &str) -> Option<usize> {
    ["ns", "us", "µs", "μs", "ms", "s", "m", "h"]
        .into_iter()
        .find_map(|unit| value.starts_with(unit).then_some(unit.len()))
}

fn bad_task_request(message: impl Into<String>) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(LoginResponse::<Value>::error(400, message)),
    )
        .into_response()
}

fn file_error(
    operation: &'static str,
) -> impl FnOnce(std::io::Error) -> AppError + Copy + Send + Sync + 'static {
    move |error| {
        if error.kind() == std::io::ErrorKind::InvalidInput {
            AppError::bad_request(error.to_string())
        } else {
            tracing::error!(operation, error = %error, "scheduled task file operation failed");
            AppError::internal(operation)
        }
    }
}
