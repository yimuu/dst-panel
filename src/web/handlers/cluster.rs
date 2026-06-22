//! Cluster CRUD handlers for `/api/cluster`.
//!
//! Go's create route persists the cluster row, installs the game when
//! `force_install_dir` is missing, and then initializes world files. Rust
//! preserves that install-on-create behavior through the argv-based command
//! runner boundary, while start/stop side effects remain in the game lifecycle
//! routes.

use axum::{
    Json,
    extract::{Query, State},
};
use chrono::{DateTime, Utc};
use futures_util::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    domain::cluster::model::{ClusterRecord, NewCluster},
    domain::cluster::repository::ClusterRepository,
    domain::cluster::{install as cluster_install, runtime as cluster_runtime},
    dst,
    validation::validate_cluster_name,
    web::app::AppState,
    web::error::{AppError, AppResult},
    web::handlers::{legacy_empty_success, legacy_success, repository_error},
    web::response::LoginResponse,
};

const CLUSTER_RUNTIME_CONCURRENCY: usize = 4;

/// Query parameters accepted by Go's cluster list route.
#[derive(Debug, Deserialize)]
pub(crate) struct ClusterListQuery {
    #[serde(rename = "clusterName")]
    cluster_name: Option<String>,
    page: Option<i64>,
    size: Option<i64>,
}

/// Query parameters accepted by Go's cluster delete route.
#[derive(Debug, Deserialize)]
pub(crate) struct DeleteClusterQuery {
    id: Option<i64>,
}

/// Go-compatible cluster update request. The Go implementation only applies
/// `description`, despite binding the full model.
#[derive(Debug, Deserialize)]
pub(crate) struct UpdateClusterRequest {
    #[serde(rename = "ID")]
    uppercase_id: Option<i64>,
    id: Option<i64>,
    description: Option<String>,
}

impl UpdateClusterRequest {
    fn id(&self) -> Option<i64> {
        self.uppercase_id.or(self.id)
    }
}

/// Go `vo.Page` response shape.
#[derive(Debug, Serialize)]
pub(crate) struct Page<T> {
    data: Vec<T>,
    total: i64,
    #[serde(rename = "totalPages")]
    total_pages: i64,
    page: i64,
    size: i64,
}

/// Runtime cluster list item matching the Go `ClusterVO` shape.
#[derive(Debug, Serialize)]
pub(crate) struct ClusterVo {
    #[serde(rename = "ID")]
    id: i64,
    #[serde(rename = "CreatedAt")]
    created_at: Option<DateTime<Utc>>,
    #[serde(rename = "UpdatedAt")]
    updated_at: Option<DateTime<Utc>>,
    #[serde(rename = "clusterName")]
    cluster_name: String,
    description: String,
    #[serde(rename = "steamcmd")]
    steam_cmd: String,
    force_install_dir: String,
    backup: String,
    mod_download_path: String,
    uuid: String,
    beta: i64,
    master: bool,
    caves: bool,
    #[serde(rename = "rowId")]
    row_id: String,
    connected: i64,
    #[serde(rename = "maxConnections")]
    max_connections: i64,
    mode: String,
    mods: i64,
    season: String,
    password: String,
    region: String,
}

impl ClusterVo {
    fn from_record_and_runtime(
        record: ClusterRecord,
        runtime: cluster_runtime::ClusterRuntimeInfo,
    ) -> Self {
        Self {
            id: record.id,
            created_at: record.created_at,
            updated_at: record.updated_at,
            cluster_name: record.cluster_name,
            description: record.description,
            steam_cmd: record.steam_cmd,
            force_install_dir: record.force_install_dir,
            backup: record.backup,
            mod_download_path: record.mod_download_path,
            uuid: record.uuid,
            beta: record.beta,
            master: runtime.master,
            caves: runtime.caves,
            row_id: runtime.row_id,
            connected: runtime.connected,
            max_connections: runtime.max_connections,
            mode: runtime.mode,
            mods: runtime.mods,
            season: runtime.season,
            password: String::new(),
            region: runtime.region,
        }
    }
}

pub(crate) async fn list_handler(
    State(state): State<AppState>,
    Query(query): Query<ClusterListQuery>,
) -> AppResult<Json<LoginResponse<Page<ClusterVo>>>> {
    let page = query.page.unwrap_or(1).max(1);
    let size = match query.size.unwrap_or(10) {
        value if value <= 0 => 10,
        value => value,
    };
    let filter = query
        .cluster_name
        .as_deref()
        .filter(|value| !value.is_empty());

    tracing::debug!(page, size, cluster_name_filter = filter, "listing clusters");
    let repository = ClusterRepository::new(state.db);
    let total = repository
        .count_filtered(filter)
        .await
        .map_err(|error| repository_error("count clusters", error))?;
    let clusters = repository
        .list_filtered_page(filter, page, size)
        .await
        .map_err(|error| repository_error("list clusters", error))?;
    let total_pages = if total == 0 {
        0
    } else {
        (total + size - 1) / size
    };

    let snapshots = match state.process_snapshot_provider.snapshots() {
        Ok(snapshots) => snapshots,
        Err(error) => {
            tracing::warn!(
                error = %error,
                "failed to collect process snapshots for cluster list; using empty snapshot set"
            );
            Vec::new()
        }
    };
    tracing::debug!(
        snapshot_count = snapshots.len(),
        cluster_count = clusters.len(),
        "collected shared process snapshots for cluster list"
    );

    let root_path = state.root_path.clone();
    let http_client = state.http_client.clone();
    let lobby_rows = cluster_runtime::collect_lobby_rows_for_clusters(
        &root_path,
        &clusters,
        http_client.as_ref(),
    )
    .await;
    let snapshots = snapshots.as_slice();
    let lobby_rows = &lobby_rows;
    let data = stream::iter(clusters.into_iter().map(|cluster| {
        let root_path = root_path.clone();
        async move {
            let runtime =
                cluster_runtime::collect_for_cluster(&root_path, &cluster, snapshots, lobby_rows)
                    .await;
            ClusterVo::from_record_and_runtime(cluster, runtime)
        }
    }))
    .buffered(CLUSTER_RUNTIME_CONCURRENCY)
    .collect::<Vec<_>>()
    .await;

    Ok(Json(legacy_success(Page {
        data,
        total,
        total_pages,
        page,
        size,
    })))
}

pub(crate) async fn create_handler(
    State(state): State<AppState>,
    Json(mut request): Json<NewCluster>,
) -> AppResult<Json<LoginResponse<Value>>> {
    let cluster_name = validate_cluster_name(&request.cluster_name)
        .map_err(|error| AppError::bad_request(error.to_string()))?;
    if request.steam_cmd.trim().is_empty() || request.force_install_dir.trim().is_empty() {
        tracing::warn!("rejected cluster create with missing required install paths");
        return Err(AppError::bad_request(
            "clusterName, steamcmd and force_install_dir are required",
        ));
    }
    if request.uuid.trim().is_empty() {
        request.uuid = dst::generate_uuid_v4().map_err(AppError::from)?;
    }

    let token = state.config.token.as_deref().unwrap_or_default().to_owned();
    let steam_cmd = request.steam_cmd.clone();
    let force_install_dir = request.force_install_dir.clone();
    let backup = request.backup.clone();
    let mod_download_path = request.mod_download_path.clone();

    let repository = ClusterRepository::new(state.db);
    let created = repository
        .create(request)
        .await
        .map_err(|error| repository_error("create cluster", error))?;

    if let Err(error) = cluster_install::install_dedicated_server_if_missing(
        state.command_runner.as_ref(),
        cluster_name.as_str(),
        &steam_cmd,
        &force_install_dir,
    )
    .await
    {
        tracing::error!(
            id = created.id,
            cluster_name = cluster_name.as_str(),
            error = %error,
            "rolling back cluster record after DST install failed"
        );
        if let Err(delete_error) = repository.hard_delete_for_rollback(created.id).await {
            tracing::error!(
                id = created.id,
                error = %delete_error,
                "failed to remove cluster row after DST install failure"
            );
        }
        return Err(error);
    }

    let file_result = (|| -> std::io::Result<()> {
        // Keep the durable row and filesystem skeleton in sync as closely as
        // possible without wrapping filesystem calls in the SQLite transaction.
        if !backup.is_empty() {
            dst::safe_ensure_configured_dir(&state.root_path, &backup)?;
        }
        if !mod_download_path.is_empty() {
            dst::safe_ensure_configured_dir(&state.root_path, &mod_download_path)?;
        }
        dst::init_cluster_files(&state.root_path, cluster_name.as_str(), &token)?;
        Ok(())
    })();
    if let Err(error) = file_result {
        tracing::error!(
            id = created.id,
            cluster_name = cluster_name.as_str(),
            error = %error,
            "rolling back cluster record after file initialization failed"
        );
        if let Err(delete_error) = repository.hard_delete_for_rollback(created.id).await {
            tracing::error!(
                id = created.id,
                error = %delete_error,
                "failed to remove cluster row after file initialization failure"
            );
        }
        return Err(file_error("initialize cluster files")(error));
    }

    tracing::info!(
        cluster_name = cluster_name.as_str(),
        "created cluster record and file skeleton without starting DST process"
    );
    Ok(Json(legacy_empty_success()))
}

pub(crate) async fn update_handler(
    State(state): State<AppState>,
    Json(request): Json<UpdateClusterRequest>,
) -> AppResult<Json<LoginResponse<Value>>> {
    let id = request
        .id()
        .filter(|id| *id > 0)
        .ok_or_else(|| AppError::bad_request("cluster ID is required"))?;
    let description = request.description.unwrap_or_default();

    let repository = ClusterRepository::new(state.db);
    repository
        .update_description(id, &description)
        .await
        .map_err(|error| repository_error("update cluster", error))?;

    tracing::info!(id, "updated cluster description");
    Ok(Json(legacy_empty_success()))
}

pub(crate) async fn delete_handler(
    State(state): State<AppState>,
    Query(query): Query<DeleteClusterQuery>,
) -> AppResult<Json<LoginResponse<Value>>> {
    let id = query
        .id
        .filter(|id| *id > 0)
        .ok_or_else(|| AppError::bad_request("cluster id is required"))?;
    let repository = ClusterRepository::new(state.db);
    repository
        .delete(id)
        .await
        .map_err(|error| repository_error("delete cluster", error))?;

    tracing::info!(id, "deleted cluster");
    Ok(Json(legacy_empty_success()))
}

fn file_error(
    operation: &'static str,
) -> impl FnOnce(std::io::Error) -> AppError + Copy + Send + Sync + 'static {
    move |error| {
        if error.kind() == std::io::ErrorKind::InvalidInput {
            AppError::bad_request(error.to_string())
        } else {
            tracing::error!(operation, error = %error, "cluster file operation failed");
            AppError::internal(operation)
        }
    }
}
