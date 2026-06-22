//! Mod metadata database row and input models.
//!
//! These shapes preserve the Go `mod_infos` columns and JSON names.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Mod metadata row stored in Go's `mod_infos` table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromRow)]
pub struct ModInfoRecord {
    /// GORM model primary key.
    #[serde(rename = "ID")]
    pub id: i64,
    /// GORM creation timestamp.
    #[serde(rename = "CreatedAt")]
    pub created_at: Option<DateTime<Utc>>,
    /// GORM update timestamp.
    #[serde(rename = "UpdatedAt")]
    pub updated_at: Option<DateTime<Utc>>,
    /// GORM soft-delete timestamp.
    #[serde(rename = "DeletedAt")]
    pub deleted_at: Option<DateTime<Utc>>,
    /// Steam profile URL for the creator.
    pub auth: String,
    /// Steam consumer app id.
    pub consumer_appid: f64,
    /// Steam creator app id.
    pub creator_appid: f64,
    /// Steam file description.
    pub description: String,
    /// Direct v1 zip URL, when Steam exposes one.
    pub file_url: String,
    /// Workshop id or local mod id.
    pub modid: String,
    /// Preview image URL.
    pub img: String,
    /// Steam `time_updated` value.
    pub last_time: f64,
    /// JSON string containing parsed configuration options.
    pub mod_config: String,
    /// Mod display name.
    pub name: String,
    /// Version tag parsed from Steam tags.
    pub v: String,
    /// Whether the mod needs an update.
    #[serde(rename = "update")]
    pub update_available: bool,
}

impl ModInfoRecord {
    /// Returns the Go zero-value record used when `db.Find(&modInfo)` misses.
    pub fn zero() -> Self {
        Self {
            id: 0,
            created_at: None,
            updated_at: None,
            deleted_at: None,
            auth: String::new(),
            consumer_appid: 0.0,
            creator_appid: 0.0,
            description: String::new(),
            file_url: String::new(),
            modid: String::new(),
            img: String::new(),
            last_time: 0.0,
            mod_config: String::new(),
            name: String::new(),
            v: String::new(),
            update_available: false,
        }
    }
}

/// Writable fields accepted by the legacy `POST /api/mod/modinfo` route.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModInfoInput {
    /// Optional GORM id. Go `Save` updates when this is non-zero.
    #[serde(rename = "ID", alias = "id", default)]
    pub id: i64,
    /// Steam profile URL for the creator.
    #[serde(default)]
    pub auth: String,
    /// Steam consumer app id.
    #[serde(default)]
    pub consumer_appid: f64,
    /// Steam creator app id.
    #[serde(default)]
    pub creator_appid: f64,
    /// Steam file description.
    #[serde(default)]
    pub description: String,
    /// Direct v1 zip URL, when Steam exposes one.
    #[serde(default)]
    pub file_url: String,
    /// Workshop id or local mod id.
    #[serde(default)]
    pub modid: String,
    /// Preview image URL.
    #[serde(default)]
    pub img: String,
    /// Steam `time_updated` value.
    #[serde(default)]
    pub last_time: f64,
    /// JSON string containing parsed configuration options.
    #[serde(default)]
    pub mod_config: String,
    /// Mod display name.
    #[serde(default)]
    pub name: String,
    /// Version tag parsed from Steam tags.
    #[serde(default)]
    pub v: String,
    /// Whether the mod needs an update.
    #[serde(rename = "update", default)]
    pub update_available: bool,
}

impl From<&ModInfoRecord> for ModInfoInput {
    fn from(record: &ModInfoRecord) -> Self {
        Self {
            id: record.id,
            auth: record.auth.clone(),
            consumer_appid: record.consumer_appid,
            creator_appid: record.creator_appid,
            description: record.description.clone(),
            file_url: record.file_url.clone(),
            modid: record.modid.clone(),
            img: record.img.clone(),
            last_time: record.last_time,
            mod_config: record.mod_config.clone(),
            name: record.name.clone(),
            v: record.v.clone(),
            update_available: record.update_available,
        }
    }
}
