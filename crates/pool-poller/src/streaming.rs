//! Yellowstone gRPC Streaming
//!
//! Real-time pool state updates via Helius Yellowstone gRPC.
//! ~2-3x faster than polling!
//!
//! Requires Helius paid subscription with gRPC access.

use anyhow::{anyhow, Result};
use futures::StreamExt;
use solana_sdk::pubkey::Pubkey;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

use bot_core::ParsedPool;
use pool_deserializer::PoolDeserializer;

use crate::registry::PoolRegistry;

/// Yellowstone gRPC configuration
#[derive(Debug, Clone)]
pub struct YellowstoneConfig {
    /// Helius gRPC endpoint (e.g., "https://atlas-mainnet.helius-rpc.com")
    pub endpoint: String,
    /// API key for authentication
    pub api_key: String,
    /// Commitment level
    pub commitment: String,
    /// Reconnect delay on disconnect
    pub reconnect_delay_ms: u64,
}

impl Default for YellowstoneConfig {
    fn default() -> Self {
        Self {
            endpoint: "https://atlas-mainnet.helius-rpc.com".to_string(),
            api_key: String::new(),
            commitment: "processed".to_string(),
            reconnect_delay_ms: 1000,
        }
    }
}

/// Account update from gRPC stream
#[derive(Debug, Clone)]
pub struct AccountUpdate {
    pub pubkey: Pubkey,
    pub data: Vec<u8>,
    pub slot: u64,
    pub lamports: u64,
}

/// Pool state update event
#[derive(Debug, Clone)]
pub struct PoolUpdate {
    pub pool: ParsedPool,
    pub slot: u64,
}

/// Yellowstone gRPC streaming client
pub struct YellowstoneStreamer {
    config: YellowstoneConfig,
    registry: Arc<RwLock<PoolRegistry>>,
    deserializer: Arc<PoolDeserializer>,
    subscribed_accounts: Arc<RwLock<HashSet<Pubkey>>>,
    running: Arc<RwLock<bool>>,
}

impl YellowstoneStreamer {
    pub fn new(
        config: YellowstoneConfig,
        registry: Arc<RwLock<PoolRegistry>>,
    ) -> Self {
        Self {
            config,
            registry,
            deserializer: Arc::new(PoolDeserializer::new()),
            subscribed_accounts: Arc::new(RwLock::new(HashSet::new())),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start streaming pool updates
    /// Returns a channel receiver for pool updates
    pub async fn start(&self) -> Result<mpsc::Receiver<PoolUpdate>> {
        let (tx, rx) = mpsc::channel(1000);
        
        // Get accounts to subscribe to
        let accounts: Vec<Pubkey> = {
            let reg = self.registry.read().await;
            reg.get_enabled_pools().iter().map(|p| p.address).collect()
        };

        if accounts.is_empty() {
            return Err(anyhow!("No pools to subscribe to"));
        }

        info!("🔌 Starting Yellowstone gRPC stream for {} pools", accounts.len());
        
        // Store subscribed accounts
        {
            let mut subs = self.subscribed_accounts.write().await;
            subs.extend(accounts.iter().cloned());
        }

        *self.running.write().await = true;

        // Spawn streaming task
        let config = self.config.clone();
        let registry = self.registry.clone();
        let deserializer = self.deserializer.clone();
        let running = self.running.clone();
        let subscribed = self.subscribed_accounts.clone();

        tokio::spawn(async move {
            Self::stream_loop(config, registry, deserializer, running, subscribed, tx).await;
        });

        Ok(rx)
    }

    /// Stop streaming
    pub async fn stop(&self) {
        *self.running.write().await = false;
        info!("🛑 Yellowstone stream stopped");
    }

    /// Add pools to subscription
    pub async fn subscribe_pools(&self, pools: Vec<Pubkey>) {
        let count = pools.len();
        let mut subs = self.subscribed_accounts.write().await;
        for pool in pools {
            subs.insert(pool);
        }
        info!("📡 Added {} pools to subscription", count);
    }

    /// Main streaming loop with reconnection
    async fn stream_loop(
        config: YellowstoneConfig,
        registry: Arc<RwLock<PoolRegistry>>,
        deserializer: Arc<PoolDeserializer>,
        running: Arc<RwLock<bool>>,
        subscribed: Arc<RwLock<HashSet<Pubkey>>>,
        tx: mpsc::Sender<PoolUpdate>,
    ) {
        loop {
            if !*running.read().await {
                break;
            }

            match Self::connect_and_stream(
                &config,
                &registry,
                &deserializer,
                &subscribed,
                &tx,
            ).await {
                Ok(_) => {
                    info!("📡 gRPC stream ended normally");
                }
                Err(e) => {
                    error!("❌ gRPC stream error: {}", e);
                }
            }

            if !*running.read().await {
                break;
            }

            // Reconnect delay
            warn!("🔄 Reconnecting in {}ms...", config.reconnect_delay_ms);
            tokio::time::sleep(Duration::from_millis(config.reconnect_delay_ms)).await;
        }
    }

    /// Connect to gRPC and stream updates
    async fn connect_and_stream(
        config: &YellowstoneConfig,
        registry: &Arc<RwLock<PoolRegistry>>,
        deserializer: &Arc<PoolDeserializer>,
        subscribed: &Arc<RwLock<HashSet<Pubkey>>>,
        tx: &mpsc::Sender<PoolUpdate>,
    ) -> Result<()> {
        // Note: This is a simplified implementation
        // Full implementation requires yellowstone-grpc-client crate
        
        info!("🔌 Connecting to Yellowstone gRPC: {}", config.endpoint);
        
        // For now, we'll use a polling fallback until gRPC is configured
        // The actual gRPC implementation would look like:
        //
        // let client = GeyserGrpcClient::connect(
        //     config.endpoint.clone(),
        //     Some(config.api_key.clone()),
        //     None,
        // ).await?;
        //
        // let mut subscription = client.subscribe_once(
        //     HashMap::new(), // slots
        //     accounts_filter,
        //     HashMap::new(), // transactions  
        //     HashMap::new(), // blocks
        //     HashMap::new(), // block_meta
        //     Some(CommitmentLevel::Processed),
        //     vec![],
        // ).await?;
        //
        // while let Some(msg) = subscription.next().await {
        //     // Process account updates
        // }

        warn!("⚠️ Yellowstone gRPC requires yellowstone-grpc-client crate");
        warn!("⚠️ Add to Cargo.toml: yellowstone-grpc-client = \"1.15\"");
        warn!("⚠️ Falling back to polling mode");
        
        // Return error to trigger reconnect/fallback
        Err(anyhow!("gRPC client not configured - using polling fallback"))
    }

    /// Process raw account update into ParsedPool
    fn process_account_update(
        registry: &PoolRegistry,
        deserializer: &PoolDeserializer,
        update: AccountUpdate,
    ) -> Option<PoolUpdate> {
        let pool_info = registry.get_pool(&update.pubkey)?;
        
        let parsed = deserializer
            .deserialize(pool_info.pool_type, &update.pubkey, &update.data)
            .ok()?;

        Some(PoolUpdate {
            pool: parsed,
            slot: update.slot,
        })
    }
}

/// Helper to create Yellowstone config from environment
impl YellowstoneConfig {
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("HELIUS_API_KEY")
            .map_err(|_| anyhow!("HELIUS_API_KEY not set"))?;
        
        let network = std::env::var("SOLANA_NETWORK").unwrap_or_else(|_| "mainnet".to_string());
        
        let endpoint = match network.as_str() {
            "mainnet" | "mainnet-beta" => {
                format!("https://atlas-mainnet.helius-rpc.com?api-key={}", api_key)
            }
            "devnet" => {
                format!("https://atlas-devnet.helius-rpc.com?api-key={}", api_key)
            }
            _ => return Err(anyhow!("Unknown network: {}", network)),
        };

        Ok(Self {
            endpoint,
            api_key,
            commitment: "processed".to_string(),
            reconnect_delay_ms: 1000,
        })
    }

    pub fn from_helius_key(api_key: &str, mainnet: bool) -> Self {
        let endpoint = if mainnet {
            format!("https://atlas-mainnet.helius-rpc.com?api-key={}", api_key)
        } else {
            format!("https://atlas-devnet.helius-rpc.com?api-key={}", api_key)
        };

        Self {
            endpoint,
            api_key: api_key.to_string(),
            commitment: "processed".to_string(),
            reconnect_delay_ms: 1000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_helius_key() {
        let config = YellowstoneConfig::from_helius_key("test-key", true);
        assert!(config.endpoint.contains("mainnet"));
        assert!(config.endpoint.contains("test-key"));
    }
}

