//! Quote engine implementation

use crate::amm::{get_amount_out_clmm, calculate_price_impact, AmmError};
use bot_core::{ParsedPool, PoolType, Quote, SwapDirection};
use parking_lot::RwLock;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PricingError {
    #[error("Pool not found: {0}")]
    PoolNotFound(String),

    #[error("AMM error: {0}")]
    AmmError(#[from] AmmError),

    #[error("Unsupported pool type")]
    UnsupportedPoolType,
}

pub struct PricingEngine {
    pools: Arc<RwLock<HashMap<Pubkey, ParsedPool>>>,
}

impl PricingEngine {
    pub fn new() -> Self {
        Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Update pool state
    pub fn update_pool(&self, pool: ParsedPool) {
        let mut pools = self.pools.write();
        pools.insert(pool.address, pool);
    }

    /// Remove pool
    pub fn remove_pool(&self, address: &Pubkey) {
        let mut pools = self.pools.write();
        pools.remove(address);
    }

    /// Get quote for a swap
    pub fn quote(
        &self,
        pool_address: &Pubkey,
        amount_in: u64,
        direction: SwapDirection,
    ) -> Result<Quote, PricingError> {
        let pools = self.pools.read();
        let pool = pools
            .get(pool_address)
            .ok_or_else(|| PricingError::PoolNotFound(pool_address.to_string()))?;

        let a_to_b = direction == SwapDirection::AtoB;

        let amount_out = match pool.pool_type {
            PoolType::OrcaWhirlpool | PoolType::RaydiumClmm => {
                get_amount_out_clmm(
                    pool.sqrt_price,
                    pool.liquidity,
                    amount_in,
                    a_to_b,
                    pool.fee_rate_bps,
                )?
            }
            _ => return Err(PricingError::UnsupportedPoolType),
        };

        let price_impact = calculate_price_impact(amount_in, amount_out, pool.sqrt_price);
        let fee_amount = (amount_in as u128 * pool.fee_rate_bps as u128 / 10000) as u64;

        // Calculate minimum out with 0.5% slippage
        let minimum_out = amount_out * 995 / 1000;

        Ok(Quote {
            pool: *pool_address,
            amount_in,
            amount_out,
            price_impact_bps: price_impact,
            fee_amount,
            minimum_out,
        })
    }

    /// Get all tracked pools
    pub fn get_all_pools(&self) -> Vec<ParsedPool> {
        let pools = self.pools.read();
        pools.values().cloned().collect()
    }

    /// Get pool count
    pub fn pool_count(&self) -> usize {
        self.pools.read().len()
    }
}

impl Default for PricingEngine {
    fn default() -> Self {
        Self::new()
    }
}

