//! Request query/body DTOs for mod routes.

use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub(crate) struct SearchQuery {
    pub(super) text: Option<String>,
    pub(super) page: Option<i64>,
    pub(super) size: Option<i64>,
    pub(super) lang: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct LangQuery {
    pub(super) lang: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ManualModInfoPayload {
    #[serde(rename = "workshopId")]
    pub(super) workshop_id: String,
    pub(super) modinfo: String,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct UgcLevelQuery {
    #[serde(rename = "levelName")]
    pub(super) level_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeleteUgcQuery {
    #[serde(rename = "levelName")]
    pub(super) level_name: String,
    #[serde(rename = "workshopId")]
    pub(super) workshop_id: String,
}
