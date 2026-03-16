//! Raydium CLMM deserializer

use crate::traits::DeserializeError;
use bot_core::{ParsedPool, PoolType, TokenInfo};
use solana_sdk::pubkey::Pubkey;

/// Raydium CLMM discriminator
const RAYDIUM_DISCRIMINATOR: [u8; 8] = [247, 237, 227, 245, 215, 195, 222, 70];

/// Raydium CLMM pool state - parsed manually
pub struct RaydiumPoolState {
    pub discriminator: [u8; 8],
    pub bump: u8,
    pub amm_config: Pubkey,
    pub owner: Pubkey,
    pub token_mint_0: Pubkey,
    pub token_mint_1: Pubkey,
    pub token_vault_0: Pubkey,
    pub token_vault_1: Pubkey,
    pub observation_key: Pubkey,
    pub mint_decimals_0: u8,
    pub mint_decimals_1: u8,
    pub tick_spacing: u16,
    pub liquidity: u128,
    pub sqrt_price_x64: u128,
    pub tick_current: i32,
}

pub struct RaydiumDeserializer;

impl RaydiumDeserializer {
    pub fn new() -> Self {
        Self
    }

    fn parse_state(data: &[u8]) -> Result<RaydiumPoolState, DeserializeError> {
        if data.len() < 300 {
            return Err(DeserializeError::InvalidLength {
                expected: 300,
                got: data.len(),
            });
        }

        let mut offset = 0;

        let discriminator: [u8; 8] = data[offset..offset + 8].try_into().unwrap();
        offset += 8;

        let bump = data[offset];
        offset += 1;

        let amm_config = Pubkey::try_from(&data[offset..offset + 32])
            .map_err(|_| DeserializeError::InvalidFormat)?;
        offset += 32;

        let owner = Pubkey::try_from(&data[offset..offset + 32])
            .map_err(|_| DeserializeError::InvalidFormat)?;
        offset += 32;

        let token_mint_0 = Pubkey::try_from(&data[offset..offset + 32])
            .map_err(|_| DeserializeError::InvalidFormat)?;
        offset += 32;

        let token_mint_1 = Pubkey::try_from(&data[offset..offset + 32])
            .map_err(|_| DeserializeError::InvalidFormat)?;
        offset += 32;

        let token_vault_0 = Pubkey::try_from(&data[offset..offset + 32])
            .map_err(|_| DeserializeError::InvalidFormat)?;
        offset += 32;

        let token_vault_1 = Pubkey::try_from(&data[offset..offset + 32])
            .map_err(|_| DeserializeError::InvalidFormat)?;
        offset += 32;

        let observation_key = Pubkey::try_from(&data[offset..offset + 32])
            .map_err(|_| DeserializeError::InvalidFormat)?;
        offset += 32;

        let mint_decimals_0 = data[offset];
        offset += 1;

        let mint_decimals_1 = data[offset];
        offset += 1;

        let tick_spacing = u16::from_le_bytes([data[offset], data[offset + 1]]);
        offset += 2;

        let liquidity = u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap());
        offset += 16;

        let sqrt_price_x64 = u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap());
        offset += 16;

        let tick_current = i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());

        Ok(RaydiumPoolState {
            discriminator,
            bump,
            amm_config,
            owner,
            token_mint_0,
            token_mint_1,
            token_vault_0,
            token_vault_1,
            observation_key,
            mint_decimals_0,
            mint_decimals_1,
            tick_spacing,
            liquidity,
            sqrt_price_x64,
            tick_current,
        })
    }
}

impl crate::traits::PoolDeserializerTrait for RaydiumDeserializer {
    fn deserialize(&self, data: &[u8]) -> Result<ParsedPool, DeserializeError> {
        let state = Self::parse_state(data)?;

        if state.discriminator != RAYDIUM_DISCRIMINATOR {
            return Err(DeserializeError::UnknownDiscriminator);
        }

        Ok(ParsedPool {
            address: Pubkey::default(),
            pool_type: PoolType::RaydiumClmm,
            token_a: TokenInfo {
                mint: state.token_mint_0,
                vault: state.token_vault_0,
                decimals: state.mint_decimals_0,
            },
            token_b: TokenInfo {
                mint: state.token_mint_1,
                vault: state.token_vault_1,
                decimals: state.mint_decimals_1,
            },
            sqrt_price: state.sqrt_price_x64,
            liquidity: state.liquidity,
            tick_current: state.tick_current,
            fee_rate_bps: 25, // Default, actual fee from amm_config
            last_updated_slot: 0,
        })
    }

    fn pool_type(&self) -> PoolType {
        PoolType::RaydiumClmm
    }

    fn min_size(&self) -> usize {
        300 // Approximate minimum size
    }

    fn matches(&self, data: &[u8]) -> bool {
        data.len() >= 8 && data[..8] == RAYDIUM_DISCRIMINATOR
    }
}

