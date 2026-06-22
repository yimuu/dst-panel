//! Repository for Go-compatible scheduled task persistence.

use chrono::Utc;
use sqlx::SqlitePool;

use crate::domain::scheduler::model::{JobTaskRecord, SaveJobTask};

const JOB_TASK_COLUMNS: &str = "id, created_at, updated_at, deleted_at, cluster_name, level_name, uuid, cron, category, comment, announcement, sleep, times, script";

/// Persistence operations for the `job_tasks` table.
#[derive(Debug, Clone)]
pub struct JobTaskRepository {
    pool: SqlitePool,
}

impl JobTaskRepository {
    /// Creates a repository backed by the shared SQLite pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Lists active scheduled tasks in insertion order.
    pub async fn list_active(&self) -> Result<Vec<JobTaskRecord>, sqlx::Error> {
        sqlx::query_as::<_, JobTaskRecord>(&format!(
            "SELECT {JOB_TASK_COLUMNS}
             FROM job_tasks
             WHERE deleted_at IS NULL
             ORDER BY id ASC"
        ))
        .fetch_all(&self.pool)
        .await
    }

    /// Inserts a new scheduled task and returns the stored row.
    pub async fn create(&self, input: SaveJobTask) -> Result<JobTaskRecord, sqlx::Error> {
        let now = Utc::now();
        let result = sqlx::query(
            "INSERT INTO job_tasks (
                created_at, updated_at, cluster_name, level_name, uuid, cron,
                category, comment, announcement, sleep, times, script
             )
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(now)
        .bind(now)
        .bind(input.cluster_name)
        .bind(input.level_name)
        .bind(input.uuid)
        .bind(input.cron)
        .bind(input.category)
        .bind(input.comment)
        .bind(input.announcement)
        .bind(input.sleep)
        .bind(input.times)
        .bind(input.script)
        .execute(&self.pool)
        .await?;

        let id = result.last_insert_rowid();
        tracing::info!(id, "created scheduled task");
        self.get_by_id(id).await
    }

    /// Soft-deletes the task addressed by the exposed runtime `jobId`.
    ///
    /// The background runtime polls active rows from this table, so a soft
    /// delete stops future executions while preserving Go-compatible delete
    /// behavior. The stable exposed job id is the persisted row id.
    pub async fn delete_by_job_id(&self, job_id: i64) -> Result<bool, sqlx::Error> {
        let now = Utc::now();
        let result = sqlx::query(
            "UPDATE job_tasks
             SET deleted_at = ?, updated_at = ?
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(now)
        .bind(now)
        .bind(job_id)
        .execute(&self.pool)
        .await?;

        let deleted = result.rows_affected() > 0;
        if deleted {
            tracing::info!(job_id, "deleted scheduled task");
        } else {
            tracing::debug!(job_id, "scheduled task delete matched no active row");
        }
        Ok(deleted)
    }

    async fn get_by_id(&self, id: i64) -> Result<JobTaskRecord, sqlx::Error> {
        sqlx::query_as::<_, JobTaskRecord>(&format!(
            "SELECT {JOB_TASK_COLUMNS}
             FROM job_tasks
             WHERE id = ? AND deleted_at IS NULL"
        ))
        .bind(id)
        .fetch_one(&self.pool)
        .await
    }
}
