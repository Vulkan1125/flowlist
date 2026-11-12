//! Flowlist core: domain model, commands, persistence and queries.
//!
//! This crate deliberately has no UI dependencies so the CLI, MCP server and
//! GPUI application all share one implementation of the rules.

// Recursive filter trees plus serde's tagged-enum serializer need headroom.
#![recursion_limit = "512"]

pub mod command;
pub mod db;
pub mod error;
pub mod export;
pub mod ids;
pub mod model;
pub mod parse;
pub mod paths;
pub mod persist;
pub mod query;
pub mod search;
pub mod stats;
pub mod store;
pub mod taskpaper;
pub mod time;

pub use error::{Error, Result};
