//! Login-compatible auth handlers for the Rust migration.
//!
//! The public functions in this module perform the same work as the Axum
//! wrappers but accept plain Rust values, which keeps Task 4 testable before
//! the full router is migrated in a later task.

use std::{fmt, net::SocketAddr, path::PathBuf};

use axum::{
    Json,
    extract::{ConnectInfo, State},
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{COOKIE, SET_COOKIE},
    },
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    domain::auth::{
        PasswordFile, SessionStore, UserCredentials, clear_session_cookie_value,
        empty_login_response, is_white_ip, session_cookie_name, session_cookie_value,
    },
    web::error::{AppError, AppResult},
    web::response::LoginResponse,
};

/// Shared state required by auth handlers.
#[derive(Clone)]
pub struct AuthState {
    /// Path to the Go-compatible `password.txt` file.
    pub password_path: PathBuf,
    /// In-memory session store shared by auth middleware and handlers.
    pub sessions: SessionStore,
    /// Optional comma-separated exact IP or CIDR allowlist for direct login.
    pub white_admin_ip: Option<String>,
}

impl AuthState {
    /// Builds auth state from the password file path, session store, and allowlist.
    pub fn new(
        password_path: impl Into<PathBuf>,
        sessions: SessionStore,
        white_admin_ip: Option<String>,
    ) -> Self {
        Self {
            password_path: password_path.into(),
            sessions,
            white_admin_ip,
        }
    }
}

impl fmt::Debug for AuthState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthState")
            .field("password_path", &self.password_path)
            .field("sessions", &self.sessions)
            .field("has_white_admin_ip", &self.white_admin_ip.is_some())
            .finish()
    }
}

/// Request body for the Go-compatible change-password route.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
    /// New password that replaces the password in `password.txt`.
    pub new_password: String,
}

impl fmt::Debug for ChangePasswordRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ChangePasswordRequest")
            .field("new_password", &"<redacted>")
            .finish()
    }
}

/// Request body for the Go-compatible update-user route.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserInfoRequest {
    /// Username that should be persisted to `password.txt`.
    pub username: String,
    /// Display name that should be persisted to `password.txt`.
    pub display_name: String,
    /// Avatar URL that should be persisted to `password.txt`.
    #[serde(rename = "photoURL")]
    pub photo_url: String,
    /// Password that should be persisted to `password.txt`.
    pub password: String,
}

impl fmt::Debug for UpdateUserInfoRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UpdateUserInfoRequest")
            .field("username", &self.username)
            .field("display_name", &self.display_name)
            .field("photo_url", &self.photo_url)
            .field("password", &"<redacted>")
            .finish()
    }
}

/// HTTP-ready auth response carrying a Go-compatible body and optional cookie.
#[derive(Clone, PartialEq, Eq)]
pub struct AuthResponse {
    /// HTTP status returned by the handler.
    pub status: StatusCode,
    /// Go-compatible login-style response body.
    pub body: LoginResponse<Value>,
    /// Optional `Set-Cookie` header value.
    pub set_cookie: Option<String>,
}

impl fmt::Debug for AuthResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthResponse")
            .field("status", &self.status)
            .field("body", &self.body)
            .field("has_set_cookie", &self.set_cookie.is_some())
            .finish()
    }
}

impl AuthResponse {
    /// Creates a successful auth response with caller-provided data.
    pub fn success_with_data(
        message: impl Into<String>,
        data: impl serde::Serialize,
    ) -> AppResult<Self> {
        Ok(Self {
            status: StatusCode::OK,
            body: LoginResponse {
                code: 200,
                msg: message.into(),
                data: serde_json::to_value(data)
                    .map_err(|error| AppError::internal(error.to_string()))?,
            },
            set_cookie: None,
        })
    }

    /// Creates a successful auth response whose Go data field is `null`.
    pub fn success_empty(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::OK,
            body: empty_login_response(message),
            set_cookie: None,
        }
    }

    /// Creates the Go-compatible login failure response.
    pub fn login_failed() -> Self {
        Self {
            status: StatusCode::OK,
            body: LoginResponse::error(401, "User authentication failed"),
            set_cookie: None,
        }
    }

    /// Adds a `Set-Cookie` header value to the response.
    pub fn with_cookie(mut self, cookie: String) -> Self {
        self.set_cookie = Some(cookie);
        self
    }
}

impl IntoResponse for AuthResponse {
    fn into_response(self) -> Response {
        let mut headers = HeaderMap::new();
        if let Some(cookie) = self.set_cookie {
            headers.insert(
                SET_COOKIE,
                HeaderValue::from_str(&cookie).expect("auth cookie header value must be valid"),
            );
        }

        (self.status, headers, Json(self.body)).into_response()
    }
}

/// Authenticates a login request and returns the public user plus session cookie.
pub fn login(
    state: &AuthState,
    remote_addr: Option<&str>,
    mut credentials: UserCredentials,
) -> AppResult<AuthResponse> {
    let password_file = PasswordFile::read(&state.password_path)?;
    let white_ip_matched = remote_addr
        .map(|addr| is_white_ip(addr, state.white_admin_ip.as_deref()))
        .unwrap_or(false);

    if !white_ip_matched
        && (password_file.username != credentials.username
            || password_file.password != credentials.password)
    {
        credentials.clear_password();
        tracing::warn!(white_ip_matched = false, "user authentication failed");
        return Ok(AuthResponse::login_failed());
    }

    credentials.clear_password();
    let session_id = state.sessions.create_session(&password_file.username);
    let public_user = password_file.public_user();
    let response = AuthResponse {
        status: StatusCode::OK,
        body: LoginResponse::success(serde_json::to_value(public_user).map_err(|error| {
            AppError::internal(format!("failed to serialize public user: {error}"))
        })?),
        set_cookie: Some(session_cookie_value(&session_id)),
    };

    tracing::info!(white_ip_matched, "user authentication succeeded");
    Ok(response)
}

/// Clears an optional session id and returns the Go-compatible logout response.
pub fn logout(state: &AuthState, session_id: Option<&str>) -> AppResult<AuthResponse> {
    if let Some(session_id) = session_id.filter(|value| !value.is_empty()) {
        state.sessions.remove(session_id);
    } else {
        tracing::debug!("logout requested without a session cookie");
    }

    tracing::info!("user logout completed");
    Ok(AuthResponse::success_empty("Logout success").with_cookie(clear_session_cookie_value()))
}

/// Returns public user information from `password.txt`.
pub fn get_user_info(state: &AuthState) -> AppResult<AuthResponse> {
    let public_user = PasswordFile::read(&state.password_path)?.public_user();
    tracing::debug!("loaded public user info");
    AuthResponse::success_with_data("Init user success", public_user)
}

/// Rewrites only the password field in `password.txt`.
pub fn change_password(
    state: &AuthState,
    request: ChangePasswordRequest,
) -> AppResult<AuthResponse> {
    let mut password_file = PasswordFile::read(&state.password_path)?;
    password_file.password = request.new_password;
    password_file.write(&state.password_path)?;

    tracing::info!("user password updated");
    Ok(AuthResponse::success_empty(
        "Update user new password success",
    ))
}

/// Rewrites all user fields in `password.txt` and preserves Go's success text.
pub fn update_user_info(
    state: &AuthState,
    request: UpdateUserInfoRequest,
) -> AppResult<AuthResponse> {
    let password_file = PasswordFile {
        username: request.username,
        password: request.password,
        display_name: request.display_name,
        photo_url: request.photo_url,
    };
    password_file.write(&state.password_path)?;

    tracing::info!("user profile updated");
    Ok(AuthResponse::success_empty("Logout success"))
}

/// Axum wrapper for the login route using only trusted connection metadata.
///
/// Forwarded headers are intentionally ignored here because `whiteadminip`
/// bypasses password checks. Task 5 should run the router through Axum's
/// `into_make_service_with_connect_info::<SocketAddr>()` so direct requests
/// use the peer address supplied by the TCP listener, not client-controlled
/// proxy headers.
pub async fn login_handler(
    State(state): State<AuthState>,
    trusted_peer_addr: Option<ConnectInfo<SocketAddr>>,
    _headers: HeaderMap,
    Json(credentials): Json<UserCredentials>,
) -> AppResult<AuthResponse> {
    let remote_addr = trusted_peer_addr.map(|ConnectInfo(addr)| addr.to_string());
    login(&state, remote_addr.as_deref(), credentials)
}

/// Axum wrapper for the logout route.
pub async fn logout_handler(
    State(state): State<AuthState>,
    headers: HeaderMap,
) -> AppResult<AuthResponse> {
    let session_id = extract_token_cookie(&headers);
    logout(&state, session_id.as_deref())
}

/// Axum wrapper for the user-info route.
pub async fn get_user_info_handler(State(state): State<AuthState>) -> AppResult<AuthResponse> {
    get_user_info(&state)
}

/// Axum wrapper for the change-password route.
pub async fn change_password_handler(
    State(state): State<AuthState>,
    Json(request): Json<ChangePasswordRequest>,
) -> AppResult<AuthResponse> {
    change_password(&state, request)
}

/// Axum wrapper for the update-user route.
pub async fn update_user_info_handler(
    State(state): State<AuthState>,
    Json(request): Json<UpdateUserInfoRequest>,
) -> AppResult<AuthResponse> {
    update_user_info(&state, request)
}

/// Extracts the legacy `token` cookie value from request headers.
pub fn extract_token_cookie(headers: &HeaderMap) -> Option<String> {
    let cookie_header = headers.get(COOKIE)?.to_str().ok()?;
    cookie_header.split(';').find_map(|cookie| {
        let (name, value) = cookie.trim().split_once('=')?;
        if name == session_cookie_name() && !value.is_empty() {
            Some(value.to_owned())
        } else {
            None
        }
    })
}
