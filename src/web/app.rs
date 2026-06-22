//! Axum application assembly for migrated Rust HTTP routes.
//!
//! This module is the boundary the future binary should call: it owns the
//! shared state shape, route mounting, and the auth middleware that mirrors the
//! current Go middleware. The binary must serve the returned router through
//! [`Router::into_make_service_with_connect_info`] so white-admin-IP checks use
//! trusted TCP peer metadata instead of client-controlled forwarding headers.

use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};

use axum::{
    Json, Router,
    body::Body,
    extract::connect_info::IntoMakeServiceWithConnectInfo,
    extract::{ConnectInfo, DefaultBodyLimit, FromRequest, Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{any, delete, get, post},
};

use crate::{
    domain::auth::{SessionStore, UserCredentials, is_white_ip, is_whitelisted_path},
    infra::command::{CommandRunner, TokioCommandRunner},
    infra::config::AppConfig,
    infra::db::SqlitePool,
    infra::http_client::{HttpClient, ReqwestHttpClient},
    infra::process::{ProcessSnapshotProvider, SystemProcessSnapshotProvider},
    web::error::{AppError, AppResult},
    web::handlers::{
        announcement,
        auth::{
            AuthState, change_password_handler, get_user_info_handler, login_handler,
            logout_handler, update_user_info_handler,
        },
        auto_check, backup, cluster, dst_config, dst_static, files, game, init, install, kv, level,
        logs, map, mods, player, player_log, share, static_files, statistics, steam_news, streams,
        tasks, third_party, web_link, webhook, ws,
    },
};

/// Shared application state used by the migrated HTTP handlers.
#[derive(Clone)]
pub struct AppState {
    /// Runtime configuration loaded from Go-compatible config files.
    pub config: AppConfig,
    /// SQLite pool shared by repository-backed handlers.
    pub db: SqlitePool,
    /// In-memory auth sessions shared by middleware and auth handlers.
    pub sessions: SessionStore,
    /// Auth handler state derived from the root path and shared session store.
    pub auth: AuthState,
    /// Filesystem root used for legacy relative files such as `password.txt`.
    pub root_path: PathBuf,
    /// External command runner used for SteamCMD, screen, and DST processes.
    pub command_runner: Arc<dyn CommandRunner>,
    /// HTTP client used for third-party read-only proxy routes.
    pub http_client: Arc<dyn HttpClient>,
    /// Process snapshot provider used by status and lifecycle control routes.
    pub process_snapshot_provider: Arc<dyn ProcessSnapshotProvider>,
    /// Grace period after a graceful DST shutdown command before hard-kill fallback.
    pub lifecycle_grace_period: Duration,
    /// Delay after Go-compatible `c_save()` before backup zipping starts.
    pub backup_c_save_delay: Duration,
    /// Delay after sending the Go-compatible online-player Lua query.
    pub player_query_delay: Duration,
    /// Optional deterministic online-player marker for tests.
    pub player_query_marker_override: Option<String>,
}

impl AppState {
    /// Creates application state from config, database pool, sessions, and root path.
    ///
    /// The password file is resolved under `root_path` so tests and the future
    /// binary can run against an explicit application root without changing the
    /// process working directory.
    pub fn new(
        config: AppConfig,
        db: SqlitePool,
        sessions: SessionStore,
        root_path: impl Into<PathBuf>,
    ) -> Self {
        Self::new_with_command_runner_and_http_client(
            config,
            db,
            sessions,
            root_path,
            TokioCommandRunner::new(),
            ReqwestHttpClient::new(),
            SystemProcessSnapshotProvider,
        )
    }

    /// Creates application state with an injected command runner.
    ///
    /// Integration tests use this to assert argv construction without launching
    /// real `screen`, SteamCMD, or DST processes. Production code should call
    /// [`Self::new`], which installs the Tokio process-backed runner.
    pub fn new_with_command_runner<R>(
        config: AppConfig,
        db: SqlitePool,
        sessions: SessionStore,
        root_path: impl Into<PathBuf>,
        command_runner: R,
    ) -> Self
    where
        R: CommandRunner + 'static,
    {
        Self::new_with_command_runner_and_http_client(
            config,
            db,
            sessions,
            root_path,
            command_runner,
            ReqwestHttpClient::new(),
            SystemProcessSnapshotProvider,
        )
    }

    /// Creates application state with injected command and HTTP clients.
    ///
    /// Slice 1 introduces read-only third-party proxy endpoints. They must be
    /// fakeable in tests so CI never depends on Steam, Klei, or community
    /// server-list availability.
    pub fn new_with_command_runner_and_process_snapshot_provider<R, P>(
        config: AppConfig,
        db: SqlitePool,
        sessions: SessionStore,
        root_path: impl Into<PathBuf>,
        command_runner: R,
        process_snapshot_provider: P,
    ) -> Self
    where
        R: CommandRunner + 'static,
        P: ProcessSnapshotProvider + 'static,
    {
        Self::new_with_command_runner_and_http_client(
            config,
            db,
            sessions,
            root_path,
            command_runner,
            ReqwestHttpClient::new(),
            process_snapshot_provider,
        )
    }

    pub fn new_with_command_runner_and_http_client<R, H, P>(
        config: AppConfig,
        db: SqlitePool,
        sessions: SessionStore,
        root_path: impl Into<PathBuf>,
        command_runner: R,
        http_client: H,
        process_snapshot_provider: P,
    ) -> Self
    where
        R: CommandRunner + 'static,
        H: HttpClient + 'static,
        P: ProcessSnapshotProvider + 'static,
    {
        let root_path = root_path.into();
        let password_path = root_path.join("password.txt");
        let auth = AuthState::new(
            password_path,
            sessions.clone(),
            config.white_admin_ip.clone(),
        );

        tracing::debug!(
            root_path = %root_path.display(),
            has_white_admin_ip = config.white_admin_ip.is_some(),
            "created application state"
        );

        Self {
            config,
            db,
            sessions,
            auth,
            root_path,
            command_runner: Arc::new(command_runner),
            http_client: Arc::new(http_client),
            process_snapshot_provider: Arc::new(process_snapshot_provider),
            lifecycle_grace_period: Duration::from_secs(3),
            backup_c_save_delay: Duration::from_secs(5),
            player_query_delay: Duration::from_secs(1),
            player_query_marker_override: None,
        }
    }

    /// Overrides the lifecycle shutdown grace period.
    ///
    /// Production uses Go's three-second delay. Tests set this to zero so they
    /// can verify fallback behavior without slowing the suite.
    pub fn with_lifecycle_grace_period(mut self, lifecycle_grace_period: Duration) -> Self {
        self.lifecycle_grace_period = lifecycle_grace_period;
        self
    }

    /// Overrides the backup `c_save()` delay.
    ///
    /// Production keeps Go's five-second wait so the game can flush world state
    /// before zipping. Tests set this to zero to avoid slowing route coverage.
    pub fn with_backup_c_save_delay(mut self, backup_c_save_delay: Duration) -> Self {
        self.backup_c_save_delay = backup_c_save_delay;
        self
    }

    /// Overrides the online-player log-scrape delay.
    ///
    /// Production waits one second like Go so the Lua `print` output reaches
    /// `server_log.txt`. Tests set this to zero and pre-seed the log file.
    pub fn with_player_query_delay(mut self, player_query_delay: Duration) -> Self {
        self.player_query_delay = player_query_delay;
        self
    }

    /// Overrides the online-player log marker.
    ///
    /// Tests use this to make the Lua command and parsed log line deterministic.
    pub fn with_player_query_marker_override(
        mut self,
        player_query_marker_override: impl Into<String>,
    ) -> Self {
        self.player_query_marker_override = Some(player_query_marker_override.into());
        self
    }
}

/// Builds the migrated Axum router with auth middleware and low-risk routes.
pub fn build_router(state: AppState) -> Router {
    let auth_routes = Router::new()
        .route("/api/login", post(login_route_handler))
        .route("/api/logout", get(logout_handler).post(logout_handler))
        .route("/api/change/password", post(change_password_handler))
        .route(
            "/api/user",
            get(get_user_info_handler).post(update_user_info_handler),
        )
        .with_state(state.auth.clone());

    let app_routes = Router::new()
        .route("/hello", get(hello_handler))
        .route("/", get(static_files::index_handler))
        .route(
            "/assets/{*filepath}",
            get(static_files::file_handler).head(static_files::file_handler),
        )
        .route(
            "/misc/{*filepath}",
            get(static_files::file_handler).head(static_files::file_handler),
        )
        .route(
            "/static/js/{*filepath}",
            get(static_files::file_handler).head(static_files::file_handler),
        )
        .route(
            "/static/css/{*filepath}",
            get(static_files::file_handler).head(static_files::file_handler),
        )
        .route(
            "/static/img/{*filepath}",
            get(static_files::file_handler).head(static_files::file_handler),
        )
        .route(
            "/static/fonts/{*filepath}",
            get(static_files::file_handler).head(static_files::file_handler),
        )
        .route(
            "/static/media/{*filepath}",
            get(static_files::file_handler).head(static_files::file_handler),
        )
        .route(
            "/favicon.ico",
            get(static_files::file_handler).head(static_files::file_handler),
        )
        .route(
            "/asset-manifest.json",
            get(static_files::file_handler).head(static_files::file_handler),
        )
        .route(
            "/api/init",
            get(init::check_first_handler).post(init::init_first_handler),
        )
        .route(
            "/api/install/steamcmd",
            get(install::install_steamcmd_handler),
        )
        .route("/api/kv", get(kv::get_handler).post(kv::save_handler))
        .route(
            "/api/cluster",
            get(cluster::list_handler)
                .post(cluster::create_handler)
                .put(cluster::update_handler)
                .delete(cluster::delete_handler),
        )
        .route(
            "/api/cluster/level",
            get(level::list_handler)
                .put(level::save_all_handler)
                .post(level::create_handler)
                .delete(level::delete_handler),
        )
        .route(
            "/api/game/8level/clusterIni",
            get(dst_config::get_cluster_ini_handler).post(dst_config::save_cluster_ini_handler),
        )
        .route(
            "/api/game/8level/players",
            get(player::online_players_handler),
        )
        .route(
            "/api/game/8level/players/all",
            get(player::all_online_players_handler),
        )
        .route(
            "/api/game/8level/adminilist",
            get(player::get_adminlist_handler).post(player::overwrite_adminlist_handler),
        )
        .route(
            "/api/game/8level/whitelist",
            get(player::get_whitelist_handler).post(player::overwrite_whitelist_handler),
        )
        .route(
            "/api/game/8level/blacklist",
            get(player::get_blacklist_handler).post(player::overwrite_blacklist_handler),
        )
        .route("/api/game/8level/status", get(game::status_handler))
        .route(
            "/api/game/8level/status/stream",
            get(streams::status_stream_handler),
        )
        .route("/api/game/8level/start", get(game::start_level_handler))
        .route("/api/game/8level/stop", get(game::stop_level_handler))
        .route("/api/game/8level/start/all", get(game::start_all_handler))
        .route("/api/game/8level/stop/all", get(game::stop_all_handler))
        .route("/api/game/8level/udp/port", get(game::udp_ports_handler))
        .route(
            "/api/game/8level/command",
            post(game::level_command_handler),
        )
        .route(
            "/api/game/player",
            get(player::master_online_players_handler),
        )
        .route("/api/player/log", get(player_log::list_handler))
        .route("/api/player/log/delete", post(player_log::delete_handler))
        .route(
            "/api/statistics/active/user",
            get(statistics::active_user_handler),
        )
        .route(
            "/api/statistics/top/death",
            get(statistics::top_death_handler),
        )
        .route(
            "/api/statistics/top/login",
            get(statistics::top_login_handler),
        )
        .route(
            "/api/statistics/top/active",
            get(statistics::top_active_handler),
        )
        .route(
            "/api/statistics/rate/role",
            get(statistics::role_rate_handler),
        )
        .route(
            "/api/statistics/regenerate",
            get(statistics::regenerate_handler),
        )
        .route("/api/dst/version", get(third_party::dst_version_handler))
        .route(
            "/api/dst/home/server",
            post(third_party::home_server_handler),
        )
        .route(
            "/api/dst/home/server/detail",
            post(third_party::home_server_detail_handler),
        )
        .route(
            "/api/dst/lobby/server/detail",
            get(third_party::lobby_server_detail_handler),
        )
        .route(
            "/api/dst/home/server2",
            get(third_party::home_server2_handler),
        )
        .route(
            "/api/dst/home/server/detail2",
            get(third_party::home_server_detail2_handler),
        )
        .route(
            "/api/dst-static/{*filepath}",
            any(dst_static::proxy_handler),
        )
        .route("/steam/dst/news", get(steam_news::dst_news_handler))
        .route("/api/mod/search", get(mods::search_handler))
        .route("/api/mod", get(mods::list_handler))
        .route(
            "/api/mod/{modId}",
            get(mods::get_handler)
                .put(mods::update_handler)
                .delete(mods::delete_handler),
        )
        .route(
            "/api/mod/setup/workshop",
            delete(mods::delete_setup_workshop_handler),
        )
        .route("/api/mod/modinfo/{modId}", get(mods::raw_modinfo_handler))
        .route(
            "/api/mod/modinfo",
            post(mods::save_raw_modinfo_handler).put(mods::update_all_handler),
        )
        .route(
            "/api/mod/modinfo/file",
            post(mods::add_modinfo_file_handler),
        )
        .route("/api/mod/ugc/acf", get(mods::ugc_acf_handler))
        .route("/api/mod/ugc", delete(mods::delete_ugc_handler))
        .route("/api/file/ugc/upload", post(files::upload_ugc_handler))
        .route(
            "/api/file/background",
            post(files::upload_background_handler).get(files::get_background_handler),
        )
        .route(
            "/api/game/player/adminlist",
            get(player::get_adminlist_handler)
                .post(player::append_adminlist_handler)
                .delete(player::delete_adminlist_handler),
        )
        .route(
            "/api/game/player/blacklist",
            get(player::get_blacklist_handler)
                .post(player::append_blacklist_handler)
                .delete(player::delete_blacklist_handler),
        )
        .route(
            "/api/dst/config",
            get(dst_config::get_dst_config_handler).post(dst_config::save_dst_config_handler),
        )
        .route(
            "/api/game/config",
            get(dst_config::get_game_config_handler).post(dst_config::save_game_config_handler),
        )
        .route("/api/game/sent/broadcast", get(game::broadcast_handler))
        .route("/api/game/kick/player", get(game::kick_player_handler))
        .route("/api/game/kill/player", get(game::kill_player_handler))
        .route(
            "/api/game/respawn/player",
            get(game::respawn_player_handler),
        )
        .route("/api/game/rollback", get(game::rollback_handler))
        .route(
            "/api/game/regenerateworld",
            get(game::regenerate_world_handler),
        )
        .route(
            "/api/game/operate/player",
            get(game::operate_player_handler),
        )
        .route("/api/game/clean", get(game::clean_world_handler))
        .route("/api/game/clean/level", get(game::clean_level_handler))
        .route(
            "/api/game/clean/level/all",
            get(game::clean_all_levels_handler),
        )
        .route(
            "/api/game/level/server/log",
            get(logs::level_server_log_handler),
        )
        .route(
            "/api/game/level/server/chat/log",
            get(logs::level_server_chat_log_handler),
        )
        .route(
            "/api/game/level/server/download",
            get(logs::level_log_download_handler),
        )
        .route("/api/game/dst-admin-go/log", get(logs::panel_log_handler))
        .route(
            "/api/game/dst-admin-go/log/download",
            get(logs::panel_log_download_handler),
        )
        .route(
            "/api/game/master/console",
            post(game::master_console_handler),
        )
        .route("/api/game/caves/console", post(game::caves_console_handler))
        .route("/api/game/preinstall", get(game::preinstall_handler))
        .route("/api/game/update", get(game::update_game_handler))
        .route(
            "/api/game/backup",
            get(backup::list_handler)
                .post(backup::create_handler)
                .delete(backup::delete_handler)
                .put(backup::rename_handler),
        )
        .route("/api/game/backup/download", get(backup::download_handler))
        .route(
            "/api/game/backup/upload",
            post(backup::upload_handler).layer(DefaultBodyLimit::max(
                backup::MAX_BACKUP_UPLOAD_BYTES + 1024 * 1024,
            )),
        )
        .route("/api/game/backup/restore", get(backup::restore_handler))
        .route("/api/game/archive", get(backup::archive_handler))
        .route(
            "/api/game/backup/snapshot/setting",
            post(backup::save_snapshot_setting_handler).get(backup::get_snapshot_setting_handler),
        )
        .route(
            "/api/game/backup/snapshot/list",
            get(backup::snapshot_list_handler),
        )
        .route(
            "/api/game/announce/setting",
            get(announcement::get_handler).post(announcement::save_handler),
        )
        .route(
            "/api/task",
            get(tasks::list_handler)
                .post(tasks::create_handler)
                .delete(tasks::delete_handler),
        )
        .route("/api/task/instruct", get(tasks::instruct_handler))
        .route(
            "/api/auto/check2",
            get(auto_check::list_handler).post(auto_check::save_handler),
        )
        .route("/webhook", post(webhook::handler))
        .route("/ws", get(ws::handler))
        .route("/api/share/keyCer", get(share::get_key_handler))
        .route("/api/share/keyCer/reflush", get(share::refresh_key_handler))
        .route("/api/share/keyCer/enable", get(share::enable_key_handler))
        .route(
            "/api/share/cluster/import",
            post(share::import_cluster_handler),
        )
        .route("/share/cluster", get(share::share_cluster_handler))
        .route("/api/dst/map/gen", get(map::generate_handler))
        .route("/api/dst/map/image", get(map::image_handler))
        .route(
            "/api/dst/map/has/walrusHut/plains",
            get(map::has_walrus_hut_plains_handler),
        )
        .route("/api/dst/map/session/file", get(map::session_file_handler))
        .route(
            "/api/dst/map/player/session/file",
            get(map::player_session_file_handler),
        )
        .route("/api/game/system/info", get(game::system_info_handler))
        .route(
            "/api/game/system/info/stream",
            get(streams::system_info_stream_handler),
        )
        .route("/api/game/log/stream", get(streams::log_stream_handler))
        .route(
            "/api/web/link",
            get(web_link::list_handler)
                .post(web_link::create_handler)
                .delete(web_link::delete_handler),
        )
        .with_state(state.clone());

    auth_routes
        .merge(app_routes)
        .layer(middleware::from_fn_with_state(state, auth_middleware))
}

/// Builds the make-service required for trusted peer-address extraction.
///
/// Server code should prefer this helper over calling `build_router` directly
/// when binding a socket. It guarantees the router is wrapped with
/// `into_make_service_with_connect_info::<SocketAddr>()`, which is required by
/// the Task 4 login handler and this module's white-admin-IP middleware.
pub fn build_connect_info_service(
    state: AppState,
) -> IntoMakeServiceWithConnectInfo<Router, SocketAddr> {
    build_router(state).into_make_service_with_connect_info::<SocketAddr>()
}

async fn hello_handler() -> &'static str {
    tracing::trace!("served hello route");
    "Hello! Dont starve together"
}

async fn login_route_handler(
    State(state): State<AuthState>,
    request: Request<Body>,
) -> AppResult<crate::web::handlers::auth::AuthResponse> {
    let (parts, body) = request.into_parts();
    let trusted_peer_addr = parts.extensions.get::<ConnectInfo<SocketAddr>>().cloned();
    let headers = parts.headers.clone();
    let request = Request::from_parts(parts, body);
    let Json(credentials) = Json::<UserCredentials>::from_request(request, &())
        .await
        .map_err(|_| {
            tracing::warn!("rejected malformed login request");
            AppError::bad_request("invalid login request")
        })?;

    login_handler(State(state), trusted_peer_addr, headers, Json(credentials)).await
}

async fn auth_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path().to_owned();
    if is_whitelisted_path(&path) {
        tracing::trace!(path = %path, "auth middleware allowed whitelisted path");
        return next.run(request).await;
    }

    if let Some(session_id) = crate::web::handlers::auth::extract_token_cookie(request.headers()) {
        if state.sessions.validate(&session_id).is_some() {
            tracing::trace!(path = %path, "auth middleware accepted session cookie");
            return next.run(request).await;
        }

        tracing::warn!(path = %path, "auth middleware rejected invalid session cookie");
    }

    let trusted_peer_addr = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(addr)| addr.to_string());
    if trusted_peer_addr
        .as_deref()
        .is_some_and(|addr| is_white_ip(addr, state.config.white_admin_ip.as_deref()))
    {
        tracing::info!(path = %path, "auth middleware allowed trusted white-admin-ip request");
        return next.run(request).await;
    }

    tracing::warn!(path = %path, "auth middleware rejected unauthenticated api request");
    StatusCode::UNAUTHORIZED.into_response()
}
