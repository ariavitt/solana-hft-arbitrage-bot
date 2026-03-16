//! Transaction builder implementation
//!
//! Builds arbitrage transactions using our on-chain aggregator

use crate::aggregator::{build_execute_arbitrage_ix, route_to_swap_legs};
use bot_core::{config::ExecutionConfig, Route};
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    instruction::AccountMeta,
    message::Message,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
};
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, info};

#[derive(Error, Debug)]
pub enum BuildError {
    #[error("Invalid route: {0}")]
    InvalidRoute(String),

    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Signing error: {0}")]
    SigningError(String),

    #[error("Route not closed: first token != last token")]
    RouteNotClosed,
}

pub struct TxBuilder {
    payer: Arc<Keypair>,
    config: ExecutionConfig,
}

impl TxBuilder {
    pub fn new(payer: Arc<Keypair>, config: ExecutionConfig) -> Self {
        Self { payer, config }
    }

    /// Build an arbitrage transaction using the aggregator contract
    ///
    /// # Parameters
    /// - `route` - The arbitrage route (must be closed: start token == end token)
    /// - `min_profit` - Minimum profit in lamports/base token units
    /// - `max_slippage_bps` - Maximum slippage in basis points (100 = 1%)
    /// - `base_token_account` - Token account for the base token
    /// - `recent_blockhash` - Recent blockhash for the transaction
    /// - `remaining_accounts` - DEX accounts needed for the swaps
    pub async fn build_arbitrage_tx(
        &self,
        route: &Route,
        min_profit: u64,
        max_slippage_bps: u16,
        base_token_account: &Pubkey,
        recent_blockhash: solana_sdk::hash::Hash,
        remaining_accounts: Vec<AccountMeta>,
    ) -> Result<Transaction, BuildError> {
        // Validate route
        if route.legs.is_empty() {
            return Err(BuildError::InvalidRoute("Empty route".to_string()));
        }
        if route.legs.len() < 2 {
            return Err(BuildError::InvalidRoute(
                "Route must have at least 2 legs".to_string(),
            ));
        }
        if route.legs.len() > 4 {
            return Err(BuildError::InvalidRoute(
                "Route too long (max 4 hops)".to_string(),
            ));
        }

        // Verify route is closed
        let first_token = route.legs.first().unwrap().token_in;
        let last_token = route.legs.last().unwrap().token_out;
        if first_token != last_token {
            return Err(BuildError::RouteNotClosed);
        }

        let mut instructions = Vec::new();

        // 1. Add compute budget instructions
        instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(
            self.config.compute_unit_limit,
        ));
        instructions.push(ComputeBudgetInstruction::set_compute_unit_price(
            self.config.compute_unit_price,
        ));

        // 2. Convert route to swap legs
        let swap_legs = route_to_swap_legs(route);

        // 3. Build aggregator execute_arbitrage instruction
        let execute_ix = build_execute_arbitrage_ix(
            &self.payer.pubkey(),
            base_token_account,
            swap_legs,
            min_profit,
            max_slippage_bps,
            remaining_accounts,
        );
        instructions.push(execute_ix);

        info!(
            "Built arbitrage TX: {} legs, min_profit={}, slippage={}bps",
            route.legs.len(),
            min_profit,
            max_slippage_bps
        );

        debug!(
            "Transaction has {} instructions",
            instructions.len()
        );

        // 4. Create and sign transaction
        let message = Message::new(&instructions, Some(&self.payer.pubkey()));
        let mut tx = Transaction::new_unsigned(message);
        tx.sign(&[self.payer.as_ref()], recent_blockhash);

        Ok(tx)
    }

    /// Build a simple swap transaction (legacy, for testing)
    pub async fn build_swap_tx(
        &self,
        route: &Route,
        _min_out: u64,
        recent_blockhash: solana_sdk::hash::Hash,
    ) -> Result<Transaction, BuildError> {
        if route.legs.is_empty() {
            return Err(BuildError::InvalidRoute("Empty route".to_string()));
        }

        let mut instructions = Vec::new();

        // Add compute budget instructions
        instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(
            self.config.compute_unit_limit,
        ));
        instructions.push(ComputeBudgetInstruction::set_compute_unit_price(
            self.config.compute_unit_price,
        ));

        debug!(
            "Built legacy swap transaction with {} instructions",
            instructions.len()
        );

        let message = Message::new(&instructions, Some(&self.payer.pubkey()));
        let mut tx = Transaction::new_unsigned(message);
        tx.sign(&[self.payer.as_ref()], recent_blockhash);

        Ok(tx)
    }

    /// Calculate minimum profit based on basis points
    pub fn calculate_min_profit(&self, amount_in: u64, min_profit_bps: u16) -> u64 {
        amount_in
            .checked_mul(min_profit_bps as u64)
            .unwrap_or(0)
            .checked_div(10000)
            .unwrap_or(0)
    }

    /// Get payer pubkey
    pub fn payer(&self) -> Pubkey {
        self.payer.pubkey()
    }
}

