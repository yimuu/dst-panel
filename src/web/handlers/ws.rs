//! WebSocket log tail route.
//!
//! The Go implementation allowed `tailf <filename>` to read any filesystem
//! path. Rust keeps the WebSocket handshake public for frontend compatibility,
//! but requires a valid admin session before serving `tailf` and then narrows
//! the target to the current cluster's DST log files. Public clients therefore
//! cannot read DST logs, `password.txt`, `dst_config`, cluster tokens, or
//! database files without an authenticated browser session.

use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    path::{Component, Path, PathBuf},
    time::Duration,
};

use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{
        HeaderMap, Uri,
        header::{HOST, ORIGIN},
    },
    response::Response,
};
use bytes::Bytes;

use crate::{
    domain::auth::SessionStore,
    dst,
    logs::{RecentLinesError, recent_lines_from_file},
    validation::validate_level_name,
    web::app::AppState,
    web::handlers::auth::extract_token_cookie,
};

const WS_POLL_INTERVAL: Duration = Duration::from_secs(1);
const WS_PING_INTERVAL: Duration = Duration::from_secs(54);
const WS_SNAPSHOT_LINES: usize = 100;
const MAX_WS_APPEND_BYTES: u64 = 256 * 1024;

pub(crate) async fn handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    websocket: WebSocketUpgrade,
) -> Response {
    // Browsers send cookies during the WebSocket upgrade, but they cannot
    // attach arbitrary Authorization headers. Keep the socket itself public for
    // compatibility and re-check this session before every sensitive `tailf`
    // operation so logout/expiry takes effect for already-open sockets.
    let tail_auth = TailAuth {
        sessions: state.sessions.clone(),
        session_id: extract_token_cookie(&headers),
        origin_allowed: tailf_origin_allowed(&headers),
    };
    let tail_authenticated = tail_auth.is_valid();
    tracing::debug!(
        tail_authenticated,
        "accepted websocket upgrade with tailf auth state"
    );
    websocket.on_upgrade(move |socket| handle_socket(socket, state.root_path, tail_auth))
}

#[derive(Clone, Debug)]
struct TailAuth {
    sessions: SessionStore,
    session_id: Option<String>,
    origin_allowed: bool,
}

impl TailAuth {
    fn is_valid(&self) -> bool {
        self.origin_allowed
            && self
                .session_id
                .as_deref()
                .and_then(|session_id| self.sessions.validate(session_id))
                .is_some()
    }
}

fn tailf_origin_allowed(headers: &HeaderMap) -> bool {
    let Some(origin) = headers.get(ORIGIN) else {
        // Non-browser clients commonly omit Origin. They still need a valid
        // session cookie, so keeping this path open preserves legacy tooling
        // without allowing browser-based cross-site log reads.
        return true;
    };
    let Ok(origin) = origin.to_str() else {
        tracing::warn!("rejected websocket tailf due to non-utf8 origin header");
        return false;
    };
    let Some(host) = headers.get(HOST).and_then(|value| value.to_str().ok()) else {
        tracing::warn!("rejected websocket tailf because host header is missing");
        return false;
    };
    let Ok(origin_uri) = origin.parse::<Uri>() else {
        tracing::warn!("rejected websocket tailf due to malformed origin header");
        return false;
    };
    if !matches!(origin_uri.scheme_str(), Some("http" | "https")) {
        tracing::warn!("rejected websocket tailf due to unsupported origin scheme");
        return false;
    }

    let allowed = origin_uri
        .authority()
        .is_some_and(|authority| authority.as_str().eq_ignore_ascii_case(host));
    if !allowed {
        tracing::warn!("rejected websocket tailf due to cross-origin websocket request");
    }
    allowed
}

async fn handle_socket(mut socket: WebSocket, root_path: PathBuf, tail_auth: TailAuth) {
    tracing::debug!("accepted websocket log-tail connection");
    while let Some(message) = socket.recv().await {
        match message {
            Ok(Message::Text(text)) if text == "byte" => {
                tracing::debug!("websocket client requested close");
                break;
            }
            Ok(Message::Text(text)) if text.starts_with("tailf") => {
                if !tail_auth.is_valid() {
                    tracing::warn!("rejected unauthenticated websocket tailf command");
                    continue;
                }
                let parts = text.split(' ').collect::<Vec<_>>();
                if parts.len() != 2 {
                    tracing::warn!("ignored malformed websocket tailf command");
                    continue;
                }
                if let Err(error) = tail_file(&mut socket, &root_path, parts[1], &tail_auth).await {
                    tracing::warn!(error = %error, "websocket tailf command stopped");
                }
            }
            Ok(Message::Close(_)) => break,
            Ok(_) => {}
            Err(error) => {
                tracing::debug!(error = %error, "websocket receive failed");
                break;
            }
        }
    }
    tracing::debug!("closed websocket log-tail connection");
}

async fn tail_file(
    socket: &mut WebSocket,
    root_path: &Path,
    requested_path: &str,
    tail_auth: &TailAuth,
) -> io::Result<()> {
    if !tail_auth.is_valid() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "websocket tailf session is not authenticated",
        ));
    }

    let target = tail_target(root_path, requested_path)?;
    let Some(mut file) = dst::safe_open_cluster_file(&target.cluster_dir, &target.relative_path)?
    else {
        return Ok(());
    };
    let mut offset = file.metadata()?.len();
    for line in last_lines_chronological(&mut file, WS_SNAPSHOT_LINES)? {
        if socket.send(Message::Text(line.into())).await.is_err() {
            return Ok(());
        }
    }

    let mut poll = tokio::time::interval(WS_POLL_INTERVAL);
    let mut ping = tokio::time::interval(WS_PING_INTERVAL);
    loop {
        tokio::select! {
            _ = poll.tick() => {
                if !tail_auth.is_valid() {
                    tracing::warn!("stopped websocket tailf after session became invalid");
                    return Ok(());
                }
                let lines = read_appended_lines(&target.cluster_dir, &target.relative_path, &mut offset)?;
                for line in lines {
                    if socket.send(Message::Text(line.into())).await.is_err() {
                        return Ok(());
                    }
                }
            }
            _ = ping.tick() => {
                if socket.send(Message::Ping(Bytes::new())).await.is_err() {
                    return Ok(());
                }
            }
            message = socket.recv() => {
                match message {
                    Some(Ok(Message::Text(text))) if text == "byte" => return Ok(()),
                    Some(Ok(Message::Close(_))) | None => return Ok(()),
                    Some(Ok(_)) => {}
                    Some(Err(error)) => {
                        return Err(io::Error::new(io::ErrorKind::ConnectionAborted, error));
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
struct TailTarget {
    cluster_dir: PathBuf,
    relative_path: PathBuf,
}

fn tail_target(root_path: &Path, requested_path: &str) -> io::Result<TailTarget> {
    let cluster_dir = dst::current_cluster_dir(root_path)?;
    let requested_relative = cluster_relative_tail_path(root_path, &cluster_dir, requested_path)?;
    let relative_path = validate_allowed_tail_path(&requested_relative)?;
    Ok(TailTarget {
        cluster_dir,
        relative_path,
    })
}

fn cluster_relative_tail_path(
    root_path: &Path,
    cluster_dir: &Path,
    requested_path: &str,
) -> io::Result<PathBuf> {
    let path = Path::new(requested_path);
    if path.is_absolute() {
        return path
            .strip_prefix(cluster_dir)
            .map(Path::to_path_buf)
            .map_err(|_| {
                io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "tail path is not an allowed DST log",
                )
            });
    }

    if let Ok(root_relative_cluster) = cluster_dir.strip_prefix(root_path)
        && let Ok(cluster_relative) = path.strip_prefix(root_relative_cluster)
    {
        return Ok(cluster_relative.to_path_buf());
    }

    Ok(path.to_path_buf())
}

fn validate_allowed_tail_path(relative_path: &Path) -> io::Result<PathBuf> {
    let components = relative_path.components().collect::<Vec<_>>();
    let [Component::Normal(level), Component::Normal(file_name)] = components.as_slice() else {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "tail path is not an allowed DST log",
        ));
    };
    let level = level
        .to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid level name"))?;
    let level = validate_level_name(level)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    let file_name = file_name
        .to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid log file name"))?;
    if !matches!(file_name, "server_log.txt" | "server_chat_log.txt") {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "tail path is not an allowed DST log",
        ));
    }

    Ok(Path::new(level.as_str()).join(file_name))
}

fn read_appended_lines(
    cluster_dir: &Path,
    relative_path: &Path,
    offset: &mut u64,
) -> io::Result<Vec<String>> {
    let Some(mut file) = dst::safe_open_cluster_file(cluster_dir, relative_path)? else {
        return Ok(Vec::new());
    };
    let len = file.metadata()?.len();
    if len < *offset {
        *offset = 0;
    }
    if len == *offset {
        return Ok(Vec::new());
    }

    file.seek(SeekFrom::Start(*offset))?;
    let read_limit = len.saturating_sub(*offset).min(MAX_WS_APPEND_BYTES);
    let mut bytes = Vec::with_capacity(read_limit as usize);
    file.take(read_limit).read_to_end(&mut bytes)?;
    *offset += bytes.len() as u64;
    let contents = String::from_utf8_lossy(&bytes);
    Ok(contents.lines().map(ToOwned::to_owned).collect())
}

fn last_lines_chronological(file: &mut File, limit: usize) -> io::Result<Vec<String>> {
    let mut lines = recent_lines_from_file(file, limit).map_err(recent_lines_error)?;
    lines.reverse();
    Ok(lines)
}

fn recent_lines_error(error: RecentLinesError) -> io::Error {
    match error {
        RecentLinesError::SnapshotTooLarge => io::Error::new(
            io::ErrorKind::InvalidData,
            "log snapshot exceeds safety limit",
        ),
        RecentLinesError::Io(error) => error,
    }
}
