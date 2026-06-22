//! Statistics database row models.
//!
//! These models back player-log listing and aggregate statistics endpoints.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Player log row stored by the Go collector in `player_logs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromRow)]
pub struct PlayerLogRecord {
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
    /// Player display name parsed from DST logs.
    pub name: String,
    /// Character role parsed from DST logs.
    pub role: String,
    /// Klei user id.
    #[serde(rename = "kuId")]
    pub ku_id: String,
    /// Steam id.
    #[serde(rename = "steamId")]
    pub steam_id: String,
    /// Original event timestamp string.
    pub time: String,
    /// Go collector action token such as `[JoinAnnouncement]`.
    pub action: String,
    /// Action detail text, such as death or chat content.
    #[serde(rename = "actionDesc")]
    pub action_desc: String,
    /// Player IP captured by the collector.
    pub ip: String,
    /// DST cluster name.
    #[serde(rename = "clusterName")]
    pub cluster_name: String,
}

/// Regenerate row stored by the Go collector in `regenerates`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromRow)]
pub struct RegenerateRecord {
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
    /// DST cluster name.
    #[serde(rename = "clusterName")]
    pub cluster_name: String,
}
