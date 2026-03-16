//! Error types for the bot

use thiserror::Error;

#[derive(Error, Debug)]
pub enum BotError {
    // RPC errors
    #[error("RPC connection failed: {0}")]
    RpcConnection(String),

    #[error("RPC timeout")]
    RpcTimeout,

    #[error("Rate limited")]
    RateLimited,

    // Deserialization errors
    #[error("Invalid pool data: {0}")]
    InvalidPoolData(String),

    #[error("Unknown pool type")]
    UnknownPoolType,

    // Pricing errors
    #[error("Insufficient liquidity")]
    InsufficientLiquidity,

    #[error("Price impact too high: {0} bps")]
    PriceImpactTooHigh(u16),

    #[error("Pool not found: {0}")]
    PoolNotFound(String),

    // Simulation errors
    #[error("Simulation failed: {0}")]
    SimulationFailed(String),

    #[error("Insufficient funds")]
    InsufficientFunds,

    // Execution errors
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Slippage exceeded")]
    SlippageExceeded,

    #[error("Bundle rejected")]
    BundleRejected,

    // Generic errors
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

pub type Result<T> = std::result::Result<T, BotError>;

