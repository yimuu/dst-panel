//! Announcement setting handlers for `/api/game/announce/setting`.
//!
//! Go persists announcement settings with `db.Save` and returns the saved
//! struct in a `vo.Response` envelope. Binding errors are logged but ignored,
//! so the Rust handler deliberately falls back to a zero-value body when JSON
//! parsing fails.

use axum::{Json, body::Bytes, extract::State};

use crate::{
    domain::scheduler::model::{AnnounceRecord, SaveAnnounce},
    domain::scheduler::repository::announcement::AnnouncementRepository,
    web::app::AppState,
    web::error::AppResult,
    web::handlers::{legacy_success, repository_error},
    web::response::LoginResponse,
};

/// Returns the first active announcement setting or Go's zero-value model.
pub(crate) async fn get_handler(
    State(state): State<AppState>,
) -> AppResult<Json<LoginResponse<AnnounceRecord>>> {
    let repository = AnnouncementRepository::new(state.db);
    let setting = repository
        .first_or_zero()
        .await
        .map_err(|error| repository_error("get announcement setting", error))?;
    tracing::debug!(id = setting.id, "loaded announcement setting");
    Ok(Json(legacy_success(setting)))
}

/// Saves an announcement setting using GORM-like insert/update semantics.
pub(crate) async fn save_handler(
    State(state): State<AppState>,
    body: Bytes,
) -> AppResult<Json<LoginResponse<AnnounceRecord>>> {
    let request = parse_announce_body(&body);
    let repository = AnnouncementRepository::new(state.db);
    let saved = repository
        .save(request)
        .await
        .map_err(|error| repository_error("save announcement setting", error))?;
    tracing::info!(
        id = saved.id,
        enabled = saved.enable,
        "saved announcement setting"
    );
    Ok(Json(legacy_success(saved)))
}

fn parse_announce_body(body: &[u8]) -> SaveAnnounce {
    if body.is_empty() {
        return SaveAnnounce::default();
    }
    match serde_json::from_slice::<SaveAnnounce>(body) {
        Ok(request) => request,
        Err(error) => {
            tracing::warn!(error = %error, "ignored malformed announcement setting body");
            SaveAnnounce::default()
        }
    }
}
