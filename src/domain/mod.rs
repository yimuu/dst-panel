//! Feature domains for the DST admin backend.
//!
//! Domain modules contain business behavior and DTOs that are independent from
//! Axum route assembly. Handlers call into these modules; infrastructure is
//! passed in through fakeable traits.

pub mod admin;
pub mod auth;
pub mod backup;
pub mod cluster;
pub mod game;
pub mod map;
pub mod mods;
pub mod scheduler;
pub mod statistics;
