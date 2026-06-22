//! Share-key and cluster export handlers.
//!
//! Go stores the share switch and secret in a root-relative `./key` file where
//! byte 0 is the enable flag and the remainder is the bearer key. The Rust
//! migration keeps that file contract but avoids logging the secret.

use std::path::Path;

use axum::{
    Json,
    body::Bytes,
    extract::{Query, State},
};
use getrandom::fill as fill_random;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::{
    domain::game::level,
    dst,
    infra::fs_paths::{
        safe_open_optional_existing_file_under_base, safe_overwrite_file_under_base,
    },
    web::app::AppState,
    web::error::{AppError, AppResult},
    web::handlers::{legacy_success, response_with_message},
    web::response::LoginResponse,
};

#[derive(Debug, Serialize)]
pub(crate) struct KeyCer {
    key: String,
    enable: String,
    ip: String,
    port: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ShareCluster {
    cluster_ini: String,
    cluster_token: String,
    adminlist: String,
    blocklist: String,
    whitelist: String,
    level_json: String,
    levels: Vec<level::World>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EnableQuery {
    enable: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ShareClusterQuery {
    key: String,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ImportClusterRequest {
    url: Option<String>,
}

pub(crate) async fn get_key_handler(
    State(state): State<AppState>,
) -> AppResult<Json<LoginResponse<KeyCer>>> {
    let key = read_key_info(&state)?;
    Ok(Json(legacy_success(key)))
}

pub(crate) async fn refresh_key_handler(
    State(state): State<AppState>,
) -> AppResult<Json<LoginResponse<KeyCer>>> {
    let key = generate_uuid_v4()?;
    safe_overwrite_file_under_base(&state.root_path, "key", format!("0{key}"))
        .map_err(fs_bad_request)?;
    tracing::info!("refreshed share key");
    Ok(Json(legacy_success(KeyCer {
        key,
        enable: "0".to_owned(),
        ip: public_ip_for_response(&state),
        port: state.config.port.clone(),
    })))
}

pub(crate) async fn enable_key_handler(
    State(state): State<AppState>,
    Query(query): Query<EnableQuery>,
) -> AppResult<Json<LoginResponse<KeyCer>>> {
    let mut key_info = read_key_info(&state)?;
    let enable = query.enable.unwrap_or_else(|| "0".to_owned());
    safe_overwrite_file_under_base(&state.root_path, "key", format!("{enable}{}", key_info.key))
        .map_err(fs_bad_request)?;
    key_info.enable = enable;
    tracing::info!(enabled = %key_info.enable, "updated share key enable flag");
    Ok(Json(legacy_success(key_info)))
}

pub(crate) async fn share_cluster_handler(
    State(state): State<AppState>,
    Query(query): Query<ShareClusterQuery>,
) -> AppResult<Json<LoginResponse<ShareCluster>>> {
    if !check_key(&state.root_path, &query.key)? {
        tracing::warn!("rejected share cluster request with invalid key");
        return Err(AppError::bad_request("key verification failed"));
    }
    let cluster_dir =
        dst::current_cluster_dir(&state.root_path).map_err(file_error("resolve cluster"))?;
    let cluster_ini = read_cluster_text(&cluster_dir, "cluster.ini")?;
    if cluster_ini.is_empty() {
        return Err(AppError::bad_request("cluster does not exist"));
    }
    let cluster = ShareCluster {
        cluster_ini,
        cluster_token: read_cluster_text(&cluster_dir, "cluster_token.txt")?,
        adminlist: read_cluster_text(&cluster_dir, "adminlist.txt")?,
        blocklist: read_cluster_text(&cluster_dir, "blocklist.txt")?,
        whitelist: read_cluster_text(&cluster_dir, "whitelist.txt")?,
        // Go accidentally joins `level.json` below the whitelist file path.
        // That path is normally invalid, so the public field is empty. Preserve
        // the compatibility quirk until import/export is redesigned.
        level_json: String::new(),
        levels: level::list_share_worlds_from_cluster_dir(&cluster_dir)?,
    };
    tracing::info!(
        level_count = cluster.levels.len(),
        "served shared cluster config"
    );
    Ok(Json(legacy_success(cluster)))
}

pub(crate) async fn import_cluster_handler(
    State(_state): State<AppState>,
    body: Bytes,
) -> AppResult<Json<LoginResponse<Value>>> {
    // Go currently logs the URL and returns success without importing. Keep the
    // no-op contract but avoid logging potentially secret share URLs.
    let request: ImportClusterRequest = best_effort_json_body(&body);
    tracing::info!(
        has_url = request.url.as_deref().is_some_and(|url| !url.is_empty()),
        "accepted legacy no-op cluster import"
    );
    Ok(Json(response_with_message("success", Value::Null)))
}

fn best_effort_json_body<T>(body: &Bytes) -> T
where
    T: DeserializeOwned + Default,
{
    if body.is_empty() {
        return T::default();
    }
    serde_json::from_slice(body).unwrap_or_default()
}

fn read_key_info(state: &AppState) -> AppResult<KeyCer> {
    let Some(mut file) = safe_open_optional_existing_file_under_base(&state.root_path, "key")
        .map_err(fs_bad_request)?
    else {
        safe_overwrite_file_under_base(&state.root_path, "key", "").map_err(fs_bad_request)?;
        return Ok(empty_key());
    };
    let mut contents = String::new();
    use std::io::Read;
    file.read_to_string(&mut contents)
        .map_err(file_error("read share key"))?;
    if contents.len() < 2 {
        return Ok(empty_key());
    }
    let enable = contents[..1].to_owned();
    let key = contents[1..].replace('\n', "");
    Ok(KeyCer {
        key,
        enable,
        ip: public_ip_for_response(state),
        port: state.config.port.clone(),
    })
}

fn empty_key() -> KeyCer {
    KeyCer {
        key: String::new(),
        enable: String::new(),
        ip: String::new(),
        port: String::new(),
    }
}

fn check_key(root: &Path, key: &str) -> AppResult<bool> {
    let Some(mut file) =
        safe_open_optional_existing_file_under_base(root, "key").map_err(fs_bad_request)?
    else {
        return Ok(false);
    };
    let mut contents = String::new();
    use std::io::Read;
    file.read_to_string(&mut contents)
        .map_err(file_error("read share key"))?;
    if contents.len() < 2 || &contents[..1] == "0" {
        return Ok(false);
    }
    Ok(contents[1..].replace('\n', "") == key)
}

fn read_cluster_text(cluster_dir: &Path, relative_path: &str) -> AppResult<String> {
    Ok(
        dst::safe_read_cluster_file_to_string(cluster_dir, relative_path)
            .map_err(file_error("read share cluster file"))?
            .unwrap_or_default(),
    )
}

fn public_ip_for_response(state: &AppState) -> String {
    state.config.wan_ip.clone()
}

fn generate_uuid_v4() -> AppResult<String> {
    let mut uuid = [0_u8; 16];
    fill_random(&mut uuid).map_err(|_| AppError::internal("generate share key"))?;
    uuid[6] = (uuid[6] & 0x0f) | 0x40;
    uuid[8] = (uuid[8] & 0x3f) | 0x80;
    Ok(format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        uuid[0],
        uuid[1],
        uuid[2],
        uuid[3],
        uuid[4],
        uuid[5],
        uuid[6],
        uuid[7],
        uuid[8],
        uuid[9],
        uuid[10],
        uuid[11],
        uuid[12],
        uuid[13],
        uuid[14],
        uuid[15]
    ))
}

fn fs_bad_request(error: impl std::fmt::Display) -> AppError {
    tracing::warn!(error = %error, "rejected unsafe share path");
    AppError::bad_request(error.to_string())
}

fn file_error(operation: &'static str) -> impl FnOnce(std::io::Error) -> AppError {
    move |error| {
        tracing::error!(operation, error = %error, "share filesystem operation failed");
        if error.kind() == std::io::ErrorKind::InvalidInput {
            AppError::bad_request(error.to_string())
        } else {
            AppError::internal(operation)
        }
    }
}
