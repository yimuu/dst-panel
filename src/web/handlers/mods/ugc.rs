//! UGC install inspection and cleanup route handlers.

use axum::{
    Json,
    extract::{Query, State},
};
use serde_json::{Value, json};

use crate::{
    dst::DstConfig,
    infra::fs_paths::{safe_directory_exists_under_base, safe_remove_dir_all_under_base},
    validation::validate_level_name,
    web::{
        app::AppState,
        error::{AppError, AppResult},
        handlers::{legacy_empty_success, legacy_success},
    },
};

use super::{
    dto::{DeleteUgcQuery, UgcLevelQuery},
    file_ops::{
        configured_directory_exists, fs_bad_request, parse_acf_file, ugc_acf_location,
        ugc_content_root, validate_any_mod_id,
    },
    steam_api::{fetch_steam_details, image_with_suffix},
};

/// Reads `appworkshop_322330.acf` and enriches installed ids from Steam.
pub(crate) async fn ugc_acf_handler(
    State(state): State<AppState>,
    Query(query): Query<UgcLevelQuery>,
) -> AppResult<Json<crate::web::response::LoginResponse<Value>>> {
    let config = DstConfig::load(&state.root_path)?;
    let level = validate_level_name(&query.level_name.unwrap_or_default())
        .map_err(|error| AppError::bad_request(error.to_string()))?;
    let (acf_base, acf_file) = ugc_acf_location(&config, level.as_str());
    let items = parse_acf_file(&state.root_path, &acf_base, &acf_file)?;
    if items.is_empty() {
        tracing::info!(
            acf_base = %acf_base.display(),
            acf_file = %acf_file.display(),
            "UGC ACF contained no workshop ids"
        );
        return Ok(Json(legacy_success(Value::Array(Vec::new()))));
    }

    let ids = items.keys().map(String::as_str).collect::<Vec<_>>();
    let details = fetch_steam_details(&state, &ids, "zh").await?;
    let mut data = Vec::new();
    for detail in details {
        let Some(item) = items.get(&detail.publishedfileid) else {
            continue;
        };
        data.push(json!({
            "workshopId": detail.publishedfileid,
            "name": detail.title,
            "timeupdated": item.time_updated,
            "timelast": detail.time_updated,
            "img": image_with_suffix(&detail.preview_url),
        }));
    }

    tracing::info!(
        acf_base = %acf_base.display(),
        acf_file = %acf_file.display(),
        count = data.len(),
        "served UGC ACF details"
    );
    Ok(Json(legacy_success(Value::Array(data))))
}

/// Deletes one installed UGC workshop directory.
pub(crate) async fn delete_ugc_handler(
    State(state): State<AppState>,
    Query(query): Query<DeleteUgcQuery>,
) -> AppResult<Json<crate::web::response::LoginResponse<Value>>> {
    let config = DstConfig::load(&state.root_path)?;
    let level = validate_level_name(&query.level_name)
        .map_err(|error| AppError::bad_request(error.to_string()))?;
    let workshop_id = validate_any_mod_id(&query.workshop_id)?;
    let base = ugc_content_root(&config, level.as_str());
    if !configured_directory_exists(&state.root_path, &base)? {
        return Ok(Json(legacy_empty_success()));
    }
    if safe_directory_exists_under_base(&base, &workshop_id).map_err(fs_bad_request)? {
        safe_remove_dir_all_under_base(&base, &workshop_id).map_err(fs_bad_request)?;
        tracing::info!(
            workshop_id = %workshop_id,
            level = %level.as_str(),
            "deleted UGC workshop directory"
        );
    }
    Ok(Json(legacy_empty_success()))
}
