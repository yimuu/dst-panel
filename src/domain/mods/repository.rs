//! Repository for Go-compatible `mod_infos` rows.

use chrono::Utc;
use sqlx::SqlitePool;

use crate::domain::mods::model::{ModInfoInput, ModInfoRecord};

const MOD_INFO_COLUMNS: &str = "id, created_at, updated_at, deleted_at, auth, consumer_appid, creator_appid, description, file_url, modid, img, last_time, mod_config, name, v, \"update\" as update_available";

/// Persistence operations for mod metadata.
#[derive(Debug, Clone)]
pub struct ModInfoRepository {
    pool: SqlitePool,
}

impl ModInfoRepository {
    /// Creates a repository backed by the shared SQLite pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Lists active rows using GORM's implicit soft-delete filter.
    pub async fn list_active(&self) -> Result<Vec<ModInfoRecord>, sqlx::Error> {
        sqlx::query_as::<_, ModInfoRecord>(&format!(
            "SELECT {MOD_INFO_COLUMNS}
             FROM mod_infos
             WHERE deleted_at IS NULL
             ORDER BY id ASC"
        ))
        .fetch_all(&self.pool)
        .await
    }

    /// Finds the first active row for a mod id.
    pub async fn find_active_by_modid(
        &self,
        modid: &str,
    ) -> Result<Option<ModInfoRecord>, sqlx::Error> {
        sqlx::query_as::<_, ModInfoRecord>(&format!(
            "SELECT {MOD_INFO_COLUMNS}
             FROM mod_infos
             WHERE deleted_at IS NULL AND modid = ?
             ORDER BY id ASC
             LIMIT 1"
        ))
        .bind(modid)
        .fetch_optional(&self.pool)
        .await
    }

    /// Saves a mod row, preserving Go `db.Save` create-or-update behavior.
    pub async fn save(&self, input: ModInfoInput) -> Result<ModInfoRecord, sqlx::Error> {
        if input.id > 0 {
            let updated = self.update_active(&input).await?;
            if updated {
                return self.get_by_id(input.id).await;
            }
            let id = input.id;
            return self.insert_with_optional_id(input, Some(id)).await;
        }
        self.insert_with_optional_id(input, None).await
    }

    /// Creates a new row and returns it.
    pub async fn create(&self, input: ModInfoInput) -> Result<ModInfoRecord, sqlx::Error> {
        self.insert_with_optional_id(input, None).await
    }

    /// Updates the first active row with the same mod id, or creates one.
    ///
    /// Go's `AddModInfo` eventually calls `Save` on an existing row when one
    /// can be found by `modid`. Keeping this helper explicit avoids creating a
    /// second active row when a manual `modinfo.lua` upload refreshes metadata.
    pub async fn upsert_by_modid(
        &self,
        mut input: ModInfoInput,
    ) -> Result<ModInfoRecord, sqlx::Error> {
        if let Some(existing) = self.find_active_by_modid(&input.modid).await? {
            input.id = existing.id;
            return self.save(input).await;
        }
        self.create(input).await
    }

    /// Soft-deletes active rows for a mod id.
    pub async fn soft_delete_by_modid(&self, modid: &str) -> Result<u64, sqlx::Error> {
        let now = Utc::now();
        let result = sqlx::query(
            "UPDATE mod_infos
             SET deleted_at = ?, updated_at = ?
             WHERE modid = ? AND deleted_at IS NULL",
        )
        .bind(now)
        .bind(now)
        .bind(modid)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    async fn update_active(&self, input: &ModInfoInput) -> Result<bool, sqlx::Error> {
        let now = Utc::now();
        let result = sqlx::query(
            "UPDATE mod_infos
             SET auth = ?,
                 consumer_appid = ?,
                 creator_appid = ?,
                 description = ?,
                 file_url = ?,
                 modid = ?,
                 img = ?,
                 last_time = ?,
                 mod_config = ?,
                 name = ?,
                 v = ?,
                 \"update\" = ?,
                 updated_at = ?
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(&input.auth)
        .bind(input.consumer_appid)
        .bind(input.creator_appid)
        .bind(&input.description)
        .bind(&input.file_url)
        .bind(&input.modid)
        .bind(&input.img)
        .bind(input.last_time)
        .bind(&input.mod_config)
        .bind(&input.name)
        .bind(&input.v)
        .bind(input.update_available)
        .bind(now)
        .bind(input.id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn insert_with_optional_id(
        &self,
        input: ModInfoInput,
        explicit_id: Option<i64>,
    ) -> Result<ModInfoRecord, sqlx::Error> {
        let now = Utc::now();
        let id = if let Some(id) = explicit_id {
            sqlx::query(
                "INSERT INTO mod_infos (
                    id,
                    created_at,
                    updated_at,
                    auth,
                    consumer_appid,
                    creator_appid,
                    description,
                    file_url,
                    modid,
                    img,
                    last_time,
                    mod_config,
                    name,
                    v,
                    \"update\"
                 )
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(id)
            .bind(now)
            .bind(now)
            .bind(&input.auth)
            .bind(input.consumer_appid)
            .bind(input.creator_appid)
            .bind(&input.description)
            .bind(&input.file_url)
            .bind(&input.modid)
            .bind(&input.img)
            .bind(input.last_time)
            .bind(&input.mod_config)
            .bind(&input.name)
            .bind(&input.v)
            .bind(input.update_available)
            .execute(&self.pool)
            .await?;
            id
        } else {
            let result = sqlx::query(
                "INSERT INTO mod_infos (
                    created_at,
                    updated_at,
                    auth,
                    consumer_appid,
                    creator_appid,
                    description,
                    file_url,
                    modid,
                    img,
                    last_time,
                    mod_config,
                    name,
                    v,
                    \"update\"
                 )
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(now)
            .bind(now)
            .bind(&input.auth)
            .bind(input.consumer_appid)
            .bind(input.creator_appid)
            .bind(&input.description)
            .bind(&input.file_url)
            .bind(&input.modid)
            .bind(&input.img)
            .bind(input.last_time)
            .bind(&input.mod_config)
            .bind(&input.name)
            .bind(&input.v)
            .bind(input.update_available)
            .execute(&self.pool)
            .await?;
            result.last_insert_rowid()
        };
        self.get_by_id(id).await
    }

    async fn get_by_id(&self, id: i64) -> Result<ModInfoRecord, sqlx::Error> {
        sqlx::query_as::<_, ModInfoRecord>(&format!(
            "SELECT {MOD_INFO_COLUMNS}
             FROM mod_infos
             WHERE id = ?"
        ))
        .bind(id)
        .fetch_one(&self.pool)
        .await
    }
}
