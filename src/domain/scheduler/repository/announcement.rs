//! Repository for Go-compatible announcement settings.

use chrono::Utc;
use sqlx::SqlitePool;

use crate::domain::scheduler::model::{AnnounceRecord, SaveAnnounce};

const ANNOUNCE_COLUMNS: &str = "id, created_at, updated_at, deleted_at, enable, frequency, interval, interval_unit, method, content";

/// Persistence operations for the `announces` table.
#[derive(Debug, Clone)]
pub struct AnnouncementRepository {
    pool: SqlitePool,
}

impl AnnouncementRepository {
    /// Creates a repository backed by the shared SQLite pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Returns the first active announcement setting or Go's zero-value model.
    pub async fn first_or_zero(&self) -> Result<AnnounceRecord, sqlx::Error> {
        let row = sqlx::query_as::<_, AnnounceRecord>(&format!(
            "SELECT {ANNOUNCE_COLUMNS}
             FROM announces
             WHERE deleted_at IS NULL
             ORDER BY id ASC
             LIMIT 1"
        ))
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.unwrap_or_else(AnnounceRecord::zero))
    }

    /// Saves an announcement row with GORM-like insert-or-update behavior.
    pub async fn save(&self, input: SaveAnnounce) -> Result<AnnounceRecord, sqlx::Error> {
        let now = Utc::now();
        if input.id == 0 {
            let result = sqlx::query(
                "INSERT INTO announces (
                    created_at, updated_at, enable, frequency, interval, interval_unit, method, content
                 )
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(now)
            .bind(now)
            .bind(input.enable)
            .bind(input.frequency)
            .bind(input.interval)
            .bind(input.interval_unit)
            .bind(input.method)
            .bind(input.content)
            .execute(&self.pool)
            .await?;

            let id = result.last_insert_rowid();
            tracing::info!(id, "inserted announcement setting");
            return self.get_by_id(id).await;
        }

        let result = sqlx::query(
            "UPDATE announces
             SET enable = ?,
                 frequency = ?,
                 interval = ?,
                 interval_unit = ?,
                 method = ?,
                 content = ?,
                 updated_at = ?,
                 deleted_at = NULL
             WHERE id = ?",
        )
        .bind(input.enable)
        .bind(input.frequency)
        .bind(input.interval)
        .bind(input.interval_unit.clone())
        .bind(input.method.clone())
        .bind(input.content.clone())
        .bind(now)
        .bind(input.id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            sqlx::query(
                "INSERT INTO announces (
                    id, created_at, updated_at, enable, frequency, interval, interval_unit, method, content
                 )
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(input.id)
            .bind(now)
            .bind(now)
            .bind(input.enable)
            .bind(input.frequency)
            .bind(input.interval)
            .bind(input.interval_unit)
            .bind(input.method)
            .bind(input.content)
            .execute(&self.pool)
            .await?;
            tracing::info!(
                id = input.id,
                "inserted announcement setting with explicit id"
            );
        } else {
            tracing::info!(id = input.id, "updated announcement setting");
        }

        self.get_by_id(input.id).await
    }

    async fn get_by_id(&self, id: i64) -> Result<AnnounceRecord, sqlx::Error> {
        sqlx::query_as::<_, AnnounceRecord>(&format!(
            "SELECT {ANNOUNCE_COLUMNS}
             FROM announces
             WHERE id = ? AND deleted_at IS NULL"
        ))
        .bind(id)
        .fetch_one(&self.pool)
        .await
    }
}
