//! Shared mod metadata service helpers.

use serde_json::{Value, json};

use crate::{
    domain::mods::{
        model::{ModInfoInput, ModInfoRecord},
        repository::ModInfoRepository,
    },
    dst::DstConfig,
    web::{
        app::AppState,
        error::{AppError, AppResult},
        handlers::repository_error,
    },
};

use super::{
    file_ops::{
        STEAMCMD_MOD_DOWNLOAD_FAILED, mod_config_from_detail_source,
        mod_config_from_existing_files, mod_config_from_existing_or_steamcmd,
    },
    lua_config::parse_mod_config,
    steam_api::{
        STEAM_DETAIL_LANG, SteamDetail, author_url, fetch_steam_details, image_with_suffix,
        version_from_tags,
    },
};

pub(super) async fn subscribe_mod_by_id(
    state: &AppState,
    mod_id: &str,
    lang: &str,
) -> AppResult<ModInfoRecord> {
    let repository = ModInfoRepository::new(state.db.clone());
    if mod_id.parse::<i64>().is_err() {
        let config = DstConfig::load(&state.root_path)?;
        let mod_config = mod_config_from_existing_files(&state.root_path, &config, lang, mod_id)?;
        let record = local_mod_record(mod_id, mod_config);
        return repository
            .create(ModInfoInput::from(&record))
            .await
            .map_err(|error| repository_error("create local mod info", error));
    }

    let mut details = match fetch_steam_details(state, &[mod_id], STEAM_DETAIL_LANG).await {
        Ok(details) => details,
        Err(AppError::BadRequest(message)) if message == "steam API key is not configured" => {
            let config = DstConfig::load(&state.root_path)?;
            let mod_config = recover_subscribe_mod_config(
                mod_config_from_existing_or_steamcmd(state, &config, lang, mod_id).await,
                mod_id,
            )?;
            let record = local_mod_record(mod_id, mod_config);
            return repository
                .upsert_by_modid(ModInfoInput::from(&record))
                .await
                .map_err(|error| repository_error("save steamcmd-only mod info", error));
        }
        Err(error) => return Err(error),
    };
    let Some(detail) = details.pop() else {
        return Err(AppError::bad_request("steam mod details not found"));
    };
    if let Some(existing) = repository
        .find_active_by_modid(mod_id)
        .await
        .map_err(|error| repository_error("find mod info before subscribe", error))?
        && (existing.last_time - detail.time_updated).abs() < f64::EPSILON
    {
        return Ok(existing);
    }

    let config = DstConfig::load(&state.root_path)?;
    let mod_config = recover_subscribe_mod_config(
        mod_config_from_detail_source(state, &config, lang, mod_id, &detail).await,
        mod_id,
    )?;
    let record = mod_record_from_detail(&detail, mod_config);
    repository
        .upsert_by_modid(ModInfoInput::from(&record))
        .await
        .map_err(|error| repository_error("save steam mod info", error))
}

fn recover_subscribe_mod_config(result: AppResult<String>, mod_id: &str) -> AppResult<String> {
    match result {
        Ok(mod_config) => Ok(mod_config),
        Err(AppError::BadRequest(message)) if message == STEAMCMD_MOD_DOWNLOAD_FAILED => {
            tracing::warn!(
                modid = %mod_id,
                "SteamCMD failed during initial subscribe; saving metadata with empty mod_config"
            );
            Ok("{}".to_owned())
        }
        Err(error) => Err(error),
    }
}

pub(super) fn local_mod_record(mod_id: &str, mod_config: String) -> ModInfoRecord {
    ModInfoRecord {
        modid: mod_id.to_owned(),
        img: "xxx".to_owned(),
        name: mod_id.to_owned(),
        mod_config,
        ..ModInfoRecord::zero()
    }
}

pub(super) fn mod_record_from_detail(detail: &SteamDetail, mod_config: String) -> ModInfoRecord {
    ModInfoRecord {
        auth: author_url(&detail.creator),
        consumer_appid: detail.consumer_appid,
        creator_appid: detail.creator_appid,
        description: detail.file_description.clone(),
        file_url: detail.file_url.clone().unwrap_or_default(),
        modid: detail.publishedfileid.clone(),
        img: image_with_suffix(&detail.preview_url),
        last_time: detail.time_updated,
        mod_config,
        name: detail.title.clone(),
        v: version_from_tags(&detail.views),
        ..ModInfoRecord::zero()
    }
}
pub(super) fn mod_detail_data(record: &ModInfoRecord) -> Value {
    json!({
        "auth": record.auth,
        "consumer_id": record.consumer_appid,
        "creator_appid": record.creator_appid,
        "description": record.description,
        "file_url": record.file_url,
        "modid": record.modid,
        "img": record.img,
        "last_time": record.last_time,
        "name": record.name,
        "v": record.v,
        "mod_config": parse_mod_config(&record.mod_config),
        "update": record.update_available,
    })
}
