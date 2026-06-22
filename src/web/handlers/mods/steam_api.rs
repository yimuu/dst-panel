//! Steam Web API helpers for mod routes.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::{
    infra::http_client::HttpRequest,
    web::{
        app::AppState,
        error::{AppError, AppResult},
    },
};

const STEAM_DETAILS_URL: &str = "http://api.steampowered.com/IPublishedFileService/GetDetails/v1/";
const STEAM_QUERY_URL: &str = "http://api.steampowered.com/IPublishedFileService/QueryFiles/v1/";
pub(super) const STEAM_DETAIL_LANG: &str = "zh";
const IMAGE_SUFFIX: &str =
    "?imw=64&imh=64&ima=fit&impolicy=Letterbox&imcolor=%23000000&letterbox=true";
pub(super) async fn fetch_steam_details(
    state: &AppState,
    ids: &[&str],
    lang: &str,
) -> AppResult<Vec<SteamDetail>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let api_key = steam_api_key(state)?;
    let mut params = vec![
        ("key".to_owned(), api_key),
        (
            "language".to_owned(),
            if lang == "zh" { "6" } else { "" }.to_owned(),
        ),
    ];
    for (index, id) in ids.iter().enumerate() {
        params.push((format!("publishedfileids[{index}]"), (*id).to_owned()));
    }
    let request = HttpRequest::new(
        "GET",
        format!("{STEAM_DETAILS_URL}?{}", encode_query(&params)),
    );
    let response = state.http_client.send(request).await.map_err(|error| {
        tracing::warn!(error = %error, "Steam details request failed");
        AppError::bad_request("steam details request failed")
    })?;
    if response.status != 200 {
        tracing::warn!(status = response.status, "Steam details returned non-200");
        return Err(AppError::bad_request("steam details request failed"));
    }
    let body: SteamDetailsResponse = serde_json::from_slice(&response.body).map_err(|error| {
        tracing::warn!(error = %error, "Steam details response was malformed");
        AppError::bad_request("steam details response was malformed")
    })?;
    Ok(body.response.publishedfiledetails)
}

pub(super) async fn query_steam_files(
    state: &AppState,
    text: &str,
    page: i64,
    size: i64,
    lang: &str,
) -> AppResult<Value> {
    let api_key = steam_api_key(state)?;
    let params = vec![
        ("appid".to_owned(), "322330".to_owned()),
        ("key".to_owned(), api_key),
        (
            "language".to_owned(),
            if lang == "zh" { "6" } else { "" }.to_owned(),
        ),
        ("numperpage".to_owned(), size.to_string()),
        ("page".to_owned(), page.to_string()),
        ("return_children".to_owned(), "true".to_owned()),
        ("return_tags".to_owned(), "true".to_owned()),
        ("return_vote_data".to_owned(), "true".to_owned()),
        ("search_text".to_owned(), text.to_owned()),
    ];
    let request = HttpRequest::new(
        "GET",
        format!("{STEAM_QUERY_URL}?{}", encode_query(&params)),
    );
    let response = state.http_client.send(request).await.map_err(|error| {
        tracing::warn!(error = %error, "Steam query request failed");
        AppError::bad_request("steam query request failed")
    })?;
    if response.status != 200 {
        return Err(AppError::bad_request("steam query request failed"));
    }
    let body: SteamQueryResponse = serde_json::from_slice(&response.body).map_err(|error| {
        tracing::warn!(error = %error, "Steam query response was malformed");
        AppError::bad_request("steam query response was malformed")
    })?;
    let total_page = if size <= 0 {
        0
    } else {
        ((body.response.total as f64) / (size as f64)).ceil() as i64
    };
    Ok(json!({
        "page": page,
        "size": size,
        "total": body.response.total,
        "totalPage": total_page,
            "data": body.response.publishedfiledetails.iter().map(|detail| {
            search_item_from_detail(detail, SearchImageMode::Raw, SearchVoteMode::Calculated)
        }).collect::<Vec<_>>(),
    }))
}

pub(super) fn search_item_from_detail(
    detail: &SteamDetail,
    image_mode: SearchImageMode,
    vote_mode: SearchVoteMode,
) -> Value {
    let mut object = Map::new();
    object.insert("id".to_owned(), json!(detail.publishedfileid));
    object.insert("name".to_owned(), json!(detail.title));
    object.insert("author".to_owned(), json!(author_url(&detail.creator)));
    object.insert("desc".to_owned(), json!(detail.file_description));
    object.insert("time".to_owned(), json!(detail.time_updated as i64));
    object.insert("sub".to_owned(), json!(detail.subscriptions as i64));
    object.insert(
        "img".to_owned(),
        json!(match image_mode {
            SearchImageMode::Raw => detail.preview_url.clone(),
            SearchImageMode::Suffixed => image_with_suffix(&detail.preview_url),
        }),
    );
    // Go's search DTO serializes these fields, but SearchModList never fills
    // them. Keep the zero values instead of reusing the richer detail model.
    object.insert("file_url".to_owned(), json!(""));
    object.insert("v".to_owned(), json!(""));
    object.insert("last_time".to_owned(), json!(0.0));
    object.insert("consumer_appid".to_owned(), json!(0.0));
    object.insert("creator_appid".to_owned(), json!(0.0));
    let (star, num) = match vote_mode {
        SearchVoteMode::Zero => (0, 0),
        SearchVoteMode::Calculated => (
            (detail.vote_data.score * 5.0) as i64 + 1,
            (detail.vote_data.votes_up + detail.vote_data.votes_down) as i64,
        ),
    };
    object.insert("vote".to_owned(), json!({"star": star, "num": num}));
    if !detail.children.is_empty() {
        object.insert(
            "child".to_owned(),
            json!(
                detail
                    .children
                    .iter()
                    .map(|child| child.publishedfileid.clone())
                    .collect::<Vec<_>>()
            ),
        );
    }
    Value::Object(object)
}
pub(super) fn author_url(creator: &str) -> String {
    if creator.is_empty() {
        String::new()
    } else {
        format!("https://steamcommunity.com/profiles/{creator}/?xml=1")
    }
}

pub(super) fn image_with_suffix(preview_url: &str) -> String {
    format!("{preview_url}{IMAGE_SUFFIX}")
}

pub(super) fn version_from_tags(tags: &[SteamTag]) -> String {
    tags.iter()
        .find_map(|tag| tag.tag.strip_prefix("version:"))
        .unwrap_or_default()
        .to_owned()
}

fn steam_api_key(state: &AppState) -> AppResult<String> {
    state
        .config
        .steam_api_key
        .as_deref()
        .map(str::trim)
        .filter(|api_key| !api_key.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            tracing::warn!("Steam API key is not configured");
            AppError::bad_request("steam API key is not configured")
        })
}

fn encode_query(params: &[(String, String)]) -> String {
    let mut params = params.to_vec();
    params.sort_by(|left, right| left.0.cmp(&right.0));
    params
        .iter()
        .map(|(key, value)| {
            format!(
                "{}={}",
                encode_query_component(key),
                encode_query_component(value)
            )
        })
        .collect::<Vec<_>>()
        .join("&")
}

fn encode_query_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char)
            }
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}
#[derive(Debug, Clone, Copy)]
pub(super) enum SearchImageMode {
    Raw,
    Suffixed,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum SearchVoteMode {
    Calculated,
    Zero,
}
#[derive(Debug, Deserialize)]
struct SteamDetailsResponse {
    response: SteamDetailsBody,
}

#[derive(Debug, Deserialize)]
struct SteamDetailsBody {
    #[serde(default)]
    publishedfiledetails: Vec<SteamDetail>,
}

#[derive(Debug, Deserialize)]
struct SteamQueryResponse {
    response: SteamQueryBody,
}

#[derive(Debug, Deserialize)]
struct SteamQueryBody {
    #[serde(default)]
    total: i64,
    #[serde(default)]
    publishedfiledetails: Vec<SteamDetail>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(super) struct SteamDetail {
    #[serde(default)]
    pub(super) publishedfileid: String,
    #[serde(default)]
    pub(super) title: String,
    #[serde(default)]
    pub(super) creator: String,
    #[serde(default, alias = "creator_app_id")]
    pub(super) creator_appid: f64,
    #[serde(default, alias = "consumer_app_id")]
    pub(super) consumer_appid: f64,
    #[serde(default, alias = "description")]
    pub(super) file_description: String,
    #[serde(default)]
    pub(super) preview_url: String,
    #[serde(default)]
    pub(super) time_updated: f64,
    #[serde(default)]
    pub(super) subscriptions: f64,
    #[serde(default)]
    pub(super) file_url: Option<String>,
    #[serde(default, deserialize_with = "deserialize_steam_tags_or_empty")]
    pub(super) views: Vec<SteamTag>,
    #[serde(default, deserialize_with = "deserialize_steam_tags_or_empty")]
    pub(super) tags: Vec<SteamTag>,
    #[serde(default)]
    pub(super) vote_data: SteamVoteData,
    #[serde(default)]
    pub(super) children: Vec<SteamChild>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(super) struct SteamTag {
    #[serde(default)]
    tag: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(super) struct SteamVoteData {
    #[serde(default)]
    score: f64,
    #[serde(default)]
    votes_up: f64,
    #[serde(default)]
    votes_down: f64,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(super) struct SteamChild {
    #[serde(default)]
    pub(super) publishedfileid: String,
}

fn deserialize_steam_tags_or_empty<'de, D>(deserializer: D) -> Result<Vec<SteamTag>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    let Value::Array(values) = value else {
        return Ok(Vec::new());
    };
    Ok(values
        .into_iter()
        .filter_map(|value| serde_json::from_value::<SteamTag>(value).ok())
        .collect())
}
