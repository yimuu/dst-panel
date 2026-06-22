//! Scheduler, announcement, and auto-check database row/input models.
//!
//! Timestamp serialization preserves Go's zero-time JSON shape for missing
//! GORM timestamps.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, Serializer};
use sqlx::FromRow;

/// Announcement setting row stored in Go's `announces` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromRow)]
pub struct AnnounceRecord {
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
    /// Whether periodic announcements are enabled.
    pub enable: bool,
    /// Number of announcement repetitions configured by the UI.
    pub frequency: i64,
    /// Announcement interval value.
    pub interval: i64,
    /// Unit for the interval field.
    #[serde(rename = "intervalUnit")]
    pub interval_unit: String,
    /// Announcement delivery method token.
    pub method: String,
    /// Announcement text, including any embedded newlines.
    pub content: String,
}

impl AnnounceRecord {
    /// Returns the zero-value model that Go serializes when `First` misses.
    pub fn zero() -> Self {
        Self {
            id: 0,
            created_at: None,
            updated_at: None,
            deleted_at: None,
            enable: false,
            frequency: 0,
            interval: 0,
            interval_unit: String::new(),
            method: String::new(),
            content: String::new(),
        }
    }
}

/// Writable announcement fields accepted by `/api/game/announce/setting`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveAnnounce {
    /// Optional GORM id. Go `Save` inserts when this is zero.
    #[serde(rename = "ID", alias = "id", default)]
    pub id: i64,
    /// Whether periodic announcements are enabled.
    #[serde(default)]
    pub enable: bool,
    /// Number of announcement repetitions configured by the UI.
    #[serde(default)]
    pub frequency: i64,
    /// Announcement interval value.
    #[serde(default)]
    pub interval: i64,
    /// Unit for the interval field.
    #[serde(rename = "intervalUnit", default)]
    pub interval_unit: String,
    /// Announcement delivery method token.
    #[serde(default)]
    pub method: String,
    /// Announcement text, including any embedded newlines.
    #[serde(default)]
    pub content: String,
}

/// Persisted scheduled job row from the Go `job_tasks` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromRow)]
pub struct JobTaskRecord {
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
    /// DST cluster name.
    #[serde(rename = "clusterName")]
    pub cluster_name: String,
    /// Human-facing level name.
    #[serde(rename = "levelName")]
    pub level_name: String,
    /// Level folder/uuid passed to the strategy.
    pub uuid: String,
    /// Five-field robfig/cron standard expression.
    pub cron: String,
    /// Task category such as `backup`, `update`, or `none`.
    pub category: String,
    /// UI comment.
    pub comment: String,
    /// Announcement text sent before task execution.
    pub announcement: String,
    /// Seconds between repeated announcements.
    pub sleep: i64,
    /// Number of announcement repeats.
    pub times: i64,
    /// Legacy script flag.
    pub script: i64,
}

/// Writable scheduled job body accepted by `POST /api/task`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveJobTask {
    /// Optional GORM id. Public creation requests usually omit this.
    #[serde(rename = "ID", alias = "id", default)]
    pub id: i64,
    /// DST cluster name, filled from `dst_config` when omitted.
    #[serde(rename = "clusterName", default)]
    pub cluster_name: String,
    /// Human-facing level name.
    #[serde(rename = "levelName", default)]
    pub level_name: String,
    /// Level folder/uuid passed to the strategy.
    #[serde(default)]
    pub uuid: String,
    /// Five-field robfig/cron standard expression.
    #[serde(default)]
    pub cron: String,
    /// Task category such as `backup`, `update`, or `none`.
    #[serde(default)]
    pub category: String,
    /// UI comment.
    #[serde(default)]
    pub comment: String,
    /// Announcement text sent before task execution.
    #[serde(default)]
    pub announcement: String,
    /// Seconds between repeated announcements.
    #[serde(default)]
    pub sleep: i64,
    /// Number of announcement repeats.
    #[serde(default)]
    pub times: i64,
    /// Legacy script flag.
    #[serde(default)]
    pub script: i64,
}

/// Auto-check row stored in Go's `auto_checks` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromRow)]
pub struct AutoCheckRecord {
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
    /// Legacy display/configuration name.
    pub name: String,
    /// DST cluster name.
    #[serde(rename = "clusterName")]
    pub cluster_name: String,
    /// Human-facing level name.
    #[serde(rename = "levelName")]
    pub level_name: String,
    /// Level folder or update-game synthetic uuid.
    pub uuid: String,
    /// Enabled flag stored as an integer by Go.
    pub enable: i64,
    /// Announcement text used when a check triggers an action.
    pub announcement: String,
    /// Number of retries or announcement repeats.
    pub times: i64,
    /// Sleep interval in seconds.
    pub sleep: i64,
    /// Check interval in minutes.
    pub interval: i64,
    /// Check category token such as `LEVEL_MOD`.
    #[serde(rename = "checkType")]
    pub check_type: String,
}

impl AutoCheckRecord {
    /// Builds a generated, unsaved Go-compatible auto-check row.
    pub fn generated(
        cluster_name: String,
        level_name: String,
        uuid: String,
        check_type: &str,
    ) -> Self {
        Self {
            id: 0,
            created_at: None,
            updated_at: None,
            deleted_at: None,
            name: String::new(),
            cluster_name,
            level_name,
            uuid,
            enable: 0,
            announcement: String::new(),
            times: 1,
            sleep: 5,
            interval: 5,
            check_type: check_type.to_owned(),
        }
    }

    /// Applies the fields that Go overlays from matching persisted settings.
    pub fn overlay_persisted_settings(&mut self, persisted: &AutoCheckRecord) {
        self.id = persisted.id;
        self.created_at = persisted.created_at;
        self.updated_at = persisted.updated_at;
        self.deleted_at = persisted.deleted_at;
        self.enable = persisted.enable;
        self.announcement = persisted.announcement.clone();
        self.times = persisted.times;
        self.sleep = persisted.sleep;
        self.interval = persisted.interval;
    }
}

/// Writable auto-check body accepted by `POST /api/auto/check2`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveAutoCheck {
    /// Optional GORM id. Go `Save` inserts when this is zero.
    #[serde(rename = "ID", alias = "id", default)]
    pub id: i64,
    /// Legacy display/configuration name.
    #[serde(default)]
    pub name: String,
    /// DST cluster name.
    #[serde(rename = "clusterName", default)]
    pub cluster_name: String,
    /// Human-facing level name.
    #[serde(rename = "levelName", default)]
    pub level_name: String,
    /// Level folder or update-game synthetic uuid.
    #[serde(default)]
    pub uuid: String,
    /// Enabled flag stored as an integer by Go.
    #[serde(default)]
    pub enable: i64,
    /// Announcement text used when a check triggers an action.
    #[serde(default)]
    pub announcement: String,
    /// Number of retries or announcement repeats.
    #[serde(default)]
    pub times: i64,
    /// Sleep interval in seconds.
    #[serde(default)]
    pub sleep: i64,
    /// Check interval in minutes.
    #[serde(default)]
    pub interval: i64,
    /// Check category token such as `LEVEL_MOD`.
    #[serde(rename = "checkType", default)]
    pub check_type: String,
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
