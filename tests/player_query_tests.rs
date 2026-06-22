use std::{
    fs,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{
        Method, Request, Response, StatusCode,
        header::{CONTENT_TYPE, COOKIE, SET_COOKIE},
    },
};
use dst_admin_rust::{
    domain::auth::SessionStore,
    infra::command::{CommandOutput, FakeCommandRunner},
    infra::config::{AppConfig, AutoUpdateModinfoConfig},
    infra::db::{connect_sqlite_memory, migrate},
    infra::process::{ProcessSnapshot, ProcessSnapshotProvider},
    web::app::{AppState, build_router},
};
use serde_json::{Value, json};
use tempfile::{TempDir, tempdir};
use tower::ServiceExt;

#[tokio::test]
async fn online_players_route_sends_go_lua_command_and_parses_recent_log_lines() {
    let runner = FakeCommandRunner::new(vec![CommandOutput::success(Vec::new(), Vec::new())]);
    let processes =
        FakeProcessSnapshotProvider::new(vec![process_snapshot("ClusterPlayers", "Master")]);
    let (app, dir, runner) = test_router(runner, processes).await;
    let cookie = login(&app).await;
    write_dst_config(dir.path(), "ClusterPlayers");
    write_master_log(
        dir.path(),
        "[00:00:00]: player: {[1700000000] [1] [12] [KU_abc123] [Alice] [wilson]}\n",
    );

    let response = send(
        &app,
        Method::GET,
        "/api/game/8level/players?levelName=Master",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!({
            "code": 200,
            "msg": "success",
            "data": [{
                "key": "1",
                "day": "12",
                "name": "Alice",
                "kuId": "KU_abc123",
                "role": "wilson"
            }]
        })
    );
    let calls = runner.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].program(), "screen");
    assert_eq!(calls[0].args()[1], "DST_8level_ClusterPlayers_Master");
    assert!(
        calls[0]
            .args()
            .last()
            .expect("screen stuff argument")
            .contains("AllPlayers")
    );
}

#[tokio::test]
async fn stopped_level_returns_empty_players_without_screen_command() {
    let runner = FakeCommandRunner::default();
    let (app, dir, runner) = test_router(runner, FakeProcessSnapshotProvider::default()).await;
    let cookie = login(&app).await;
    write_dst_config(dir.path(), "ClusterPlayers");
    write_master_log(
        dir.path(),
        "[00:00:00]: player: {[1700000000] [1] [12] [KU_abc123] [Alice] [wilson]}\n",
    );

    let response = send(
        &app,
        Method::GET,
        "/api/game/8level/players?levelName=Master",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await["data"], json!([]));
    assert!(runner.calls().is_empty());
}

#[tokio::test]
async fn master_player_route_defaults_to_master_session() {
    let runner = FakeCommandRunner::new(vec![CommandOutput::success(Vec::new(), Vec::new())]);
    let processes =
        FakeProcessSnapshotProvider::new(vec![process_snapshot("ClusterPlayers", "Master")]);
    let (app, dir, runner) = test_router(runner, processes).await;
    let cookie = login(&app).await;
    write_dst_config(dir.path(), "ClusterPlayers");
    write_master_log(
        dir.path(),
        "[00:00:00]: player: {[1700000000] [2] [34] [KU_def456] [Bob] [wendy]}\n",
    );

    let response = send(&app, Method::GET, "/api/game/player", None, Some(&cookie)).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await["data"][0]["kuId"],
        "KU_def456"
    );
    let calls = runner.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].args()[1], "DST_8level_ClusterPlayers_Master");
}

#[tokio::test]
async fn all_online_players_route_uses_master_session_and_client_table_command() {
    let runner = FakeCommandRunner::new(vec![CommandOutput::success(Vec::new(), Vec::new())]);
    let processes =
        FakeProcessSnapshotProvider::new(vec![process_snapshot("ClusterPlayers", "Master")]);
    let (app, dir, runner) = test_router(runner, processes).await;
    let cookie = login(&app).await;
    write_dst_config(dir.path(), "ClusterPlayers");
    write_master_log(
        dir.path(),
        "[00:00:00]: player: {[1700000000] [3] [56] [KU_all789] [Cara] [willow]}\n",
    );

    let response = send(
        &app,
        Method::GET,
        "/api/game/8level/players/all",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await["data"][0]["kuId"],
        "KU_all789"
    );
    let calls = runner.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].args()[1], "DST_8level_ClusterPlayers_Master");
    let command = calls[0].args().last().expect("screen stuff argument");
    assert!(command.contains("TheNet:GetClientTable"));
    assert!(!command.contains("AllPlayers"));
}

#[cfg(unix)]
#[tokio::test]
async fn online_players_route_rejects_symlinked_log_directory_without_parsing_escape() {
    let runner = FakeCommandRunner::new(vec![CommandOutput::success(Vec::new(), Vec::new())]);
    let processes =
        FakeProcessSnapshotProvider::new(vec![process_snapshot("ClusterPlayers", "Master")]);
    let (app, dir, _runner) = test_router(runner, processes).await;
    let cookie = login(&app).await;
    write_dst_config(dir.path(), "ClusterPlayers");
    let escape_dir = dir.path().join("escape");
    fs::create_dir_all(&escape_dir).unwrap();
    fs::write(
        escape_dir.join("server_log.txt"),
        "[00:00:00]: player: {[1700000000] [4] [78] [KU_escape] [Eve] [waxwell]}\n",
    )
    .unwrap();
    let cluster_dir = dir.path().join(".klei/DoNotStarveTogether/ClusterPlayers");
    fs::create_dir_all(&cluster_dir).unwrap();
    std::os::unix::fs::symlink(&escape_dir, cluster_dir.join("Master")).unwrap();

    let response = send(
        &app,
        Method::GET,
        "/api/game/8level/players?levelName=Master",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await["data"], json!([]));
}

async fn test_router(
    runner: FakeCommandRunner,
    processes: FakeProcessSnapshotProvider,
) -> (Router, TempDir, FakeCommandRunner) {
    let dir = tempdir().unwrap();
    write_password_file(dir.path());
    let pool = connect_sqlite_memory().await.unwrap();
    migrate(&pool).await.unwrap();
    let state = AppState::new_with_command_runner_and_process_snapshot_provider(
        test_config(),
        pool,
        SessionStore::new(),
        dir.path(),
        runner.clone(),
        processes,
    )
    .with_player_query_delay(Duration::ZERO)
    .with_player_query_marker_override("1700000000");
    (build_router(state), dir, runner)
}

#[derive(Clone, Debug, Default)]
struct FakeProcessSnapshotProvider {
    snapshots: Vec<ProcessSnapshot>,
    calls: Arc<AtomicUsize>,
}

impl FakeProcessSnapshotProvider {
    fn new(snapshots: Vec<ProcessSnapshot>) -> Self {
        Self {
            snapshots,
            calls: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl ProcessSnapshotProvider for FakeProcessSnapshotProvider {
    fn snapshots(&self) -> std::io::Result<Vec<ProcessSnapshot>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.snapshots.clone())
    }
}

fn process_snapshot(cluster_name: &str, level_name: &str) -> ProcessSnapshot {
    ProcessSnapshot {
        pid: Some(1000),
        cpu_usage: "0.1".to_owned(),
        mem_usage: "0.2".to_owned(),
        virtual_size: "10".to_owned(),
        resident_set_size: "20".to_owned(),
        command: format!(
            "/srv/dst/bin/dontstarve_dedicated_server_nullrenderer -console -cluster {cluster_name} -shard {level_name}"
        ),
    }
}

fn test_config() -> AppConfig {
    AppConfig {
        bind_address: "127.0.0.1".to_owned(),
        port: "0".to_owned(),
        path: String::new(),
        data_dir: String::new(),
        database: ":memory:".to_owned(),
        steamcmd: String::new(),
        steam_api_key: None,
        flag: String::new(),
        wan_ip: String::new(),
        white_admin_ip: None,
        token: Some("player-token".to_owned()),
        dst_version_url: "https://example.test/version".to_owned(),
        auto_update_modinfo: AutoUpdateModinfoConfig {
            enable: false,
            check_interval: 5,
            update_check_interval: 10,
        },
        dst_cli_port: String::new(),
    }
}

fn write_password_file(root: &Path) {
    fs::write(
        root.join("password.txt"),
        "username=admin\npassword=123456\ndisplayName=Admin\nphotoURL=https://example.test/avatar.png\n",
    )
    .unwrap();
}

fn write_dst_config(root: &Path, cluster: &str) {
    fs::write(
        root.join("dst_config"),
        format!(
            "steamcmd=/steam\nforce_install_dir={}\ncluster={cluster}\nbackup={}\nmod_download_path={}\nbin=64\nbeta=0\n",
            root.join("server").display(),
            root.join("backup").display(),
            root.join("mods").display()
        ),
    )
    .unwrap();
}

fn write_master_log(root: &Path, contents: &str) {
    let log_dir = root
        .join(".klei")
        .join("DoNotStarveTogether")
        .join("ClusterPlayers")
        .join("Master");
    fs::create_dir_all(&log_dir).unwrap();
    fs::write(log_dir.join("server_log.txt"), contents).unwrap();
}

async fn login(app: &Router) -> String {
    let response = send(
        app,
        Method::POST,
        "/api/login",
        Some(json!({"username": "admin", "password": "123456"})),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    response
        .headers()
        .get(SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}

async fn send(
    app: &Router,
    method: Method,
    uri: &str,
    json_body: Option<Value>,
    cookie: Option<&str>,
) -> Response<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if json_body.is_some() {
        builder = builder.header(CONTENT_TYPE, "application/json");
    }
    if let Some(cookie) = cookie {
        builder = builder.header(COOKIE, cookie);
    }

    let body = json_body
        .map(|value| Body::from(value.to_string()))
        .unwrap_or_else(Body::empty);

    app.clone()
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap()
}

async fn response_json(response: Response<Body>) -> Value {
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("expected JSON response for status {status}: {error}"))
}
