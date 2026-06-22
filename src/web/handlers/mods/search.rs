//! Steam workshop search route handlers.

use axum::{
    Json,
    extract::{Query, State},
};
use serde_json::{Value, json};

use crate::web::{app::AppState, error::AppResult, handlers::legacy_success};

use super::{
    dto::SearchQuery,
    steam_api::{
        STEAM_DETAIL_LANG, SearchImageMode, SearchVoteMode, fetch_steam_details, query_steam_files,
        search_item_from_detail,
    },
};

/// Searches Steam workshop metadata using the Go query shape.
pub(crate) async fn search_handler(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> AppResult<Json<crate::web::response::LoginResponse<Value>>> {
    let page = query.page.unwrap_or(1).max(1);
    let size = query.size.unwrap_or(10).max(1);
    let lang = query.lang.unwrap_or_else(|| "zh".to_owned());
    let text = query.text.unwrap_or_default();

    let data = if text.parse::<i64>().is_ok() {
        let mut items = Vec::new();
        if let Some(detail) = fetch_steam_details(&state, &[text.as_str()], STEAM_DETAIL_LANG)
            .await?
            .pop()
            && detail.consumer_appid == 322330.0
        {
            items.push(search_item_from_detail(
                &detail,
                SearchImageMode::Suffixed,
                SearchVoteMode::Zero,
            ));
        }
        json!({
            "page": 1,
            "size": 1,
            "total": 1,
            "totalPage": 1,
            "data": items,
        })
    } else {
        query_steam_files(&state, &text, page, size, &lang).await?
    };

    tracing::info!(
        page,
        size,
        numeric_query = text.parse::<i64>().is_ok(),
        "served mod search"
    );
    Ok(Json(legacy_success(data)))
}
