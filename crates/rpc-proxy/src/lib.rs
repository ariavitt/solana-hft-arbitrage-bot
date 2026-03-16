//! RPC Proxy Service
//!
//! Provides a caching layer over RPC calls with fallback support.
//!
//! # Features
//!
//! - **Caching**: Redis-based caching with configurable TTL
//! - **Fallback**: Automatic failover to backup RPC endpoints
//! - **Retry**: Exponential backoff retry logic
//! - **Metrics**: Prometheus-compatible metrics
//!
//! # Example
//!
//! ```ignore
//! use rpc_proxy::{RpcProxy, ProxyConfig};
//! use bot_core::config::RpcConfig;
//!
//! let config = ProxyConfig {
//!     rpc: RpcConfig { /* ... */ },
//!     redis_url: "redis://localhost:6379".to_string(),
//!     cache_ttl_ms: 200,
//! };
//!
//! let proxy = RpcProxy::new(config).await?;
//! let accounts = proxy.get_multiple_accounts(&pubkeys).await?;
//! ```

pub mod cache;
pub mod client;
pub mod proxy;

pub use client::{RpcClientWrapper, RetryConfig};
pub use proxy::{ProxyConfig, RpcProxy};

#[cfg(test)]
mod tests {
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    /// Test pubkeys for integration tests (well-known programs)
    pub fn test_pubkeys() -> Vec<Pubkey> {
        vec![
            // System Program
            Pubkey::from_str("11111111111111111111111111111111").unwrap(),
            // Token Program
            Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap(),
            // Associated Token Program
            Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL").unwrap(),
        ]
    }

    #[tokio::test]
    async fn test_client_get_slot() {
        use crate::client::RpcClientWrapper;
        use std::time::Duration;

        let client = RpcClientWrapper::new(
            "https://api.devnet.solana.com",
            "test",
            Duration::from_secs(10),
        );

        let result = client.get_slot().await;
        assert!(result.is_ok(), "Failed to get slot: {:?}", result.err());
        
        let slot = result.unwrap();
        assert!(slot > 0, "Slot should be positive");
        println!("Current devnet slot: {}", slot);
    }

    #[tokio::test]
    async fn test_client_get_multiple_accounts() {
        use crate::client::RpcClientWrapper;
        use std::time::Duration;

        let client = RpcClientWrapper::new(
            "https://api.devnet.solana.com",
            "test",
            Duration::from_secs(10),
        );

        let pubkeys = test_pubkeys();
        let result = client.get_multiple_accounts(&pubkeys).await;
        
        assert!(result.is_ok(), "Failed to get accounts: {:?}", result.err());
        
        let accounts = result.unwrap();
        assert_eq!(accounts.len(), pubkeys.len());
        
        // System program should exist and be executable
        assert!(accounts[0].is_some(), "System program should exist");
        let system_program = accounts[0].as_ref().unwrap();
        assert!(system_program.executable, "System program should be executable");
        
        println!("Got {} accounts", accounts.iter().filter(|a| a.is_some()).count());
    }

    #[tokio::test]
    async fn test_client_retry_config() {
        use crate::client::{RpcClientWrapper, RetryConfig};
        use std::time::Duration;

        let retry_config = RetryConfig {
            max_retries: 2,
            initial_backoff_ms: 50,
            max_backoff_ms: 500,
        };

        let client = RpcClientWrapper::with_retry_config(
            "https://api.devnet.solana.com",
            "test_retry",
            Duration::from_secs(10),
            retry_config,
        );

        let result = client.get_slot().await;
        assert!(result.is_ok(), "Retry-configured client should work");
        println!("Retry client got slot: {}", result.unwrap());
    }

    #[tokio::test]
    async fn test_client_get_latest_blockhash() {
        use crate::client::RpcClientWrapper;
        use std::time::Duration;

        let client = RpcClientWrapper::new(
            "https://api.devnet.solana.com",
            "test",
            Duration::from_secs(10),
        );

        let result = client.get_latest_blockhash().await;
        assert!(result.is_ok(), "Failed to get blockhash: {:?}", result.err());
        
        let blockhash = result.unwrap();
        println!("Latest blockhash: {}", blockhash);
    }
}

/// Integration tests that require Redis
#[cfg(test)]
mod integration_tests {
    use crate::proxy::{ProxyConfig, RpcProxy};
    use bot_core::config::RpcConfig;
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    fn redis_available() -> bool {
        // Quick check if Redis is available
        std::env::var("REDIS_URL").is_ok() || {
            // Try default connection
            std::process::Command::new("redis-cli")
                .arg("ping")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        }
    }

    #[tokio::test]
    async fn test_proxy_with_cache() {
        if !redis_available() {
            println!("⚠️  Redis not available, skipping integration test");
            println!("   Start Redis with: docker run -d -p 6379:6379 redis:alpine");
            return;
        }

        let config = ProxyConfig {
            rpc: RpcConfig {
                primary_url: "https://api.devnet.solana.com".to_string(),
                fallback_urls: vec![],
                timeout_ms: 10000,
                max_retries: 3,
            },
            redis_url: "redis://localhost:6379".to_string(),
            cache_ttl_ms: 1000,
        };

        let proxy = RpcProxy::new(config).await;
        assert!(proxy.is_ok(), "Failed to create proxy: {:?}", proxy.err());
        let proxy = proxy.unwrap();

        // Health check
        let health = proxy.health_check().await;
        assert!(health.is_ok(), "Health check failed: {:?}", health.err());

        // Get accounts - first call should fetch from RPC
        let pubkeys = vec![
            Pubkey::from_str("11111111111111111111111111111111").unwrap(),
            Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap(),
        ];

        let accounts1 = proxy.get_multiple_accounts(&pubkeys).await;
        assert!(accounts1.is_ok(), "First fetch failed: {:?}", accounts1.err());
        println!("First fetch: {} accounts", accounts1.unwrap().iter().filter(|a| a.is_some()).count());

        // Second call should use cache
        let accounts2 = proxy.get_multiple_accounts(&pubkeys).await;
        assert!(accounts2.is_ok(), "Cached fetch failed: {:?}", accounts2.err());
        println!("Cached fetch: {} accounts", accounts2.unwrap().iter().filter(|a| a.is_some()).count());

        println!("✅ Proxy with cache test passed!");
    }

    #[tokio::test]
    async fn test_proxy_single_account() {
        if !redis_available() {
            println!("⚠️  Redis not available, skipping test");
            return;
        }

        let config = ProxyConfig {
            rpc: RpcConfig {
                primary_url: "https://api.devnet.solana.com".to_string(),
                fallback_urls: vec![],
                timeout_ms: 10000,
                max_retries: 3,
            },
            redis_url: "redis://localhost:6379".to_string(),
            cache_ttl_ms: 500,
        };

        let proxy = RpcProxy::new(config).await.unwrap();
        
        let system_program = Pubkey::from_str("11111111111111111111111111111111").unwrap();
        let account = proxy.get_account(&system_program).await;
        
        assert!(account.is_ok(), "Failed to get account: {:?}", account.err());
        assert!(account.unwrap().is_some(), "System program should exist");
        
        println!("✅ Single account test passed!");
    }
}

