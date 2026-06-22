//! Level HTTP handlers for `/api/cluster/level`.

use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    domain::game::level::{self, World},
    dst,
    web::app::AppState,
    web::error::{AppError, AppResult},
    web::handlers::{legacy_empty_success, legacy_success},
    web::response::LoginResponse,
};

#[derive(Debug, Deserialize)]
pub(crate) struct SaveLevelsRequest {
    levels: Vec<World>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeleteLevelQuery {
    #[serde(rename = "levelName")]
    level_name: Option<String>,
}

pub(crate) async fn list_handler(
    State(state): State<AppState>,
) -> AppResult<Json<LoginResponse<Vec<World>>>> {
    let cluster_dir =
        dst::current_cluster_dir(&state.root_path).map_err(file_error("resolve cluster"))?;
    let worlds = level::list_worlds_from_cluster_dir(&cluster_dir)?;
    tracing::debug!(count = worlds.len(), "listed DST levels");
    Ok(Json(legacy_success(worlds)))
}

pub(crate) async fn save_all_handler(
    State(state): State<AppState>,
    Json(request): Json<SaveLevelsRequest>,
) -> AppResult<Json<LoginResponse<Value>>> {
    let cluster_dir =
        dst::current_cluster_dir(&state.root_path).map_err(file_error("resolve cluster"))?;
    let count = level::save_worlds_to_cluster_dir(&state.root_path, &cluster_dir, request.levels)?;
    tracing::info!(count, "saved DST level list");
    Ok(Json(legacy_empty_success()))
}

pub(crate) async fn create_handler(
    State(state): State<AppState>,
    Json(world): Json<World>,
) -> AppResult<Json<LoginResponse<World>>> {
    let cluster_dir =
        dst::current_cluster_dir(&state.root_path).map_err(file_error("resolve cluster"))?;
    let world = level::create_world_in_cluster_dir(&cluster_dir, world)?;
    tracing::info!(level_uuid = %world.uuid, "created DST level file set");
    Ok(Json(legacy_success(world)))
}

pub(crate) async fn delete_handler(
    State(state): State<AppState>,
    Query(query): Query<DeleteLevelQuery>,
) -> AppResult<Json<LoginResponse<Value>>> {
    let level_name = query
        .level_name
        .ok_or_else(|| AppError::bad_request("levelName is required"))?;
    let cluster_dir =
        dst::current_cluster_dir(&state.root_path).map_err(file_error("resolve cluster"))?;
    level::delete_world_from_cluster_dir(&cluster_dir, &level_name)?;
    tracing::info!(level_name, "deleted DST level");
    Ok(Json(legacy_empty_success()))
}

fn file_error(
    operation: &'static str,
) -> impl FnOnce(std::io::Error) -> AppError + Copy + Send + Sync + 'static {
    move |error| {
        if error.kind() == std::io::ErrorKind::InvalidInput {
            AppError::bad_request(error.to_string())
        } else {
            tracing::error!(operation, error = %error, "level handler file operation failed");
            AppError::internal(operation)
        }
    }
}
