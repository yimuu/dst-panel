//! Fakeable HTTP client adapter for third-party read-only integrations.
//!
//! The Go panel calls several public DST/Steam/Lobby endpoints directly from
//! handlers. This module keeps the Rust migration testable by pushing those
//! calls behind a small trait: production uses reqwest, while integration tests
//! inject [`FakeHttpClient`] and assert the exact method, URL, headers, and
//! body without reaching the network.

use std::{
    collections::VecDeque,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use futures_util::StreamExt;
use thiserror::Error;

const DEFAULT_HTTP_TIMEOUT: Duration = Duration::from_secs(10);
/// Maximum upstream response body retained in memory for read-only proxy routes.
pub const MAX_HTTP_RESPONSE_BYTES: usize = 10 * 1024 * 1024;

/// HTTP request captured at the application boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpRequest {
    /// Uppercase method string such as `GET` or `POST`.
    pub method: String,
    /// Absolute upstream URL.
    pub url: String,
    /// Request headers. Values are stored as UTF-8 strings because all migrated
    /// third-party routes send JSON or simple proxy metadata.
    pub headers: Vec<(String, String)>,
    /// Raw request body bytes.
    pub body: Vec<u8>,
}

impl HttpRequest {
    /// Creates a request with no headers or body.
    pub fn new(method: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            url: url.into(),
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    /// Adds one request header.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Sets the request body.
    pub fn body(mut self, body: impl AsRef<[u8]>) -> Self {
        self.body = body.as_ref().to_vec();
        self
    }
}

/// HTTP response returned from an upstream service.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpResponse {
    /// Numeric upstream status code.
    pub status: u16,
    /// Response headers preserved as UTF-8 pairs.
    pub headers: Vec<(String, String)>,
    /// Raw response body bytes.
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// Creates an empty response with a configured status.
    pub fn new(status: u16) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    /// Adds one response header.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Sets the raw response body.
    pub fn body(mut self, body: impl AsRef<[u8]>) -> Self {
        self.body = body.as_ref().to_vec();
        self
    }

    /// Returns the response content type, if the upstream sent one.
    pub fn content_type(&self) -> Option<&str> {
        self.headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
            .map(|(_, value)| value.as_str())
    }
}

/// Errors from the fakeable HTTP adapter.
#[derive(Debug, Error)]
pub enum HttpError {
    /// The caller supplied a method reqwest cannot parse.
    #[error("invalid upstream HTTP method")]
    InvalidMethod,
    /// Reqwest failed while building, sending, or reading the response.
    #[error("upstream request failed")]
    Request(#[source] reqwest::Error),
    /// The upstream body exceeded the bounded in-memory proxy limit.
    #[error("upstream response body is too large")]
    ResponseTooLarge,
    /// Test fake was asked for more responses than configured.
    #[error("fake HTTP client has no response configured")]
    FakeExhausted,
}

/// Future returned by HTTP clients.
pub type HttpFuture<'a> =
    Pin<Box<dyn Future<Output = Result<HttpResponse, HttpError>> + Send + 'a>>;

/// Fakeable HTTP client used by migrated handlers.
pub trait HttpClient: Send + Sync {
    fn send<'a>(&'a self, request: HttpRequest) -> HttpFuture<'a>;
}

/// Reqwest-backed production HTTP client.
#[derive(Clone, Debug)]
pub struct ReqwestHttpClient {
    client: reqwest::Client,
}

impl ReqwestHttpClient {
    /// Creates a reqwest client with rustls TLS support.
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(DEFAULT_HTTP_TIMEOUT)
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("static reqwest client configuration is valid"),
        }
    }

    async fn send_inner(&self, request: HttpRequest) -> Result<HttpResponse, HttpError> {
        let method = reqwest::Method::from_bytes(request.method.as_bytes())
            .map_err(|_| HttpError::InvalidMethod)?;
        let method_label = method.as_str().to_owned();
        let body_len = request.body.len();
        tracing::info!(
            method = %method_label,
            body_len,
            "sending upstream HTTP request"
        );

        let mut builder = self.client.request(method, &request.url);
        for (name, value) in &request.headers {
            builder = builder.header(name, value);
        }
        if !request.body.is_empty() {
            builder = builder.body(request.body);
        }

        let response = builder.send().await.map_err(HttpError::Request)?;
        let status = response.status().as_u16();
        let headers = response_headers(&response);
        let body = read_bounded_body(response).await?;
        tracing::info!(
            method = %method_label,
            status,
            response_body_len = body.len(),
            "received upstream HTTP response"
        );

        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}

async fn read_bounded_body(response: reqwest::Response) -> Result<Vec<u8>, HttpError> {
    if response
        .content_length()
        .is_some_and(|length| length > MAX_HTTP_RESPONSE_BYTES as u64)
    {
        return Err(HttpError::ResponseTooLarge);
    }

    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(HttpError::Request)?;
        if body.len().saturating_add(chunk.len()) > MAX_HTTP_RESPONSE_BYTES {
            return Err(HttpError::ResponseTooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

impl Default for ReqwestHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient for ReqwestHttpClient {
    fn send<'a>(&'a self, request: HttpRequest) -> HttpFuture<'a> {
        Box::pin(async move { self.send_inner(request).await })
    }
}

fn response_headers(response: &reqwest::Response) -> Vec<(String, String)> {
    response
        .headers()
        .iter()
        .filter_map(|(name, value)| {
            Some((name.as_str().to_owned(), value.to_str().ok()?.to_owned()))
        })
        .collect()
}

/// Test HTTP client that records outgoing requests and returns queued responses.
#[derive(Clone, Debug, Default)]
pub struct FakeHttpClient {
    responses: Arc<Mutex<VecDeque<HttpResponse>>>,
    requests: Arc<Mutex<Vec<HttpRequest>>>,
}

impl FakeHttpClient {
    /// Creates a fake with preconfigured responses in call order.
    pub fn new(responses: Vec<HttpResponse>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses.into())),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Returns all requests sent through the fake so tests can assert parity.
    pub fn requests(&self) -> Vec<HttpRequest> {
        self.requests
            .lock()
            .expect("fake HTTP requests poisoned")
            .clone()
    }
}

impl HttpClient for FakeHttpClient {
    fn send<'a>(&'a self, request: HttpRequest) -> HttpFuture<'a> {
        let result = {
            self.requests
                .lock()
                .expect("fake HTTP requests poisoned")
                .push(request);
            self.responses
                .lock()
                .expect("fake HTTP responses poisoned")
                .pop_front()
                .ok_or(HttpError::FakeExhausted)
        };
        Box::pin(async move { result })
    }
}
