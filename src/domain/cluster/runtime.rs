//! Best-effort runtime enrichment for cluster list rows.
//!
//! The legacy Go list route mixes persisted cluster records with live shard
//! process state and a community lobby lookup. This module keeps those side
//! effects behind fakeable process and HTTP adapters so `/api/cluster` can
//! report Go-compatible runtime fields without making list failures depend on
//! optional upstream data.

use std::{
    collections::{HashMap, HashSet},
    path::Path,
    time::Duration,
};

use futures_util::stream::{self, StreamExt};
use serde_json::{Value, json};

use crate::{
    domain::cluster::model::ClusterRecord,
    dst::{self, cluster_ini::ClusterIni},
    infra::http_client::{HttpClient, HttpRequest},
    infra::process::{self, ProcessSnapshot},
};

const HOME_SERVER_LIST_URL: &str = "https://dst.liuyh.com/index/serverlist/getserverlist.html";
const LOBBY_LOOKUP_TIMEOUT: Duration = Duration::from_secs(2);
const LOBBY_LOOKUP_CONCURRENCY: usize = 4;

/// Runtime-only fields in Go's `ClusterVO` list item.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ClusterRuntimeInfo {
    pub(crate) master: bool,
    pub(crate) caves: bool,
    pub(crate) row_id: String,
    pub(crate) connected: i64,
    pub(crate) max_connections: i64,
    pub(crate) mode: String,
    pub(crate) mods: i64,
    pub(crate) season: String,
    pub(crate) region: String,
}

/// Request-scoped lobby lookup rows keyed by the upstream search term.
#[derive(Debug, Clone, Default)]
pub(crate) struct ClusterLobbyRows {
    rows_by_key: HashMap<LobbySearchKey, Vec<LobbyRow>>,
}

impl ClusterLobbyRows {
    fn matching_row(&self, cluster_ini: &ClusterIni) -> Option<&LobbyRow> {
        self.rows_by_key
            .get(&LobbySearchKey::from(cluster_ini))
            .and_then(|rows| rows.iter().find(|row| row.matches(cluster_ini)))
    }
}

/// Collects lobby lookup responses once per unique upstream search key.
pub(crate) async fn collect_lobby_rows_for_clusters(
    root: &Path,
    records: &[ClusterRecord],
    http_client: &dyn HttpClient,
) -> ClusterLobbyRows {
    let mut seen = HashSet::new();
    let mut lookups = Vec::new();
    for record in records {
        let Some(cluster_ini) = read_cluster_ini(root, record) else {
            continue;
        };
        let key = LobbySearchKey::from(&cluster_ini);
        if seen.insert(key.clone()) {
            lookups.push((key, cluster_ini));
        }
    }

    let lookup_count = lookups.len();
    let pairs = stream::iter(lookups.into_iter().map(|(key, cluster_ini)| async move {
        let rows = fetch_lobby_rows(http_client, &cluster_ini)
            .await
            .unwrap_or_default();
        (key, rows)
    }))
    .buffered(LOBBY_LOOKUP_CONCURRENCY)
    .collect::<Vec<_>>()
    .await;

    tracing::debug!(
        lookup_count,
        "collected request-scoped cluster lobby lookup cache"
    );
    ClusterLobbyRows {
        rows_by_key: pairs.into_iter().collect(),
    }
}

/// Collects runtime data for a persisted cluster record.
pub(crate) async fn collect_for_cluster(
    root: &Path,
    record: &ClusterRecord,
    snapshots: &[ProcessSnapshot],
    lobby_rows: &ClusterLobbyRows,
) -> ClusterRuntimeInfo {
    let mut runtime = collect_process_runtime(record, snapshots);
    let Some(cluster_ini) = read_cluster_ini(root, record) else {
        return runtime;
    };

    match lobby_rows.matching_row(&cluster_ini) {
        Some(lobby) => apply_lobby_row(&mut runtime, lobby),
        None => tracing::debug!(
            cluster_name = %record.cluster_name,
            lobby_name = %cluster_ini.cluster_name,
            "cluster list lobby lookup produced no matching row"
        ),
    }

    runtime
}

fn collect_process_runtime(
    record: &ClusterRecord,
    snapshots: &[ProcessSnapshot],
) -> ClusterRuntimeInfo {
    let master = process::first_level_process(snapshots, &record.cluster_name, "Master").is_some();
    let caves = process::first_level_process(snapshots, &record.cluster_name, "Caves").is_some();
    tracing::debug!(
        cluster_name = %record.cluster_name,
        snapshot_count = snapshots.len(),
        master,
        caves,
        "collected cluster list process runtime"
    );

    ClusterRuntimeInfo {
        master,
        caves,
        ..ClusterRuntimeInfo::default()
    }
}

fn read_cluster_ini(root: &Path, record: &ClusterRecord) -> Option<ClusterIni> {
    let cluster_dir = match dst::cluster_dir(root, &record.cluster_name) {
        Ok(cluster_dir) => cluster_dir,
        Err(error) => {
            tracing::warn!(
                cluster_name = %record.cluster_name,
                error = %error,
                "failed to resolve cluster directory for lobby runtime"
            );
            return None;
        }
    };
    let contents = match dst::safe_read_cluster_file_to_string(&cluster_dir, "cluster.ini") {
        Ok(Some(contents)) => contents,
        Ok(None) => {
            tracing::debug!(
                cluster_name = %record.cluster_name,
                "cluster.ini missing; skipping lobby runtime enrichment"
            );
            return None;
        }
        Err(error) => {
            tracing::warn!(
                cluster_name = %record.cluster_name,
                error = %error,
                "failed to read cluster.ini for lobby runtime"
            );
            return None;
        }
    };

    Some(ClusterIni::from_contents(&contents))
}

async fn fetch_lobby_rows(
    http_client: &dyn HttpClient,
    cluster_ini: &ClusterIni,
) -> Option<Vec<LobbyRow>> {
    let request = HttpRequest::new("POST", HOME_SERVER_LIST_URL)
        .header("X-Requested-With", "XMLHttpRequest")
        .header("Content-Type", "application/json")
        .body(
            serde_json::to_vec(&lobby_request_payload(cluster_ini))
                .expect("cluster lobby request payload is serializable"),
        );
    let response = match tokio::time::timeout(LOBBY_LOOKUP_TIMEOUT, http_client.send(request)).await
    {
        Ok(Ok(response)) => response,
        Ok(Err(error)) => {
            tracing::warn!(error = %error, "cluster list lobby request failed");
            return None;
        }
        Err(error) => {
            tracing::warn!(
                timeout_secs = LOBBY_LOOKUP_TIMEOUT.as_secs(),
                error = %error,
                "cluster list lobby request timed out"
            );
            return None;
        }
    };
    if response.status != 200 {
        tracing::warn!(
            status = response.status,
            body_len = response.body.len(),
            "cluster list lobby request returned non-200 status"
        );
        return None;
    }

    let rows = match parse_lobby_response(&response.body) {
        Ok(rows) => rows,
        Err(error) => {
            tracing::warn!(
                body_len = response.body.len(),
                error = %error,
                "failed to parse cluster list lobby response"
            );
            return None;
        }
    };

    tracing::debug!(
        lobby_name = %cluster_ini.cluster_name,
        row_count = rows.len(),
        "parsed cluster list lobby response"
    );
    Some(rows)
}

fn lobby_request_payload(cluster_ini: &ClusterIni) -> Value {
    json!({
        "page": 1,
        "paginate": 10,
        "sort_type": "name",
        "sort_way": 1,
        "search_type": 1,
        "search_content": cluster_ini.cluster_name,
        "mod": 1
    })
}

fn parse_lobby_response(bytes: &[u8]) -> Result<Vec<LobbyRow>, serde_json::Error> {
    let value: Value = serde_json::from_slice(bytes)?;
    let value = match value {
        Value::String(contents) => serde_json::from_str(&contents)?,
        other => other,
    };

    let rows = lobby_rows_value(&value)
        .and_then(Value::as_array)
        .map(|rows| rows.iter().filter_map(LobbyRow::from_value).collect())
        .unwrap_or_default();
    Ok(rows)
}

fn lobby_rows_value(value: &Value) -> Option<&Value> {
    if value.as_array().is_some() {
        return Some(value);
    }

    if let Some(successinfo_data) = value
        .get("successinfo")
        .and_then(|successinfo| successinfo.get("data"))
        .filter(|nested| nested.as_array().is_some())
    {
        return Some(successinfo_data);
    }

    let data = value.get("data")?;
    if data.as_array().is_some() {
        return Some(data);
    }
    data.get("data")
        .filter(|nested| nested.as_array().is_some())
}

fn apply_lobby_row(runtime: &mut ClusterRuntimeInfo, lobby: &LobbyRow) {
    runtime.row_id = lobby.row_id.clone();
    runtime.connected = lobby.connected;
    runtime.max_connections = lobby.max_connections;
    runtime.mode = lobby.mode.clone();
    runtime.mods = lobby.mods;
    runtime.season = lobby.season.clone();
    runtime.region = lobby.region.clone();
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LobbyRow {
    row_id: String,
    connected: i64,
    max_connections: i64,
    mode: String,
    mods: i64,
    name: String,
    password: bool,
    season: String,
    region: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct LobbySearchKey {
    cluster_name: String,
}

impl From<&ClusterIni> for LobbySearchKey {
    fn from(cluster_ini: &ClusterIni) -> Self {
        Self {
            cluster_name: cluster_ini.cluster_name.clone(),
        }
    }
}

impl LobbyRow {
    fn from_value(value: &Value) -> Option<Self> {
        let row = value.as_array()?;
        Some(Self {
            row_id: string_at(row, 0),
            connected: i64_at(row, 5),
            max_connections: i64_at(row, 6),
            mode: string_at(row, 8),
            mods: i64_at(row, 9),
            name: string_at(row, 10),
            password: bool_at(row, 11),
            season: string_at(row, 14),
            region: string_at(row, 20),
        })
    }

    fn matches(&self, cluster_ini: &ClusterIni) -> bool {
        self.name == cluster_ini.cluster_name
            && u64::try_from(self.max_connections).ok() == Some(cluster_ini.max_players)
            && self.mode == cluster_ini.game_mode
            && self.password != cluster_ini.cluster_password.is_empty()
    }
}

fn string_at(row: &[Value], index: usize) -> String {
    row.get(index)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn i64_at(row: &[Value], index: usize) -> i64 {
    row.get(index)
        .and_then(|value| {
            value
                .as_i64()
                .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
        })
        .unwrap_or_default()
}

fn bool_at(row: &[Value], index: usize) -> bool {
    row.get(index)
        .and_then(|value| {
            value.as_bool().or_else(|| {
                value
                    .as_i64()
                    .map(|value| value != 0)
                    .or_else(|| value.as_str().and_then(parse_boolish))
            })
        })
        .unwrap_or(false)
}

fn parse_boolish(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Some(true),
        "false" | "0" | "no" => Some(false),
        _ => None,
    }
}
