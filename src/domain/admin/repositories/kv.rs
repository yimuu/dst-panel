//! Repository for the Go-compatible `kvs` table.

use chrono::Utc;
use sqlx::SqlitePool;

/// Persistence operations for key/value settings.
#[derive(Debug, Clone)]
pub struct KvRepository {
    pool: SqlitePool,
}

impl KvRepository {
    /// Creates a KV repository backed by a SQLite pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Reads the active value for a key, ignoring GORM soft-deleted rows.
    pub async fn get(&self, key: &str) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar::<_, String>(
            "SELECT value FROM kvs
             WHERE key = ? AND deleted_at IS NULL
             ORDER BY id DESC
             LIMIT 1",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await
    }

    /// Inserts or updates the active value for a key.
    pub async fn save(&self, key: &str, value: &str) -> Result<(), sqlx::Error> {
        if key.trim().is_empty() {
            tracing::warn!("refusing to save kv row with empty key");
            return Err(sqlx::Error::Protocol("kv key must not be empty".to_owned()));
        }

        let active_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM kvs WHERE key = ? AND deleted_at IS NULL",
        )
        .bind(key)
        .fetch_one(&self.pool)
        .await?;
        if active_count > 1 {
            tracing::warn!(
                key,
                active_count,
                "multiple active kv rows found; updating newest row"
            );
        }

        let existing_id = sqlx::query_scalar::<_, i64>(
            "SELECT id FROM kvs
             WHERE key = ? AND deleted_at IS NULL
             ORDER BY id DESC
             LIMIT 1",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(id) = existing_id {
            let now = Utc::now();
            sqlx::query(
                "UPDATE kvs
                 SET value = ?, updated_at = ?
                 WHERE id = ? AND deleted_at IS NULL",
            )
            .bind(value)
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        } else {
            let now = Utc::now();
            sqlx::query(
                "INSERT INTO kvs (created_at, updated_at, key, value)
                 VALUES (?, ?, ?, ?)",
            )
            .bind(now)
            .bind(now)
            .bind(key)
            .bind(value)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }
}
