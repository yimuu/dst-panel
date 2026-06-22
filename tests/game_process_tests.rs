use std::{fs, io, path::Path};

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
    domain::game::{cpu_info_from_proc_stat_pair, platform_from_os_release},
    infra::command::TokioCommandRunner,
    infra::config::{AppConfig, AutoUpdateModinfoConfig},
    infra::db::{connect_sqlite_memory, migrate},
    infra::process::{ProcessSnapshot, ProcessSnapshotProvider},
    web::app::{AppState, build_router},
};
use serde_json::{Value, json};
use tempfile::{TempDir, tempdir};
use tower::ServiceExt;

#[tokio::test]
async fn game_status_route_reports_level_files_without_starting_dst() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    write_dst_config(dir.path(), "ClusterStatusNoProc");
    write_level_fixture(dir.path(), "ClusterStatusNoProc");

    let response = send(
        &app,
        Method::GET,
        "/api/game/8level/status",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["code"], 200);
    assert_eq!(body["msg"], "success");
    assert_eq!(body["data"].as_array().unwrap().len(), 2);
    assert_eq!(body["data"][0]["levelName"], "森林");
    assert_eq!(body["data"][0]["is_master"], true);
    assert_eq!(body["data"][0]["uuid"], "Master");
    assert_eq!(body["data"][0]["status"], false);
    assert_eq!(
        body["data"][0]["Ps"],
        json!({"cpuUage": "", "memUage": "", "VSZ": "", "RSS": ""})
    );
    assert_eq!(body["data"][0]["leveldataoverride"], "return {}");
    assert_eq!(body["data"][0]["modoverrides"], "return {}");
    assert_eq!(body["data"][0]["server_ini"]["server_port"], 10999);
    assert_eq!(body["data"][1]["levelName"], "洞穴");
    assert_eq!(body["data"][1]["is_master"], false);
    assert_eq!(body["data"][1]["uuid"], "Caves");
    assert_eq!(body["data"][1]["status"], false);
    assert_eq!(
        body["data"][1]["Ps"],
        json!({"cpuUage": "", "memUage": "", "VSZ": "", "RSS": ""})
    );
}

#[tokio::test]
async fn game_status_route_uses_injected_process_snapshot_provider() {
    let matching_process = ProcessSnapshot {
        pid: Some(4242),
        cpu_usage: "12.5".to_owned(),
        mem_usage: "6.25".to_owned(),
        virtual_size: "123456".to_owned(),
        resident_set_size: "65432".to_owned(),
        command: "/srv/dst/bin/dontstarve_dedicated_server_nullrenderer_x64 -cluster ClusterInjectedStatus -shard Master".to_owned(),
    };
    let (app, dir) = test_router_with_processes(vec![matching_process]).await;
    let cookie = login(&app).await;
    write_dst_config(dir.path(), "ClusterInjectedStatus");
    write_level_fixture(dir.path(), "ClusterInjectedStatus");

    let response = send(
        &app,
        Method::GET,
        "/api/game/8level/status",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["code"], 200);
    assert_eq!(body["data"][0]["uuid"], "Master");
    assert_eq!(body["data"][0]["status"], true);
    assert_eq!(
        body["data"][0]["Ps"],
        json!({"cpuUage": "12.5", "memUage": "6.25", "VSZ": "123456", "RSS": "65432"})
    );
}

#[tokio::test]
async fn game_status_route_rejects_invalid_cluster_before_process_matching() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    fs::write(
        dir.path().join("dst_config"),
        "steamcmd=/steam\ncluster=ClusterStatus;touch-pwned\nbin=64\nbeta=0\n",
    )
    .unwrap();

    let response = send(
        &app,
        Method::GET,
        "/api/game/8level/status",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn game_status_route_rejects_invalid_level_uuid_before_process_matching() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    write_dst_config(dir.path(), "ClusterBadLevelArg");
    let cluster_dir = dir
        .path()
        .join(".klei/DoNotStarveTogether/ClusterBadLevelArg");
    let level_dir = cluster_dir.join("Master;touch-pwned");
    fs::create_dir_all(&level_dir).unwrap();
    fs::write(
        cluster_dir.join("level.json"),
        r#"{"levelList":[{"name":"bad","file":"Master;touch-pwned"}]}"#,
    )
    .unwrap();
    fs::write(level_dir.join("leveldataoverride.lua"), "return {}").unwrap();
    fs::write(level_dir.join("modoverrides.lua"), "return {}").unwrap();
    fs::write(
        level_dir.join("server.ini"),
        "[NETWORK]\nserver_port = 10999\n[SHARD]\nis_master = true\nname = Master\nid = 10000\n[ACCOUNT]\nencode_user_path = true\n",
    )
    .unwrap();

    let response = send(
        &app,
        Method::GET,
        "/api/game/8level/status",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn game_status_route_does_not_initialize_missing_level_index() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    write_dst_config(dir.path(), "ClusterNoStatusIndex");
    let cluster_dir = dir
        .path()
        .join(".klei/DoNotStarveTogether/ClusterNoStatusIndex");

    let response = send(
        &app,
        Method::GET,
        "/api/game/8level/status",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body, json!({"code": 200, "msg": "success", "data": []}));
    assert!(
        !cluster_dir.exists(),
        "status is read-only and must not create a default cluster/level skeleton"
    );
}

#[tokio::test]
async fn system_info_route_returns_go_compatible_dashboard_shape() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    write_dst_config(dir.path(), "ClusterSystemInfo");

    let response = send(
        &app,
        Method::GET,
        "/api/game/system/info",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["code"], 200);
    assert_eq!(body["msg"], "success");
    assert!(body["data"]["host"]["os"].is_string());
    assert!(body["data"]["host"]["hostname"].is_string());
    assert!(body["data"]["host"]["platform"].is_string());
    assert!(body["data"]["host"]["kernelArch"].is_string());
    assert!(body["data"]["cpu"]["cores"].is_i64());
    assert!(body["data"]["cpu"]["cpuPercent"].is_array());
    assert!(body["data"]["cpu"]["cpuUsedPercent"].is_number());
    assert!(body["data"]["cpu"]["cpuUsed"].is_number());
    assert!(body["data"]["mem"]["total"].is_u64());
    assert!(body["data"]["mem"]["available"].is_u64());
    assert!(body["data"]["mem"]["used"].is_u64());
    assert!(body["data"]["mem"]["usedPercent"].is_number());
    assert!(body["data"]["disk"]["devices"].is_array());
    assert!(body["data"]["panelMemUsage"].is_u64());
    assert!(body["data"]["panelCpuUsage"].is_number());

    #[cfg(target_os = "linux")]
    {
        assert!(body["data"]["cpu"]["cores"].as_i64().unwrap() >= 1);
        assert!(body["data"]["cpu"]["cpuPercent"].as_array().unwrap().len() >= 1);
        assert!(body["data"]["mem"]["total"].as_u64().unwrap() > 0);
    }
}

#[test]
fn cpu_stat_parser_reports_aggregate_and_per_core_percentages() {
    let previous = "\
cpu  100 0 100 800 0 0 0 0 0 0\n\
cpu0 50 0 50 400 0 0 0 0 0 0\n\
cpu1 50 0 50 400 0 0 0 0 0 0\n";
    let current = "\
cpu  150 0 150 900 0 0 0 0 0 0\n\
cpu0 70 0 80 450 0 0 0 0 0 0\n\
cpu1 80 0 70 450 0 0 0 0 0 0\n";

    let info = cpu_info_from_proc_stat_pair(previous, current, 2);
    let value = serde_json::to_value(info).unwrap();

    assert_eq!(value["cores"], 2);
    assert_eq!(value["cpuPercent"], json!([50.0, 50.0]));
    assert_eq!(value["cpuUsedPercent"], 50.0);
    assert_eq!(value["cpuUsed"], 1.0);
}

#[test]
fn cpu_stat_parser_returns_empty_percentages_when_stat_rows_are_unavailable() {
    let info = cpu_info_from_proc_stat_pair("", "", 2);
    let value = serde_json::to_value(info).unwrap();

    assert_eq!(value["cores"], 2);
    assert_eq!(value["cpuPercent"], json!([]));
    assert_eq!(value["cpuUsedPercent"], 0.0);
    assert_eq!(value["cpuUsed"], 0.0);
}

#[test]
fn cpu_stat_parser_skips_rows_with_malformed_numeric_tokens() {
    let previous = "\
cpu  100 0 bad 800 0 0 0 0 0 0\n\
cpu0 50 0 nope 400 0 0 0 0 0 0\n\
cpu1 50 0 50 400 0 0 0 0 0 0\n";
    let current = "\
cpu  150 0 150 900 0 0 0 0 0 0\n\
cpu0 70 0 80 450 0 0 0 0 0 0\n\
cpu1 80 0 70 450 0 0 0 0 0 0\n";

    let info = cpu_info_from_proc_stat_pair(previous, current, 2);
    let value = serde_json::to_value(info).unwrap();

    assert_eq!(value["cores"], 2);
    assert_eq!(value["cpuPercent"], json!([0.0, 50.0]));
    assert_eq!(value["cpuUsedPercent"], 0.0);
    assert_eq!(value["cpuUsed"], 0.0);
}

#[test]
fn cpu_stat_parser_skips_rows_when_jiffy_totals_overflow() {
    let previous = "\
cpu  18446744073709551615 1 1 1 0 0 0 0 0 0\n\
cpu0 18446744073709551615 1 1 1 0 0 0 0 0 0\n\
cpu1 50 0 50 400 0 0 0 0 0 0\n";
    let current = "\
cpu  150 0 150 900 0 0 0 0 0 0\n\
cpu0 70 0 80 450 0 0 0 0 0 0\n\
cpu1 80 0 70 450 0 0 0 0 0 0\n";

    let info = cpu_info_from_proc_stat_pair(previous, current, 2);
    let value = serde_json::to_value(info).unwrap();

    assert_eq!(value["cores"], 2);
    assert_eq!(value["cpuPercent"], json!([0.0, 50.0]));
    assert_eq!(value["cpuUsedPercent"], 0.0);
    assert_eq!(value["cpuUsed"], 0.0);
}

#[test]
fn cpu_stat_parser_skips_unreasonable_cpu_indexes_without_allocating() {
    let previous = "\
cpu  100 0 100 800 0 0 0 0 0 0\n\
cpu0 50 0 50 400 0 0 0 0 0 0\n\
cpu1000000000 50 0 50 400 0 0 0 0 0 0\n";
    let current = "\
cpu  150 0 150 900 0 0 0 0 0 0\n\
cpu0 70 0 80 450 0 0 0 0 0 0\n\
cpu1000000000 70 0 80 450 0 0 0 0 0 0\n";

    let info = cpu_info_from_proc_stat_pair(previous, current, 1);
    let value = serde_json::to_value(info).unwrap();

    assert_eq!(value["cores"], 1);
    assert_eq!(value["cpuPercent"], json!([50.0]));
    assert_eq!(value["cpuUsedPercent"], 50.0);
    assert_eq!(value["cpuUsed"], 0.5);
}

#[test]
fn host_platform_parser_reads_os_release_id() {
    let platform = platform_from_os_release("NAME=\"Ubuntu\"\nID=ubuntu\n");

    assert_eq!(platform, "ubuntu");
}

async fn test_router() -> (Router, TempDir) {
    test_router_with_processes(Vec::new()).await
}

async fn test_router_with_processes(snapshots: Vec<ProcessSnapshot>) -> (Router, TempDir) {
    let dir = tempdir().unwrap();
    write_password_file(dir.path());
    let pool = connect_sqlite_memory().await.unwrap();
    migrate(&pool).await.unwrap();
    let state = AppState::new_with_command_runner_and_process_snapshot_provider(
        test_config(),
        pool,
        SessionStore::new(),
        dir.path(),
        TokioCommandRunner::new(),
        FakeProcessSnapshotProvider { snapshots },
    );
    (build_router(state), dir)
}

#[derive(Debug, Clone)]
struct FakeProcessSnapshotProvider {
    snapshots: Vec<ProcessSnapshot>,
}

impl ProcessSnapshotProvider for FakeProcessSnapshotProvider {
    fn snapshots(&self) -> io::Result<Vec<ProcessSnapshot>> {
        Ok(self.snapshots.clone())
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

fn write_level_fixture(root: &Path, cluster: &str) {
    let cluster_dir = root.join(".klei/DoNotStarveTogether").join(cluster);
    fs::create_dir_all(cluster_dir.join("Master")).unwrap();
    fs::create_dir_all(cluster_dir.join("Caves")).unwrap();
    fs::write(
        cluster_dir.join("level.json"),
        r#"{"levelList":[{"name":"森林","file":"Master"},{"name":"洞穴","file":"Caves"}]}"#,
    )
    .unwrap();
    fs::write(
        cluster_dir.join("Master/leveldataoverride.lua"),
        "return {}",
    )
    .unwrap();
    fs::write(cluster_dir.join("Master/modoverrides.lua"), "return {}").unwrap();
    fs::write(
        cluster_dir.join("Master/server.ini"),
        "[NETWORK]\nserver_port = 10999\n[SHARD]\nis_master = true\nname = Master\nid = 10000\n[ACCOUNT]\nencode_user_path = true\n",
    )
    .unwrap();
    fs::write(cluster_dir.join("Caves/leveldataoverride.lua"), "return {}").unwrap();
    fs::write(cluster_dir.join("Caves/modoverrides.lua"), "return {}").unwrap();
    fs::write(
        cluster_dir.join("Caves/server.ini"),
        "[NETWORK]\nserver_port = 10998\n[SHARD]\nis_master = false\nname = Caves\nid = 10010\n[ACCOUNT]\nencode_user_path = true\n[STEAM]\nauthentication_port = 8766\nmaster_server_port = 27016\n",
    )
    .unwrap();
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
