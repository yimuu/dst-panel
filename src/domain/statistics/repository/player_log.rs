//! Repository for Go-compatible player log listing and deletion.

use chrono::Utc;
use sqlx::{QueryBuilder, Sqlite};

use crate::{domain::statistics::model::PlayerLogRecord, infra::db::SqlitePool};

/// Query parameters supported by Go's `/api/player/log` route.
#[derive(Debug, Clone, Default)]
pub struct PlayerLogFilter {
    pub name: Option<String>,
    pub ku_id: Option<String>,
    pub steam_id: Option<String>,
    pub role: Option<String>,
    pub action: Option<String>,
    pub ip: Option<String>,
}

/// Page of player log records in the Go `vo.Page` shape.
#[derive(Debug, Clone)]
pub struct PlayerLogPage {
    pub data: Vec<PlayerLogRecord>,
    pub total: i64,
    pub total_pages: i64,
    pub page: i64,
    pub size: i64,
}

/// SQLite repository for `player_logs`.
pub struct PlayerLogRepository {
    pool: SqlitePool,
}

impl PlayerLogRepository {
    /// Creates a repository backed by the shared SQLite pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Lists active player logs, preserving Go's filters, order, and pagination.
    pub async fn list_page(
        &self,
        filter: &PlayerLogFilter,
        page: i64,
        size: i64,
    ) -> Result<PlayerLogPage, sqlx::Error> {
        let page = page.max(1);
        let size = normalize_page_size(size);
        if size == 0 {
            return Err(sqlx::Error::Protocol(
                "size must be greater than zero".into(),
            ));
        }
        if filter.steam_id.is_some() {
            // Go's handler writes the raw predicate `steamId LIKE ?` against a
            // GORM-created `steam_id` column. SQLite rejects that query, the Go
            // code ignores the DB error, and the endpoint returns an empty page.
            return Ok(PlayerLogPage {
                data: Vec::new(),
                total: 0,
                total_pages: 0,
                page,
                size,
            });
        }

        let total = self.count(filter).await?;
        let total_pages = total_pages(total, size);
        let offset = (page - 1).saturating_mul(size);

        let mut builder = QueryBuilder::<Sqlite>::new(
            "SELECT id, created_at, updated_at, deleted_at, name, role, ku_id, steam_id, time, action, action_desc, ip, cluster_name FROM player_logs",
        );
        append_active_filters(&mut builder, filter);
        builder.push(" ORDER BY created_at DESC LIMIT ");
        builder.push_bind(size);
        builder.push(" OFFSET ");
        builder.push_bind(offset);

        let data = builder
            .build_query_as::<PlayerLogRecord>()
            .fetch_all(&self.pool)
            .await?;

        Ok(PlayerLogPage {
            data,
            total,
            total_pages,
            page,
            size,
        })
    }

    /// Soft-deletes active player log rows by id, matching GORM `Delete`.
    pub async fn soft_delete_ids(&self, ids: &[i64]) -> Result<(), sqlx::Error> {
        if ids.is_empty() {
            return Ok(());
        }
        let now = Utc::now();
        let mut builder = QueryBuilder::<Sqlite>::new("UPDATE player_logs SET deleted_at = ");
        builder.push_bind(now);
        builder.push(" WHERE deleted_at IS NULL AND id IN (");
        let mut separated = builder.separated(", ");
        for id in ids {
            separated.push_bind(id);
        }
        separated.push_unseparated(")");
        builder.build().execute(&self.pool).await?;
        Ok(())
    }

    async fn count(&self, filter: &PlayerLogFilter) -> Result<i64, sqlx::Error> {
        let mut builder = QueryBuilder::<Sqlite>::new("SELECT COUNT(*) FROM player_logs");
        append_active_filters(&mut builder, filter);
        builder.build_query_scalar().fetch_one(&self.pool).await
    }
}

fn append_active_filters(builder: &mut QueryBuilder<'_, Sqlite>, filter: &PlayerLogFilter) {
    builder.push(" WHERE deleted_at IS NULL");
    append_like_filter(builder, "name", filter.name.as_deref());
    append_like_filter(builder, "ku_id", filter.ku_id.as_deref());
    append_like_filter(builder, "role", filter.role.as_deref());
    append_like_filter(builder, "action", filter.action.as_deref());
    append_like_filter(builder, "ip", filter.ip.as_deref());
}

fn append_like_filter(
    builder: &mut QueryBuilder<'_, Sqlite>,
    column: &'static str,
    value: Option<&str>,
) {
    if let Some(value) = value {
        builder.push(" AND ");
        builder.push(column);
        builder.push(" LIKE ");
        builder.push_bind(format!("%{value}%"));
    }
}

fn total_pages(total: i64, size: i64) -> i64 {
    if total == 0 {
        0
    } else {
        (total - 1) / size + 1
    }
}

/// Normalizes a Go-style page size while preserving Go's unbounded positive sizes.
pub fn normalize_page_size(size: i64) -> i64 {
    if size < 0 { 10 } else { size }
}
