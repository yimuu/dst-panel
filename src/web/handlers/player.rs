//! Player list and online-player handlers backed by DST files and console logs.

use std::io;

use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    domain::game::player_query::{self, PlayerVo},
    dst::{self, player_lists},
    web::app::AppState,
    web::error::{AppError, AppResult},
    web::handlers::{legacy_empty_success, legacy_success},
    web::response::LoginResponse,
};

#[derive(Debug, Deserialize)]
pub(crate) struct PlayersQuery {
    #[serde(rename = "levelName")]
    level_name: Option<String>,
}

pub(crate) async fn online_players_handler(
    State(state): State<AppState>,
    Query(query): Query<PlayersQuery>,
) -> AppResult<Json<LoginResponse<Vec<PlayerVo>>>> {
    let Some(level_name) = query
        .level_name
        .as_deref()
        .filter(|value| !value.is_empty())
    else {
        tracing::debug!("online player query omitted levelName; returning empty list");
        return Ok(Json(legacy_success(Vec::new())));
    };
    let players = query_players(&state, level_name).await?;
    Ok(Json(legacy_success(players)))
}

pub(crate) async fn all_online_players_handler(
    State(state): State<AppState>,
) -> AppResult<Json<LoginResponse<Vec<PlayerVo>>>> {
    let players = query_players(&state, "#ALL_LEVEL").await?;
    Ok(Json(legacy_success(players)))
}

pub(crate) async fn master_online_players_handler(
    State(state): State<AppState>,
) -> AppResult<Json<LoginResponse<Vec<PlayerVo>>>> {
    let players = query_players(&state, "Master").await?;
    Ok(Json(legacy_success(players)))
}

pub(crate) async fn get_adminlist_handler(
    State(state): State<AppState>,
) -> AppResult<Json<LoginResponse<Vec<String>>>> {
    read_list(&state.root_path, "adminlist.txt")
}

pub(crate) async fn overwrite_adminlist_handler(
    State(state): State<AppState>,
    Json(request): Json<player_lists::AdminListRequest>,
) -> AppResult<Json<LoginResponse<Value>>> {
    overwrite_list(&state.root_path, "adminlist.txt", &request.admin_list)
}

pub(crate) async fn append_adminlist_handler(
    State(state): State<AppState>,
    Json(request): Json<player_lists::AdminListRequest>,
) -> AppResult<Json<LoginResponse<Value>>> {
    let cluster_dir = current_cluster_dir(&state.root_path)?;
    player_lists::append_unique_in_cluster(&cluster_dir, "adminlist.txt", &request.admin_list)
        .map_err(file_error("append adminlist"))?;
    Ok(Json(legacy_empty_success()))
}

pub(crate) async fn delete_adminlist_handler(
    State(state): State<AppState>,
    Json(request): Json<player_lists::AdminListRequest>,
) -> AppResult<Json<LoginResponse<Value>>> {
    let cluster_dir = current_cluster_dir(&state.root_path)?;
    player_lists::remove_values_in_cluster(&cluster_dir, "adminlist.txt", &request.admin_list)
        .map_err(file_error("delete adminlist"))?;
    Ok(Json(legacy_empty_success()))
}

pub(crate) async fn get_whitelist_handler(
    State(state): State<AppState>,
) -> AppResult<Json<LoginResponse<Vec<String>>>> {
    read_list(&state.root_path, "whitelist.txt")
}

pub(crate) async fn overwrite_whitelist_handler(
    State(state): State<AppState>,
    Json(request): Json<player_lists::WhitelistRequest>,
) -> AppResult<Json<LoginResponse<Value>>> {
    overwrite_list(&state.root_path, "whitelist.txt", &request.whitelist)
}

pub(crate) async fn get_blacklist_handler(
    State(state): State<AppState>,
) -> AppResult<Json<LoginResponse<Vec<String>>>> {
    read_list(&state.root_path, "blocklist.txt")
}

pub(crate) async fn overwrite_blacklist_handler(
    State(state): State<AppState>,
    Json(request): Json<player_lists::BlacklistRequest>,
) -> AppResult<Json<LoginResponse<Value>>> {
    overwrite_list(&state.root_path, "blocklist.txt", &request.blacklist)
}

pub(crate) async fn append_blacklist_handler(
    State(state): State<AppState>,
    Json(request): Json<player_lists::BlacklistRequest>,
) -> AppResult<Json<LoginResponse<Value>>> {
    let cluster_dir = current_cluster_dir(&state.root_path)?;
    player_lists::append_unique_in_cluster(&cluster_dir, "blocklist.txt", &request.blacklist)
        .map_err(file_error("append blacklist"))?;
    Ok(Json(legacy_empty_success()))
}

pub(crate) async fn delete_blacklist_handler(
    State(state): State<AppState>,
    Json(request): Json<player_lists::BlacklistRequest>,
) -> AppResult<Json<LoginResponse<Value>>> {
    let cluster_dir = current_cluster_dir(&state.root_path)?;
    player_lists::remove_values_in_cluster(&cluster_dir, "blocklist.txt", &request.blacklist)
        .map_err(file_error("delete blacklist"))?;
    Ok(Json(legacy_empty_success()))
}

fn read_list(
    root: &std::path::Path,
    file_name: &str,
) -> AppResult<Json<LoginResponse<Vec<String>>>> {
    let cluster_dir = current_cluster_dir(root)?;
    let values = player_lists::read_in_cluster(&cluster_dir, file_name)
        .map_err(file_error("read player list"))?;
    Ok(Json(legacy_success(values)))
}

fn overwrite_list(
    root: &std::path::Path,
    file_name: &str,
    values: &[String],
) -> AppResult<Json<LoginResponse<Value>>> {
    let cluster_dir = current_cluster_dir(root)?;
    player_lists::overwrite_in_cluster(&cluster_dir, file_name, values)
        .map_err(file_error("overwrite player list"))?;
    Ok(Json(legacy_empty_success()))
}

fn current_cluster_dir(root: &std::path::Path) -> AppResult<std::path::PathBuf> {
    dst::current_cluster_dir(root).map_err(file_error("resolve cluster"))
}

async fn query_players(state: &AppState, level_name: &str) -> AppResult<Vec<PlayerVo>> {
    player_query::query_online_players(
        &state.root_path,
        state.command_runner.as_ref(),
        state.process_snapshot_provider.as_ref(),
        level_name,
        state.player_query_delay,
        state.player_query_marker_override.as_deref(),
    )
    .await
}

fn file_error(
    operation: &'static str,
) -> impl FnOnce(io::Error) -> AppError + Copy + Send + Sync + 'static {
    move |error| {
        if error.kind() == io::ErrorKind::InvalidInput {
            AppError::bad_request(error.to_string())
        } else {
            tracing::error!(operation, error = %error, "player list operation failed");
            AppError::internal(operation)
        }
    }
}
