use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{
        Method, Request, Response, StatusCode,
        header::{CONTENT_DISPOSITION, CONTENT_TYPE, COOKIE, SET_COOKIE},
    },
};
use dst_admin_rust::{
    domain::auth::SessionStore,
    infra::config::{AppConfig, AutoUpdateModinfoConfig},
    infra::db::{connect_sqlite_memory, migrate},
    infra::logging::DEFAULT_LOG_FILE,
    web::app::{AppState, build_router},
};
use serde_json::{Value, json};
use tempfile::{TempDir, tempdir};
use tower::ServiceExt;

#[tokio::test]
async fn level_server_log_routes_return_recent_lines_in_reverse_order() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterLog");
    let master_dir = cluster_dir.join("Master");
    fs::create_dir_all(&master_dir).unwrap();
    fs::write(master_dir.join("server_log.txt"), "one\ntwo\nthree\nfour").unwrap();
    fs::write(
        master_dir.join("server_chat_log.txt"),
        "chat1\nchat2\nchat3",
    )
    .unwrap();

    let response = send(
        &app,
        Method::GET,
        "/api/game/level/server/log?levelName=Master&lines=2",
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!({"code": 200, "msg": "success", "data": ["four", "three"]})
    );

    let response = send(
        &app,
        Method::GET,
        "/api/game/level/server/chat/log?levelName=Master&lines=2",
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!({"code": 200, "msg": "success", "data": ["chat3", "chat2"]})
    );
}

#[tokio::test]
async fn level_server_log_routes_preserve_go_reverse_read_newline_shape() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterLogNewlineShape");
    let master_dir = cluster_dir.join("Master");
    fs::create_dir_all(&master_dir).unwrap();
    fs::write(master_dir.join("server_log.txt"), "one\r\ntwo\r\nthree\r\n").unwrap();

    let response = send(
        &app,
        Method::GET,
        "/api/game/level/server/log?levelName=Master&lines=3",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!({"code": 200, "msg": "success", "data": ["", "three\r", "two\r"]})
    );
}

#[tokio::test]
async fn level_server_log_route_reads_across_large_tail_window_without_truncating_lines() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterLogLargeTail");
    let master_dir = cluster_dir.join("Master");
    fs::create_dir_all(&master_dir).unwrap();
    let long_line = "x".repeat(1100 * 1024);
    fs::write(
        master_dir.join("server_log.txt"),
        format!("{long_line}\nlast-one\nlast-two"),
    )
    .unwrap();

    let response = send(
        &app,
        Method::GET,
        "/api/game/level/server/log?levelName=Master&lines=3",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!({"code": 200, "msg": "success", "data": ["last-two", "last-one", long_line]})
    );
}

#[tokio::test]
async fn level_server_log_routes_missing_files_return_empty_array() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterLogMissing");
    fs::create_dir_all(cluster_dir.join("Master")).unwrap();

    let response = send(
        &app,
        Method::GET,
        "/api/game/level/server/log?levelName=Master",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!({"code": 200, "msg": "success", "data": []})
    );

    let response = send(
        &app,
        Method::GET,
        "/api/game/level/server/chat/log?levelName=Master",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!({"code": 200, "msg": "success", "data": []})
    );
}

#[tokio::test]
async fn level_server_log_routes_reject_traversal_without_reading_escape_files() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    write_cluster_fixture(dir.path(), "ClusterLogReject");
    fs::write(dir.path().join("secret-log.txt"), "secret").unwrap();

    let response = send(
        &app,
        Method::GET,
        "/api/game/level/server/log?levelName=..%2F..%2Fsecret-log.txt&lines=5",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response_json(response).await;
    assert_eq!(body["code"], 400);
    assert_ne!(body["msg"], "secret");
}

#[tokio::test]
async fn panel_log_route_reads_legacy_dst_admin_log_file() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    fs::write(
        dir.path().join(DEFAULT_LOG_FILE),
        "panel-one\npanel-two\npanel-three",
    )
    .unwrap();

    let response = send(
        &app,
        Method::GET,
        "/api/game/dst-admin-go/log?lines=2",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!({"code": 200, "msg": "success", "data": ["panel-three", "panel-two"]})
    );
}

#[tokio::test]
async fn download_log_routes_stream_raw_files_with_go_headers() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterDownload");
    let master_dir = cluster_dir.join("Master");
    fs::create_dir_all(&master_dir).unwrap();
    fs::write(master_dir.join("server_log.txt"), "server-log-body").unwrap();
    fs::write(dir.path().join(DEFAULT_LOG_FILE), "panel-log-body").unwrap();

    let response = send(
        &app,
        Method::GET,
        "/api/game/level/server/download?levelName=Master&fileName=server_log.txt",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(CONTENT_TYPE).unwrap(),
        "application/octet-stream"
    );
    assert_eq!(
        response.headers().get(CONTENT_DISPOSITION).unwrap(),
        "attachment; filename=\"server_log.txt\""
    );
    assert_eq!(
        response.headers().get("content-transfer-encoding").unwrap(),
        "binary"
    );
    assert_eq!(response.headers().get("content-length").unwrap(), "15");
    assert_eq!(response_bytes(response).await, b"server-log-body");

    let response = send(
        &app,
        Method::GET,
        "/api/game/dst-admin-go/log/download",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(CONTENT_DISPOSITION).unwrap(),
        "attachment; filename=\"dst-admin-go.log\""
    );
    assert_eq!(
        response.headers().get(CONTENT_TYPE).unwrap(),
        "application/octet-stream"
    );
    assert_eq!(
        response.headers().get("content-transfer-encoding").unwrap(),
        "binary"
    );
    assert_eq!(response.headers().get("content-length").unwrap(), "14");
    assert_eq!(response_bytes(response).await, b"panel-log-body");
}

#[tokio::test]
async fn download_level_log_does_not_stream_bytes_appended_after_headers() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterDownloadStableLength");
    let master_dir = cluster_dir.join("Master");
    fs::create_dir_all(&master_dir).unwrap();
    let log_path = master_dir.join("server_log.txt");
    fs::write(&log_path, "short").unwrap();

    let response = send(
        &app,
        Method::GET,
        "/api/game/level/server/download?levelName=Master&fileName=server_log.txt",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().get("content-length").unwrap(), "5");
    OpenOptions::new()
        .append(true)
        .open(&log_path)
        .unwrap()
        .write_all(b"-appended-after-response")
        .unwrap();
    assert_eq!(response_bytes(response).await, b"short");
}

#[tokio::test]
async fn download_level_log_rejects_unsafe_file_names() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    write_cluster_fixture(dir.path(), "ClusterDownloadReject");
    fs::write(dir.path().join("secret-log.txt"), "secret").unwrap();

    let response = send(
        &app,
        Method::GET,
        "/api/game/level/server/download?levelName=Master&fileName=..%2F..%2Fsecret-log.txt",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response_json(response).await;
    assert_eq!(body["code"], 400);
    assert_ne!(body["msg"], "secret");
}

#[tokio::test]
async fn level_server_log_route_returns_413_when_snapshot_exceeds_safety_limit() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterLogTooLarge");
    let master_dir = cluster_dir.join("Master");
    fs::create_dir_all(&master_dir).unwrap();
    fs::write(
        master_dir.join("server_log.txt"),
        "x".repeat(8 * 1024 * 1024 + 1),
    )
    .unwrap();

    let response = send(
        &app,
        Method::GET,
        "/api/game/level/server/log?levelName=Master&lines=1",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let body = response_json(response).await;
    assert_eq!(body["code"], 413);
    assert_eq!(body["msg"], "log snapshot is too large");
}

#[cfg(unix)]
#[tokio::test]
async fn level_server_log_routes_reject_symlinked_klei_cluster_and_level_paths() {
    use std::os::unix::fs::symlink;

    let (app, dir) = test_router().await;
    let cookie = login(&app).await;

    write_dst_config(dir.path(), "ClusterLogSymlinkKlei");
    let escaped_klei = dir.path().join("escaped-klei");
    fs::create_dir_all(escaped_klei.join("DoNotStarveTogether/ClusterLogSymlinkKlei/Master"))
        .unwrap();
    fs::write(
        escaped_klei.join("DoNotStarveTogether/ClusterLogSymlinkKlei/Master/server_log.txt"),
        "secret-klei",
    )
    .unwrap();
    symlink(&escaped_klei, dir.path().join(".klei")).unwrap();
    assert_rejects_log_escape(
        &app,
        &cookie,
        "/api/game/level/server/log?levelName=Master&lines=1",
        "secret-klei",
    )
    .await;
    fs::remove_file(dir.path().join(".klei")).unwrap();

    write_dst_config(dir.path(), "ClusterLogSymlinkCluster");
    let klei_root = dir.path().join(".klei/DoNotStarveTogether");
    fs::create_dir_all(&klei_root).unwrap();
    let escaped_cluster = dir.path().join("escaped-cluster");
    fs::create_dir_all(escaped_cluster.join("Master")).unwrap();
    fs::write(
        escaped_cluster.join("Master/server_log.txt"),
        "secret-cluster",
    )
    .unwrap();
    symlink(&escaped_cluster, klei_root.join("ClusterLogSymlinkCluster")).unwrap();
    assert_rejects_log_escape(
        &app,
        &cookie,
        "/api/game/level/server/log?levelName=Master&lines=1",
        "secret-cluster",
    )
    .await;
    fs::remove_file(klei_root.join("ClusterLogSymlinkCluster")).unwrap();

    write_dst_config(dir.path(), "ClusterLogSymlinkLevel");
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterLogSymlinkLevel");
    let escaped_level = dir.path().join("escaped-level");
    fs::create_dir_all(&escaped_level).unwrap();
    fs::write(escaped_level.join("server_log.txt"), "secret-level").unwrap();
    symlink(&escaped_level, cluster_dir.join("Master")).unwrap();
    assert_rejects_log_escape(
        &app,
        &cookie,
        "/api/game/level/server/log?levelName=Master&lines=1",
        "secret-level",
    )
    .await;
}

async fn test_router() -> (Router, TempDir) {
    let dir = tempdir().unwrap();
    write_password_file(dir.path());
    write_dst_config(dir.path(), "ClusterLog");
    let pool = connect_sqlite_memory().await.unwrap();
    migrate(&pool).await.unwrap();
    let state = AppState::new(test_config(), pool, SessionStore::new(), dir.path());
    (build_router(state), dir)
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
        token: Some("cluster-token".to_owned()),
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

fn write_cluster_fixture(root: &Path, cluster: &str) -> std::path::PathBuf {
    write_dst_config(root, cluster);
    let cluster_dir = root.join(".klei/DoNotStarveTogether").join(cluster);
    fs::create_dir_all(&cluster_dir).unwrap();
    cluster_dir
}

async fn assert_rejects_log_escape(app: &Router, cookie: &str, uri: &str, secret: &str) {
    let response = send(app, Method::GET, uri, None, Some(cookie)).await;
    assert_ne!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_ne!(body["msg"], secret);
    assert_ne!(body["data"], secret);
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
    let bytes = response_bytes(response).await;
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("expected JSON response for status {status}: {error}"))
}

async fn response_bytes(response: Response<Body>) -> Vec<u8> {
    to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec()
}
