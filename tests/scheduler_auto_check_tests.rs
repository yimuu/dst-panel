use std::{fs, io, path::Path, sync::Arc, time::Duration};

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
    domain::scheduler::runtime::{RuntimeState, SchedulerRuntimeContext, run_due_tasks_once},
    infra::command::{CommandOutput, FakeCommandRunner},
    infra::config::{AppConfig, AutoUpdateModinfoConfig},
    infra::db::{SqlitePool, connect_sqlite_memory, migrate},
    infra::process::{ProcessSnapshot, ProcessSnapshotProvider},
    web::app::{AppState, build_router},
};
use serde_json::{Value, json};
use tempfile::{TempDir, tempdir};
use tower::ServiceExt;

#[tokio::test]
async fn announcement_routes_save_new_rows_and_return_first_active_setting() {
    let (app, dir, pool) = test_router().await;
    let cookie = login(&app).await;
    write_cluster_fixture(dir.path(), "ClusterAnnouncement");

    let empty = send(
        &app,
        Method::GET,
        "/api/game/announce/setting",
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(empty.status(), StatusCode::OK);
    assert_eq!(
        response_json(empty).await,
        json!({
            "code": 200,
            "msg": "success",
            "data": {
                "ID": 0,
                "CreatedAt": "0001-01-01T00:00:00Z",
                "UpdatedAt": "0001-01-01T00:00:00Z",
                "DeletedAt": null,
                "enable": false,
                "frequency": 0,
                "interval": 0,
                "intervalUnit": "",
                "method": "",
                "content": ""
            }
        })
    );

    let first = send(
        &app,
        Method::POST,
        "/api/game/announce/setting",
        Some(json!({
            "enable": true,
            "frequency": 2,
            "interval": 10,
            "intervalUnit": "M",
            "method": "order",
            "content": "first\nmessage"
        })),
        Some(&cookie),
    )
    .await;
    assert_eq!(first.status(), StatusCode::OK);
    let first_body = response_json(first).await;
    assert_eq!(first_body["code"], 200);
    assert_eq!(first_body["msg"], "success");
    assert_eq!(first_body["data"]["enable"], true);
    assert_eq!(first_body["data"]["content"], "first\nmessage");
    assert!(first_body["data"]["ID"].as_i64().unwrap() > 0);

    let second = send(
        &app,
        Method::POST,
        "/api/game/announce/setting",
        Some(json!({
            "enable": false,
            "frequency": 5,
            "interval": 20,
            "intervalUnit": "S",
            "method": "broadcast",
            "content": "second message"
        })),
        Some(&cookie),
    )
    .await;
    assert_eq!(second.status(), StatusCode::OK);

    let row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM announces")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row_count, 2);

    let fetched = send(
        &app,
        Method::GET,
        "/api/game/announce/setting",
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(fetched.status(), StatusCode::OK);
    let fetched_body = response_json(fetched).await;
    assert_eq!(fetched_body["data"]["content"], "first\nmessage");
    assert_eq!(fetched_body["data"]["enable"], true);
}

#[tokio::test]
async fn task_routes_create_list_delete_and_validate_cron_requests() {
    let (app, dir, pool) = test_router().await;
    let cookie = login(&app).await;
    write_cluster_fixture(dir.path(), "ClusterTasks");

    let instruct = send(&app, Method::GET, "/api/task/instruct", None, Some(&cookie)).await;
    assert_eq!(instruct.status(), StatusCode::OK);
    assert_eq!(
        response_json(instruct).await,
        json!({"code": 200, "msg": "success", "data": [{"backup": "备份"}, {"update": "更新"}]})
    );

    let empty_cron = send(
        &app,
        Method::POST,
        "/api/task",
        Some(json!({"cron": "", "category": "backup"})),
        Some(&cookie),
    )
    .await;
    assert_eq!(empty_cron.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_json(empty_cron).await,
        json!({"code": 400, "msg": "cron 表达式不能为空", "data": null})
    );

    for cron in [
        "not-a-cron",
        "99 99 99 99 99",
        "*/0 * * * *",
        "10-5 * * * *",
        "* * * FOO *",
    ] {
        let malformed_cron = send(
            &app,
            Method::POST,
            "/api/task",
            Some(json!({"cron": cron, "category": "backup"})),
            Some(&cookie),
        )
        .await;
        assert_eq!(malformed_cron.status(), StatusCode::BAD_REQUEST);
        let body = response_json(malformed_cron).await;
        assert_eq!(body["code"], 400);
        assert_eq!(body["data"], Value::Null);
        assert!(
            body["msg"]
                .as_str()
                .unwrap()
                .starts_with("cron 表达式格式错误")
        );
    }

    let invalid_category = send(
        &app,
        Method::POST,
        "/api/task",
        Some(json!({"cron": "*/5 * * * *", "category": "bad-category"})),
        Some(&cookie),
    )
    .await;
    assert_eq!(invalid_category.status(), StatusCode::BAD_REQUEST);
    let invalid_category_body = response_json(invalid_category).await;
    assert_eq!(invalid_category_body["code"], 400);
    assert_eq!(invalid_category_body["data"], Value::Null);
    assert!(
        invalid_category_body["msg"]
            .as_str()
            .unwrap()
            .contains("category")
    );
    let rows_after_reject: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM job_tasks")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(rows_after_reject, 0);

    let created = send(
        &app,
        Method::POST,
        "/api/task",
        Some(json!({
            "levelName": "森林",
            "uuid": "Master",
            "cron": "CRON_TZ=UTC */5 * * * *",
            "category": "none",
            "comment": "heartbeat",
            "announcement": "hello",
            "sleep": 1,
            "times": 2,
            "script": 0
        })),
        Some(&cookie),
    )
    .await;
    assert_eq!(created.status(), StatusCode::OK);
    assert_eq!(
        response_json(created).await,
        json!({"code": 200, "msg": "success", "data": null})
    );

    let listed = send(&app, Method::GET, "/api/task", None, Some(&cookie)).await;
    assert_eq!(listed.status(), StatusCode::OK);
    let listed_body = response_json(listed).await;
    let task = &listed_body["data"][0];
    assert_eq!(task["clusterName"], "ClusterTasks");
    assert_eq!(task["levelName"], "森林");
    assert_eq!(task["uuid"], "Master");
    assert_eq!(task["cron"], "CRON_TZ=UTC */5 * * * *");
    assert_eq!(task["category"], "none");
    assert_eq!(task["comment"], "heartbeat");
    assert_eq!(task["announcement"], "hello");
    assert_eq!(task["valid"], true);
    assert!(task["jobId"].as_i64().unwrap() > 0);
    assert!(task.get("next").is_some());
    assert!(task.get("prev").is_some());

    let job_id = task["jobId"].as_i64().unwrap();
    let deleted = send(
        &app,
        Method::DELETE,
        &format!("/api/task?jobId={job_id}"),
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(deleted.status(), StatusCode::OK);
    assert_eq!(
        response_json(deleted).await,
        json!({"code": 200, "msg": "success", "data": null})
    );

    let listed_after_delete = send(&app, Method::GET, "/api/task", None, Some(&cookie)).await;
    assert_eq!(listed_after_delete.status(), StatusCode::OK);
    assert_eq!(response_json(listed_after_delete).await["data"], json!([]));

    let soft_deleted_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM job_tasks WHERE deleted_at IS NOT NULL")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(soft_deleted_rows, 1);

    let delete_missing = send(
        &app,
        Method::DELETE,
        "/api/task?jobId=abc",
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(delete_missing.status(), StatusCode::OK);
    assert_eq!(
        response_json(delete_missing).await,
        json!({"code": 200, "msg": "success", "data": null})
    );
}

#[tokio::test]
async fn scheduler_runtime_runs_due_task_announcements_once_per_minute() {
    let dir = tempdir().unwrap();
    write_cluster_fixture(dir.path(), "ClusterRuntime");
    let pool = connect_sqlite_memory().await.unwrap();
    migrate(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO job_tasks (
            created_at, updated_at, deleted_at, cluster_name, level_name, uuid,
            cron, category, comment, announcement, sleep, times, script
         ) VALUES (
            '2026-06-21T12:00:00Z', '2026-06-21T12:00:00Z', NULL,
            'ClusterRuntime', '森林', 'Master', '* * * * *', 'none',
            'announce only', 'hello', 0, 1, 0
         )",
    )
    .execute(&pool)
    .await
    .unwrap();

    let runner = FakeCommandRunner::new(vec![
        CommandOutput::success(Vec::new(), Vec::new()),
        CommandOutput::success(Vec::new(), Vec::new()),
    ]);
    let context = SchedulerRuntimeContext::new(
        dir.path().to_path_buf(),
        pool,
        Arc::new(runner.clone()),
        Arc::new(EmptyProcessSnapshotProvider),
        Duration::ZERO,
    );
    let mut runtime_state = RuntimeState::default();
    let now = chrono::DateTime::parse_from_rfc3339("2026-06-21T12:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);

    let executed = run_due_tasks_once(&context, now, &mut runtime_state)
        .await
        .unwrap();
    assert_eq!(executed, 1);

    let calls = runner.calls();
    assert_eq!(calls.len(), 2);
    assert_screen_call(
        &calls[0],
        "DST_8level_ClusterRuntime_Master",
        "c_announce(\"hello\")\n",
    );
    assert_screen_call(
        &calls[1],
        "DST_8level_ClusterRuntime_Caves",
        "c_announce(\"hello\")\n",
    );

    let executed_again = run_due_tasks_once(&context, now, &mut runtime_state)
        .await
        .unwrap();
    assert_eq!(executed_again, 0);
    assert_eq!(runner.calls().len(), 2);
}

#[tokio::test]
async fn scheduler_runtime_respects_range_step_from_range_start() {
    let dir = tempdir().unwrap();
    write_cluster_fixture(dir.path(), "ClusterRangeStep");
    let pool = connect_sqlite_memory().await.unwrap();
    migrate(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO job_tasks (
            created_at, updated_at, deleted_at, cluster_name, level_name, uuid,
            cron, category, comment, announcement, sleep, times, script
         ) VALUES (
            '2026-06-21T12:00:00Z', '2026-06-21T12:00:00Z', NULL,
            'ClusterRangeStep', '森林', 'Master', '5-10/2 * * * *', 'none',
            'range step announcement', 'range-step', 0, 1, 0
         )",
    )
    .execute(&pool)
    .await
    .unwrap();

    let runner = FakeCommandRunner::new(vec![
        CommandOutput::success(Vec::new(), Vec::new()),
        CommandOutput::success(Vec::new(), Vec::new()),
    ]);
    let context = SchedulerRuntimeContext::new(
        dir.path().to_path_buf(),
        pool,
        Arc::new(runner.clone()),
        Arc::new(EmptyProcessSnapshotProvider),
        Duration::ZERO,
    );
    let mut runtime_state = RuntimeState::default();
    let now = chrono::DateTime::parse_from_rfc3339("2026-06-21T12:05:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);

    let executed = run_due_tasks_once(&context, now, &mut runtime_state)
        .await
        .unwrap();

    assert_eq!(executed, 1);
    assert_eq!(runner.calls().len(), 2);
}

#[tokio::test]
async fn scheduler_runtime_respects_cron_tz_wall_clock_time() {
    let dir = tempdir().unwrap();
    write_cluster_fixture(dir.path(), "ClusterCronTz");
    let pool = connect_sqlite_memory().await.unwrap();
    migrate(&pool).await.unwrap();
    insert_runtime_announcement_task(
        &pool,
        "ClusterCronTz",
        "CRON_TZ=Asia/Shanghai 0 16 * * *",
        "cron tz announcement",
        "cron-tz",
    )
    .await;

    let runner = FakeCommandRunner::new(vec![
        CommandOutput::success(Vec::new(), Vec::new()),
        CommandOutput::success(Vec::new(), Vec::new()),
    ]);
    let context = SchedulerRuntimeContext::new(
        dir.path().to_path_buf(),
        pool,
        Arc::new(runner.clone()),
        Arc::new(EmptyProcessSnapshotProvider),
        Duration::ZERO,
    );
    let mut runtime_state = RuntimeState::default();
    let now = chrono::DateTime::parse_from_rfc3339("2026-06-21T08:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);

    let executed = run_due_tasks_once(&context, now, &mut runtime_state)
        .await
        .unwrap();

    assert_eq!(executed, 1);
    assert_eq!(runner.calls().len(), 2);
}

#[tokio::test]
async fn scheduler_runtime_uses_robfig_dom_dow_or_when_both_restricted() {
    let dir = tempdir().unwrap();
    write_cluster_fixture(dir.path(), "ClusterDomDow");
    let pool = connect_sqlite_memory().await.unwrap();
    migrate(&pool).await.unwrap();
    insert_runtime_announcement_task(
        &pool,
        "ClusterDomDow",
        "CRON_TZ=UTC 0 12 15 * MON",
        "dom dow announcement",
        "dom-dow",
    )
    .await;

    let runner = FakeCommandRunner::new(vec![
        CommandOutput::success(Vec::new(), Vec::new()),
        CommandOutput::success(Vec::new(), Vec::new()),
    ]);
    let context = SchedulerRuntimeContext::new(
        dir.path().to_path_buf(),
        pool,
        Arc::new(runner.clone()),
        Arc::new(EmptyProcessSnapshotProvider),
        Duration::ZERO,
    );
    let mut runtime_state = RuntimeState::default();
    let now = chrono::DateTime::parse_from_rfc3339("2026-06-22T12:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);

    let executed = run_due_tasks_once(&context, now, &mut runtime_state)
        .await
        .unwrap();

    assert_eq!(executed, 1);
    assert_eq!(runner.calls().len(), 2);
}

#[tokio::test]
async fn scheduler_runtime_accepts_decimal_go_every_durations() {
    let dir = tempdir().unwrap();
    write_cluster_fixture(dir.path(), "ClusterEveryDecimal");
    let pool = connect_sqlite_memory().await.unwrap();
    migrate(&pool).await.unwrap();
    insert_runtime_announcement_task(
        &pool,
        "ClusterEveryDecimal",
        "@every 1.5h",
        "decimal every announcement",
        "decimal-every",
    )
    .await;

    let runner = FakeCommandRunner::new(vec![
        CommandOutput::success(Vec::new(), Vec::new()),
        CommandOutput::success(Vec::new(), Vec::new()),
    ]);
    let context = SchedulerRuntimeContext::new(
        dir.path().to_path_buf(),
        pool,
        Arc::new(runner.clone()),
        Arc::new(EmptyProcessSnapshotProvider),
        Duration::ZERO,
    );
    let mut runtime_state = RuntimeState::default();
    let now = chrono::DateTime::parse_from_rfc3339("2026-06-21T12:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);

    let executed = run_due_tasks_once(&context, now, &mut runtime_state)
        .await
        .unwrap();

    assert_eq!(executed, 1);
    assert_eq!(runner.calls().len(), 2);
}

#[tokio::test]
async fn scheduler_runtime_executes_due_backup_strategy() {
    let dir = tempdir().unwrap();
    write_cluster_fixture(dir.path(), "ClusterBackupRuntime");
    let pool = connect_sqlite_memory().await.unwrap();
    migrate(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO job_tasks (
            created_at, updated_at, deleted_at, cluster_name, level_name, uuid,
            cron, category, comment, announcement, sleep, times, script
         ) VALUES (
            '2026-06-21T12:00:00Z', '2026-06-21T12:00:00Z', NULL,
            'ClusterBackupRuntime', '', '', '* * * * *', 'backup',
            'backup cluster', '', 0, 0, 0
         )",
    )
    .execute(&pool)
    .await
    .unwrap();

    let runner = FakeCommandRunner::default();
    let context = SchedulerRuntimeContext::new(
        dir.path().to_path_buf(),
        pool,
        Arc::new(runner.clone()),
        Arc::new(EmptyProcessSnapshotProvider),
        Duration::ZERO,
    );
    let mut runtime_state = RuntimeState::default();
    let now = chrono::DateTime::parse_from_rfc3339("2026-06-21T12:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);

    let executed = run_due_tasks_once(&context, now, &mut runtime_state)
        .await
        .unwrap();

    assert_eq!(executed, 1);
    let backup_files = fs::read_dir(dir.path().join("backup"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert_eq!(backup_files.len(), 1);
    assert!(backup_files[0].ends_with(".zip"));
    assert!(runner.calls().is_empty());
}

#[tokio::test]
async fn auto_check_routes_generate_rows_and_overlay_persisted_settings() {
    let (app, dir, pool) = test_router().await;
    let cookie = login(&app).await;
    write_cluster_fixture(dir.path(), "ClusterAuto");

    sqlx::query(
        "INSERT INTO auto_checks (
            created_at, updated_at, deleted_at, name, cluster_name, level_name,
            uuid, enable, announcement, times, sleep, interval, check_type
        ) VALUES (
            '2026-06-14T03:00:00Z', '2026-06-14T03:00:00Z', NULL, 'foreign', 'OtherCluster', 'Other',
            'Master', 1, 'cross-cluster overlay', 9, 8, 7, 'LEVEL_MOD'
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    let listed = send(&app, Method::GET, "/api/auto/check2", None, Some(&cookie)).await;
    assert_eq!(listed.status(), StatusCode::OK);
    let listed_body = response_json(listed).await;
    let rows = listed_body["data"].as_array().unwrap();
    assert_eq!(rows.len(), 5);
    assert_eq!(rows[0]["name"], "");
    assert_eq!(rows[0]["clusterName"], "ClusterAuto");
    assert_eq!(rows[0]["CreatedAt"], "2026-06-14T03:00:00Z");
    assert_eq!(rows[0]["levelName"], "森林");
    assert_eq!(rows[0]["uuid"], "Master");
    assert_eq!(rows[0]["checkType"], "LEVEL_MOD");
    assert_eq!(rows[0]["ID"], 1);
    assert_eq!(rows[0]["enable"], 1);
    assert_eq!(rows[0]["announcement"], "cross-cluster overlay");
    assert_eq!(rows[0]["times"], 9);
    assert_eq!(rows[0]["sleep"], 8);
    assert_eq!(rows[0]["interval"], 7);
    assert_eq!(rows[1]["name"], "");
    assert_eq!(rows[1]["CreatedAt"], "0001-01-01T00:00:00Z");
    assert_eq!(rows[1]["UpdatedAt"], "0001-01-01T00:00:00Z");
    assert_eq!(rows[1]["checkType"], "LEVEL_DOWN");
    assert_eq!(rows[4]["name"], "");
    assert_eq!(rows[4]["levelName"], "ClusterAuto");
    assert_eq!(rows[4]["uuid"], "");
    assert_eq!(rows[4]["checkType"], "UPDATE_GAME");
    assert_eq!(rows[4]["enable"], 0);

    let saved = send(
        &app,
        Method::POST,
        "/api/auto/check2",
        Some(json!({
            "name": "custom update",
            "clusterName": "ClusterAuto",
            "levelName": "",
            "uuid": "",
            "enable": 1,
            "announcement": "update soon",
            "times": 3,
            "sleep": 4,
            "interval": 6,
            "checkType": "UPDATE_GAME"
        })),
        Some(&cookie),
    )
    .await;
    assert_eq!(saved.status(), StatusCode::OK);
    let saved_body = response_json(saved).await;
    let saved_id = saved_body["data"]["ID"].as_i64().unwrap();
    assert!(saved_id > 0);
    assert_eq!(saved_body["data"]["uuid"], "UPDATE_GAME_ClusterAuto");

    let filtered = send(
        &app,
        Method::GET,
        "/api/auto/check2?checkType=UPDATE_GAME",
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(filtered.status(), StatusCode::OK);
    let filtered_body = response_json(filtered).await;
    assert_eq!(filtered_body["data"][0]["ID"], saved_id);
    assert_eq!(filtered_body["data"][0]["uuid"], "UPDATE_GAME_ClusterAuto");
    assert_eq!(filtered_body["data"][0]["announcement"], "update soon");

    let unfiltered_again = send(&app, Method::GET, "/api/auto/check2", None, Some(&cookie)).await;
    assert_eq!(unfiltered_again.status(), StatusCode::OK);
    let unfiltered_body = response_json(unfiltered_again).await;
    assert_eq!(unfiltered_body["data"][4]["ID"], 0);
    assert_eq!(unfiltered_body["data"][4]["uuid"], "");
}

#[tokio::test]
async fn auto_check_get_initializes_missing_level_index_like_go() {
    let (app, dir, _pool) = test_router().await;
    let cookie = login(&app).await;
    write_cluster_fixture(dir.path(), "ClusterAutoFallback");
    let cluster_dir = dir
        .path()
        .join(".klei/DoNotStarveTogether/ClusterAutoFallback");
    fs::remove_file(cluster_dir.join("level.json")).unwrap();

    let listed = send(&app, Method::GET, "/api/auto/check2", None, Some(&cookie)).await;
    assert_eq!(listed.status(), StatusCode::OK);
    let body = response_json(listed).await;
    let rows = body["data"].as_array().unwrap();
    assert_eq!(rows.len(), 5);
    assert_eq!(rows[0]["levelName"], "森林");
    assert_eq!(rows[0]["uuid"], "Master");
    assert_eq!(rows[2]["levelName"], "洞穴");
    assert_eq!(rows[2]["uuid"], "Caves");
    assert_eq!(
        fs::read_to_string(cluster_dir.join("level.json")).unwrap(),
        r#"{"levelList":[{"name":"森林","file":"Master"},{"name":"洞穴","file":"Caves"}]}"#
    );
}

#[tokio::test]
async fn webhook_verifies_key_and_never_creates_query_key_path() {
    let (app, dir, _pool) = test_router().await;
    let cookie = login(&app).await;
    write_cluster_fixture(dir.path(), "ClusterWebhook");
    fs::write(dir.path().join("key"), "secret\n").unwrap();

    let missing_key = send(
        &app,
        Method::POST,
        "/webhook",
        Some(json!({"msgtype": "unknown", "param": null})),
        Some(&cookie),
    )
    .await;
    assert_eq!(missing_key.status(), StatusCode::BAD_REQUEST);

    let wrong_key = send(
        &app,
        Method::POST,
        "/webhook?key=created-by-query",
        Some(json!({"msgtype": "unknown", "param": null})),
        Some(&cookie),
    )
    .await;
    assert_eq!(wrong_key.status(), StatusCode::BAD_REQUEST);
    assert!(!dir.path().join("created-by-query").exists());

    #[cfg(unix)]
    {
        let symlink_dir = tempdir().unwrap();
        let symlink_target = symlink_dir.path().join("outside-key");
        fs::write(&symlink_target, "outside-secret\n").unwrap();
        fs::remove_file(dir.path().join("key")).unwrap();
        std::os::unix::fs::symlink(&symlink_target, dir.path().join("key")).unwrap();
        let symlinked_key = send(
            &app,
            Method::POST,
            "/webhook?key=outside-secret",
            Some(json!({"msgtype": "unknown", "param": null})),
            Some(&cookie),
        )
        .await;
        assert_eq!(symlinked_key.status(), StatusCode::BAD_REQUEST);
        fs::remove_file(dir.path().join("key")).unwrap();
        fs::write(dir.path().join("key"), "secret\n").unwrap();
    }

    let default_response = send(
        &app,
        Method::POST,
        "/webhook?key=secret",
        Some(json!({"msgtype": "unknown", "param": null})),
        Some(&cookie),
    )
    .await;
    assert_eq!(default_response.status(), StatusCode::OK);
    assert_eq!(
        response_json(default_response).await,
        json!({"code": 200, "msg": "success", "data": null})
    );

    let home_info = send(
        &app,
        Method::POST,
        "/webhook?key=secret",
        Some(json!({"msgtype": "homeInfo", "param": null})),
        Some(&cookie),
    )
    .await;
    assert_eq!(home_info.status(), StatusCode::OK);
    assert_eq!(
        response_json(home_info).await["data"]["clusterName"],
        "ClusterWebhook"
    );

    let online_players = send(
        &app,
        Method::POST,
        "/webhook?key=secret",
        Some(json!({"msgtype": "onlinePlayers", "param": null})),
        Some(&cookie),
    )
    .await;
    assert_eq!(online_players.status(), StatusCode::OK);
    assert_eq!(response_json(online_players).await["data"], json!([]));

    let search_home = send(
        &app,
        Method::POST,
        "/webhook?key=secret",
        Some(json!({"msgtype": "searchHome", "param": null})),
        Some(&cookie),
    )
    .await;
    assert_eq!(search_home.status(), StatusCode::OK);
    assert!(response_bytes(search_home).await.is_empty());
}

async fn test_router() -> (Router, TempDir, SqlitePool) {
    let dir = tempdir().unwrap();
    write_password_file(dir.path());
    let pool = connect_sqlite_memory().await.unwrap();
    migrate(&pool).await.unwrap();
    let state = AppState::new(test_config(), pool.clone(), SessionStore::new(), dir.path());
    (build_router(state), dir, pool)
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
        wan_ip: "203.0.113.9".to_owned(),
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

#[derive(Debug, Clone)]
struct EmptyProcessSnapshotProvider;

impl ProcessSnapshotProvider for EmptyProcessSnapshotProvider {
    fn snapshots(&self) -> io::Result<Vec<ProcessSnapshot>> {
        Ok(Vec::new())
    }
}

async fn insert_runtime_announcement_task(
    pool: &SqlitePool,
    cluster_name: &str,
    cron: &str,
    comment: &str,
    announcement: &str,
) {
    sqlx::query(
        "INSERT INTO job_tasks (
            created_at, updated_at, deleted_at, cluster_name, level_name, uuid,
            cron, category, comment, announcement, sleep, times, script
         ) VALUES (
            '2026-06-21T12:00:00Z', '2026-06-21T12:00:00Z', NULL,
            ?, '森林', 'Master', ?, 'none', ?, ?, 0, 1, 0
         )",
    )
    .bind(cluster_name)
    .bind(cron)
    .bind(comment)
    .bind(announcement)
    .execute(pool)
    .await
    .unwrap();
}

fn assert_screen_call(
    spec: &dst_admin_rust::infra::command::CommandSpec,
    session: &str,
    command: &str,
) {
    assert_eq!(spec.program(), "screen");
    assert_eq!(
        spec.args(),
        ["-S", session, "-p", "0", "-X", "stuff", command]
    );
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
        format!(
            "[GAMEPLAY]\ngame_mode = survival\n[NETWORK]\ncluster_name = {cluster}\ncluster_description = test world\ncluster_password = secret\ncluster_intention = cooperative\nmax_players = 6\n[MISC]\nmax_snapshots = 6\n"
        ),
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
