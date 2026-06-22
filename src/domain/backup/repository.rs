//! Repository for Go-compatible backup snapshot settings.

use chrono::Utc;
use sqlx::SqlitePool;

use crate::domain::backup::model::{BackupSnapshotRecord, SaveBackupSnapshot};

const BACKUP_SNAPSHOT_COLUMNS: &str =
    "id, created_at, updated_at, deleted_at, name, interval, max_snapshots, enable, is_c_save";

/// Persistence operations for the singleton `backup_snapshots` setting row.
#[derive(Debug, Clone)]
pub struct BackupSnapshotRepository {
    pool: SqlitePool,
}

impl BackupSnapshotRepository {
    /// Creates a repository backed by the shared SQLite pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Returns the first active snapshot setting or Go's zero-value model.
    pub async fn first_or_zero(&self) -> Result<BackupSnapshotRecord, sqlx::Error> {
        let row = sqlx::query_as::<_, BackupSnapshotRecord>(&format!(
            "SELECT {BACKUP_SNAPSHOT_COLUMNS}
             FROM backup_snapshots
             WHERE deleted_at IS NULL
             ORDER BY id ASC
             LIMIT 1"
        ))
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.unwrap_or_else(BackupSnapshotRecord::zero))
    }

    /// Saves the singleton setting, inserting it if Go would have loaded a
    /// zero-value model and called `Save`.
    pub async fn save_singleton(
        &self,
        input: SaveBackupSnapshot,
    ) -> Result<BackupSnapshotRecord, sqlx::Error> {
        let mut connection = self.pool.acquire().await?;
        // Go treats this table as a singleton. `BEGIN IMMEDIATE` takes the
        // SQLite write lock before the first read so two first-save requests
        // cannot both observe an empty table and insert duplicate active rows.
        sqlx::query("BEGIN IMMEDIATE")
            .execute(&mut *connection)
            .await?;
        let result = self.save_singleton_locked(&mut connection, input).await;
        match result {
            Ok(record) => {
                sqlx::query("COMMIT").execute(&mut *connection).await?;
                Ok(record)
            }
            Err(error) => {
                let _ = sqlx::query("ROLLBACK").execute(&mut *connection).await;
                Err(error)
            }
        }
    }

    async fn save_singleton_locked(
        &self,
        connection: &mut sqlx::SqliteConnection,
        input: SaveBackupSnapshot,
    ) -> Result<BackupSnapshotRecord, sqlx::Error> {
        let current = Self::first_or_zero_on(connection).await?;
        let now = Utc::now();
        if current.id == 0 {
            let result = sqlx::query(
                "INSERT INTO backup_snapshots (
                    created_at,
                    updated_at,
                    name,
                    interval,
                    max_snapshots,
                    enable,
                    is_c_save
                 )
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(now)
            .bind(now)
            .bind("")
            .bind(input.interval)
            .bind(input.max_snapshots)
            .bind(input.enable)
            .bind(input.is_c_save)
            .execute(&mut *connection)
            .await?;
            return Self::get_by_id_on(connection, result.last_insert_rowid()).await;
        }

        sqlx::query(
            "UPDATE backup_snapshots
             SET interval = ?,
                 max_snapshots = ?,
                 enable = ?,
                 is_c_save = ?,
                 updated_at = ?
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(input.interval)
        .bind(input.max_snapshots)
        .bind(input.enable)
        .bind(input.is_c_save)
        .bind(now)
        .bind(current.id)
        .execute(&mut *connection)
        .await?;

        Self::get_by_id_on(connection, current.id).await
    }

    async fn first_or_zero_on(
        connection: &mut sqlx::SqliteConnection,
    ) -> Result<BackupSnapshotRecord, sqlx::Error> {
        let row = sqlx::query_as::<_, BackupSnapshotRecord>(&format!(
            "SELECT {BACKUP_SNAPSHOT_COLUMNS}
             FROM backup_snapshots
             WHERE deleted_at IS NULL
             ORDER BY id ASC
             LIMIT 1"
        ))
        .fetch_optional(&mut *connection)
        .await?;
        Ok(row.unwrap_or_else(BackupSnapshotRecord::zero))
    }

    async fn get_by_id_on(
        connection: &mut sqlx::SqliteConnection,
        id: i64,
    ) -> Result<BackupSnapshotRecord, sqlx::Error> {
        sqlx::query_as::<_, BackupSnapshotRecord>(&format!(
            "SELECT {BACKUP_SNAPSHOT_COLUMNS}
             FROM backup_snapshots
             WHERE id = ? AND deleted_at IS NULL"
        ))
        .bind(id)
        .fetch_one(&mut *connection)
        .await
    }
}
