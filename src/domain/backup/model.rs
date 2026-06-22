//! Backup and snapshot database row/input models.
//!
//! Snapshot timestamp serialization preserves Go's zero-time JSON shape.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, Serializer};
use sqlx::FromRow;

/// Backup snapshot singleton row stored in Go's `backup_snapshots` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromRow)]
pub struct BackupSnapshotRecord {
    /// GORM model primary key.
    #[serde(rename = "ID")]
    pub id: i64,
    /// GORM creation timestamp.
    #[serde(rename = "CreatedAt", serialize_with = "serialize_gorm_time")]
    pub created_at: Option<DateTime<Utc>>,
    /// GORM update timestamp.
    #[serde(rename = "UpdatedAt", serialize_with = "serialize_gorm_time")]
    pub updated_at: Option<DateTime<Utc>>,
    /// GORM soft-delete timestamp.
    #[serde(rename = "DeletedAt")]
    pub deleted_at: Option<DateTime<Utc>>,
    /// Snapshot profile name.
    pub name: String,
    /// Interval in minutes.
    pub interval: i64,
    /// Maximum retained snapshots. Go JSON uses camelCase here.
    #[serde(rename = "maxSnapshots")]
    pub max_snapshots: i64,
    /// Whether snapshot scheduling is enabled.
    pub enable: i64,
    /// Whether a `c_save()` should be issued before snapshotting.
    #[serde(rename = "isCSave")]
    pub is_c_save: i64,
}

impl BackupSnapshotRecord {
    /// Returns Go's zero-value model when the table has no active row.
    pub fn zero() -> Self {
        Self {
            id: 0,
            created_at: None,
            updated_at: None,
            deleted_at: None,
            name: String::new(),
            interval: 0,
            max_snapshots: 0,
            enable: 0,
            is_c_save: 0,
        }
    }
}

/// Backup snapshot request body. It intentionally omits GORM timestamps so
/// callers cannot set soft-delete metadata through the public API.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveBackupSnapshot {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub interval: i64,
    #[serde(rename = "maxSnapshots", default)]
    pub max_snapshots: i64,
    #[serde(default)]
    pub enable: i64,
    #[serde(rename = "isCSave", default)]
    pub is_c_save: i64,
}

fn serialize_gorm_time<S>(value: &Option<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(value) => value.serialize(serializer),
        None => serializer.serialize_str("0001-01-01T00:00:00Z"),
    }
}
