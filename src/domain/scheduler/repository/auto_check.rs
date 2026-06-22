//! Repository for Go-compatible auto-check settings.

use chrono::Utc;
use sqlx::{QueryBuilder, Sqlite, SqlitePool};

use crate::domain::scheduler::model::{AutoCheckRecord, SaveAutoCheck};

const AUTO_CHECK_COLUMNS: &str = "id, created_at, updated_at, deleted_at, name, cluster_name, level_name, uuid, enable, announcement, times, sleep, interval, check_type";

/// Persistence operations for the `auto_checks` table.
#[derive(Debug, Clone)]
pub struct AutoCheckRepository {
    pool: SqlitePool,
}

impl AutoCheckRepository {
    /// Creates a repository backed by the shared SQLite pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Saves an auto-check row with GORM-like insert-or-update behavior.
    pub async fn save(&self, mut input: SaveAutoCheck) -> Result<AutoCheckRecord, sqlx::Error> {
        if input.uuid.is_empty() {
            input.uuid = format!("UPDATE_GAME_{}", input.cluster_name);
        }

        let now = Utc::now();
        if input.id == 0 {
            let result = sqlx::query(
                "INSERT INTO auto_checks (
                    created_at, updated_at, name, cluster_name, level_name, uuid,
                    enable, announcement, times, sleep, interval, check_type
                 )
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(now)
            .bind(now)
            .bind(input.name)
            .bind(input.cluster_name)
            .bind(input.level_name)
            .bind(input.uuid)
            .bind(input.enable)
            .bind(input.announcement)
            .bind(input.times)
            .bind(input.sleep)
            .bind(input.interval)
            .bind(input.check_type)
            .execute(&self.pool)
            .await?;

            let id = result.last_insert_rowid();
            tracing::info!(id, "inserted auto-check setting");
            return self.get_by_id(id).await;
        }

        let result = sqlx::query(
            "UPDATE auto_checks
             SET name = ?,
                 cluster_name = ?,
                 level_name = ?,
                 uuid = ?,
                 enable = ?,
                 announcement = ?,
                 times = ?,
                 sleep = ?,
                 interval = ?,
                 check_type = ?,
                 updated_at = ?,
                 deleted_at = NULL
             WHERE id = ?",
        )
        .bind(input.name.clone())
        .bind(input.cluster_name.clone())
        .bind(input.level_name.clone())
        .bind(input.uuid.clone())
        .bind(input.enable)
        .bind(input.announcement.clone())
        .bind(input.times)
        .bind(input.sleep)
        .bind(input.interval)
        .bind(input.check_type.clone())
        .bind(now)
        .bind(input.id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            sqlx::query(
                "INSERT INTO auto_checks (
                    id, created_at, updated_at, name, cluster_name, level_name, uuid,
                    enable, announcement, times, sleep, interval, check_type
                 )
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(input.id)
            .bind(now)
            .bind(now)
            .bind(input.name)
            .bind(input.cluster_name)
            .bind(input.level_name)
            .bind(input.uuid)
            .bind(input.enable)
            .bind(input.announcement)
            .bind(input.times)
            .bind(input.sleep)
            .bind(input.interval)
            .bind(input.check_type)
            .execute(&self.pool)
            .await?;
            tracing::info!(
                id = input.id,
                "inserted auto-check setting with explicit id"
            );
        } else {
            tracing::info!(id = input.id, "updated auto-check setting");
        }

        self.get_by_id(input.id).await
    }

    /// Lists persisted auto-check rows by uuid, matching Go's unfiltered query.
    pub async fn list_by_uuids(
        &self,
        uuids: &[String],
    ) -> Result<Vec<AutoCheckRecord>, sqlx::Error> {
        if uuids.is_empty() {
            return Ok(Vec::new());
        }

        let mut builder = QueryBuilder::<Sqlite>::new(format!(
            "SELECT {AUTO_CHECK_COLUMNS}
             FROM auto_checks
             WHERE deleted_at IS NULL AND uuid IN ("
        ));
        {
            let mut separated = builder.separated(", ");
            for uuid in uuids {
                separated.push_bind(uuid);
            }
        }
        builder.push(") ORDER BY id ASC");
        builder
            .build_query_as::<AutoCheckRecord>()
            .fetch_all(&self.pool)
            .await
    }

    /// Lists persisted rows for `UPDATE_GAME`, intentionally without a cluster filter.
    pub async fn list_by_check_type(
        &self,
        check_type: &str,
    ) -> Result<Vec<AutoCheckRecord>, sqlx::Error> {
        sqlx::query_as::<_, AutoCheckRecord>(&format!(
            "SELECT {AUTO_CHECK_COLUMNS}
             FROM auto_checks
             WHERE deleted_at IS NULL AND check_type = ?
             ORDER BY id ASC"
        ))
        .bind(check_type)
        .fetch_all(&self.pool)
        .await
    }

    /// Lists persisted rows for a level-scoped check type and generated uuid set.
    pub async fn list_by_check_type_and_uuids(
        &self,
        check_type: &str,
        uuids: &[String],
    ) -> Result<Vec<AutoCheckRecord>, sqlx::Error> {
        if uuids.is_empty() {
            return Ok(Vec::new());
        }

        let mut builder = QueryBuilder::<Sqlite>::new(format!(
            "SELECT {AUTO_CHECK_COLUMNS}
             FROM auto_checks
             WHERE deleted_at IS NULL AND check_type = "
        ));
        builder.push_bind(check_type);
        builder.push(" AND uuid IN (");
        {
            let mut separated = builder.separated(", ");
            for uuid in uuids {
                separated.push_bind(uuid);
            }
        }
        builder.push(") ORDER BY id ASC");
        builder
            .build_query_as::<AutoCheckRecord>()
            .fetch_all(&self.pool)
            .await
    }

    async fn get_by_id(&self, id: i64) -> Result<AutoCheckRecord, sqlx::Error> {
        sqlx::query_as::<_, AutoCheckRecord>(&format!(
            "SELECT {AUTO_CHECK_COLUMNS}
             FROM auto_checks
             WHERE id = ? AND deleted_at IS NULL"
        ))
        .bind(id)
        .fetch_one(&self.pool)
        .await
    }
}
