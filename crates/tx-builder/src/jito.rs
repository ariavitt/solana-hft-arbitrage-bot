//! Jito Bundle integration
//!
//! Provides MEV protection through atomic bundle submission

use anyhow::{anyhow, Result};
use solana_sdk::{
    pubkey::Pubkey,
    signature::Signature,
    transaction::Transaction,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::{debug, info, warn};

// ============================================
// Jito Configuration
// ============================================

/// Jito Block Engine endpoints
pub const JITO_MAINNET_BLOCK_ENGINE: &str = "https://mainnet.block-engine.jito.wtf";
pub const JITO_AMSTERDAM_BLOCK_ENGINE: &str = "https://amsterdam.mainnet.block-engine.jito.wtf";
pub const JITO_FRANKFURT_BLOCK_ENGINE: &str = "https://frankfurt.mainnet.block-engine.jito.wtf";
pub const JITO_NY_BLOCK_ENGINE: &str = "https://ny.mainnet.block-engine.jito.wtf";
pub const JITO_TOKYO_BLOCK_ENGINE: &str = "https://tokyo.mainnet.block-engine.jito.wtf";

/// Jito tip accounts (random selection recommended)
pub const JITO_TIP_ACCOUNTS: [&str; 8] = [
    "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5",
    "HFqU5x63VTqvQss8hp11i4bVgYxSzqAYHJoRBXH6UdUk",
    "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY",
    "ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49",
    "DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh",
    "ADuUkR4vqLUMWXxW9gh6D6L8pMSawimctcNZ5pGwDcEt",
    "DttWaMuVvTiduZRnguLF7jNxTgiMBZ1hyAumKUiL2KRL",
    "3AVi9Tg9Uo68tJfuvoKvqKNWKkC5wPdSSdeBnizKZ6jT",
];

/// Jito bundle configuration
#[derive(Debug, Clone)]
pub struct JitoConfig {
    /// Block engine URL
    pub block_engine_url: String,
    /// Tip amount in lamports
    pub tip_lamports: u64,
    /// Maximum bundle size (usually 5)
    pub max_bundle_size: usize,
    /// Timeout for bundle submission (ms)
    pub timeout_ms: u64,
}

impl Default for JitoConfig {
    fn default() -> Self {
        Self {
            block_engine_url: JITO_MAINNET_BLOCK_ENGINE.to_string(),
            tip_lamports: 10_000, // 0.00001 SOL
            max_bundle_size: 5,
            timeout_ms: 5000,
        }
    }
}

// ============================================
// Bundle Types
// ============================================

/// A Jito bundle containing multiple transactions
#[derive(Debug, Clone)]
pub struct JitoBundle {
    /// Transactions in the bundle
    pub transactions: Vec<Transaction>,
    /// Tip transaction (last in bundle)
    pub tip_transaction: Option<Transaction>,
}

/// Bundle submission result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleResult {
    /// Bundle UUID
    pub bundle_id: String,
    /// Status
    pub status: BundleStatus,
    /// Signatures of included transactions
    pub signatures: Vec<String>,
}

/// Bundle status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BundleStatus {
    /// Bundle accepted by block engine
    Accepted,
    /// Bundle is being processed
    Processing,
    /// Bundle landed on-chain
    Landed,
    /// Bundle failed
    Failed(String),
}

// ============================================
// Jito Client
// ============================================

/// Jito Bundle client
pub struct JitoClient {
    config: JitoConfig,
    http_client: reqwest::Client,
}

impl JitoClient {
    /// Create new Jito client
    pub fn new(config: JitoConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            http_client,
        }
    }

    /// Get a random tip account
    pub fn get_tip_account(&self) -> Pubkey {
        use rand::Rng;
        let idx = rand::thread_rng().gen_range(0..JITO_TIP_ACCOUNTS.len());
        Pubkey::from_str(JITO_TIP_ACCOUNTS[idx]).unwrap()
    }

    /// Create a tip instruction
    pub fn create_tip_instruction(
        &self,
        payer: &Pubkey,
    ) -> solana_sdk::instruction::Instruction {
        let tip_account = self.get_tip_account();
        
        solana_sdk::system_instruction::transfer(
            payer,
            &tip_account,
            self.config.tip_lamports,
        )
    }

    /// Submit a bundle to Jito
    pub async fn send_bundle(&self, bundle: &JitoBundle) -> Result<BundleResult> {
        let endpoint = format!("{}/api/v1/bundles", self.config.block_engine_url);

        // Serialize transactions
        let serialized_txs: Vec<String> = bundle
            .transactions
            .iter()
            .chain(bundle.tip_transaction.iter())
            .map(|tx| {
                let bytes = bincode::serialize(tx)
                    .expect("Failed to serialize transaction");
                base64::encode(&bytes)
            })
            .collect();

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendBundle",
            "params": [serialized_txs]
        });

        info!("Submitting bundle with {} transactions", serialized_txs.len());

        let response = self
            .http_client
            .post(&endpoint)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            warn!("Jito bundle submission failed: {}", error_text);
            return Err(anyhow!("Bundle submission failed: {}", error_text));
        }

        let result: serde_json::Value = response.json().await?;
        
        if let Some(error) = result.get("error") {
            return Err(anyhow!("Jito error: {}", error));
        }

        let bundle_id = result["result"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        info!("✅ Bundle submitted: {}", bundle_id);

        Ok(BundleResult {
            bundle_id,
            status: BundleStatus::Accepted,
            signatures: bundle
                .transactions
                .iter()
                .map(|tx| tx.signatures[0].to_string())
                .collect(),
        })
    }

    /// Check bundle status
    pub async fn get_bundle_status(&self, bundle_id: &str) -> Result<BundleStatus> {
        let endpoint = format!(
            "{}/api/v1/bundles/{}",
            self.config.block_engine_url,
            bundle_id
        );

        let response = self.http_client.get(&endpoint).send().await?;
        let result: serde_json::Value = response.json().await?;

        // Parse status from response
        if let Some(status) = result.get("status") {
            match status.as_str() {
                Some("Landed") => Ok(BundleStatus::Landed),
                Some("Processing") => Ok(BundleStatus::Processing),
                Some("Accepted") => Ok(BundleStatus::Accepted),
                Some(s) => Ok(BundleStatus::Failed(s.to_string())),
                None => Ok(BundleStatus::Processing),
            }
        } else {
            Ok(BundleStatus::Processing)
        }
    }

    /// Send bundle and wait for confirmation
    pub async fn send_and_confirm_bundle(
        &self,
        bundle: &JitoBundle,
        max_retries: u32,
    ) -> Result<BundleResult> {
        let mut result = self.send_bundle(bundle).await?;

        for attempt in 0..max_retries {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            
            let status = self.get_bundle_status(&result.bundle_id).await?;
            result.status = status.clone();

            match status {
                BundleStatus::Landed => {
                    info!("🎯 Bundle landed on-chain!");
                    return Ok(result);
                }
                BundleStatus::Failed(reason) => {
                    warn!("Bundle failed: {}", reason);
                    return Err(anyhow!("Bundle failed: {}", reason));
                }
                _ => {
                    debug!("Bundle status: {:?} (attempt {})", status, attempt + 1);
                }
            }
        }

        warn!("Bundle confirmation timeout");
        Ok(result)
    }
}

// ============================================
// Bundle Builder
// ============================================

/// Builder for creating Jito bundles
pub struct BundleBuilder {
    transactions: Vec<Transaction>,
    jito_config: JitoConfig,
}

impl BundleBuilder {
    pub fn new(jito_config: JitoConfig) -> Self {
        Self {
            transactions: Vec::new(),
            jito_config,
        }
    }

    /// Add transaction to bundle
    pub fn add_transaction(&mut self, tx: Transaction) -> &mut Self {
        self.transactions.push(tx);
        self
    }

    /// Build the bundle with tip
    pub fn build(
        self,
        payer: &Pubkey,
        recent_blockhash: solana_sdk::hash::Hash,
        signer: &dyn solana_sdk::signer::Signer,
    ) -> Result<JitoBundle> {
        if self.transactions.is_empty() {
            return Err(anyhow!("Bundle must contain at least one transaction"));
        }

        if self.transactions.len() > self.jito_config.max_bundle_size - 1 {
            return Err(anyhow!(
                "Bundle too large: {} transactions (max {})",
                self.transactions.len(),
                self.jito_config.max_bundle_size - 1
            ));
        }

        // Create tip transaction
        let tip_account = {
            use rand::Rng;
            let idx = rand::thread_rng().gen_range(0..JITO_TIP_ACCOUNTS.len());
            Pubkey::from_str(JITO_TIP_ACCOUNTS[idx]).unwrap()
        };

        let tip_ix = solana_sdk::system_instruction::transfer(
            payer,
            &tip_account,
            self.jito_config.tip_lamports,
        );

        let tip_tx = Transaction::new_signed_with_payer(
            &[tip_ix],
            Some(payer),
            &[signer],
            recent_blockhash,
        );

        Ok(JitoBundle {
            transactions: self.transactions,
            tip_transaction: Some(tip_tx),
        })
    }

    /// Build bundle without tip (for testing)
    pub fn build_without_tip(self) -> Result<JitoBundle> {
        if self.transactions.is_empty() {
            return Err(anyhow!("Bundle must contain at least one transaction"));
        }

        Ok(JitoBundle {
            transactions: self.transactions,
            tip_transaction: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tip_accounts() {
        for account in JITO_TIP_ACCOUNTS.iter() {
            let pubkey = Pubkey::from_str(account);
            assert!(pubkey.is_ok(), "Invalid tip account: {}", account);
        }
    }

    #[test]
    fn test_jito_config_default() {
        let config = JitoConfig::default();
        assert_eq!(config.tip_lamports, 10_000);
        assert_eq!(config.max_bundle_size, 5);
    }

    #[test]
    fn test_get_tip_account() {
        let client = JitoClient::new(JitoConfig::default());
        let tip = client.get_tip_account();
        assert!(JITO_TIP_ACCOUNTS.iter().any(|a| Pubkey::from_str(a).unwrap() == tip));
    }
}



