//! Game status, dashboard, console, and cleanup HTTP handlers.
//!
//! Status/system-info handlers are read-only. Console handlers intentionally
//! mutate live DST state by sending commands through `screen`, but they still
//! avoid installing, starting, stopping, or updating a DST server. Cleanup
//! handlers delete DST runtime save/log files under the selected cluster while
//! preserving configuration files.

use std::path::{Path, PathBuf};

use axum::{
    Json,
    extract::{Query, RawQuery, State},
};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    domain::backup,
    domain::game::{self, LevelStatusInfo, SystemInfo, console, level as game_level},
    dst,
    validation::{validate_level_name, validate_safe_command_arg},
    web::app::AppState,
    web::error::{AppError, AppResult},
    web::handlers::{legacy_empty_success, legacy_success, response_with_message},
    web::response::LoginResponse,
};

#[derive(Debug, Deserialize)]
pub(crate) struct LevelCommandRequest {
    #[serde(rename = "levelName")]
    level_name: String,
    command: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ConsoleCommandRequest {
    command: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BroadcastQuery {
    message: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct KuIdQuery {
    #[serde(rename = "kuId")]
    ku_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RollbackQuery {
    #[serde(rename = "dayNums")]
    day_nums: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LevelNameQuery {
    #[serde(rename = "levelName")]
    level_name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PreinstallQuery {
    name: Option<String>,
}

pub(crate) async fn status_handler(
    State(state): State<AppState>,
) -> AppResult<Json<LoginResponse<Vec<LevelStatusInfo>>>> {
    let root_path = state.root_path.clone();
    let process_provider = state.process_snapshot_provider.clone();
    let statuses = tokio::task::spawn_blocking(move || {
        let cluster_name =
            dst::current_cluster_name(&root_path).map_err(file_error("resolve cluster name"))?;
        let cluster_name = validate_safe_command_arg("cluster name", &cluster_name)
            .map_err(|error| AppError::bad_request(error.to_string()))?
            .into_string();
        let cluster_dir = dst::cluster_dir(&root_path, &cluster_name)
            .map_err(file_error("resolve cluster directory"))?;
        let worlds = game_level::list_existing_worlds_from_cluster_dir(&cluster_dir)?;
        let snapshots = match process_provider.snapshots() {
            Ok(snapshots) => snapshots,
            Err(error) => {
                tracing::warn!(
                    cluster_name,
                    error = %error,
                    "failed to collect process snapshots; reporting levels as stopped"
                );
                Vec::new()
            }
        };
        tracing::debug!(
            cluster_name,
            level_count = worlds.len(),
            process_count = snapshots.len(),
            "built DST level status response"
        );
        game::level_statuses_from_snapshots(&cluster_name, worlds, &snapshots)
            .map_err(|error| AppError::bad_request(error.to_string()))
    })
    .await
    .map_err(|error| {
        tracing::error!(error = %error, "status handler worker panicked or was cancelled");
        AppError::internal("collect level status")
    })??;

    Ok(Json(legacy_success(statuses)))
}

pub(crate) async fn system_info_handler(
    State(state): State<AppState>,
) -> AppResult<Json<LoginResponse<SystemInfo>>> {
    let root_path = state.root_path.clone();
    let system_info = tokio::task::spawn_blocking(move || {
        let cluster_name =
            dst::current_cluster_name(&root_path).map_err(file_error("resolve cluster name"))?;
        tracing::debug!(cluster_name, "collecting game system info");
        Ok::<_, AppError>(game::collect_system_info(&root_path))
    })
    .await
    .map_err(|error| {
        tracing::error!(error = %error, "system info handler worker panicked or was cancelled");
        AppError::internal("collect system info")
    })??;

    Ok(Json(legacy_success(system_info)))
}

pub(crate) async fn start_level_handler(
    State(state): State<AppState>,
    Query(query): Query<LevelNameQuery>,
) -> AppResult<Json<crate::web::response::LoginResponse<Value>>> {
    let context = game::lifecycle::LifecycleContext::load(&state.root_path)?;
    game::lifecycle::copy_steamclient_before_single_start(&state.root_path, &context)?;
    game::lifecycle::start_level(
        state.command_runner.as_ref(),
        state.process_snapshot_provider.as_ref(),
        &state.root_path,
        &context,
        &query.level_name,
        state.lifecycle_grace_period,
    )
    .await?;
    tracing::info!(
        cluster_name = %context.cluster_name,
        level_name = %query.level_name,
        "handled DST level start request"
    );
    Ok(Json(response_with_message(
        format!(
            "start {} {} success",
            context.cluster_name, query.level_name
        ),
        Value::Null,
    )))
}

pub(crate) async fn stop_level_handler(
    State(state): State<AppState>,
    Query(query): Query<LevelNameQuery>,
) -> AppResult<Json<crate::web::response::LoginResponse<Value>>> {
    let context = game::lifecycle::LifecycleContext::load(&state.root_path)?;
    game::lifecycle::stop_level(
        state.command_runner.as_ref(),
        state.process_snapshot_provider.as_ref(),
        &context,
        &query.level_name,
        state.lifecycle_grace_period,
    )
    .await?;
    tracing::info!(
        cluster_name = %context.cluster_name,
        level_name = %query.level_name,
        "handled DST level stop request"
    );
    // Go's stop route historically returned "start <cluster> <level> success".
    Ok(Json(response_with_message(
        format!(
            "start {} {} success",
            context.cluster_name, query.level_name
        ),
        Value::Null,
    )))
}

pub(crate) async fn start_all_handler(
    State(state): State<AppState>,
) -> AppResult<Json<crate::web::response::LoginResponse<Value>>> {
    let context = game::lifecycle::LifecycleContext::load(&state.root_path)?;
    game::lifecycle::start_all(
        state.command_runner.as_ref(),
        state.process_snapshot_provider.as_ref(),
        &state.root_path,
        &context,
        state.lifecycle_grace_period,
    )
    .await?;
    tracing::info!(
        cluster_name = %context.cluster_name,
        "handled DST start-all request"
    );
    Ok(Json(response_with_message(
        "start all success",
        Value::Null,
    )))
}

pub(crate) async fn stop_all_handler(
    State(state): State<AppState>,
) -> AppResult<Json<crate::web::response::LoginResponse<Value>>> {
    let context = game::lifecycle::LifecycleContext::load(&state.root_path)?;
    game::lifecycle::stop_all(
        state.command_runner.as_ref(),
        state.process_snapshot_provider.as_ref(),
        &state.root_path,
        &context,
        state.lifecycle_grace_period,
    )
    .await?;
    tracing::info!(
        cluster_name = %context.cluster_name,
        "handled DST stop-all request"
    );
    Ok(Json(response_with_message("stop all success", Value::Null)))
}

pub(crate) async fn udp_ports_handler()
-> AppResult<Json<crate::web::response::LoginResponse<Vec<u16>>>> {
    let ports = game::udp::bounded_legacy_udp_ports().map_err(AppError::internal)?;
    Ok(Json(legacy_success(ports)))
}

pub(crate) async fn update_game_handler(
    State(state): State<AppState>,
) -> AppResult<Json<crate::web::response::LoginResponse<Value>>> {
    let context = game::lifecycle::LifecycleContext::load(&state.root_path)?;
    game::lifecycle::update_game(
        state.command_runner.as_ref(),
        state.process_snapshot_provider.as_ref(),
        &state.root_path,
        &context,
        state.lifecycle_grace_period,
    )
    .await?;
    Ok(Json(response_with_message(
        "update dst success",
        Value::Null,
    )))
}

pub(crate) async fn operate_player_handler(
    State(state): State<AppState>,
) -> AppResult<Json<crate::web::response::LoginResponse<Value>>> {
    let cluster_name = current_safe_cluster_name(&state)?;
    // The Go route is registered as `/api/game/operate/player` but reads
    // `ctx.Param("type")` and `ctx.Param("kuId")`, so both values are empty
    // for the real route. Preserve that compatibility quirk instead of reading
    // query parameters that Go ignored.
    console::master_console(state.command_runner.as_ref(), &cluster_name, "").await?;
    console::caves_console(state.command_runner.as_ref(), &cluster_name, "").await?;
    tracing::info!(cluster_name, "handled legacy operate-player request");
    Ok(Json(legacy_empty_success()))
}

pub(crate) async fn preinstall_handler(
    State(state): State<AppState>,
    Query(query): Query<PreinstallQuery>,
) -> AppResult<Json<crate::web::response::LoginResponse<Value>>> {
    let name = query.name.as_deref().unwrap_or("default");
    let context = game::lifecycle::LifecycleContext::load(&state.root_path)?;
    game::lifecycle::stop_all_strict(
        state.command_runner.as_ref(),
        state.process_snapshot_provider.as_ref(),
        &state.root_path,
        &context,
        state.lifecycle_grace_period,
    )
    .await?;
    console::master_console(
        state.command_runner.as_ref(),
        &context.cluster_name,
        "c_save()",
    )
    .await?;
    let backup_name = backup::create_cluster_backup(&state.root_path, &context.cluster_name, None)?;
    tracing::info!(
        cluster_name = %context.cluster_name,
        backup_name,
        "created preinstall backup"
    );
    game::preinstall::apply(&state.root_path, name)?;
    tracing::info!(
        cluster_name = %context.cluster_name,
        template_name = name,
        "handled preinstall template request"
    );
    Ok(Json(legacy_empty_success()))
}

pub(crate) async fn level_command_handler(
    State(state): State<AppState>,
    Json(request): Json<LevelCommandRequest>,
) -> AppResult<Json<crate::web::response::LoginResponse<Value>>> {
    let cluster_name = current_safe_cluster_name(&state)?;
    console::send_level_command(
        state.command_runner.as_ref(),
        &cluster_name,
        &request.level_name,
        &request.command,
    )
    .await?;
    tracing::info!(
        cluster_name,
        level_name = %request.level_name,
        command_len = request.command.len(),
        "handled level console command request"
    );
    Ok(Json(legacy_empty_success()))
}

pub(crate) async fn broadcast_handler(
    State(state): State<AppState>,
    Query(query): Query<BroadcastQuery>,
) -> AppResult<Json<crate::web::response::LoginResponse<Value>>> {
    let cluster_name = current_safe_cluster_name(&state)?;
    console::broadcast(state.command_runner.as_ref(), &cluster_name, &query.message).await?;
    tracing::info!(
        cluster_name,
        message_len = query.message.len(),
        "handled broadcast request"
    );
    Ok(Json(legacy_empty_success()))
}

pub(crate) async fn kick_player_handler(
    State(state): State<AppState>,
    Query(query): Query<KuIdQuery>,
) -> AppResult<Json<crate::web::response::LoginResponse<Value>>> {
    let cluster_name = current_safe_cluster_name(&state)?;
    console::kick_player(state.command_runner.as_ref(), &cluster_name, &query.ku_id).await?;
    tracing::info!(cluster_name, "handled kick player request");
    Ok(Json(legacy_empty_success()))
}

pub(crate) async fn kill_player_handler(
    State(state): State<AppState>,
    Query(query): Query<KuIdQuery>,
) -> AppResult<Json<crate::web::response::LoginResponse<Value>>> {
    let cluster_name = current_safe_cluster_name(&state)?;
    console::kill_player(state.command_runner.as_ref(), &cluster_name, &query.ku_id).await?;
    tracing::info!(cluster_name, "handled kill player request");
    Ok(Json(legacy_empty_success()))
}

pub(crate) async fn respawn_player_handler(
    State(state): State<AppState>,
    Query(query): Query<KuIdQuery>,
) -> AppResult<Json<crate::web::response::LoginResponse<Value>>> {
    let cluster_name = current_safe_cluster_name(&state)?;
    console::respawn_player(state.command_runner.as_ref(), &cluster_name, &query.ku_id).await?;
    tracing::info!(cluster_name, "handled respawn player request");
    Ok(Json(legacy_empty_success()))
}

pub(crate) async fn rollback_handler(
    State(state): State<AppState>,
    Query(query): Query<RollbackQuery>,
) -> AppResult<Json<crate::web::response::LoginResponse<Value>>> {
    let days = query
        .day_nums
        .parse::<i64>()
        .map_err(|_| AppError::bad_request("dayNums must be an integer"))?;
    let cluster_name = current_safe_cluster_name(&state)?;
    console::rollback(state.command_runner.as_ref(), &cluster_name, days).await?;
    tracing::info!(cluster_name, days, "handled rollback request");
    Ok(Json(legacy_empty_success()))
}

pub(crate) async fn regenerate_world_handler(
    State(state): State<AppState>,
) -> AppResult<Json<crate::web::response::LoginResponse<Value>>> {
    let cluster_name = current_safe_cluster_name(&state)?;
    console::regenerate_world(state.command_runner.as_ref(), &cluster_name).await?;
    tracing::info!(cluster_name, "handled regenerate world request");
    Ok(Json(legacy_empty_success()))
}

pub(crate) async fn master_console_handler(
    State(state): State<AppState>,
    Json(request): Json<ConsoleCommandRequest>,
) -> AppResult<Json<crate::web::response::LoginResponse<Value>>> {
    let cluster_name = current_safe_cluster_name(&state)?;
    console::master_console(
        state.command_runner.as_ref(),
        &cluster_name,
        &request.command,
    )
    .await?;
    tracing::info!(
        cluster_name,
        command_len = request.command.len(),
        "handled master console request"
    );
    Ok(Json(legacy_empty_success()))
}

pub(crate) async fn caves_console_handler(
    State(state): State<AppState>,
    Json(request): Json<ConsoleCommandRequest>,
) -> AppResult<Json<crate::web::response::LoginResponse<Value>>> {
    let cluster_name = current_safe_cluster_name(&state)?;
    console::caves_console(
        state.command_runner.as_ref(),
        &cluster_name,
        &request.command,
    )
    .await?;
    tracing::info!(
        cluster_name,
        command_len = request.command.len(),
        "handled caves console request"
    );
    Ok(Json(legacy_empty_success()))
}

pub(crate) async fn clean_world_handler(
    State(state): State<AppState>,
) -> AppResult<Json<crate::web::response::LoginResponse<Value>>> {
    let (cluster_name, cluster_dir) = current_cluster_name_and_dir(&state)?;
    for level_name in ["Master", "Caves"] {
        clean_level_save_dirs(&cluster_dir, level_name)?;
    }
    tracing::info!(
        cluster_name,
        level_count = 2,
        "cleaned default DST level save directories"
    );
    Ok(Json(legacy_empty_success()))
}

pub(crate) async fn clean_level_handler(
    State(state): State<AppState>,
    RawQuery(raw_query): RawQuery,
) -> AppResult<Json<crate::web::response::LoginResponse<Value>>> {
    let levels = parse_clean_level_query(raw_query)?;
    for level_name in &levels {
        validate_level_name(level_name)
            .map_err(|error| AppError::bad_request(error.to_string()))?;
    }

    let (cluster_name, cluster_dir) = current_cluster_name_and_dir(&state)?;
    for level_name in &levels {
        clean_level_runtime_files(&cluster_dir, level_name)?;
    }
    tracing::info!(
        cluster_name,
        level_count = levels.len(),
        "cleaned selected DST level runtime files"
    );
    Ok(Json(legacy_empty_success()))
}

pub(crate) async fn clean_all_levels_handler(
    State(state): State<AppState>,
) -> AppResult<Json<crate::web::response::LoginResponse<Value>>> {
    let (cluster_name, cluster_dir) = current_cluster_name_and_dir(&state)?;
    let worlds = game_level::list_indexed_worlds_from_cluster_dir(&cluster_dir)?;
    for world in &worlds {
        clean_level_runtime_files(&cluster_dir, &world.uuid)?;
    }
    tracing::info!(
        cluster_name,
        level_count = worlds.len(),
        "cleaned all indexed DST level runtime files"
    );
    Ok(Json(legacy_empty_success()))
}

fn current_safe_cluster_name(state: &AppState) -> AppResult<String> {
    let cluster_name =
        dst::current_cluster_name(&state.root_path).map_err(file_error("resolve cluster name"))?;
    validate_safe_command_arg("cluster name", &cluster_name)
        .map(|cluster| cluster.into_string())
        .map_err(|error| AppError::bad_request(error.to_string()))
}

fn current_cluster_name_and_dir(state: &AppState) -> AppResult<(String, PathBuf)> {
    let cluster_name =
        dst::current_cluster_name(&state.root_path).map_err(file_error("resolve cluster name"))?;
    let cluster_dir = dst::cluster_dir(&state.root_path, &cluster_name)
        .map_err(file_error("resolve cluster directory"))?;
    Ok((cluster_name, cluster_dir))
}

fn clean_level_runtime_files(cluster_dir: &Path, level_name: &str) -> AppResult<()> {
    clean_level_save_dirs(cluster_dir, level_name)?;
    let safe_level = validate_level_name(level_name)
        .map_err(|error| AppError::bad_request(error.to_string()))?;
    let level_path = Path::new(safe_level.as_str());
    for file_name in ["server_chat_log.txt", "server_log.txt"] {
        let deleted = dst::safe_remove_cluster_file(cluster_dir, level_path.join(file_name))
            .map_err(file_error("delete level runtime log file"))?;
        tracing::debug!(
            level_name = safe_level.as_str(),
            file_name,
            deleted,
            "processed DST level runtime log file cleanup"
        );
    }
    Ok(())
}

fn clean_level_save_dirs(cluster_dir: &Path, level_name: &str) -> AppResult<()> {
    let safe_level = validate_level_name(level_name)
        .map_err(|error| AppError::bad_request(error.to_string()))?;
    let level_path = Path::new(safe_level.as_str());
    for directory_name in ["backup", "save"] {
        dst::safe_remove_cluster_dir(cluster_dir, level_path.join(directory_name))
            .map_err(file_error("delete level runtime directory"))?;
        tracing::debug!(
            level_name = safe_level.as_str(),
            directory_name,
            "processed DST level runtime directory cleanup"
        );
    }
    Ok(())
}

fn parse_clean_level_query(raw_query: Option<String>) -> AppResult<Vec<String>> {
    let Some(raw_query) = raw_query else {
        return Ok(Vec::new());
    };
    let mut levels = Vec::new();
    for pair in raw_query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (raw_key, raw_value) = pair.split_once('=').unwrap_or((pair, ""));
        if decode_form_component(raw_key)? == "level" {
            levels.push(decode_form_component(raw_value)?);
        }
    }
    Ok(levels)
}

fn decode_form_component(value: &str) -> AppResult<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                decoded.push(b' ');
                index += 1;
            }
            b'%' => {
                if index + 2 >= bytes.len() {
                    return Err(AppError::bad_request(
                        "query contains invalid percent encoding",
                    ));
                }
                let high = hex_value(bytes[index + 1]).ok_or_else(|| {
                    AppError::bad_request("query contains invalid percent encoding")
                })?;
                let low = hex_value(bytes[index + 2]).ok_or_else(|| {
                    AppError::bad_request("query contains invalid percent encoding")
                })?;
                decoded.push((high << 4) | low);
                index += 3;
            }
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(decoded)
        .map_err(|_| AppError::bad_request("query contains invalid UTF-8 encoding"))
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn file_error(
    operation: &'static str,
) -> impl FnOnce(std::io::Error) -> AppError + Copy + Send + Sync + 'static {
    move |error| {
        if error.kind() == std::io::ErrorKind::InvalidInput {
            AppError::bad_request(error.to_string())
        } else {
            tracing::error!(operation, error = %error, "game handler file operation failed");
            AppError::internal(operation)
        }
    }
}
