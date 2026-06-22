//! Backup, restore, snapshot, and archive HTTP handlers.
//!
//! The Go service exposes backup operations as filesystem side effects with a
//! stable response envelope. This module keeps that envelope while routing all
//! file names through descriptor-anchored path helpers in [`crate::domain::backup`].

use std::{
    fs,
    io::{self, Read},
    net::{IpAddr, UdpSocket},
    path::Path,
};

use axum::{
    Json,
    body::{Body, Bytes},
    extract::{Multipart, Query, State},
    http::{
        HeaderMap, HeaderName,
        header::{CONTENT_DISPOSITION, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
};
use futures_util::stream;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::{
    domain::backup,
    domain::backup::model::{BackupSnapshotRecord, SaveBackupSnapshot},
    domain::backup::repository::BackupSnapshotRepository,
    domain::game::{console, mod_setup},
    dst::{self, DstConfig, cluster_ini::ClusterIni, server_ini::ServerIni},
    infra::config::AppConfig,
    validation::validate_filename,
    web::app::AppState,
    web::error::{AppError, AppResult},
    web::handlers::{legacy_success, repository_error, response_with_message},
    web::response::LoginResponse,
};

pub(crate) const MAX_BACKUP_UPLOAD_BYTES: usize = 128 * 1024 * 1024;
const DOWNLOAD_CHUNK_BYTES: usize = 64 * 1024;

#[derive(Debug, Default, Deserialize)]
pub(crate) struct CreateBackupRequest {
    #[serde(rename = "backupName", default)]
    backup_name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeleteBackupRequest {
    #[serde(rename = "fileNames", default)]
    file_names: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RenameBackupRequest {
    #[serde(rename = "fileName")]
    file_name: String,
    #[serde(rename = "newName")]
    new_name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BackupFileQuery {
    #[serde(rename = "fileName")]
    file_name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RestoreBackupQuery {
    #[serde(rename = "backupName")]
    backup_name: String,
}

/// Lists `.zip` and `.tar` backup files for the active cluster.
pub(crate) async fn list_handler(
    State(state): State<AppState>,
) -> AppResult<Json<LoginResponse<Vec<backup::BackupEntry>>>> {
    let cluster_name =
        dst::current_cluster_name(&state.root_path).map_err(file_error("resolve cluster"))?;
    let root = state.root_path.clone();
    let backups =
        tokio::task::spawn_blocking(move || backup::list_cluster_backups(&root, &cluster_name))
            .await
            .map_err(join_error("list backups"))??;
    tracing::debug!(count = backups.len(), "listed backup archives");
    Ok(Json(response_with_message(
        "get backup list success",
        backups,
    )))
}

/// Creates a Go-shaped backup after sending `c_save()` to Master.
pub(crate) async fn create_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> AppResult<Json<LoginResponse<Value>>> {
    let cluster_name =
        dst::current_cluster_name(&state.root_path).map_err(file_error("resolve cluster"))?;
    // Go triggers Master `c_save()` before zipping. The console adapter already
    // preserves Go's best-effort success behavior when `screen` is absent.
    console::send_level_command(
        state.command_runner.as_ref(),
        &cluster_name,
        "Master",
        "c_save()",
    )
    .await?;
    if !state.backup_c_save_delay.is_zero() {
        tokio::time::sleep(state.backup_c_save_delay).await;
    }
    let request: CreateBackupRequest = best_effort_json_body(&headers, &body);
    let backup_name = request.backup_name;
    let root = state.root_path.clone();
    let cluster_for_backup = cluster_name.clone();
    tokio::task::spawn_blocking(move || {
        let name = if backup_name.is_empty() {
            None
        } else {
            Some(backup_name.as_str())
        };
        backup::create_cluster_backup(&root, &cluster_for_backup, name)
    })
    .await
    .map_err(join_error("create backup"))??;
    tracing::info!(cluster_name, "handled backup create request");
    Ok(Json(response_with_message(
        "create backup success",
        Value::Null,
    )))
}

/// Deletes selected backup archives.
pub(crate) async fn delete_handler(
    State(state): State<AppState>,
    Json(request): Json<DeleteBackupRequest>,
) -> AppResult<Json<LoginResponse<Value>>> {
    let root = state.root_path.clone();
    tokio::task::spawn_blocking(move || backup::delete_cluster_backups(&root, &request.file_names))
        .await
        .map_err(join_error("delete backups"))??;
    Ok(Json(response_with_message(
        "delete backups success",
        Value::Null,
    )))
}

/// Renames one backup archive.
pub(crate) async fn rename_handler(
    State(state): State<AppState>,
    Json(request): Json<RenameBackupRequest>,
) -> AppResult<Json<LoginResponse<Value>>> {
    let root = state.root_path.clone();
    tokio::task::spawn_blocking(move || {
        backup::rename_cluster_backup(&root, &request.file_name, &request.new_name)
    })
    .await
    .map_err(join_error("rename backup"))??;
    Ok(Json(response_with_message(
        "rename backup success",
        Value::Null,
    )))
}

/// Serves a raw backup archive download with Go-compatible headers.
pub(crate) async fn download_handler(
    State(state): State<AppState>,
    Query(query): Query<BackupFileQuery>,
) -> AppResult<Response> {
    let root = state.root_path.clone();
    let (file_name, file) =
        tokio::task::spawn_blocking(move || backup::open_cluster_backup(&root, &query.file_name))
            .await
            .map_err(join_error("download backup"))??;
    Ok((
        [
            (CONTENT_TYPE, "application/octet-stream".to_owned()),
            (
                CONTENT_DISPOSITION,
                format!("attachment; filename={file_name}"),
            ),
            (
                HeaderName::from_static("content-transfer-encoding"),
                "binary".to_owned(),
            ),
        ],
        Body::from_stream(file_byte_stream(file)),
    )
        .into_response())
}

/// Accepts a single `file` multipart part and stores it as a backup archive.
pub(crate) async fn upload_handler(
    State(state): State<AppState>,
    multipart: Multipart,
) -> AppResult<Json<LoginResponse<Value>>> {
    read_single_backup_upload(&state.root_path, multipart).await?;
    Ok(Json(response_with_message(
        "upload backup success",
        Value::Null,
    )))
}

/// Restores the selected backup archive and merges Master mod setup.
pub(crate) async fn restore_handler(
    State(state): State<AppState>,
    Query(query): Query<RestoreBackupQuery>,
) -> AppResult<Json<LoginResponse<Value>>> {
    let cluster_name =
        dst::current_cluster_name(&state.root_path).map_err(file_error("resolve cluster"))?;
    let root = state.root_path.clone();
    let backup_name = query.backup_name.clone();
    let merge_root = root.clone();
    tokio::task::spawn_blocking(move || {
        backup::restore_cluster_backup(&root, &cluster_name, &backup_name, |staged_cluster_dir| {
            merge_restored_master_mod_setup(&merge_root, staged_cluster_dir)
        })
    })
    .await
    .map_err(join_error("restore backup"))??;
    tracing::info!(backup_name = %query.backup_name, "handled backup restore request");
    Ok(Json(response_with_message(
        "restore backup success",
        Value::Null,
    )))
}

fn merge_restored_master_mod_setup(root: &Path, staged_cluster_dir: &Path) -> AppResult<()> {
    if let Some(modoverrides) =
        dst::safe_read_cluster_file_to_string(staged_cluster_dir, "Master/modoverrides.lua")
            .map_err(file_error("read staged restored modoverrides"))?
    {
        mod_setup::merge_dedicated_server_mods_setup(root, &modoverrides)
            .map_err(file_error("merge staged restored mod setup"))?;
    }
    Ok(())
}

/// Returns a best-effort Go-shaped archive summary without external network.
pub(crate) async fn archive_handler(
    State(state): State<AppState>,
) -> AppResult<Json<LoginResponse<GameArchiveResponse>>> {
    let root = state.root_path.clone();
    let app_config = state.config.clone();
    let archive = tokio::task::spawn_blocking(move || build_game_archive(&root, &app_config))
        .await
        .map_err(join_error("build game archive"))??;
    Ok(Json(legacy_success(archive)))
}

pub(crate) async fn save_snapshot_setting_handler(
    State(state): State<AppState>,
    Json(request): Json<SaveBackupSnapshot>,
) -> AppResult<Json<LoginResponse<BackupSnapshotRecord>>> {
    let repository = BackupSnapshotRepository::new(state.db.clone());
    let saved = repository
        .save_singleton(request)
        .await
        .map_err(|error| repository_error("save backup snapshot setting", error))?;
    tracing::info!(id = saved.id, "saved backup snapshot setting");
    Ok(Json(legacy_success(saved)))
}

pub(crate) async fn get_snapshot_setting_handler(
    State(state): State<AppState>,
) -> AppResult<Json<LoginResponse<BackupSnapshotRecord>>> {
    let repository = BackupSnapshotRepository::new(state.db.clone());
    let setting = repository
        .first_or_zero()
        .await
        .map_err(|error| repository_error("get backup snapshot setting", error))?;
    Ok(Json(legacy_success(setting)))
}

pub(crate) async fn snapshot_list_handler(
    State(state): State<AppState>,
) -> AppResult<Json<LoginResponse<Vec<backup::BackupEntry>>>> {
    let cluster_name =
        dst::current_cluster_name(&state.root_path).map_err(file_error("resolve cluster"))?;
    let root = state.root_path.clone();
    let backups = tokio::task::spawn_blocking(move || {
        let all = backup::list_cluster_backups(&root, &cluster_name)?;
        Ok::<_, AppError>(
            all.into_iter()
                .filter(|entry| {
                    entry.file_name.starts_with("(snapshot)")
                        && entry.file_name.contains(&cluster_name)
                })
                .collect::<Vec<_>>(),
        )
    })
    .await
    .map_err(join_error("list snapshot backups"))??;
    Ok(Json(legacy_success(backups)))
}

async fn read_single_backup_upload(root: &Path, mut multipart: Multipart) -> AppResult<()> {
    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::bad_request("invalid multipart upload"))?
    {
        if field.name() != Some("file") {
            continue;
        }
        let filename = field
            .file_name()
            .map(str::to_owned)
            .ok_or_else(|| AppError::bad_request("uploaded file is missing filename"))?;
        let filename = validate_filename(&filename)
            .map_err(|error| AppError::bad_request(error.to_string()))?
            .into_string();
        let upload_root = root.to_path_buf();
        let mut upload = tokio::task::spawn_blocking(move || {
            backup::begin_cluster_backup_upload(&upload_root, &filename)
        })
        .await
        .map_err(join_error("begin backup upload"))??;
        let mut uploaded_bytes = 0usize;
        while let Some(chunk) = field
            .chunk()
            .await
            .map_err(|_| AppError::bad_request("invalid uploaded file"))?
        {
            uploaded_bytes = uploaded_bytes
                .checked_add(chunk.len())
                .ok_or_else(|| AppError::payload_too_large("backup upload is too large"))?;
            if uploaded_bytes > MAX_BACKUP_UPLOAD_BYTES {
                return Err(AppError::payload_too_large("backup upload is too large"));
            }
            upload = tokio::task::spawn_blocking(move || {
                upload.write_chunk(&chunk)?;
                Ok::<_, AppError>(upload)
            })
            .await
            .map_err(join_error("write backup upload"))??;
        }
        tokio::task::spawn_blocking(move || upload.commit())
            .await
            .map_err(join_error("commit backup upload"))??;
        return Ok(());
    }
    Err(AppError::bad_request("uploaded file is missing"))
}

fn best_effort_json_body<T>(headers: &HeaderMap, body: &Bytes) -> T
where
    T: DeserializeOwned + Default,
{
    if body.is_empty() || !is_json_content_type(headers) {
        return T::default();
    }
    serde_json::from_slice(body).unwrap_or_default()
}

fn is_json_content_type(headers: &HeaderMap) -> bool {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| {
            let media_type = value
                .split(';')
                .next()
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase();
            media_type == "application/json" || media_type.ends_with("+json")
        })
        .unwrap_or(false)
}

fn file_byte_stream(
    file: fs::File,
) -> impl futures_util::Stream<Item = Result<Bytes, io::Error>> + Send + 'static {
    stream::unfold(Some(file), |state| async move {
        let file = state?;
        match tokio::task::spawn_blocking(move || {
            let mut file = file;
            let mut buffer = vec![0_u8; DOWNLOAD_CHUNK_BYTES];
            let item = match file.read(&mut buffer) {
                Ok(0) => None,
                Ok(size) => {
                    buffer.truncate(size);
                    Some(Ok(Bytes::from(buffer)))
                }
                Err(error) => Some(Err(error)),
            };
            (item, file)
        })
        .await
        {
            Ok((Some(item), file)) => Some((item, Some(file))),
            Ok((None, _file)) => None,
            Err(error) => Some((Err(io::Error::other(error)), None)),
        }
    })
}

#[derive(Debug, Serialize)]
pub(crate) struct GameArchiveResponse {
    #[serde(rename = "clusterName")]
    cluster_name: String,
    #[serde(rename = "clusterDescription")]
    cluster_description: String,
    #[serde(rename = "clusterPassword")]
    cluster_password: String,
    #[serde(rename = "gameMod")]
    game_mod: String,
    #[serde(rename = "maxPlayers")]
    max_players: i64,
    mods: usize,
    #[serde(rename = "ipConnect")]
    ip_connect: String,
    port: u64,
    ip: String,
    meta: Value,
    version: i64,
    #[serde(rename = "lastVersion")]
    last_version: i64,
}

pub(crate) fn build_game_archive(
    root: &Path,
    app_config: &AppConfig,
) -> AppResult<GameArchiveResponse> {
    let config = DstConfig::load(root).map_err(file_error("load dst_config"))?;
    let cluster_dir = config.klei_root(root).join(&config.cluster);
    let cluster_ini = read_cluster_ini(&cluster_dir)?;
    let master_modoverrides =
        dst::safe_read_cluster_file_to_string(&cluster_dir, "Master/modoverrides.lua")
            .map_err(file_error("read Master modoverrides"))?
            .unwrap_or_default();
    let server_ini = dst::safe_read_cluster_file_to_string(&cluster_dir, "Master/server.ini")
        .map_err(file_error("read Master server.ini"))?
        .map(|contents| ServerIni::from_contents(&contents, true))
        .unwrap_or_else(ServerIni::master_default);
    let ip = archive_ip(app_config);
    let ip_connect = if ip.is_empty() {
        String::new()
    } else if cluster_ini.cluster_password.is_empty() {
        format!("c_connect(\"{}\",{})", ip, server_ini.server_port)
    } else {
        format!(
            "c_connect(\"{}\",{},\"{}\")",
            ip, server_ini.server_port, cluster_ini.cluster_password
        )
    };
    Ok(GameArchiveResponse {
        cluster_name: cluster_ini.cluster_name,
        cluster_description: cluster_ini.cluster_description,
        cluster_password: cluster_ini.cluster_password,
        game_mod: cluster_ini.game_mode,
        max_players: i64::try_from(cluster_ini.max_players).unwrap_or(i64::MAX),
        mods: workshop_ids(&master_modoverrides).len(),
        ip_connect,
        port: server_ini.server_port,
        ip,
        // Go returns a zero-value nested Meta struct when no save meta exists.
        // Rust parses the `.meta` Lua table without executing it.
        meta: backup::archive_meta_value(root, &config, &config.cluster),
        version: local_dst_version(&config),
        last_version: -1,
    })
}

fn archive_ip(app_config: &AppConfig) -> String {
    if !app_config.wan_ip.trim().is_empty() {
        return app_config.wan_ip.clone();
    }
    local_ipv4_for_connect().unwrap_or_default()
}

fn local_ipv4_for_connect() -> Option<String> {
    let socket = UdpSocket::bind(("0.0.0.0", 0)).ok()?;
    socket.connect(("8.8.8.8", 80)).ok()?;
    match socket.local_addr().ok()?.ip() {
        IpAddr::V4(ip) if !ip.is_loopback() => Some(ip.to_string()),
        _ => None,
    }
}

fn local_dst_version(config: &DstConfig) -> i64 {
    let install_dir = if config.beta == 1 {
        format!("{}-beta", config.force_install_dir)
    } else {
        config.force_install_dir.clone()
    };
    let path = Path::new(&install_dir).join("version.txt");
    fs::read_to_string(path)
        .ok()
        .and_then(|contents| contents.trim().parse::<i64>().ok())
        .unwrap_or_default()
}

fn read_cluster_ini(cluster_dir: &Path) -> AppResult<ClusterIni> {
    let contents = dst::safe_read_cluster_file_to_string(cluster_dir, "cluster.ini")
        .map_err(file_error("read cluster.ini"))?;
    Ok(contents
        .as_deref()
        .map(ClusterIni::from_contents)
        .unwrap_or_else(ClusterIni::default_for_new_cluster))
}

fn workshop_ids(mod_data: &str) -> Vec<String> {
    const PREFIX: &str = "\"workshop-";
    let mut ids = Vec::new();
    let mut rest = mod_data;
    while let Some(start) = rest.find(PREFIX) {
        let after = &rest[start + PREFIX.len()..];
        let Some(end) = after.find('"') else {
            break;
        };
        let id = &after[..end];
        if !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit()) {
            ids.push(id.to_owned());
        }
        rest = &after[end + 1..];
    }
    ids.sort();
    ids.dedup();
    ids
}

fn file_error(operation: &'static str) -> impl FnOnce(std::io::Error) -> AppError {
    move |error| {
        tracing::error!(operation, error = %error, "backup handler filesystem operation failed");
        if error.kind() == std::io::ErrorKind::InvalidInput {
            AppError::bad_request(error.to_string())
        } else {
            AppError::internal(operation)
        }
    }
}

fn join_error(operation: &'static str) -> impl FnOnce(tokio::task::JoinError) -> AppError {
    move |error| {
        tracing::error!(operation, error = %error, "backup handler worker failed");
        AppError::internal(operation)
    }
}
