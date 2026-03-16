//! Transaction executor with Jito bundle support

use anyhow::Result;
use rpc_proxy::RpcProxy;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info, warn};
use tx_builder::{BundleBuilder, JitoClient, JitoConfig};

/// Transaction executor with Jito support
pub struct Executor {
    rpc: Arc<RpcProxy>,
    keypair: Arc<Keypair>,
    use_jito: bool,
    dry_run: bool,
    jito_client: Option<JitoClient>,
    jito_config: JitoConfig,
    /// Direct RPC client for sending transactions
    send_client: RpcClient,
}

impl Executor {
    pub fn new(
        rpc: Arc<RpcProxy>,
        keypair: Arc<Keypair>,
        use_jito: bool,
        dry_run: bool,
    ) -> Self {
        // Use mainnet RPC for simulate (pools exist there)
        let send_client = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
        
        let jito_config = JitoConfig::default();
        let jito_client = if use_jito && !dry_run {
            Some(JitoClient::new(jito_config.clone()))
        } else {
            None
        };
        
        Self {
            rpc,
            keypair,
            use_jito,
            dry_run,
            jito_client,
            jito_config,
            send_client,
        }
    }

    /// Set Jito tip amount
    pub fn set_jito_tip(&mut self, tip_lamports: u64) {
        self.jito_config.tip_lamports = tip_lamports;
        if self.use_jito && !self.dry_run {
            self.jito_client = Some(JitoClient::new(self.jito_config.clone()));
        }
    }

    /// Execute a transaction
    pub async fn execute(&self, tx: &Transaction) -> Result<String> {
        if self.dry_run {
            return self.simulate(tx).await;
        }

        if self.use_jito {
            self.send_jito_bundle(tx).await
        } else {
            self.send_transaction(tx).await
        }
    }

    /// Simulate transaction (dry run)
    pub async fn simulate(&self, tx: &Transaction) -> Result<String> {
        let start = Instant::now();
        
        let result = self.send_client.simulate_transaction(tx).await?;
        
        let elapsed = start.elapsed();
        debug!("Simulation completed in {:?}", elapsed);

        if let Some(err) = result.value.err {
            warn!("❌ Simulation failed: {:?}", err);
            if let Some(logs) = result.value.logs {
                for log in &logs {
                    warn!("  {}", log);
                }
            }
            return Err(anyhow::anyhow!("Simulation failed: {:?}", err));
        }

        if let Some(logs) = result.value.logs {
            for log in &logs {
                debug!("  {}", log);
            }
        }

        let units = result.value.units_consumed.unwrap_or(0);
        info!("✅ Simulation success, compute units: {}", units);
        
        metrics::histogram!("simulation_units").record(units as f64);

        Ok("DRY_RUN_SIMULATION".to_string())
    }

    /// Send transaction via standard RPC
    pub async fn send_transaction(&self, tx: &Transaction) -> Result<String> {
        let start = Instant::now();
        
        let sig = self.send_client
            .send_and_confirm_transaction(tx)
            .await
            .map_err(|e| anyhow::anyhow!("Send failed: {}", e))?;

        let elapsed = start.elapsed();
        info!("✅ Transaction confirmed in {:?}: {}", elapsed, sig);
        
        metrics::histogram!("tx_confirm_ms").record(elapsed.as_millis() as f64);
        metrics::counter!("tx_confirmed").increment(1);

        Ok(sig.to_string())
    }

    /// Send transaction via Jito bundle (MEV protection)
    async fn send_jito_bundle(&self, tx: &Transaction) -> Result<String> {
        let jito_client = match &self.jito_client {
            Some(client) => client,
            None => {
                warn!("Jito client not initialized, using standard RPC");
                return self.send_transaction(tx).await;
            }
        };

        let start = Instant::now();
        info!("🚀 Sending via Jito bundle (tip: {} lamports)", self.jito_config.tip_lamports);

        // Get recent blockhash for tip transaction
        let blockhash = self.send_client.get_latest_blockhash().await?;

        // Build bundle with tip
        let mut bundle_builder = BundleBuilder::new(self.jito_config.clone());
        bundle_builder.add_transaction(tx.clone());

        let bundle = bundle_builder.build(
            &self.keypair.pubkey(),
            blockhash,
            self.keypair.as_ref(),
        )?;

        // Submit and wait for confirmation
        let result = jito_client.send_and_confirm_bundle(&bundle, 10).await?;

        let elapsed = start.elapsed();
        
        match result.status {
            tx_builder::BundleStatus::Landed => {
                info!("✅ Jito bundle landed in {:?}: {}", elapsed, result.bundle_id);
                metrics::counter!("jito_landed").increment(1);
            }
            tx_builder::BundleStatus::Accepted => {
                info!("⏳ Bundle accepted but not confirmed: {}", result.bundle_id);
            }
            tx_builder::BundleStatus::Processing => {
                info!("⏳ Bundle processing: {}", result.bundle_id);
            }
            tx_builder::BundleStatus::Failed(reason) => {
                warn!("❌ Bundle failed: {}", reason);
                metrics::counter!("jito_failed").increment(1);
            }
        }

        metrics::histogram!("jito_bundle_ms").record(elapsed.as_millis() as f64);

        Ok(result.bundle_id)
    }

    /// Send with skip preflight (faster but riskier)
    pub async fn send_fast(&self, tx: &Transaction) -> Result<String> {
        use solana_client::rpc_config::RpcSendTransactionConfig;
        use solana_sdk::commitment_config::CommitmentLevel;

        let config = RpcSendTransactionConfig {
            skip_preflight: true,
            preflight_commitment: Some(CommitmentLevel::Processed),
            ..Default::default()
        };

        let sig = self.send_client
            .send_transaction_with_config(tx, config)
            .await
            .map_err(|e| anyhow::anyhow!("Send failed: {}", e))?;

        info!("⚡ Transaction sent (skip preflight): {}", sig);
        
        Ok(sig.to_string())
    }

    /// Execute multiple transactions as Jito bundle (atomic)
    pub async fn execute_bundle(&self, transactions: Vec<Transaction>) -> Result<String> {
        if transactions.is_empty() {
            return Err(anyhow::anyhow!("Empty transaction bundle"));
        }

        if self.dry_run {
            // Simulate each transaction in order
            for (i, tx) in transactions.iter().enumerate() {
                info!("Simulating transaction {} of {}", i + 1, transactions.len());
                self.simulate(tx).await?;
            }
            return Ok("DRY_RUN_BUNDLE_SIMULATION".to_string());
        }

        let jito_client = match &self.jito_client {
            Some(client) => client,
            None => {
                // Fall back to sequential execution
                warn!("Jito not available, executing sequentially");
                let mut last_sig = String::new();
                for tx in &transactions {
                    last_sig = self.send_transaction(tx).await?;
                }
                return Ok(last_sig);
            }
        };

        let blockhash = self.send_client.get_latest_blockhash().await?;

        let mut bundle_builder = BundleBuilder::new(self.jito_config.clone());
        for tx in transactions {
            bundle_builder.add_transaction(tx);
        }

        let bundle = bundle_builder.build(
            &self.keypair.pubkey(),
            blockhash,
            self.keypair.as_ref(),
        )?;

        let result = jito_client.send_and_confirm_bundle(&bundle, 10).await?;
        
        Ok(result.bundle_id)
    }
}
