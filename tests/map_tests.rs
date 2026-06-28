use std::{fs, path::Path, thread, time::Duration};

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
async fn map_session_routes_read_latest_non_meta_session_files() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterMapSession");
    write_session_fixture(
        &cluster_dir,
        "Master",
        "session-A",
        "height=1\nwidth=1\ntiles=\"AQ==\"\nWalrusHut_Plains",
    );

    let response = send(
        &app,
        Method::GET,
        "/api/dst/map/session/file?levelName=Master",
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
            "data": "height=1\nwidth=1\ntiles=\"AQ==\"\nWalrusHut_Plains"
        })
    );

    let response = send(
        &app,
        Method::GET,
        "/api/dst/map/has/walrusHut/plains?levelName=Master",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!({"code": 200, "msg": "success", "data": true})
    );
}

#[tokio::test]
async fn map_session_latest_file_ignores_newer_meta_files_and_uses_newest_non_meta() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterMapLatest");
    let first = cluster_dir.join("Master/save/session/session-A");
    fs::create_dir_all(&first).unwrap();
    fs::write(first.join("0000000001"), "old-world").unwrap();
    thread::sleep(Duration::from_millis(20));

    let second = cluster_dir.join("Master/save/session/session-B");
    fs::create_dir_all(&second).unwrap();
    fs::write(second.join("0000000002"), "new-world").unwrap();
    thread::sleep(Duration::from_millis(20));
    fs::write(second.join("0000000003.meta"), "newer-meta-must-be-ignored").unwrap();

    let response = send(
        &app,
        Method::GET,
        "/api/dst/map/session/file?levelName=Master",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!({"code": 200, "msg": "success", "data": "new-world"})
    );
}

#[tokio::test]
async fn map_session_latest_file_ignores_root_files_like_go() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterMapRootFile");
    let session_root = cluster_dir.join("Master/save/session");
    let session_dir = session_root.join("session-A");
    fs::create_dir_all(&session_dir).unwrap();
    fs::write(session_dir.join("0000000001"), "go-visible-world").unwrap();
    thread::sleep(Duration::from_millis(20));
    fs::write(session_root.join("9999999999"), "root-file-must-be-ignored").unwrap();

    let response = send(
        &app,
        Method::GET,
        "/api/dst/map/session/file?levelName=Master",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!({"code": 200, "msg": "success", "data": "go-visible-world"})
    );
}

#[cfg(unix)]
#[tokio::test]
async fn map_session_latest_file_matches_go_sorted_tie_break_when_mtimes_equal() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterMapTieBreak");

    // Create the lexicographically later directory first. Rust's raw
    // `read_dir` order often follows creation order, while Go's `ioutil.ReadDir`
    // sorted names before applying the strict `ModTime.After` comparison.
    let second = cluster_dir.join("Master/save/session/session-B");
    fs::create_dir_all(&second).unwrap();
    let second_file = second.join("0000000001");
    fs::write(&second_file, "second-created").unwrap();

    let first = cluster_dir.join("Master/save/session/session-A");
    fs::create_dir_all(&first).unwrap();
    let first_file = first.join("0000000001");
    fs::write(&first_file, "first-sorted").unwrap();
    set_same_mtime(&[&first_file, &second_file]);

    let response = send(
        &app,
        Method::GET,
        "/api/dst/map/session/file?levelName=Master",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!({"code": 200, "msg": "success", "data": "first-sorted"})
    );
}

#[tokio::test]
async fn player_session_latest_file_ignores_nested_files_like_go() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterMapPlayerNested");
    let session_dir = write_session_fixture(
        &cluster_dir,
        "Master",
        "session-player",
        "height=1\nwidth=1\ntiles=\"AQ==\"",
    );
    let player_dir = session_dir.join("KU_player-1_");
    fs::create_dir_all(&player_dir).unwrap();
    fs::write(player_dir.join("0000000001"), "direct-player-state").unwrap();
    thread::sleep(Duration::from_millis(20));
    let nested_dir = player_dir.join("nested");
    fs::create_dir_all(&nested_dir).unwrap();
    fs::write(nested_dir.join("9999999999"), "nested-must-be-ignored").unwrap();

    let response = send(
        &app,
        Method::GET,
        "/api/dst/map/player/session/file?levelName=Master&kuId=KU_player-1",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!({"code": 200, "msg": "success", "data": "direct-player-state"})
    );
}

#[tokio::test]
async fn player_session_route_uses_world_session_id_and_ku_directory() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterMapPlayer");
    let session_dir = write_session_fixture(
        &cluster_dir,
        "Master",
        "session-player",
        "height=1\nwidth=1\ntiles=\"AQ==\"",
    );
    let player_dir = session_dir.join("KU_player-1_");
    fs::create_dir_all(&player_dir).unwrap();
    fs::write(player_dir.join("0000000001"), "old-player-state").unwrap();
    thread::sleep(Duration::from_millis(20));
    fs::write(player_dir.join("0000000002"), "latest-player-state").unwrap();

    let response = send(
        &app,
        Method::GET,
        "/api/dst/map/player/session/file?levelName=Master&kuId=KU_player-1",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!({"code": 200, "msg": "success", "data": "latest-player-state"})
    );
}

#[tokio::test]
async fn map_generation_writes_png_bytes_to_legacy_jpg_filename_and_image_streams_it() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterMapGen");
    write_session_fixture(
        &cluster_dir,
        "Master",
        "session-map",
        "height=1\nwidth=1\ntiles=\"AQ==\"",
    );

    let response = send(
        &app,
        Method::GET,
        "/api/dst/map/gen?levelName=Master",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!({"code": 200, "msg": "success", "data": null})
    );

    let image_path = cluster_dir.join("dst_map_Master.jpg");
    let generated = fs::read(&image_path).unwrap();
    assert!(generated.starts_with(b"\x89PNG\r\n\x1a\n"));

    let response = send(
        &app,
        Method::GET,
        "/api/dst/map/image?levelName=Master",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().get(CONTENT_TYPE).unwrap(), "image/png");
    assert_eq!(response_bytes(response).await, generated);
}

#[tokio::test]
async fn map_generation_downscales_real_world_sized_session_maps() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterMapGenLarge");
    write_session_fixture(
        &cluster_dir,
        "Master",
        "session-map",
        "height=518\nwidth=518\ntiles=\"AQ==\"",
    );

    let response = send(
        &app,
        Method::GET,
        "/api/dst/map/gen?levelName=Master",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let generated = fs::read(cluster_dir.join("dst_map_Master.jpg")).unwrap();
    assert!(generated.starts_with(b"\x89PNG\r\n\x1a\n"));
    let width = u32::from_be_bytes(generated[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(generated[20..24].try_into().unwrap());
    assert!(width <= 2048, "generated width was {width}");
    assert!(height <= 2048, "generated height was {height}");
}

#[tokio::test]
async fn map_routes_preserve_legacy_missing_query_messages() {
    let (app, _dir) = test_router().await;
    let cookie = login(&app).await;

    for uri in [
        "/api/dst/map/gen",
        "/api/dst/map/image",
        "/api/dst/map/has/walrusHut/plains",
        "/api/dst/map/session/file",
    ] {
        let response = send(&app, Method::GET, uri, None, Some(&cookie)).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response_json(response).await,
            json!({"code": 400, "msg": "levelName 参数不能为空", "data": null})
        );
    }

    let response = send(
        &app,
        Method::GET,
        "/api/dst/map/player/session/file?levelName=Master",
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_json(response).await,
        json!({"code": 400, "msg": "levelName or kuId 参数不能为空", "data": null})
    );
}

#[tokio::test]
async fn map_routes_reject_traversal_and_symlink_session_entries() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterMapSafe");
    fs::write(dir.path().join("secret-session"), "secret").unwrap();

    let response = send(
        &app,
        Method::GET,
        "/api/dst/map/session/file?levelName=..%2F..%2Fsecret-session",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response_json(response).await;
    assert_eq!(body["code"], 400);
    assert_ne!(body["data"], "secret");

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let session_dir = cluster_dir.join("Master/save/session/session-symlink");
        fs::create_dir_all(&session_dir).unwrap();
        symlink(
            dir.path().join("secret-session"),
            session_dir.join("0000000001"),
        )
        .unwrap();

        let response = send(
            &app,
            Method::GET,
            "/api/dst/map/session/file?levelName=Master",
            None,
            Some(&cookie),
        )
        .await;

        assert_ne!(response.status(), StatusCode::OK);
        let body = response_json(response).await;
        assert_ne!(body["data"], "secret");
    }
}

#[tokio::test]
async fn map_image_missing_file_returns_not_found_without_json_envelope() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    write_cluster_fixture(dir.path(), "ClusterMapMissingImage");

    let response = send(
        &app,
        Method::GET,
        "/api/dst/map/image?levelName=Master",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(response_bytes(response).await, b"404 page not found");
}

#[tokio::test]
async fn map_session_and_image_routes_reject_oversized_files() {
    let (app, dir) = test_router().await;
    let cookie = login(&app).await;
    let cluster_dir = write_cluster_fixture(dir.path(), "ClusterMapOversized");
    write_session_fixture(
        &cluster_dir,
        "Master",
        "session-large",
        &"x".repeat(9 * 1024 * 1024),
    );

    let response = send(
        &app,
        Method::GET,
        "/api/dst/map/session/file?levelName=Master",
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_json(response).await,
        json!({"code": 400, "msg": "session file exceeds safety limit", "data": null})
    );

    fs::File::create(cluster_dir.join("dst_map_Master.jpg"))
        .unwrap()
        .set_len(73 * 1024 * 1024)
        .unwrap();
    let response = send(
        &app,
        Method::GET,
        "/api/dst/map/image?levelName=Master",
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_json(response).await,
        json!({"code": 400, "msg": "map image exceeds safety limit", "data": null})
    );
}

async fn test_router() -> (Router, TempDir) {
    let dir = tempdir().unwrap();
    write_password_file(dir.path());
    write_dst_config(dir.path(), "ClusterMap");
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

fn write_session_fixture(
    cluster_dir: &Path,
    level_name: &str,
    session_id: &str,
    world_contents: &str,
) -> std::path::PathBuf {
    let session_dir = cluster_dir
        .join(level_name)
        .join("save/session")
        .join(session_id);
    fs::create_dir_all(&session_dir).unwrap();
    fs::write(session_dir.join("0000000001.meta"), "ignored-meta").unwrap();
    fs::write(session_dir.join("0000000001"), world_contents).unwrap();
    session_dir
}

#[cfg(unix)]
fn set_same_mtime(paths: &[&Path]) {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};

    for path in paths {
        let c_path = CString::new(path.as_os_str().as_bytes()).unwrap();
        let time = libc::timeval {
            tv_sec: 1_700_000_000,
            tv_usec: 0,
        };
        let times = [time, time];
        let result = unsafe { libc::utimes(c_path.as_ptr(), times.as_ptr()) };
        assert_eq!(
            result,
            0,
            "utimes failed for {}: {}",
            path.display(),
            std::io::Error::last_os_error()
        );
    }
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
