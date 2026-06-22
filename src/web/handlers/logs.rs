//! Log snapshot and download handlers for game and panel logs.
//!
//! These routes are read-only. They preserve the Go response envelope and
//! newest-first line order while using descriptor-anchored safe file helpers so
//! `levelName` and `fileName` query values cannot escape the selected cluster.

use std::{fs::File, io, path::Path};

use axum::{
    Json,
    body::Body,
    extract::{Query, State},
    http::{
        HeaderName, HeaderValue, StatusCode,
        header::{CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use futures_util::stream;
use serde::Deserialize;
use tokio::io::AsyncReadExt;

use crate::{
    dst,
    infra::fs_paths::safe_open_optional_existing_file_under_base,
    infra::logging::DEFAULT_LOG_FILE,
    logs::{RecentLinesError, parse_line_limit, recent_lines_from_file},
    validation::{validate_filename, validate_level_name},
    web::app::AppState,
    web::error::{AppError, AppResult},
    web::handlers::legacy_success,
    web::response::LoginResponse,
};

const CONTENT_TRANSFER_ENCODING: HeaderName = HeaderName::from_static("content-transfer-encoding");
const DOWNLOAD_CHUNK_SIZE: usize = 64 * 1024;

#[derive(Debug, Deserialize)]
pub(crate) struct LevelLogQuery {
    #[serde(rename = "levelName")]
    level_name: String,
    lines: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PanelLogQuery {
    lines: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LevelLogDownloadQuery {
    #[serde(rename = "levelName")]
    level_name: String,
    #[serde(rename = "fileName")]
    file_name: String,
}

pub(crate) async fn level_server_log_handler(
    State(state): State<AppState>,
    Query(query): Query<LevelLogQuery>,
) -> AppResult<Json<LoginResponse<Vec<String>>>> {
    let logs = read_cluster_log_lines(
        &state,
        &query.level_name,
        "server_log.txt",
        query.lines.as_deref(),
    )?;
    tracing::debug!(
        level_name = %query.level_name,
        line_count = logs.len(),
        "read DST level server log snapshot"
    );
    Ok(Json(legacy_success(logs)))
}

pub(crate) async fn level_server_chat_log_handler(
    State(state): State<AppState>,
    Query(query): Query<LevelLogQuery>,
) -> AppResult<Json<LoginResponse<Vec<String>>>> {
    let logs = read_cluster_log_lines(
        &state,
        &query.level_name,
        "server_chat_log.txt",
        query.lines.as_deref(),
    )?;
    tracing::debug!(
        level_name = %query.level_name,
        line_count = logs.len(),
        "read DST level server chat log snapshot"
    );
    Ok(Json(legacy_success(logs)))
}

pub(crate) async fn panel_log_handler(
    State(state): State<AppState>,
    Query(query): Query<PanelLogQuery>,
) -> AppResult<Json<LoginResponse<Vec<String>>>> {
    let mut file = open_panel_log(&state)?.ok_or_else(|| AppError::internal("read panel log"))?;
    let logs = recent_lines_from_file(&mut file, parse_line_limit(query.lines.as_deref()))
        .map_err(recent_lines_error("read panel log"))?;
    tracing::debug!(line_count = logs.len(), "read panel log snapshot");
    Ok(Json(legacy_success(logs)))
}

pub(crate) async fn level_log_download_handler(
    State(state): State<AppState>,
    Query(query): Query<LevelLogDownloadQuery>,
) -> AppResult<Response> {
    let safe_level =
        validate_level_name(&query.level_name).map_err(validation_error_to_bad_request)?;
    let safe_file = validate_filename(&query.file_name).map_err(validation_error_to_bad_request)?;
    let cluster_dir = dst::current_cluster_dir(&state.root_path)
        .map_err(file_error("resolve cluster directory"))?;
    let relative_path = Path::new(safe_level.as_str()).join(safe_file.as_str());
    let file = dst::safe_open_cluster_file(&cluster_dir, &relative_path)
        .map_err(file_error("open level log download"))?
        .ok_or_else(|| AppError::not_found("log file not found"))?;
    tracing::info!(
        level_name = safe_level.as_str(),
        file_name = safe_file.as_str(),
        "serving DST level log download"
    );
    download_response(file, safe_file.as_str())
}

pub(crate) async fn panel_log_download_handler(
    State(state): State<AppState>,
) -> AppResult<Response> {
    let file = open_panel_log(&state)?.ok_or_else(|| AppError::not_found("log file not found"))?;
    tracing::info!("serving panel log download");
    download_response(file, DEFAULT_LOG_FILE)
}

fn read_cluster_log_lines(
    state: &AppState,
    level_name: &str,
    file_name: &str,
    line_query: Option<&str>,
) -> AppResult<Vec<String>> {
    let safe_level = validate_level_name(level_name).map_err(validation_error_to_bad_request)?;
    let cluster_dir = dst::current_cluster_dir(&state.root_path)
        .map_err(file_error("resolve cluster directory"))?;
    let relative_path = Path::new(safe_level.as_str()).join(file_name);
    let Some(mut file) = dst::safe_open_cluster_file(&cluster_dir, &relative_path)
        .map_err(file_error("read level log file"))?
    else {
        return Ok(Vec::new());
    };
    recent_lines_from_file(&mut file, parse_line_limit(line_query))
        .map_err(recent_lines_error("read level log file"))
}

fn open_panel_log(state: &AppState) -> AppResult<Option<File>> {
    safe_open_optional_existing_file_under_base(&state.root_path, DEFAULT_LOG_FILE).map_err(
        |error| {
            tracing::error!(error = %error, "panel log path is unavailable or unsafe");
            AppError::internal("open panel log")
        },
    )
}

fn download_response(file: File, file_name: &str) -> AppResult<Response> {
    let len = file
        .metadata()
        .map_err(|error| file_error("read log download metadata")(error))?
        .len();
    let disposition = content_disposition_attachment(file_name)?;
    let content_length = HeaderValue::from_str(&len.to_string())
        .map_err(|_| AppError::internal("encode download length"))?;

    let file = tokio::fs::File::from_std(file);
    let stream = stream::unfold((file, len), |(mut file, remaining)| async move {
        if remaining == 0 {
            return None;
        }
        let read_len = remaining.min(DOWNLOAD_CHUNK_SIZE as u64) as usize;
        let mut buffer = vec![0_u8; read_len];
        match file.read(&mut buffer).await {
            Ok(0) => Some((
                Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "log file ended before advertised content length",
                )),
                (file, 0),
            )),
            Ok(bytes_read) => {
                buffer.truncate(bytes_read);
                let remaining = remaining.saturating_sub(bytes_read as u64);
                Some((
                    Ok::<Bytes, io::Error>(Bytes::from(buffer)),
                    (file, remaining),
                ))
            }
            Err(error) => Some((Err(error), (file, 0))),
        }
    });

    let mut response = (StatusCode::OK, Body::from_stream(stream)).into_response();
    let headers = response.headers_mut();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    headers.insert(CONTENT_DISPOSITION, disposition);
    headers.insert(
        CONTENT_TRANSFER_ENCODING,
        HeaderValue::from_static("binary"),
    );
    headers.insert(CONTENT_LENGTH, content_length);
    Ok(response)
}

fn content_disposition_attachment(file_name: &str) -> AppResult<HeaderValue> {
    let escaped = file_name.replace('\\', "\\\\").replace('"', "\\\"");
    HeaderValue::from_str(&format!("attachment; filename=\"{escaped}\""))
        .map_err(|_| AppError::internal("encode download filename"))
}

fn validation_error_to_bad_request(error: crate::validation::ValidationError) -> AppError {
    AppError::bad_request(error.to_string())
}

fn recent_lines_error(operation: &'static str) -> impl FnOnce(RecentLinesError) -> AppError {
    move |error| match error {
        RecentLinesError::SnapshotTooLarge => {
            tracing::warn!(
                operation,
                "log snapshot request exceeded configured safety limit"
            );
            AppError::payload_too_large("log snapshot is too large")
        }
        RecentLinesError::Io(error) => file_error(operation)(error),
    }
}

fn file_error(
    operation: &'static str,
) -> impl FnOnce(io::Error) -> AppError + Copy + Send + Sync + 'static {
    move |error| {
        if error.kind() == io::ErrorKind::InvalidInput {
            AppError::bad_request(error.to_string())
        } else {
            tracing::error!(operation, error = %error, "log handler file operation failed");
            AppError::internal(operation)
        }
    }
}
