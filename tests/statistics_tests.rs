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
    infra::db::{SqlitePool, connect_sqlite_memory, migrate},
    web::app::{AppState, build_router},
};
use serde_json::{Value, json};
use tempfile::{TempDir, tempdir};
use tower::ServiceExt;

#[tokio::test]
async fn player_log_route_filters_pages_and_soft_deletes_rows() {
    let (app, dir, pool) = test_router().await;
    let cookie = login(&app).await;
    write_dst_config(dir.path(), "ClusterStats");

    let alice_id = insert_player_log(
        &pool,
        PlayerLogFixture {
            created_at: "2026-01-01T10:00:00.000Z",
            name: "Alice",
            role: "Wilson",
            ku_id: "KU_ALICE",
            steam_id: "STEAM_ALICE",
            action: "[JoinAnnouncement]",
            action_desc: "Alice joined",
            ip: "10.0.0.1",
            cluster_name: "ClusterStats",
            deleted_at: None,
        },
    )
    .await;
    insert_player_log(
        &pool,
        PlayerLogFixture {
            created_at: "2026-01-01T09:00:00.000Z",
            name: "Bob",
            role: "Wendy",
            ku_id: "KU_BOB",
            steam_id: "STEAM_BOB",
            action: "[DeathAnnouncement]",
            action_desc: "Bob died",
            ip: "10.0.0.2",
            cluster_name: "ClusterStats",
            deleted_at: None,
        },
    )
    .await;
    let already_deleted_id = insert_player_log(
        &pool,
        PlayerLogFixture {
            created_at: "2026-01-01T11:00:00.000Z",
            name: "Alice Deleted",
            role: "Wilson",
            ku_id: "KU_DELETED",
            steam_id: "STEAM_ALICE",
            action: "[JoinAnnouncement]",
            action_desc: "deleted row",
            ip: "10.0.0.9",
            cluster_name: "ClusterStats",
            deleted_at: Some("2026-01-02T00:00:00.000Z"),
        },
    )
    .await;

    let response = send(
        &app,
        Method::GET,
        "/api/player/log?page=1&size=1&name=Ali&kuId=KU_ALICE",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["code"], 200);
    assert_eq!(body["msg"], "success");
    assert_eq!(body["data"]["page"], 1);
    assert_eq!(body["data"]["size"], 1);
    assert_eq!(body["data"]["total"], 1);
    assert_eq!(body["data"]["totalPages"], 1);
    assert_eq!(body["data"]["data"][0]["ID"], alice_id);
    assert_eq!(body["data"]["data"][0]["name"], "Alice");
    assert_eq!(body["data"]["data"][0]["kuId"], "KU_ALICE");
    assert_eq!(body["data"]["data"][0]["steamId"], "STEAM_ALICE");
    assert_eq!(body["data"]["data"][0]["actionDesc"], "Alice joined");
    assert_eq!(body["data"]["data"][0]["clusterName"], "ClusterStats");

    let response = send(
        &app,
        Method::POST,
        "/api/player/log/delete",
        Some(json!({"ids": [alice_id, already_deleted_id]})),
        Some(&cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!({"code": 200, "msg": "success", "data": null})
    );

    let response = send(
        &app,
        Method::GET,
        "/api/player/log?page=1&size=10&name=Ali&steamId=STEAM_ALICE",
        None,
        Some(&cookie),
    )
    .await;
    let body = response_json(response).await;
    assert_eq!(body["data"]["total"], 0);
    assert_eq!(body["data"]["data"], json!([]));

    let (updated_at, deleted_at): (Option<String>, Option<String>) =
        sqlx::query_as("SELECT updated_at, deleted_at FROM player_logs WHERE id = ?")
            .bind(alice_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(updated_at.as_deref(), Some("2026-01-01T10:00:00.000Z"));
    assert!(deleted_at.is_some());

    let already_deleted_at: Option<String> =
        sqlx::query_scalar("SELECT deleted_at FROM player_logs WHERE id = ?")
            .bind(already_deleted_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        already_deleted_at.as_deref(),
        Some("2026-01-02T00:00:00.000Z")
    );
}

#[tokio::test]
async fn player_log_route_preserves_go_steam_id_filter_typo() {
    let (app, dir, pool) = test_router().await;
    let cookie = login(&app).await;
    write_dst_config(dir.path(), "ClusterStats");

    insert_player_log(
        &pool,
        PlayerLogFixture {
            created_at: "2026-01-01T10:00:00.000Z",
            name: "Alice",
            role: "Wilson",
            ku_id: "KU_ALICE",
            steam_id: "STEAM_ALICE",
            action: "[JoinAnnouncement]",
            action_desc: "Alice joined",
            ip: "10.0.0.1",
            cluster_name: "ClusterStats",
            deleted_at: None,
        },
    )
    .await;

    let response = send(
        &app,
        Method::GET,
        "/api/player/log?page=1&size=10&steamId=STEAM_ALICE",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["code"], 200);
    assert_eq!(body["data"]["page"], 1);
    assert_eq!(body["data"]["size"], 10);
    assert_eq!(body["data"]["total"], 0);
    assert_eq!(body["data"]["totalPages"], 0);
    assert_eq!(body["data"]["data"], json!([]));
}

#[tokio::test]
async fn player_log_route_preserves_uncapped_page_size_without_overflowing_total_pages() {
    let (app, dir, pool) = test_router().await;
    let cookie = login(&app).await;
    write_dst_config(dir.path(), "ClusterStats");

    for name in ["Alice", "Bob"] {
        insert_player_log(
            &pool,
            PlayerLogFixture {
                created_at: "2026-01-01T10:00:00.000Z",
                name,
                role: "Wilson",
                ku_id: "KU_SAFE",
                steam_id: "STEAM_SAFE",
                action: "[JoinAnnouncement]",
                action_desc: "joined",
                ip: "10.0.0.1",
                cluster_name: "ClusterStats",
                deleted_at: None,
            },
        )
        .await;
    }

    let response = send(
        &app,
        Method::GET,
        "/api/player/log?page=1&size=9223372036854775807",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["code"], 200);
    assert_eq!(body["data"]["size"], 9_223_372_036_854_775_807i64);
    assert_eq!(body["data"]["total"], 2);
    assert_eq!(body["data"]["totalPages"], 1);
    assert_eq!(body["data"]["data"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn statistics_routes_return_go_compatible_aggregates() {
    let (app, dir, pool) = test_router().await;
    let cookie = login(&app).await;
    write_dst_config(dir.path(), "ClusterStats");

    seed_statistics(&pool).await;

    let date_query = "startDate=2026-01-01T00:00:00.000Z&endDate=2026-01-02T00:00:00.000Z";

    let response = send(
        &app,
        Method::GET,
        &format!("/api/statistics/active/user?unit=DAY&{date_query}"),
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["code"], 200);
    assert_eq!(body["data"]["y1"], json!([2, 0]));
    assert_eq!(body["data"]["y2"], json!([3, 0]));
    assert_eq!(body["data"]["x"].as_array().unwrap().len(), 2);

    let response = send(
        &app,
        Method::GET,
        &format!("/api/statistics/top/death?N=2&{date_query}"),
        None,
        Some(&cookie),
    )
    .await;
    let body = response_json(response).await;
    assert_eq!(body["data"][0]["name"], "Alice");
    assert_eq!(body["data"][0]["count"], 2);
    assert_eq!(body["data"][0]["kuId"], "KU_ALICE");
    assert_eq!(body["data"][1]["name"], "Bob");
    assert_eq!(body["data"][1]["count"], 1);

    let response = send(
        &app,
        Method::GET,
        &format!("/api/statistics/top/login?N=2&{date_query}"),
        None,
        Some(&cookie),
    )
    .await;
    let body = response_json(response).await;
    assert_eq!(body["data"][0]["name"], "Alice");
    assert_eq!(body["data"][0]["count"], 2);
    assert_eq!(body["data"][0]["kuId"], "KU_ALICE");
    assert_eq!(body["data"][0]["steamId"], "STEAM_ALICE");

    let response = send(
        &app,
        Method::GET,
        &format!("/api/statistics/top/active?N=2&{date_query}"),
        None,
        Some(&cookie),
    )
    .await;
    let body = response_json(response).await;
    assert_eq!(body["data"][0]["name"], "Alice");
    assert_eq!(body["data"][0]["count"], 2);

    let response = send(
        &app,
        Method::GET,
        &format!("/api/statistics/rate/role?{date_query}"),
        None,
        Some(&cookie),
    )
    .await;
    let body = response_json(response).await;
    let mut roles = body["data"].as_array().unwrap().clone();
    roles.sort_by(|left, right| {
        left["role"]
            .as_str()
            .unwrap()
            .cmp(right["role"].as_str().unwrap())
    });
    assert_eq!(
        roles,
        vec![
            json!({"role": "Wendy", "count": 1}),
            json!({"role": "Wilson", "count": 1})
        ]
    );

    let response = send(
        &app,
        Method::GET,
        "/api/statistics/regenerate?N=1",
        None,
        Some(&cookie),
    )
    .await;
    let body = response_json(response).await;
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    assert_eq!(body["data"][0]["clusterName"], "ClusterDeleted");
}

#[tokio::test]
async fn statistics_routes_preserve_go_raw_limit_and_null_count_semantics() {
    let (app, dir, pool) = test_router().await;
    let cookie = login(&app).await;
    write_dst_config(dir.path(), "ClusterStats");
    seed_statistics(&pool).await;
    insert_null_name_player_log(&pool, "2026-01-01T16:00:00.000Z", "[JoinAnnouncement]").await;

    let date_query = "startDate=2026-01-01T00:00:00.000Z&endDate=2026-01-02T00:00:00.000Z";

    for uri in [
        format!("/api/statistics/top/death?{date_query}"),
        format!("/api/statistics/top/death?N=0&{date_query}"),
        format!("/api/statistics/top/death?N=abc&{date_query}"),
        "/api/statistics/regenerate".to_owned(),
        "/api/statistics/regenerate?N=0".to_owned(),
        "/api/statistics/regenerate?N=abc".to_owned(),
    ] {
        let response = send(&app, Method::GET, &uri, None, Some(&cookie)).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_json(response).await;
        assert_eq!(body["code"], 200);
        assert_eq!(body["data"], json!([]));
    }

    let response = send(
        &app,
        Method::GET,
        "/api/statistics/regenerate?N=-1",
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["data"].as_array().unwrap().len(), 3);

    let response = send(
        &app,
        Method::GET,
        &format!("/api/statistics/top/active?N=-1&{date_query}"),
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    let rows = body["data"].as_array().unwrap();
    let null_name_row = rows.iter().find(|row| row["name"] == "").unwrap();
    assert_eq!(null_name_row["count"], 0);

    let response = send(
        &app,
        Method::GET,
        &format!("/api/statistics/top/login?N=-1&{date_query}"),
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    let rows = body["data"].as_array().unwrap();
    let null_name_row = rows.iter().find(|row| row["name"] == "").unwrap();
    assert_eq!(null_name_row["count"], 0);
}

struct PlayerLogFixture<'a> {
    created_at: &'a str,
    name: &'a str,
    role: &'a str,
    ku_id: &'a str,
    steam_id: &'a str,
    action: &'a str,
    action_desc: &'a str,
    ip: &'a str,
    cluster_name: &'a str,
    deleted_at: Option<&'a str>,
}

async fn seed_statistics(pool: &SqlitePool) {
    for fixture in [
        PlayerLogFixture {
            created_at: "2026-01-01T10:00:00.000Z",
            name: "Alice",
            role: "Wilson",
            ku_id: "KU_ALICE",
            steam_id: "STEAM_ALICE",
            action: "[JoinAnnouncement]",
            action_desc: "Alice joined",
            ip: "10.0.0.1",
            cluster_name: "ClusterStats",
            deleted_at: None,
        },
        PlayerLogFixture {
            created_at: "2026-01-01T11:00:00.000Z",
            name: "Alice",
            role: "Wilson",
            ku_id: "KU_ALICE",
            steam_id: "STEAM_ALICE",
            action: "[JoinAnnouncement]",
            action_desc: "Alice joined again",
            ip: "10.0.0.1",
            cluster_name: "ClusterStats",
            deleted_at: None,
        },
        PlayerLogFixture {
            created_at: "2026-01-01T12:00:00.000Z",
            name: "Bob",
            role: "Wendy",
            ku_id: "KU_BOB",
            steam_id: "STEAM_BOB",
            action: "[JoinAnnouncement]",
            action_desc: "Bob joined",
            ip: "10.0.0.2",
            cluster_name: "ClusterStats",
            deleted_at: None,
        },
        PlayerLogFixture {
            created_at: "2026-01-01T13:00:00.000Z",
            name: "Alice",
            role: "Wilson",
            ku_id: "KU_ALICE",
            steam_id: "STEAM_ALICE",
            action: "[DeathAnnouncement]",
            action_desc: "Alice died",
            ip: "10.0.0.1",
            cluster_name: "ClusterStats",
            deleted_at: None,
        },
        PlayerLogFixture {
            created_at: "2026-01-01T14:00:00.000Z",
            name: "Alice",
            role: "Wilson",
            ku_id: "KU_ALICE",
            steam_id: "STEAM_ALICE",
            action: "[DeathAnnouncement]",
            action_desc: "Alice died again",
            ip: "10.0.0.1",
            cluster_name: "ClusterStats",
            deleted_at: None,
        },
        PlayerLogFixture {
            created_at: "2026-01-01T15:00:00.000Z",
            name: "Bob",
            role: "Wendy",
            ku_id: "KU_BOB",
            steam_id: "STEAM_BOB",
            action: "[DeathAnnouncement]",
            action_desc: "Bob died",
            ip: "10.0.0.2",
            cluster_name: "ClusterStats",
            deleted_at: None,
        },
        PlayerLogFixture {
            created_at: "2026-01-02T10:00:00.000Z",
            name: "Eve",
            role: "Willow",
            ku_id: "KU_EVE",
            steam_id: "STEAM_EVE",
            action: "[JoinAnnouncement]",
            action_desc: "deleted join",
            ip: "10.0.0.3",
            cluster_name: "ClusterStats",
            deleted_at: Some("2026-01-02T11:00:00.000Z"),
        },
    ] {
        insert_player_log(pool, fixture).await;
    }

    insert_connect(
        pool,
        "2026-01-01T09:30:00.000Z",
        "Alice",
        "KU_ALICE",
        "STEAM_ALICE",
    )
    .await;
    insert_connect(
        pool,
        "2026-01-01T09:35:00.000Z",
        "Bob",
        "KU_BOB",
        "STEAM_BOB",
    )
    .await;
    insert_regenerate(pool, "2026-01-03T00:00:00.000Z", "ClusterA", None).await;
    insert_regenerate(pool, "2026-01-04T00:00:00.000Z", "ClusterB", None).await;
    insert_regenerate(
        pool,
        "2026-01-05T00:00:00.000Z",
        "ClusterDeleted",
        Some("2026-01-05T01:00:00.000Z"),
    )
    .await;
}

async fn insert_player_log(pool: &SqlitePool, fixture: PlayerLogFixture<'_>) -> i64 {
    sqlx::query(
        "INSERT INTO player_logs \
         (created_at, updated_at, deleted_at, name, role, ku_id, steam_id, time, action, action_desc, ip, cluster_name) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(fixture.created_at)
    .bind(fixture.created_at)
    .bind(fixture.deleted_at)
    .bind(fixture.name)
    .bind(fixture.role)
    .bind(fixture.ku_id)
    .bind(fixture.steam_id)
    .bind(fixture.created_at)
    .bind(fixture.action)
    .bind(fixture.action_desc)
    .bind(fixture.ip)
    .bind(fixture.cluster_name)
    .execute(pool)
    .await
    .unwrap()
    .last_insert_rowid()
}

async fn insert_null_name_player_log(pool: &SqlitePool, created_at: &str, action: &str) -> i64 {
    sqlx::query(
        "INSERT INTO player_logs \
         (created_at, updated_at, deleted_at, name, role, ku_id, steam_id, time, action, action_desc, ip, cluster_name) \
         VALUES (?, ?, NULL, NULL, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(created_at)
    .bind(created_at)
    .bind("Wilson")
    .bind("KU_NULL")
    .bind("STEAM_NULL")
    .bind(created_at)
    .bind(action)
    .bind("missing name")
    .bind("10.0.0.4")
    .bind("ClusterStats")
    .execute(pool)
    .await
    .unwrap()
    .last_insert_rowid()
}

async fn insert_connect(
    pool: &SqlitePool,
    created_at: &str,
    name: &str,
    ku_id: &str,
    steam_id: &str,
) {
    sqlx::query(
        "INSERT INTO connects \
         (created_at, updated_at, deleted_at, ip, name, ku_id, steam_id, time, cluster_name, session_file) \
         VALUES (?, ?, NULL, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(created_at)
    .bind(created_at)
    .bind("10.0.0.1")
    .bind(name)
    .bind(ku_id)
    .bind(steam_id)
    .bind(created_at)
    .bind("ClusterStats")
    .bind("session")
    .execute(pool)
    .await
    .unwrap();
}

async fn insert_regenerate(
    pool: &SqlitePool,
    created_at: &str,
    cluster_name: &str,
    deleted_at: Option<&str>,
) {
    sqlx::query(
        "INSERT INTO regenerates (created_at, updated_at, deleted_at, cluster_name) VALUES (?, ?, ?, ?)",
    )
    .bind(created_at)
    .bind(created_at)
    .bind(deleted_at)
    .bind(cluster_name)
    .execute(pool)
    .await
    .unwrap();
}

async fn test_router() -> (Router, TempDir, SqlitePool) {
    let dir = tempdir().unwrap();
    write_password_file(dir.path());
    write_dst_config(dir.path(), "ClusterStats");
    let pool = connect_sqlite_memory().await.unwrap();
    migrate(&pool).await.unwrap();
    let state = AppState::new(test_config(), pool.clone(), SessionStore::new(), dir.path());
    (build_router(state), dir, pool)
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
