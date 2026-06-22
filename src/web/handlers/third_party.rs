//! Third-party read-only proxy handlers.
//!
//! These routes intentionally preserve the Go panel's raw-proxy behavior:
//! successful upstream responses are forwarded without the normal JSON
//! envelope, while upstream transport failures or non-200 statuses become a
//! bare `503 Service Unavailable`. The Klei lobby detail route is the exception
//! because the Go code enriches the upstream object with parsed Lua fields and
//! wraps it in the legacy `{code,msg,data}` response.

use axum::{
    Json,
    body::Body,
    body::Bytes,
    extract::{Query, State},
    http::{StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::{
    infra::http_client::{HttpRequest, HttpResponse, MAX_HTTP_RESPONSE_BYTES},
    web::app::AppState,
    web::handlers::legacy_success,
};

const DST_VERSION_URL: &str = "http://ver.tugos.cn/getLocalVersion";
const HOME_SERVER_URL: &str = "http://dst.liuyh.com/index/serverlist/getserverlist.html";
const HOME_DETAIL_URL: &str = "http://dst.liuyh.com/index/serverlist/getserverdetail.html";
const SERVER_LIST2_URL: &str = "https://api.dstserverlist.top/api/list";
const SERVER_DETAIL2_URL: &str = "https://api.dstserverlist.top/api/details/";
const LOBBY_READ_TOKEN: &str = "pds-g^KU_qE7e8rv1^VVrVXd/01kBDicd7UO5LeL+uYZH1+geZlrutzItvOaw=";

/// Proxies the community endpoint that reports the latest DST version.
pub(crate) async fn dst_version_handler(State(state): State<AppState>) -> Response {
    let request = HttpRequest::new("GET", DST_VERSION_URL);
    send_raw_proxy(&state, request, "dst_version", &[]).await
}

/// Proxies the legacy dst.liuyh.com server list endpoint.
pub(crate) async fn home_server_handler(State(state): State<AppState>, body: Bytes) -> Response {
    let param: HomeServerParam = parse_json_or_default(&body, "home_server");
    let payload = build_home_server_payload(param);
    let request = HttpRequest::new("POST", HOME_SERVER_URL)
        .header("X-Requested-With", "XMLHttpRequest")
        .header("Content-Type", "application/json")
        .body(serde_json::to_vec(&payload).expect("home-server payload is serializable"));
    send_raw_proxy(
        &state,
        request,
        "home_server",
        &[("X-Requested-With", "XMLHttpRequest")],
    )
    .await
}

/// Proxies the legacy dst.liuyh.com server detail endpoint.
pub(crate) async fn home_server_detail_handler(
    State(state): State<AppState>,
    body: Bytes,
) -> Response {
    let param: HomeDetailParam = parse_json_or_default(&body, "home_server_detail");
    let request = HttpRequest::new("POST", HOME_DETAIL_URL)
        .header("X-Requested-With", "XMLHttpRequest")
        .header("Content-Type", "application/json")
        .body(serde_json::to_vec(&param).expect("home-detail payload is serializable"));
    send_raw_proxy(
        &state,
        request,
        "home_server_detail",
        &[("X-Requested-With", "XMLHttpRequest")],
    )
    .await
}

/// Reads one Klei lobby row and adds Go-compatible parsed Lua helper fields.
pub(crate) async fn lobby_server_detail_handler(
    State(state): State<AppState>,
    Query(query): Query<LobbyDetailQuery>,
) -> Response {
    if !is_safe_lobby_region(&query.region) {
        tracing::warn!("rejected unsafe Klei lobby region");
        return Json(legacy_success(empty_lobby_home_detail())).into_response();
    }

    let url = format!("https://lobby-v2-{}.klei.com/lobby/read", query.region);
    let payload = json!({
        "__gameId": "DontStarveTogether",
        "__token": LOBBY_READ_TOKEN,
        "query": {
            "__rowId": query.row_id,
        },
    });
    let request = HttpRequest::new("POST", url)
        .header("Content-Type", "application/json")
        .body(serde_json::to_vec(&payload).expect("lobby payload is serializable"));

    let response = match state.http_client.send(request).await {
        Ok(response) => response,
        Err(error) => {
            tracing::warn!(error = %error, "Klei lobby detail request failed");
            return Json(legacy_success(empty_lobby_home_detail())).into_response();
        }
    };

    let body: Value = match serde_json::from_slice(&response.body) {
        Ok(body) => body,
        Err(error) => {
            tracing::warn!(
                status = response.status,
                body_len = response.body.len(),
                error = %error,
                "Klei lobby detail response was not valid JSON"
            );
            return Json(legacy_success(empty_lobby_home_detail())).into_response();
        }
    };
    let Some(mut detail) = body
        .get("GET")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .cloned()
    else {
        tracing::info!(
            status = response.status,
            body_len = response.body.len(),
            "Klei lobby detail response had no rows"
        );
        return Json(legacy_success(empty_lobby_home_detail())).into_response();
    };

    enrich_lobby_detail(&mut detail);
    tracing::info!(
        status = response.status,
        body_len = response.body.len(),
        "served Klei lobby detail"
    );
    Json(legacy_success(detail)).into_response()
}

/// Proxies the dstserverlist.top list endpoint.
pub(crate) async fn home_server2_handler(
    State(state): State<AppState>,
    Query(query): Query<HomeServer2Query>,
) -> Response {
    // Go's url.Values.Encode sorts keys lexicographically, which yields this
    // order. Keeping it stable makes recorded fake requests byte-for-byte
    // comparable to the legacy implementation.
    let current = query.current.unwrap_or_else(|| "1".to_owned());
    let page_size = query.page_size.unwrap_or_else(|| "10".to_owned());
    let name = query.name.unwrap_or_default();
    let url = format!(
        "{SERVER_LIST2_URL}?name={}&page={}&pageCount={}",
        encode_query_component(&name),
        encode_query_component(&current),
        encode_query_component(&page_size)
    );
    let request = HttpRequest::new("POST", url).header("Content-Type", "application/json");
    send_raw_proxy(&state, request, "home_server2", &[]).await
}

/// Proxies the dstserverlist.top detail endpoint.
pub(crate) async fn home_server_detail2_handler(
    State(state): State<AppState>,
    Query(query): Query<HomeDetail2Query>,
) -> Response {
    // Go appends `rowId` directly to the path. Rust percent-encodes the segment
    // as a documented hardening so a crafted slash cannot alter the upstream
    // route while normal Klei row ids remain byte-for-byte unchanged.
    let url = format!(
        "{SERVER_DETAIL2_URL}{}",
        encode_path_segment(&query.row_id.unwrap_or_default())
    );
    let request = HttpRequest::new("POST", url).header("Content-Type", "application/json");
    send_raw_proxy(&state, request, "home_server_detail2", &[]).await
}

async fn send_raw_proxy(
    state: &AppState,
    request: HttpRequest,
    upstream: &'static str,
    extra_headers: &[(&'static str, &'static str)],
) -> Response {
    match state.http_client.send(request).await {
        Ok(response)
            if response.status == 200 && response.body.len() <= MAX_HTTP_RESPONSE_BYTES =>
        {
            tracing::info!(
                upstream,
                status = response.status,
                body_len = response.body.len(),
                "forwarding successful upstream response"
            );
            raw_response(response, extra_headers)
        }
        Ok(response) if response.status == 200 => {
            tracing::warn!(
                upstream,
                status = response.status,
                body_len = response.body.len(),
                max_body_len = MAX_HTTP_RESPONSE_BYTES,
                "upstream response exceeded proxy body limit"
            );
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
        Ok(response) => {
            tracing::warn!(
                upstream,
                status = response.status,
                body_len = response.body.len(),
                "upstream returned non-200 status"
            );
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
        Err(error) => {
            tracing::warn!(upstream, error = %error, "upstream request failed");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}

fn empty_lobby_home_detail() -> Value {
    let mut object = Map::new();
    for key in [
        "__addr",
        "__rowId",
        "host",
        "steamclanid",
        "name",
        "session",
        "guid",
        "mode",
        "tags",
        "season",
        "intent",
        "steamid",
        "steamroom",
        "secondariesJson",
        "data",
        "worldgen",
        "players",
        "desc",
    ] {
        object.insert(key.to_owned(), Value::String(String::new()));
    }
    for key in [
        "clanonly",
        "mods",
        "pvp",
        "fo",
        "password",
        "dedicated",
        "clienthosted",
        "lanonly",
        "allownewplayers",
        "serverpaused",
        "clientmodsoff",
    ] {
        object.insert(key.to_owned(), Value::Bool(false));
    }
    for key in [
        "platform",
        "maxconnections",
        "connected",
        "port",
        "v",
        "tick",
        "nat",
    ] {
        object.insert(key.to_owned(), json!(0));
    }
    object.insert("secondaries".to_owned(), Value::Null);
    object.insert("mods_info".to_owned(), Value::Null);
    object.insert("playerList".to_owned(), Value::Null);
    object.insert(
        "dayData".to_owned(),
        json!({
            "day": 0,
            "dayselapsedinseason": 0,
            "daysleftinseason": 0,
        }),
    );
    Value::Object(object)
}

fn raw_response(
    response: HttpResponse,
    extra_headers: &[(&'static str, &'static str)],
) -> Response {
    let mut builder = Response::builder().status(StatusCode::OK);
    if let Some(content_type) = response.content_type() {
        builder = builder.header(CONTENT_TYPE, content_type);
    }
    for (name, value) in extra_headers {
        builder = builder.header(*name, *value);
    }
    builder
        .body(Body::from(response.body))
        .expect("raw proxy response is valid")
}

fn parse_json_or_default<T>(body: &[u8], route: &'static str) -> T
where
    T: Default + for<'de> Deserialize<'de>,
{
    if body.is_empty() {
        return T::default();
    }
    match serde_json::from_slice(body) {
        Ok(value) => value,
        Err(error) => {
            // Gin's ShouldBind logs parse errors but keeps processing with the
            // zero-value struct. Preserve that odd compatibility behavior here.
            tracing::warn!(route, error = %error, "using default request payload after JSON parse failure");
            T::default()
        }
    }
}

fn build_home_server_payload(param: HomeServerParam) -> Value {
    let mut payload = Map::new();
    payload.insert("page".to_owned(), json!(param.page));
    payload.insert("paginate".to_owned(), json!(param.paginate));
    payload.insert("sort_type".to_owned(), json!(param.sort_type));
    payload.insert("sort_way".to_owned(), json!(param.sort_way));
    payload.insert("search_type".to_owned(), json!(param.search_type));
    insert_if_non_empty(&mut payload, "search_content", param.search_content);
    insert_if_non_empty(&mut payload, "mode", param.mode);
    insert_if_non_empty(&mut payload, "season", param.season);
    insert_if_not_minus_one(&mut payload, "pvp", param.pvp);
    insert_if_not_minus_one(&mut payload, "mod", param.mod_filter);
    insert_if_not_minus_one(&mut payload, "password", param.password);
    insert_if_not_minus_one(&mut payload, "world", param.world);
    insert_if_non_empty(&mut payload, "playerpercent", param.playerpercent);
    Value::Object(payload)
}

fn insert_if_non_empty(payload: &mut Map<String, Value>, key: &'static str, value: String) {
    if !value.is_empty() {
        payload.insert(key.to_owned(), Value::String(value));
    }
}

fn insert_if_not_minus_one(payload: &mut Map<String, Value>, key: &'static str, value: i64) {
    if value != -1 {
        payload.insert(key.to_owned(), json!(value));
    }
}

fn enrich_lobby_detail(detail: &mut Value) {
    let Some(object) = detail.as_object_mut() else {
        return;
    };
    let players = object
        .get("players")
        .and_then(Value::as_str)
        .map(parse_players_lua)
        .unwrap_or_default();
    let day_data = object
        .get("data")
        .and_then(Value::as_str)
        .map(parse_day_data_lua)
        .unwrap_or_else(|| {
            json!({
                "day": 0,
                "dayselapsedinseason": 0,
                "daysleftinseason": 0,
            })
        });

    object.insert("playerList".to_owned(), Value::Array(players));
    object.insert("dayData".to_owned(), day_data);
    let secondaries_json = object
        .get("secondaries")
        .filter(|value| !value.is_null())
        .and_then(|value| serde_json::to_string(value).ok());
    if let Some(serialized) = secondaries_json {
        object.insert("secondariesJson".to_owned(), Value::String(serialized));
    }
}

fn parse_players_lua(lua: &str) -> Vec<Value> {
    if lua.trim() == "return {  }" || lua.trim() == "return { }" {
        return Vec::new();
    }

    let mut players = Vec::new();
    let mut depth = 0_i32;
    let mut start = None;
    for (index, character) in lua.char_indices() {
        match character {
            '{' => {
                depth += 1;
                if depth == 2 {
                    start = Some(index + character.len_utf8());
                }
            }
            '}' => {
                if depth == 2
                    && let Some(start_index) = start.take()
                {
                    players.push(parse_player_table(&lua[start_index..index]));
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    players
}

fn parse_player_table(table: &str) -> Value {
    json!({
        "colour": extract_lua_string(table, &["colour"]).unwrap_or_default(),
        "eventLevel": extract_lua_number(table, "eventlevel").unwrap_or_default(),
        "name": extract_lua_string(table, &["name"]).unwrap_or_default(),
        "netID": extract_lua_string(table, &["netID", "netid"]).unwrap_or_default(),
        "prefab": extract_lua_string(table, &["prefab"]).unwrap_or_default(),
    })
}

fn parse_day_data_lua(lua: &str) -> Value {
    json!({
        "day": extract_lua_number(lua, "day").unwrap_or_default(),
        "dayselapsedinseason": extract_lua_number(lua, "dayselapsedinseason").unwrap_or_default(),
        "daysleftinseason": extract_lua_number(lua, "daysleftinseason").unwrap_or_default(),
    })
}

fn extract_lua_string(table: &str, keys: &[&str]) -> Option<String> {
    for key in keys {
        let Some(after_key) = after_lua_key(table, key) else {
            continue;
        };
        let after_equals = after_key.trim_start();
        let after_quote = after_equals.strip_prefix('"')?;
        let mut value = String::new();
        let mut escaped = false;
        for character in after_quote.chars() {
            if escaped {
                value.push(character);
                escaped = false;
                continue;
            }
            match character {
                '\\' => escaped = true,
                '"' => return Some(value),
                _ => value.push(character),
            }
        }
    }
    None
}

fn extract_lua_number(table: &str, key: &str) -> Option<i64> {
    let after_key = after_lua_key(table, key)?;
    let mut number = String::new();
    for character in after_key.trim_start().chars() {
        if (character == '-' && number.is_empty()) || character.is_ascii_digit() {
            number.push(character);
            continue;
        }
        break;
    }
    number.parse().ok()
}

fn after_lua_key<'a>(table: &'a str, key: &str) -> Option<&'a str> {
    for (start, _) in table.match_indices(key) {
        let before = table[..start].chars().rev().find(|c| !c.is_whitespace());
        let after_key = &table[start + key.len()..];
        let Some(after) = after_key.chars().find(|c| !c.is_whitespace()) else {
            continue;
        };
        if !matches!(before, None | Some('{') | Some(',')) || after != '=' {
            continue;
        }
        let equals = after_key.find('=')?;
        return Some(&after_key[equals + 1..]);
    }
    None
}

fn is_safe_lobby_region(region: &str) -> bool {
    !region.is_empty()
        && region
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
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

fn encode_path_segment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char)
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

#[derive(Debug, Default, Deserialize)]
struct HomeServerParam {
    #[serde(default)]
    page: i64,
    #[serde(default)]
    paginate: i64,
    #[serde(default)]
    sort_type: String,
    #[serde(default)]
    sort_way: i64,
    #[serde(default, rename = "search_type")]
    search_type: i64,
    #[serde(default, rename = "search_content")]
    search_content: String,
    #[serde(default)]
    mode: String,
    #[serde(default, rename = "mod")]
    mod_filter: i64,
    #[serde(default)]
    season: String,
    #[serde(default)]
    pvp: i64,
    #[serde(default)]
    password: i64,
    #[serde(default)]
    world: i64,
    #[serde(default)]
    playerpercent: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct HomeDetailParam {
    #[serde(default, rename = "rowId")]
    row_id: String,
    #[serde(default)]
    region: String,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct LobbyDetailQuery {
    #[serde(default)]
    region: String,
    #[serde(default, rename = "rowId")]
    row_id: String,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct HomeServer2Query {
    #[serde(default)]
    current: Option<String>,
    #[serde(default, rename = "pageSize")]
    page_size: Option<String>,
    #[serde(default, rename = "Name")]
    name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct HomeDetail2Query {
    #[serde(default, rename = "rowId")]
    row_id: Option<String>,
}
