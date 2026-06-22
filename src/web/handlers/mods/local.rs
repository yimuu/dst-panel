//! Local mod and setup-workshop route handlers.

use std::{fs, path::PathBuf};

use axum::{
    Json,
    extract::{Path as AxumPath, Query, State},
};
use serde_json::Value;

use crate::{
    dst::DstConfig,
    infra::fs_paths::{safe_directory_exists_under_base, safe_remove_dir_all_under_base},
    web::{
        app::AppState,
        error::AppResult,
        handlers::{legacy_empty_success, legacy_success},
    },
};

use super::{
    dto::LangQuery,
    file_ops::{configured_directory_exists, fs_bad_request, validate_any_mod_id},
    service::{mod_detail_data, subscribe_mod_by_id},
};

/// Subscribes to or reads one mod and returns the legacy UI detail object.
pub(crate) async fn get_handler(
    State(state): State<AppState>,
    AxumPath(mod_id): AxumPath<String>,
    Query(query): Query<LangQuery>,
) -> AppResult<Json<crate::web::response::LoginResponse<Value>>> {
    let lang = query.lang.unwrap_or_else(|| "zh".to_owned());
    let mod_id = validate_any_mod_id(&mod_id)?;
    let record = subscribe_mod_by_id(&state, &mod_id, &lang).await?;
    tracing::info!(modid = %mod_id, "served mod detail");
    Ok(Json(legacy_success(mod_detail_data(&record))))
}

/// Deletes setup `workshop-*` directories under the installed server `mods`.
pub(crate) async fn delete_setup_workshop_handler(
    State(state): State<AppState>,
) -> AppResult<Json<crate::web::response::LoginResponse<Value>>> {
    let config = DstConfig::load(&state.root_path)?;
    let mods_path = PathBuf::from(&config.force_install_dir).join("mods");
    if !configured_directory_exists(&state.root_path, &mods_path)? {
        tracing::info!(mods_path = %mods_path.display(), "setup workshop directory is absent");
        return Ok(Json(legacy_empty_success()));
    }

    let mut removed = 0_usize;
    for entry in fs::read_dir(&mods_path)? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !name.contains("workshop") {
            continue;
        }
        if safe_directory_exists_under_base(&mods_path, name).map_err(fs_bad_request)? {
            safe_remove_dir_all_under_base(&mods_path, name).map_err(fs_bad_request)?;
            removed += 1;
        }
    }

    tracing::info!(
        mods_path = %mods_path.display(),
        removed,
        "deleted setup workshop directories"
    );
    Ok(Json(legacy_empty_success()))
}
