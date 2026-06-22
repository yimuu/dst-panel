//! Steam DST news endpoint.
//!
//! Go fetches Steam's JSON feed on every request, filters items with
//! `feed_type == 1`, and returns only title, URL, and date in the legacy
//! response envelope. Rust preserves that shape while routing the network call
//! through the fakeable HTTP client.

use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use serde_json::Value;

use crate::{infra::http_client::HttpRequest, web::app::AppState, web::handlers::legacy_success};

const STEAM_DST_NEWS_URL: &str = "https://steamcommunity-a.akamaihd.net/news/newsforapp/v0002/?appid=322330&count=10&maxlength=300&format=json";

#[derive(Debug, Serialize)]
struct SteamNewsTask {
    title: String,
    url: String,
    date: f64,
}

/// Returns filtered DST Steam news.
pub(crate) async fn dst_news_handler(State(state): State<AppState>) -> Response {
    let request = HttpRequest::new("GET", STEAM_DST_NEWS_URL);
    let response = match state.http_client.send(request).await {
        Ok(response) if response.status == 200 => response,
        Ok(response) => {
            tracing::warn!(
                status = response.status,
                body_len = response.body.len(),
                "Steam news upstream returned non-200 status"
            );
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        }
        Err(error) => {
            tracing::warn!(error = %error, "Steam news upstream request failed");
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        }
    };

    let body: Value = match serde_json::from_slice(&response.body) {
        Ok(body) => body,
        Err(error) => {
            tracing::warn!(
                body_len = response.body.len(),
                error = %error,
                "Steam news upstream response was not valid JSON"
            );
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    let tasks = filtered_tasks(&body);
    tracing::info!(task_count = tasks.len(), "served filtered Steam news");
    Json(legacy_success(tasks)).into_response()
}

fn filtered_tasks(body: &Value) -> Vec<SteamNewsTask> {
    body.get("appnews")
        .and_then(|appnews| appnews.get("newsitems"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| item.get("feed_type").and_then(Value::as_i64) == Some(1))
        .filter_map(|item| {
            Some(SteamNewsTask {
                title: item.get("title")?.as_str()?.to_owned(),
                url: item.get("url")?.as_str()?.to_owned(),
                date: item.get("date")?.as_f64()?,
            })
        })
        .collect()
}
