//! Raydium CLMM CPI implementation
//!
//! Implements swap via CPI to Raydium Concentrated Liquidity program.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

/// Raydium CLMM program ID (mainnet)
pub const RAYDIUM_CLMM_PROGRAM_ID: Pubkey = pubkey!("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK");

/// Raydium CLMM swap instruction discriminator
/// Anchor discriminator for "swap" instruction
pub const SWAP_DISCRIMINATOR: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];

/// Raydium CLMM swap_v2 discriminator (newer version)
pub const SWAP_V2_DISCRIMINATOR: [u8; 8] = [43, 4, 237, 11, 26, 201, 30, 98];

/// Raydium swap instruction data
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct SwapSingleInstructionData {
    /// Amount of tokens to swap
    pub amount: u64,
    /// Minimum/maximum amount depending on swap type
    pub other_amount_threshold: u64,
    /// Sqrt price limit (0 for no limit)
    pub sqrt_price_limit_x64: u128,
    /// True if amount is input amount
    pub is_base_input: bool,
}

/// Execute swap on Raydium CLMM via CPI
///
/// # Accounts required in remaining_accounts (in order):
/// 0. `[]` Token program
/// 1. `[signer]` Payer
/// 2. `[]` AMM config
/// 3. `[writable]` Pool state
/// 4. `[writable]` Input token account (user)
/// 5. `[writable]` Output token account (user)
/// 6. `[writable]` Input vault
/// 7. `[writable]` Output vault
/// 8. `[]` Observation state
/// 9. `[writable]` Tick array bitmap extension (optional, can be program_id if not needed)
/// 10. `[writable]` Tick array account 0
/// 11. `[writable]` Tick array account 1
/// 12. `[writable]` Tick array account 2
pub fn swap_cpi<'info>(
    remaining_accounts: &[AccountInfo<'info>],
    start_index: usize,
    amount: u64,
    min_amount_out: u64,
    a_to_b: bool,
) -> Result<usize> {
    const ACCOUNTS_NEEDED: usize = 13;
    
    require!(
        remaining_accounts.len() >= start_index + ACCOUNTS_NEEDED,
        crate::error::AggregatorError::NotEnoughAccounts
    );

    let accounts = &remaining_accounts[start_index..start_index + ACCOUNTS_NEEDED];

    // Build instruction data
    let ix_data = SwapSingleInstructionData {
        amount,
        other_amount_threshold: min_amount_out,
        sqrt_price_limit_x64: if a_to_b {
            MIN_SQRT_PRICE_X64 + 1
        } else {
            MAX_SQRT_PRICE_X64 - 1
        },
        is_base_input: true,
    };

    let mut data = Vec::with_capacity(8 + 32);
    data.extend_from_slice(&SWAP_V2_DISCRIMINATOR);
    ix_data.serialize(&mut data)?;

    // Build account metas
    let account_metas = vec![
        // Payer/Authority (signer)
        AccountMeta::new(accounts[1].key(), true),
        // AMM Config
        AccountMeta::new_readonly(accounts[2].key(), false),
        // Pool State
        AccountMeta::new(accounts[3].key(), false),
        // Input token account
        AccountMeta::new(accounts[4].key(), false),
        // Output token account
        AccountMeta::new(accounts[5].key(), false),
        // Input vault
        AccountMeta::new(accounts[6].key(), false),
        // Output vault
        AccountMeta::new(accounts[7].key(), false),
        // Observation state
        AccountMeta::new(accounts[8].key(), false),
        // Token program
        AccountMeta::new_readonly(accounts[0].key(), false),
        // Tick array bitmap extension (can be empty)
        AccountMeta::new(accounts[9].key(), false),
        // Tick arrays
        AccountMeta::new(accounts[10].key(), false),
        AccountMeta::new(accounts[11].key(), false),
        AccountMeta::new(accounts[12].key(), false),
    ];

    let ix = Instruction {
        program_id: RAYDIUM_CLMM_PROGRAM_ID,
        accounts: account_metas,
        data,
    };

    // Convert to AccountInfo slice for invoke
    let account_infos: Vec<AccountInfo> = accounts.to_vec();

    msg!("  → CPI to Raydium CLMM: amount={}, min_out={}, a_to_b={}", 
         amount, min_amount_out, a_to_b);

    invoke(&ix, &account_infos)?;

    msg!("  ✅ Raydium CLMM swap completed");

    Ok(start_index + ACCOUNTS_NEEDED)
}

/// Minimum sqrt price limit
pub const MIN_SQRT_PRICE_X64: u128 = 4295048016;

/// Maximum sqrt price limit
pub const MAX_SQRT_PRICE_X64: u128 = 79226673515401279992447579055;

