use dst_admin_rust::{
    domain::{
        admin::{
            model::NewWebLink,
            repositories::{kv::KvRepository, web_link::WebLinkRepository},
        },
        cluster::{
            model::{ClusterRecord, NewCluster},
            repository::ClusterRepository,
        },
    },
    infra::db,
};
use sqlx::Row;
use tempfile::tempdir;

#[tokio::test]
async fn migration_creates_go_compatible_tables() {
    let pool = db::connect_sqlite_memory().await.unwrap();
    db::migrate(&pool).await.unwrap();

    for table in [
        "clusters",
        "kvs",
        "web_links",
        "backup_snapshots",
        "auto_checks",
        "job_tasks",
        "mod_infos",
        "player_logs",
        "spawns",
        "connects",
        "regenerates",
        "announces",
        "log_records",
    ] {
        assert!(
            db::table_exists(&pool, table).await.unwrap(),
            "{table} missing"
        );
        for column in ["id", "created_at", "updated_at", "deleted_at"] {
            assert!(
                db::column_exists(&pool, table, column).await.unwrap(),
                "{table}.{column} missing"
            );
        }
    }
}

#[tokio::test]
async fn connect_sqlite_creates_missing_database_file_like_gorm() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("dst-db");
    assert!(!db_path.exists());

    let pool = db::connect_sqlite(db_path.to_str().unwrap()).await.unwrap();
    db::migrate(&pool).await.unwrap();

    assert!(db_path.exists());
    assert!(db::table_exists(&pool, "clusters").await.unwrap());
}

#[tokio::test]
async fn connect_sqlite_creates_missing_database_parent_directory_like_gorm() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("data").join("dst-db");
    assert!(!db_path.parent().unwrap().exists());

    let pool = db::connect_sqlite(db_path.to_str().unwrap()).await.unwrap();
    db::migrate(&pool).await.unwrap();

    assert!(db_path.exists());
    assert!(db::table_exists(&pool, "clusters").await.unwrap());
}

#[tokio::test]
async fn migration_preserves_expected_columns_and_indexes() {
    let pool = db::connect_sqlite_memory().await.unwrap();
    db::migrate(&pool).await.unwrap();

    assert_columns(
        &pool,
        "mod_infos",
        &[
            "auth",
            "consumer_appid",
            "creator_appid",
            "description",
            "file_url",
            "modid",
            "img",
            "last_time",
            "mod_config",
            "name",
            "v",
            "update",
        ],
    )
    .await;
    assert_columns(
        &pool,
        "clusters",
        &[
            "cluster_name",
            "description",
            "steam_cmd",
            "force_install_dir",
            "backup",
            "mod_download_path",
            "uuid",
            "beta",
            "bin",
            "ugc_directory",
            "persistent_storage_root",
            "conf_dir",
        ],
    )
    .await;

    assert!(unique_index_exists(&pool, "clusters", "idx_clusters_cluster_name").await);
    assert!(index_exists(&pool, "mod_infos", "idx_mod_infos_deleted_at").await);
}

#[tokio::test]
async fn migration_adds_missing_columns_to_existing_go_tables_without_losing_rows() {
    let pool = db::connect_sqlite_memory().await.unwrap();
    sqlx::query(
        "CREATE TABLE clusters (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cluster_name TEXT
        )",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO clusters (cluster_name) VALUES (?)")
        .bind("legacy-cluster")
        .execute(&pool)
        .await
        .unwrap();

    db::migrate(&pool).await.unwrap();

    assert_eq!(count_rows(&pool, "clusters").await, 1);
    assert_columns(
        &pool,
        "clusters",
        &[
            "created_at",
            "updated_at",
            "deleted_at",
            "description",
            "steam_cmd",
            "force_install_dir",
            "backup",
            "mod_download_path",
            "uuid",
            "beta",
            "bin",
            "ugc_directory",
            "persistent_storage_root",
            "conf_dir",
        ],
    )
    .await;
    assert!(unique_index_exists(&pool, "clusters", "idx_clusters_cluster_name").await);
}

#[tokio::test]
async fn migration_repairs_every_known_legacy_go_table_before_creating_indexes() {
    let pool = db::connect_sqlite_memory().await.unwrap();

    for schema in legacy_additive_schemas() {
        sqlx::query(&format!(
            "CREATE TABLE {} (id INTEGER PRIMARY KEY AUTOINCREMENT)",
            schema.table
        ))
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(&format!("INSERT INTO {} DEFAULT VALUES", schema.table))
            .execute(&pool)
            .await
            .unwrap();
    }

    db::migrate(&pool).await.unwrap();

    for schema in legacy_additive_schemas() {
        assert_eq!(
            count_rows(&pool, schema.table).await,
            1,
            "{} legacy row was lost during additive migration",
            schema.table
        );
        assert_columns(&pool, schema.table, schema.columns).await;
    }
    assert!(unique_index_exists(&pool, "clusters", "idx_clusters_cluster_name").await);
    assert!(index_exists(&pool, "kvs", "idx_kvs_deleted_at").await);
    assert!(index_exists(&pool, "player_logs", "idx_player_logs_deleted_at").await);
}

#[tokio::test]
async fn kv_repository_saves_updates_and_ignores_soft_deleted_rows() {
    let pool = db::connect_sqlite_memory().await.unwrap();
    db::migrate(&pool).await.unwrap();
    let repo = KvRepository::new(pool.clone());

    assert_eq!(repo.get("theme").await.unwrap(), None);

    repo.save("theme", "dark").await.unwrap();
    assert_eq!(repo.get("theme").await.unwrap().as_deref(), Some("dark"));

    repo.save("theme", "light").await.unwrap();
    assert_eq!(repo.get("theme").await.unwrap().as_deref(), Some("light"));
    assert_eq!(count_rows(&pool, "kvs").await, 1);

    sqlx::query("UPDATE kvs SET deleted_at = CURRENT_TIMESTAMP WHERE key = ?")
        .bind("theme")
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(repo.get("theme").await.unwrap(), None);
    assert_eq!(count_rows(&pool, "kvs").await, 1);
    assert_eq!(count_soft_deleted_rows(&pool, "kvs").await, 1);
}

#[tokio::test]
async fn web_link_repository_crud_uses_soft_delete() {
    let pool = db::connect_sqlite_memory().await.unwrap();
    db::migrate(&pool).await.unwrap();
    let repo = WebLinkRepository::new(pool.clone());

    let created = repo
        .add(NewWebLink {
            title: "Console".to_owned(),
            url: "https://example.test/console".to_owned(),
            width: "1024".to_owned(),
            height: "768".to_owned(),
        })
        .await
        .unwrap();

    assert!(created.id > 0);
    assert_rfc3339_json_timestamp(&created, "CreatedAt");
    assert_rfc3339_json_timestamp(&created, "UpdatedAt");
    let links = repo.list().await.unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].title, "Console");

    assert!(repo.delete(created.id).await.unwrap());

    assert!(repo.list().await.unwrap().is_empty());
    assert_eq!(count_rows(&pool, "web_links").await, 1);
    assert_eq!(count_soft_deleted_rows(&pool, "web_links").await, 1);
}

#[tokio::test]
async fn cluster_repository_crud_uses_unique_names_and_soft_delete() {
    let pool = db::connect_sqlite_memory().await.unwrap();
    db::migrate(&pool).await.unwrap();
    let repo = ClusterRepository::new(pool.clone());

    let created = repo.create(new_cluster("Cluster1")).await.unwrap();
    assert!(created.id > 0);
    assert_eq!(created.cluster_name, "Cluster1");
    assert_rfc3339_json_timestamp(&created, "CreatedAt");
    assert_rfc3339_json_timestamp(&created, "UpdatedAt");

    let duplicate = repo.create(new_cluster("Cluster1")).await;
    assert!(duplicate.is_err(), "cluster_name must be unique");

    let clusters = repo.list().await.unwrap();
    assert_eq!(clusters.len(), 1);

    let mut updated: ClusterRecord = created;
    updated.description = "Updated description".to_owned();
    updated.bin = 64;
    let updated = repo.update(updated).await.unwrap();
    assert_eq!(updated.description, "Updated description");
    assert_eq!(updated.bin, 64);

    assert!(repo.delete(updated.id).await.unwrap());

    assert!(repo.list().await.unwrap().is_empty());
    assert_eq!(count_rows(&pool, "clusters").await, 1);
    assert_eq!(count_soft_deleted_rows(&pool, "clusters").await, 1);
}

fn new_cluster(name: &str) -> NewCluster {
    NewCluster {
        cluster_name: name.to_owned(),
        description: "Primary test cluster".to_owned(),
        steam_cmd: "/opt/steamcmd".to_owned(),
        force_install_dir: "/opt/dst".to_owned(),
        backup: "/opt/backups".to_owned(),
        mod_download_path: "/opt/mods".to_owned(),
        uuid: format!("{name}-uuid"),
        beta: 0,
        bin: 32,
        ugc_directory: "/opt/ugc".to_owned(),
        persistent_storage_root: "/opt/klei".to_owned(),
        conf_dir: "DoNotStarveTogether".to_owned(),
    }
}

struct LegacySchema {
    table: &'static str,
    columns: &'static [&'static str],
}

fn legacy_additive_schemas() -> &'static [LegacySchema] {
    &[
        LegacySchema {
            table: "spawns",
            columns: &[
                "created_at",
                "updated_at",
                "deleted_at",
                "name",
                "role",
                "time",
                "cluster_name",
            ],
        },
        LegacySchema {
            table: "player_logs",
            columns: &[
                "created_at",
                "updated_at",
                "deleted_at",
                "name",
                "role",
                "ku_id",
                "steam_id",
                "time",
                "action",
                "action_desc",
                "ip",
                "cluster_name",
            ],
        },
        LegacySchema {
            table: "connects",
            columns: &[
                "created_at",
                "updated_at",
                "deleted_at",
                "ip",
                "name",
                "ku_id",
                "steam_id",
                "time",
                "cluster_name",
                "session_file",
            ],
        },
        LegacySchema {
            table: "regenerates",
            columns: &["created_at", "updated_at", "deleted_at", "cluster_name"],
        },
        LegacySchema {
            table: "mod_infos",
            columns: &[
                "created_at",
                "updated_at",
                "deleted_at",
                "auth",
                "consumer_appid",
                "creator_appid",
                "description",
                "file_url",
                "modid",
                "img",
                "last_time",
                "mod_config",
                "name",
                "v",
                "update",
            ],
        },
        LegacySchema {
            table: "clusters",
            columns: &[
                "created_at",
                "updated_at",
                "deleted_at",
                "cluster_name",
                "description",
                "steam_cmd",
                "force_install_dir",
                "backup",
                "mod_download_path",
                "uuid",
                "beta",
                "bin",
                "ugc_directory",
                "persistent_storage_root",
                "conf_dir",
            ],
        },
        LegacySchema {
            table: "job_tasks",
            columns: &[
                "created_at",
                "updated_at",
                "deleted_at",
                "cluster_name",
                "level_name",
                "uuid",
                "cron",
                "category",
                "comment",
                "announcement",
                "sleep",
                "times",
                "script",
            ],
        },
        LegacySchema {
            table: "auto_checks",
            columns: &[
                "created_at",
                "updated_at",
                "deleted_at",
                "name",
                "cluster_name",
                "level_name",
                "uuid",
                "enable",
                "announcement",
                "times",
                "sleep",
                "interval",
                "check_type",
            ],
        },
        LegacySchema {
            table: "announces",
            columns: &[
                "created_at",
                "updated_at",
                "deleted_at",
                "enable",
                "frequency",
                "interval",
                "interval_unit",
                "method",
                "content",
            ],
        },
        LegacySchema {
            table: "web_links",
            columns: &[
                "created_at",
                "updated_at",
                "deleted_at",
                "title",
                "url",
                "width",
                "height",
            ],
        },
        LegacySchema {
            table: "backup_snapshots",
            columns: &[
                "created_at",
                "updated_at",
                "deleted_at",
                "name",
                "interval",
                "max_snapshots",
                "enable",
                "is_c_save",
            ],
        },
        LegacySchema {
            table: "log_records",
            columns: &[
                "created_at",
                "updated_at",
                "deleted_at",
                "action",
                "cluster_name",
                "level_name",
            ],
        },
        LegacySchema {
            table: "kvs",
            columns: &["created_at", "updated_at", "deleted_at", "key", "value"],
        },
    ]
}

async fn count_rows(pool: &sqlx::SqlitePool, table: &str) -> i64 {
    let sql = format!("SELECT COUNT(*) FROM {table}");
    sqlx::query_scalar::<_, i64>(&sql)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn count_soft_deleted_rows(pool: &sqlx::SqlitePool, table: &str) -> i64 {
    let sql = format!("SELECT COUNT(*) FROM {table} WHERE deleted_at IS NOT NULL");
    sqlx::query_scalar::<_, i64>(&sql)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn assert_columns(pool: &sqlx::SqlitePool, table: &str, columns: &[&str]) {
    for column in columns {
        assert!(
            db::column_exists(pool, table, column).await.unwrap(),
            "{table}.{column} missing"
        );
    }
}

async fn index_exists(pool: &sqlx::SqlitePool, table: &str, index_name: &str) -> bool {
    sqlx::query(&format!("PRAGMA index_list({table})"))
        .fetch_all(pool)
        .await
        .unwrap()
        .iter()
        .any(|row| row.get::<String, _>("name") == index_name)
}

async fn unique_index_exists(pool: &sqlx::SqlitePool, table: &str, index_name: &str) -> bool {
    sqlx::query(&format!("PRAGMA index_list({table})"))
        .fetch_all(pool)
        .await
        .unwrap()
        .iter()
        .any(|row| row.get::<String, _>("name") == index_name && row.get::<i64, _>("unique") == 1)
}

fn assert_rfc3339_json_timestamp<T: serde::Serialize>(value: &T, field: &str) {
    let json = serde_json::to_value(value).unwrap();
    let timestamp = json[field]
        .as_str()
        .expect("timestamp should serialize as string");

    chrono::DateTime::parse_from_rfc3339(timestamp)
        .unwrap_or_else(|_| panic!("{field} is not RFC3339: {timestamp}"));
    assert!(
        timestamp.contains('T'),
        "{field} should use JSON/RFC3339 separator: {timestamp}"
    );
}
