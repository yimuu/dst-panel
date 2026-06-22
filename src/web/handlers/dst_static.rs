//! Fixed-host proxy for `/api/dst-static/{*filepath}`.
//!
//! Go forwards this route to Gitee for every HTTP method. Rust keeps the fixed
//! upstream host and all-method behavior but refuses traversal path segments and
//! strips sensitive panel/auth headers before sending the request upstream.

use axum::{
    body::{Body, Bytes},
    extract::{Path, State},
    http::{HeaderMap, Method, StatusCode},
    response::Response,
};

use crate::{
    infra::http_client::{HttpRequest, MAX_HTTP_RESPONSE_BYTES},
    web::app::AppState,
    web::error::{AppError, AppResult},
};

const DST_STATIC_BASE_URL: &str = "https://gitee.com/hhhuhu23/dst-static/raw/master";

/// Proxies one static resource to the fixed Gitee raw endpoint.
pub(crate) async fn proxy_handler(
    State(state): State<AppState>,
    method: Method,
    Path(filepath): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> AppResult<Response> {
    if body.len() > MAX_HTTP_RESPONSE_BYTES {
        return Err(AppError::payload_too_large(
            "proxy request body is too large",
        ));
    }
    let upstream_path = safe_upstream_path(&filepath)?;
    let url = format!("{DST_STATIC_BASE_URL}/{upstream_path}");
    let mut request = HttpRequest::new(method.as_str(), url).body(&body);
    for (name, value) in proxy_headers(&headers) {
        request = request.header(name, value);
    }

    let response = state.http_client.send(request).await.map_err(|error| {
        tracing::warn!(error = %error, "dst-static upstream request failed");
        AppError::bad_request("failed to fetch dst-static resource")
    })?;
    let status = StatusCode::from_u16(response.status).unwrap_or(StatusCode::BAD_GATEWAY);
    let mut builder = Response::builder()
        .status(status)
        .header("Access-Control-Allow-Origin", "*")
        .header(
            "Access-Control-Allow-Methods",
            "GET, POST, PUT, DELETE, OPTIONS",
        )
        .header("Access-Control-Allow-Headers", "Origin, Content-Type");
    for (name, value) in response.headers {
        if is_safe_response_header(&name) {
            builder = builder.header(name, value);
        }
    }

    tracing::info!(
        method = %method,
        status = status.as_u16(),
        body_len = response.body.len(),
        "proxied dst-static resource"
    );
    builder
        .body(Body::from(response.body))
        .map_err(|_| AppError::internal("failed to build dst-static response"))
}

fn safe_upstream_path(filepath: &str) -> AppResult<String> {
    let trimmed = filepath.strip_prefix('/').unwrap_or(filepath);
    if trimmed.is_empty() {
        return Err(AppError::bad_request("dst-static path cannot be empty"));
    }
    let mut encoded = Vec::new();
    for segment in trimmed.split('/') {
        if segment.is_empty()
            || segment == "."
            || segment == ".."
            || segment.contains('\\')
            || segment.contains('?')
            || segment.contains('#')
            || segment.chars().any(char::is_control)
        {
            return Err(AppError::bad_request("invalid dst-static path"));
        }
        encoded.push(encode_path_segment(segment));
    }
    Ok(encoded.join("/"))
}

fn proxy_headers(headers: &HeaderMap) -> Vec<(String, String)> {
    let mut forwarded = Vec::new();
    for name in [
        "accept",
        "accept-language",
        "cache-control",
        "content-type",
        "if-none-match",
        "if-modified-since",
        "user-agent",
        "x-requested-with",
    ] {
        let Some(value) = headers.get(name).and_then(|value| value.to_str().ok()) else {
            continue;
        };
        forwarded.push((name.to_owned(), value.to_owned()));
    }
    forwarded
}

fn is_safe_response_header(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    !matches!(
        name.as_str(),
        "connection"
            | "content-length"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "set-cookie"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
    )
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
