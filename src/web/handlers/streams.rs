//! Server-sent event routes for game status, system info, and live DST logs.
//!
//! Gin formats `ctx.SSEvent("message", string(json))` without a space after
//! `event:` or `data:`. The log stream is handwritten in Go and does include a
//! space after the colon. Both wire formats are intentionally preserved here.

use std::{
    convert::Infallible,
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    time::Duration,
};

use axum::{
    Json,
    body::Body,
    extract::{Query, State},
    http::{
        HeaderValue, StatusCode,
        header::{CACHE_CONTROL, CONNECTION, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use futures_util::stream;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    domain::game::{self, LevelStatusInfo, SystemInfo, level as game_level},
    dst,
    logs::{RecentLinesError, recent_lines_from_file},
    validation::{validate_level_name, validate_safe_command_arg},
    web::app::AppState,
    web::error::{AppError, AppResult},
    web::handlers::legacy_success,
    web::response::LoginResponse,
};

const STATUS_STREAM_INTERVAL: Duration = Duration::from_secs(2);
const LOG_POLL_INTERVAL: Duration = Duration::from_secs(1);
const LOG_HEARTBEAT_INTERVALS: u8 = 15;
const MAX_LOG_APPEND_BYTES: u64 = 256 * 1024;

#[derive(Debug, Deserialize)]
pub(crate) struct LogStreamQuery {
    #[serde(rename = "levelName")]
    level_name: Option<String>,
}

pub(crate) async fn status_stream_handler(State(state): State<AppState>) -> Response {
    periodic_message_stream(state, collect_status_snapshot)
}

pub(crate) async fn system_info_stream_handler(State(state): State<AppState>) -> Response {
    system_info_message_stream(state)
}

pub(crate) async fn log_stream_handler(
    State(state): State<AppState>,
    Query(query): Query<LogStreamQuery>,
) -> Response {
    let Some(level_name) = query.level_name.filter(|value| !value.is_empty()) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "cluster and level required"})),
        )
            .into_response();
    };
    let Ok(level_name) = validate_level_name(&level_name) else {
        tracing::warn!("rejected unsafe levelName for log stream");
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "cluster and level required"})),
        )
            .into_response();
    };

    let cluster_dir = match dst::current_cluster_dir(&state.root_path) {
        Ok(cluster_dir) => cluster_dir,
        Err(error) => {
            tracing::warn!(error = %error, "failed to resolve cluster for log stream");
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "cluster and level required"})),
            )
                .into_response();
        }
    };
    let relative_path = Path::new(level_name.as_str()).join("server_log.txt");
    let mut file = match dst::safe_open_cluster_file(&cluster_dir, &relative_path) {
        Ok(Some(file)) => file,
        Ok(None) => {
            tracing::warn!(
                level_name = level_name.as_str(),
                "server log file missing for stream"
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "log file not found"})),
            )
                .into_response();
        }
        Err(error) => {
            tracing::warn!(level_name = level_name.as_str(), error = %error, "server log path rejected");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "log stream unavailable"})),
            )
                .into_response();
        }
    };

    let offset = file.metadata().map(|metadata| metadata.len()).unwrap_or(0);
    let snapshot = match last_lines_chronological(&mut file, 100) {
        Ok(lines) => render_log_events(&lines),
        Err(error) => {
            tracing::warn!(level_name = level_name.as_str(), error = %error, "failed to snapshot log stream");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": log_stream_client_error(&error)})),
            )
                .into_response();
        }
    };

    tracing::info!(
        level_name = level_name.as_str(),
        "opened DST log SSE stream"
    );
    let stream = stream::unfold(
        LogStreamState {
            cluster_dir,
            relative_path,
            offset,
            first_chunk: Some(snapshot),
            idle_ticks: 0,
        },
        next_log_stream_chunk,
    );
    sse_response(Body::from_stream(stream))
}

fn periodic_message_stream<T>(state: AppState, collect: fn(&AppState) -> AppResult<T>) -> Response
where
    T: serde::Serialize + Send + 'static,
{
    let stream = stream::unfold((state, true), move |(state, first)| async move {
        if !first {
            tokio::time::sleep(STATUS_STREAM_INTERVAL).await;
        }
        let event = periodic_message_event(collect(&state));
        Some((Ok::<Bytes, Infallible>(Bytes::from(event)), (state, false)))
    });
    sse_response(Body::from_stream(stream))
}

fn system_info_message_stream(state: AppState) -> Response {
    let stream = stream::unfold((state, true), move |(state, first)| async move {
        if !first {
            tokio::time::sleep(STATUS_STREAM_INTERVAL).await;
        }
        let snapshot_state = state.clone();
        let result =
            tokio::task::spawn_blocking(move || collect_system_info_snapshot(&snapshot_state))
                .await
                .unwrap_or_else(|error| {
                    tracing::error!(%error, "system info SSE blocking task failed");
                    Err(AppError::internal("collect system info"))
                });
        let event = periodic_message_event(result);
        Some((Ok::<Bytes, Infallible>(Bytes::from(event)), (state, false)))
    });
    sse_response(Body::from_stream(stream))
}

fn periodic_message_event<T>(result: AppResult<T>) -> String
where
    T: serde::Serialize,
{
    match result {
        Ok(data) => {
            let response = legacy_success(data);
            match serde_json::to_string(&response) {
                Ok(payload) => format!("event:message\ndata:{payload}\n\n"),
                Err(error) => {
                    tracing::error!(error = %error, "failed to serialize SSE message payload");
                    String::new()
                }
            }
        }
        Err(error) => {
            tracing::warn!(error = %error, "failed to collect periodic SSE snapshot");
            let response = LoginResponse::<Value>::error(500, "internal server error");
            let payload = serde_json::to_string(&response).unwrap_or_else(|_| {
                "{\"code\":500,\"msg\":\"internal server error\",\"data\":null}".to_owned()
            });
            format!("event:message\ndata:{payload}\n\n")
        }
    }
}

fn collect_status_snapshot(state: &AppState) -> AppResult<Vec<LevelStatusInfo>> {
    let cluster_name =
        dst::current_cluster_name(&state.root_path).map_err(file_error("resolve cluster name"))?;
    let cluster_name = validate_safe_command_arg("cluster name", &cluster_name)
        .map_err(|error| AppError::bad_request(error.to_string()))?
        .into_string();
    let cluster_dir = dst::cluster_dir(&state.root_path, &cluster_name)
        .map_err(file_error("resolve cluster directory"))?;
    let worlds = game_level::list_existing_worlds_from_cluster_dir(&cluster_dir)?;
    let snapshots = match state.process_snapshot_provider.snapshots() {
        Ok(snapshots) => snapshots,
        Err(error) => {
            tracing::warn!(
                cluster_name,
                error = %error,
                "failed to collect process snapshots for SSE; reporting levels as stopped"
            );
            Vec::new()
        }
    };
    game::level_statuses_from_snapshots(&cluster_name, worlds, &snapshots)
        .map_err(|error| AppError::bad_request(error.to_string()))
}

fn collect_system_info_snapshot(state: &AppState) -> AppResult<SystemInfo> {
    let cluster_name =
        dst::current_cluster_name(&state.root_path).map_err(file_error("resolve cluster name"))?;
    tracing::debug!(cluster_name, "collecting system info for SSE stream");
    Ok(game::collect_system_info(&state.root_path))
}

#[derive(Debug)]
struct LogStreamState {
    cluster_dir: PathBuf,
    relative_path: PathBuf,
    offset: u64,
    first_chunk: Option<String>,
    idle_ticks: u8,
}

async fn next_log_stream_chunk(
    mut state: LogStreamState,
) -> Option<(Result<Bytes, Infallible>, LogStreamState)> {
    if let Some(first_chunk) = state.first_chunk.take() {
        return Some((Ok(Bytes::from(first_chunk)), state));
    }

    loop {
        tokio::time::sleep(LOG_POLL_INTERVAL).await;
        match read_appended_log_lines(&state.cluster_dir, &state.relative_path, state.offset) {
            Ok((new_offset, lines)) => {
                state.offset = new_offset;
                if !lines.is_empty() {
                    state.idle_ticks = 0;
                    return Some((Ok(Bytes::from(render_log_events(&lines))), state));
                }
            }
            Err(error) => {
                tracing::warn!(error = %error, "log stream follow failed");
                let event = "event: error\ndata: log stream unavailable\n\n".to_owned();
                return Some((Ok(Bytes::from(event)), state));
            }
        }

        state.idle_ticks = state.idle_ticks.saturating_add(1);
        if state.idle_ticks >= LOG_HEARTBEAT_INTERVALS {
            state.idle_ticks = 0;
            return Some((Ok(Bytes::from_static(b"event: ping\n\n")), state));
        }
    }
}

fn read_appended_log_lines(
    cluster_dir: &Path,
    relative_path: &Path,
    mut offset: u64,
) -> io::Result<(u64, Vec<String>)> {
    let Some(mut file) = dst::safe_open_cluster_file(cluster_dir, relative_path)? else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "log file not found",
        ));
    };
    let len = file.metadata()?.len();
    if len < offset {
        offset = 0;
    }
    if len == offset {
        return Ok((offset, Vec::new()));
    }

    file.seek(SeekFrom::Start(offset))?;
    let read_limit = len.saturating_sub(offset).min(MAX_LOG_APPEND_BYTES);
    let mut bytes = Vec::with_capacity(read_limit as usize);
    file.take(read_limit).read_to_end(&mut bytes)?;
    let new_offset = offset + bytes.len() as u64;
    let appended = String::from_utf8_lossy(&bytes);
    Ok((
        new_offset,
        appended.lines().map(ToOwned::to_owned).collect(),
    ))
}

fn last_lines_chronological(file: &mut File, limit: usize) -> io::Result<Vec<String>> {
    let mut lines = recent_lines_from_file(file, limit).map_err(recent_lines_error)?;
    lines.reverse();
    Ok(lines)
}

fn render_log_events(lines: &[String]) -> String {
    let mut output = String::new();
    for line in lines {
        output.push_str("event: log\n");
        for data_line in line.split('\n') {
            output.push_str("data: ");
            output.push_str(data_line);
            output.push('\n');
        }
        output.push('\n');
    }
    output
}

fn sse_response(body: Body) -> Response {
    let mut response = (StatusCode::OK, body).into_response();
    let headers = response.headers_mut();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/event-stream"));
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    headers.insert(CONNECTION, HeaderValue::from_static("keep-alive"));
    headers.insert("x-accel-buffering", HeaderValue::from_static("no"));
    response
}

fn file_error(
    operation: &'static str,
) -> impl FnOnce(io::Error) -> AppError + Copy + Send + Sync + 'static {
    move |error| {
        if error.kind() == io::ErrorKind::InvalidInput {
            AppError::bad_request(error.to_string())
        } else {
            tracing::error!(operation, error = %error, "stream handler file operation failed");
            AppError::internal(operation)
        }
    }
}

fn recent_lines_error(error: RecentLinesError) -> io::Error {
    match error {
        RecentLinesError::SnapshotTooLarge => io::Error::new(
            io::ErrorKind::InvalidData,
            "log snapshot exceeds safety limit",
        ),
        RecentLinesError::Io(error) => error,
    }
}

fn log_stream_client_error(error: &io::Error) -> &'static str {
    if error.kind() == io::ErrorKind::InvalidData {
        "log snapshot exceeds safety limit"
    } else {
        "log stream unavailable"
    }
}
