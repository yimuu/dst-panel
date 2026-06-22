//! Steam-backed mod refresh route handlers.

use axum::{
    Json,
    extract::{Path as AxumPath, Query, State},
};
use serde_json::Value;

use crate::{
    domain::mods::{model::ModInfoInput, repository::ModInfoRepository},
    dst::DstConfig,
    web::{
        app::AppState,
        error::{AppError, AppResult},
        handlers::{legacy_empty_success, legacy_success, repository_error},
    },
};

use super::{
    dto::LangQuery,
    file_ops::{delete_mod_download_dir, refresh_mod_config_for_detail, validate_any_mod_id},
    service::{mod_detail_data, mod_record_from_detail, subscribe_mod_by_id},
    steam_api::{STEAM_DETAIL_LANG, fetch_steam_details},
};

/// Best-effort refresh of all stored mod infos.
pub(crate) async fn update_all_handler(
    State(state): State<AppState>,
    Query(query): Query<LangQuery>,
) -> AppResult<Json<crate::web::response::LoginResponse<Value>>> {
    let lang = query.lang.unwrap_or_else(|| "zh".to_owned());
    let repository = ModInfoRepository::new(state.db.clone());
    let rows = repository
        .list_active()
        .await
        .map_err(|error| repository_error("list mod infos for refresh", error))?;
    let mut refreshed = 0_usize;
    for row in rows {
        if row.modid.parse::<i64>().is_err() {
            continue;
        }
        match fetch_steam_details(&state, &[row.modid.as_str()], STEAM_DETAIL_LANG).await {
            Ok(mut details) => {
                let Some(detail) = details.pop() else {
                    continue;
                };
                if detail.time_updated <= row.last_time {
                    continue;
                }
                let config = DstConfig::load(&state.root_path)?;
                let mod_config = match refresh_mod_config_for_detail(
                    &state, &config, &lang, &row.modid, &detail,
                )
                .await
                {
                    Ok(mod_config) => mod_config,
                    Err(error) => {
                        tracing::warn!(
                            modid = %row.modid,
                            error = %error,
                            "skipping mod refresh after mod config refresh failed"
                        );
                        continue;
                    }
                };
                let mut input = ModInfoInput::from(&mod_record_from_detail(&detail, mod_config));
                input.id = row.id;
                repository
                    .save(input)
                    .await
                    .map_err(|error| repository_error("refresh mod info", error))?;
                refreshed += 1;
            }
            Err(error) => {
                tracing::warn!(
                    modid = %row.modid,
                    error = %error,
                    "skipping failed mod refresh"
                );
            }
        }
    }

    tracing::info!(refreshed, "finished best-effort mod info refresh");
    Ok(Json(legacy_empty_success()))
}

/// Forces a single mod refresh after deleting the existing row/cache.
pub(crate) async fn update_handler(
    State(state): State<AppState>,
    AxumPath(mod_id): AxumPath<String>,
    Query(query): Query<LangQuery>,
) -> AppResult<Json<crate::web::response::LoginResponse<Value>>> {
    let lang = query.lang.unwrap_or_else(|| "zh".to_owned());
    let mod_id = validate_any_mod_id(&mod_id)?;
    let repository = ModInfoRepository::new(state.db.clone());
    if mod_id.parse::<i64>().is_err() {
        delete_mod_download_dir(&state.root_path, &mod_id)?;
        repository
            .soft_delete_by_modid(&mod_id)
            .await
            .map_err(|error| repository_error("soft delete before mod refresh", error))?;
        let record = subscribe_mod_by_id(&state, &mod_id, &lang).await?;
        tracing::info!(modid = %mod_id, "force-refreshed local mod info");
        return Ok(Json(legacy_success(mod_detail_data(&record))));
    }

    let mut details = fetch_steam_details(&state, &[mod_id.as_str()], STEAM_DETAIL_LANG).await?;
    let Some(detail) = details.pop() else {
        return Err(AppError::bad_request("steam mod details not found"));
    };
    let config = DstConfig::load(&state.root_path)?;
    let mod_config =
        refresh_mod_config_for_detail(&state, &config, &lang, &mod_id, &detail).await?;
    let record = repository
        .upsert_by_modid(ModInfoInput::from(&mod_record_from_detail(
            &detail, mod_config,
        )))
        .await
        .map_err(|error| repository_error("save refreshed steam mod info", error))?;
    tracing::info!(modid = %mod_id, "force-refreshed mod info");
    Ok(Json(legacy_success(mod_detail_data(&record))))
}
