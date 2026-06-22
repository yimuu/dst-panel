//! Application error type and HTTP response mapping for migrated Rust routes.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

use crate::{infra::config::ConfigError, infra::logging::LoggingError, web::response::ApiResponse};

/// Result alias for functions that return an [`AppError`].
pub type AppResult<T> = Result<T, AppError>;

/// Errors raised by request handlers and shared application setup code.
#[derive(Debug, Error)]
pub enum AppError {
    /// Request input failed validation.
    #[error("{0}")]
    BadRequest(String),
    /// The request is missing valid authentication.
    #[error("{0}")]
    Unauthorized(String),
    /// The requested resource was not found.
    #[error("{0}")]
    NotFound(String),
    /// The request conflicts with existing state.
    #[error("{0}")]
    Conflict(String),
    /// Request output would exceed the route's configured safety limit.
    #[error("{0}")]
    PayloadTooLarge(String),
    /// The route is registered for compatibility but not migrated yet.
    #[error("{0}")]
    NotImplemented(String),
    /// A non-recoverable application error occurred.
    #[error("{0}")]
    Internal(String),
    /// Filesystem or operating-system I/O failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Configuration loading failed.
    #[error(transparent)]
    Config(#[from] ConfigError),
    /// Logging setup failed.
    #[error(transparent)]
    Logging(#[from] LoggingError),
}

impl AppError {
    /// Creates a `400 Bad Request` application error.
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest(message.into())
    }

    /// Creates a `401 Unauthorized` application error.
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::Unauthorized(message.into())
    }

    /// Creates a `404 Not Found` application error.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }

    /// Creates a `409 Conflict` application error.
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict(message.into())
    }

    /// Creates a `413 Payload Too Large` application error.
    pub fn payload_too_large(message: impl Into<String>) -> Self {
        Self::PayloadTooLarge(message.into())
    }

    /// Creates a `501 Not Implemented` compatibility-stub error.
    pub fn not_implemented(message: impl Into<String>) -> Self {
        Self::NotImplemented(message.into())
    }

    /// Creates a `500 Internal Server Error` application error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    /// Returns the HTTP status that should wrap this error response.
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::PayloadTooLarge(_) => StatusCode::PAYLOAD_TOO_LARGE,
            Self::NotImplemented(_) => StatusCode::NOT_IMPLEMENTED,
            Self::Internal(_) | Self::Io(_) | Self::Config(_) | Self::Logging(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    fn client_message(&self) -> String {
        match self {
            Self::BadRequest(_)
            | Self::Unauthorized(_)
            | Self::NotFound(_)
            | Self::Conflict(_)
            | Self::PayloadTooLarge(_)
            | Self::NotImplemented(_) => self.to_string(),
            // Internal variants can contain filesystem paths or setup details.
            // Preserve them in structured logs, but keep the API response stable
            // and non-sensitive for clients.
            Self::Internal(_) | Self::Io(_) | Self::Config(_) | Self::Logging(_) => {
                "internal server error".to_owned()
            }
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let msg = self.client_message();
        if status.is_server_error() {
            tracing::error!(status = status.as_u16(), error = %self, "internal application error");
        }
        let body = ApiResponse::error(i32::from(status.as_u16()), msg);
        (status, Json(body)).into_response()
    }
}
