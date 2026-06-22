//! DB-backed player log query and soft-delete handlers.

use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    domain::statistics::model::PlayerLogRecord,
    domain::statistics::repository::player_log::{
        PlayerLogFilter, PlayerLogPage, PlayerLogRepository, normalize_page_size,
    },
    web::app::AppState,
    web::error::{AppError, AppResult},
    web::handlers::{legacy_empty_success, legacy_success, repository_error},
    web::response::LoginResponse,
};

/// Go-compatible player log query parameters.
#[derive(Debug, Deserialize)]
pub(crate) struct PlayerLogQuery {
    page: Option<i64>,
    size: Option<i64>,
    name: Option<String>,
    #[serde(rename = "kuId")]
    ku_id: Option<String>,
    #[serde(rename = "steamId")]
    steam_id: Option<String>,
    role: Option<String>,
    action: Option<String>,
    ip: Option<String>,
}

/// Body accepted by Go's player-log delete route.
#[derive(Debug, Deserialize)]
pub(crate) struct DeletePlayerLogRequest {
    ids: Vec<i64>,
}

/// Go `vo.Page` response shape for player logs.
#[derive(Debug, Serialize)]
pub(crate) struct Page<T> {
    data: Vec<T>,
    total: i64,
    #[serde(rename = "totalPages")]
    total_pages: i64,
    page: i64,
    size: i64,
}

impl From<PlayerLogPage> for Page<PlayerLogRecord> {
    fn from(page: PlayerLogPage) -> Self {
        Self {
            data: page.data,
            total: page.total,
            total_pages: page.total_pages,
            page: page.page,
            size: page.size,
        }
    }
}

pub(crate) async fn list_handler(
    State(state): State<AppState>,
    Query(query): Query<PlayerLogQuery>,
) -> AppResult<Json<LoginResponse<Page<PlayerLogRecord>>>> {
    let page = query.page.unwrap_or(1).max(1);
    let size = normalize_page_size(query.size.unwrap_or(10));
    if size == 0 {
        return Err(AppError::bad_request("size must be greater than zero"));
    }

    let filter = PlayerLogFilter {
        name: query.name,
        ku_id: query.ku_id,
        steam_id: query.steam_id,
        role: query.role,
        action: query.action,
        ip: query.ip,
    };
    let repository = PlayerLogRepository::new(state.db);
    let page_data = repository
        .list_page(&filter, page, size)
        .await
        .map_err(|error| repository_error("list player logs", error))?;
    tracing::debug!(
        page,
        size,
        total = page_data.total,
        returned = page_data.data.len(),
        "listed player logs"
    );
    Ok(Json(legacy_success(page_data.into())))
}

pub(crate) async fn delete_handler(
    State(state): State<AppState>,
    Json(request): Json<DeletePlayerLogRequest>,
) -> AppResult<Json<LoginResponse<Value>>> {
    let delete_count = request.ids.len();
    let repository = PlayerLogRepository::new(state.db);
    repository
        .soft_delete_ids(&request.ids)
        .await
        .map_err(|error| repository_error("delete player logs", error))?;
    tracing::info!(delete_count, "soft-deleted player logs");
    Ok(Json(legacy_empty_success()))
}
