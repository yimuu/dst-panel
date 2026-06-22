use std::{fs, path::Path, time::Duration};

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{
        Method, Request, Response, StatusCode,
        header::{CACHE_CONTROL, CONNECTION, CONTENT_TYPE, COOKIE, SET_COOKIE},
    },
};
use dst_admin_rust::{
    domain::auth::SessionStore,
    infra::config::{AppConfig, AutoUpdateModinfoConfig},
    infra::db::{connect_sqlite_memory, migrate},
    web::app::{AppState, build_router},
};
use futures_util::{SinkExt, StreamExt};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tempfile::{TempDir, tempdir};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tower::ServiceExt;

#[tokio::test]
async fn status_stream_requires_auth_and_emits_go_sse_message_frame() {
    let (app, dir) = test_router().await;
    write_cluster_fixture(dir.path(), "ClusterStatusStream");

    let unauthenticated = send(
        &app,
        Method::GET,
        "/api/game/8level/status/stream",
        None,
        None,
    )
    .await;
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);

    let cookie = login(&app).await;
    let response = send(
        &app,
        Method::GET,
        "/api/game/8level/status/stream",
        None,
        Some(&cookie),
    )
    .await;

    assert_sse_headers(&response);
    let event = first_body_chunk(response).await;
    assert!(event.starts_with("event:message\ndata:"));
    let payload = parse_message_event_payload(&event);
    assert_eq!(payload["code"], 200);
    assert_eq!(payload["msg"], "success");
    assert_eq!(payload["data"][0]["uuid"], "Master");
    assert_eq!(payload["data"][0]["status"], false);
}

#[tokio::test]
async fn system_info_stream_emits_go_sse_message_frame_with_dashboard_fields() {
    let (app, dir) = test_router().await;
    write_cluster_fixture(dir.path(), "ClusterSystemStream");
    let cookie = login(&app).await;

    let response = send(
        &app,
        Method::GET,
        "/api/game/system/info/stream",
        None,
        Some(&cookie),
    )
    .await;

    assert_sse_headers(&response);
    let event = first_body_chunk(response).await;
    assert!(event.starts_with("event:message\ndata:"));
    let payload = parse_message_event_payload(&event);
    assert_eq!(payload["code"], 200);
    assert!(payload["data"].get("host").is_some());
    assert!(payload["data"].get("cpu").is_some());
    assert!(payload["data"].get("mem").is_some());
    assert!(payload["data"].get("disk").is_some());
    assert!(payload["data"].get("panelMemUsage").is_some());
    assert!(payload["data"].get("panelCpuUsage").is_some());
}

#[tokio::test]
async fn log_stream_validates_level_and_emits_go_log_events() {
    let (app, dir) = test_router().await;
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterLogStream");
    fs::write(cluster_dir.join("Master/server_log.txt"), "one\ntwo\n").unwrap();
    let cookie = login(&app).await;

    let missing_level = send(
        &app,
        Method::GET,
        "/api/game/log/stream",
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(missing_level.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_json(missing_level).await,
        json!({"error": "cluster and level required"})
    );

    let response = send(
        &app,
        Method::GET,
        "/api/game/log/stream?levelName=Master",
        None,
        Some(&cookie),
    )
    .await;

    assert_sse_headers(&response);
    let event = first_body_chunk(response).await;
    assert!(event.contains("event: log\n"));
    assert!(event.contains("data: one\n\n"));
    assert!(event.contains("data: two\n\n"));
}

#[tokio::test]
async fn websocket_route_is_public_and_allows_arbitrary_origin_upgrade() {
    let (app, _dir) = test_router().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let mut request = format!("ws://{addr}/ws").into_client_request().unwrap();
    request
        .headers_mut()
        .insert("origin", "https://untrusted.example.test".parse().unwrap());
    let (_socket, response) = tokio_tungstenite::connect_async(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);
    server.abort();
}

#[tokio::test]
async fn websocket_tailf_requires_session_cookie_and_rejects_app_root_secrets() {
    let (app, dir) = test_router().await;
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterWsTail");
    let log_path = cluster_dir.join("Master/server_log.txt");
    fs::write(&log_path, "one\ntwo\n").unwrap();
    let cookie = login(&app).await;

    let (mut socket, server) = connect_ws(app.clone()).await;
    socket
        .send(WsMessage::Text(
            format!("tailf {}", log_path.display()).into(),
        ))
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(200), socket.next())
            .await
            .is_err(),
        "public websocket handshake must not grant log tail access"
    );
    socket.send(WsMessage::Text("byte".into())).await.unwrap();
    server.abort();

    let (mut socket, server) = connect_ws_with_cookie(app.clone(), Some(&cookie)).await;
    socket
        .send(WsMessage::Text(
            format!("tailf {}", log_path.display()).into(),
        ))
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(200), socket.next())
            .await
            .is_err(),
        "cross-origin websocket with cookies must not grant log tail access"
    );
    socket.send(WsMessage::Text("byte".into())).await.unwrap();
    server.abort();

    let (mut socket, server) =
        connect_ws_with_cookie_and_origin(app.clone(), Some(&cookie), WsOrigin::SameHost).await;
    socket
        .send(WsMessage::Text(
            format!("tailf {}", log_path.display()).into(),
        ))
        .await
        .unwrap();
    let first = tokio::time::timeout(Duration::from_secs(1), socket.next())
        .await
        .expect("cluster log tail should emit snapshot")
        .expect("websocket closed")
        .unwrap();
    assert!(matches!(first, WsMessage::Text(_)));
    socket.send(WsMessage::Text("byte".into())).await.unwrap();
    server.abort();

    let (mut socket, server) =
        connect_ws_with_cookie_and_origin(app, Some(&cookie), WsOrigin::SameHost).await;
    socket
        .send(WsMessage::Text(
            format!("tailf {}", dir.path().join("password.txt").display()).into(),
        ))
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(200), socket.next())
            .await
            .is_err(),
        "authenticated websocket tailf must still reject app-root secrets"
    );
    server.abort();
}

#[tokio::test]
async fn websocket_tailf_rechecks_session_after_logout() {
    let (app, dir) = test_router().await;
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterWsLogout");
    let log_path = cluster_dir.join("Master/server_log.txt");
    fs::write(&log_path, "one\ntwo\n").unwrap();
    let cookie = login(&app).await;

    let (mut socket, server) =
        connect_ws_with_cookie_and_origin(app.clone(), Some(&cookie), WsOrigin::SameHost).await;
    let logout = send(&app, Method::POST, "/api/logout", None, Some(&cookie)).await;
    assert_eq!(logout.status(), StatusCode::OK);

    socket
        .send(WsMessage::Text(
            format!("tailf {}", log_path.display()).into(),
        ))
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(200), socket.next())
            .await
            .is_err(),
        "tailf must re-check the session instead of trusting upgrade-time cookies"
    );

    socket.send(WsMessage::Text("byte".into())).await.unwrap();
    server.abort();
}

#[tokio::test]
async fn log_stream_snapshots_large_logs_with_bounded_recent_tail() {
    let (app, dir) = test_router().await;
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterLargeLogStream");
    let mut log = String::with_capacity(2 * 1024 * 1024);
    for index in 0..150_000 {
        log.push_str("old-line-");
        log.push_str(&index.to_string());
        log.push('\n');
    }
    log.push_str("recent-one\nrecent-two\n");
    fs::write(cluster_dir.join("Master/server_log.txt"), log).unwrap();
    let cookie = login(&app).await;

    let response = send(
        &app,
        Method::GET,
        "/api/game/log/stream?levelName=Master",
        None,
        Some(&cookie),
    )
    .await;

    assert_sse_headers(&response);
    let event = first_body_chunk(response).await;
    assert!(event.contains("data: recent-one\n\n"));
    assert!(event.contains("data: recent-two\n\n"));
    assert!(!event.contains("old-line-0"));
}

#[tokio::test]
async fn log_stream_rejects_oversized_snapshot_instead_of_streaming_unbounded_text() {
    let (app, dir) = test_router().await;
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterOversizedLogStream");
    fs::write(
        cluster_dir.join("Master/server_log.txt"),
        "x".repeat(9 * 1024 * 1024),
    )
    .unwrap();
    let cookie = login(&app).await;

    let response = send(
        &app,
        Method::GET,
        "/api/game/log/stream?levelName=Master",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        response_json(response).await,
        json!({"error": "log snapshot exceeds safety limit"})
    );
}

async fn test_router() -> (Router, TempDir) {
    let dir = tempdir().unwrap();
    write_password_file(dir.path());
    write_dst_config(dir.path(), "ClusterStream");
    let pool = connect_sqlite_memory().await.unwrap();
    migrate(&pool).await.unwrap();
    let state = AppState::new(test_config(), pool, SessionStore::new(), dir.path());
    (build_router(state), dir)
}

async fn connect_ws(
    app: Router,
) -> (
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    tokio::task::JoinHandle<()>,
) {
    connect_ws_with_cookie(app, None).await
}

async fn connect_ws_with_cookie(
    app: Router,
    cookie: Option<&str>,
) -> (
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    tokio::task::JoinHandle<()>,
) {
    connect_ws_with_cookie_and_origin(app, cookie, WsOrigin::Untrusted).await
}

#[derive(Clone, Copy)]
enum WsOrigin {
    SameHost,
    Untrusted,
}

async fn connect_ws_with_cookie_and_origin(
    app: Router,
    cookie: Option<&str>,
    origin: WsOrigin,
) -> (
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    tokio::task::JoinHandle<()>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let mut request = format!("ws://{addr}/ws").into_client_request().unwrap();
    let origin = match origin {
        WsOrigin::SameHost => format!("http://{addr}"),
        WsOrigin::Untrusted => "https://untrusted.example.test".to_owned(),
    };
    request
        .headers_mut()
        .insert("origin", origin.parse().unwrap());
    if let Some(cookie) = cookie {
        request
            .headers_mut()
            .insert(COOKIE, cookie.parse().unwrap());
    }
    let (socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    (socket, server)
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
    let master_dir = cluster_dir.join("Master");
    fs::create_dir_all(&master_dir).unwrap();
    fs::write(
        master_dir.join("server.ini"),
        "[NETWORK]\nserver_port = 11000\n",
    )
    .unwrap();
    fs::write(master_dir.join("leveldataoverride.lua"), "return {}").unwrap();
    fs::write(master_dir.join("modoverrides.lua"), "return {}").unwrap();
    cluster_dir
}

fn assert_sse_headers(response: &Response<Body>) {
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(CONTENT_TYPE).unwrap(),
        "text/event-stream"
    );
    assert_eq!(response.headers().get(CACHE_CONTROL).unwrap(), "no-cache");
    assert_eq!(response.headers().get(CONNECTION).unwrap(), "keep-alive");
    assert_eq!(response.headers().get("x-accel-buffering").unwrap(), "no");
}

async fn first_body_chunk(response: Response<Body>) -> String {
    let mut body = response.into_body();
    let frame = tokio::time::timeout(Duration::from_secs(1), body.frame())
        .await
        .expect("first SSE frame timed out")
        .expect("first SSE frame missing")
        .expect("first SSE frame failed");
    let bytes = frame.into_data().expect("first SSE frame was not data");
    String::from_utf8(bytes.to_vec()).expect("SSE frame should be utf-8")
}

fn parse_message_event_payload(event: &str) -> Value {
    let data_line = event
        .lines()
        .find_map(|line| line.strip_prefix("data:"))
        .expect("message event should contain data line");
    serde_json::from_str(data_line).expect("message event data should be JSON")
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
