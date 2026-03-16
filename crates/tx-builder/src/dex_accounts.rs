//! DEX Account builders for CPI
//!
//! Builds the remaining_accounts needed for CPI calls to DEX programs

use solana_sdk::{
    instruction::AccountMeta,
    pubkey::Pubkey,
};
use std::str::FromStr;

// ============================================
// Program IDs
// ============================================

/// Orca Whirlpool Program ID
pub const ORCA_WHIRLPOOL_PROGRAM: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";

/// Raydium CLMM Program ID  
pub const RAYDIUM_CLMM_PROGRAM: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";

/// SPL Token Program ID
pub const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

// ============================================
// Orca Whirlpool Accounts
// ============================================

/// Orca Whirlpool pool configuration
#[derive(Debug, Clone)]
pub struct OrcaWhirlpoolAccounts {
    pub pool: Pubkey,
    pub whirlpools_config: Pubkey,
    pub token_mint_a: Pubkey,
    pub token_mint_b: Pubkey,
    pub token_vault_a: Pubkey,
    pub token_vault_b: Pubkey,
    pub tick_array_0: Pubkey,
    pub tick_array_1: Pubkey,
    pub tick_array_2: Pubkey,
    pub oracle: Pubkey,
}

impl OrcaWhirlpoolAccounts {
    /// Mainnet SOL/USDC Whirlpool (64 tick spacing)
    pub fn mainnet_sol_usdc() -> Self {
        Self {
            pool: Pubkey::from_str("HJPjoWUrhoZzkNfRpHuieeFk9WcZWjwy6PBjZ81ngndJ").unwrap(),
            whirlpools_config: Pubkey::from_str("2LecshUwdy9xi7meFgHtFJQNSKk4KdTrcpvaB56dP2NQ").unwrap(),
            token_mint_a: Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap(),
            token_mint_b: Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
            token_vault_a: Pubkey::from_str("3YQm7ujtXWJU2e9jhp2QGHpnn1ShXn12QjvzMvDgabpX").unwrap(),
            token_vault_b: Pubkey::from_str("2JTw1fE2wz1SymWUQ7UqpVtrTuKjcd6mWwYwUJUCh2rq").unwrap(),
            // Tick arrays need to be derived based on current tick
            // For tick -20228 with spacing 64:
            // tick_array_start = floor(-20228 / (64 * 88)) * 64 * 88 = -22528
            tick_array_0: Pubkey::from_str("6vtnrYbBLEP6E6PEYJhDoqNExgNzdJbaLRut4HuWKX7S").unwrap(),
            tick_array_1: Pubkey::from_str("3nFMXvTnFqsg9MVwLYdP8n7xHtDd2NBZYs8xEBdWjBxL").unwrap(),
            tick_array_2: Pubkey::from_str("5mFGJ4p6foZDkKRFWoLrZ2E8bVkHYmNR5k2rT3hNZxCu").unwrap(),
            oracle: Pubkey::from_str("4GkRbcYg1VKsZropgai4dMf2Nj2PkXNLf43knFpavrSi").unwrap(),
        }
    }

    /// Build remaining accounts for Orca swap CPI
    /// 
    /// Order:
    /// 0. whirlpool (pool)
    /// 1. token_authority (signer) - provided separately
    /// 2. token_owner_account_a - user's token account
    /// 3. token_vault_a
    /// 4. token_owner_account_b - user's token account
    /// 5. token_vault_b
    /// 6. tick_array_0
    /// 7. tick_array_1
    /// 8. tick_array_2
    /// 9. oracle
    /// 10. token_program
    /// 11. whirlpool_program
    pub fn build_remaining_accounts(
        &self,
        user_token_account_a: &Pubkey,
        user_token_account_b: &Pubkey,
    ) -> Vec<AccountMeta> {
        let token_program = Pubkey::from_str(TOKEN_PROGRAM).unwrap();
        let whirlpool_program = Pubkey::from_str(ORCA_WHIRLPOOL_PROGRAM).unwrap();

        vec![
            AccountMeta::new(self.pool, false),                    // 0: whirlpool
            // 1: token_authority is the signer, handled by aggregator
            AccountMeta::new(*user_token_account_a, false),        // 2: token_owner_account_a
            AccountMeta::new(self.token_vault_a, false),           // 3: token_vault_a
            AccountMeta::new(*user_token_account_b, false),        // 4: token_owner_account_b
            AccountMeta::new(self.token_vault_b, false),           // 5: token_vault_b
            AccountMeta::new(self.tick_array_0, false),            // 6: tick_array_0
            AccountMeta::new(self.tick_array_1, false),            // 7: tick_array_1
            AccountMeta::new(self.tick_array_2, false),            // 8: tick_array_2
            AccountMeta::new_readonly(self.oracle, false),         // 9: oracle
            AccountMeta::new_readonly(token_program, false),       // 10: token_program
            AccountMeta::new_readonly(whirlpool_program, false),   // 11: whirlpool_program
        ]
    }
}

// ============================================
// Raydium CLMM Accounts
// ============================================

/// Raydium CLMM pool configuration
#[derive(Debug, Clone)]
pub struct RaydiumClmmAccounts {
    pub pool: Pubkey,
    pub amm_config: Pubkey,
    pub token_mint_0: Pubkey,
    pub token_mint_1: Pubkey,
    pub token_vault_0: Pubkey,
    pub token_vault_1: Pubkey,
    pub observation_state: Pubkey,
    pub tick_array_bitmap: Pubkey,
}

impl RaydiumClmmAccounts {
    /// Mainnet SOL/USDC Raydium CLMM
    pub fn mainnet_sol_usdc() -> Self {
        // These are example addresses - need to be verified
        Self {
            pool: Pubkey::from_str("2QdhepnKRTLjjSqPL1PtKNwqrUkoLee5Gqs8bvZhRdMv").unwrap(),
            amm_config: Pubkey::from_str("D4FPEruKEHrG5TenZ2Yh3MJqNLVnSH6M9BYSLfkyH8Cz").unwrap(),
            token_mint_0: Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap(),
            token_mint_1: Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
            token_vault_0: Pubkey::from_str("BPPn6pHHHd2qL6SJrBK4TLFKgLJ2VVBhCKmeAqR1xMJD").unwrap(),
            token_vault_1: Pubkey::from_str("FbXvZwdNkNe3VmEn3JjDJFTmJDDX8rD8LbfSvPhFj9VB").unwrap(),
            observation_state: Pubkey::from_str("11111111111111111111111111111111").unwrap(), // placeholder
            tick_array_bitmap: Pubkey::from_str("11111111111111111111111111111111").unwrap(), // placeholder
        }
    }

    /// Build remaining accounts for Raydium CLMM swap CPI
    pub fn build_remaining_accounts(
        &self,
        user_token_account_0: &Pubkey,
        user_token_account_1: &Pubkey,
    ) -> Vec<AccountMeta> {
        let token_program = Pubkey::from_str(TOKEN_PROGRAM).unwrap();
        let raydium_program = Pubkey::from_str(RAYDIUM_CLMM_PROGRAM).unwrap();

        vec![
            AccountMeta::new(self.pool, false),                    // pool_state
            AccountMeta::new_readonly(self.amm_config, false),     // amm_config
            AccountMeta::new(*user_token_account_0, false),        // input_token_account
            AccountMeta::new(*user_token_account_1, false),        // output_token_account
            AccountMeta::new(self.token_vault_0, false),           // input_vault
            AccountMeta::new(self.token_vault_1, false),           // output_vault
            AccountMeta::new(self.observation_state, false),       // observation_state
            AccountMeta::new_readonly(token_program, false),       // token_program
            AccountMeta::new(self.tick_array_bitmap, false),       // tick_array_bitmap
            AccountMeta::new_readonly(raydium_program, false),     // raydium_program
        ]
    }
}

// ============================================
// Helper functions
// ============================================

/// Derive Orca tick array address from whirlpool and start tick
pub fn derive_orca_tick_array(
    whirlpool: &Pubkey,
    start_tick_index: i32,
) -> Pubkey {
    let whirlpool_program = Pubkey::from_str(ORCA_WHIRLPOOL_PROGRAM).unwrap();
    let seeds = &[
        b"tick_array",
        whirlpool.as_ref(),
        &start_tick_index.to_le_bytes(),
    ];
    let (pda, _bump) = Pubkey::find_program_address(seeds, &whirlpool_program);
    pda
}

/// Calculate tick array start index
pub fn get_tick_array_start_index(tick: i32, tick_spacing: i32) -> i32 {
    let ticks_in_array = tick_spacing * 88; // Orca uses 88 ticks per array
    let mut start = tick / ticks_in_array;
    if tick < 0 && tick % ticks_in_array != 0 {
        start -= 1;
    }
    start * ticks_in_array
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orca_accounts() {
        let orca = OrcaWhirlpoolAccounts::mainnet_sol_usdc();
        assert_eq!(
            orca.pool.to_string(),
            "HJPjoWUrhoZzkNfRpHuieeFk9WcZWjwy6PBjZ81ngndJ"
        );
        println!("Orca pool: {}", orca.pool);
    }

    #[test]
    fn test_build_remaining_accounts() {
        let orca = OrcaWhirlpoolAccounts::mainnet_sol_usdc();
        let user_a = Pubkey::new_unique();
        let user_b = Pubkey::new_unique();
        
        let accounts = orca.build_remaining_accounts(&user_a, &user_b);
        assert_eq!(accounts.len(), 11);
        println!("Built {} remaining accounts", accounts.len());
    }

    #[test]
    fn test_tick_array_calculation() {
        // For tick -20228 with spacing 64
        // ticks_in_array = 64 * 88 = 5632
        // start = floor(-20228 / 5632) = -4 → -4 * 5632 = -22528
        let start = get_tick_array_start_index(-20228, 64);
        assert_eq!(start, -22528);
        println!("Tick array start: {}", start);
    }
}


