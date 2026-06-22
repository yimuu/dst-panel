//! Stored mod metadata route handlers.

use axum::{
    Json,
    extract::{Path as AxumPath, State},
};
use serde_json::Value;

use crate::{
    domain::mods::{
        model::{ModInfoInput, ModInfoRecord},
        repository::ModInfoRepository,
    },
    web::{
        app::AppState,
        error::{AppError, AppResult},
        handlers::{legacy_success, repository_error},
    },
};

use super::{
    file_ops::{delete_mod_download_dir, validate_any_mod_id},
    service::mod_detail_data,
};

/// Lists stored mods using the Go detail-list shape.
pub(crate) async fn list_handler(
    State(state): State<AppState>,
) -> AppResult<Json<crate::web::response::LoginResponse<Value>>> {
    let repository = ModInfoRepository::new(state.db.clone());
    let rows = repository
        .list_active()
        .await
        .map_err(|error| repository_error("list mod infos", error))?;
    let data = rows.iter().map(mod_detail_data).collect::<Vec<_>>();
    tracing::info!(count = data.len(), "listed stored mod infos");
    Ok(Json(legacy_success(Value::Array(data))))
}

/// Deletes stored metadata and the SteamCMD download cache for a mod.
pub(crate) async fn delete_handler(
    State(state): State<AppState>,
    AxumPath(mod_id): AxumPath<String>,
) -> AppResult<Json<crate::web::response::LoginResponse<String>>> {
    let mod_id = validate_any_mod_id(&mod_id)?;
    delete_mod_download_dir(&state.root_path, &mod_id)?;
    let repository = ModInfoRepository::new(state.db.clone());
    let deleted_rows = repository
        .soft_delete_by_modid(&mod_id)
        .await
        .map_err(|error| repository_error("delete mod info", error))?;
    tracing::info!(
        modid = %mod_id,
        deleted_rows,
        "deleted mod metadata and SteamCMD cache"
    );
    Ok(Json(legacy_success(mod_id)))
}

/// Returns the raw `model.ModInfo` record shape used by Go's editor route.
pub(crate) async fn raw_modinfo_handler(
    State(state): State<AppState>,
    AxumPath(mod_id): AxumPath<String>,
) -> AppResult<Json<crate::web::response::LoginResponse<ModInfoRecord>>> {
    let mod_id = validate_any_mod_id(&mod_id)?;
    let repository = ModInfoRepository::new(state.db.clone());
    let record = repository
        .find_active_by_modid(&mod_id)
        .await
        .map_err(|error| repository_error("get mod info", error))?
        .unwrap_or_else(ModInfoRecord::zero);
    tracing::info!(modid = %mod_id, found = record.id != 0, "served raw mod info");
    Ok(Json(legacy_success(record)))
}

/// Creates or updates a raw `mod_infos` row.
pub(crate) async fn save_raw_modinfo_handler(
    State(state): State<AppState>,
    Json(input): Json<ModInfoInput>,
) -> AppResult<Json<crate::web::response::LoginResponse<ModInfoRecord>>> {
    if input.modid.trim().is_empty() {
        return Err(AppError::bad_request("modid must not be empty"));
    }
    let mut input = input;
    input.modid = validate_any_mod_id(&input.modid)?;
    let repository = ModInfoRepository::new(state.db.clone());
    let record = if input.id > 0 {
        repository
            .save(input)
            .await
            .map_err(|error| repository_error("save mod info", error))?
    } else {
        repository
            .upsert_by_modid(input)
            .await
            .map_err(|error| repository_error("save mod info", error))?
    };
    tracing::info!(modid = %record.modid, id = record.id, "saved raw mod info");
    Ok(Json(legacy_success(record)))
}
