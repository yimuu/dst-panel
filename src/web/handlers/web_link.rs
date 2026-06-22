//! Web-link compatibility handlers for `/api/web/link`.
//!
//! The handlers expose the simple add/list/delete surface from the Go backend
//! and keep the legacy `code:200,msg:"success"` response envelope. URLs are
//! not logged because they can carry query-string credentials in deployments.

use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    domain::admin::model::{NewWebLink, WebLinkRecord},
    domain::admin::repositories::web_link::WebLinkRepository,
    web::app::AppState,
    web::error::{AppError, AppResult},
    web::handlers::{legacy_empty_success, legacy_success, repository_error},
    web::response::LoginResponse,
};

/// Query parameters accepted by the web-link delete endpoint.
#[derive(Debug, Deserialize)]
pub(crate) struct DeleteWebLinkQuery {
    #[serde(rename = "ID")]
    uppercase_id: Option<i64>,
    id: Option<i64>,
}

impl DeleteWebLinkQuery {
    fn id(&self) -> Option<i64> {
        self.uppercase_id.or(self.id)
    }
}

/// Lists all active web links.
pub(crate) async fn list_handler(
    State(state): State<AppState>,
) -> AppResult<Json<LoginResponse<Vec<WebLinkRecord>>>> {
    tracing::debug!("listing web links");
    let repository = WebLinkRepository::new(state.db);
    let links = repository
        .list()
        .await
        .map_err(|error| repository_error("list web links", error))?;

    tracing::debug!(count = links.len(), "listed web links");
    Ok(Json(legacy_success(links)))
}

/// Creates a web link from the JSON body.
pub(crate) async fn create_handler(
    State(state): State<AppState>,
    Json(request): Json<NewWebLink>,
) -> AppResult<Json<LoginResponse<Value>>> {
    if request.title.trim().is_empty() || request.url.trim().is_empty() {
        tracing::warn!(
            title_empty = request.title.trim().is_empty(),
            url_empty = request.url.trim().is_empty(),
            "rejected web link with missing required fields"
        );
        return Err(AppError::bad_request("web link title and url are required"));
    }

    tracing::debug!(title = %request.title, "creating web link");
    let repository = WebLinkRepository::new(state.db);
    repository
        .add(request)
        .await
        .map_err(|error| repository_error("create web link", error))?;

    tracing::info!("created web link");
    Ok(Json(legacy_empty_success()))
}

/// Soft-deletes a web link by `ID` query parameter, accepting lowercase `id`.
pub(crate) async fn delete_handler(
    State(state): State<AppState>,
    Query(query): Query<DeleteWebLinkQuery>,
) -> AppResult<Json<LoginResponse<Value>>> {
    let id = query
        .id()
        .filter(|id| *id > 0)
        .ok_or_else(|| AppError::bad_request("web link ID is required"))?;

    tracing::debug!(id, "deleting web link");
    let repository = WebLinkRepository::new(state.db);
    repository
        .delete(id)
        .await
        .map_err(|error| repository_error("delete web link", error))?;

    tracing::info!(id, "deleted web link");
    Ok(Json(legacy_empty_success()))
}
