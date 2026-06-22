//! Manual `modinfo.lua` upload route handlers.

use std::path::PathBuf;

use axum::{
    Json,
    extract::{Query, State},
};
use serde_json::Value;

use crate::{
    domain::mods::{model::ModInfoInput, repository::ModInfoRepository},
    dst::{DstConfig, safe_ensure_configured_dir},
    infra::fs_paths::safe_overwrite_file_under_base,
    web::{
        app::AppState,
        error::{AppError, AppResult},
        handlers::{legacy_empty_success, repository_error},
    },
};

use super::{
    dto::{LangQuery, ManualModInfoPayload},
    file_ops::{MAX_MODINFO_BYTES, ensure_absolute_dir, fs_bad_request, validate_any_mod_id},
    lua_config::mod_config_from_lua_source,
    service::{local_mod_record, mod_record_from_detail},
    steam_api::{STEAM_DETAIL_LANG, fetch_steam_details},
};

/// Writes a manually supplied `modinfo.lua` file and stores local metadata.
pub(crate) async fn add_modinfo_file_handler(
    State(state): State<AppState>,
    Query(query): Query<LangQuery>,
    Json(payload): Json<ManualModInfoPayload>,
) -> AppResult<Json<crate::web::response::LoginResponse<Value>>> {
    let lang = query.lang.unwrap_or_else(|| "zh".to_owned());
    let workshop_id = validate_any_mod_id(&payload.workshop_id)?;
    if payload.modinfo.len() > MAX_MODINFO_BYTES {
        return Err(AppError::payload_too_large("modinfo.lua is too large"));
    }
    let config = DstConfig::load(&state.root_path)?;
    let base = PathBuf::from(&config.mod_download_path)
        .join("steamapps")
        .join("workshop")
        .join("content")
        .join("322330");
    ensure_absolute_dir(&state.root_path, &base)?;
    safe_ensure_configured_dir(
        &state.root_path,
        &base.join(&workshop_id).display().to_string(),
    )?;
    let relative_file = PathBuf::from(&workshop_id).join("modinfo.lua");
    safe_overwrite_file_under_base(&base, relative_file, payload.modinfo.as_bytes())
        .map_err(fs_bad_request)?;

    let repository = ModInfoRepository::new(state.db.clone());
    let mod_config = mod_config_from_lua_source(&payload.modinfo, &lang, &workshop_id);
    let record = if workshop_id.parse::<i64>().is_ok() {
        let mut details =
            fetch_steam_details(&state, &[workshop_id.as_str()], STEAM_DETAIL_LANG).await?;
        let Some(detail) = details.pop() else {
            return Err(AppError::bad_request("steam mod details not found"));
        };
        mod_record_from_detail(&detail, mod_config)
    } else {
        local_mod_record(&workshop_id, mod_config)
    };
    repository
        .upsert_by_modid(ModInfoInput::from(&record))
        .await
        .map_err(|error| repository_error("save manual mod info", error))?;

    tracing::info!(
        modid = %workshop_id,
        bytes = payload.modinfo.len(),
        "wrote manual modinfo.lua"
    );
    Ok(Json(legacy_empty_success()))
}
