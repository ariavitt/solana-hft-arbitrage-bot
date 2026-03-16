//! Orca Whirlpool deserializer

use crate::traits::DeserializeError;
use bot_core::{ParsedPool, PoolType, TokenInfo};
use solana_sdk::pubkey::Pubkey;

/// Orca Whirlpool discriminator
const WHIRLPOOL_DISCRIMINATOR: [u8; 8] = [63, 149, 209, 12, 225, 128, 99, 9];

/// Orca Whirlpool state - parsed manually from bytes
pub struct WhirlpoolState {
    pub discriminator: [u8; 8],
    pub whirlpools_config: Pubkey,
    pub whirlpool_bump: u8,
    pub tick_spacing: u16,
    pub tick_spacing_seed: [u8; 2],
    pub fee_rate: u16,
    pub protocol_fee_rate: u16,
    pub liquidity: u128,
    pub sqrt_price: u128,
    pub tick_current_index: i32,
    pub protocol_fee_owed_a: u64,
    pub protocol_fee_owed_b: u64,
    pub token_mint_a: Pubkey,
    pub token_vault_a: Pubkey,
    pub fee_growth_global_a: u128,
    pub token_mint_b: Pubkey,
    pub token_vault_b: Pubkey,
    pub fee_growth_global_b: u128,
}

pub struct OrcaDeserializer;

impl OrcaDeserializer {
    pub fn new() -> Self {
        Self
    }

    fn parse_state(data: &[u8]) -> Result<WhirlpoolState, DeserializeError> {
        if data.len() < 300 {
            return Err(DeserializeError::InvalidLength {
                expected: 300,
                got: data.len(),
            });
        }

        let mut offset = 0;

        let discriminator: [u8; 8] = data[offset..offset + 8].try_into().unwrap();
        offset += 8;

        let whirlpools_config = Pubkey::try_from(&data[offset..offset + 32])
            .map_err(|_| DeserializeError::InvalidFormat)?;
        offset += 32;

        let whirlpool_bump = data[offset];
        offset += 1;

        let tick_spacing = u16::from_le_bytes([data[offset], data[offset + 1]]);
        offset += 2;

        let tick_spacing_seed: [u8; 2] = [data[offset], data[offset + 1]];
        offset += 2;

        let fee_rate = u16::from_le_bytes([data[offset], data[offset + 1]]);
        offset += 2;

        let protocol_fee_rate = u16::from_le_bytes([data[offset], data[offset + 1]]);
        offset += 2;

        let liquidity = u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap());
        offset += 16;

        let sqrt_price = u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap());
        offset += 16;

        let tick_current_index = i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
        offset += 4;

        let protocol_fee_owed_a = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let protocol_fee_owed_b = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let token_mint_a = Pubkey::try_from(&data[offset..offset + 32])
            .map_err(|_| DeserializeError::InvalidFormat)?;
        offset += 32;

        let token_vault_a = Pubkey::try_from(&data[offset..offset + 32])
            .map_err(|_| DeserializeError::InvalidFormat)?;
        offset += 32;

        let fee_growth_global_a = u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap());
        offset += 16;

        let token_mint_b = Pubkey::try_from(&data[offset..offset + 32])
            .map_err(|_| DeserializeError::InvalidFormat)?;
        offset += 32;

        let token_vault_b = Pubkey::try_from(&data[offset..offset + 32])
            .map_err(|_| DeserializeError::InvalidFormat)?;
        offset += 32;

        let fee_growth_global_b = u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap());

        Ok(WhirlpoolState {
            discriminator,
            whirlpools_config,
            whirlpool_bump,
            tick_spacing,
            tick_spacing_seed,
            fee_rate,
            protocol_fee_rate,
            liquidity,
            sqrt_price,
            tick_current_index,
            protocol_fee_owed_a,
            protocol_fee_owed_b,
            token_mint_a,
            token_vault_a,
            fee_growth_global_a,
            token_mint_b,
            token_vault_b,
            fee_growth_global_b,
        })
    }
}

impl crate::traits::PoolDeserializerTrait for OrcaDeserializer {
    fn deserialize(&self, data: &[u8]) -> Result<ParsedPool, DeserializeError> {
        let state = Self::parse_state(data)?;

        if state.discriminator != WHIRLPOOL_DISCRIMINATOR {
            return Err(DeserializeError::UnknownDiscriminator);
        }

        // Orca fee_rate is in hundredths of a basis point (1e-6)
        // Convert to bps: fee_rate / 100
        let fee_rate_bps = state.fee_rate / 100;
        
        Ok(ParsedPool {
            address: Pubkey::default(), // Set by caller
            pool_type: PoolType::OrcaWhirlpool,
            token_a: TokenInfo {
                mint: state.token_mint_a,
                vault: state.token_vault_a,
                decimals: 9, // SOL decimals (default)
            },
            token_b: TokenInfo {
                mint: state.token_mint_b,
                vault: state.token_vault_b,
                decimals: 6, // USDC decimals (default)
            },
            sqrt_price: state.sqrt_price,
            liquidity: state.liquidity,
            tick_current: state.tick_current_index,
            fee_rate_bps,
            last_updated_slot: 0,
        })
    }

    fn pool_type(&self) -> PoolType {
        PoolType::OrcaWhirlpool
    }

    fn min_size(&self) -> usize {
        300 // Approximate minimum size
    }

    fn matches(&self, data: &[u8]) -> bool {
        data.len() >= 8 && data[..8] == WHIRLPOOL_DISCRIMINATOR
    }
}

