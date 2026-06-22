use std::{fs, io::Write, path::Path, time::Duration};

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
    domain::backup::repository::BackupSnapshotRepository,
    infra::command::{CommandOutput, FakeCommandRunner},
    infra::config::{AppConfig, AutoUpdateModinfoConfig},
    infra::db::{connect_sqlite_memory, migrate},
    web::app::{AppState, build_router},
};
use serde_json::{Value, json};
use tempfile::{TempDir, tempdir};
use tower::ServiceExt;
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

#[tokio::test]
async fn backup_routes_create_list_rename_download_and_delete_archives() {
    let runner = FakeCommandRunner::new(vec![
        CommandOutput::success(Vec::new(), Vec::new()),
        CommandOutput::success(Vec::new(), Vec::new()),
        CommandOutput::success(Vec::new(), Vec::new()),
        CommandOutput::success(Vec::new(), Vec::new()),
    ]);
    let (app, dir, runner) = test_router(runner).await;
    let cookie = login(&app).await;
    write_cluster_fixture(dir.path(), "ClusterBackup");

    let empty = send(&app, Method::GET, "/api/game/backup", None, Some(&cookie)).await;
    assert_eq!(empty.status(), StatusCode::OK);
    assert_eq!(response_json(empty).await["data"], json!([]));

    let created = send(
        &app,
        Method::POST,
        "/api/game/backup",
        Some(json!({"backupName": "manual.zip"})),
        Some(&cookie),
    )
    .await;
    assert_eq!(created.status(), StatusCode::OK);
    assert_eq!(
        response_json(created).await,
        json!({"code": 200, "msg": "create backup success", "data": null})
    );
    assert_eq!(runner.calls().len(), 1);
    assert_eq!(
        runner.calls()[0].args(),
        [
            "-S",
            "DST_8level_ClusterBackup_Master",
            "-p",
            "0",
            "-X",
            "stuff",
            "c_save()\n"
        ]
    );

    let listed = send(&app, Method::GET, "/api/game/backup", None, Some(&cookie)).await;
    assert_eq!(listed.status(), StatusCode::OK);
    let body = response_json(listed).await;
    assert_eq!(body["msg"], "get backup list success");
    assert_eq!(body["data"][0]["fileName"], "manual.zip");
    assert!(body["data"][0]["fileSize"].as_i64().unwrap() > 0);
    assert!(body["data"][0]["time"].as_i64().unwrap() > 0);
    assert!(body["data"][0]["createTime"].as_str().is_some());

    let renamed = send(
        &app,
        Method::PUT,
        "/api/game/backup",
        Some(json!({"fileName": "manual.zip", "newName": "renamed.zip"})),
        Some(&cookie),
    )
    .await;
    assert_eq!(renamed.status(), StatusCode::OK);
    assert_eq!(response_json(renamed).await["msg"], "rename backup success");

    let downloaded = send(
        &app,
        Method::GET,
        "/api/game/backup/download?fileName=renamed.zip",
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(downloaded.status(), StatusCode::OK);
    assert_eq!(
        downloaded.headers().get(CONTENT_DISPOSITION).unwrap(),
        "attachment; filename=renamed.zip"
    );
    assert!(!response_bytes(downloaded).await.is_empty());

    let deleted = send(
        &app,
        Method::DELETE,
        "/api/game/backup",
        Some(json!({"fileNames": ["renamed.zip"]})),
        Some(&cookie),
    )
    .await;
    assert_eq!(deleted.status(), StatusCode::OK);
    assert_eq!(
        response_json(deleted).await,
        json!({"code": 200, "msg": "delete backups success", "data": null})
    );
    assert!(!dir.path().join("backup/renamed.zip").exists());

    let created_with_empty_body =
        send(&app, Method::POST, "/api/game/backup", None, Some(&cookie)).await;
    assert_eq!(created_with_empty_body.status(), StatusCode::OK);
    assert_eq!(
        response_json(created_with_empty_body).await["msg"],
        "create backup success"
    );
    assert_eq!(runner.calls().len(), 2);
    assert_eq!(
        fs::read_dir(dir.path().join("backup"))
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".zip"))
            .count(),
        1
    );
    for entry in fs::read_dir(dir.path().join("backup")).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name().to_string_lossy().ends_with(".zip") {
            fs::remove_file(entry.path()).unwrap();
        }
    }

    let malformed = send_raw(
        &app,
        Method::POST,
        "/api/game/backup",
        "application/json",
        b"{not-json",
        Some(&cookie),
    )
    .await;
    assert_eq!(malformed.status(), StatusCode::OK);
    assert_eq!(
        response_json(malformed).await["msg"],
        "create backup success"
    );
    assert_eq!(runner.calls().len(), 3);

    for entry in fs::read_dir(dir.path().join("backup")).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name().to_string_lossy().ends_with(".zip") {
            fs::remove_file(entry.path()).unwrap();
        }
    }

    let plain_json = send_raw(
        &app,
        Method::POST,
        "/api/game/backup",
        "text/plain",
        br#"{"backupName":"plain-body.zip"}"#,
        Some(&cookie),
    )
    .await;
    assert_eq!(plain_json.status(), StatusCode::OK);
    assert_eq!(
        response_json(plain_json).await["msg"],
        "create backup success"
    );
    assert!(
        !dir.path().join("backup/plain-body.zip").exists(),
        "Go ShouldBind does not bind JSON from text/plain for this route"
    );
    assert_eq!(
        fs::read_dir(dir.path().join("backup"))
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".zip"))
            .count(),
        1
    );
}

#[tokio::test]
async fn backup_upload_restore_and_archive_routes_preserve_go_shapes_safely() {
    let (app, dir, _runner) = test_router(FakeCommandRunner::default()).await;
    let cookie = login(&app).await;
    write_cluster_fixture(dir.path(), "ClusterRestore");
    fs::write(
        dir.path()
            .join(".klei/DoNotStarveTogether/ClusterRestore/old-only.txt"),
        "remove me",
    )
    .unwrap();
    let uploaded_zip = build_restore_zip();

    let uploaded = send_multipart_file(
        &app,
        "/api/game/backup/upload",
        "uploaded.zip",
        &uploaded_zip,
        &cookie,
    )
    .await;
    assert_eq!(uploaded.status(), StatusCode::OK);
    assert_eq!(
        response_json(uploaded).await["msg"],
        "upload backup success"
    );

    let large_body = vec![b'x'; 2 * 1024 * 1024 + 1];
    let large_upload = send_multipart_file(
        &app,
        "/api/game/backup/upload",
        "large.zip",
        &large_body,
        &cookie,
    )
    .await;
    assert_eq!(large_upload.status(), StatusCode::OK);
    assert_eq!(
        fs::metadata(dir.path().join("backup/large.zip"))
            .unwrap()
            .len(),
        large_body.len() as u64
    );

    let duplicate = send_multipart_file(
        &app,
        "/api/game/backup/upload",
        "uploaded.zip",
        b"not a valid replacement",
        &cookie,
    )
    .await;
    assert_eq!(duplicate.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        fs::read(dir.path().join("backup/uploaded.zip")).unwrap(),
        uploaded_zip
    );
    let leaked_upload_temps = fs::read_dir(dir.path().join("backup"))
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".dst-admin-rust-upload-")
        })
        .count();
    assert_eq!(leaked_upload_temps, 0);

    let restored = send(
        &app,
        Method::GET,
        "/api/game/backup/restore?backupName=uploaded.zip",
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(restored.status(), StatusCode::OK);
    assert_eq!(
        response_json(restored).await,
        json!({"code": 200, "msg": "restore backup success", "data": null})
    );
    let cluster_dir = dir.path().join(".klei/DoNotStarveTogether/ClusterRestore");
    assert_eq!(
        fs::read_to_string(cluster_dir.join("cluster.ini")).unwrap(),
        "[GAMEPLAY]\ngame_mode = endless\n[NETWORK]\ncluster_name = Restored World\ncluster_description = restored\ncluster_password = restored-pass\ncluster_intention = cooperative\nmax_players = 8\n[MISC]\nmax_snapshots = 6\n"
    );
    assert!(!cluster_dir.join("old-only.txt").exists());
    assert!(
        fs::read_to_string(
            dir.path()
                .join("dst-dedicated-server/mods/dedicated_server_mods_setup.lua")
        )
        .unwrap()
        .contains("ServerModSetup(\"123\")")
    );

    let archive = send(&app, Method::GET, "/api/game/archive", None, Some(&cookie)).await;
    assert_eq!(archive.status(), StatusCode::OK);
    let archive = response_json(archive).await;
    assert_eq!(archive["msg"], "success");
    assert_eq!(archive["data"]["clusterName"], "Restored World");
    assert_eq!(archive["data"]["clusterDescription"], "restored");
    assert_eq!(archive["data"]["clusterPassword"], "restored-pass");
    assert_eq!(archive["data"]["gameMod"], "endless");
    assert_eq!(archive["data"]["maxPlayers"], 8);
    assert_eq!(archive["data"]["mods"], 1);
    assert_eq!(archive["data"]["ip"], "203.0.113.7");
    assert_eq!(
        archive["data"]["ipConnect"],
        "c_connect(\"203.0.113.7\",10999,\"restored-pass\")"
    );
    assert_eq!(archive["data"]["version"], 676042);
    assert_eq!(archive["data"]["lastVersion"], -1);
    assert_eq!(archive["data"]["meta"]["Clock"]["Cycles"], 0);
    assert_eq!(archive["data"]["meta"]["Seasons"]["Season"], "");
}

#[cfg(unix)]
#[tokio::test]
async fn restore_rejects_traversal_zip_without_replacing_cluster() {
    let (app, dir, _runner) = test_router(FakeCommandRunner::default()).await;
    let cookie = login(&app).await;
    write_cluster_fixture(dir.path(), "ClusterUnsafeRestore");
    fs::write(dir.path().join("backup/evil.zip"), build_evil_restore_zip()).unwrap();

    let restored = send(
        &app,
        Method::GET,
        "/api/game/backup/restore?backupName=evil.zip",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(restored.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        fs::read_to_string(
            dir.path()
                .join(".klei/DoNotStarveTogether/ClusterUnsafeRestore/cluster.ini")
        )
        .unwrap(),
        cluster_ini("ClusterUnsafeRestore", "survival")
    );
    assert!(!dir.path().join("pwned.txt").exists());
}

#[tokio::test]
async fn restore_rejects_zip_with_excessive_uncompressed_size_without_replacing_cluster() {
    let (app, dir, _runner) = test_router(FakeCommandRunner::default()).await;
    let cookie = login(&app).await;
    write_cluster_fixture(dir.path(), "ClusterHugeRestore");
    fs::write(dir.path().join("backup/huge.zip"), build_huge_restore_zip()).unwrap();

    let restored = send(
        &app,
        Method::GET,
        "/api/game/backup/restore?backupName=huge.zip",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(restored.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(
        fs::read_to_string(
            dir.path()
                .join(".klei/DoNotStarveTogether/ClusterHugeRestore/cluster.ini")
        )
        .unwrap(),
        cluster_ini("ClusterHugeRestore", "survival")
    );
    assert!(
        !dir.path()
            .join(".klei/DoNotStarveTogether/Imported")
            .exists()
    );
}

#[cfg(unix)]
#[tokio::test]
async fn restore_rejects_unsafe_mod_setup_before_replacing_cluster() {
    let (app, dir, _runner) = test_router(FakeCommandRunner::default()).await;
    let cookie = login(&app).await;
    write_cluster_fixture(dir.path(), "ClusterAtomicRestore");
    fs::write(
        dir.path().join("backup/mod-setup-fail.zip"),
        build_restore_zip(),
    )
    .unwrap();
    std::os::unix::fs::symlink(
        dir.path().join("outside-mod-setup.lua"),
        dir.path()
            .join("dst-dedicated-server/mods/dedicated_server_mods_setup.lua"),
    )
    .unwrap();

    let restored = send(
        &app,
        Method::GET,
        "/api/game/backup/restore?backupName=mod-setup-fail.zip",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(restored.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        fs::read_to_string(
            dir.path()
                .join(".klei/DoNotStarveTogether/ClusterAtomicRestore/cluster.ini")
        )
        .unwrap(),
        cluster_ini("ClusterAtomicRestore", "survival")
    );
    assert_eq!(
        fs::read_to_string(
            dir.path()
                .join(".klei/DoNotStarveTogether/ClusterAtomicRestore/cluster_token.txt")
        )
        .unwrap(),
        "cluster-token-file"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn restore_pre_swap_failure_removes_staging_directory() {
    let (app, dir, _runner) = test_router(FakeCommandRunner::default()).await;
    let cookie = login(&app).await;
    write_cluster_fixture(dir.path(), "ClusterStagingCleanup");
    let cluster_dir = dir
        .path()
        .join(".klei/DoNotStarveTogether/ClusterStagingCleanup");
    fs::remove_dir_all(&cluster_dir).unwrap();
    fs::write(&cluster_dir, "not a directory").unwrap();
    fs::write(dir.path().join("backup/valid.zip"), build_restore_zip()).unwrap();

    let restored = send(
        &app,
        Method::GET,
        "/api/game/backup/restore?backupName=valid.zip",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(restored.status(), StatusCode::BAD_REQUEST);
    assert_eq!(fs::read_to_string(&cluster_dir).unwrap(), "not a directory");
    assert_eq!(
        hidden_restore_dirs(dir.path(), "ClusterStagingCleanup"),
        Vec::<String>::new()
    );
}

#[tokio::test]
async fn restore_rejects_zip_declaring_too_many_entries_before_deep_parse() {
    let (app, dir, _runner) = test_router(FakeCommandRunner::default()).await;
    let cookie = login(&app).await;
    write_cluster_fixture(dir.path(), "ClusterEntryCap");
    fs::write(
        dir.path().join("backup/many.zip"),
        build_zip_eocd_declaring_entries(10_001),
    )
    .unwrap();

    let restored = send(
        &app,
        Method::GET,
        "/api/game/backup/restore?backupName=many.zip",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(restored.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(
        fs::read_to_string(
            dir.path()
                .join(".klei/DoNotStarveTogether/ClusterEntryCap/cluster.ini")
        )
        .unwrap(),
        cluster_ini("ClusterEntryCap", "survival")
    );
}

#[tokio::test]
async fn archive_and_generated_backup_name_use_latest_save_meta() {
    let runner = FakeCommandRunner::new(vec![CommandOutput::success(Vec::new(), Vec::new())]);
    let (app, dir, _runner) = test_router(runner).await;
    let cookie = login(&app).await;
    write_cluster_fixture(dir.path(), "ClusterMeta");
    write_meta_fixture(dir.path(), "ClusterMeta");

    let archive = send(&app, Method::GET, "/api/game/archive", None, Some(&cookie)).await;
    assert_eq!(archive.status(), StatusCode::OK);
    let archive = response_json(archive).await;
    assert_eq!(archive["data"]["meta"]["Clock"]["Cycles"], 12);
    assert_eq!(archive["data"]["meta"]["Clock"]["Phase"], "dusk");
    assert_eq!(archive["data"]["meta"]["Clock"]["Segs"]["Night"], 2);
    assert_eq!(archive["data"]["meta"]["Seasons"]["Season"], "spring");
    assert_eq!(archive["data"]["meta"]["Seasons"]["ElapsedDaysInSeason"], 3);
    assert_eq!(
        archive["data"]["meta"]["Seasons"]["RemainingDaysInSeason"],
        17
    );

    let created = send(&app, Method::POST, "/api/game/backup", None, Some(&cookie)).await;
    assert_eq!(created.status(), StatusCode::OK);
    let generated_name = fs::read_dir(dir.path().join("backup"))
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .find(|name| name.ends_with(".zip"))
        .unwrap();
    assert!(generated_name.ends_with("_ClusterMeta_12day_dusk_春天(3_20).zip"));
}

#[tokio::test]
async fn snapshot_setting_and_list_routes_use_go_json_shape() {
    let (app, dir, _runner) = test_router(FakeCommandRunner::default()).await;
    let cookie = login(&app).await;
    write_cluster_fixture(dir.path(), "ClusterSnapshot");
    fs::write(
        dir.path().join("backup/(snapshot)ClusterSnapshot-a.zip"),
        b"a",
    )
    .unwrap();
    fs::write(dir.path().join("backup/manual.zip"), b"b").unwrap();

    let saved = send(
        &app,
        Method::POST,
        "/api/game/backup/snapshot/setting",
        Some(json!({"name": "nightly", "interval": 30, "maxSnapshots": 4, "enable": 1, "isCSave": 1})),
        Some(&cookie),
    )
    .await;
    assert_eq!(saved.status(), StatusCode::OK);
    let saved = response_json(saved).await;
    assert_eq!(saved["msg"], "success");
    assert_eq!(
        saved["data"]["name"], "",
        "Go binds name but never copies it into the persisted singleton"
    );
    assert_eq!(saved["data"]["maxSnapshots"], 4);
    assert_eq!(saved["data"]["isCSave"], 1);
    assert!(saved["data"]["CreatedAt"].as_str().is_some());
    assert!(saved["data"]["UpdatedAt"].as_str().is_some());

    let fetched = send(
        &app,
        Method::GET,
        "/api/game/backup/snapshot/setting",
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(fetched.status(), StatusCode::OK);
    assert_eq!(response_json(fetched).await["data"]["interval"], 30);

    let partial = send(
        &app,
        Method::POST,
        "/api/game/backup/snapshot/setting",
        Some(json!({"enable": 1})),
        Some(&cookie),
    )
    .await;
    assert_eq!(partial.status(), StatusCode::OK);
    let partial = response_json(partial).await;
    assert_eq!(partial["data"]["name"], "");
    assert_eq!(partial["data"]["interval"], 0);
    assert_eq!(partial["data"]["maxSnapshots"], 0);
    assert_eq!(partial["data"]["enable"], 1);
    assert_eq!(partial["data"]["isCSave"], 0);

    let snapshots = send(
        &app,
        Method::GET,
        "/api/game/backup/snapshot/list",
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(snapshots.status(), StatusCode::OK);
    let snapshots = response_json(snapshots).await;
    assert_eq!(snapshots["msg"], "success");
    assert_eq!(snapshots["data"].as_array().unwrap().len(), 1);
    assert_eq!(
        snapshots["data"][0]["fileName"],
        "(snapshot)ClusterSnapshot-a.zip"
    );
}

#[tokio::test]
async fn share_key_and_cluster_routes_match_legacy_contract() {
    let (app, dir, _runner) = test_router(FakeCommandRunner::default()).await;
    let cookie = login(&app).await;
    write_cluster_fixture(dir.path(), "ClusterShare");
    fs::write(
        dir.path()
            .join(".klei/DoNotStarveTogether/ClusterShare/adminlist.txt"),
        "KU_admin\n",
    )
    .unwrap();
    fs::write(
        dir.path()
            .join(".klei/DoNotStarveTogether/ClusterShare/blocklist.txt"),
        "KU_block\n",
    )
    .unwrap();
    fs::write(
        dir.path()
            .join(".klei/DoNotStarveTogether/ClusterShare/whitelist.txt"),
        "KU_allow\n",
    )
    .unwrap();

    let key = send(
        &app,
        Method::GET,
        "/api/share/keyCer/reflush",
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(key.status(), StatusCode::OK);
    let key = response_json(key).await;
    assert_eq!(key["data"]["enable"], "0");
    assert_eq!(key["data"]["port"], "0");
    let shared_key = key["data"]["key"].as_str().unwrap().to_owned();
    assert_eq!(
        fs::read_to_string(dir.path().join("key")).unwrap(),
        format!("0{shared_key}")
    );

    let enabled = send(
        &app,
        Method::GET,
        "/api/share/keyCer/enable?enable=1",
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(enabled.status(), StatusCode::OK);
    assert_eq!(response_json(enabled).await["data"]["enable"], "1");

    let shared = send(
        &app,
        Method::GET,
        &format!("/share/cluster?key={shared_key}"),
        None,
        None,
    )
    .await;
    assert_eq!(shared.status(), StatusCode::OK);
    let shared = response_json(shared).await;
    assert_eq!(shared["msg"], "success");
    assert!(
        shared["data"]["cluster_ini"]
            .as_str()
            .unwrap()
            .contains("ClusterShare")
    );
    assert_eq!(shared["data"]["cluster_token"], "cluster-token-file");
    assert_eq!(shared["data"]["adminlist"], "KU_admin\n");
    assert_eq!(shared["data"]["blocklist"], "KU_block\n");
    assert_eq!(shared["data"]["whitelist"], "KU_allow\n");
    assert_eq!(
        shared["data"]["level_json"], "",
        "Go joins level.json below whitelist.txt; preserve that empty field"
    );
    assert_eq!(
        shared["data"]["levels"][0]["levelName"], "Master",
        "Go ShareClusterConfig calls GetLevel with the level folder name"
    );
    assert_eq!(shared["data"]["levels"][0]["is_master"], false);
    assert_eq!(shared["data"]["levels"][0]["uuid"], "");

    let imported = send(
        &app,
        Method::POST,
        "/api/share/cluster/import",
        Some(json!({"url": "https://example.test/share"})),
        Some(&cookie),
    )
    .await;
    assert_eq!(imported.status(), StatusCode::OK);
    assert_eq!(
        response_json(imported).await,
        json!({"code": 200, "msg": "success", "data": null})
    );

    let imported_without_body = send(
        &app,
        Method::POST,
        "/api/share/cluster/import",
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(imported_without_body.status(), StatusCode::OK);
    assert_eq!(
        response_json(imported_without_body).await,
        json!({"code": 200, "msg": "success", "data": null})
    );

    let malformed_import = send_raw(
        &app,
        Method::POST,
        "/api/share/cluster/import",
        "application/json",
        b"{not-json",
        Some(&cookie),
    )
    .await;
    assert_eq!(malformed_import.status(), StatusCode::OK);
    assert_eq!(
        response_json(malformed_import).await,
        json!({"code": 200, "msg": "success", "data": null})
    );
}

async fn test_router(runner: FakeCommandRunner) -> (Router, TempDir, FakeCommandRunner) {
    test_router_with_config(runner, {
        let mut config = test_config();
        config.wan_ip = "203.0.113.7".to_owned();
        config
    })
    .await
}

async fn test_router_with_config(
    runner: FakeCommandRunner,
    config: AppConfig,
) -> (Router, TempDir, FakeCommandRunner) {
    let dir = tempdir().unwrap();
    write_password_file(dir.path());
    let pool = connect_sqlite_memory().await.unwrap();
    migrate(&pool).await.unwrap();
    let state = AppState::new_with_command_runner(
        config,
        pool,
        SessionStore::new(),
        dir.path(),
        runner.clone(),
    )
    .with_backup_c_save_delay(Duration::ZERO);
    (build_router(state), dir, runner)
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
    if let Some(cookie) = cookie {
        builder = builder.header(COOKIE, cookie);
    }
    let body = if let Some(value) = json_body {
        builder = builder.header(CONTENT_TYPE, "application/json");
        Body::from(serde_json::to_vec(&value).unwrap())
    } else {
        Body::empty()
    };
    app.clone()
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap()
}

async fn send_raw(
    app: &Router,
    method: Method,
    uri: &str,
    content_type: &str,
    body: &[u8],
    cookie: Option<&str>,
) -> Response<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(CONTENT_TYPE, content_type);
    if let Some(cookie) = cookie {
        builder = builder.header(COOKIE, cookie);
    }
    app.clone()
        .oneshot(builder.body(Body::from(body.to_vec())).unwrap())
        .await
        .unwrap()
}

async fn send_multipart_file(
    app: &Router,
    uri: &str,
    filename: &str,
    contents: &[u8],
    cookie: &str,
) -> Response<Body> {
    let boundary = "dst-admin-rust-boundary";
    let mut body = Vec::new();
    write!(
        &mut body,
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: application/zip\r\n\r\n"
    )
    .unwrap();
    body.extend_from_slice(contents);
    write!(&mut body, "\r\n--{boundary}--\r\n").unwrap();
    app.clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(uri)
                .header(COOKIE, cookie)
                .header(
                    CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn response_json(response: Response<Body>) -> Value {
    let bytes = response_bytes(response).await;
    serde_json::from_slice(&bytes).unwrap()
}

async fn response_bytes(response: Response<Body>) -> Vec<u8> {
    to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec()
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

fn write_cluster_fixture(root: &Path, cluster: &str) {
    fs::write(
        root.join("dst_config"),
        format!(
            "steamcmd={0}/steamcmd\nforce_install_dir={0}/dst-dedicated-server\ncluster={cluster}\nbackup={0}/backup\nmod_download_path={0}/mods\nbin=64\nbeta=0\n",
            root.display()
        ),
    )
    .unwrap();
    fs::create_dir_all(root.join("backup")).unwrap();
    fs::create_dir_all(root.join("dst-dedicated-server/mods")).unwrap();
    fs::write(root.join("dst-dedicated-server/version.txt"), "676042\n").unwrap();
    let cluster_dir = root.join(".klei/DoNotStarveTogether").join(cluster);
    fs::create_dir_all(cluster_dir.join("Master")).unwrap();
    fs::create_dir_all(cluster_dir.join("Caves")).unwrap();
    fs::write(
        cluster_dir.join("cluster.ini"),
        cluster_ini(cluster, "survival"),
    )
    .unwrap();
    fs::write(cluster_dir.join("cluster_token.txt"), "cluster-token-file").unwrap();
    fs::write(
        cluster_dir.join("level.json"),
        r#"{"levelList":[{"name":"森林","file":"Master"},{"name":"洞穴","file":"Caves"}]}"#,
    )
    .unwrap();
    for (level, is_master, port) in [("Master", true, 10999), ("Caves", false, 11000)] {
        fs::write(
            cluster_dir.join(level).join("leveldataoverride.lua"),
            "return {}",
        )
        .unwrap();
        fs::write(
            cluster_dir.join(level).join("modoverrides.lua"),
            "return {}",
        )
        .unwrap();
        fs::write(
            cluster_dir.join(level).join("server.ini"),
            format!(
                "[NETWORK]\nserver_port = {port}\n[SHARD]\nis_master = {is_master}\nname = {level}\nid = 10000\n[ACCOUNT]\nencode_user_path = true\n"
            ),
        )
        .unwrap();
    }
}

fn cluster_ini(cluster: &str, mode: &str) -> String {
    format!(
        "[GAMEPLAY]\ngame_mode = {mode}\n[NETWORK]\ncluster_name = {cluster}\ncluster_description = test world\ncluster_password = secret\ncluster_intention = cooperative\nmax_players = 6\n[MISC]\nmax_snapshots = 6\n"
    )
}

fn build_restore_zip() -> Vec<u8> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        zip.start_file("Imported/cluster.ini", options).unwrap();
        zip.write_all(b"[GAMEPLAY]\ngame_mode = endless\n[NETWORK]\ncluster_name = Restored World\ncluster_description = restored\ncluster_password = restored-pass\ncluster_intention = cooperative\nmax_players = 8\n[MISC]\nmax_snapshots = 6\n").unwrap();
        zip.start_file("Imported/cluster_token.txt", options)
            .unwrap();
        zip.write_all(b"restored-token").unwrap();
        zip.start_file("Imported/Master/modoverrides.lua", options)
            .unwrap();
        zip.write_all(b"return { [\"workshop-123\"] = { enabled = true } }")
            .unwrap();
        zip.start_file("Imported/Master/server.ini", options)
            .unwrap();
        zip.write_all(b"[NETWORK]\nserver_port = 10999\n[SHARD]\nis_master = true\nname = Master\nid = 10000\n[ACCOUNT]\nencode_user_path = true\n").unwrap();
        zip.start_file("Imported/Master/leveldataoverride.lua", options)
            .unwrap();
        zip.write_all(b"return {}").unwrap();
        zip.start_file("Imported/level.json", options).unwrap();
        zip.write_all(r#"{"levelList":[{"name":"森林","file":"Master"}]}"#.as_bytes())
            .unwrap();
        zip.finish().unwrap();
    }
    cursor.into_inner()
}

fn build_evil_restore_zip() -> Vec<u8> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        zip.start_file("Imported/cluster.ini", options).unwrap();
        zip.write_all(cluster_ini("Evil", "survival").as_bytes())
            .unwrap();
        zip.start_file("../pwned.txt", options).unwrap();
        zip.write_all(b"pwned").unwrap();
        zip.finish().unwrap();
    }
    cursor.into_inner()
}

fn build_huge_restore_zip() -> Vec<u8> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        zip.start_file("Imported/cluster.ini", options).unwrap();
        zip.write_all(cluster_ini("Huge", "survival").as_bytes())
            .unwrap();
        zip.start_file("Imported/Master/save/session/blob.bin", options)
            .unwrap();
        let chunk = vec![b'x'; 1024 * 1024];
        for _ in 0..130 {
            zip.write_all(&chunk).unwrap();
        }
        zip.finish().unwrap();
    }
    cursor.into_inner()
}

fn build_zip_eocd_declaring_entries(entries: u16) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"PK\x05\x06");
    bytes.extend_from_slice(&0_u16.to_le_bytes());
    bytes.extend_from_slice(&0_u16.to_le_bytes());
    bytes.extend_from_slice(&entries.to_le_bytes());
    bytes.extend_from_slice(&entries.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&0_u16.to_le_bytes());
    bytes
}

fn hidden_restore_dirs(root: &Path, cluster: &str) -> Vec<String> {
    let prefix = format!(".dst-admin-rust-restore-{cluster}-");
    fs::read_dir(root.join(".klei/DoNotStarveTogether"))
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with(&prefix))
        .collect()
}

fn write_meta_fixture(root: &Path, cluster: &str) {
    let session_dir = root
        .join(".klei/DoNotStarveTogether")
        .join(cluster)
        .join("Master/save/session/session-1");
    fs::create_dir_all(&session_dir).unwrap();
    fs::write(
        session_dir.join("0000000001.meta"),
        r#"return {
    clock = {
        totaltimeinphase = 200,
        cycles = 12,
        phase = "dusk",
        remainingtimeinphase = 0.25,
        mooomphasecycle = 7,
        segs = { night = 2, day = 11, dusk = 3 },
    },
    seasons = {
        premode = true,
        season = "spring",
        elapseddaysinseason = 3,
        israndom = { summer = true, autumn = false, spring = true, winter = false },
        lengths = { summer = 5, autumn = 20, spring = 20, winter = 15 },
        remainingdaysinseason = 17,
        mode = "cycle",
        totaldaysinseason = 20,
        segs = { spring = 4 },
    },
}"#,
    )
    .unwrap();
}

#[tokio::test]
async fn snapshot_setting_repository_keeps_a_single_active_row() {
    let pool = connect_sqlite_memory().await.unwrap();
    migrate(&pool).await.unwrap();
    let repository = BackupSnapshotRepository::new(pool.clone());

    let first = repository
        .save_singleton(json_snapshot(15, 2, 1, 1))
        .await
        .unwrap();
    let second = repository
        .save_singleton(json_snapshot(45, 6, 0, 0))
        .await
        .unwrap();

    assert_eq!(first.id, second.id);
    let active_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM backup_snapshots WHERE deleted_at IS NULL")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(active_rows, 1);
    assert_eq!(second.name, "");
    assert_eq!(second.interval, 45);
    assert_eq!(second.max_snapshots, 6);
    assert_eq!(second.enable, 0);
    assert_eq!(second.is_c_save, 0);
}

fn json_snapshot(
    interval: i64,
    max_snapshots: i64,
    enable: i64,
    is_c_save: i64,
) -> dst_admin_rust::domain::backup::model::SaveBackupSnapshot {
    serde_json::from_value(json!({
        "name": "ignored",
        "interval": interval,
        "maxSnapshots": max_snapshots,
        "enable": enable,
        "isCSave": is_c_save
    }))
    .unwrap()
}
