//! Auto-check compatibility handlers for `/api/auto/check2`.
//!
//! The Go API synthesizes rows from `level.json` on every GET, then overlays a
//! small subset of persisted settings from `auto_checks`. It intentionally
//! does not filter those overlay queries by cluster, so this implementation
//! preserves that quirk for compatibility.

use std::{io, path::Path};

use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};

use crate::{
    domain::scheduler::model::{AutoCheckRecord, SaveAutoCheck},
    domain::scheduler::repository::auto_check::AutoCheckRepository,
    dst::{self, DstConfig},
    web::app::AppState,
    web::error::{AppError, AppResult},
    web::handlers::{legacy_success, repository_error},
    web::response::LoginResponse,
};

const LEVEL_MOD: &str = "LEVEL_MOD";
const LEVEL_DOWN: &str = "LEVEL_DOWN";
const UPDATE_GAME: &str = "UPDATE_GAME";

/// Query parameters accepted by the auto-check list endpoint.
#[derive(Debug, Deserialize)]
pub(crate) struct AutoCheckQuery {
    #[serde(rename = "checkType", default)]
    check_type: String,
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct LevelIndex {
    #[serde(rename = "levelList", default)]
    level_list: Vec<LevelIndexItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct LevelIndexItem {
    name: String,
    file: String,
}

/// Lists generated auto-check rows with persisted settings overlaid.
pub(crate) async fn list_handler(
    State(state): State<AppState>,
    Query(query): Query<AutoCheckQuery>,
) -> AppResult<Json<LoginResponse<Vec<AutoCheckRecord>>>> {
    let config = DstConfig::load(&state.root_path).map_err(file_error("load dst_config"))?;
    let levels = load_level_index(&state.root_path, &config)?;
    let (mut generated, uuid_set) =
        generated_auto_checks(&config.cluster, &levels, &query.check_type);

    let repository = AutoCheckRepository::new(state.db);
    let persisted = if query.check_type.is_empty() {
        repository.list_by_uuids(&uuid_set).await
    } else if query.check_type == UPDATE_GAME {
        repository.list_by_check_type(&query.check_type).await
    } else {
        repository
            .list_by_check_type_and_uuids(&query.check_type, &uuid_set)
            .await
    }
    .map_err(|error| repository_error("list auto-check settings", error))?;

    overlay_auto_checks(&mut generated, &persisted);
    tracing::debug!(
        check_type = %query.check_type,
        generated = generated.len(),
        persisted = persisted.len(),
        "listed auto-check settings"
    );
    Ok(Json(legacy_success(generated)))
}

/// Saves an auto-check row and returns the saved model, matching Go `SaveAutoCheck2`.
pub(crate) async fn save_handler(
    State(state): State<AppState>,
    Json(request): Json<SaveAutoCheck>,
) -> AppResult<Json<LoginResponse<AutoCheckRecord>>> {
    let repository = AutoCheckRepository::new(state.db);
    let saved = repository
        .save(request)
        .await
        .map_err(|error| repository_error("save auto-check setting", error))?;
    tracing::info!(
        id = saved.id,
        cluster_name = %saved.cluster_name,
        check_type = %saved.check_type,
        "saved auto-check setting"
    );
    Ok(Json(legacy_success(saved)))
}

fn generated_auto_checks(
    cluster_name: &str,
    levels: &[LevelIndexItem],
    check_type: &str,
) -> (Vec<AutoCheckRecord>, Vec<String>) {
    let mut uuid_set = Vec::new();
    let mut result = Vec::new();

    match check_type {
        "" => {
            for level in levels {
                uuid_set.push(level.file.clone());
                result.push(AutoCheckRecord::generated(
                    cluster_name.to_owned(),
                    level.name.clone(),
                    level.file.clone(),
                    LEVEL_MOD,
                ));
                result.push(AutoCheckRecord::generated(
                    cluster_name.to_owned(),
                    level.name.clone(),
                    level.file.clone(),
                    LEVEL_DOWN,
                ));
            }
            result.push(AutoCheckRecord::generated(
                cluster_name.to_owned(),
                cluster_name.to_owned(),
                String::new(),
                UPDATE_GAME,
            ));
        }
        LEVEL_DOWN | LEVEL_MOD => {
            for level in levels {
                uuid_set.push(level.file.clone());
                result.push(AutoCheckRecord::generated(
                    cluster_name.to_owned(),
                    level.name.clone(),
                    level.file.clone(),
                    check_type,
                ));
            }
        }
        _ => {
            result.push(AutoCheckRecord::generated(
                cluster_name.to_owned(),
                String::new(),
                format!("{UPDATE_GAME}_{cluster_name}"),
                UPDATE_GAME,
            ));
        }
    }

    (result, uuid_set)
}

fn overlay_auto_checks(generated: &mut [AutoCheckRecord], persisted: &[AutoCheckRecord]) {
    for row in generated {
        for persisted_row in persisted {
            if row.uuid == persisted_row.uuid && row.check_type == persisted_row.check_type {
                row.overlay_persisted_settings(persisted_row);
            }
        }
    }
}

fn load_level_index(root: &Path, config: &DstConfig) -> AppResult<Vec<LevelIndexItem>> {
    let cluster_dir =
        dst::cluster_dir(root, &config.cluster).map_err(file_error("resolve cluster"))?;
    dst::safe_ensure_cluster_dir(&cluster_dir).map_err(file_error("ensure cluster directory"))?;
    let contents = match dst::safe_read_cluster_file_to_string(&cluster_dir, "level.json")
        .map_err(file_error("read level.json"))?
    {
        Some(contents) => contents,
        None => {
            tracing::info!(
                cluster_name = %config.cluster,
                "created missing level.json for auto-check compatibility"
            );
            dst::safe_write_cluster_file(&cluster_dir, "level.json", "{}")
                .map_err(file_error("write missing level.json"))?;
            "{}".to_owned()
        }
    };

    let mut index = serde_json::from_str::<LevelIndex>(&contents).map_err(|error| {
        tracing::error!(error = %error, "failed to parse level.json for auto-check rows");
        AppError::internal("parse level.json")
    })?;
    if index.level_list.is_empty() {
        index.level_list =
            default_level_index_items(&cluster_dir).map_err(file_error("initialize levels"))?;
        save_level_index(&cluster_dir, &index)?;
    };

    Ok(index.level_list)
}

fn default_level_index_items(cluster_dir: &Path) -> io::Result<Vec<LevelIndexItem>> {
    if !dst::safe_optional_cluster_dir_exists(cluster_dir, "Master")? {
        write_default_master_level(cluster_dir)?;
        return Ok(vec![LevelIndexItem {
            name: "森林".to_owned(),
            file: "Master".to_owned(),
        }]);
    }

    let mut levels = vec![LevelIndexItem {
        name: "森林".to_owned(),
        file: "Master".to_owned(),
    }];
    if dst::safe_optional_cluster_dir_exists(cluster_dir, "Caves")? {
        levels.push(LevelIndexItem {
            name: "洞穴".to_owned(),
            file: "Caves".to_owned(),
        });
    }
    Ok(levels)
}

fn write_default_master_level(cluster_dir: &Path) -> io::Result<()> {
    dst::safe_write_cluster_file(cluster_dir, "Master/leveldataoverride.lua", "return {}")?;
    dst::safe_write_cluster_file(cluster_dir, "Master/modoverrides.lua", "return {}")?;
    dst::safe_write_cluster_file(
        cluster_dir,
        "Master/server.ini",
        "[NETWORK]\nserver_port = 10999\n[SHARD]\nis_master = true\nname = Master\nid = 10000\n[ACCOUNT]\nencode_user_path = true\n",
    )
}

fn save_level_index(cluster_dir: &Path, index: &LevelIndex) -> AppResult<()> {
    let contents =
        serde_json::to_string(index).map_err(|_| AppError::internal("encode level.json"))?;
    dst::safe_write_cluster_file(cluster_dir, "level.json", contents)
        .map_err(file_error("save level.json"))?;
    tracing::info!(
        levels = index.level_list.len(),
        "saved initialized level.json for auto-check compatibility"
    );
    Ok(())
}

fn file_error(
    operation: &'static str,
) -> impl FnOnce(io::Error) -> AppError + Copy + Send + Sync + 'static {
    move |error| {
        if error.kind() == io::ErrorKind::InvalidInput {
            AppError::bad_request(error.to_string())
        } else {
            tracing::error!(operation, error = %error, "auto-check file operation failed");
            AppError::internal(operation)
        }
    }
}
