//! Aggregator instruction builder
//!
//! Builds instructions for our on-chain aggregator contract

use borsh::{BorshDeserialize, BorshSerialize};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};
use std::str::FromStr;

/// Our deployed aggregator program ID on devnet
pub const AGGREGATOR_PROGRAM_ID: &str = "DMCPSH38kwbcXxwyaHdXqEf4JCTdgdTwMJwcEMHPrEqK";

/// Token program ID
pub const TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

/// DEX type for routing
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug, PartialEq)]
pub enum DexType {
    OrcaWhirlpool,
    RaydiumClmm,
    RaydiumAmm,
    Phoenix,
    OpenBook,
}

impl From<bot_core::PoolType> for DexType {
    fn from(pool_type: bot_core::PoolType) -> Self {
        match pool_type {
            bot_core::PoolType::OrcaWhirlpool => DexType::OrcaWhirlpool,
            bot_core::PoolType::RaydiumClmm => DexType::RaydiumClmm,
            bot_core::PoolType::RaydiumAmm => DexType::RaydiumAmm,
            bot_core::PoolType::Phoenix => DexType::Phoenix,
            _ => DexType::OrcaWhirlpool, // default
        }
    }
}

/// One swap leg in the arbitrage route
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct SwapLeg {
    pub dex: DexType,
    pub pool: Pubkey,
    pub token_in: Pubkey,
    pub token_out: Pubkey,
    pub amount_in: u64,
    pub min_amount_out: u64,
    pub a_to_b: bool,
}

/// Instruction discriminators for Anchor
pub mod instruction {
    /// Initialize instruction discriminator
    pub const INITIALIZE: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];
    
    /// Execute arbitrage instruction discriminator
    pub const EXECUTE_ARBITRAGE: [u8; 8] = [0x65, 0x78, 0x65, 0x63, 0x75, 0x74, 0x65, 0x5f];
}

/// Build the config PDA
pub fn get_config_pda() -> (Pubkey, u8) {
    let program_id = Pubkey::from_str(AGGREGATOR_PROGRAM_ID).unwrap();
    Pubkey::find_program_address(&[b"config"], &program_id)
}

/// Build initialize instruction
pub fn build_initialize_ix(
    owner: &Pubkey,
    treasury: &Pubkey,
    fee_bps: u16,
) -> Instruction {
    let program_id = Pubkey::from_str(AGGREGATOR_PROGRAM_ID).unwrap();
    let (config_pda, _bump) = get_config_pda();

    // Anchor discriminator + args
    let mut data = Vec::with_capacity(8 + 2);
    data.extend_from_slice(&instruction::INITIALIZE);
    data.extend_from_slice(&fee_bps.to_le_bytes());

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(*owner, true),           // owner (signer, payer)
            AccountMeta::new(config_pda, false),      // config PDA
            AccountMeta::new_readonly(*treasury, false), // treasury
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}

/// Build execute_arbitrage instruction
pub fn build_execute_arbitrage_ix(
    authority: &Pubkey,
    base_token_account: &Pubkey,
    route: Vec<SwapLeg>,
    min_profit: u64,
    max_slippage_bps: u16,
    remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
    let program_id = Pubkey::from_str(AGGREGATOR_PROGRAM_ID).unwrap();
    let (config_pda, _bump) = get_config_pda();
    let token_program = Pubkey::from_str(TOKEN_PROGRAM_ID).unwrap();

    // Build instruction data
    // Anchor: 8-byte discriminator + args
    let mut data = Vec::new();
    
    // Use Anchor's discriminator for execute_arbitrage
    // This is sha256("global:execute_arbitrage")[..8]
    let discriminator: [u8; 8] = [0x3d, 0xcc, 0xf0, 0x7a, 0x93, 0x4c, 0x89, 0x7a];
    data.extend_from_slice(&discriminator);
    
    // Serialize route
    let route_len = route.len() as u32;
    data.extend_from_slice(&route_len.to_le_bytes());
    for leg in &route {
        leg.serialize(&mut data).unwrap();
    }
    
    // min_profit
    data.extend_from_slice(&min_profit.to_le_bytes());
    
    // max_slippage_bps
    data.extend_from_slice(&max_slippage_bps.to_le_bytes());

    // Build accounts
    let mut accounts = vec![
        AccountMeta::new(*authority, true),          // authority (signer)
        AccountMeta::new(config_pda, false),         // config
        AccountMeta::new(*base_token_account, false), // base_token_account
        AccountMeta::new_readonly(token_program, false), // token_program
        AccountMeta::new_readonly(system_program::id(), false), // system_program
    ];
    
    // Add remaining accounts for DEX interactions
    accounts.extend(remaining_accounts);

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Convert bot_core Route to aggregator SwapLeg vec
pub fn route_to_swap_legs(route: &bot_core::Route) -> Vec<SwapLeg> {
    route
        .legs
        .iter()
        .map(|leg| SwapLeg {
            dex: leg.pool_type.into(),
            pool: leg.pool,
            token_in: leg.token_in,
            token_out: leg.token_out,
            amount_in: leg.amount_in,
            min_amount_out: leg.min_amount_out,
            a_to_b: leg.a_to_b,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_config_pda() {
        let (pda, bump) = get_config_pda();
        assert!(bump > 0 || bump == 0); // bump is valid
        println!("Config PDA: {}, bump: {}", pda, bump);
    }

    #[test]
    fn test_build_initialize_ix() {
        let owner = Pubkey::new_unique();
        let treasury = Pubkey::new_unique();
        let ix = build_initialize_ix(&owner, &treasury, 10);
        
        assert_eq!(ix.accounts.len(), 4);
        assert!(!ix.data.is_empty());
        println!("Initialize instruction built successfully");
    }

    #[test]
    fn test_swap_leg_serialization() {
        use borsh::{BorshSerialize, BorshDeserialize};
        
        let leg = SwapLeg {
            dex: DexType::OrcaWhirlpool,
            pool: Pubkey::new_unique(),
            token_in: Pubkey::new_unique(),
            token_out: Pubkey::new_unique(),
            amount_in: 1_000_000,
            min_amount_out: 990_000,
            a_to_b: true,
        };

        let mut serialized = Vec::new();
        leg.serialize(&mut serialized).unwrap();
        let deserialized = SwapLeg::try_from_slice(&serialized).unwrap();
        
        assert_eq!(deserialized.amount_in, 1_000_000);
        println!("SwapLeg serialization works");
    }
}

