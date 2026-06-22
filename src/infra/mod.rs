//! Infrastructure boundaries for the Rust backend.
//!
//! These modules isolate external systems and process-wide services from
//! domain logic: command execution, HTTP, SQLite, configuration, logging,
//! process snapshots, and safe filesystem primitives.

pub mod command;
pub mod config;
pub mod db;
pub mod fs_paths;
pub mod http_client;
pub mod logging;
pub mod process;
