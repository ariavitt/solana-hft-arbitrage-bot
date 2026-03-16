//! RPC Client wrapper with metrics and retry logic

use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{account::Account, commitment_config::CommitmentConfig, pubkey::Pubkey, hash::Hash};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{debug, error, warn};

/// Configuration for retry behavior
#[derive(Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 2000,
        }
    }
}

pub struct RpcClientWrapper {
    client: RpcClient,
    name: String,
    retry_config: RetryConfig,
}

impl RpcClientWrapper {
    pub fn new(url: &str, name: &str, timeout: Duration) -> Self {
        Self::with_retry_config(url, name, timeout, RetryConfig::default())
    }

    pub fn with_retry_config(url: &str, name: &str, timeout: Duration, retry_config: RetryConfig) -> Self {
        let client = RpcClient::new_with_timeout_and_commitment(
            url.to_string(),
            timeout,
            CommitmentConfig::confirmed(),
        );

        Self {
            client,
            name: name.to_string(),
            retry_config,
        }
    }

    /// Execute with retry and exponential backoff
    async fn with_retry<T, F, Fut>(&self, operation_name: &str, f: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, solana_client::client_error::ClientError>>,
    {
        let mut last_error = None;
        let mut backoff_ms = self.retry_config.initial_backoff_ms;

        for attempt in 0..=self.retry_config.max_retries {
            if attempt > 0 {
                warn!(
                    "RPC {} {}: retry attempt {}/{} after {}ms",
                    self.name, operation_name, attempt, self.retry_config.max_retries, backoff_ms
                );
                sleep(Duration::from_millis(backoff_ms)).await;
                backoff_ms = (backoff_ms * 2).min(self.retry_config.max_backoff_ms);
            }

            let start = Instant::now();
            match f().await {
                Ok(result) => {
                    let latency = start.elapsed();
                    
                    // Record success metrics
                    metrics::histogram!("rpc_latency_ms", "endpoint" => self.name.clone(), "operation" => operation_name.to_string())
                        .record(latency.as_millis() as f64);
                    metrics::counter!("rpc_requests_total", "endpoint" => self.name.clone(), "status" => "success")
                        .increment(1);
                    
                    if attempt > 0 {
                        metrics::counter!("rpc_retries_total", "endpoint" => self.name.clone())
                            .increment(attempt as u64);
                    }

                    debug!(
                        "RPC {} {}: success in {:?}{}",
                        self.name,
                        operation_name,
                        latency,
                        if attempt > 0 { format!(" (after {} retries)", attempt) } else { String::new() }
                    );

                    return Ok(result);
                }
                Err(e) => {
                    metrics::counter!("rpc_requests_total", "endpoint" => self.name.clone(), "status" => "error")
                        .increment(1);
                    
                    last_error = Some(e);
                }
            }
        }

        let err = last_error.unwrap();
        error!(
            "RPC {} {}: failed after {} retries: {}",
            self.name, operation_name, self.retry_config.max_retries, err
        );
        
        Err(anyhow::anyhow!("RPC {} failed: {}", operation_name, err))
    }

    /// Get multiple accounts with retry
    pub async fn get_multiple_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> Result<Vec<Option<Account>>> {
        let pubkeys_owned: Vec<Pubkey> = pubkeys.to_vec();
        
        self.with_retry(
            &format!("getMultipleAccounts({})", pubkeys.len()),
            || {
                let pks = pubkeys_owned.clone();
                async move { self.client.get_multiple_accounts(&pks).await }
            },
        ).await
    }

    /// Get current slot with retry
    pub async fn get_slot(&self) -> Result<u64> {
        self.with_retry("getSlot", || async {
            self.client.get_slot().await
        }).await
    }

    /// Get latest blockhash with retry
    pub async fn get_latest_blockhash(&self) -> Result<Hash> {
        self.with_retry("getLatestBlockhash", || async {
            self.client.get_latest_blockhash().await
        }).await
    }

    /// Get account with retry
    pub async fn get_account(&self, pubkey: &Pubkey) -> Result<Option<Account>> {
        let pk = *pubkey;
        self.with_retry(
            "getAccount",
            || async move {
                match self.client.get_account(&pk).await {
                    Ok(acc) => Ok(Some(acc)),
                    Err(e) => {
                        // Account not found is not an error
                        if e.to_string().contains("AccountNotFound") {
                            Ok(None)
                        } else {
                            Err(e)
                        }
                    }
                }
            },
        ).await
    }

    pub fn url(&self) -> String {
        self.client.url()
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

