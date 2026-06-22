//! Initialization handlers for first-run compatibility.
//!
//! The Go GET endpoint reports whether `./first` exists and may perform
//! additional world initialization as a side effect. The Rust GET handler stays
//! read-only, while POST mirrors the first-run writes to `password.txt`,
//! `first`, and the base `MyDediServer` cluster skeleton.

use std::{
    fs,
    path::{Path, PathBuf},
};

use axum::{Json, extract::State};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    dst::{self, cluster_ini::ClusterIni},
    infra::fs_paths,
    web::app::AppState,
    web::error::{AppError, AppResult},
    web::handlers::legacy_empty_success,
    web::response::LoginResponse,
};

pub(crate) const DEFAULT_FIRST_RUN_CLUSTER: &str = "MyDediServer";
const MASTER_LEVELDATAOVERRIDE: &str = include_str!("../../../static/Master/leveldataoverride.lua");
const MASTER_MODOVERRIDES: &str = include_str!("../../../static/Master/modoverrides.lua");
const MASTER_SERVER_INI: &str = include_str!("../../../static/Master/server.ini");
const CAVES_LEVELDATAOVERRIDE: &str = include_str!("../../../static/Caves/leveldataoverride.lua");
const CAVES_MODOVERRIDES: &str = include_str!("../../../static/Caves/modoverrides.lua");
const CAVES_SERVER_INI: &str = include_str!("../../../static/Caves/server.ini");

/// Request body accepted by Go's `InitDstData`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InitFirstRequest {
    /// The Go handler accepts `dstConfig` but `InitDstEnv` does not persist it.
    /// Rust intentionally ignores unknown fields, preserving that behavior.
    #[serde(default)]
    user_info: InitUserInfo,
}

/// First-run user info payload.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InitUserInfo {
    #[serde(default)]
    username: String,
    #[serde(default)]
    password: String,
    #[serde(default)]
    display_name: String,
    #[serde(default, rename = "photoURL")]
    photo_url: String,
}

/// Returns Go-compatible first-run status based on the root `first` marker file.
pub(crate) async fn check_first_handler(
    State(state): State<AppState>,
) -> Json<LoginResponse<Value>> {
    let first_path = state.root_path.join("first");
    if path_entry_exists(&first_path) {
        tracing::info!(
            first_path = %first_path.display(),
            "checked first-run marker and found existing installation"
        );
        return Json(LoginResponse {
            code: 400,
            msg: "is not first".to_owned(),
            data: Value::Null,
        });
    }

    tracing::info!(
        first_path = %first_path.display(),
        "checked first-run marker; base-level initialization intentionally skipped"
    );
    Json(LoginResponse {
        code: 200,
        msg: "is first".to_owned(),
        data: Value::Null,
    })
}

/// Performs Go-compatible first-run initialization.
///
/// The Go implementation panics when `./first` already exists, which the Gin
/// recovery middleware turns into HTTP 500. Returning an internal error keeps
/// the wire behavior while avoiding an actual Rust panic.
pub(crate) async fn init_first_handler(
    State(state): State<AppState>,
    Json(request): Json<InitFirstRequest>,
) -> AppResult<Response> {
    let first_path = state.root_path.join("first");
    if path_entry_exists(&first_path) {
        tracing::warn!("rejected first-run initialization because marker already exists");
        return Ok(legacy_recover_response("非法请求\n"));
    }

    fs::create_dir_all(&state.root_path)?;
    let _lock = match acquire_init_lock(&state.root_path) {
        Ok(lock) => lock,
        Err(error) => {
            tracing::warn!(error = %error, "rejected first-run initialization because lock is unavailable");
            return Ok(legacy_recover_response("非法请求\n"));
        }
    };
    write_initial_password_file(&state, &request.user_info)?;
    initialize_default_cluster(&state.root_path, Some(&request.user_info.display_name))?;
    fs_paths::safe_create_new_file_under_base(&state.root_path, "first")
        .map_err(|error| AppError::internal(format!("create first marker: {error}")))?;

    tracing::info!(
        first_path = %first_path.display(),
        cluster_name = DEFAULT_FIRST_RUN_CLUSTER,
        "completed first-run initialization"
    );
    Ok(Json(legacy_empty_success()).into_response())
}

fn write_initial_password_file(state: &AppState, user_info: &InitUserInfo) -> AppResult<()> {
    let contents = format!(
        "username={}\npassword={}\ndisplayName={}\nphotoURL={}\n",
        user_info.username, user_info.password, user_info.display_name, user_info.photo_url
    );
    fs_paths::safe_overwrite_file_under_base(&state.root_path, "password.txt", contents)
        .map_err(|error| AppError::internal(format!("write password file: {error}")))?;
    tracing::info!(
        has_username = !user_info.username.is_empty(),
        has_display_name = !user_info.display_name.is_empty(),
        "wrote initial user info"
    );
    Ok(())
}

/// Initializes the first-run cluster using Go's static Master/Caves templates.
pub(crate) fn initialize_default_cluster(
    root_path: &Path,
    display_name: Option<&str>,
) -> AppResult<()> {
    let cluster_dir = dst::cluster_dir(root_path, DEFAULT_FIRST_RUN_CLUSTER)?;
    if cluster_dir.exists() {
        tracing::info!(
            cluster_name = DEFAULT_FIRST_RUN_CLUSTER,
            "first-run cluster directory already exists; preserving Go early-return behavior"
        );
        return Ok(());
    }

    dst::init_cluster_files(root_path, DEFAULT_FIRST_RUN_CLUSTER, "")?;
    let mut cluster_ini = ClusterIni::default_for_new_cluster();
    if let Some(display_name) = display_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        cluster_ini.cluster_name = format!("{display_name}的世界");
    }
    dst::safe_write_cluster_file(&cluster_dir, "cluster.ini", cluster_ini.to_ini())?;
    write_static_world_templates(&cluster_dir)?;
    tracing::info!(
        cluster_name = DEFAULT_FIRST_RUN_CLUSTER,
        "initialized first-run cluster skeleton"
    );
    Ok(())
}

fn write_static_world_templates(cluster_dir: &Path) -> AppResult<()> {
    // Go copies these files from ./static/Master and ./static/Caves. Embedding
    // the same templates keeps release binaries self-contained and prevents
    // local edits to runtime static files from changing first-run output.
    dst::safe_write_cluster_file(
        cluster_dir,
        "Master/leveldataoverride.lua",
        MASTER_LEVELDATAOVERRIDE,
    )?;
    dst::safe_write_cluster_file(cluster_dir, "Master/modoverrides.lua", MASTER_MODOVERRIDES)?;
    dst::safe_write_cluster_file(cluster_dir, "Master/server.ini", MASTER_SERVER_INI)?;
    dst::safe_write_cluster_file(
        cluster_dir,
        "Caves/leveldataoverride.lua",
        CAVES_LEVELDATAOVERRIDE,
    )?;
    dst::safe_write_cluster_file(cluster_dir, "Caves/modoverrides.lua", CAVES_MODOVERRIDES)?;
    dst::safe_write_cluster_file(cluster_dir, "Caves/server.ini", CAVES_SERVER_INI)?;
    Ok(())
}

struct InitLock {
    root_path: PathBuf,
}

impl Drop for InitLock {
    fn drop(&mut self) {
        if let Err(error) = fs_paths::safe_remove_file_under_base(&self.root_path, "first.lock") {
            tracing::warn!(error = %error, "failed to remove first-run initialization lock");
        }
    }
}

fn acquire_init_lock(root_path: &Path) -> Result<InitLock, String> {
    fs_paths::safe_create_new_file_under_base(root_path, "first.lock")
        .map(|_| InitLock {
            root_path: root_path.to_path_buf(),
        })
        .map_err(|error| error.to_string())
}

fn legacy_recover_response(message: &str) -> Response {
    (
        StatusCode::OK,
        Json(json!({
            "code": "500",
            "msg": message,
            "data": Value::Null,
        })),
    )
        .into_response()
}

pub(crate) fn path_entry_exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}
