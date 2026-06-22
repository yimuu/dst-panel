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
    infra::command::{CommandSpec, FakeCommandRunner},
    infra::config::{AppConfig, AutoUpdateModinfoConfig},
    infra::db::{connect_sqlite_memory, migrate},
    web::app::{AppState, build_router},
};
use serde_json::{Value, json};
use tempfile::{TempDir, tempdir};
use tower::ServiceExt;

#[tokio::test]
async fn level_command_route_uses_screen_argv_and_go_envelope() {
    let (app, _dir, runner) = test_router().await;
    let cookie = login(&app).await;

    let response = send(
        &app,
        Method::POST,
        "/api/game/8level/command",
        Some(json!({"levelName": "Master", "command": "c_save(); print(\"ok\")"})),
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!({"code": 200, "msg": "success", "data": null})
    );
    let calls = runner.calls();
    assert_eq!(calls.len(), 1);
    assert_screen_call(
        &calls[0],
        "DST_8level_ClusterCommand_Master",
        "c_save(); print(\"ok\")\n",
    );
}

#[tokio::test]
async fn player_command_routes_validate_ku_id_and_use_screen_argv() {
    let (app, _dir, runner) = test_router().await;
    let cookie = login(&app).await;

    let response = send(
        &app,
        Method::GET,
        "/api/game/kick/player?kuId=KU_admin-1",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await["data"], Value::Null);
    let calls = runner.calls();
    assert_eq!(calls.len(), 2);
    assert_screen_call(
        &calls[0],
        "DST_8level_ClusterCommand_Master",
        "TheNet:Kick(\"KU_admin-1\")\n",
    );
    assert_screen_call(
        &calls[1],
        "DST_8level_ClusterCommand_Caves",
        "TheNet:Kick(\"KU_admin-1\")\n",
    );

    let rejected = send(
        &app,
        Method::GET,
        "/api/game/kill/player?kuId=KU_bad;touch-pwned",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(rejected.status(), StatusCode::BAD_REQUEST);
    assert_eq!(runner.calls().len(), 2);
}

#[tokio::test]
async fn broadcast_and_rollback_routes_escape_user_text_and_use_screen_argv() {
    let (app, _dir, runner) = test_router().await;
    let cookie = login(&app).await;

    let broadcast = send(
        &app,
        Method::GET,
        "/api/game/sent/broadcast?message=hi%22%29%3Bc_shutdown%28%29%3B--",
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(broadcast.status(), StatusCode::OK);

    let rollback = send(
        &app,
        Method::GET,
        "/api/game/rollback?dayNums=3",
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(rollback.status(), StatusCode::OK);

    let calls = runner.calls();
    assert_eq!(calls.len(), 6);
    assert_screen_call(
        &calls[0],
        "DST_8level_ClusterCommand_Master",
        "c_announce(\"hi\\\");c_shutdown();--\")\n",
    );
    assert_screen_call(
        &calls[1],
        "DST_8level_ClusterCommand_Caves",
        "c_announce(\"hi\\\");c_shutdown();--\")\n",
    );
    assert_screen_call(
        &calls[2],
        "DST_8level_ClusterCommand_Master",
        "c_announce(\":pig 正在回档3天\")\n",
    );
    assert_screen_call(
        &calls[3],
        "DST_8level_ClusterCommand_Caves",
        "c_announce(\":pig 正在回档3天\")\n",
    );
    assert_screen_call(
        &calls[4],
        "DST_8level_ClusterCommand_Master",
        "c_rollback(3)\n",
    );
    assert_screen_call(
        &calls[5],
        "DST_8level_ClusterCommand_Caves",
        "c_rollback(3)\n",
    );
}

#[tokio::test]
async fn console_and_regenerate_routes_keep_stream_routes_stubbed() {
    let (app, _dir, runner) = test_router().await;
    let cookie = login(&app).await;

    let master = send(
        &app,
        Method::POST,
        "/api/game/master/console",
        Some(json!({"command": "c_reset()"})),
        Some(&cookie),
    )
    .await;
    let caves = send(
        &app,
        Method::POST,
        "/api/game/caves/console",
        Some(json!({"command": "c_countprefabs(\"berrybush\")"})),
        Some(&cookie),
    )
    .await;
    let regenerate = send(
        &app,
        Method::GET,
        "/api/game/regenerateworld",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(master.status(), StatusCode::OK);
    assert_eq!(caves.status(), StatusCode::OK);
    assert_eq!(regenerate.status(), StatusCode::OK);
    let calls = runner.calls();
    assert_eq!(calls.len(), 6);
    assert_screen_call(&calls[0], "DST_8level_ClusterCommand_Master", "c_reset()\n");
    // Go's CavesConsole handler targets Master; keep that compatibility quirk.
    assert_screen_call(
        &calls[1],
        "DST_8level_ClusterCommand_Master",
        "c_countprefabs(\"berrybush\")\n",
    );
    assert_screen_call(
        &calls[4],
        "DST_8level_ClusterCommand_Master",
        "c_regenerateworld()\n",
    );
    assert_screen_call(
        &calls[5],
        "DST_8level_ClusterCommand_Caves",
        "c_regenerateworld()\n",
    );
}

async fn test_router() -> (Router, TempDir, FakeCommandRunner) {
    let dir = tempdir().unwrap();
    write_password_file(dir.path());
    write_dst_config(dir.path(), "ClusterCommand");
    let pool = connect_sqlite_memory().await.unwrap();
    migrate(&pool).await.unwrap();
    let runner = FakeCommandRunner::default();
    let state = AppState::new_with_command_runner(
        test_config(),
        pool,
        SessionStore::new(),
        dir.path(),
        runner.clone(),
    );
    (build_router(state), dir, runner)
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

fn assert_screen_call(spec: &CommandSpec, session: &str, command: &str) {
    assert_eq!(spec.program(), "screen");
    assert_eq!(
        spec.args(),
        ["-S", session, "-p", "0", "-X", "stuff", command]
    );
}
