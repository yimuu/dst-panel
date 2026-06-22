//! Repository for Go-compatible player-log statistics queries.

pub mod player_log;

use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use sqlx::FromRow;

use crate::{domain::statistics::model::RegenerateRecord, infra::db::SqlitePool};

const JOIN_ACTION: &str = "[JoinAnnouncement]";
const DEATH_ACTION: &str = "[DeathAnnouncement]";
const GO_DATE_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.3fZ";
const CHINA_OFFSET_MILLIS: i64 = 8 * 60 * 60 * 1000;

/// Time window parsed from Go's `startDate` and `endDate` query parameters.
#[derive(Debug, Clone)]
pub struct DateWindow {
    pub start: String,
    pub end: String,
    pub start_day: NaiveDate,
    pub end_day: NaiveDate,
}

impl DateWindow {
    /// Parses a Go-style date window. Missing or malformed values become the
    /// Go zero date for SQL purposes; the DAY axis stays empty if either side
    /// cannot form a meaningful calendar range.
    pub fn parse(start: Option<&str>, end: Option<&str>) -> Self {
        let start = parse_go_datetime(start);
        let end = parse_go_datetime(end);
        let start_day = start
            .map(|date| date.date_naive())
            .unwrap_or(NaiveDate::MIN);
        let end_day = end.map(|date| date.date_naive()).unwrap_or(NaiveDate::MIN);
        Self {
            start: start
                .map(|date| date.format(GO_DATE_FORMAT).to_string())
                .unwrap_or_else(|| "0001-01-01T00:00:00.000Z".to_owned()),
            end: end
                .map(|date| date.format(GO_DATE_FORMAT).to_string())
                .unwrap_or_else(|| "0001-01-01T00:00:00.000Z".to_owned()),
            start_day,
            end_day,
        }
    }
}

/// Axis data returned by `/api/statistics/active/user`.
#[derive(Debug, Clone, Serialize)]
pub struct ActiveUserAxis {
    pub x: Option<Vec<i64>>,
    pub y1: Option<Vec<i64>>,
    pub y2: Option<Vec<i64>>,
}

/// Top-statistics row returned by the Go API.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct TopStatistics {
    pub id: i64,
    pub count: i64,
    pub name: String,
    #[serde(rename = "kuId")]
    pub ku_id: String,
    #[serde(rename = "steamId")]
    pub steam_id: String,
    pub role: String,
    #[serde(rename = "actionDesc")]
    pub action_desc: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

/// Role-rate row returned by the Go API.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct RoleRateStatistics {
    pub role: String,
    pub count: i64,
}

#[derive(Debug, Clone, FromRow)]
struct CountByDate {
    count: i64,
    date: String,
}

/// SQLite repository for the statistics endpoints.
pub struct StatisticsRepository {
    pool: SqlitePool,
}

impl StatisticsRepository {
    /// Creates a repository backed by the shared SQLite pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Counts distinct active users and login events on the daily axis.
    pub async fn active_user_day(
        &self,
        window: &DateWindow,
    ) -> Result<ActiveUserAxis, sqlx::Error> {
        let distinct_users = sqlx::query_as::<_, CountByDate>(
            "SELECT COUNT(DISTINCT name) AS count, strftime('%Y-%m-%dT00:00:00.000Z', created_at) AS date \
             FROM player_logs \
             WHERE datetime(created_at) BETWEEN datetime(?) AND datetime(?) \
             GROUP BY strftime('%m', created_at), strftime('%d', created_at)",
        )
        .bind(&window.start)
        .bind(&window.end)
        .fetch_all(&self.pool)
        .await?;
        let joins = sqlx::query_as::<_, CountByDate>(
            "SELECT COUNT(name) AS count, strftime('%Y-%m-%dT00:00:00.000Z', created_at) AS date \
             FROM player_logs \
             WHERE datetime(created_at) BETWEEN datetime(?) AND datetime(?) AND action LIKE ? \
             GROUP BY strftime('%m', created_at), strftime('%d', created_at)",
        )
        .bind(&window.start)
        .bind(&window.end)
        .bind(JOIN_ACTION)
        .fetch_all(&self.pool)
        .await?;

        let mut axis = ActiveUserAxis {
            x: Some(Vec::new()),
            y1: Some(Vec::new()),
            y2: Some(Vec::new()),
        };
        for stamp in day_stamps(window) {
            axis.x.as_mut().expect("day axis initialized").push(stamp);
            axis.y1
                .as_mut()
                .expect("day axis initialized")
                .push(count_for_stamp(stamp, &distinct_users));
            axis.y2
                .as_mut()
                .expect("day axis initialized")
                .push(count_for_stamp(stamp, &joins));
        }
        Ok(axis)
    }

    /// Returns Go's empty axis for MONTH or unknown unit branches.
    pub fn empty_active_axis(&self) -> ActiveUserAxis {
        ActiveUserAxis {
            x: None,
            y1: None,
            y2: None,
        }
    }

    /// Top death statistics.
    pub async fn top_deaths(
        &self,
        window: &DateWindow,
        limit: &str,
    ) -> Result<Vec<TopStatistics>, sqlx::Error> {
        self.top_from_player_logs(window, limit, DEATH_ACTION).await
    }

    /// Top active statistics.
    pub async fn top_active(
        &self,
        window: &DateWindow,
        limit: &str,
    ) -> Result<Vec<TopStatistics>, sqlx::Error> {
        let result = sqlx::query_as::<_, TopStatistics>(
            "SELECT MAX(id) AS id, COUNT(name) AS count, COALESCE(name, '') AS name, \
                    COALESCE(ku_id, '') AS ku_id, COALESCE(steam_id, '') AS steam_id, \
                    COALESCE(role, '') AS role, COALESCE(action_desc, '') AS action_desc, \
                    COALESCE(created_at, '') AS created_at \
             FROM player_logs \
             WHERE datetime(created_at) BETWEEN datetime(?) AND datetime(?) AND action LIKE ? \
             GROUP BY name ORDER BY COUNT(id) DESC LIMIT ?",
        )
        .bind(&window.start)
        .bind(&window.end)
        .bind(JOIN_ACTION)
        .bind(limit)
        .fetch_all(&self.pool)
        .await;
        ignore_go_raw_limit_error(result)
    }

    /// Top login statistics, preserving Go's name-only join to `connects`.
    pub async fn top_login(
        &self,
        window: &DateWindow,
        limit: &str,
    ) -> Result<Vec<TopStatistics>, sqlx::Error> {
        let result = sqlx::query_as::<_, TopStatistics>(
            "SELECT MAX(p.id) AS id, COUNT(p.name) AS count, p.name AS name, \
                    COALESCE(c.ku_id, '') AS ku_id, COALESCE(c.steam_id, '') AS steam_id, \
                    COALESCE(p.role, '') AS role, COALESCE(p.action_desc, '') AS action_desc, \
                    COALESCE(p.created_at, '') AS created_at \
             FROM player_logs p \
             LEFT JOIN connects c ON p.name = c.name \
             WHERE datetime(p.created_at) BETWEEN datetime(?) AND datetime(?) AND p.action LIKE ? \
             GROUP BY p.name ORDER BY COUNT(p.id) DESC LIMIT ?",
        )
        .bind(&window.start)
        .bind(&window.end)
        .bind(JOIN_ACTION)
        .bind(limit)
        .fetch_all(&self.pool)
        .await;
        ignore_go_raw_limit_error(result)
    }

    /// Counts distinct names by role.
    pub async fn role_rate(
        &self,
        window: &DateWindow,
    ) -> Result<Vec<RoleRateStatistics>, sqlx::Error> {
        sqlx::query_as::<_, RoleRateStatistics>(
            "SELECT role AS role, COUNT(DISTINCT name) AS count \
             FROM player_logs \
             WHERE role != '' AND datetime(created_at) BETWEEN datetime(?) AND datetime(?) \
             GROUP BY role",
        )
        .bind(&window.start)
        .bind(&window.end)
        .fetch_all(&self.pool)
        .await
    }

    /// Returns the last `limit` regeneration records. Go's Raw SQL does not
    /// filter soft-deleted rows here, so Rust preserves that behavior.
    pub async fn regenerates(&self, limit: &str) -> Result<Vec<RegenerateRecord>, sqlx::Error> {
        let result = sqlx::query_as::<_, RegenerateRecord>(
            "SELECT id, created_at, updated_at, deleted_at, cluster_name \
             FROM regenerates ORDER BY created_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await;
        ignore_go_raw_limit_error(result)
    }

    async fn top_from_player_logs(
        &self,
        window: &DateWindow,
        limit: &str,
        action: &'static str,
    ) -> Result<Vec<TopStatistics>, sqlx::Error> {
        let result = sqlx::query_as::<_, TopStatistics>(
            "SELECT MAX(id) AS id, COUNT(id) AS count, COALESCE(name, '') AS name, \
                    COALESCE(ku_id, '') AS ku_id, COALESCE(steam_id, '') AS steam_id, \
                    COALESCE(role, '') AS role, COALESCE(action_desc, '') AS action_desc, \
                    COALESCE(created_at, '') AS created_at \
             FROM player_logs \
             WHERE datetime(created_at) BETWEEN datetime(?) AND datetime(?) AND action LIKE ? \
             GROUP BY name ORDER BY COUNT(id) DESC LIMIT ?",
        )
        .bind(&window.start)
        .bind(&window.end)
        .bind(action)
        .bind(limit)
        .fetch_all(&self.pool)
        .await;
        ignore_go_raw_limit_error(result)
    }
}

fn parse_go_datetime(value: Option<&str>) -> Option<DateTime<Utc>> {
    let value = value?;
    DateTime::parse_from_str(value, GO_DATE_FORMAT)
        .or_else(|_| DateTime::parse_from_rfc3339(value))
        .ok()
        .map(|date| date.with_timezone(&Utc))
}

fn day_stamps(window: &DateWindow) -> Vec<i64> {
    if window.start_day == NaiveDate::MIN || window.end_day == NaiveDate::MIN {
        return Vec::new();
    }
    let mut day = window.start_day;
    let mut stamps = Vec::new();
    while day <= window.end_day {
        let Some(midnight) = day.and_hms_opt(0, 0, 0) else {
            break;
        };
        stamps.push(midnight.and_utc().timestamp_millis() - CHINA_OFFSET_MILLIS);
        let Some(next_day) = day.succ_opt() else {
            break;
        };
        day = next_day;
    }
    stamps
}

fn count_for_stamp(stamp: i64, rows: &[CountByDate]) -> i64 {
    rows.iter()
        .find(|row| date_stamp(row.date.as_str()) == Some(stamp))
        .map(|row| row.count)
        .unwrap_or(0)
}

fn date_stamp(value: &str) -> Option<i64> {
    parse_go_datetime(Some(value)).map(|date| {
        date.date_naive()
            .and_hms_opt(0, 0, 0)
            .expect("valid date at midnight")
            .and_utc()
            .timestamp_millis()
            - CHINA_OFFSET_MILLIS
    })
}

fn ignore_go_raw_limit_error<T>(
    result: Result<Vec<T>, sqlx::Error>,
) -> Result<Vec<T>, sqlx::Error> {
    match result {
        Err(error) if is_sqlite_datatype_mismatch(&error) => Ok(Vec::new()),
        other => other,
    }
}

fn is_sqlite_datatype_mismatch(error: &sqlx::Error) -> bool {
    match error {
        sqlx::Error::Database(database_error) => {
            database_error.code().as_deref() == Some("20")
                || database_error.message().contains("datatype mismatch")
        }
        _ => false,
    }
}
