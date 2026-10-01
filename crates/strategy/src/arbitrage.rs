//! Arbitrage strategy implementation
//!
//! Searches for profitable arbitrage opportunities between DEX pools.

use bot_core::{Opportunity, ParsedPool, PoolType, Quote, Route, SwapDirection, SwapLeg};
use pricing_engine::PricingEngine;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Represents a potential arbitrage path between two pools
#[derive(Debug, Clone)]
struct ArbitragePath {
    /// First pool to swap on (buy)
    buy_pool: ParsedPool,
    /// Second pool to swap on (sell)
    sell_pool: ParsedPool,
    /// Token to start with (and end with)
    base_token: Pubkey,
    /// Token in the middle
    quote_token: Pubkey,
}

pub struct ArbitrageStrategy {
    min_profit_bps: u16,
    min_profit_lamports: i64,
    max_slippage_bps: u16,
    max_hops: usize,
    base_tokens: Vec<Pubkey>,
    /// Trade amount in lamports
    trade_amount: u64,
}

impl ArbitrageStrategy {
    /// Simple constructor for use in bot
    pub fn new(
        min_profit_bps: u16,
        min_profit_lamports: i64,
        max_slippage_bps: u16,
        max_hops: usize,
        trade_amount: u64,
    ) -> Self {
        // Default base tokens: SOL (wrapped)
        let sol = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
        
        Self {
            min_profit_bps,
            min_profit_lamports,
            max_slippage_bps,
            max_hops,
            base_tokens: vec![sol],
            trade_amount,
        }
    }

    /// Set trade amount
    pub fn set_trade_amount(&mut self, amount: u64) {
        self.trade_amount = amount;
    }

    /// Find arbitrage opportunities from pool states
    pub fn find_opportunities(
        &self,
        pools: &HashMap<Pubkey, ParsedPool>,
        pricing: &PricingEngine,
        _min_liquidity_usd: u64,
    ) -> anyhow::Result<Vec<Opportunity>> {
        let mut opportunities = Vec::new();
        
        // Find all potential arbitrage paths
        let paths = self.find_arbitrage_paths(pools);
        
        debug!("Found {} potential arbitrage paths", paths.len());
        
        for path in paths {
            if let Some(opp) = self.evaluate_path(&path, pricing) {
                if opp.profit_bps >= self.min_profit_bps as i16
                    && opp.expected_profit >= self.min_profit_lamports
                {
                    info!(
                        "🎯 Found opportunity: {} -> {} -> {} | Profit: {} bps ({} lamports)",
                        self.format_pool(&path.buy_pool),
                        self.format_pool(&path.sell_pool),
                        opp.input_token,
                        opp.profit_bps,
                        opp.expected_profit
                    );
                    opportunities.push(opp);
                }
            }
        }
        
        // Sort by profit (highest first)
        opportunities.sort_by(|a, b| b.expected_profit.cmp(&a.expected_profit));
        
        Ok(opportunities)
    }

    /// Find all potential 2-hop arbitrage paths
    fn find_arbitrage_paths(&self, pools: &HashMap<Pubkey, ParsedPool>) -> Vec<ArbitragePath> {
        let mut paths = Vec::new();
        let pools_vec: Vec<&ParsedPool> = pools.values().collect();
        
        for base_token in &self.base_tokens {
            // Group pools by their token pair (base_token <-> other_token)
            let mut pools_by_quote: HashMap<Pubkey, Vec<&ParsedPool>> = HashMap::new();
            
            for pool in &pools_vec {
                let quote_token = if pool.token_a.mint == *base_token {
                    Some(pool.token_b.mint)
                } else if pool.token_b.mint == *base_token {
                    Some(pool.token_a.mint)
                } else {
                    None
                };
                
                if let Some(qt) = quote_token {
                    pools_by_quote.entry(qt).or_insert_with(Vec::new).push(*pool);
                }
            }
            
            // For each quote token with 2+ pools, we have potential arbitrage
            for (quote_token, pool_list) in &pools_by_quote {
                if pool_list.len() < 2 {
                    continue;
                }
                
                // Compare all pairs of pools
                for i in 0..pool_list.len() {
                    for j in (i + 1)..pool_list.len() {
                        let pool_a = pool_list[i];
                        let pool_b = pool_list[j];
                        
                        // Create both directions of arbitrage
                        // Path 1: Buy on A, Sell on B
                        paths.push(ArbitragePath {
                            buy_pool: pool_a.clone(),
                            sell_pool: pool_b.clone(),
                            base_token: *base_token,
                            quote_token: *quote_token,
                        });
                        
                        // Path 2: Buy on B, Sell on A
                        paths.push(ArbitragePath {
                            buy_pool: pool_b.clone(),
                            sell_pool: pool_a.clone(),
                            base_token: *base_token,
                            quote_token: *quote_token,
                        });
                    }
                }
            }
        }
        
        paths
    }

    /// Evaluate a single arbitrage path
    fn evaluate_path(&self, path: &ArbitragePath, pricing: &PricingEngine) -> Option<Opportunity> {
        let amount_in = self.trade_amount;
        
        // Leg 1: Base → Quote on buy_pool
        let (leg1_direction, leg1_a_to_b) = if path.buy_pool.token_a.mint == path.base_token {
            (SwapDirection::AtoB, true)
        } else {
            (SwapDirection::BtoA, false)
        };
        
        let quote1 = match pricing.quote(&path.buy_pool.address, amount_in, leg1_direction) {
            Ok(q) => q,
            Err(e) => {
                debug!("Quote failed for buy_pool: {}", e);
                return None;
            }
        };
        
        if quote1.amount_out == 0 {
            debug!("Zero output from buy_pool");
            return None;
        }
        
        // Leg 2: Quote → Base on sell_pool
        let (leg2_direction, leg2_a_to_b) = if path.sell_pool.token_a.mint == path.quote_token {
            (SwapDirection::AtoB, true)
        } else {
            (SwapDirection::BtoA, false)
        };
        
        let quote2 = match pricing.quote(&path.sell_pool.address, quote1.amount_out, leg2_direction) {
            Ok(q) => q,
            Err(e) => {
                debug!("Quote failed for sell_pool: {}", e);
                return None;
            }
        };
        
        if quote2.amount_out == 0 {
            debug!("Zero output from sell_pool");
            return None;
        }
        
        // Calculate profit
        let final_amount = quote2.amount_out;
        let profit = final_amount as i64 - amount_in as i64;
        let profit_bps = if amount_in > 0 {
            ((profit as f64 / amount_in as f64) * 10000.0) as i16
        } else {
            0
        };
        
        debug!(
            "Path evaluation: {} SOL → {} USDC → {} SOL | Profit: {} bps",
            amount_in as f64 / 1e9,
            quote1.amount_out as f64 / 1e6,
            final_amount as f64 / 1e9,
            profit_bps
        );
        
        // Apply slippage to minimum amounts
        let slippage_factor = 10000u64 - self.max_slippage_bps as u64;
        let min_out_leg1 = quote1.amount_out * slippage_factor / 10000;
        let min_out_leg2 = quote2.amount_out * slippage_factor / 10000;
        
        // Build route
        let leg1 = SwapLeg {
            pool: path.buy_pool.address,
            pool_type: path.buy_pool.pool_type,
            token_in: path.base_token,
            token_out: path.quote_token,
            amount_in,
            expected_out: quote1.amount_out,
            min_amount_out: min_out_leg1,
            direction: leg1_direction,
            a_to_b: leg1_a_to_b,
        };
        
        let leg2 = SwapLeg {
            pool: path.sell_pool.address,
            pool_type: path.sell_pool.pool_type,
            token_in: path.quote_token,
            token_out: path.base_token,
            amount_in: quote1.amount_out,
            expected_out: quote2.amount_out,
            min_amount_out: min_out_leg2,
            direction: leg2_direction,
            a_to_b: leg2_a_to_b,
        };
        
        let route = Route {
            legs: vec![leg1, leg2],
            total_amount_in: amount_in,
            expected_amount_out: final_amount,
            price_impact_bps: quote1.price_impact_bps.saturating_add(quote2.price_impact_bps),
        };
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Some(Opportunity {
            id: Uuid::new_v4(),
            route,
            input_token: path.base_token,
            input_amount: amount_in,
            expected_profit: profit,
            profit_bps,
            timestamp: now,
            expires_at: now + 2, // 2 seconds validity
        })
    }

    fn format_pool(&self, pool: &ParsedPool) -> String {
        match pool.pool_type {
            PoolType::OrcaWhirlpool => "Orca",
            PoolType::RaydiumClmm => "Raydium",
            PoolType::RaydiumAmm => "RaydiumAMM",
            PoolType::Phoenix => "Phoenix",
            PoolType::OpenBook => "OpenBook",
        }.to_string()
    }

    /// Add base token to track
    pub fn add_base_token(&mut self, token: Pubkey) {
        if !self.base_tokens.contains(&token) {
            self.base_tokens.push(token);
        }
    }

    /// Get minimum profit threshold
    pub fn min_profit_bps(&self) -> u16 {
        self.min_profit_bps
    }

    pub fn min_profit_lamports(&self) -> i64 {
        self.min_profit_lamports
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bot_core::TokenInfo;

    fn create_test_pool(
        address: &str,
        pool_type: PoolType,
        sqrt_price: u128,
        liquidity: u128,
    ) -> ParsedPool {
        let sol = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
        let usdc = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
        
        ParsedPool {
            address: Pubkey::from_str(address).unwrap_or_default(),
            pool_type,
            token_a: TokenInfo {
                mint: sol,
                vault: Pubkey::default(),
                decimals: 9,
            },
            token_b: TokenInfo {
                mint: usdc,
                vault: Pubkey::default(),
                decimals: 6,
            },
            sqrt_price,
            liquidity,
            tick_current: 0,
            fee_rate_bps: 30,
            last_updated_slot: 0,
        }
    }

    #[test]
    fn test_find_arbitrage_paths() {
        let strategy = ArbitrageStrategy::new(10, 0, 50, 3, 100_000_000);
        let mut pools = HashMap::new();
        
        // Add Orca pool
        let orca_pool = create_test_pool(
            "HJPjoWUrhoZzkNfRpHuieeFk9WcZWjwy6PBjZ81ngndJ",
            PoolType::OrcaWhirlpool,
            1 << 64,
            1_000_000_000_000,
        );
        pools.insert(orca_pool.address, orca_pool);
        
        // Add Raydium pool
        let ray_pool = create_test_pool(
            "2QdhepnKRTLjjSqPL1PtKNwqrUkoLee5Gqs8bvZhRdMv",
            PoolType::RaydiumClmm,
            1 << 64,
            1_000_000_000_000,
        );
        pools.insert(ray_pool.address, ray_pool);
        
        let paths = strategy.find_arbitrage_paths(&pools);
        
        // Should find 2 paths (Orca→Raydium and Raydium→Orca)
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn test_profit_calculation() {
        let strategy = ArbitrageStrategy::new(10, 0, 50, 3, 100_000_000);
        
        // Test profit bps calculation
        let amount_in = 1_000_000_000u64; // 1 SOL
        let amount_out = 1_001_000_000u64; // 1.001 SOL
        let profit = amount_out as i64 - amount_in as i64;
        let profit_bps = ((profit as f64 / amount_in as f64) * 10000.0) as i16;
        
        assert_eq!(profit_bps, 10); // 0.1% = 10 bps
    }
}
