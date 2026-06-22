//! HTTP-facing application layer.
//!
//! `web` owns Axum app assembly, handlers, legacy response envelopes, and
//! web-facing errors. Domain and infrastructure modules must not depend on
//! concrete Axum router assembly.

pub mod app;
pub mod error;
pub mod handlers;
pub mod response;
