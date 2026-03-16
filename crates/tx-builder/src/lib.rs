//! Transaction Builder
//!
//! Builds and signs swap transactions using our on-chain aggregator.

pub mod aggregator;
pub mod builder;
pub mod dex_accounts;
pub mod jito;
pub mod simulator;

pub use aggregator::{
    build_execute_arbitrage_ix, build_initialize_ix, get_config_pda,
    route_to_swap_legs, DexType, SwapLeg, AGGREGATOR_PROGRAM_ID,
};
pub use builder::TxBuilder;
pub use dex_accounts::{OrcaWhirlpoolAccounts, RaydiumClmmAccounts};
pub use jito::{BundleBuilder, BundleResult, BundleStatus, JitoClient, JitoConfig, JitoBundle};
pub use simulator::Simulator;

