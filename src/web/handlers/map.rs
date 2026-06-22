//! HTTP handlers for DST map generation and session-file inspection.
//!
//! The response bodies intentionally use Go's `vo.Response` envelope
//! (`code: 200`, `msg: "success"`, `data: null`) instead of the newer
//! zero-code envelope used by other routes in this panel.

use std::io;

use axum::{
    Json,
    body::Body,
    extract::{Query, State},
    http::{
        HeaderValue, StatusCode,
        header::{CONTENT_LENGTH, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    domain::map::{self as dst_map, MapKuId, MapLevel},
    dst,
    web::app::AppState,
    web::handlers::{legacy_empty_success, legacy_success},
    web::response::LoginResponse,
};

#[derive(Debug, Deserialize)]
pub(crate) struct LevelQuery {
    #[serde(rename = "levelName")]
    level_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PlayerSessionQuery {
    #[serde(rename = "levelName")]
    level_name: Option<String>,
    #[serde(rename = "kuId")]
    ku_id: Option<String>,
}

pub(crate) async fn generate_handler(
    State(state): State<AppState>,
    Query(query): Query<LevelQuery>,
) -> Response {
    let level = match required_level(query.level_name) {
        Ok(level) => level,
        Err(message) => return legacy_bad_request_response(message),
    };
    let cluster_dir = match current_cluster_dir(&state) {
        Ok(cluster_dir) => cluster_dir,
        Err(error) => return file_error_response("resolve cluster directory", error),
    };
    if let Err(error) = dst_map::generate_map_image(&cluster_dir, &level) {
        return file_error_response("generate map image", error);
    }
    tracing::info!(
        level_name = level.as_str(),
        "served DST map generation request"
    );
    Json(legacy_empty_success()).into_response()
}

pub(crate) async fn image_handler(
    State(state): State<AppState>,
    Query(query): Query<LevelQuery>,
) -> Response {
    let level = match required_level(query.level_name) {
        Ok(level) => level,
        Err(message) => return legacy_bad_request_response(message),
    };
    let cluster_dir = match current_cluster_dir(&state) {
        Ok(cluster_dir) => cluster_dir,
        Err(error) => return file_error_response("resolve cluster directory", error),
    };
    let bytes = match dst_map::read_map_image(&cluster_dir, &level) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => {
            tracing::debug!(
                level_name = level.as_str(),
                "DST map image is missing at legacy jpg path"
            );
            return (StatusCode::NOT_FOUND, "404 page not found").into_response();
        }
        Err(error) => return file_error_response("read map image", error),
    };

    if bytes.is_empty() {
        tracing::debug!(level_name = level.as_str(), "served empty DST map image");
    }

    let len = bytes.len();
    let mut response = (StatusCode::OK, Body::from(bytes)).into_response();
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("image/png"));
    response.headers_mut().insert(
        CONTENT_LENGTH,
        HeaderValue::from_str(&len.to_string())
            .expect("decimal content length is a valid header value"),
    );
    tracing::debug!(
        level_name = level.as_str(),
        bytes = len,
        "served DST map image"
    );
    response
}

pub(crate) async fn has_walrus_hut_plains_handler(
    State(state): State<AppState>,
    Query(query): Query<LevelQuery>,
) -> Response {
    let level = match required_level(query.level_name) {
        Ok(level) => level,
        Err(message) => return legacy_bad_request_response(message),
    };
    let cluster_dir = match current_cluster_dir(&state) {
        Ok(cluster_dir) => cluster_dir,
        Err(error) => return file_error_response("resolve cluster directory", error),
    };
    let has_marker = match dst_map::latest_session_has_walrus_hut_plains(&cluster_dir, &level) {
        Ok(has_marker) => has_marker,
        Err(error) => return file_error_response("read latest session file", error),
    };
    tracing::debug!(
        level_name = level.as_str(),
        has_walrus_hut_plains = has_marker,
        "checked DST session map marker"
    );
    Json(legacy_success(has_marker)).into_response()
}

pub(crate) async fn session_file_handler(
    State(state): State<AppState>,
    Query(query): Query<LevelQuery>,
) -> Response {
    let level = match required_level(query.level_name) {
        Ok(level) => level,
        Err(message) => return legacy_bad_request_response(message),
    };
    let cluster_dir = match current_cluster_dir(&state) {
        Ok(cluster_dir) => cluster_dir,
        Err(error) => return file_error_response("resolve cluster directory", error),
    };
    let contents = match dst_map::read_latest_session_file(&cluster_dir, &level) {
        Ok(contents) => contents,
        Err(error) => return file_error_response("read latest session file", error),
    };
    tracing::debug!(
        level_name = level.as_str(),
        bytes = contents.len(),
        "served latest DST world session file"
    );
    Json(legacy_success(contents)).into_response()
}

pub(crate) async fn player_session_file_handler(
    State(state): State<AppState>,
    Query(query): Query<PlayerSessionQuery>,
) -> Response {
    let (level, ku_id) = match required_level_and_ku_id(query) {
        Ok(values) => values,
        Err(message) => return legacy_bad_request_response(message),
    };
    let cluster_dir = match current_cluster_dir(&state) {
        Ok(cluster_dir) => cluster_dir,
        Err(error) => return file_error_response("resolve cluster directory", error),
    };
    let contents = match dst_map::read_latest_player_session_file(&cluster_dir, &level, &ku_id) {
        Ok(contents) => contents,
        Err(error) => return file_error_response("read latest player session file", error),
    };
    tracing::debug!(
        level_name = level.as_str(),
        bytes = contents.len(),
        "served latest DST player session file"
    );
    Json(legacy_success(contents)).into_response()
}

fn required_level(level_name: Option<String>) -> Result<MapLevel, String> {
    let Some(level_name) = level_name.filter(|value| !value.is_empty()) else {
        return Err("levelName 参数不能为空".to_owned());
    };
    MapLevel::parse(&level_name).map_err(|error| error.to_string())
}

fn required_level_and_ku_id(query: PlayerSessionQuery) -> Result<(MapLevel, MapKuId), String> {
    let Some(level_name) = query.level_name.filter(|value| !value.is_empty()) else {
        return Err("levelName or kuId 参数不能为空".to_owned());
    };
    let Some(ku_id) = query.ku_id.filter(|value| !value.is_empty()) else {
        return Err("levelName or kuId 参数不能为空".to_owned());
    };
    let level = MapLevel::parse(&level_name).map_err(|error| error.to_string())?;
    let ku_id = MapKuId::parse(&ku_id).map_err(|error| error.to_string())?;
    Ok((level, ku_id))
}

fn current_cluster_dir(state: &AppState) -> io::Result<std::path::PathBuf> {
    dst::current_cluster_dir(&state.root_path)
}

fn legacy_bad_request_response(message: impl Into<String>) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(LoginResponse::<Value>::error(400, message.into())),
    )
        .into_response()
}

fn file_error_response(operation: &'static str, error: io::Error) -> Response {
    if matches!(
        error.kind(),
        io::ErrorKind::InvalidInput | io::ErrorKind::InvalidData | io::ErrorKind::NotFound
    ) {
        tracing::warn!(operation, error = %error, "map handler rejected request");
        return (
            StatusCode::BAD_REQUEST,
            Json(LoginResponse::<Value>::error(400, error.to_string())),
        )
            .into_response();
    }

    tracing::error!(operation, error = %error, "map handler file operation failed");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(LoginResponse::<Value>::error(500, operation)),
    )
        .into_response()
}
