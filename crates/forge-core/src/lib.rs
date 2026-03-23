//! `forge-core` — shared types, configuration, error handling, and project
//! management for the Daedalus ECU chip-tuning platform.
//!
//! This crate is a dependency of every other `forge-*` crate.  It intentionally
//! has no hardware or protocol logic — only data types and infrastructure.
#![deny(clippy::all)]

pub mod config;
pub mod error;
pub mod project;
pub mod types;

// Re-export the most commonly used items at crate root for convenience.
pub use error::{DaedalusError, Result};
