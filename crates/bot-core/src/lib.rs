//! Bot Core - Shared types, configuration, and utilities
//!
//! This crate contains common types used across all services.

pub mod config;
pub mod error;
pub mod types;

pub use config::BotConfig;
pub use error::BotError;
pub use types::*;

