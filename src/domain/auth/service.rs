//! Authentication primitives shared by migrated handlers.
//!
//! This module keeps the legacy Go `password.txt` contract intact while
//! providing Rust-native session storage and compatibility helpers for the
//! auth middleware that will be mounted by later migration tasks.

use std::{
    collections::HashMap,
    fmt, fs,
    net::{IpAddr, SocketAddr},
    path::Path,
    sync::{Arc, RwLock},
    time::{Duration, SystemTime},
};

use serde::{Deserialize, Serialize};

use crate::{
    web::error::{AppError, AppResult},
    web::response::LoginResponse,
};

/// Legacy relative password file path used by the Go backend.
pub const DEFAULT_PASSWORD_PATH: &str = "./password.txt";

const SESSION_COOKIE_NAME: &str = "token";
const SESSION_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const SESSION_MAX_AGE_SECONDS: u64 = SESSION_TTL.as_secs();
const WHITELISTED_PATHS: &[&str] = &[
    "/api/login",
    "/api/logout",
    "/ws",
    "/api/bootstrap",
    "/api/init",
    "/api/install/steamcmd",
];

/// On-disk login file compatible with Go's four-line `password.txt` format.
#[derive(Clone, PartialEq, Eq)]
pub struct PasswordFile {
    /// Username used for administrator login.
    pub username: String,
    /// Plaintext legacy password read from and written to `password.txt`.
    pub password: String,
    /// Display name returned to the frontend after successful login.
    pub display_name: String,
    /// Avatar URL returned to the frontend after successful login.
    pub photo_url: String,
}

impl PasswordFile {
    /// Reads and parses a Go-compatible `password.txt` file.
    ///
    /// Both `key=value` and `key = value` lines are accepted. Unknown or blank
    /// lines are ignored so older deployments can carry harmless comments or
    /// extra metadata without breaking the migrated auth path.
    pub fn read(path: impl AsRef<Path>) -> AppResult<Self> {
        let path = path.as_ref();
        tracing::debug!(password_path = %path.display(), "reading password file");

        let contents = fs::read_to_string(path)?;
        let mut username = None;
        let mut password = None;
        let mut display_name = None;
        let mut photo_url = None;

        for (line_index, line) in contents.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let Some((key, value)) = line.split_once('=') else {
                tracing::debug!(
                    line_number = line_index + 1,
                    "ignored malformed password file line"
                );
                continue;
            };

            match key.trim() {
                "username" => username = Some(value.trim().to_owned()),
                "password" => password = Some(value.trim().to_owned()),
                "displayName" => display_name = Some(value.trim().to_owned()),
                "photoURL" => photo_url = Some(value.trim().to_owned()),
                _ => tracing::debug!(
                    line_number = line_index + 1,
                    "ignored unknown password file key"
                ),
            }
        }

        let file = Self {
            username: required_password_key(username, "username")?,
            password: required_password_key(password, "password")?,
            display_name: required_password_key(display_name, "displayName")?,
            photo_url: required_password_key(photo_url, "photoURL")?,
        };

        tracing::debug!(password_path = %path.display(), "parsed password file metadata");
        Ok(file)
    }

    /// Rewrites `password.txt` using the key order and mixed spacing used by Go.
    pub fn write(&self, path: impl AsRef<Path>) -> AppResult<()> {
        let path = path.as_ref();
        let contents = format!(
            "username = {}\npassword = {}\ndisplayName={}\nphotoURL={}\n",
            self.username, self.password, self.display_name, self.photo_url
        );

        fs::write(path, contents)?;
        tracing::info!(password_path = %path.display(), "rewrote password file");
        Ok(())
    }

    /// Builds the public user payload returned by login-style endpoints.
    pub fn public_user(&self) -> PublicUser {
        PublicUser {
            username: self.username.clone(),
            display_name: self.display_name.clone(),
            photo_url: self.photo_url.clone(),
        }
    }
}

impl fmt::Debug for PasswordFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PasswordFile")
            .field("username", &self.username)
            .field("password", &"<redacted>")
            .field("display_name", &self.display_name)
            .field("photo_url", &self.photo_url)
            .finish()
    }
}

/// Public user information returned to the frontend after authentication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicUser {
    /// Username stored in the legacy password file.
    pub username: String,
    /// Display name stored in the legacy password file.
    pub display_name: String,
    /// Avatar URL stored in the legacy password file.
    #[serde(rename = "photoURL")]
    pub photo_url: String,
}

/// Login request payload compatible with Go's `UserVO`.
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserCredentials {
    /// Username submitted by the client.
    pub username: String,
    /// Password submitted by the client.
    #[serde(skip_serializing)]
    pub password: String,
    /// Optional legacy session id field accepted for compatibility.
    #[serde(default, skip_serializing)]
    pub session_id: Option<String>,
}

impl UserCredentials {
    /// Clears the in-memory password after authentication checks are complete.
    pub fn clear_password(&mut self) {
        self.password.clear();
    }
}

impl fmt::Debug for UserCredentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UserCredentials")
            .field("username", &self.username)
            .field("password", &"<redacted>")
            .field(
                "session_id",
                &self.session_id.as_ref().map(|_| "<redacted>"),
            )
            .finish()
    }
}

/// Stored session metadata for an authenticated admin user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRecord {
    /// Username associated with the session.
    pub username: String,
    /// Absolute expiration timestamp used by validation and cleanup.
    pub expires_at: SystemTime,
}

/// Thread-safe in-memory session store used by the migrated auth handlers.
#[derive(Clone, Default)]
pub struct SessionStore {
    sessions: Arc<RwLock<HashMap<String, SessionRecord>>>,
}

impl SessionStore {
    /// Creates an empty in-memory session store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a seven-day session for `username` and returns an opaque id.
    pub fn create_session(&self, username: impl Into<String>) -> String {
        let session_id = generate_session_id();
        let record = SessionRecord {
            username: username.into(),
            expires_at: SystemTime::now() + SESSION_TTL,
        };

        self.sessions
            .write()
            .expect("session store lock poisoned")
            .insert(session_id.clone(), record);
        tracing::info!("created auth session");

        session_id
    }

    /// Returns the username for a valid, unexpired session id.
    pub fn validate(&self, session_id: &str) -> Option<String> {
        if session_id.is_empty() {
            tracing::debug!("session validation skipped for empty session id");
            return None;
        }

        let mut sessions = self.sessions.write().expect("session store lock poisoned");
        let Some(record) = sessions.get(session_id) else {
            tracing::debug!("session validation failed");
            return None;
        };

        if record.expires_at <= SystemTime::now() {
            sessions.remove(session_id);
            tracing::debug!("removed expired auth session during validation");
            return None;
        }

        tracing::trace!("session validation succeeded");
        Some(record.username.clone())
    }

    /// Removes a session id from the store and returns whether it existed.
    pub fn remove(&self, session_id: &str) -> bool {
        if session_id.is_empty() {
            tracing::debug!("session removal skipped for empty session id");
            return false;
        }

        let removed = self
            .sessions
            .write()
            .expect("session store lock poisoned")
            .remove(session_id)
            .is_some();
        tracing::info!(removed, "removed auth session");
        removed
    }

    /// Removes all expired sessions and returns the number of discarded records.
    pub fn clear_expired(&self) -> usize {
        let now = SystemTime::now();
        let mut sessions = self.sessions.write().expect("session store lock poisoned");
        let before = sessions.len();
        sessions.retain(|_, record| record.expires_at > now);
        let removed = before.saturating_sub(sessions.len());
        tracing::debug!(removed, "cleared expired auth sessions");
        removed
    }
}

impl fmt::Debug for SessionStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let session_count = self
            .sessions
            .read()
            .map(|sessions| sessions.len())
            .unwrap_or_default();
        formatter
            .debug_struct("SessionStore")
            .field("session_count", &session_count)
            .finish()
    }
}

/// Returns the legacy session cookie name used by Go.
pub fn session_cookie_name() -> &'static str {
    SESSION_COOKIE_NAME
}

/// Builds the `Set-Cookie` value for a newly created session.
pub fn session_cookie_value(session_id: &str) -> String {
    format!(
        "{}={}; Max-Age={}; Path=/; HttpOnly; SameSite=Lax",
        session_cookie_name(),
        session_id,
        SESSION_MAX_AGE_SECONDS
    )
}

/// Builds the `Set-Cookie` value used to clear a browser session cookie.
pub fn clear_session_cookie_value() -> String {
    format!(
        "{}=; Max-Age=0; Path=/; HttpOnly; SameSite=Lax",
        session_cookie_name()
    )
}

/// Returns whether a request path should bypass authentication middleware.
pub fn is_whitelisted_path(path: &str) -> bool {
    let is_api_path = path.starts_with("/api");
    let allowed = !is_api_path || WHITELISTED_PATHS.contains(&path);
    tracing::trace!(is_api_path, allowed, "evaluated auth path whitelist");
    allowed
}

/// Returns whether `remote_addr` matches an exact IP or CIDR in `white_admin_ip`.
///
/// The `white_admin_ip` value follows Go config compatibility: a comma-separated
/// list where each entry is either a single IP address or a CIDR range. The
/// remote address may be a plain IP or a `host:port` socket address.
pub fn is_white_ip(remote_addr: &str, white_admin_ip: Option<&str>) -> bool {
    let Some(config) = white_admin_ip.filter(|value| !value.trim().is_empty()) else {
        tracing::debug!(matched = false, "evaluated white admin ip rule");
        return false;
    };
    let Some(remote_ip) = parse_remote_ip(remote_addr) else {
        tracing::warn!(
            matched = false,
            "could not parse remote address for white admin ip"
        );
        return false;
    };

    for entry in config
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        if entry.contains('/') {
            match cidr_contains(entry, remote_ip) {
                Some(true) => {
                    tracing::info!(matched = true, "evaluated white admin ip rule");
                    return true;
                }
                Some(false) => continue,
                None => {
                    tracing::warn!(matched = false, "ignored invalid white admin cidr entry");
                    continue;
                }
            }
        }

        match entry.parse::<IpAddr>() {
            Ok(admin_ip) if admin_ip == remote_ip => {
                tracing::info!(matched = true, "evaluated white admin ip rule");
                return true;
            }
            Ok(_) => {}
            Err(_) => tracing::warn!(matched = false, "ignored invalid white admin ip entry"),
        }
    }

    tracing::debug!(matched = false, "evaluated white admin ip rule");
    false
}

fn required_password_key(value: Option<String>, key: &'static str) -> AppResult<String> {
    value.ok_or_else(|| AppError::bad_request(format!("password file missing `{key}`")))
}

fn generate_session_id() -> String {
    let mut bytes = [0_u8; 32];
    if let Err(error) = getrandom::fill(&mut bytes) {
        tracing::error!(error = %error, "failed to generate auth session id");
        panic!("secure random source unavailable for auth session id");
    }

    hex_encode(&bytes)
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn parse_remote_ip(remote_addr: &str) -> Option<IpAddr> {
    if let Ok(socket_addr) = remote_addr.parse::<SocketAddr>() {
        return Some(socket_addr.ip());
    }

    remote_addr.parse::<IpAddr>().ok()
}

fn cidr_contains(cidr: &str, remote_ip: IpAddr) -> Option<bool> {
    let (network, prefix) = cidr.split_once('/')?;
    let network = network.trim().parse::<IpAddr>().ok()?;
    let prefix = prefix.trim().parse::<u8>().ok()?;

    match (network, remote_ip) {
        (IpAddr::V4(network), IpAddr::V4(remote_ip)) if prefix <= 32 => {
            let mask = prefix_mask_v4(prefix);
            Some((u32::from(network) & mask) == (u32::from(remote_ip) & mask))
        }
        (IpAddr::V6(network), IpAddr::V6(remote_ip)) if prefix <= 128 => {
            let mask = prefix_mask_v6(prefix);
            Some((u128::from(network) & mask) == (u128::from(remote_ip) & mask))
        }
        (IpAddr::V4(_), IpAddr::V4(_)) | (IpAddr::V6(_), IpAddr::V6(_)) => None,
        _ => Some(false),
    }
}

fn prefix_mask_v4(prefix: u8) -> u32 {
    if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    }
}

fn prefix_mask_v6(prefix: u8) -> u128 {
    if prefix == 0 {
        0
    } else {
        u128::MAX << (128 - prefix)
    }
}

/// Builds a login-style response with `null` data for non-login endpoints.
pub fn empty_login_response(message: impl Into<String>) -> LoginResponse<serde_json::Value> {
    LoginResponse {
        code: 200,
        msg: message.into(),
        data: serde_json::Value::Null,
    }
}
