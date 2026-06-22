//! SQLite connection and schema migration support.
//!
//! The migration SQL intentionally mirrors the existing Go/GORM table names
//! and soft-delete columns so the Rust backend can open existing databases.

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    str::FromStr,
};

use sqlx::{
    Row, Sqlite, Transaction,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use tracing::info;

pub use sqlx::SqlitePool;

macro_rules! common_columns {
    () => {
        "
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        created_at DATETIME,
        updated_at DATETIME,
        deleted_at DATETIME"
    };
}

const MIGRATIONS: &[&str] = &[
    concat!(
        "CREATE TABLE IF NOT EXISTS spawns (",
        common_columns!(),
        ",
        name TEXT,
        role TEXT,
        time TEXT,
        cluster_name TEXT
    )"
    ),
    "CREATE INDEX IF NOT EXISTS idx_spawns_deleted_at ON spawns(deleted_at)",
    concat!(
        "CREATE TABLE IF NOT EXISTS player_logs (",
        common_columns!(),
        ",
        name TEXT,
        role TEXT,
        ku_id TEXT,
        steam_id TEXT,
        time TEXT,
        action TEXT,
        action_desc TEXT,
        ip TEXT,
        cluster_name TEXT
    )"
    ),
    "CREATE INDEX IF NOT EXISTS idx_player_logs_deleted_at ON player_logs(deleted_at)",
    concat!(
        "CREATE TABLE IF NOT EXISTS connects (",
        common_columns!(),
        ",
        ip TEXT,
        name TEXT,
        ku_id TEXT,
        steam_id TEXT,
        time TEXT,
        cluster_name TEXT,
        session_file TEXT
    )"
    ),
    "CREATE INDEX IF NOT EXISTS idx_connects_deleted_at ON connects(deleted_at)",
    concat!(
        "CREATE TABLE IF NOT EXISTS regenerates (",
        common_columns!(),
        ",
        cluster_name TEXT
    )"
    ),
    "CREATE INDEX IF NOT EXISTS idx_regenerates_deleted_at ON regenerates(deleted_at)",
    concat!(
        "CREATE TABLE IF NOT EXISTS mod_infos (",
        common_columns!(),
        ",
        auth TEXT,
        consumer_appid REAL,
        creator_appid REAL,
        description TEXT,
        file_url TEXT,
        modid TEXT,
        img TEXT,
        last_time REAL,
        mod_config TEXT,
        name TEXT,
        v TEXT,
        \"update\" INTEGER
    )"
    ),
    "CREATE INDEX IF NOT EXISTS idx_mod_infos_deleted_at ON mod_infos(deleted_at)",
    concat!(
        "CREATE TABLE IF NOT EXISTS clusters (",
        common_columns!(),
        ",
        cluster_name TEXT,
        description TEXT,
        steam_cmd TEXT,
        force_install_dir TEXT,
        backup TEXT,
        mod_download_path TEXT,
        uuid TEXT,
        beta INTEGER,
        bin INTEGER,
        ugc_directory TEXT,
        persistent_storage_root TEXT,
        conf_dir TEXT
    )"
    ),
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_clusters_cluster_name ON clusters(cluster_name)",
    "CREATE INDEX IF NOT EXISTS idx_clusters_deleted_at ON clusters(deleted_at)",
    concat!(
        "CREATE TABLE IF NOT EXISTS job_tasks (",
        common_columns!(),
        ",
        cluster_name TEXT,
        level_name TEXT,
        uuid TEXT,
        cron TEXT,
        category TEXT,
        comment TEXT,
        announcement TEXT,
        sleep INTEGER,
        times INTEGER,
        script INTEGER
    )"
    ),
    "CREATE INDEX IF NOT EXISTS idx_job_tasks_deleted_at ON job_tasks(deleted_at)",
    concat!(
        "CREATE TABLE IF NOT EXISTS auto_checks (",
        common_columns!(),
        ",
        name TEXT,
        cluster_name TEXT,
        level_name TEXT,
        uuid TEXT,
        enable INTEGER,
        announcement TEXT,
        times INTEGER,
        sleep INTEGER,
        interval INTEGER,
        check_type TEXT
    )"
    ),
    "CREATE INDEX IF NOT EXISTS idx_auto_checks_deleted_at ON auto_checks(deleted_at)",
    concat!(
        "CREATE TABLE IF NOT EXISTS announces (",
        common_columns!(),
        ",
        enable INTEGER,
        frequency INTEGER,
        interval INTEGER,
        interval_unit TEXT,
        method TEXT,
        content TEXT
    )"
    ),
    "CREATE INDEX IF NOT EXISTS idx_announces_deleted_at ON announces(deleted_at)",
    concat!(
        "CREATE TABLE IF NOT EXISTS web_links (",
        common_columns!(),
        ",
        title TEXT,
        url TEXT,
        width TEXT,
        height TEXT
    )"
    ),
    "CREATE INDEX IF NOT EXISTS idx_web_links_deleted_at ON web_links(deleted_at)",
    concat!(
        "CREATE TABLE IF NOT EXISTS backup_snapshots (",
        common_columns!(),
        ",
        name TEXT,
        interval INTEGER,
        max_snapshots INTEGER,
        enable INTEGER,
        is_c_save INTEGER
    )"
    ),
    "CREATE INDEX IF NOT EXISTS idx_backup_snapshots_deleted_at ON backup_snapshots(deleted_at)",
    concat!(
        "CREATE TABLE IF NOT EXISTS log_records (",
        common_columns!(),
        ",
        action INTEGER,
        cluster_name TEXT,
        level_name TEXT
    )"
    ),
    "CREATE INDEX IF NOT EXISTS idx_log_records_deleted_at ON log_records(deleted_at)",
    concat!(
        "CREATE TABLE IF NOT EXISTS kvs (",
        common_columns!(),
        ",
        key TEXT,
        value TEXT
    )"
    ),
    "CREATE INDEX IF NOT EXISTS idx_kvs_deleted_at ON kvs(deleted_at)",
];

struct TableSchema {
    table: &'static str,
    columns: &'static [ColumnSchema],
}

struct ColumnSchema {
    name: &'static str,
    definition: &'static str,
}

macro_rules! columns {
    ($($name:literal => $definition:literal),+ $(,)?) => {
        &[
            $(
                ColumnSchema {
                    name: $name,
                    definition: $definition,
                },
            )+
        ]
    };
}

const ADDITIVE_TABLE_SCHEMAS: &[TableSchema] = &[
    TableSchema {
        table: "spawns",
        columns: columns![
            "created_at" => "created_at DATETIME",
            "updated_at" => "updated_at DATETIME",
            "deleted_at" => "deleted_at DATETIME",
            "name" => "name TEXT",
            "role" => "role TEXT",
            "time" => "time TEXT",
            "cluster_name" => "cluster_name TEXT",
        ],
    },
    TableSchema {
        table: "player_logs",
        columns: columns![
            "created_at" => "created_at DATETIME",
            "updated_at" => "updated_at DATETIME",
            "deleted_at" => "deleted_at DATETIME",
            "name" => "name TEXT",
            "role" => "role TEXT",
            "ku_id" => "ku_id TEXT",
            "steam_id" => "steam_id TEXT",
            "time" => "time TEXT",
            "action" => "action TEXT",
            "action_desc" => "action_desc TEXT",
            "ip" => "ip TEXT",
            "cluster_name" => "cluster_name TEXT",
        ],
    },
    TableSchema {
        table: "connects",
        columns: columns![
            "created_at" => "created_at DATETIME",
            "updated_at" => "updated_at DATETIME",
            "deleted_at" => "deleted_at DATETIME",
            "ip" => "ip TEXT",
            "name" => "name TEXT",
            "ku_id" => "ku_id TEXT",
            "steam_id" => "steam_id TEXT",
            "time" => "time TEXT",
            "cluster_name" => "cluster_name TEXT",
            "session_file" => "session_file TEXT",
        ],
    },
    TableSchema {
        table: "regenerates",
        columns: columns![
            "created_at" => "created_at DATETIME",
            "updated_at" => "updated_at DATETIME",
            "deleted_at" => "deleted_at DATETIME",
            "cluster_name" => "cluster_name TEXT",
        ],
    },
    TableSchema {
        table: "mod_infos",
        columns: columns![
            "created_at" => "created_at DATETIME",
            "updated_at" => "updated_at DATETIME",
            "deleted_at" => "deleted_at DATETIME",
            "auth" => "auth TEXT",
            "consumer_appid" => "consumer_appid REAL",
            "creator_appid" => "creator_appid REAL",
            "description" => "description TEXT",
            "file_url" => "file_url TEXT",
            "modid" => "modid TEXT",
            "img" => "img TEXT",
            "last_time" => "last_time REAL",
            "mod_config" => "mod_config TEXT",
            "name" => "name TEXT",
            "v" => "v TEXT",
            "update" => "\"update\" INTEGER",
        ],
    },
    TableSchema {
        table: "clusters",
        columns: columns![
            "created_at" => "created_at DATETIME",
            "updated_at" => "updated_at DATETIME",
            "deleted_at" => "deleted_at DATETIME",
            "cluster_name" => "cluster_name TEXT",
            "description" => "description TEXT",
            "steam_cmd" => "steam_cmd TEXT",
            "force_install_dir" => "force_install_dir TEXT",
            "backup" => "backup TEXT",
            "mod_download_path" => "mod_download_path TEXT",
            "uuid" => "uuid TEXT",
            "beta" => "beta INTEGER",
            "bin" => "bin INTEGER",
            "ugc_directory" => "ugc_directory TEXT",
            "persistent_storage_root" => "persistent_storage_root TEXT",
            "conf_dir" => "conf_dir TEXT",
        ],
    },
    TableSchema {
        table: "job_tasks",
        columns: columns![
            "created_at" => "created_at DATETIME",
            "updated_at" => "updated_at DATETIME",
            "deleted_at" => "deleted_at DATETIME",
            "cluster_name" => "cluster_name TEXT",
            "level_name" => "level_name TEXT",
            "uuid" => "uuid TEXT",
            "cron" => "cron TEXT",
            "category" => "category TEXT",
            "comment" => "comment TEXT",
            "announcement" => "announcement TEXT",
            "sleep" => "sleep INTEGER",
            "times" => "times INTEGER",
            "script" => "script INTEGER",
        ],
    },
    TableSchema {
        table: "auto_checks",
        columns: columns![
            "created_at" => "created_at DATETIME",
            "updated_at" => "updated_at DATETIME",
            "deleted_at" => "deleted_at DATETIME",
            "name" => "name TEXT",
            "cluster_name" => "cluster_name TEXT",
            "level_name" => "level_name TEXT",
            "uuid" => "uuid TEXT",
            "enable" => "enable INTEGER",
            "announcement" => "announcement TEXT",
            "times" => "times INTEGER",
            "sleep" => "sleep INTEGER",
            "interval" => "interval INTEGER",
            "check_type" => "check_type TEXT",
        ],
    },
    TableSchema {
        table: "announces",
        columns: columns![
            "created_at" => "created_at DATETIME",
            "updated_at" => "updated_at DATETIME",
            "deleted_at" => "deleted_at DATETIME",
            "enable" => "enable INTEGER",
            "frequency" => "frequency INTEGER",
            "interval" => "interval INTEGER",
            "interval_unit" => "interval_unit TEXT",
            "method" => "method TEXT",
            "content" => "content TEXT",
        ],
    },
    TableSchema {
        table: "web_links",
        columns: columns![
            "created_at" => "created_at DATETIME",
            "updated_at" => "updated_at DATETIME",
            "deleted_at" => "deleted_at DATETIME",
            "title" => "title TEXT",
            "url" => "url TEXT",
            "width" => "width TEXT",
            "height" => "height TEXT",
        ],
    },
    TableSchema {
        table: "backup_snapshots",
        columns: columns![
            "created_at" => "created_at DATETIME",
            "updated_at" => "updated_at DATETIME",
            "deleted_at" => "deleted_at DATETIME",
            "name" => "name TEXT",
            "interval" => "interval INTEGER",
            "max_snapshots" => "max_snapshots INTEGER",
            "enable" => "enable INTEGER",
            "is_c_save" => "is_c_save INTEGER",
        ],
    },
    TableSchema {
        table: "log_records",
        columns: columns![
            "created_at" => "created_at DATETIME",
            "updated_at" => "updated_at DATETIME",
            "deleted_at" => "deleted_at DATETIME",
            "action" => "action INTEGER",
            "cluster_name" => "cluster_name TEXT",
            "level_name" => "level_name TEXT",
        ],
    },
    TableSchema {
        table: "kvs",
        columns: columns![
            "created_at" => "created_at DATETIME",
            "updated_at" => "updated_at DATETIME",
            "deleted_at" => "deleted_at DATETIME",
            "key" => "key TEXT",
            "value" => "value TEXT",
        ],
    },
];

/// Opens a SQLite connection pool for the provided database URL.
pub async fn connect_sqlite(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    ensure_sqlite_parent_dir(database_url)?;
    let options = sqlite_connect_options(database_url)?.create_if_missing(true);
    info!(database = %database_url, "opening sqlite database");
    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
}

/// Opens a single-connection in-memory SQLite pool for tests.
///
/// SQLite in-memory databases are per connection, so this pool is restricted
/// to one connection to keep schema and repository calls on the same database.
pub async fn connect_sqlite_memory() -> Result<SqlitePool, sqlx::Error> {
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
}

/// Creates all Go/GORM-compatible tables and indexes, adding missing columns
/// for existing older tables before indexes are created.
pub async fn migrate(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let table_statement_count = MIGRATIONS
        .iter()
        .filter(|statement| is_create_table_statement(statement))
        .count();
    let index_statement_count = MIGRATIONS.len() - table_statement_count;
    info!(
        statement_count = MIGRATIONS.len(),
        table_statement_count, index_statement_count, "starting sqlite schema migration"
    );
    let mut transaction = pool.begin().await?;
    for statement in MIGRATIONS {
        if is_create_table_statement(statement) {
            sqlx::query(statement).execute(&mut *transaction).await?;
        }
    }

    add_missing_columns(&mut transaction).await?;

    for statement in MIGRATIONS {
        if !is_create_table_statement(statement) {
            sqlx::query(statement).execute(&mut *transaction).await?;
        }
    }
    transaction.commit().await?;
    info!("completed sqlite schema migration");
    Ok(())
}

/// Returns whether a SQLite table exists.
pub async fn table_exists(pool: &SqlitePool, table: &str) -> Result<bool, sqlx::Error> {
    let exists = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?",
    )
    .bind(table)
    .fetch_one(pool)
    .await?;

    Ok(exists > 0)
}

/// Returns whether a SQLite table contains a column.
pub async fn column_exists(
    pool: &SqlitePool,
    table: &str,
    column: &str,
) -> Result<bool, sqlx::Error> {
    if !valid_identifier(table) {
        tracing::warn!(table, "invalid table identifier for column lookup");
        return Ok(false);
    }

    let sql = format!("PRAGMA table_info({table})");
    let rows = sqlx::query(&sql).fetch_all(pool).await?;
    Ok(rows
        .iter()
        .any(|row| row.get::<String, _>("name") == column))
}

fn valid_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(first) if first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

async fn add_missing_columns(transaction: &mut Transaction<'_, Sqlite>) -> Result<(), sqlx::Error> {
    for schema in ADDITIVE_TABLE_SCHEMAS {
        let mut existing_columns = existing_columns(transaction, schema.table).await?;
        for column in schema.columns {
            if existing_columns.contains(column.name) {
                continue;
            }

            let sql = format!(
                "ALTER TABLE {} ADD COLUMN {}",
                schema.table, column.definition
            );
            sqlx::query(&sql).execute(&mut **transaction).await?;
            existing_columns.insert(column.name.to_owned());
            tracing::info!(
                table = schema.table,
                column = column.name,
                "added missing sqlite column"
            );
        }
    }

    Ok(())
}

async fn existing_columns(
    transaction: &mut Transaction<'_, Sqlite>,
    table: &str,
) -> Result<HashSet<String>, sqlx::Error> {
    let sql = format!("PRAGMA table_info({table})");
    let rows = sqlx::query(&sql).fetch_all(&mut **transaction).await?;
    Ok(rows
        .into_iter()
        .map(|row| row.get::<String, _>("name"))
        .collect())
}

fn is_create_table_statement(statement: &str) -> bool {
    statement.trim_start().starts_with("CREATE TABLE")
}

fn sqlite_connect_options(database_url: &str) -> Result<SqliteConnectOptions, sqlx::Error> {
    if database_url.starts_with("sqlite:") {
        // Preserve explicit SQLx-style URLs while still enabling fresh-install
        // creation behavior to match Go's sqlite.Open("dst-db").
        SqliteConnectOptions::from_str(database_url)
    } else {
        Ok(SqliteConnectOptions::new().filename(database_url))
    }
}

fn ensure_sqlite_parent_dir(database_url: &str) -> Result<(), sqlx::Error> {
    let Some(database_path) = sqlite_database_path(database_url) else {
        return Ok(());
    };
    let Some(parent) = database_path.parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }

    std::fs::create_dir_all(parent).map_err(sqlx::Error::Io)?;
    info!(
        database = %database_url,
        directory = %parent.display(),
        "ensured sqlite database directory"
    );
    Ok(())
}

fn sqlite_database_path(database_url: &str) -> Option<PathBuf> {
    if database_url == ":memory:" || database_url.starts_with("sqlite::memory:") {
        return None;
    }

    let path = if let Some(path) = database_url.strip_prefix("sqlite://") {
        path
    } else if let Some(path) = database_url.strip_prefix("sqlite:") {
        path
    } else {
        database_url
    };
    let path = path.split_once('?').map_or(path, |(path, _)| path);
    if path.is_empty() || path == ":memory:" {
        return None;
    }
    Some(Path::new(path).to_path_buf())
}
