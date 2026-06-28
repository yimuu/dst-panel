//! Background scheduler runtime for persisted `job_tasks`.
//!
//! Go starts a `robfig/cron` runtime during router construction, loads active
//! rows from `job_tasks`, and executes a category strategy whenever cron fires.
//! This module keeps the same operational contract by polling the table,
//! matching each cron expression for the current schedule-local minute,
//! deduplicating each task/minute bucket, and delegating work to the
//! already-migrated Rust game, backup, and console services.

use std::{collections::HashSet, fmt, path::PathBuf, sync::Arc, time::Duration};

use chrono::{DateTime, Datelike, Local, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use tokio::{task::JoinHandle, time::MissedTickBehavior};

use crate::{
    domain::{
        backup,
        game::{console, lifecycle},
        scheduler::{model::JobTaskRecord, repository::tasks::JobTaskRepository},
    },
    dst::DstConfig,
    infra::{command::CommandRunner, db::SqlitePool, process::ProcessSnapshotProvider},
    validation::validate_safe_command_arg,
    web::error::{AppError, AppResult},
};

const SCHEDULER_TICK: Duration = Duration::from_secs(1);

/// Shared dependencies needed by scheduled task strategies.
#[derive(Clone)]
pub struct SchedulerRuntimeContext {
    root_path: PathBuf,
    db: SqlitePool,
    command_runner: Arc<dyn CommandRunner>,
    process_snapshot_provider: Arc<dyn ProcessSnapshotProvider>,
    lifecycle_grace_period: Duration,
}

impl SchedulerRuntimeContext {
    /// Creates a scheduler context from the same dependencies used by HTTP handlers.
    pub fn new(
        root_path: PathBuf,
        db: SqlitePool,
        command_runner: Arc<dyn CommandRunner>,
        process_snapshot_provider: Arc<dyn ProcessSnapshotProvider>,
        lifecycle_grace_period: Duration,
    ) -> Self {
        Self {
            root_path,
            db,
            command_runner,
            process_snapshot_provider,
            lifecycle_grace_period,
        }
    }
}

/// In-memory state used to avoid running the same task twice in one due bucket.
#[derive(Debug, Default)]
pub struct RuntimeState {
    executed_buckets: HashSet<(i64, String)>,
}

/// Starts the background scheduler loop and returns the detached task handle.
pub fn spawn_scheduler_runtime(context: SchedulerRuntimeContext) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut runtime_state = RuntimeState::default();
        let mut ticker = tokio::time::interval(SCHEDULER_TICK);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        tracing::info!("started scheduled task runtime");
        loop {
            ticker.tick().await;
            if let Err(error) = run_due_tasks_once(&context, Utc::now(), &mut runtime_state).await {
                tracing::warn!(error = %error, "scheduled task runtime tick failed");
            }
        }
    })
}

/// Executes all tasks due at `now` once and returns the number executed.
///
/// Tests call this function directly with a fixed timestamp. Production calls
/// it from [`spawn_scheduler_runtime`], where `RuntimeState` persists across
/// ticks and prevents duplicate executions while the same minute still matches.
pub async fn run_due_tasks_once(
    context: &SchedulerRuntimeContext,
    now: DateTime<Utc>,
    runtime_state: &mut RuntimeState,
) -> AppResult<usize> {
    let repository = JobTaskRepository::new(context.db.clone());
    let tasks = repository
        .list_active()
        .await
        .map_err(|error| repository_error("list scheduled tasks", error))?;
    let mut executed = 0;

    for task in tasks {
        let Some(bucket) = due_bucket(&task.cron, now) else {
            continue;
        };
        let dedupe_key = (task.id, bucket);
        if !runtime_state.executed_buckets.insert(dedupe_key) {
            tracing::debug!(
                task_id = task.id,
                cron = %task.cron,
                "skipped scheduled task already executed for due bucket"
            );
            continue;
        }

        match execute_task(context, &task).await {
            Ok(()) => {
                executed += 1;
                tracing::info!(
                    task_id = task.id,
                    category = %task.category,
                    cluster_name = %task.cluster_name,
                    "executed scheduled task"
                );
            }
            Err(error) => {
                tracing::warn!(
                    task_id = task.id,
                    category = %task.category,
                    cluster_name = %task.cluster_name,
                    error = %error,
                    "scheduled task execution failed"
                );
            }
        }
    }

    Ok(executed)
}

async fn execute_task(context: &SchedulerRuntimeContext, task: &JobTaskRecord) -> AppResult<()> {
    send_announcements(context, task).await?;

    match task.category.as_str() {
        "backup" => create_backup(context, task).await,
        "update" => update_game(context, task).await,
        "start" => start_level(context, task).await,
        "stop" => stop_level(context, task).await,
        "restart" => {
            stop_level(context, task).await?;
            start_level(context, task).await
        }
        "regenerate" => regenerate_level(context, task).await,
        "startGame" => start_game(context, task).await,
        "stopGame" => stop_game(context, task).await,
        "none" | "" => Ok(()),
        other => {
            tracing::warn!(
                task_id = task.id,
                category = other,
                "scheduled task category has no strategy"
            );
            Ok(())
        }
    }
}

async fn send_announcements(
    context: &SchedulerRuntimeContext,
    task: &JobTaskRecord,
) -> AppResult<()> {
    if task.announcement.is_empty() || task.times <= 0 {
        return Ok(());
    }

    for repeat in 0..task.times {
        for line in task.announcement.lines() {
            if line.is_empty() {
                continue;
            }
            console::broadcast(context.command_runner.as_ref(), &task.cluster_name, line).await?;
        }
        if repeat + 1 < task.times && task.sleep > 0 {
            tokio::time::sleep(Duration::from_secs(task.sleep as u64)).await;
        }
    }
    Ok(())
}

async fn create_backup(context: &SchedulerRuntimeContext, task: &JobTaskRecord) -> AppResult<()> {
    let root = context.root_path.clone();
    let cluster_name = task.cluster_name.clone();
    tokio::task::spawn_blocking(move || {
        backup::create_cluster_backup(&root, &cluster_name, None).map(|_| ())
    })
    .await
    .map_err(|error| {
        tracing::error!(error = %error, "scheduled backup worker failed");
        AppError::internal("scheduled backup")
    })?
}

async fn update_game(context: &SchedulerRuntimeContext, task: &JobTaskRecord) -> AppResult<()> {
    let lifecycle = lifecycle_context(context, task)?;
    lifecycle::update_game(
        context.command_runner.as_ref(),
        context.process_snapshot_provider.as_ref(),
        &context.root_path,
        &lifecycle,
        context.lifecycle_grace_period,
    )
    .await
}

async fn start_level(context: &SchedulerRuntimeContext, task: &JobTaskRecord) -> AppResult<()> {
    let lifecycle = lifecycle_context(context, task)?;
    lifecycle::start_level(
        context.command_runner.as_ref(),
        context.process_snapshot_provider.as_ref(),
        &context.root_path,
        &lifecycle,
        &task_level(task),
        context.lifecycle_grace_period,
    )
    .await
}

async fn stop_level(context: &SchedulerRuntimeContext, task: &JobTaskRecord) -> AppResult<()> {
    let lifecycle = lifecycle_context(context, task)?;
    lifecycle::stop_level(
        context.command_runner.as_ref(),
        context.process_snapshot_provider.as_ref(),
        &lifecycle,
        &task_level(task),
        context.lifecycle_grace_period,
    )
    .await
}

async fn regenerate_level(
    context: &SchedulerRuntimeContext,
    task: &JobTaskRecord,
) -> AppResult<()> {
    console::send_level_command(
        context.command_runner.as_ref(),
        &task.cluster_name,
        &task_level(task),
        "c_regenerateworld()",
    )
    .await
}

async fn start_game(context: &SchedulerRuntimeContext, task: &JobTaskRecord) -> AppResult<()> {
    let lifecycle = lifecycle_context(context, task)?;
    lifecycle::start_all(
        context.command_runner.as_ref(),
        context.process_snapshot_provider.as_ref(),
        &context.root_path,
        &lifecycle,
        context.lifecycle_grace_period,
    )
    .await
}

async fn stop_game(context: &SchedulerRuntimeContext, task: &JobTaskRecord) -> AppResult<()> {
    let lifecycle = lifecycle_context(context, task)?;
    lifecycle::stop_all(
        context.command_runner.as_ref(),
        context.process_snapshot_provider.as_ref(),
        &context.root_path,
        &lifecycle,
        context.lifecycle_grace_period,
    )
    .await
}

fn lifecycle_context(
    context: &SchedulerRuntimeContext,
    task: &JobTaskRecord,
) -> AppResult<lifecycle::LifecycleContext> {
    let config = DstConfig::load(&context.root_path).map_err(file_error("load dst_config"))?;
    let cluster_name = if task.cluster_name.trim().is_empty() {
        config.cluster.clone()
    } else {
        task.cluster_name.clone()
    };
    let cluster_name = validate_safe_command_arg("cluster name", &cluster_name)
        .map_err(|error| AppError::bad_request(error.to_string()))?
        .into_string();
    Ok(lifecycle::LifecycleContext {
        config,
        cluster_name,
    })
}

fn task_level(task: &JobTaskRecord) -> String {
    if !task.uuid.trim().is_empty() {
        task.uuid.clone()
    } else if !task.level_name.trim().is_empty() {
        task.level_name.clone()
    } else {
        "Master".to_owned()
    }
}

fn due_bucket(expression: &str, now: DateTime<Utc>) -> Option<String> {
    let parsed = parse_cron_expression(expression.trim())?;
    let expression = parsed.expression;
    if let Some(interval) = every_interval_seconds(expression) {
        let bucket = now.timestamp().div_euclid(interval as i64);
        return Some(format!("@every:{interval}:{bucket}"));
    }
    let moment = parsed.location.moment(now);
    if descriptor_matches(expression, &moment) || standard_cron_matches(expression, &moment) {
        return Some(moment.bucket);
    }
    None
}

struct ParsedCronExpression<'a> {
    expression: &'a str,
    location: CronLocation,
}

#[derive(Clone, Copy)]
enum CronLocation {
    Local,
    Tz(Tz),
}

impl CronLocation {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "Local" => Some(Self::Local),
            "UTC" | "GMT" | "Etc/UTC" => Some(Self::Tz(chrono_tz::UTC)),
            other => other.parse::<Tz>().ok().map(Self::Tz),
        }
    }

    fn moment(self, now: DateTime<Utc>) -> CronMoment {
        match self {
            Self::Local => CronMoment::from_datetime(now.with_timezone(&Local), "Local"),
            Self::Tz(timezone) => {
                let label = timezone.name();
                CronMoment::from_datetime(now.with_timezone(&timezone), label)
            }
        }
    }
}

struct CronMoment {
    minute: u32,
    hour: u32,
    day: u32,
    month: u32,
    day_of_week: u32,
    bucket: String,
}

impl CronMoment {
    fn from_datetime<TzImpl>(datetime: DateTime<TzImpl>, location: &str) -> Self
    where
        TzImpl: TimeZone,
        TzImpl::Offset: fmt::Display,
    {
        Self {
            minute: datetime.minute(),
            hour: datetime.hour(),
            day: datetime.day(),
            month: datetime.month(),
            day_of_week: datetime.weekday().num_days_from_sunday(),
            bucket: format!("{location}:{}", datetime.format("%Y-%m-%dT%H:%M")),
        }
    }
}

fn parse_cron_expression(expression: &str) -> Option<ParsedCronExpression<'_>> {
    for prefix in ["CRON_TZ=", "TZ="] {
        if let Some(rest) = expression.strip_prefix(prefix) {
            let (zone, cron) = rest.split_once(char::is_whitespace)?;
            let cron = cron.trim();
            if cron.is_empty() {
                return None;
            }
            return Some(ParsedCronExpression {
                expression: cron,
                location: CronLocation::parse(zone)?,
            });
        }
    }
    Some(ParsedCronExpression {
        expression,
        location: CronLocation::Local,
    })
}

fn descriptor_matches(expression: &str, moment: &CronMoment) -> bool {
    match expression {
        "@yearly" | "@annually" => {
            moment.month == 1 && moment.day == 1 && moment.hour == 0 && moment.minute == 0
        }
        "@monthly" => moment.day == 1 && moment.hour == 0 && moment.minute == 0,
        "@weekly" => moment.day_of_week == 0 && moment.hour == 0 && moment.minute == 0,
        "@daily" | "@midnight" => moment.hour == 0 && moment.minute == 0,
        "@hourly" => moment.minute == 0,
        _ => false,
    }
}

fn every_interval_seconds(expression: &str) -> Option<u64> {
    let duration = expression.strip_prefix("@every ")?.trim();
    parse_go_duration_seconds(duration)
}

fn parse_go_duration_seconds(mut value: &str) -> Option<u64> {
    let mut total = 0.0_f64;
    while !value.is_empty() {
        let number_len = duration_number_prefix_len(value);
        if number_len == 0 {
            return None;
        }
        let amount = value[..number_len].parse::<f64>().ok()?;
        if !amount.is_finite() || amount < 0.0 {
            return None;
        }
        value = &value[number_len..];

        let (unit_seconds, rest) = if let Some(rest) = value.strip_prefix("ns") {
            (0.000_000_001, rest)
        } else if let Some(rest) = value.strip_prefix("us") {
            (0.000_001, rest)
        } else if let Some(rest) = value.strip_prefix("\u{00B5}s") {
            (0.000_001, rest)
        } else if let Some(rest) = value.strip_prefix("\u{03BC}s") {
            (0.000_001, rest)
        } else if let Some(rest) = value.strip_prefix("ms") {
            (0.001, rest)
        } else if let Some(rest) = value.strip_prefix('s') {
            (1.0, rest)
        } else if let Some(rest) = value.strip_prefix('m') {
            (60.0, rest)
        } else if let Some(rest) = value.strip_prefix('h') {
            (60.0 * 60.0, rest)
        } else {
            return None;
        };
        total += amount * unit_seconds;
        value = rest;
    }
    if total <= 0.0 {
        None
    } else if total < 1.0 {
        Some(1)
    } else {
        Some(total.floor() as u64)
    }
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

fn standard_cron_matches(expression: &str, moment: &CronMoment) -> bool {
    let parts = expression.split_whitespace().collect::<Vec<_>>();
    if parts.len() != 5 {
        return false;
    }

    let day_of_month_matches = field_matches(parts[2], moment.day, 1, 31, &[]);
    let day_of_week_matches = field_matches(parts[4], moment.day_of_week, 0, 6, DOW_NAMES);
    let day_matches = if field_has_star_bit(parts[2]) || field_has_star_bit(parts[4]) {
        day_of_month_matches && day_of_week_matches
    } else {
        day_of_month_matches || day_of_week_matches
    };

    field_matches(parts[0], moment.minute, 0, 59, &[])
        && field_matches(parts[1], moment.hour, 0, 23, &[])
        && field_matches(parts[3], moment.month, 1, 12, MONTH_NAMES)
        && day_matches
}

fn field_matches(field: &str, value: u32, min: u32, max: u32, names: &[(&str, u32)]) -> bool {
    field
        .split(',')
        .any(|item| cron_item_matches(item.trim(), value, min, max, names))
}

fn cron_item_matches(item: &str, value: u32, min: u32, max: u32, names: &[(&str, u32)]) -> bool {
    if item.is_empty() {
        return false;
    }
    if item == "*" || item == "?" {
        return true;
    }
    let (base, step) = match item.split_once('/') {
        Some((base, step)) => match step.parse::<u32>() {
            Ok(step) if step > 0 => (base, Some(step)),
            _ => return false,
        },
        None => (item, None),
    };

    let (start, end) = if base == "*" || base == "?" {
        (min, max)
    } else if let Some((start, end)) = base.split_once('-') {
        let Some(start) = parse_cron_value(start, names) else {
            return false;
        };
        let Some(end) = parse_cron_value(end, names) else {
            return false;
        };
        if start > end || start < min || end > max {
            return false;
        }
        (start, end)
    } else {
        let Some(start) = parse_cron_value(base, names) else {
            return false;
        };
        if start < min || start > max {
            return false;
        }
        let end = if step.is_some() { max } else { start };
        (start, end)
    };

    if value < start || value > end {
        return false;
    }
    match step {
        Some(step) => (value - start).is_multiple_of(step),
        None => true,
    }
}

fn field_has_star_bit(field: &str) -> bool {
    field.split(',').any(|item| {
        let item = item.trim();
        let (base, step) = match item.split_once('/') {
            Some((base, step)) => (base, Some(step)),
            None => (item, None),
        };
        if base != "*" && base != "?" {
            return false;
        }
        step.is_none_or(|step| step.parse::<u32>().is_ok_and(|step| step <= 1))
    })
}

fn parse_cron_value(value: &str, names: &[(&str, u32)]) -> Option<u32> {
    if let Ok(parsed) = value.parse::<u32>() {
        return Some(parsed);
    }
    names
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(value))
        .map(|(_, number)| *number)
}

fn repository_error(operation: &'static str, error: sqlx::Error) -> AppError {
    tracing::error!(operation, error = %error, "scheduled task repository operation failed");
    AppError::internal(operation)
}

fn file_error(operation: &'static str) -> impl FnOnce(std::io::Error) -> AppError {
    move |error| {
        tracing::error!(operation, error = %error, "scheduled task file operation failed");
        AppError::internal(operation)
    }
}

const MONTH_NAMES: &[(&str, u32)] = &[
    ("JAN", 1),
    ("FEB", 2),
    ("MAR", 3),
    ("APR", 4),
    ("MAY", 5),
    ("JUN", 6),
    ("JUL", 7),
    ("AUG", 8),
    ("SEP", 9),
    ("OCT", 10),
    ("NOV", 11),
    ("DEC", 12),
];

const DOW_NAMES: &[(&str, u32)] = &[
    ("SUN", 0),
    ("MON", 1),
    ("TUE", 2),
    ("WED", 3),
    ("THU", 4),
    ("FRI", 5),
    ("SAT", 6),
];
