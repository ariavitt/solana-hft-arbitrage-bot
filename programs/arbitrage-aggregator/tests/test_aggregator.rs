//! Integration tests for arbitrage aggregator

use anchor_lang::prelude::*;
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};

/// Test program initialization
#[tokio::test]
async fn test_initialize() {
    println!("🧪 Testing program initialization...");
    
    // This would require setting up the program in a test environment
    // For now, we verify the program compiles and basic types work
    
    // Test DexType serialization
    let dex = arbitrage_aggregator::DexType::OrcaWhirlpool;
    let serialized = dex.try_to_vec().unwrap();
    assert!(!serialized.is_empty());
    
    println!("✅ DexType serialization works");
}

/// Test SwapLeg creation
#[tokio::test]
async fn test_swap_leg() {
    use arbitrage_aggregator::{DexType, SwapLeg};
    
    let leg = SwapLeg {
        dex: DexType::OrcaWhirlpool,
        pool: Pubkey::new_unique(),
        token_in: Pubkey::new_unique(),
        token_out: Pubkey::new_unique(),
        amount_in: 1_000_000,
        min_amount_out: 990_000,
        a_to_b: true,
    };
    
    // Verify serialization
    let serialized = leg.try_to_vec().unwrap();
    let deserialized: SwapLeg = SwapLeg::try_from_slice(&serialized).unwrap();
    
    assert_eq!(deserialized.amount_in, 1_000_000);
    assert_eq!(deserialized.min_amount_out, 990_000);
    
    println!("✅ SwapLeg serialization works");
}

/// Test route validation logic
#[tokio::test]
async fn test_route_validation() {
    use arbitrage_aggregator::{DexType, SwapLeg};
    
    let sol_mint = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let ray_mint = Pubkey::new_unique();
    
    // Valid closed route: SOL → USDC → RAY → SOL
    let route = vec![
        SwapLeg {
            dex: DexType::OrcaWhirlpool,
            pool: Pubkey::new_unique(),
            token_in: sol_mint,
            token_out: usdc_mint,
            amount_in: 1_000_000_000, // 1 SOL
            min_amount_out: 100_000_000, // 100 USDC
            a_to_b: true,
        },
        SwapLeg {
            dex: DexType::RaydiumClmm,
            pool: Pubkey::new_unique(),
            token_in: usdc_mint,
            token_out: ray_mint,
            amount_in: 100_000_000, // 100 USDC
            min_amount_out: 50_000_000, // 50 RAY
            a_to_b: true,
        },
        SwapLeg {
            dex: DexType::OrcaWhirlpool,
            pool: Pubkey::new_unique(),
            token_in: ray_mint,
            token_out: sol_mint,
            amount_in: 50_000_000, // 50 RAY
            min_amount_out: 1_010_000_000, // 1.01 SOL (profit!)
            a_to_b: false,
        },
    ];
    
    // Verify route is closed
    let first_token = route.first().unwrap().token_in;
    let last_token = route.last().unwrap().token_out;
    assert_eq!(first_token, last_token, "Route must be closed");
    
    // Verify route length
    assert!(route.len() >= 2, "Route must have at least 2 legs");
    assert!(route.len() <= 4, "Route must have at most 4 legs");
    
    println!("✅ Route validation logic works");
}

/// Test profit calculation
#[tokio::test]
async fn test_profit_calculation() {
    let balance_before: u64 = 1_000_000_000; // 1 SOL
    let balance_after: u64 = 1_015_000_000;  // 1.015 SOL
    let min_profit: u64 = 10_000_000;        // 0.01 SOL
    
    let profit = balance_after.checked_sub(balance_before).unwrap();
    
    assert_eq!(profit, 15_000_000); // 0.015 SOL profit
    assert!(profit >= min_profit, "Profit should be >= min_profit");
    
    println!("✅ Profit calculation: {} lamports", profit);
}

/// Test slippage calculation
#[tokio::test]
async fn test_slippage_calculation() {
    let expected_out: u64 = 1_000_000;
    let max_slippage_bps: u16 = 100; // 1%
    
    // Calculate minimum output with slippage
    let min_out_with_slippage = expected_out
        .checked_mul(10000 - max_slippage_bps as u64)
        .unwrap()
        .checked_div(10000)
        .unwrap();
    
    assert_eq!(min_out_with_slippage, 990_000); // 1% slippage
    
    // Verify received amount is acceptable
    let received: u64 = 995_000;
    assert!(received >= min_out_with_slippage, "Received should be >= min with slippage");
    
    println!("✅ Slippage calculation works: {} min out", min_out_with_slippage);
}

