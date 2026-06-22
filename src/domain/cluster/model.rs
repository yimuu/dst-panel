//! Cluster database row and input models.
//!
//! Serde names follow the Go cluster JSON tags exposed by migrated handlers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// New cluster input matching the Go `Cluster` JSON shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewCluster {
    /// Unique DST cluster name.
    #[serde(rename = "clusterName")]
    pub cluster_name: String,
    /// Human-facing description.
    pub description: String,
    /// SteamCMD installation directory.
    #[serde(rename = "steamcmd")]
    pub steam_cmd: String,
    /// Dedicated server installation directory.
    pub force_install_dir: String,
    /// Backup directory.
    pub backup: String,
    /// Mod download directory.
    pub mod_download_path: String,
    /// Cluster UUID.
    pub uuid: String,
    /// Whether the beta branch is used.
    pub beta: i64,
    /// Server binary architecture flag.
    pub bin: i64,
    /// UGC directory.
    pub ugc_directory: String,
    /// Klei persistent storage root.
    pub persistent_storage_root: String,
    /// Klei configuration directory.
    pub conf_dir: String,
}

/// Cluster row stored in the `clusters` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromRow)]
pub struct ClusterRecord {
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
    /// Unique DST cluster name.
    #[serde(rename = "clusterName")]
    pub cluster_name: String,
    /// Human-facing description.
    pub description: String,
    /// SteamCMD installation directory.
    #[serde(rename = "steamcmd")]
    pub steam_cmd: String,
    /// Dedicated server installation directory.
    pub force_install_dir: String,
    /// Backup directory.
    pub backup: String,
    /// Mod download directory.
    pub mod_download_path: String,
    /// Cluster UUID.
    pub uuid: String,
    /// Whether the beta branch is used.
    pub beta: i64,
    /// Server binary architecture flag.
    pub bin: i64,
    /// UGC directory.
    pub ugc_directory: String,
    /// Klei persistent storage root.
    pub persistent_storage_root: String,
    /// Klei configuration directory.
    pub conf_dir: String,
}
