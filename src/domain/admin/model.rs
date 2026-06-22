//! Admin panel database row and input models.
//!
//! These shapes preserve the Go JSON tags for KV and web-link endpoints.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Key/value row stored in the `kvs` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromRow)]
pub struct KvRecord {
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
    /// KV key.
    pub key: String,
    /// KV value.
    pub value: String,
}

/// New web-link input matching the Go `WebLink` JSON shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewWebLink {
    /// Display title.
    pub title: String,
    /// Link target URL.
    pub url: String,
    /// Preferred iframe/window width from the Go UI.
    pub width: String,
    /// Preferred iframe/window height from the Go UI.
    pub height: String,
}

/// Web-link row stored in the `web_links` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromRow)]
pub struct WebLinkRecord {
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
    /// Display title.
    pub title: String,
    /// Link target URL.
    pub url: String,
    /// Preferred iframe/window width from the Go UI.
    pub width: String,
    /// Preferred iframe/window height from the Go UI.
    pub height: String,
}
