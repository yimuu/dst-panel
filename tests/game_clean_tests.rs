use std::{fs, path::Path};

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
    infra::config::{AppConfig, AutoUpdateModinfoConfig},
    infra::db::{connect_sqlite_memory, migrate},
    web::app::{AppState, build_router},
};
use serde_json::{Value, json};
use tempfile::{TempDir, tempdir};
use tower::ServiceExt;

#[tokio::test]
async fn clean_level_route_removes_selected_level_runtime_files_only() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterClean");
    write_runtime_files(&cluster_dir, "Master");
    write_runtime_files(&cluster_dir, "Caves");
    fs::write(cluster_dir.join("Master/server.ini"), "keep").unwrap();

    let response = send(
        &app,
        Method::GET,
        "/api/game/clean/level?level=Master",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!({"code": 200, "msg": "success", "data": null})
    );
    assert_cleaned(&cluster_dir, "Master");
    assert!(cluster_dir.join("Master/server.ini").exists());
    assert!(cluster_dir.join("Caves/save").exists());
    assert!(cluster_dir.join("Caves/server_log.txt").exists());
}

#[tokio::test]
async fn clean_level_route_accepts_repeated_level_query_parameters() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterCleanRepeated");
    write_runtime_files(&cluster_dir, "Master");
    write_runtime_files(&cluster_dir, "Caves");

    let response = send(
        &app,
        Method::GET,
        "/api/game/clean/level?level=Master&level=Caves",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_cleaned(&cluster_dir, "Master");
    assert_cleaned(&cluster_dir, "Caves");
}

#[tokio::test]
async fn clean_level_route_without_level_query_is_noop() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterCleanNoop");
    write_runtime_files(&cluster_dir, "Master");

    let response = send(
        &app,
        Method::GET,
        "/api/game/clean/level",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(cluster_dir.join("Master/backup").exists());
    assert!(cluster_dir.join("Master/save").exists());
    assert!(cluster_dir.join("Master/server_log.txt").exists());
    assert!(cluster_dir.join("Master/server_chat_log.txt").exists());
}

#[tokio::test]
async fn clean_level_route_rejects_traversal_without_deleting_escape_target() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterCleanReject");
    let outside = dir.path().join("outside-save");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("keep.txt"), "keep").unwrap();
    write_runtime_files(&cluster_dir, "Master");

    let response = send(
        &app,
        Method::GET,
        "/api/game/clean/level?level=../outside-save",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        fs::read_to_string(outside.join("keep.txt")).unwrap(),
        "keep"
    );
    assert!(cluster_dir.join("Master/save").exists());
}

#[tokio::test]
async fn clean_level_route_validates_all_repeated_levels_before_deleting() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterCleanMixedReject");
    write_runtime_files(&cluster_dir, "Master");
    write_runtime_files(&cluster_dir, "Caves");

    let response = send(
        &app,
        Method::GET,
        "/api/game/clean/level?level=Master&level=..%2FCaves",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(cluster_dir.join("Master/save").exists());
    assert!(cluster_dir.join("Master/server_log.txt").exists());
    assert!(cluster_dir.join("Caves/save").exists());
    assert!(cluster_dir.join("Caves/server_log.txt").exists());
}

#[tokio::test]
async fn clean_all_level_route_uses_level_index_and_leaves_config_files() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterCleanAll");
    write_runtime_files(&cluster_dir, "Master");
    write_runtime_files(&cluster_dir, "Caves");
    fs::write(cluster_dir.join("Master/modoverrides.lua"), "return {}").unwrap();
    fs::write(cluster_dir.join("Caves/server.ini"), "keep").unwrap();

    let response = send(
        &app,
        Method::GET,
        "/api/game/clean/level/all",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_cleaned(&cluster_dir, "Master");
    assert_cleaned(&cluster_dir, "Caves");
    assert!(cluster_dir.join("Master/modoverrides.lua").exists());
    assert!(cluster_dir.join("Caves/server.ini").exists());
}

#[tokio::test]
async fn clean_all_level_route_ignores_missing_or_empty_level_index() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;

    let missing_index_dir =
        write_cluster_without_level_index(dir.path(), "ClusterCleanMissingIndex");
    write_runtime_files(&missing_index_dir, "Master");
    write_runtime_files(&missing_index_dir, "Caves");

    let response = send(
        &app,
        Method::GET,
        "/api/game/clean/level/all",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(missing_index_dir.join("Master/save").exists());
    assert!(missing_index_dir.join("Master/server_log.txt").exists());
    assert!(missing_index_dir.join("Caves/save").exists());
    assert!(missing_index_dir.join("Caves/server_log.txt").exists());

    let empty_index_dir =
        write_cluster_with_level_index(dir.path(), "ClusterCleanEmptyIndex", "{}");
    write_runtime_files(&empty_index_dir, "Master");
    write_runtime_files(&empty_index_dir, "Caves");

    let response = send(
        &app,
        Method::GET,
        "/api/game/clean/level/all",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(empty_index_dir.join("Master/save").exists());
    assert!(empty_index_dir.join("Master/server_log.txt").exists());
    assert!(empty_index_dir.join("Caves/save").exists());
    assert!(empty_index_dir.join("Caves/server_log.txt").exists());
}

#[tokio::test]
async fn clean_world_route_removes_master_and_caves_save_directories() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterCleanWorld");
    write_runtime_files(&cluster_dir, "Master");
    write_runtime_files(&cluster_dir, "Caves");
    write_runtime_files(&cluster_dir, "Extra");
    fs::write(cluster_dir.join("Master/server.ini"), "keep").unwrap();

    let response = send(&app, Method::GET, "/api/game/clean", None, Some(&cookie)).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(!cluster_dir.join("Master/backup").exists());
    assert!(!cluster_dir.join("Master/save").exists());
    assert!(!cluster_dir.join("Caves/backup").exists());
    assert!(!cluster_dir.join("Caves/save").exists());
    assert!(cluster_dir.join("Master/server_log.txt").exists());
    assert!(cluster_dir.join("Master/server_chat_log.txt").exists());
    assert!(cluster_dir.join("Master/server.ini").exists());
    assert!(cluster_dir.join("Extra/save").exists());
}

async fn test_router() -> (Router, TempDir) {
    let dir = tempdir().unwrap();
    write_password_file(dir.path());
    write_dst_config(dir.path(), "ClusterClean");
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
    write_cluster_with_level_index(
        root,
        cluster,
        r#"{"levelList":[{"name":"森林","file":"Master"},{"name":"洞穴","file":"Caves"}]}"#,
    )
}

fn write_cluster_with_level_index(
    root: &Path,
    cluster: &str,
    level_index: &str,
) -> std::path::PathBuf {
    write_dst_config(root, cluster);
    let cluster_dir = root.join(".klei/DoNotStarveTogether").join(cluster);
    fs::create_dir_all(&cluster_dir).unwrap();
    fs::write(cluster_dir.join("level.json"), level_index).unwrap();
    cluster_dir
}

fn write_cluster_without_level_index(root: &Path, cluster: &str) -> std::path::PathBuf {
    write_dst_config(root, cluster);
    let cluster_dir = root.join(".klei/DoNotStarveTogether").join(cluster);
    fs::create_dir_all(&cluster_dir).unwrap();
    cluster_dir
}

fn write_runtime_files(cluster_dir: &Path, level: &str) {
    let level_dir = cluster_dir.join(level);
    fs::create_dir_all(level_dir.join("backup")).unwrap();
    fs::create_dir_all(level_dir.join("save")).unwrap();
    fs::write(level_dir.join("backup/slot"), "delete").unwrap();
    fs::write(level_dir.join("save/session"), "delete").unwrap();
    fs::write(level_dir.join("server_log.txt"), "delete").unwrap();
    fs::write(level_dir.join("server_chat_log.txt"), "delete").unwrap();
}

fn assert_cleaned(cluster_dir: &Path, level: &str) {
    let level_dir = cluster_dir.join(level);
    assert!(!level_dir.join("backup").exists());
    assert!(!level_dir.join("save").exists());
    assert!(!level_dir.join("server_log.txt").exists());
    assert!(!level_dir.join("server_chat_log.txt").exists());
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
