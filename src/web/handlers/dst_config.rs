//! DST config handlers for cluster.ini, `dst_config`, and game mod config.

use std::{io, path::Path};

use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    domain::game::mod_setup,
    dst::{self, DstConfig, cluster_ini::ClusterIni, lua_files, server_ini::ServerIni},
    validation::validate_level_name,
    web::app::AppState,
    web::error::{AppError, AppResult},
    web::handlers::{legacy_success, response_with_message},
    web::response::LoginResponse,
};

/// Request and response body used by `/api/game/8level/clusterIni`.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ClusterIniEnvelope {
    cluster: ClusterIni,
    token: String,
}

/// Go `GameConfigVO` JSON shape used by `/api/game/config`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub(crate) struct GameConfigVo {
    #[serde(rename = "clusterIntention")]
    cluster_intention: String,
    #[serde(rename = "clusterName")]
    cluster_name: String,
    #[serde(rename = "clusterDescription")]
    cluster_description: String,
    #[serde(rename = "gameMode")]
    game_mode: String,
    pvp: bool,
    #[serde(rename = "maxPlayers")]
    max_players: u64,
    #[serde(rename = "max_snapshots")]
    max_snapshots: u64,
    #[serde(rename = "clusterPassword")]
    cluster_password: String,
    token: String,
    #[serde(rename = "masterMapData")]
    master_map_data: String,
    #[serde(rename = "cavesMapData")]
    caves_map_data: String,
    #[serde(rename = "modData")]
    mod_data: String,
    #[serde(rename = "type")]
    otype: u64,
    #[serde(rename = "pause_when_nobody")]
    pause_when_nobody: bool,
    #[serde(rename = "vote_enabled")]
    vote_enabled: bool,
}

pub(crate) async fn get_cluster_ini_handler(
    State(state): State<AppState>,
) -> AppResult<Json<LoginResponse<ClusterIniEnvelope>>> {
    let cluster_dir =
        dst::current_cluster_dir(&state.root_path).map_err(file_error("resolve cluster"))?;
    let cluster = read_cluster_ini(&cluster_dir)?;
    let token = dst::safe_read_cluster_file_to_string(&cluster_dir, "cluster_token.txt")
        .map_err(file_error("read cluster token"))?
        .unwrap_or_default();
    Ok(Json(legacy_success(ClusterIniEnvelope { cluster, token })))
}

pub(crate) async fn save_cluster_ini_handler(
    State(state): State<AppState>,
    Json(request): Json<ClusterIniEnvelope>,
) -> AppResult<Json<LoginResponse<ClusterIniEnvelope>>> {
    let cluster_dir =
        dst::current_cluster_dir(&state.root_path).map_err(file_error("resolve cluster"))?;
    dst::safe_write_cluster_file(&cluster_dir, "cluster.ini", request.cluster.to_ini())
        .map_err(file_error("write cluster.ini"))?;
    dst::safe_write_cluster_file(&cluster_dir, "cluster_token.txt", &request.token)
        .map_err(file_error("write cluster token"))?;

    tracing::info!("saved DST cluster.ini and cluster_token.txt");
    Ok(Json(legacy_success(request)))
}

pub(crate) async fn get_dst_config_handler(
    State(state): State<AppState>,
) -> AppResult<Json<LoginResponse<DstConfig>>> {
    let config = DstConfig::load(&state.root_path).map_err(file_error("read dst_config"))?;
    Ok(Json(legacy_success(config)))
}

pub(crate) async fn save_dst_config_handler(
    State(state): State<AppState>,
    Json(request): Json<DstConfig>,
) -> AppResult<Json<LoginResponse<Value>>> {
    let config = request
        .save_with_fallbacks(&state.root_path)
        .map_err(file_error("write dst_config"))?;
    let cluster_dir = config.klei_root(&state.root_path).join(&config.cluster);
    dst::safe_ensure_cluster_dir(&cluster_dir).map_err(file_error("create cluster directory"))?;

    tracing::info!(cluster_name = %config.cluster, "saved dst_config without starting DST");
    Ok(Json(response_with_message(
        "save dst_config success",
        Value::Null,
    )))
}

pub(crate) async fn get_game_config_handler(
    State(state): State<AppState>,
) -> AppResult<Json<LoginResponse<GameConfigVo>>> {
    let cluster_dir =
        dst::current_cluster_dir(&state.root_path).map_err(file_error("resolve cluster"))?;
    let cluster = read_cluster_ini(&cluster_dir)?;
    let token = dst::safe_read_cluster_file_to_string(&cluster_dir, "cluster_token.txt")
        .map_err(file_error("read cluster token"))?
        .unwrap_or_default();

    Ok(Json(legacy_success(GameConfigVo {
        cluster_intention: cluster.cluster_intention,
        cluster_name: cluster.cluster_name,
        cluster_description: cluster.cluster_description,
        game_mode: cluster.game_mode,
        pvp: cluster.pvp,
        max_players: cluster.max_players,
        max_snapshots: cluster.max_snapshots,
        cluster_password: cluster.cluster_password,
        token,
        master_map_data: read_lua_or_default(
            &cluster_dir,
            Path::new("Master").join("leveldataoverride.lua"),
        )
        .map_err(file_error("read master leveldataoverride"))?,
        caves_map_data: read_lua_or_default(
            &cluster_dir,
            Path::new("Caves").join("leveldataoverride.lua"),
        )
        .map_err(file_error("read caves leveldataoverride"))?,
        mod_data: read_lua_or_default(&cluster_dir, Path::new("Master").join("modoverrides.lua"))
            .map_err(file_error("read modoverrides"))?,
        otype: 0,
        pause_when_nobody: cluster.pause_when_nobody,
        vote_enabled: cluster.vote_enabled,
    })))
}

pub(crate) async fn save_game_config_handler(
    State(state): State<AppState>,
    Json(request): Json<GameConfigVo>,
) -> AppResult<Json<LoginResponse<Value>>> {
    let cluster_dir =
        dst::current_cluster_dir(&state.root_path).map_err(file_error("resolve cluster"))?;
    if request.mod_data.is_empty() {
        tracing::info!("skipped game modoverrides save because modData is empty");
        return Ok(Json(response_with_message(
            "save dst server config success",
            Value::Null,
        )));
    }

    let level_names = game_config_level_names(&cluster_dir)?;
    for level_name in level_names {
        dst::safe_write_cluster_file(
            &cluster_dir,
            std::path::Path::new(&level_name).join("modoverrides.lua"),
            &request.mod_data,
        )
        .map_err(file_error("write modoverrides"))?;
    }
    mod_setup::write_dedicated_server_mods_setup(&state.root_path, &request.mod_data)
        .map_err(file_error("write dedicated server mods setup"))?;

    tracing::info!("saved game modoverrides config");
    Ok(Json(response_with_message(
        "save dst server config success",
        Value::Null,
    )))
}

fn game_config_level_names(cluster_dir: &std::path::Path) -> AppResult<Vec<String>> {
    let Some(contents) = dst::safe_read_cluster_file_to_string(cluster_dir, "level.json")
        .map_err(file_error("read level.json"))?
    else {
        return fallback_level_names(cluster_dir);
    };
    let value = serde_json::from_str::<serde_json::Value>(&contents).map_err(|error| {
        tracing::error!(error = %error, "failed to parse DST level.json for game config save");
        AppError::internal("parse level.json")
    })?;
    let Some(levels) = value.get("levelList").and_then(serde_json::Value::as_array) else {
        return fallback_level_names(cluster_dir);
    };

    let mut parsed_level_names = Vec::new();
    for item in levels {
        let Some(level_name) = item.get("file").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let safe_level_name = validate_level_name(level_name)
            .map_err(|error| AppError::bad_request(error.to_string()))?;
        parsed_level_names.push(safe_level_name.as_str().to_owned());
    }
    if parsed_level_names.is_empty() {
        fallback_level_names(cluster_dir)
    } else {
        Ok(parsed_level_names)
    }
}

fn fallback_level_names(cluster_dir: &std::path::Path) -> AppResult<Vec<String>> {
    let mut level_names = vec!["Master".to_owned()];
    if dst::safe_read_cluster_file_to_string(
        cluster_dir,
        std::path::Path::new("Caves").join("modoverrides.lua"),
    )
    .map_err(file_error("read caves modoverrides"))?
    .is_some()
    {
        level_names.push("Caves".to_owned());
    }
    Ok(level_names)
}

fn read_cluster_ini(cluster_dir: &Path) -> AppResult<ClusterIni> {
    let contents = dst::safe_read_cluster_file_to_string(cluster_dir, "cluster.ini")
        .map_err(file_error("read cluster.ini"))?;
    Ok(contents
        .as_deref()
        .map(ClusterIni::from_contents)
        .unwrap_or_else(ClusterIni::default_for_new_cluster))
}

fn read_lua_or_default(cluster_dir: &Path, relative_path: impl AsRef<Path>) -> io::Result<String> {
    let contents = dst::safe_read_cluster_file_to_string(cluster_dir, relative_path)?;
    Ok(lua_files::contents_or_default(contents))
}

#[allow(dead_code)]
fn default_master_world() -> ServerIni {
    ServerIni::master_default()
}

fn file_error(
    operation: &'static str,
) -> impl FnOnce(io::Error) -> AppError + Copy + Send + Sync + 'static {
    move |error| {
        if error.kind() == io::ErrorKind::InvalidInput {
            AppError::bad_request(error.to_string())
        } else {
            tracing::error!(operation, error = %error, "DST config file operation failed");
            AppError::internal(operation)
        }
    }
}
