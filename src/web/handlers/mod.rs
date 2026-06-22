//! Axum handler modules for migrated HTTP routes.

pub mod announcement;
pub mod auth;
pub mod auto_check;
pub mod backup;
pub mod cluster;
pub mod compat;
pub mod dst_config;
pub mod dst_static;
pub mod files;
pub mod game;
pub mod init;
pub mod install;
pub mod kv;
pub mod level;
pub mod logs;
pub mod map;
pub mod mods;
pub mod player;
pub mod player_log;
pub mod share;
pub mod static_files;
pub mod statistics;
pub mod steam_news;
pub mod streams;
pub mod tasks;
pub mod third_party;
pub mod web_link;
pub mod webhook;
pub mod ws;

use serde_json::Value;

use crate::{web::error::AppError, web::response::LoginResponse};

pub(crate) fn legacy_success<T>(data: T) -> LoginResponse<T> {
    LoginResponse {
        code: 200,
        msg: "success".to_owned(),
        data,
    }
}

pub(crate) fn legacy_empty_success() -> LoginResponse<Value> {
    legacy_success(Value::Null)
}

pub(crate) fn response_with_message<T>(msg: impl Into<String>, data: T) -> LoginResponse<T> {
    LoginResponse {
        code: 200,
        msg: msg.into(),
        data,
    }
}

pub(crate) fn repository_error(operation: &'static str, error: sqlx::Error) -> AppError {
    let error_kind = sqlx_error_kind(&error);
    tracing::error!(operation, error_kind, "repository operation failed");

    if is_unique_constraint_error(&error) {
        return AppError::conflict("record already exists");
    }

    match error {
        sqlx::Error::RowNotFound => AppError::not_found("record not found"),
        sqlx::Error::Protocol(message) => AppError::bad_request(message),
        _ => AppError::internal(operation),
    }
}

fn sqlx_error_kind(error: &sqlx::Error) -> &'static str {
    match error {
        sqlx::Error::Configuration(_) => "configuration",
        sqlx::Error::Database(_) => "database",
        sqlx::Error::Io(_) => "io",
        sqlx::Error::Tls(_) => "tls",
        sqlx::Error::Protocol(_) => "protocol",
        sqlx::Error::RowNotFound => "row_not_found",
        sqlx::Error::TypeNotFound { .. } => "type_not_found",
        sqlx::Error::ColumnIndexOutOfBounds { .. } => "column_index_out_of_bounds",
        sqlx::Error::ColumnNotFound(_) => "column_not_found",
        sqlx::Error::ColumnDecode { .. } => "column_decode",
        sqlx::Error::Decode(_) => "decode",
        sqlx::Error::PoolTimedOut => "pool_timed_out",
        sqlx::Error::PoolClosed => "pool_closed",
        sqlx::Error::WorkerCrashed => "worker_crashed",
        sqlx::Error::Migrate(_) => "migrate",
        _ => "unknown",
    }
}

fn is_unique_constraint_error(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .is_some_and(|db_error| db_error.message().contains("UNIQUE constraint failed"))
}
