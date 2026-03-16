//! AMM math implementations for CLMM (Concentrated Liquidity)
//!
//! Handles price calculations for Orca Whirlpools and Raydium CLMM

use thiserror::Error;
use uint::construct_uint;

construct_uint! {
    pub struct U256(4);
}

/// Q64 constant (2^64)
pub const Q64: u128 = 1u128 << 64;

#[derive(Error, Debug)]
pub enum AmmError {
    #[error("Insufficient liquidity")]
    InsufficientLiquidity,

    #[error("Price impact too high")]
    PriceImpactTooHigh,

    #[error("Math overflow")]
    Overflow,

    #[error("Division by zero")]
    DivisionByZero,
}

/// Calculate output amount for concentrated liquidity AMM
/// 
/// # Arguments
/// * `sqrt_price` - sqrt(price) * 2^64 (Q64.64 format)
/// * `liquidity` - Current pool liquidity
/// * `amount_in` - Input amount in native units (with decimals)
/// * `a_to_b` - Direction: true = token_a -> token_b
/// * `fee_rate_bps` - Fee in basis points (e.g., 30 = 0.3%)
/// * `decimals_a` - Decimals of token A
/// * `decimals_b` - Decimals of token B
pub fn get_amount_out_clmm(
    sqrt_price: u128,
    liquidity: u128,
    amount_in: u64,
    a_to_b: bool,
    fee_rate_bps: u16,
) -> Result<u64, AmmError> {
    if liquidity == 0 {
        return Err(AmmError::InsufficientLiquidity);
    }

    if sqrt_price == 0 {
        return Err(AmmError::DivisionByZero);
    }

    // Apply fee
    let fee_factor = 10000u128 - fee_rate_bps as u128;
    let amount_in_after_fee = (amount_in as u128 * fee_factor) / 10000;

    // For CLMM, we use the constant product formula within the current tick range
    // Price = (sqrt_price / Q64)^2 = token_b_per_token_a
    // 
    // A -> B: amount_out_b = amount_in_a * price
    // B -> A: amount_out_a = amount_in_b / price
    
    let amount_out = if a_to_b {
        // Selling token A for token B
        // amount_out = amount_in * (sqrt_price^2 / Q128)
        calculate_b_output_v2(sqrt_price, amount_in_after_fee)?
    } else {
        // Selling token B for token A  
        // amount_out = amount_in / (sqrt_price^2 / Q128)
        calculate_a_output_v2(sqrt_price, amount_in_after_fee)?
    };

    Ok(amount_out as u64)
}

/// Calculate B output when swapping A -> B
/// Formula: amount_out_b = amount_in_a * price
/// Where: price = (sqrt_price / 2^64)^2 = sqrt_price^2 / 2^128
fn calculate_b_output_v2(sqrt_price: u128, amount_in: u128) -> Result<u128, AmmError> {
    let sqrt_price_u256 = U256::from(sqrt_price);
    let amount_u256 = U256::from(amount_in);
    
    // price_q128 = sqrt_price^2 (this gives price * 2^128)
    let price_q128 = sqrt_price_u256 * sqrt_price_u256;
    
    // amount_out = amount_in * price = amount_in * sqrt_price^2 / 2^128
    let result = (amount_u256 * price_q128) >> 128;
    
    if result > U256::from(u128::MAX) {
        return Err(AmmError::Overflow);
    }
    
    Ok(result.as_u128())
}

/// Calculate A output when swapping B -> A
/// Formula: amount_out_a = amount_in_b / price  
/// Where: price = sqrt_price^2 / 2^128
fn calculate_a_output_v2(sqrt_price: u128, amount_in: u128) -> Result<u128, AmmError> {
    let sqrt_price_u256 = U256::from(sqrt_price);
    let amount_u256 = U256::from(amount_in);
    
    // price_q128 = sqrt_price^2
    let price_q128 = sqrt_price_u256 * sqrt_price_u256;
    
    if price_q128.is_zero() {
        return Err(AmmError::DivisionByZero);
    }
    
    // amount_out = amount_in * 2^128 / sqrt_price^2
    let result = (amount_u256 << 128) / price_q128;
    
    if result > U256::from(u128::MAX) {
        return Err(AmmError::Overflow);
    }
    
    Ok(result.as_u128())
}

/// Convert sqrt_price (Q64.64) to human readable price
/// 
/// For SOL/USDC where SOL=9 decimals, USDC=6 decimals:
/// price = (sqrt_price / 2^64)^2 * 10^(decimals_a - decimals_b)
pub fn sqrt_price_to_price(sqrt_price: u128, decimals_a: u8, decimals_b: u8) -> f64 {
    let sqrt_price_f = sqrt_price as f64 / (Q64 as f64);
    let price = sqrt_price_f * sqrt_price_f;
    
    // Adjust for decimal difference
    let decimal_adjustment = 10f64.powi(decimals_a as i32 - decimals_b as i32);
    
    price * decimal_adjustment
}

/// Convert human readable price to sqrt_price (Q64.64)
pub fn price_to_sqrt_price(price: f64, decimals_a: u8, decimals_b: u8) -> u128 {
    // Adjust for decimal difference
    let decimal_adjustment = 10f64.powi(decimals_a as i32 - decimals_b as i32);
    let adjusted_price = price / decimal_adjustment;
    
    let sqrt_price_f = adjusted_price.sqrt() * (Q64 as f64);
    sqrt_price_f as u128
}

/// Calculate price impact in basis points
pub fn calculate_price_impact(
    amount_in: u64,
    amount_out: u64,
    sqrt_price: u128,
) -> u16 {
    if amount_in == 0 || sqrt_price == 0 {
        return 0;
    }

    // Expected output at spot price
    let spot_price_q128 = U256::from(sqrt_price) * U256::from(sqrt_price);
    let expected = (U256::from(amount_in) * spot_price_q128) >> 128;
    
    let expected_u64 = expected.as_u128() as u64;
    
    if expected_u64 == 0 {
        return 0;
    }

    let impact = if amount_out < expected_u64 {
        ((expected_u64 - amount_out) as u128 * 10000 / expected_u64 as u128) as u16
    } else {
        0
    };

    impact.min(10000) // Cap at 100%
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clmm_swap_price_1() {
        // Price = 1 (1:1 ratio, same decimals)
        let sqrt_price = Q64; // sqrt(1) * 2^64
        let liquidity = 1_000_000_000_000u128;
        let amount_in = 1_000_000u64;
        let fee_bps = 30; // 0.3%

        let result = get_amount_out_clmm(sqrt_price, liquidity, amount_in, true, fee_bps);
        assert!(result.is_ok());
        
        let amount_out = result.unwrap();
        // Should be ~997000 (1M - 0.3% fee)
        assert!(amount_out > 990_000);
        assert!(amount_out < 1_000_000);
    }

    #[test]
    fn test_sqrt_price_to_price() {
        // SOL/USDC: price ~$220, SOL=9 dec, USDC=6 dec
        // Raw price in pool terms = 220 / 10^3 = 0.22
        // sqrt(0.22) * 2^64 ≈ 8.67e18
        
        let sqrt_price = 8_663_000_000_000_000_000u128;
        let price = sqrt_price_to_price(sqrt_price, 9, 6);
        
        // Should be around 220
        assert!(price > 200.0);
        assert!(price < 250.0);
    }

    #[test]
    fn test_price_to_sqrt_price() {
        // Convert $220 SOL/USDC to sqrt_price
        let sqrt_price = price_to_sqrt_price(220.0, 9, 6);
        let price_back = sqrt_price_to_price(sqrt_price, 9, 6);
        
        assert!((price_back - 220.0).abs() < 1.0);
    }

    #[test]
    fn test_sol_usdc_swap() {
        // Realistic SOL/USDC swap
        // Price ~$220, so sqrt_price = sqrt(0.22) * 2^64
        let sqrt_price = price_to_sqrt_price(220.0, 9, 6);
        let liquidity = 10_000_000_000_000u128;
        let amount_in = 100_000_000u64; // 0.1 SOL
        let fee_bps = 30;

        // Swap SOL -> USDC
        let usdc_out = get_amount_out_clmm(sqrt_price, liquidity, amount_in, true, fee_bps).unwrap();
        
        // Should get ~21.93 USDC (0.1 * 220 * 0.997)
        let usdc_f = usdc_out as f64 / 1_000_000.0;
        println!("0.1 SOL -> {} USDC", usdc_f);
        
        assert!(usdc_f > 20.0);
        assert!(usdc_f < 25.0);
    }

    #[test]
    fn test_usdc_sol_swap() {
        // Swap USDC -> SOL
        let sqrt_price = price_to_sqrt_price(220.0, 9, 6);
        let liquidity = 10_000_000_000_000u128;
        let amount_in = 22_000_000u64; // $22 USDC
        let fee_bps = 30;

        // Swap USDC -> SOL (b_to_a)
        let sol_out = get_amount_out_clmm(sqrt_price, liquidity, amount_in, false, fee_bps).unwrap();
        
        // Should get ~0.0997 SOL (22 / 220 * 0.997)
        let sol_f = sol_out as f64 / 1_000_000_000.0;
        println!("$22 USDC -> {} SOL", sol_f);
        
        assert!(sol_f > 0.09);
        assert!(sol_f < 0.11);
    }
}
