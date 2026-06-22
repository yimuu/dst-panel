//! Compatibility manifest tests for the migrated Go route surface.
//!
//! These tests intentionally exercise concrete HTTP requests instead of only
//! checking a Rust data structure. That proves every compatibility route is
//! registered in Axum and that the router does not depend on a broad `/api/*`
//! catch-all to hide missing migration work.

use std::{collections::BTreeSet, fs, path::Path};

use axum::{
    Router,
    body::Body,
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
    web::handlers::compat::{COMPATIBILITY_ROUTE_MANIFEST, CompatibilityRouteStatus},
};
use serde_json::{Value, json};
use tempfile::{TempDir, tempdir};
use tower::ServiceExt;

#[test]
fn compatibility_manifest_has_no_remaining_stubs() {
    assert!(
        COMPATIBILITY_ROUTE_MANIFEST
            .iter()
            .all(|route| route.status == CompatibilityRouteStatus::Implemented),
        "all Go routes should be migrated off compatibility stubs"
    );
}

#[tokio::test]
async fn already_migrated_routes_keep_real_handlers_instead_of_501_stubs() {
    let (app, dir) = test_router().await;

    let hello = send(&app, Method::GET, "/hello", None, None).await;
    assert_eq!(hello.status(), StatusCode::OK);
    assert_ne!(hello.status(), StatusCode::NOT_IMPLEMENTED);

    let root = send(&app, Method::GET, "/", None, None).await;
    assert_ne!(root.status(), StatusCode::NOT_IMPLEMENTED);
    let root_head = send(&app, Method::HEAD, "/", None, None).await;
    assert_ne!(root_head.status(), StatusCode::NOT_IMPLEMENTED);

    let dist = dir.path().join("dist");
    fs::create_dir_all(dist.join("assets")).unwrap();
    fs::create_dir_all(dist.join("misc")).unwrap();
    fs::create_dir_all(dist.join("static/js")).unwrap();
    fs::create_dir_all(dist.join("static/css")).unwrap();
    fs::create_dir_all(dist.join("static/img")).unwrap();
    fs::create_dir_all(dist.join("static/fonts")).unwrap();
    fs::create_dir_all(dist.join("static/media")).unwrap();
    fs::write(dist.join("assets/app.js"), "console.log('ok');").unwrap();
    fs::write(dist.join("misc/manifest.json"), "{}").unwrap();
    fs::write(dist.join("static/js/app.js"), "console.log('ok');").unwrap();
    fs::write(dist.join("static/css/app.css"), "body{}").unwrap();
    fs::write(dist.join("static/img/logo.png"), [0_u8, 1]).unwrap();
    fs::write(dist.join("static/fonts/app.woff2"), [2_u8, 3]).unwrap();
    fs::write(dist.join("static/media/clip.webm"), [4_u8, 5]).unwrap();
    fs::write(dist.join("favicon.ico"), [6_u8, 7]).unwrap();
    fs::write(dist.join("asset-manifest.json"), "{}").unwrap();

    for (method, uri) in [
        (Method::GET, "/assets/app.js"),
        (Method::HEAD, "/assets/app.js"),
        (Method::GET, "/misc/manifest.json"),
        (Method::HEAD, "/misc/manifest.json"),
        (Method::GET, "/static/js/app.js"),
        (Method::HEAD, "/static/js/app.js"),
        (Method::GET, "/static/css/app.css"),
        (Method::HEAD, "/static/css/app.css"),
        (Method::GET, "/static/img/logo.png"),
        (Method::HEAD, "/static/img/logo.png"),
        (Method::GET, "/static/fonts/app.woff2"),
        (Method::HEAD, "/static/fonts/app.woff2"),
        (Method::GET, "/static/media/clip.webm"),
        (Method::HEAD, "/static/media/clip.webm"),
        (Method::GET, "/favicon.ico"),
        (Method::HEAD, "/favicon.ico"),
        (Method::GET, "/asset-manifest.json"),
        (Method::HEAD, "/asset-manifest.json"),
    ] {
        let response = send(&app, method.clone(), uri, None, None).await;
        assert_ne!(
            response.status(),
            StatusCode::NOT_IMPLEMENTED,
            "{method} {uri} should keep the migrated static handler"
        );
    }

    let login_response = send(
        &app,
        Method::POST,
        "/api/login",
        Some(json!({"username": "admin", "password": "123456"})),
        None,
    )
    .await;
    assert_eq!(login_response.status(), StatusCode::OK);
    assert_ne!(login_response.status(), StatusCode::NOT_IMPLEMENTED);
    let cookie = login_cookie(&login_response);

    let protected_implemented_probes = [
        (Method::POST, "/api/change/password", None),
        (Method::GET, "/api/user", None),
        (Method::POST, "/api/user", None),
        (Method::GET, "/api/init", None),
        (Method::GET, "/api/kv?key=missing", None),
        (
            Method::POST,
            "/api/kv",
            Some(json!({"key": "compat-test", "value": "ok"})),
        ),
        (Method::GET, "/api/web/link", None),
        (Method::POST, "/api/web/link", None),
        (Method::DELETE, "/api/web/link", None),
        (Method::GET, "/api/cluster", None),
        (Method::POST, "/api/cluster", None),
        (Method::PUT, "/api/cluster", None),
        (Method::DELETE, "/api/cluster", None),
        (Method::GET, "/api/cluster/level", None),
        (Method::PUT, "/api/cluster/level", None),
        (Method::POST, "/api/cluster/level", None),
        (Method::DELETE, "/api/cluster/level", None),
        (Method::GET, "/api/game/8level/status", None),
        (Method::GET, "/api/player/log", None),
        (Method::POST, "/api/player/log/delete", None),
        (Method::GET, "/api/statistics/active/user", None),
        (Method::GET, "/api/statistics/top/death", None),
        (Method::GET, "/api/statistics/top/login", None),
        (Method::GET, "/api/statistics/top/active", None),
        (Method::GET, "/api/statistics/rate/role", None),
        (Method::GET, "/api/statistics/regenerate", None),
        (Method::GET, "/api/game/8level/clusterIni", None),
        (Method::POST, "/api/game/8level/clusterIni", None),
        (Method::GET, "/api/game/8level/players", None),
        (Method::GET, "/api/game/8level/players/all", None),
        (Method::GET, "/api/game/8level/adminilist", None),
        (Method::GET, "/api/game/8level/whitelist", None),
        (Method::GET, "/api/game/8level/blacklist", None),
        (Method::POST, "/api/game/8level/adminilist", None),
        (Method::POST, "/api/game/8level/whitelist", None),
        (Method::POST, "/api/game/8level/blacklist", None),
        (Method::GET, "/api/game/player", None),
        (Method::GET, "/api/game/player/adminlist", None),
        (Method::POST, "/api/game/player/adminlist", None),
        (Method::DELETE, "/api/game/player/adminlist", None),
        (Method::GET, "/api/game/player/blacklist", None),
        (Method::POST, "/api/game/player/blacklist", None),
        (Method::DELETE, "/api/game/player/blacklist", None),
        (Method::GET, "/api/dst/config", None),
        (Method::POST, "/api/dst/config", None),
        (Method::GET, "/api/game/config", None),
        (Method::POST, "/api/game/config", None),
        (Method::GET, "/api/game/system/info", None),
    ];

    for (method, uri, body) in protected_implemented_probes {
        let response = send(&app, method.clone(), uri, body, Some(&cookie)).await;
        assert_ne!(
            response.status(),
            StatusCode::NOT_IMPLEMENTED,
            "{method} {uri} should keep the migrated handler"
        );
    }

    for method in [Method::GET, Method::POST] {
        let logout_cookie = login(&app).await;
        let response = send(
            &app,
            method.clone(),
            "/api/logout",
            None,
            Some(&logout_cookie),
        )
        .await;
        assert_ne!(
            response.status(),
            StatusCode::NOT_IMPLEMENTED,
            "{method} /api/logout should keep the migrated handler"
        );
    }
}

#[tokio::test]
async fn unknown_api_paths_are_not_hidden_by_a_catch_all_stub() {
    let (app, _dir) = test_router().await;
    let cookie = login(&app).await;

    let response = send(
        &app,
        Method::GET,
        "/api/not-in-go-compat-manifest",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn compatibility_manifest_matches_the_exercised_route_surface() {
    let mut seen = BTreeSet::new();
    for route in COMPATIBILITY_ROUTE_MANIFEST {
        assert!(
            seen.insert((route.method, route.path)),
            "duplicate manifest route: {} {}",
            route.method,
            route.path
        );
    }

    let expected_implemented = BTreeSet::from([
        ("GET", "/hello"),
        ("GET", "/"),
        ("HEAD", "/"),
        ("GET", "/assets/{*filepath}"),
        ("HEAD", "/assets/{*filepath}"),
        ("GET", "/misc/{*filepath}"),
        ("HEAD", "/misc/{*filepath}"),
        ("GET", "/static/js/{*filepath}"),
        ("HEAD", "/static/js/{*filepath}"),
        ("GET", "/static/css/{*filepath}"),
        ("HEAD", "/static/css/{*filepath}"),
        ("GET", "/static/img/{*filepath}"),
        ("HEAD", "/static/img/{*filepath}"),
        ("GET", "/static/fonts/{*filepath}"),
        ("HEAD", "/static/fonts/{*filepath}"),
        ("GET", "/static/media/{*filepath}"),
        ("HEAD", "/static/media/{*filepath}"),
        ("GET", "/favicon.ico"),
        ("HEAD", "/favicon.ico"),
        ("GET", "/asset-manifest.json"),
        ("HEAD", "/asset-manifest.json"),
        ("POST", "/api/login"),
        ("GET", "/api/logout"),
        ("POST", "/api/logout"),
        ("POST", "/api/change/password"),
        ("GET", "/api/user"),
        ("POST", "/api/user"),
        ("GET", "/api/init"),
        ("POST", "/api/init"),
        ("GET", "/api/install/steamcmd"),
        ("GET", "/api/dst/version"),
        ("POST", "/api/dst/home/server"),
        ("POST", "/api/dst/home/server/detail"),
        ("GET", "/api/dst/lobby/server/detail"),
        ("GET", "/api/dst/home/server2"),
        ("GET", "/api/dst/home/server/detail2"),
        ("GET", "/api/dst-static/{*filepath}"),
        ("POST", "/api/dst-static/{*filepath}"),
        ("PUT", "/api/dst-static/{*filepath}"),
        ("DELETE", "/api/dst-static/{*filepath}"),
        ("PATCH", "/api/dst-static/{*filepath}"),
        ("HEAD", "/api/dst-static/{*filepath}"),
        ("OPTIONS", "/api/dst-static/{*filepath}"),
        ("CONNECT", "/api/dst-static/{*filepath}"),
        ("TRACE", "/api/dst-static/{*filepath}"),
        ("GET", "/steam/dst/news"),
        ("GET", "/ws"),
        ("GET", "/api/mod/search"),
        ("GET", "/api/mod/{modId}"),
        ("PUT", "/api/mod/{modId}"),
        ("GET", "/api/mod"),
        ("DELETE", "/api/mod/{modId}"),
        ("DELETE", "/api/mod/setup/workshop"),
        ("GET", "/api/mod/modinfo/{modId}"),
        ("POST", "/api/mod/modinfo"),
        ("POST", "/api/mod/modinfo/file"),
        ("PUT", "/api/mod/modinfo"),
        ("GET", "/api/mod/ugc/acf"),
        ("DELETE", "/api/mod/ugc"),
        ("POST", "/api/file/ugc/upload"),
        ("POST", "/api/file/background"),
        ("GET", "/api/file/background"),
        ("GET", "/api/kv"),
        ("POST", "/api/kv"),
        ("GET", "/api/web/link"),
        ("POST", "/api/web/link"),
        ("DELETE", "/api/web/link"),
        ("GET", "/api/cluster"),
        ("POST", "/api/cluster"),
        ("PUT", "/api/cluster"),
        ("DELETE", "/api/cluster"),
        ("GET", "/api/cluster/level"),
        ("PUT", "/api/cluster/level"),
        ("POST", "/api/cluster/level"),
        ("DELETE", "/api/cluster/level"),
        ("GET", "/api/game/8level/status"),
        ("GET", "/api/game/8level/status/stream"),
        ("GET", "/api/game/8level/start"),
        ("GET", "/api/game/8level/stop"),
        ("GET", "/api/game/8level/start/all"),
        ("GET", "/api/game/8level/stop/all"),
        ("GET", "/api/game/8level/udp/port"),
        ("POST", "/api/game/8level/command"),
        ("GET", "/api/game/8level/clusterIni"),
        ("POST", "/api/game/8level/clusterIni"),
        ("GET", "/api/game/8level/players"),
        ("GET", "/api/game/8level/players/all"),
        ("GET", "/api/game/8level/adminilist"),
        ("GET", "/api/game/8level/whitelist"),
        ("GET", "/api/game/8level/blacklist"),
        ("POST", "/api/game/8level/adminilist"),
        ("POST", "/api/game/8level/whitelist"),
        ("POST", "/api/game/8level/blacklist"),
        ("GET", "/api/game/player"),
        ("GET", "/api/game/player/adminlist"),
        ("POST", "/api/game/player/adminlist"),
        ("DELETE", "/api/game/player/adminlist"),
        ("GET", "/api/game/player/blacklist"),
        ("POST", "/api/game/player/blacklist"),
        ("DELETE", "/api/game/player/blacklist"),
        ("GET", "/api/dst/config"),
        ("POST", "/api/dst/config"),
        ("GET", "/api/game/config"),
        ("POST", "/api/game/config"),
        ("GET", "/api/game/system/info"),
        ("GET", "/api/game/system/info/stream"),
        ("GET", "/api/game/preinstall"),
        ("GET", "/api/game/update"),
        ("GET", "/api/game/backup"),
        ("POST", "/api/game/backup"),
        ("DELETE", "/api/game/backup"),
        ("PUT", "/api/game/backup"),
        ("GET", "/api/game/backup/download"),
        ("POST", "/api/game/backup/upload"),
        ("GET", "/api/game/backup/restore"),
        ("GET", "/api/game/archive"),
        ("POST", "/api/game/backup/snapshot/setting"),
        ("GET", "/api/game/backup/snapshot/setting"),
        ("GET", "/api/game/backup/snapshot/list"),
        ("GET", "/api/game/announce/setting"),
        ("POST", "/api/game/announce/setting"),
        ("GET", "/api/task"),
        ("POST", "/api/task"),
        ("DELETE", "/api/task"),
        ("GET", "/api/task/instruct"),
        ("GET", "/api/auto/check2"),
        ("POST", "/api/auto/check2"),
        ("POST", "/webhook"),
        ("GET", "/api/share/keyCer"),
        ("GET", "/api/share/keyCer/reflush"),
        ("GET", "/api/share/keyCer/enable"),
        ("POST", "/api/share/cluster/import"),
        ("GET", "/share/cluster"),
        ("GET", "/api/dst/map/gen"),
        ("GET", "/api/dst/map/image"),
        ("GET", "/api/dst/map/has/walrusHut/plains"),
        ("GET", "/api/dst/map/session/file"),
        ("GET", "/api/dst/map/player/session/file"),
        ("GET", "/api/game/sent/broadcast"),
        ("GET", "/api/game/kick/player"),
        ("GET", "/api/game/kill/player"),
        ("GET", "/api/game/respawn/player"),
        ("GET", "/api/game/rollback"),
        ("GET", "/api/game/regenerateworld"),
        ("GET", "/api/game/operate/player"),
        ("GET", "/api/game/clean"),
        ("GET", "/api/game/clean/level"),
        ("GET", "/api/game/clean/level/all"),
        ("GET", "/api/game/level/server/log"),
        ("GET", "/api/game/level/server/chat/log"),
        ("GET", "/api/game/level/server/download"),
        ("GET", "/api/game/log/stream"),
        ("GET", "/api/game/dst-admin-go/log"),
        ("GET", "/api/game/dst-admin-go/log/download"),
        ("GET", "/api/player/log"),
        ("POST", "/api/player/log/delete"),
        ("GET", "/api/statistics/active/user"),
        ("GET", "/api/statistics/top/death"),
        ("GET", "/api/statistics/top/login"),
        ("GET", "/api/statistics/top/active"),
        ("GET", "/api/statistics/rate/role"),
        ("GET", "/api/statistics/regenerate"),
        ("POST", "/api/game/master/console"),
        ("POST", "/api/game/caves/console"),
    ]);
    let manifest_implemented: BTreeSet<_> = COMPATIBILITY_ROUTE_MANIFEST
        .iter()
        .filter(|route| route.status == CompatibilityRouteStatus::Implemented)
        .map(|route| (route.method, route.path))
        .collect();
    assert_eq!(manifest_implemented, expected_implemented);
}

async fn test_router() -> (Router, TempDir) {
    let dir = tempdir().unwrap();
    write_password_file(dir.path());
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
        token: None,
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
    login_cookie(&response)
}

fn login_cookie(response: &Response<Body>) -> String {
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
