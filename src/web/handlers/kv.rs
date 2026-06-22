//! Key/value compatibility handlers for `/api/kv`.
//!
//! These endpoints preserve the older Go `vo.Response` success envelope while
//! delegating persistence to the existing repository. Handler logs include the
//! key name and operation outcome only; values are never logged because KV
//! entries may contain deployment secrets.

use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;

use crate::{
    domain::admin::repositories::kv::KvRepository,
    web::app::AppState,
    web::error::AppResult,
    web::handlers::{legacy_success, repository_error},
    web::response::LoginResponse,
};

/// Query parameters accepted by the KV read endpoint.
#[derive(Debug, Deserialize)]
pub(crate) struct KvQuery {
    key: Option<String>,
}

/// JSON body accepted by the KV save endpoint.
#[derive(Debug, Deserialize)]
pub(crate) struct SaveKvRequest {
    key: String,
    value: String,
}

/// Reads a KV value by query-string key, returning an empty string when absent.
pub(crate) async fn get_handler(
    State(state): State<AppState>,
    Query(query): Query<KvQuery>,
) -> AppResult<Json<LoginResponse<String>>> {
    let key = query.key.unwrap_or_default();
    tracing::debug!(key = %key, "reading kv value");

    let repository = KvRepository::new(state.db);
    let value = repository
        .get(&key)
        .await
        .map_err(|error| repository_error("read kv value", error))?
        .unwrap_or_default();

    tracing::debug!(key = %key, found = !value.is_empty(), "read kv value completed");
    Ok(Json(legacy_success(value)))
}

/// Saves or updates a KV value and returns the saved value.
pub(crate) async fn save_handler(
    State(state): State<AppState>,
    Json(request): Json<SaveKvRequest>,
) -> AppResult<Json<LoginResponse<String>>> {
    if request.key.trim().is_empty() {
        tracing::warn!("rejected kv save with empty key");
        return Err(crate::web::error::AppError::bad_request(
            "kv key must not be empty",
        ));
    }

    tracing::debug!(key = %request.key, "saving kv value");
    let repository = KvRepository::new(state.db);
    repository
        .save(&request.key, &request.value)
        .await
        .map_err(|error| repository_error("save kv value", error))?;

    tracing::info!(key = %request.key, "saved kv value");
    Ok(Json(legacy_success(request.value)))
}
