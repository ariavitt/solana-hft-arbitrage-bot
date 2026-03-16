//! Configuration management

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    pub general: GeneralConfig,
    pub rpc: RpcConfig,
    pub redis: RedisConfig,
    pub strategy: StrategyConfig,
    pub execution: ExecutionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub log_level: String,
    pub network: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcConfig {
    pub primary_url: String,
    pub fallback_urls: Vec<String>,
    pub timeout_ms: u64,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: u32,
    pub cache_ttl_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    pub min_profit_bps: u16,
    pub max_slippage_bps: u16,
    pub max_hops: usize,
    pub min_liquidity_usd: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    pub compute_unit_price: u64,
    pub compute_unit_limit: u32,
    pub use_jito: bool,
    pub jito_tip_lamports: u64,
}

impl BotConfig {
    pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: BotConfig = toml::from_str(&content)?;
        Ok(config)
    }
}

impl Default for BotConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                log_level: "info".to_string(),
                network: "devnet".to_string(),
            },
            rpc: RpcConfig {
                primary_url: "https://api.devnet.solana.com".to_string(),
                fallback_urls: vec![],
                timeout_ms: 10000,
                max_retries: 3,
            },
            redis: RedisConfig {
                url: "redis://localhost:6379".to_string(),
                pool_size: 10,
                cache_ttl_ms: 200,
            },
            strategy: StrategyConfig {
                min_profit_bps: 10,
                max_slippage_bps: 50,
                max_hops: 3,
                min_liquidity_usd: 10000,
            },
            execution: ExecutionConfig {
                compute_unit_price: 1000,
                compute_unit_limit: 400000,
                use_jito: false,
                jito_tip_lamports: 10000,
            },
        }
    }
}

