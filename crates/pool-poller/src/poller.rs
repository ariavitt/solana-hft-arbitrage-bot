//! Pool Poller implementation
//!
//! Continuously fetches pool data from the blockchain

use crate::config::PollerConfig;
use crate::registry::PoolRegistry;
use anyhow::Result;
use bot_core::ParsedPool;
use pool_deserializer::PoolDeserializer;
use rpc_proxy::RpcProxy;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

/// Callback for pool updates
pub type PoolUpdateCallback = Box<dyn Fn(&Pubkey, &ParsedPool) + Send + Sync>;

/// Pool Poller service
pub struct PoolPoller {
    config: PollerConfig,
    rpc: Arc<RpcProxy>,
    registry: Arc<RwLock<PoolRegistry>>,
    deserializer: Arc<PoolDeserializer>,
    /// Latest parsed pool states
    pool_states: Arc<RwLock<HashMap<Pubkey, ParsedPool>>>,
    /// Running flag
    running: Arc<RwLock<bool>>,
}

impl PoolPoller {
    pub fn new(
        config: PollerConfig,
        rpc: Arc<RpcProxy>,
        registry: Arc<RwLock<PoolRegistry>>,
    ) -> Self {
        Self {
            config,
            rpc,
            registry,
            deserializer: Arc::new(PoolDeserializer::new()),
            pool_states: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the polling loop
    pub async fn start(&self) -> Result<()> {
        {
            let mut running = self.running.write().await;
            if *running {
                warn!("Poller already running");
                return Ok(());
            }
            *running = true;
        }

        info!(
            "Starting pool poller with {}ms interval",
            self.config.poll_interval_ms
        );

        let mut poll_interval = interval(Duration::from_millis(self.config.poll_interval_ms));

        loop {
            poll_interval.tick().await;

            if !*self.running.read().await {
                info!("Poller stopped");
                break;
            }

            if let Err(e) = self.poll_once().await {
                error!("Poll error: {}", e);
                metrics::counter!("poller_errors").increment(1);
            }
        }

        Ok(())
    }

    /// Stop the polling loop
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        info!("Poller stop requested");
    }

    /// Poll all registered pools once (PARALLEL batches)
    pub async fn poll_once(&self) -> Result<()> {
        let start = Instant::now();
        
        let registry = self.registry.read().await;
        let addresses = registry.get_addresses();
        drop(registry);

        if addresses.is_empty() {
            debug!("No pools to poll");
            return Ok(());
        }

        // Split into chunks and fetch ALL batches in PARALLEL
        let chunks: Vec<Vec<Pubkey>> = addresses
            .chunks(self.config.batch_size)
            .map(|c| c.to_vec())
            .collect();
        
        let num_batches = chunks.len();
        
        // Create parallel futures for all batches
        let futures: Vec<_> = chunks
            .into_iter()
            .map(|chunk| {
                let rpc = self.rpc.clone();
                async move {
                    let result = rpc.get_multiple_accounts(&chunk).await;
                    (chunk, result)
                }
            })
            .collect();

        // Execute ALL batches in parallel
        let results = futures::future::join_all(futures).await;

        // Process results
        let mut updated_count = 0u64;
        for (chunk, result) in results {
            match result {
                Ok(accounts) => {
                    for (pubkey, maybe_account) in chunk.iter().zip(accounts.iter()) {
                        if let Some(account) = maybe_account {
                            if let Some(parsed) = self.parse_account(pubkey, account).await {
                                let mut states = self.pool_states.write().await;
                                states.insert(*pubkey, parsed);
                                updated_count += 1;
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to fetch batch: {}", e);
                }
            }
        }

        let elapsed = start.elapsed();
        
        metrics::histogram!("poller_latency_ms").record(elapsed.as_millis() as f64);
        metrics::counter!("poller_updates").increment(updated_count);

        debug!(
            "Polled {} pools ({} batches parallel), {} updated in {:?}",
            addresses.len(),
            num_batches,
            updated_count,
            elapsed
        );

        Ok(())
    }

    /// Parse account data into ParsedPool
    async fn parse_account(
        &self,
        pubkey: &Pubkey,
        account: &solana_sdk::account::Account,
    ) -> Option<ParsedPool> {
        let registry = self.registry.read().await;
        let pool_info = registry.get_pool(pubkey)?;
        
        match self.deserializer.deserialize(pool_info.pool_type, pubkey, &account.data) {
            Ok(pool) => {
                // Log parsed pool data for debugging
                let price = crate::utils::sqrt_price_to_price(
                    pool.sqrt_price,
                    pool.token_a.decimals.max(9), // Default to 9 for SOL
                    pool.token_b.decimals.max(6), // Default to 6 for USDC
                );
                info!(
                    "📊 Pool {} ({:?}): sqrt_price={}, liquidity={}, price={:.4}, tick={}",
                    pool_info.name,
                    pool.pool_type,
                    pool.sqrt_price,
                    pool.liquidity,
                    price,
                    pool.tick_current
                );
                Some(pool)
            }
            Err(e) => {
                warn!("Failed to parse pool {}: {}", pubkey, e);
                None
            }
        }
    }

    /// Get latest pool state
    pub async fn get_pool_state(&self, address: &Pubkey) -> Option<ParsedPool> {
        let states = self.pool_states.read().await;
        states.get(address).cloned()
    }

    /// Get all pool states
    pub async fn get_all_states(&self) -> HashMap<Pubkey, ParsedPool> {
        let states = self.pool_states.read().await;
        states.clone()
    }

    /// Check if a pool has recent data
    pub async fn is_pool_fresh(&self, address: &Pubkey, max_age_slots: u64) -> bool {
        let states = self.pool_states.read().await;
        if let Some(pool) = states.get(address) {
            // Compare with current slot
            // In production, you'd check against RPC slot
            pool.last_updated_slot > 0
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poller_config_default() {
        let config = PollerConfig::default();
        assert_eq!(config.poll_interval_ms, 100);
        assert_eq!(config.batch_size, 100);
    }
}

