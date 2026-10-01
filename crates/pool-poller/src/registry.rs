//! Pool Registry
//!
//! Tracks known pools and their metadata

use bot_core::PoolType;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

/// Metadata about a tracked pool
#[derive(Debug, Clone)]
pub struct PoolInfo {
    pub address: Pubkey,
    pub pool_type: PoolType,
    pub token_a: Pubkey,
    pub token_b: Pubkey,
    pub name: String,
    pub enabled: bool,
}

/// Registry of pools to track
pub struct PoolRegistry {
    pools: HashMap<Pubkey, PoolInfo>,
}

impl PoolRegistry {
    pub fn new() -> Self {
        Self {
            pools: HashMap::new(),
        }
    }

    /// Add a pool to the registry
    pub fn add_pool(&mut self, info: PoolInfo) {
        self.pools.insert(info.address, info);
    }

    /// Remove a pool from the registry
    pub fn remove_pool(&mut self, address: &Pubkey) {
        self.pools.remove(address);
    }

    /// Get all pool addresses
    pub fn get_addresses(&self) -> Vec<Pubkey> {
        self.pools
            .values()
            .filter(|p| p.enabled)
            .map(|p| p.address)
            .collect()
    }

    /// Get pool info by address
    pub fn get_pool(&self, address: &Pubkey) -> Option<&PoolInfo> {
        self.pools.get(address)
    }

    /// Get all enabled pools
    pub fn get_enabled_pools(&self) -> Vec<&PoolInfo> {
        self.pools.values().filter(|p| p.enabled).collect()
    }

    /// Get pools by token pair
    pub fn get_pools_for_pair(&self, token_a: &Pubkey, token_b: &Pubkey) -> Vec<&PoolInfo> {
        self.pools
            .values()
            .filter(|p| {
                p.enabled
                    && ((p.token_a == *token_a && p.token_b == *token_b)
                        || (p.token_a == *token_b && p.token_b == *token_a))
            })
            .collect()
    }

    /// Get pools by type
    pub fn get_pools_by_type(&self, pool_type: PoolType) -> Vec<&PoolInfo> {
        self.pools
            .values()
            .filter(|p| p.enabled && p.pool_type == pool_type)
            .collect()
    }

    /// Number of registered pools
    pub fn len(&self) -> usize {
        self.pools.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.pools.is_empty()
    }

    /// Load default pools for the configured network
    pub fn load_defaults(&mut self, network: &str) {
        use crate::config::{devnet_pools, mainnet_pools};
        use std::str::FromStr;

        let sol_mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
        let usdc_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
        let usdt_mint = Pubkey::from_str("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB").unwrap();
        let use_devnet = network.eq_ignore_ascii_case("devnet");

        let (orca_sol_usdc, raydium_sol_usdc, network_label) = if use_devnet {
            (
                devnet_pools::orca_sol_usdc(),
                devnet_pools::raydium_sol_usdc(),
                "devnet",
            )
        } else {
            (
                mainnet_pools::orca_sol_usdc(),
                mainnet_pools::raydium_sol_usdc(),
                "mainnet",
            )
        };

        self.add_pool(PoolInfo {
            address: orca_sol_usdc,
            pool_type: PoolType::OrcaWhirlpool,
            token_a: sol_mint,
            token_b: usdc_mint,
            name: format!("Orca SOL/USDC ({})", network_label),
            enabled: true,
        });

        self.add_pool(PoolInfo {
            address: raydium_sol_usdc,
            pool_type: PoolType::RaydiumClmm,
            token_a: sol_mint,
            token_b: usdc_mint,
            name: format!("Raydium SOL/USDC ({})", network_label),
            enabled: true,
        });

        if !use_devnet {
            self.add_pool(PoolInfo {
                address: mainnet_pools::orca_sol_usdt(),
                pool_type: PoolType::OrcaWhirlpool,
                token_a: sol_mint,
                token_b: usdt_mint,
                name: format!("Orca SOL/USDT ({})", network_label),
                enabled: true,
            });
        }
    }
}

impl Default for PoolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
