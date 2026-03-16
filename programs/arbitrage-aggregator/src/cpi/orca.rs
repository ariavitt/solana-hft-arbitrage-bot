//! Orca Whirlpool CPI implementation
//!
//! Implements swap via CPI to Orca Whirlpool program.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

/// Orca Whirlpool program ID (mainnet)
pub const WHIRLPOOL_PROGRAM_ID: Pubkey = pubkey!("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc");

/// Whirlpool swap instruction discriminator
/// This is the first 8 bytes of sha256("global:swap")
pub const SWAP_DISCRIMINATOR: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];

/// Whirlpool swap instruction data
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct SwapInstructionData {
    /// Amount of input tokens to swap
    pub amount: u64,
    /// Minimum amount of output tokens expected
    pub other_amount_threshold: u64,
    /// Square root of the limit price
    pub sqrt_price_limit: u128,
    /// True if swapping exact input, false if exact output
    pub amount_specified_is_input: bool,
    /// True if swapping A to B, false if B to A
    pub a_to_b: bool,
}

/// Execute swap on Orca Whirlpool via CPI
///
/// # Accounts required in remaining_accounts (in order):
/// 0. `[]` Token program
/// 1. `[signer]` Token authority (user)
/// 2. `[writable]` Whirlpool account
/// 3. `[writable]` Token owner account A (user's token A)
/// 4. `[writable]` Token vault A (pool's token A)
/// 5. `[writable]` Token owner account B (user's token B)
/// 6. `[writable]` Token vault B (pool's token B)
/// 7. `[writable]` Tick array 0
/// 8. `[writable]` Tick array 1
/// 9. `[writable]` Tick array 2
/// 10. `[]` Oracle account
pub fn swap_cpi<'info>(
    remaining_accounts: &[AccountInfo<'info>],
    start_index: usize,
    amount: u64,
    min_amount_out: u64,
    a_to_b: bool,
) -> Result<usize> {
    const ACCOUNTS_NEEDED: usize = 11;
    
    require!(
        remaining_accounts.len() >= start_index + ACCOUNTS_NEEDED,
        crate::error::AggregatorError::NotEnoughAccounts
    );

    let accounts = &remaining_accounts[start_index..start_index + ACCOUNTS_NEEDED];

    // Build instruction data
    let ix_data = SwapInstructionData {
        amount,
        other_amount_threshold: min_amount_out,
        sqrt_price_limit: if a_to_b { 
            4295048016u128  // MIN_SQRT_PRICE_X64 
        } else { 
            79226673515401279992447579055u128  // MAX_SQRT_PRICE_X64
        },
        amount_specified_is_input: true,
        a_to_b,
    };

    let mut data = Vec::with_capacity(8 + 32);
    data.extend_from_slice(&SWAP_DISCRIMINATOR);
    ix_data.serialize(&mut data)?;

    // Build account metas
    // Order: token_program, token_authority, whirlpool, token_owner_a, vault_a, 
    //        token_owner_b, vault_b, tick_array_0, tick_array_1, tick_array_2, oracle
    let account_metas = vec![
        AccountMeta::new_readonly(accounts[0].key(), false),  // token_program
        AccountMeta::new_readonly(accounts[1].key(), true),   // token_authority (signer)
        AccountMeta::new(accounts[2].key(), false),           // whirlpool
        AccountMeta::new(accounts[3].key(), false),           // token_owner_account_a
        AccountMeta::new(accounts[4].key(), false),           // token_vault_a
        AccountMeta::new(accounts[5].key(), false),           // token_owner_account_b
        AccountMeta::new(accounts[6].key(), false),           // token_vault_b
        AccountMeta::new(accounts[7].key(), false),           // tick_array_0
        AccountMeta::new(accounts[8].key(), false),           // tick_array_1
        AccountMeta::new(accounts[9].key(), false),           // tick_array_2
        AccountMeta::new_readonly(accounts[10].key(), false), // oracle
    ];

    let ix = Instruction {
        program_id: WHIRLPOOL_PROGRAM_ID,
        accounts: account_metas,
        data,
    };

    // Convert to AccountInfo slice for invoke
    let account_infos: Vec<AccountInfo> = accounts.to_vec();

    msg!("  → CPI to Orca Whirlpool: amount={}, min_out={}, a_to_b={}", 
         amount, min_amount_out, a_to_b);

    invoke(&ix, &account_infos)?;

    msg!("  ✅ Orca swap completed");

    Ok(start_index + ACCOUNTS_NEEDED)
}

/// Minimum sqrt price for A to B swap
pub const MIN_SQRT_PRICE_X64: u128 = 4295048016;

/// Maximum sqrt price for B to A swap  
pub const MAX_SQRT_PRICE_X64: u128 = 79226673515401279992447579055;

