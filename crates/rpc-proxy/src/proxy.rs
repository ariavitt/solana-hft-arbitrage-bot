//! Main RPC Proxy implementation

use crate::cache::CacheLayer;
use crate::client::{RpcClientWrapper, RetryConfig};
use anyhow::Result;
use bot_core::config::RpcConfig;
use solana_sdk::{account::Account, hash::Hash, pubkey::Pubkey};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// RPC Proxy configuration
pub struct ProxyConfig {
    pub rpc: RpcConfig,
    pub redis_url: String,
    pub cache_ttl_ms: u64,
}

pub struct RpcProxy {
    primary: RpcClientWrapper,
    fallbacks: Vec<RpcClientWrapper>,
    cache: CacheLayer,
    config: ProxyConfig,
}

impl RpcProxy {
    /// Create new RPC Proxy with full configuration
    pub async fn new(config: ProxyConfig) -> Result<Self> {
        let timeout = Duration::from_millis(config.rpc.timeout_ms);
        let retry_config = RetryConfig {
            max_retries: config.rpc.max_retries,
            initial_backoff_ms: 100,
            max_backoff_ms: 2000,
        };

        let primary = RpcClientWrapper::with_retry_config(
            &config.rpc.primary_url,
            "primary",
            timeout,
            retry_config.clone(),
        );

        let fallbacks: Vec<_> = config.rpc
            .fallback_urls
            .iter()
            .enumerate()
            .map(|(i, url)| {
                RpcClientWrapper::with_retry_config(
                    url,
                    &format!("fallback_{}", i),
                    timeout,
                    retry_config.clone(),
                )
            })
            .collect();

        let cache = CacheLayer::new(&config.redis_url, config.cache_ttl_ms).await?;

        info!(
            "RPC Proxy initialized: primary={}, fallbacks={}, cache_ttl={}ms",
            config.rpc.primary_url,
            fallbacks.len(),
            config.cache_ttl_ms
        );

        Ok(Self {
            primary,
            fallbacks,
            cache,
            config,
        })
    }

    /// Simple constructor for backward compatibility
    pub async fn from_rpc_config(rpc_config: &RpcConfig, redis_url: &str, cache_ttl_ms: u64) -> Result<Self> {
        Self::new(ProxyConfig {
            rpc: rpc_config.clone(),
            redis_url: redis_url.to_string(),
            cache_ttl_ms,
        }).await
    }

    /// Get multiple accounts with caching
    pub async fn get_multiple_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> Result<Vec<Option<Account>>> {
        let start = Instant::now();
        
        if pubkeys.is_empty() {
            return Ok(vec![]);
        }

        // Check cache first
        let cached = self.cache.get_multiple(pubkeys).await;

        // Find which accounts need to be fetched
        let mut to_fetch: Vec<(usize, Pubkey)> = Vec::new();
        let mut cache_hits = 0usize;
        
        for (i, (pubkey, cached_account)) in pubkeys.iter().zip(cached.iter()).enumerate() {
            if cached_account.is_none() {
                to_fetch.push((i, *pubkey));
            } else {
                cache_hits += 1;
            }
        }

        if to_fetch.is_empty() {
            // All accounts were cached
            metrics::counter!("rpc_proxy_cache_hits").increment(pubkeys.len() as u64);
            debug!(
                "get_multiple_accounts: all {} from cache in {:?}",
                pubkeys.len(),
                start.elapsed()
            );
            return Ok(cached);
        }

        metrics::counter!("rpc_proxy_cache_hits").increment(cache_hits as u64);
        metrics::counter!("rpc_proxy_cache_misses").increment(to_fetch.len() as u64);

        // Fetch missing accounts from RPC
        let fetch_pubkeys: Vec<_> = to_fetch.iter().map(|(_, pk)| *pk).collect();
        let fetched = self.fetch_with_fallback(&fetch_pubkeys).await?;

        // Merge results
        let mut results = cached;
        for ((idx, pubkey), account) in to_fetch.iter().zip(fetched.iter()) {
            results[*idx] = account.clone();

            // Update cache
            if let Some(acc) = account {
                self.cache.set_account(pubkey, acc).await;
            }
        }

        debug!(
            "get_multiple_accounts: {} total, {} from cache, {} from RPC in {:?}",
            pubkeys.len(),
            cache_hits,
            to_fetch.len(),
            start.elapsed()
        );

        Ok(results)
    }

    /// Get single account with caching
    pub async fn get_account(&self, pubkey: &Pubkey) -> Result<Option<Account>> {
        let results = self.get_multiple_accounts(&[*pubkey]).await?;
        Ok(results.into_iter().next().flatten())
    }

    /// Get SOL balance
    pub async fn get_balance(&self, pubkey: &Pubkey) -> Result<u64> {
        self.primary.get_balance(pubkey).await
    }

    async fn fetch_with_fallback(&self, pubkeys: &[Pubkey]) -> Result<Vec<Option<Account>>> {
        // Try primary first
        match self.primary.get_multiple_accounts(pubkeys).await {
            Ok(accounts) => {
                metrics::counter!("rpc_proxy_primary_success").increment(1);
                return Ok(accounts);
            }
            Err(e) => {
                metrics::counter!("rpc_proxy_primary_failure").increment(1);
                warn!("Primary RPC failed: {}, trying fallbacks", e);
            }
        }

        // Try fallbacks
        for (i, fallback) in self.fallbacks.iter().enumerate() {
            match fallback.get_multiple_accounts(pubkeys).await {
                Ok(accounts) => {
                    metrics::counter!("rpc_proxy_fallback_success", "index" => i.to_string()).increment(1);
                    info!("Fallback {} succeeded after primary failure", i);
                    return Ok(accounts);
                }
                Err(e) => {
                    metrics::counter!("rpc_proxy_fallback_failure", "index" => i.to_string()).increment(1);
                    warn!("Fallback RPC {} failed: {}", fallback.url(), e);
                }
            }
        }

        metrics::counter!("rpc_proxy_all_failed").increment(1);
        Err(anyhow::anyhow!("All RPC endpoints failed"))
    }

    /// Get current slot
    pub async fn get_slot(&self) -> Result<u64> {
        self.primary.get_slot().await
    }

    /// Get latest blockhash
    pub async fn get_latest_blockhash(&self) -> Result<Hash> {
        self.primary.get_latest_blockhash().await
    }

    /// Check if connected (by getting slot)
    pub async fn health_check(&self) -> Result<()> {
        let slot = self.get_slot().await?;
        info!("Health check passed, current slot: {}", slot);
        Ok(())
    }

    /// Get primary RPC URL
    pub fn primary_url(&self) -> &str {
        &self.config.rpc.primary_url
    }
}
