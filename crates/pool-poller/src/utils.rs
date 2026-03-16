//! Utility functions for pool poller

/// Q64 constant (2^64)
pub const Q64: u128 = 1u128 << 64;

/// Convert sqrt_price (Q64.64) to human readable price
/// 
/// For SOL/USDC where SOL=9 decimals, USDC=6 decimals:
/// price = (sqrt_price / 2^64)^2 * 10^(decimals_a - decimals_b)
pub fn sqrt_price_to_price(sqrt_price: u128, decimals_a: u8, decimals_b: u8) -> f64 {
    if sqrt_price == 0 {
        return 0.0;
    }
    
    let sqrt_price_f = sqrt_price as f64 / (Q64 as f64);
    let price = sqrt_price_f * sqrt_price_f;
    
    // Adjust for decimal difference
    let decimal_adjustment = 10f64.powi(decimals_a as i32 - decimals_b as i32);
    
    price * decimal_adjustment
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqrt_price_conversion() {
        // For SOL/USDC at $220
        // raw_price = 220 / 10^3 = 0.22 (because SOL=9, USDC=6)
        // sqrt(0.22) ≈ 0.469
        // sqrt_price = 0.469 * 2^64 ≈ 8.65e18
        
        let sqrt_price = 8_650_000_000_000_000_000u128;
        let price = sqrt_price_to_price(sqrt_price, 9, 6);
        
        println!("sqrt_price {} -> price {}", sqrt_price, price);
        assert!(price > 200.0 && price < 250.0);
    }
}

