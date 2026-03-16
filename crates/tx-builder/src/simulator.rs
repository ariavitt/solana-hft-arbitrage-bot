//! Transaction simulator

use bot_core::SimulationResult;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSimulateTransactionConfig;
use solana_sdk::{commitment_config::CommitmentConfig, transaction::Transaction};
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, warn};

#[derive(Error, Debug)]
pub enum SimulationError {
    #[error("Simulation failed: {0}")]
    Failed(String),

    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Transaction error: {0}")]
    TransactionError(String),
}

pub struct Simulator {
    rpc: Arc<RpcClient>,
}

impl Simulator {
    pub fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Simulate a transaction
    pub async fn simulate(&self, tx: &Transaction) -> Result<SimulationResult, SimulationError> {
        let config = RpcSimulateTransactionConfig {
            sig_verify: false,
            replace_recent_blockhash: true,
            commitment: Some(CommitmentConfig::confirmed()),
            ..Default::default()
        };

        let result = self
            .rpc
            .simulate_transaction_with_config(tx, config)
            .await
            .map_err(|e| SimulationError::RpcError(e.to_string()))?;

        if let Some(err) = result.value.err {
            warn!("Simulation failed: {:?}", err);
            return Err(SimulationError::Failed(format!("{:?}", err)));
        }

        let compute_units = result.value.units_consumed.unwrap_or(0);
        let logs = result.value.logs.unwrap_or_default();

        debug!(
            "Simulation successful: {} compute units, {} logs",
            compute_units,
            logs.len()
        );

        Ok(SimulationResult {
            success: true,
            compute_units,
            logs,
        })
    }

    /// Simulate multiple transactions
    pub async fn simulate_batch(
        &self,
        txs: &[Transaction],
    ) -> Vec<Result<SimulationResult, SimulationError>> {
        let mut results = Vec::with_capacity(txs.len());

        for tx in txs {
            results.push(self.simulate(tx).await);
        }

        results
    }
}

