//! Repository for the Go-compatible `web_links` table.

use chrono::Utc;
use sqlx::SqlitePool;

use crate::domain::admin::model::{NewWebLink, WebLinkRecord};

const WEB_LINK_COLUMNS: &str = "id, created_at, updated_at, deleted_at, title, url, width, height";

/// Persistence operations for UI web links.
#[derive(Debug, Clone)]
pub struct WebLinkRepository {
    pool: SqlitePool,
}

impl WebLinkRepository {
    /// Creates a web-link repository backed by a SQLite pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Lists active web links, ignoring GORM soft-deleted rows.
    pub async fn list(&self) -> Result<Vec<WebLinkRecord>, sqlx::Error> {
        sqlx::query_as::<_, WebLinkRecord>(&format!(
            "SELECT {WEB_LINK_COLUMNS}
             FROM web_links
             WHERE deleted_at IS NULL
             ORDER BY id ASC"
        ))
        .fetch_all(&self.pool)
        .await
    }

    /// Adds a web link and returns the stored row.
    pub async fn add(&self, link: NewWebLink) -> Result<WebLinkRecord, sqlx::Error> {
        if link.title.trim().is_empty() || link.url.trim().is_empty() {
            tracing::warn!(
                title = %link.title,
                "refusing to add web link with empty title or url"
            );
            return Err(sqlx::Error::Protocol(
                "web link title and url must not be empty".to_owned(),
            ));
        }

        let now = Utc::now();
        let result = sqlx::query(
            "INSERT INTO web_links (created_at, updated_at, title, url, width, height)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(now)
        .bind(now)
        .bind(link.title)
        .bind(link.url)
        .bind(link.width)
        .bind(link.height)
        .execute(&self.pool)
        .await?;

        self.get_by_id(result.last_insert_rowid()).await
    }

    /// Soft-deletes a web link by setting `deleted_at`.
    pub async fn delete(&self, id: i64) -> Result<bool, sqlx::Error> {
        let now = Utc::now();
        let result = sqlx::query(
            "UPDATE web_links
             SET deleted_at = ?, updated_at = ?
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(now)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        let deleted = result.rows_affected() > 0;
        if !deleted {
            tracing::warn!(id, "web link delete matched no active row");
        }
        Ok(deleted)
    }

    async fn get_by_id(&self, id: i64) -> Result<WebLinkRecord, sqlx::Error> {
        sqlx::query_as::<_, WebLinkRecord>(&format!(
            "SELECT {WEB_LINK_COLUMNS}
             FROM web_links
             WHERE id = ? AND deleted_at IS NULL"
        ))
        .bind(id)
        .fetch_one(&self.pool)
        .await
    }
}
