//! Common types used across the bot

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

/// Pool type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub enum PoolType {
    OrcaWhirlpool,
    RaydiumClmm,
    RaydiumAmm,  // Classic Raydium AMM
    Phoenix,
    OpenBook,
}

/// Swap direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwapDirection {
    AtoB,
    BtoA,
}

/// Token information
#[derive(Debug, Clone, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct TokenInfo {
    pub mint: Pubkey,
    pub vault: Pubkey,
    pub decimals: u8,
}

/// Parsed pool state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPool {
    pub address: Pubkey,
    pub pool_type: PoolType,
    pub token_a: TokenInfo,
    pub token_b: TokenInfo,
    pub sqrt_price: u128,
    pub liquidity: u128,
    pub tick_current: i32,
    pub fee_rate_bps: u16,
    pub last_updated_slot: u64,
}

/// Quote result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub pool: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,
    pub price_impact_bps: u16,
    pub fee_amount: u64,
    pub minimum_out: u64,
}

/// Swap leg in a route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapLeg {
    pub pool: Pubkey,
    pub pool_type: PoolType,
    pub token_in: Pubkey,
    pub token_out: Pubkey,
    pub amount_in: u64,
    pub expected_out: u64,
    pub min_amount_out: u64,  // Slippage-adjusted minimum
    pub direction: SwapDirection,
    pub a_to_b: bool,  // Direction flag for CLMM pools
}

/// Complete swap route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub legs: Vec<SwapLeg>,
    pub total_amount_in: u64,
    pub expected_amount_out: u64,
    pub price_impact_bps: u16,
}

/// Arbitrage opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opportunity {
    pub id: uuid::Uuid,
    pub route: Route,
    pub input_token: Pubkey,
    pub input_amount: u64,
    pub expected_profit: i64,
    pub profit_bps: i16,
    pub timestamp: u64,
    pub expires_at: u64,
}

/// Simulation result
#[derive(Debug, Clone)]
pub struct SimulationResult {
    pub success: bool,
    pub compute_units: u64,
    pub logs: Vec<String>,
}

