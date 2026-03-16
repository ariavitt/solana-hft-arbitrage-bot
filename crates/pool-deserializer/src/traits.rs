//! Deserializer trait definition

use bot_core::{ParsedPool, PoolType};
use solana_sdk::pubkey::Pubkey;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DeserializeError {
    #[error("Invalid data length: expected {expected}, got {got}")]
    InvalidLength { expected: usize, got: usize },

    #[error("Invalid data format")]
    InvalidFormat,

    #[error("Unknown discriminator")]
    UnknownDiscriminator,

    #[error("Unsupported pool type: {0:?}")]
    UnsupportedPoolType(PoolType),
}

/// Trait for individual pool type deserializers
pub trait PoolDeserializerTrait: Send + Sync {
    /// Deserialize raw account data into ParsedPool
    fn deserialize(&self, data: &[u8]) -> Result<ParsedPool, DeserializeError>;

    /// Pool type identifier
    fn pool_type(&self) -> PoolType;

    /// Expected minimum data length
    fn min_size(&self) -> usize;

    /// Check if data matches this deserializer
    fn matches(&self, data: &[u8]) -> bool;
}

/// Unified deserializer that handles all pool types
pub struct PoolDeserializer {
    orca: crate::orca::OrcaDeserializer,
    raydium: crate::raydium::RaydiumDeserializer,
}

impl PoolDeserializer {
    pub fn new() -> Self {
        Self {
            orca: crate::orca::OrcaDeserializer::new(),
            raydium: crate::raydium::RaydiumDeserializer::new(),
        }
    }

    /// Deserialize pool data based on pool type
    pub fn deserialize(
        &self,
        pool_type: PoolType,
        address: &Pubkey,
        data: &[u8],
    ) -> Result<ParsedPool, DeserializeError> {
        match pool_type {
            PoolType::OrcaWhirlpool => {
                let mut pool = self.orca.deserialize(data)?;
                pool.address = *address;
                Ok(pool)
            }
            PoolType::RaydiumClmm => {
                let mut pool = self.raydium.deserialize(data)?;
                pool.address = *address;
                Ok(pool)
            }
            _ => Err(DeserializeError::UnsupportedPoolType(pool_type)),
        }
    }

    /// Auto-detect pool type and deserialize
    pub fn auto_deserialize(
        &self,
        address: &Pubkey,
        data: &[u8],
    ) -> Result<ParsedPool, DeserializeError> {
        // Try Orca first
        if self.orca.matches(data) {
            let mut pool = self.orca.deserialize(data)?;
            pool.address = *address;
            return Ok(pool);
        }

        // Try Raydium
        if self.raydium.matches(data) {
            let mut pool = self.raydium.deserialize(data)?;
            pool.address = *address;
            return Ok(pool);
        }

        Err(DeserializeError::UnknownDiscriminator)
    }
}

impl Default for PoolDeserializer {
    fn default() -> Self {
        Self::new()
    }
}

