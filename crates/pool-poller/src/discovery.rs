//! Pool Discovery - автоматический поиск пулов через API
//!
//! Поддерживает:
//! - Jupiter API (все DEX)
//! - Orca Whirlpools API
//! - Raydium API

use anyhow::Result;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use tracing::{debug, info, warn};

use bot_core::PoolType;
use crate::registry::{PoolInfo, PoolRegistry};

// ============================================
// API Endpoints
// ============================================

const JUPITER_TOKENS_API: &str = "https://token.jup.ag/all";
const ORCA_WHIRLPOOLS_API: &str = "https://api.mainnet.orca.so/v1/whirlpool/list";
const RAYDIUM_POOLS_API: &str = "https://api-v3.raydium.io/pools/info/mint";

// ============================================
// Response Types
// ============================================

#[derive(Debug, Deserialize)]
pub struct OrcaWhirlpoolsResponse {
    pub whirlpools: Vec<OrcaWhirlpool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrcaWhirlpool {
    pub address: String,
    pub token_a: OrcaToken,
    pub token_b: OrcaToken,
    pub tick_spacing: u16,
    pub price: Option<f64>,
    pub tvl: Option<f64>,
    #[serde(default)]
    pub volume: OrcaVolume,
}

#[derive(Debug, Deserialize, Default)]
pub struct OrcaVolume {
    pub day: Option<f64>,
    pub week: Option<f64>,
    pub month: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct OrcaToken {
    pub mint: String,
    pub symbol: String,
    pub decimals: u8,
}

#[derive(Debug, Deserialize)]
pub struct RaydiumPoolsResponse {
    pub success: bool,
    pub data: RaydiumData,
}

#[derive(Debug, Deserialize)]
pub struct RaydiumData {
    #[serde(default)]
    pub count: u32,
    #[serde(default)]
    pub data: Vec<RaydiumPool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RaydiumPool {
    pub id: String,
    pub mint_a: RaydiumMint,
    pub mint_b: RaydiumMint,
    #[serde(default)]
    pub price: f64,
    #[serde(default)]
    pub tvl: f64,
    #[serde(default)]
    pub fee_rate: f64,
    #[serde(rename = "type")]
    pub pool_type: String,
}

#[derive(Debug, Deserialize)]
pub struct RaydiumMint {
    pub address: String,
    #[serde(default)]
    pub symbol: String,
}

#[derive(Debug, Deserialize)]
pub struct JupiterToken {
    pub address: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    #[serde(default)]
    pub daily_volume: Option<f64>,
}

// ============================================
// Pool Discovery
// ============================================

pub struct PoolDiscovery {
    http_client: reqwest::Client,
    /// Minimum TVL to include pool (USD)
    pub min_tvl: f64,
    /// Minimum 24h volume (USD)
    pub min_volume: f64,
}

impl PoolDiscovery {
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            min_tvl: 10_000.0,      // $10k minimum TVL
            min_volume: 1_000.0,    // $1k minimum daily volume
        }
    }

    /// Set minimum TVL filter
    pub fn with_min_tvl(mut self, tvl: f64) -> Self {
        self.min_tvl = tvl;
        self
    }

    /// Set minimum volume filter
    pub fn with_min_volume(mut self, volume: f64) -> Self {
        self.min_volume = volume;
        self
    }

    /// Discover all pools from all sources
    pub async fn discover_all(&self) -> Result<Vec<PoolInfo>> {
        let mut all_pools = Vec::new();

        // Orca Whirlpools
        match self.discover_orca().await {
            Ok(pools) => {
                info!("✅ Discovered {} Orca Whirlpools", pools.len());
                all_pools.extend(pools);
            }
            Err(e) => warn!("⚠️ Failed to fetch Orca pools: {}", e),
        }

        // Raydium CLMM
        match self.discover_raydium().await {
            Ok(pools) => {
                info!("✅ Discovered {} Raydium pools", pools.len());
                all_pools.extend(pools);
            }
            Err(e) => warn!("⚠️ Failed to fetch Raydium pools: {}", e),
        }

        info!("📊 Total pools discovered: {}", all_pools.len());
        Ok(all_pools)
    }

    /// Discover Orca Whirlpools
    pub async fn discover_orca(&self) -> Result<Vec<PoolInfo>> {
        info!("🔍 Fetching Orca Whirlpools...");

        let response: OrcaWhirlpoolsResponse = self
            .http_client
            .get(ORCA_WHIRLPOOLS_API)
            .send()
            .await?
            .json()
            .await?;

        let pools: Vec<PoolInfo> = response
            .whirlpools
            .into_iter()
            .filter(|p| {
                let tvl = p.tvl.unwrap_or(0.0);
                let volume = p.volume.day.unwrap_or(0.0);
                tvl >= self.min_tvl && volume >= self.min_volume
            })
            .filter_map(|p| {
                let address = Pubkey::from_str(&p.address).ok()?;
                let token_a = Pubkey::from_str(&p.token_a.mint).ok()?;
                let token_b = Pubkey::from_str(&p.token_b.mint).ok()?;

                Some(PoolInfo {
                    address,
                    pool_type: PoolType::OrcaWhirlpool,
                    token_a,
                    token_b,
                    name: format!("Orca {}/{}", p.token_a.symbol, p.token_b.symbol),
                    enabled: true,
                })
            })
            .collect();

        Ok(pools)
    }

    /// Discover Raydium CLMM pools
    /// Uses mint-based API to fetch pools for major tokens
    pub async fn discover_raydium(&self) -> Result<Vec<PoolInfo>> {
        info!("🔍 Fetching Raydium pools...");

        let mut all_pools = Vec::new();
        
        // Fetch pools for major tokens
        let major_mints = [
            "So11111111111111111111111111111111111111112",  // SOL
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
            "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB", // USDT
        ];

        for mint in major_mints {
            let url = format!(
                "{}?mint1={}&poolType=all&poolSortField=default&sortType=desc&pageSize=100&page=1",
                RAYDIUM_POOLS_API, mint
            );
            
            match self.http_client.get(&url).send().await {
                Ok(resp) => {
                    match resp.json::<RaydiumPoolsResponse>().await {
                        Ok(response) if response.success => {
                            let pools: Vec<PoolInfo> = response
                                .data
                                .data
                                .into_iter()
                                .filter(|p| p.tvl >= self.min_tvl)
                                .filter_map(|p| {
                                    let address = Pubkey::from_str(&p.id).ok()?;
                                    let token_a = Pubkey::from_str(&p.mint_a.address).ok()?;
                                    let token_b = Pubkey::from_str(&p.mint_b.address).ok()?;

                                    let pool_type = if p.pool_type == "Concentrated" {
                                        PoolType::RaydiumClmm
                                    } else {
                                        PoolType::RaydiumAmm
                                    };

                                    let name = format!(
                                        "Raydium {}/{}",
                                        if p.mint_a.symbol.is_empty() { &p.mint_a.address[..4] } else { &p.mint_a.symbol },
                                        if p.mint_b.symbol.is_empty() { &p.mint_b.address[..4] } else { &p.mint_b.symbol }
                                    );

                                    Some(PoolInfo {
                                        address,
                                        pool_type,
                                        token_a,
                                        token_b,
                                        name,
                                        enabled: true,
                                    })
                                })
                                .collect();
                            
                            debug!("Found {} Raydium pools for mint {}", pools.len(), &mint[..8]);
                            all_pools.extend(pools);
                        }
                        Ok(_) => warn!("Raydium API returned success=false for mint {}", &mint[..8]),
                        Err(e) => warn!("Failed to parse Raydium response: {}", e),
                    }
                }
                Err(e) => warn!("Failed to fetch Raydium pools for mint {}: {}", &mint[..8], e),
            }
        }

        // Deduplicate by pool address
        let mut seen = std::collections::HashSet::new();
        all_pools.retain(|p| seen.insert(p.address));

        Ok(all_pools)
    }

    /// Get popular tokens from Jupiter
    pub async fn get_popular_tokens(&self, limit: usize) -> Result<Vec<JupiterToken>> {
        info!("🔍 Fetching tokens from Jupiter...");

        let tokens: Vec<JupiterToken> = self
            .http_client
            .get(JUPITER_TOKENS_API)
            .send()
            .await?
            .json()
            .await?;

        // Sort by daily volume and take top N
        let mut sorted: Vec<_> = tokens
            .into_iter()
            .filter(|t| t.daily_volume.unwrap_or(0.0) > 0.0)
            .collect();

        sorted.sort_by(|a, b| {
            b.daily_volume
                .unwrap_or(0.0)
                .partial_cmp(&a.daily_volume.unwrap_or(0.0))
                .unwrap()
        });

        Ok(sorted.into_iter().take(limit).collect())
    }
}

// ============================================
// Registry Integration
// ============================================

impl PoolRegistry {
    /// Load pools from discovery
    pub async fn load_from_discovery(&mut self, min_tvl: f64, min_volume: f64) -> Result<usize> {
        let discovery = PoolDiscovery::new()
            .with_min_tvl(min_tvl)
            .with_min_volume(min_volume);

        let pools = discovery.discover_all().await?;
        let count = pools.len();

        for pool in pools {
            self.add_pool(pool);
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires network
    async fn test_orca_discovery() {
        let discovery = PoolDiscovery::new().with_min_tvl(100_000.0);
        let pools = discovery.discover_orca().await.unwrap();
        println!("Found {} Orca pools with TVL > $100k", pools.len());
        assert!(!pools.is_empty());
    }

    #[tokio::test]
    #[ignore] // Requires network
    async fn test_raydium_discovery() {
        let discovery = PoolDiscovery::new().with_min_tvl(100_000.0);
        let pools = discovery.discover_raydium().await.unwrap();
        println!("Found {} Raydium pools with TVL > $100k", pools.len());
    }
}

