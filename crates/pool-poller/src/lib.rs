//! Pool Poller Service
//!
//! Continuously polls DEX pools for state changes and feeds data
//! to the deserializer and pricing engine.
//!
//! # Features
//!
//! - Configurable polling intervals
//! - Batch RPC requests for efficiency
//! - Automatic pool discovery from Orca/Raydium/Jupiter APIs
//! - State change detection
//! - Yellowstone gRPC streaming (Helius)

pub mod config;
pub mod discovery;
pub mod poller;
pub mod registry;
pub mod streaming;
pub mod utils;

pub use config::PollerConfig;
pub use discovery::PoolDiscovery;
pub use poller::PoolPoller;
pub use registry::PoolRegistry;
pub use streaming::{YellowstoneConfig, YellowstoneStreamer, PoolUpdate};

