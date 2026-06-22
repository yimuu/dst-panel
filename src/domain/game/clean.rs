//! Game cleanup lifecycle helpers.
//!
//! Runtime cleanup routes currently live in the web route layer because they
//! are tightly coupled to HTTP query parsing. This module is reserved for the
//! domain cleanup implementation when those handlers move down.
