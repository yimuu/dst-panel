//! DB-backed statistics handlers migrated from Go raw SQL endpoints.

use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;

use crate::{
    domain::statistics::model::RegenerateRecord,
    domain::statistics::repository::{
        ActiveUserAxis, DateWindow, RoleRateStatistics, StatisticsRepository, TopStatistics,
    },
    web::app::AppState,
    web::error::AppResult,
    web::handlers::{legacy_success, repository_error},
    web::response::LoginResponse,
};

/// Query parameters shared by time-windowed statistics routes.
#[derive(Debug, Deserialize)]
pub(crate) struct DateRangeQuery {
    #[serde(rename = "startDate")]
    start_date: Option<String>,
    #[serde(rename = "endDate")]
    end_date: Option<String>,
}

/// Query parameters for `/api/statistics/active/user`.
#[derive(Debug, Deserialize)]
pub(crate) struct ActiveUserQuery {
    unit: Option<String>,
    #[serde(rename = "startDate")]
    start_date: Option<String>,
    #[serde(rename = "endDate")]
    end_date: Option<String>,
}

/// Query parameters for top-N statistics routes.
#[derive(Debug, Deserialize)]
pub(crate) struct TopQuery {
    #[serde(rename = "N")]
    n: Option<String>,
    #[serde(rename = "startDate")]
    start_date: Option<String>,
    #[serde(rename = "endDate")]
    end_date: Option<String>,
}

/// Query parameters for the regenerate route.
#[derive(Debug, Deserialize)]
pub(crate) struct LimitQuery {
    #[serde(rename = "N")]
    n: Option<String>,
}

pub(crate) async fn active_user_handler(
    State(state): State<AppState>,
    Query(query): Query<ActiveUserQuery>,
) -> AppResult<Json<LoginResponse<ActiveUserAxis>>> {
    let repository = StatisticsRepository::new(state.db);
    let window = DateWindow::parse(query.start_date.as_deref(), query.end_date.as_deref());
    let axis = if query.unit.as_deref() == Some("DAY") {
        repository
            .active_user_day(&window)
            .await
            .map_err(|error| repository_error("count active users", error))?
    } else {
        repository.empty_active_axis()
    };
    tracing::debug!(
        unit = query.unit.as_deref(),
        "computed active-user statistics"
    );
    Ok(Json(legacy_success(axis)))
}

pub(crate) async fn top_death_handler(
    State(state): State<AppState>,
    Query(query): Query<TopQuery>,
) -> AppResult<Json<LoginResponse<Vec<TopStatistics>>>> {
    let (repository, window, limit) = repository_window_limit(state, &query);
    let data = repository
        .top_deaths(&window, &limit)
        .await
        .map_err(|error| repository_error("count top deaths", error))?;
    tracing::debug!(
        limit = limit.as_str(),
        returned = data.len(),
        "computed top death statistics"
    );
    Ok(Json(legacy_success(data)))
}

pub(crate) async fn top_login_handler(
    State(state): State<AppState>,
    Query(query): Query<TopQuery>,
) -> AppResult<Json<LoginResponse<Vec<TopStatistics>>>> {
    let (repository, window, limit) = repository_window_limit(state, &query);
    let data = repository
        .top_login(&window, &limit)
        .await
        .map_err(|error| repository_error("count top logins", error))?;
    tracing::debug!(
        limit = limit.as_str(),
        returned = data.len(),
        "computed top login statistics"
    );
    Ok(Json(legacy_success(data)))
}

pub(crate) async fn top_active_handler(
    State(state): State<AppState>,
    Query(query): Query<TopQuery>,
) -> AppResult<Json<LoginResponse<Vec<TopStatistics>>>> {
    let (repository, window, limit) = repository_window_limit(state, &query);
    let data = repository
        .top_active(&window, &limit)
        .await
        .map_err(|error| repository_error("count top active users", error))?;
    tracing::debug!(
        limit = limit.as_str(),
        returned = data.len(),
        "computed top active statistics"
    );
    Ok(Json(legacy_success(data)))
}

pub(crate) async fn role_rate_handler(
    State(state): State<AppState>,
    Query(query): Query<DateRangeQuery>,
) -> AppResult<Json<LoginResponse<Vec<RoleRateStatistics>>>> {
    let repository = StatisticsRepository::new(state.db);
    let window = DateWindow::parse(query.start_date.as_deref(), query.end_date.as_deref());
    let data = repository
        .role_rate(&window)
        .await
        .map_err(|error| repository_error("count role rates", error))?;
    tracing::debug!(returned = data.len(), "computed role-rate statistics");
    Ok(Json(legacy_success(data)))
}

pub(crate) async fn regenerate_handler(
    State(state): State<AppState>,
    Query(query): Query<LimitQuery>,
) -> AppResult<Json<LoginResponse<Vec<RegenerateRecord>>>> {
    let repository = StatisticsRepository::new(state.db);
    let limit = go_raw_limit(query.n.as_deref());
    let data = repository
        .regenerates(&limit)
        .await
        .map_err(|error| repository_error("list regenerates", error))?;
    tracing::debug!(
        limit = limit.as_str(),
        returned = data.len(),
        "listed regenerate statistics"
    );
    Ok(Json(legacy_success(data)))
}

fn repository_window_limit(
    state: AppState,
    query: &TopQuery,
) -> (StatisticsRepository, DateWindow, String) {
    (
        StatisticsRepository::new(state.db),
        DateWindow::parse(query.start_date.as_deref(), query.end_date.as_deref()),
        go_raw_limit(query.n.as_deref()),
    )
}

/// Preserves Go's `ctx.Query("N")` behavior for raw SQLite LIMIT bindings.
///
/// Missing `N` becomes an empty string, which SQLite rejects and the Go handler
/// then ignores; the repository maps that ignored error to an empty result.
fn go_raw_limit(limit: Option<&str>) -> String {
    limit.unwrap_or_default().to_owned()
}
