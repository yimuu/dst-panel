//! Cluster lifecycle support that is shared by cluster HTTP routes.
//!
//! This module owns persisted-cluster runtime enrichment and DST dedicated
//! server installation helpers. It stays separate from `domain::game`, which is
//! reserved for active shard/gameplay operations.

pub(crate) mod install;
pub mod model;
pub mod repository;
pub(crate) mod runtime;
