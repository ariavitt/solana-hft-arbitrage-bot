//! Poller configuration

use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Pool poller configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollerConfig {
    /// Polling interval in milliseconds
    pub poll_interval_ms: u64,
    
    /// Maximum accounts per RPC batch
    pub batch_size: usize,
    
    /// Enable automatic pool discovery
    pub auto_discover: bool,
    
    /// Tokens to track (for discovery)
    pub tracked_tokens: Vec<String>,
    
    /// Minimum liquidity in USD for a pool to be tracked
    pub min_liquidity_usd: u64,
}

impl Default for PollerConfig {
    fn default() -> Self {
        Self {
            poll_interval_ms: 100, // 100ms = 10 Hz
            batch_size: 100,
            auto_discover: false,
            tracked_tokens: vec![
                // SOL
                "So11111111111111111111111111111111111111112".to_string(),
                // USDC
                "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
                // USDT
                "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB".to_string(),
            ],
            min_liquidity_usd: 10_000,
        }
    }
}

/// Well-known pool addresses for devnet testing
pub mod devnet_pools {
    use super::*;

    /// Orca Whirlpool: SOL/USDC on devnet
    pub fn orca_sol_usdc() -> Pubkey {
        // Orca Whirlpool SOL/USDC devnet (well-known public test pool)
        // Source: https://docs.orca.so/developers/addresses (devnet)
        Pubkey::from_str("H56drEq5etogmADhw5uW5J5WAZ1BfMi2jXxbHMP8n33N").unwrap()
    }

    /// Raydium CLMM pool on devnet (SOL/USDC)
    pub fn raydium_sol_usdc() -> Pubkey {
        // Reference devnet pool address for Raydium CLMM (public test)
        // If unavailable, update with a live devnet CLMM pool
        Pubkey::from_str("8TF86uXEMQ8cq3TnRooHS7zzkTz8ybDW14De3LVctk7r").unwrap()
    }
}

/// Well-known pool addresses for mainnet
pub mod mainnet_pools {
    use super::*;

    /// Orca Whirlpool: SOL/USDC
    pub fn orca_sol_usdc() -> Pubkey {
        Pubkey::from_str("HJPjoWUrhoZzkNfRpHuieeFk9WcZWjwy6PBjZ81ngndJ").unwrap()
    }

    /// Raydium CLMM: SOL/USDC
    pub fn raydium_sol_usdc() -> Pubkey {
        Pubkey::from_str("2QdhepnKRTLjjSqPL1PtKNwqrUkoLee5Gqs8bvZhRdMv").unwrap()
    }

    /// Orca Whirlpool: SOL/USDT
    pub fn orca_sol_usdt() -> Pubkey {
        Pubkey::from_str("4fuUiYxTQ6QCrdSq9ouBYcTM7bqSwYTSyLueGZLTy4T4").unwrap()
    }
}

