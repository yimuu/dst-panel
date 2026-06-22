//! Webhook compatibility handler for `POST /webhook`.
//!
//! The Go route verifies a query-string key against the local `key` file and
//! then dispatches a small set of message types. This implementation preserves
//! the public behavior while avoiding Go's unsafe side effect of creating a
//! path derived from the query key.

use std::io::Read;

use axum::{
    Json,
    body::Bytes,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    infra::fs_paths::safe_open_optional_existing_file_under_base,
    web::app::AppState,
    web::error::{AppError, AppResult},
    web::handlers::{backup, legacy_empty_success, legacy_success},
};

/// Query parameters accepted by the webhook endpoint.
#[derive(Debug, Deserialize)]
pub(crate) struct WebhookQuery {
    key: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct WebhookRequest {
    #[serde(default)]
    msgtype: String,
    #[serde(default)]
    param: Value,
}

/// Handles the Go-compatible webhook message surface.
pub(crate) async fn handler(
    State(state): State<AppState>,
    Query(query): Query<WebhookQuery>,
    body: Bytes,
) -> AppResult<Response> {
    verify_webhook_key(&state, query.key.as_deref())?;
    let request = parse_webhook_body(&body);
    tracing::debug!(msgtype = %request.msgtype, "accepted webhook request");
    let _ = request.param;

    match request.msgtype.as_str() {
        "searchHome" => Ok(StatusCode::OK.into_response()),
        "homeInfo" => {
            let root = state.root_path.clone();
            let config = state.config.clone();
            let archive =
                tokio::task::spawn_blocking(move || backup::build_game_archive(&root, &config))
                    .await
                    .map_err(|error| {
                        tracing::error!(error = %error, "homeInfo webhook join failed");
                        AppError::internal("build webhook homeInfo")
                    })??;
            Ok(Json(legacy_success(archive)).into_response())
        }
        "onlinePlayers" => Ok(Json(legacy_success(Vec::<Value>::new())).into_response()),
        _ => Ok(Json(legacy_empty_success()).into_response()),
    }
}

fn verify_webhook_key(state: &AppState, provided_key: Option<&str>) -> AppResult<()> {
    let Some(provided_key) = provided_key else {
        tracing::warn!("rejected webhook request without key");
        return Err(AppError::bad_request("invalid webhook key"));
    };

    let Some(mut key_file) = safe_open_optional_existing_file_under_base(&state.root_path, "key")
        .map_err(|error| {
        tracing::warn!(error = %error, "rejected webhook request because key file is unsafe");
        AppError::bad_request("invalid webhook key")
    })?
    else {
        tracing::warn!("rejected webhook request because key file is missing");
        return Err(AppError::bad_request("invalid webhook key"));
    };
    let mut expected_key = String::new();
    key_file.read_to_string(&mut expected_key).map_err(|error| {
        tracing::warn!(error = %error, "rejected webhook request because key file is unavailable");
        AppError::bad_request("invalid webhook key")
    })?;
    let expected_key = expected_key.trim_end_matches(['\r', '\n']);
    if provided_key != expected_key {
        tracing::warn!("rejected webhook request with invalid key");
        return Err(AppError::bad_request("invalid webhook key"));
    }

    Ok(())
}

fn parse_webhook_body(body: &[u8]) -> WebhookRequest {
    if body.is_empty() {
        return WebhookRequest::default();
    }
    match serde_json::from_slice::<WebhookRequest>(body) {
        Ok(request) => request,
        Err(error) => {
            tracing::warn!(error = %error, "ignored malformed webhook body");
            WebhookRequest::default()
        }
    }
}
