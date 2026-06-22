//! JSON response envelopes that preserve the current Go API contracts.
//!
//! Most endpoints use Go's `vo.Result`, where success is `code: 0`,
//! `msg: ""`, and missing data is an empty object. Login and user-management
//! endpoints use Go's older `vo.Response`, where success is `code: 200` with a
//! human message and unset error data serializes as `null`.

use serde::Serialize;
use serde_json::{Map, Value};

/// Go `vo.Result` compatible response body for most API routes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ApiResponse<T> {
    /// Go-compatible numeric result code.
    pub code: i32,
    /// Go-compatible message field, blank on general success.
    pub msg: String,
    /// Response payload.
    pub data: T,
}

impl<T> ApiResponse<T> {
    /// Builds a successful general API envelope with caller-provided data.
    pub fn success(data: T) -> Self {
        Self {
            code: 0,
            msg: String::new(),
            data,
        }
    }
}

impl ApiResponse<Value> {
    /// Builds a failed general API envelope with an empty-object payload.
    pub fn error(code: i32, msg: impl Into<String>) -> Self {
        Self {
            code,
            msg: msg.into(),
            data: empty_object(),
        }
    }

    /// Builds a successful general API envelope with Go's empty-object data.
    pub fn empty_success() -> Self {
        Self::success(empty_object())
    }
}

/// Go `vo.Response` compatible response body used by login-style routes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LoginResponse<T> {
    /// Go-compatible numeric response code.
    pub code: i32,
    /// Go-compatible human-readable message.
    pub msg: String,
    /// Response payload.
    pub data: T,
}

impl<T> LoginResponse<T> {
    /// Builds the successful login response emitted by the Go login service.
    pub fn success(data: T) -> Self {
        Self {
            code: 200,
            msg: "Login success".to_owned(),
            data,
        }
    }
}

impl LoginResponse<Value> {
    /// Builds a failed login-style response, preserving Go's unset data field.
    pub fn error(code: i32, msg: impl Into<String>) -> Self {
        Self {
            code,
            msg: msg.into(),
            data: Value::Null,
        }
    }
}

fn empty_object() -> Value {
    Value::Object(Map::new())
}
