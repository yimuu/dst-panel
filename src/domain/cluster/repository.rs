//! Repository for the Go-compatible `clusters` table.

use chrono::Utc;
use sqlx::SqlitePool;

use crate::domain::cluster::model::{ClusterRecord, NewCluster};

const CLUSTER_COLUMNS: &str = "id, created_at, updated_at, deleted_at, cluster_name, description, steam_cmd, force_install_dir, backup, mod_download_path, uuid, beta, bin, ugc_directory, persistent_storage_root, conf_dir";

/// Persistence operations for DST cluster records.
#[derive(Debug, Clone)]
pub struct ClusterRepository {
    pool: SqlitePool,
}

impl ClusterRepository {
    /// Creates a cluster repository backed by a SQLite pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Lists active clusters, ignoring GORM soft-deleted rows.
    pub async fn list(&self) -> Result<Vec<ClusterRecord>, sqlx::Error> {
        sqlx::query_as::<_, ClusterRecord>(&format!(
            "SELECT {CLUSTER_COLUMNS}
             FROM clusters
             WHERE deleted_at IS NULL
             ORDER BY created_at DESC, id DESC"
        ))
        .fetch_all(&self.pool)
        .await
    }

    /// Counts active clusters matching the optional Go `clusterName` filter.
    pub async fn count_filtered(&self, cluster_name: Option<&str>) -> Result<i64, sqlx::Error> {
        if let Some(cluster_name) = cluster_name.filter(|name| !name.is_empty()) {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*)
                 FROM clusters
                 WHERE deleted_at IS NULL AND cluster_name LIKE ?",
            )
            .bind(format!("%{cluster_name}%"))
            .fetch_one(&self.pool)
            .await
        } else {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*)
                 FROM clusters
                 WHERE deleted_at IS NULL",
            )
            .fetch_one(&self.pool)
            .await
        }
    }

    /// Lists active clusters with Go-compatible filtering, ordering, and paging.
    pub async fn list_filtered_page(
        &self,
        cluster_name: Option<&str>,
        page: i64,
        size: i64,
    ) -> Result<Vec<ClusterRecord>, sqlx::Error> {
        let offset = (page.saturating_sub(1)).saturating_mul(size);
        if let Some(cluster_name) = cluster_name.filter(|name| !name.is_empty()) {
            sqlx::query_as::<_, ClusterRecord>(&format!(
                "SELECT {CLUSTER_COLUMNS}
                 FROM clusters
                 WHERE deleted_at IS NULL AND cluster_name LIKE ?
                 ORDER BY created_at DESC, id DESC
                 LIMIT ? OFFSET ?"
            ))
            .bind(format!("%{cluster_name}%"))
            .bind(size)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, ClusterRecord>(&format!(
                "SELECT {CLUSTER_COLUMNS}
                 FROM clusters
                 WHERE deleted_at IS NULL
                 ORDER BY created_at DESC, id DESC
                 LIMIT ? OFFSET ?"
            ))
            .bind(size)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
        }
    }

    /// Creates a cluster and returns the stored row.
    pub async fn create(&self, cluster: NewCluster) -> Result<ClusterRecord, sqlx::Error> {
        if cluster.cluster_name.trim().is_empty() {
            tracing::warn!("refusing to create cluster with empty cluster_name");
            return Err(sqlx::Error::Protocol(
                "cluster_name must not be empty".to_owned(),
            ));
        }

        let now = Utc::now();
        let result = sqlx::query(
            "INSERT INTO clusters (
                created_at,
                updated_at,
                cluster_name,
                description,
                steam_cmd,
                force_install_dir,
                backup,
                mod_download_path,
                uuid,
                beta,
                bin,
                ugc_directory,
                persistent_storage_root,
                conf_dir
             )
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(now)
        .bind(now)
        .bind(&cluster.cluster_name)
        .bind(&cluster.description)
        .bind(&cluster.steam_cmd)
        .bind(&cluster.force_install_dir)
        .bind(&cluster.backup)
        .bind(&cluster.mod_download_path)
        .bind(&cluster.uuid)
        .bind(cluster.beta)
        .bind(cluster.bin)
        .bind(&cluster.ugc_directory)
        .bind(&cluster.persistent_storage_root)
        .bind(&cluster.conf_dir)
        .execute(&self.pool)
        .await;

        match result {
            Ok(result) => self.get_by_id(result.last_insert_rowid()).await,
            Err(error) => {
                if is_unique_constraint_error(&error) {
                    tracing::warn!(
                        cluster_name = %cluster.cluster_name,
                        "cluster create hit unique cluster_name constraint"
                    );
                }
                Err(error)
            }
        }
    }

    /// Updates an active cluster and returns the stored row.
    pub async fn update(&self, cluster: ClusterRecord) -> Result<ClusterRecord, sqlx::Error> {
        if cluster.cluster_name.trim().is_empty() {
            tracing::warn!(
                id = cluster.id,
                "refusing to update cluster with empty cluster_name"
            );
            return Err(sqlx::Error::Protocol(
                "cluster_name must not be empty".to_owned(),
            ));
        }

        let now = Utc::now();
        let result = sqlx::query(
            "UPDATE clusters
             SET cluster_name = ?,
                 description = ?,
                 steam_cmd = ?,
                 force_install_dir = ?,
                 backup = ?,
                 mod_download_path = ?,
                 uuid = ?,
                 beta = ?,
                 bin = ?,
                 ugc_directory = ?,
                 persistent_storage_root = ?,
                 conf_dir = ?,
                 updated_at = ?
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(&cluster.cluster_name)
        .bind(&cluster.description)
        .bind(&cluster.steam_cmd)
        .bind(&cluster.force_install_dir)
        .bind(&cluster.backup)
        .bind(&cluster.mod_download_path)
        .bind(&cluster.uuid)
        .bind(cluster.beta)
        .bind(cluster.bin)
        .bind(&cluster.ugc_directory)
        .bind(&cluster.persistent_storage_root)
        .bind(&cluster.conf_dir)
        .bind(now)
        .bind(cluster.id)
        .execute(&self.pool)
        .await;

        match result {
            Ok(result) if result.rows_affected() > 0 => self.get_by_id(cluster.id).await,
            Ok(_) => {
                tracing::warn!(id = cluster.id, "cluster update matched no active row");
                Err(sqlx::Error::RowNotFound)
            }
            Err(error) => {
                if is_unique_constraint_error(&error) {
                    tracing::warn!(
                        id = cluster.id,
                        cluster_name = %cluster.cluster_name,
                        "cluster update hit unique cluster_name constraint"
                    );
                }
                Err(error)
            }
        }
    }

    /// Updates only the cluster description, matching Go's current update behavior.
    pub async fn update_description(
        &self,
        id: i64,
        description: &str,
    ) -> Result<ClusterRecord, sqlx::Error> {
        let now = Utc::now();
        let result = sqlx::query(
            "UPDATE clusters
             SET description = ?, updated_at = ?
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(description)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() > 0 {
            self.get_by_id(id).await
        } else {
            tracing::warn!(id, "cluster description update matched no active row");
            Err(sqlx::Error::RowNotFound)
        }
    }

    /// Soft-deletes a cluster by setting `deleted_at`.
    pub async fn delete(&self, id: i64) -> Result<bool, sqlx::Error> {
        let now = Utc::now();
        let result = sqlx::query(
            "UPDATE clusters
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
            tracing::warn!(id, "cluster delete matched no active row");
        }
        Ok(deleted)
    }

    /// Removes a just-created row when non-database setup fails before the API
    /// returns success. Normal user deletes must keep using soft delete.
    pub async fn hard_delete_for_rollback(&self, id: i64) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM clusters WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        let deleted = result.rows_affected() > 0;
        if !deleted {
            tracing::warn!(id, "cluster rollback hard delete matched no row");
        }
        Ok(deleted)
    }

    async fn get_by_id(&self, id: i64) -> Result<ClusterRecord, sqlx::Error> {
        sqlx::query_as::<_, ClusterRecord>(&format!(
            "SELECT {CLUSTER_COLUMNS}
             FROM clusters
             WHERE id = ? AND deleted_at IS NULL"
        ))
        .bind(id)
        .fetch_one(&self.pool)
        .await
    }
}

fn is_unique_constraint_error(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .is_some_and(|db_error| db_error.message().contains("UNIQUE constraint failed"))
}
