//! Main bot orchestration

use anyhow::Result;
use bot_core::config::BotConfig;
use bot_core::{Opportunity, ParsedPool};
use pool_poller::{PoolPoller, PoolRegistry, PollerConfig};
use pricing_engine::PricingEngine;
use rpc_proxy::{ProxyConfig, RpcProxy};
use serde_json::json;
use solana_sdk::{
    instruction::AccountMeta,
    instruction::Instruction,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
    signer::Signer,
    system_instruction,
};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_associated_token_account::instruction::create_associated_token_account;
use spl_token::instruction::sync_native;
use std::collections::{HashMap, HashSet, VecDeque};
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use strategy::ArbitrageStrategy;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, error, info, warn};
use tx_builder::dex_accounts::{derive_orca_tick_array, get_tick_array_start_index};
use tx_builder::TxBuilder;

use crate::executor::Executor;
use crate::reporter::ActionReporter;

/// Main arbitrage bot
pub struct ArbBot {
    config: BotConfig,
    aggregator_program_id: Pubkey,
    rpc: Arc<RpcProxy>,
    poller: Arc<PoolPoller>,
    pricing: Arc<PricingEngine>,
    strategy: Arc<ArbitrageStrategy>,
    tx_builder: Arc<TxBuilder>,
    executor: Arc<Executor>,
    reporter: ActionReporter,
    keypair: Arc<Keypair>,
    dry_run: bool,
    recent_executions: Arc<RwLock<HashMap<String, Instant>>>,
    recent_trade_timestamps: Arc<RwLock<VecDeque<Instant>>>,
    
    /// Pool states cache
    pool_states: Arc<RwLock<HashMap<Pubkey, ParsedPool>>>,
}

impl ArbBot {
    pub async fn new(
        config: BotConfig,
        dry_run: bool,
        discover_pools: bool,
        min_tvl: f64,
        min_volume: f64,
    ) -> Result<Self> {
        info!("🔧 Initializing bot components...");

        // Load keypair
        let keypair_path = shellexpand::tilde(&config.wallet.keypair_path).to_string();
        let keypair = Arc::new(read_keypair_file(&keypair_path)
            .map_err(|e| anyhow::anyhow!("Failed to load keypair: {}", e))?);
        info!("🔑 Loaded keypair: {}", keypair.pubkey());

        let reporter = ActionReporter::new("reports/bot-actions.jsonl").await?;
        let aggregator_program_id = Pubkey::from_str(&config.aggregator.program_id)
            .map_err(|e| anyhow::anyhow!("Invalid aggregator program id: {}", e))?;
        reporter.record(
            "bot_initialized",
            json!({
                "network": config.general.network,
                "dry_run": dry_run,
                "keypair": keypair.pubkey().to_string(),
                "keypair_path": keypair_path,
                "aggregator_program_id": aggregator_program_id.to_string(),
                "report_path": reporter.path().display().to_string(),
                "human_report_path": reporter.text_path().display().to_string(),
                "opportunity_cooldown_secs": config.execution.opportunity_cooldown_secs,
                "target_trades_per_minute": config.execution.target_trades_per_minute,
                "trade_amount_lamports": config.strategy.trade_amount_lamports,
                "min_profit_lamports": config.strategy.min_profit_lamports,
                "summary": format!(
                    "network={} | dry_run={} | keypair={} | keypair_path={} | aggregator_program={} | trade_amount={} | min_profit={} | target_tpm={} | stateful_setup_in_dry_run={} | json_report={} | text_report={}",
                    config.general.network,
                    dry_run,
                    keypair.pubkey(),
                    config.wallet.keypair_path,
                    aggregator_program_id,
                    config.strategy.trade_amount_lamports,
                    config.strategy.min_profit_lamports,
                    config.execution.target_trades_per_minute,
                    config.execution.allow_stateful_setup_in_dry_run,
                    reporter.path().display(),
                    reporter.text_path().display(),
                ),
            }),
        ).await?;

        // Initialize RPC Proxy
        let proxy_config = ProxyConfig {
            rpc: config.rpc.clone(),
            redis_url: config.redis.url.clone(),
            cache_ttl_ms: config.redis.cache_ttl_ms,
        };
        let rpc = Arc::new(RpcProxy::new(proxy_config).await?);
        info!("✅ RPC Proxy initialized");
        reporter.record(
            "rpc_proxy_initialized",
            json!({
                "primary_url": config.rpc.primary_url,
                "fallback_count": config.rpc.fallback_urls.len(),
                "cache_ttl_ms": config.redis.cache_ttl_ms,
            }),
        ).await?;

        // Health check
        rpc.health_check().await?;
        info!("✅ RPC connection verified");
        reporter.record(
            "rpc_health_check_passed",
            json!({
                "network": config.general.network,
                "primary_url": config.rpc.primary_url,
            }),
        ).await?;

        // Initialize Pool Registry
        let registry = Arc::new(RwLock::new(PoolRegistry::new()));
        {
            let mut reg = registry.write().await;
            
            // Auto-discover pools from APIs
            if discover_pools {
                info!("🔍 Auto-discovering pools (min TVL=${}, min Vol=${})", min_tvl, min_volume);
                match reg.load_from_discovery(min_tvl, min_volume).await {
                    Ok(count) => info!("✅ Discovered {} pools from APIs", count),
                    Err(e) => warn!("⚠️ Pool discovery failed: {}. Using defaults.", e),
                }
            }
            
            // Always load defaults as fallback
            if reg.len() == 0 {
                reg.load_defaults(&config.general.network);
            }
            
            info!("📋 Total pools: {}", reg.len());
            reporter.record(
                "pools_loaded",
                json!({
                    "network": config.general.network,
                    "pool_count": reg.len(),
                    "pools": reg
                        .get_enabled_pools()
                        .into_iter()
                        .map(|pool| {
                            json!({
                                "name": pool.name,
                                "address": pool.address.to_string(),
                                "pool_type": format!("{:?}", pool.pool_type),
                            })
                        })
                        .collect::<Vec<_>>(),
                }),
            ).await?;
        }

        // Initialize Pool Poller
        let poller_config = PollerConfig {
            poll_interval_ms: 100, // 10 Hz
            batch_size: 100,
            auto_discover: false,
            tracked_tokens: vec![],
            min_liquidity_usd: config.strategy.min_liquidity_usd,
        };
        let poller = Arc::new(PoolPoller::new(
            poller_config,
            rpc.clone(),
            registry.clone(),
        ));
        info!("✅ Pool Poller initialized");

        // Initialize Pricing Engine
        let pricing = Arc::new(PricingEngine::new());
        info!("✅ Pricing Engine initialized");

        // Initialize Strategy
        let strategy = Arc::new(ArbitrageStrategy::new(
            config.strategy.min_profit_bps,
            config.strategy.min_profit_lamports,
            config.strategy.max_slippage_bps,
            config.strategy.max_hops,
            config.strategy.trade_amount_lamports,
        ));
        info!("✅ Strategy initialized");

        // Initialize TX Builder
        let tx_builder = Arc::new(TxBuilder::new(
            keypair.clone(),
            config.execution.clone(),
            aggregator_program_id,
        ));
        info!("✅ TX Builder initialized");

        // Initialize Executor
        let executor = Arc::new(Executor::new(
            rpc.clone(),
            keypair.clone(),
            config.rpc.primary_url.clone(),
            config.execution.use_jito,
            dry_run,
        ));
        info!("✅ Executor initialized");

        Ok(Self {
            config,
            aggregator_program_id,
            rpc,
            poller,
            pricing,
            strategy,
            tx_builder,
            executor,
            reporter,
            keypair,
            dry_run,
            recent_executions: Arc::new(RwLock::new(HashMap::new())),
            recent_trade_timestamps: Arc::new(RwLock::new(VecDeque::new())),
            pool_states: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Initialize the aggregator contract
    pub async fn initialize_aggregator(&self) -> Result<()> {
        use tx_builder::{build_initialize_ix, get_config_pda};
        
        let (config_pda, _) = get_config_pda(&self.aggregator_program_id);
        info!("📝 Config PDA: {}", config_pda);
        self.reporter.record(
            "aggregator_initialization_started",
            json!({
                "config_pda": config_pda.to_string(),
            }),
        ).await?;
        
        // Check if already initialized
        if let Ok(Some(_)) = self.rpc.get_account(&config_pda).await {
            info!("⚠️ Aggregator already initialized");
            self.reporter.record(
                "aggregator_already_initialized",
                json!({
                    "config_pda": config_pda.to_string(),
                }),
            ).await?;
            return Ok(());
        }

        // Build initialize instruction
        let ix = build_initialize_ix(
            &self.aggregator_program_id,
            &self.keypair.pubkey(),
            &self.keypair.pubkey(), // treasury = self for now
            10, // 0.1% fee
        );

        // Get recent blockhash
        let blockhash = self.rpc.get_latest_blockhash().await?;

        // Build and sign transaction
        use solana_sdk::{message::Message, transaction::Transaction};
        let message = Message::new(&[ix], Some(&self.keypair.pubkey()));
        let mut tx = Transaction::new_unsigned(message);
        tx.sign(&[self.keypair.as_ref()], blockhash);

        // Send transaction
        self.executor.send_transaction(&tx).await?;
        
        info!("✅ Aggregator initialized successfully!");
        self.reporter.record(
            "aggregator_initialized",
            json!({
                "config_pda": config_pda.to_string(),
            }),
        ).await?;
        Ok(())
    }

    /// Main bot loop
    pub async fn run(&self) -> Result<()> {
        info!("🚀 Bot starting main loop");
        self.reporter.record(
            "main_loop_started",
            json!({
                "network": self.config.general.network,
                "dry_run": self.dry_run,
                "max_opportunities_per_cycle": self.config.execution.max_opportunities_per_cycle,
                "target_trades_per_minute": self.config.execution.target_trades_per_minute,
                "trade_amount_lamports": self.config.strategy.trade_amount_lamports,
                "min_profit_lamports": self.config.strategy.min_profit_lamports,
                "summary": format!(
                    "network={} | dry_run={} | cycle_limit={} | cooldown={}s | target_tpm={} | trade_amount={} | min_profit={} | stateful_setup_in_dry_run={}",
                    self.config.general.network,
                    self.dry_run,
                    self.config.execution.max_opportunities_per_cycle,
                    self.config.execution.opportunity_cooldown_secs,
                    self.config.execution.target_trades_per_minute,
                    self.config.strategy.trade_amount_lamports,
                    self.config.strategy.min_profit_lamports,
                    self.config.execution.allow_stateful_setup_in_dry_run,
                ),
            }),
        ).await?;
        
        // Start polling in background
        let poller = self.poller.clone();
        let poller_handle = tokio::spawn(async move {
            if let Err(e) = poller.start().await {
                error!("Poller error: {}", e);
            }
        });

        // Main strategy loop
        let mut strategy_interval = interval(Duration::from_millis(50)); // 20 Hz
        let mut last_stats = Instant::now();
        let mut opportunities_found = 0u64;
        let mut trades_executed = 0u64;

        loop {
            strategy_interval.tick().await;

            // Get latest pool states
            let pools = self.poller.get_all_states().await;
            if pools.is_empty() {
                debug!("Waiting for pool data...");
                continue;
            }

            {
                let mut pool_states = self.pool_states.write().await;
                *pool_states = pools.clone();
            }

            // Find arbitrage opportunities
            match self.find_opportunities(&pools).await {
                Ok(opps) => {
                    opportunities_found += opps.len() as u64;
                    let mut executed_this_cycle = 0usize;
                    
                    for opp in opps {
                        if opp.expected_profit > 0 {
                            if !self.should_execute_by_rate_limit().await {
                                self.reporter.record(
                                    "opportunity_skipped_rate_limit",
                                    json!({
                                        "opportunity_id": opp.id.to_string(),
                                        "target_trades_per_minute": self.config.execution.target_trades_per_minute,
                                        "summary": format!(
                                            "{} | RATE_LIMIT={}/min reached",
                                            self.format_opportunity_summary(&opp, "SKIP_RATE_LIMIT").await,
                                            self.config.execution.target_trades_per_minute,
                                        ),
                                    }),
                                ).await?;
                                continue;
                            }

                            if executed_this_cycle >= self.config.execution.max_opportunities_per_cycle {
                                self.reporter.record(
                                    "opportunity_skipped_cycle_limit",
                                    json!({
                                        "opportunity_id": opp.id.to_string(),
                                        "cycle_limit": self.config.execution.max_opportunities_per_cycle,
                                        "profit_bps": opp.profit_bps,
                                    }),
                                ).await?;
                                continue;
                            }

                            let execution_key = self.execution_key(&opp);
                            if !self.should_execute(&execution_key).await {
                                let summary =
                                    self.format_opportunity_summary(&opp, "SKIP_RECENT").await;
                                self.reporter.record(
                                    "opportunity_skipped_recently_executed",
                                    json!({
                                        "opportunity_id": opp.id.to_string(),
                                        "execution_key": execution_key,
                                        "profit_bps": opp.profit_bps,
                                        "summary": summary,
                                    }),
                                ).await?;
                                continue;
                            }

                            let summary =
                                self.format_opportunity_summary(&opp, "FOUND").await;
                            info!(
                                "💰 Found opportunity: profit={} bps, route_len={}",
                                opp.profit_bps, opp.route.legs.len()
                            );
                            self.reporter.record(
                                "opportunity_found",
                                json!({
                                    "opportunity_id": opp.id.to_string(),
                                    "profit_bps": opp.profit_bps,
                                    "expected_profit": opp.expected_profit,
                                    "route_len": opp.route.legs.len(),
                                    "input_amount": opp.input_amount,
                                    "input_token": opp.input_token.to_string(),
                                    "summary": summary.clone(),
                                }),
                            ).await?;

                            // Execute if profitable
                            match self.execute_opportunity(&opp).await {
                                Ok(sig) => {
                                    trades_executed += 1;
                                    executed_this_cycle += 1;
                                    self.mark_trade_executed().await;
                                    info!("✅ Trade executed: {}", sig);
                                    self.reporter.record(
                                        "trade_executed",
                                        json!({
                                            "opportunity_id": opp.id.to_string(),
                                            "signature": sig,
                                            "dry_run": self.dry_run,
                                            "summary": self
                                                .format_opportunity_summary(&opp, "EXECUTED")
                                                .await,
                                        }),
                                    ).await?;
                                }
                                Err(e) => {
                                    warn!("❌ Trade failed: {}", e);
                                    self.reporter.record(
                                        "trade_failed",
                                        json!({
                                            "opportunity_id": opp.id.to_string(),
                                            "error": e.to_string(),
                                            "dry_run": self.dry_run,
                                            "summary": format!(
                                                "{} | ERROR={}",
                                                self.format_opportunity_summary(&opp, "FAILED").await,
                                                e
                                            ),
                                        }),
                                    ).await?;
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    debug!("Strategy error: {}", e);
                }
            }

            // Log stats every 10 seconds
            if last_stats.elapsed() > Duration::from_secs(10) {
                info!(
                    "📊 Stats: pools={}, opportunities={}, trades={}",
                    pools.len(),
                    opportunities_found,
                    trades_executed
                );
                self.reporter.record(
                    "stats_snapshot",
                    json!({
                        "pools": pools.len(),
                        "opportunities_found": opportunities_found,
                        "trades_executed": trades_executed,
                    }),
                ).await?;
                last_stats = Instant::now();
            }
        }
    }

    /// Prepare token accounts for the first supported route and exit.
    pub async fn prepare_accounts_for_next_route(&self) -> Result<()> {
        info!("🧰 Looking for the first route that can be prepared");
        self.reporter.record(
            "account_preparation_mode_started",
            json!({
                "network": self.config.general.network,
                "dry_run": self.dry_run,
                "summary": format!(
                    "PREPARE_MODE | network={} | dry_run={}",
                    self.config.general.network,
                    self.dry_run,
                ),
            }),
        ).await?;

        let poller = self.poller.clone();
        let poller_handle = tokio::spawn(async move {
            if let Err(e) = poller.start().await {
                error!("Poller error: {}", e);
            }
        });

        let mut strategy_interval = interval(Duration::from_millis(100));
        let deadline = Instant::now() + Duration::from_secs(30);

        let result = loop {
            strategy_interval.tick().await;

            if Instant::now() >= deadline {
                break Err(anyhow::anyhow!(
                    "timed out waiting for a supported route to prepare"
                ));
            }

            let pools = self.poller.get_all_states().await;
            if pools.is_empty() {
                continue;
            }

            {
                let mut pool_states = self.pool_states.write().await;
                *pool_states = pools.clone();
            }

            let opportunities = match self.find_opportunities(&pools).await {
                Ok(opps) => opps,
                Err(e) => {
                    debug!("Strategy error during account preparation: {}", e);
                    continue;
                }
            };

            let Some(opp) = opportunities.into_iter().find(|opp| opp.expected_profit > 0) else {
                continue;
            };

            let summary = self.format_opportunity_summary(&opp, "PREPARE_ROUTE").await;
            self.reporter.record(
                "account_preparation_route_selected",
                json!({
                    "opportunity_id": opp.id.to_string(),
                    "summary": summary,
                }),
            ).await?;

            let (_, setup_instructions, _) = match self.resolve_route_accounts(&opp.route).await {
                Ok(resolved) => resolved,
                Err(e) => {
                    self.reporter.record(
                        "account_preparation_route_unsupported",
                        json!({
                            "opportunity_id": opp.id.to_string(),
                            "error": e.to_string(),
                            "summary": format!(
                                "{} | ERROR={}",
                                self.format_opportunity_summary(&opp, "PREPARE_UNSUPPORTED").await,
                                e
                            ),
                        }),
                    ).await?;
                    continue;
                }
            };

            if setup_instructions.is_empty() {
                self.reporter.record(
                    "account_preparation_not_needed",
                    json!({
                        "opportunity_id": opp.id.to_string(),
                        "summary": self
                            .format_opportunity_summary(&opp, "PREPARE_NOT_NEEDED")
                            .await,
                    }),
                ).await?;
                break Ok(());
            }

            break self.execute_setup_instructions(&setup_instructions, true).await;
        };

        poller_handle.abort();

        match &result {
            Ok(()) => {
                self.reporter.record(
                    "account_preparation_mode_finished",
                    json!({
                        "summary": "PREPARE_MODE_DONE",
                    }),
                ).await?;
            }
            Err(error) => {
                self.reporter.record(
                    "account_preparation_mode_failed",
                    json!({
                        "error": error.to_string(),
                        "summary": format!("PREPARE_MODE_FAILED | ERROR={}", error),
                    }),
                ).await?;
            }
        }

        result
    }

    /// Find arbitrage opportunities in current pool states
    async fn find_opportunities(
        &self,
        pools: &HashMap<Pubkey, ParsedPool>,
    ) -> Result<Vec<Opportunity>> {
        // Update pricing engine with latest pool data
        for (address, pool) in pools {
            self.pricing.update_pool(pool.clone());
        }

        // Find routes (simplified - in production use graph search)
        let opportunities = self.strategy.find_opportunities(
            pools,
            &self.pricing,
            self.config.strategy.min_liquidity_usd,
        )?;

        Ok(opportunities)
    }

    /// Execute an arbitrage opportunity
    async fn execute_opportunity(&self, opp: &Opportunity) -> Result<String> {
        let start = Instant::now();
        self.reporter.record(
            "trade_execution_started",
            json!({
                "opportunity_id": opp.id.to_string(),
                "profit_bps": opp.profit_bps,
                "expected_profit": opp.expected_profit,
                "route_len": opp.route.legs.len(),
                "summary": self.format_opportunity_summary(opp, "EXEC_START").await,
            }),
        ).await?;

        if self.dry_run {
            match self.try_onchain_dry_run(opp).await {
                Ok(signature) => return Ok(signature),
                Err(error) => {
                    self.reporter.record(
                        "trade_onchain_simulation_failed",
                        json!({
                            "opportunity_id": opp.id.to_string(),
                            "error": error.to_string(),
                            "summary": format!(
                                "{} | FALLBACK=local_dry_run | ERROR={}",
                                self.format_opportunity_summary(opp, "ONCHAIN_SIM_FAILED").await,
                                error
                            ),
                        }),
                    ).await?;
                }
            }

            let simulated_profit = opp.expected_profit.max(0) as u64;
            let simulated_result = format!(
                "DRY_RUN_LOCAL_OK:profit_lamports={}:route_legs={}",
                simulated_profit,
                opp.route.legs.len()
            );

            self.reporter.record(
                "trade_dry_run_locally_validated",
                json!({
                    "opportunity_id": opp.id.to_string(),
                    "expected_profit": opp.expected_profit,
                    "profit_bps": opp.profit_bps,
                    "route_expected_amount_out": opp.route.expected_amount_out,
                    "route_total_amount_in": opp.route.total_amount_in,
                    "summary": self.format_opportunity_summary(opp, "DRY_RUN_OK").await,
                }),
            ).await?;

            let elapsed = start.elapsed();
            metrics::histogram!("trade_latency_ms").record(elapsed.as_millis() as f64);
            metrics::counter!("trades_total").increment(1);
            self.reporter.record(
                "trade_execution_finished",
                json!({
                    "opportunity_id": opp.id.to_string(),
                    "signature": simulated_result.clone(),
                    "elapsed_ms": elapsed.as_millis(),
                    "mode": "dry_run_local",
                    "summary": format!(
                        "{} | elapsed_ms={} | mode=dry_run_local",
                        self.format_opportunity_summary(opp, "EXEC_FINISH").await,
                        elapsed.as_millis(),
                    ),
                }),
            ).await?;

            return Ok(simulated_result);
        }

        // Calculate minimum profit in absolute terms
        let min_profit = self.tx_builder.calculate_min_profit(
            opp.input_amount,
            self.config.strategy.min_profit_bps,
        );

        // Get recent blockhash
        let blockhash = self.rpc.get_latest_blockhash().await?;
        let (base_token_account, setup_instructions, remaining_accounts) =
            self.resolve_route_accounts(&opp.route).await?;

        if !setup_instructions.is_empty() {
            self.execute_setup_instructions(&setup_instructions, false).await?;
        }

        // Build transaction
        let tx = self.tx_builder.build_arbitrage_tx(
            &opp.route,
            min_profit,
            self.config.strategy.max_slippage_bps,
            &base_token_account,
            blockhash,
            Vec::new(),
            remaining_accounts,
        ).await?;

        // Execute
        let sig = self.executor.execute(&tx).await?;

        let elapsed = start.elapsed();
        metrics::histogram!("trade_latency_ms").record(elapsed.as_millis() as f64);
        metrics::counter!("trades_total").increment(1);
        self.reporter.record(
            "trade_execution_finished",
            json!({
                "opportunity_id": opp.id.to_string(),
                "signature": sig.clone(),
                "elapsed_ms": elapsed.as_millis(),
                "summary": format!(
                    "{} | elapsed_ms={}",
                    self.format_opportunity_summary(opp, "EXEC_FINISH").await,
                    elapsed.as_millis(),
                ),
            }),
        ).await?;

        Ok(sig)
    }

    async fn try_onchain_dry_run(&self, opp: &Opportunity) -> Result<String> {
        let min_profit = self.tx_builder.calculate_min_profit(
            opp.input_amount,
            self.config.strategy.min_profit_bps,
        );
        let blockhash = self.rpc.get_latest_blockhash().await?;
        let (base_token_account, setup_instructions, remaining_accounts) =
            self.resolve_route_accounts(&opp.route).await?;

        if !setup_instructions.is_empty() {
            if !self.config.execution.allow_stateful_setup_in_dry_run {
                return Err(anyhow::anyhow!(
                    "account preparation required before honest on-chain simulation: {} setup instructions pending (set execution.allow_stateful_setup_in_dry_run=true to allow real setup transactions in dry-run)",
                    setup_instructions.len()
                ));
            }

            self.execute_setup_instructions(&setup_instructions, true).await?;
        }

        let tx = self.tx_builder.build_arbitrage_tx(
            &opp.route,
            min_profit,
            self.config.strategy.max_slippage_bps,
            &base_token_account,
            blockhash,
            Vec::new(),
            remaining_accounts,
        ).await?;

        self.reporter.record(
            "trade_onchain_simulation_started",
            json!({
                "opportunity_id": opp.id.to_string(),
                "summary": self.format_opportunity_summary(opp, "ONCHAIN_SIM_START").await,
            }),
        ).await?;

        self.executor.simulate(&tx).await
    }

    async fn resolve_route_accounts(
        &self,
        route: &bot_core::Route,
    ) -> Result<(Pubkey, Vec<Instruction>, Vec<AccountMeta>)> {
        if route.legs.is_empty() {
            return Err(anyhow::anyhow!("route has no legs"));
        }

        if !route
            .legs
            .iter()
            .all(|leg| leg.pool_type == bot_core::PoolType::OrcaWhirlpool)
        {
            return Err(anyhow::anyhow!(
                "on-chain execution path is currently wired only for Orca->Orca routes"
            ));
        }

        let mut setup_instructions = Vec::new();
        let mut prepared_accounts = HashSet::new();
        let base_token_account = self.ensure_token_account(
            &route.legs[0].token_in,
            route.total_amount_in,
            &mut setup_instructions,
            &mut prepared_accounts,
        ).await?;
        let mut remaining_accounts = Vec::new();

        for leg in &route.legs {
            let pool_states = self.pool_states.read().await;
            let pool = pool_states
                .get(&leg.pool)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("missing pool state for {}", leg.pool))?;
            drop(pool_states);

            let raw_pool_account = self
                .rpc
                .get_account(&leg.pool)
                .await?
                .ok_or_else(|| anyhow::anyhow!("missing raw pool account for {}", leg.pool))?;

            let tick_spacing = Self::parse_orca_tick_spacing(&raw_pool_account.data)?;
            let ticks_in_array = tick_spacing as i32 * 88;
            let start_tick = get_tick_array_start_index(pool.tick_current, tick_spacing as i32);
            let oracle = Self::derive_orca_oracle(&leg.pool);

            let tick_starts = if leg.a_to_b {
                [start_tick, start_tick - ticks_in_array, start_tick - (ticks_in_array * 2)]
            } else {
                [start_tick, start_tick + ticks_in_array, start_tick + (ticks_in_array * 2)]
            };

            let user_token_account_a = self
                .ensure_token_account(
                    &pool.token_a.mint,
                    0,
                    &mut setup_instructions,
                    &mut prepared_accounts,
                )
                .await?;
            let user_token_account_b = self
                .ensure_token_account(
                    &pool.token_b.mint,
                    0,
                    &mut setup_instructions,
                    &mut prepared_accounts,
                )
                .await?;

            remaining_accounts.extend(vec![
                AccountMeta::new_readonly(Self::orca_program_id(), false),
                AccountMeta::new(pool.address, false),
                AccountMeta::new(user_token_account_a, false),
                AccountMeta::new(pool.token_a.vault, false),
                AccountMeta::new(user_token_account_b, false),
                AccountMeta::new(pool.token_b.vault, false),
                AccountMeta::new(derive_orca_tick_array(&leg.pool, tick_starts[0]), false),
                AccountMeta::new(derive_orca_tick_array(&leg.pool, tick_starts[1]), false),
                AccountMeta::new(derive_orca_tick_array(&leg.pool, tick_starts[2]), false),
                AccountMeta::new_readonly(oracle, false),
            ]);
        }

        Ok((base_token_account, setup_instructions, remaining_accounts))
    }

    async fn ensure_token_account(
        &self,
        mint: &Pubkey,
        wrap_amount: u64,
        setup_instructions: &mut Vec<Instruction>,
        prepared_accounts: &mut HashSet<Pubkey>,
    ) -> Result<Pubkey> {
        let token_program_id = self.token_program_for_mint(mint).await?;
        let ata =
            Self::derive_associated_token_account(&self.keypair.pubkey(), mint, &token_program_id);
        let account = self.rpc.get_account(&ata).await?;
        let exists = account.is_some();

        if !exists && prepared_accounts.insert(ata) {
            setup_instructions.push(create_associated_token_account(
                &self.keypair.pubkey(),
                &self.keypair.pubkey(),
                mint,
                &token_program_id,
            ));
        }

        if Self::is_wrapped_sol_mint(mint) && wrap_amount > 0 {
            let current_amount = account
                .as_ref()
                .and_then(|acc| Self::parse_token_account_amount(&acc.data).ok())
                .unwrap_or(0);

            if current_amount < wrap_amount {
                setup_instructions.push(system_instruction::transfer(
                    &self.keypair.pubkey(),
                    &ata,
                    wrap_amount - current_amount,
                ));
                setup_instructions.push(sync_native(
                    &token_program_id,
                    &ata,
                )?);
            }
        }

        Ok(ata)
    }

    async fn execute_setup_instructions(
        &self,
        setup_instructions: &[Instruction],
        force_send: bool,
    ) -> Result<()> {
        if setup_instructions.is_empty() {
            return Ok(());
        }

        let payer_balance = self.rpc.get_balance(&self.keypair.pubkey()).await?;
        let estimated_required_lamports =
            Self::estimate_setup_lamports(setup_instructions) + 50_000;

        if payer_balance < estimated_required_lamports {
            return Err(anyhow::anyhow!(
                "insufficient SOL for setup: payer_balance={} lamports, estimated_required={} lamports",
                payer_balance,
                estimated_required_lamports
            ));
        }

        let blockhash = self.rpc.get_latest_blockhash().await?;
        let tx = self
            .tx_builder
            .build_signed_tx(setup_instructions.to_vec(), blockhash)?;

        self.reporter.record(
            "account_setup_started",
            json!({
                "instruction_count": setup_instructions.len(),
                "force_send": force_send,
                "summary": format!(
                    "SETUP | instructions={} | force_send={}",
                    setup_instructions.len(),
                    force_send
                ),
            }),
        ).await?;

        let signature = if force_send {
            self.executor.send_transaction(&tx).await?
        } else {
            self.executor.execute(&tx).await?
        };

        self.reporter.record(
            "account_setup_finished",
            json!({
                "instruction_count": setup_instructions.len(),
                "signature": signature,
                "force_send": force_send,
                "summary": format!(
                    "SETUP_DONE | instructions={} | force_send={}",
                    setup_instructions.len(),
                    force_send
                ),
            }),
        ).await?;

        Ok(())
    }

    async fn token_program_for_mint(&self, mint: &Pubkey) -> Result<Pubkey> {
        let mint_account = self
            .rpc
            .get_account(mint)
            .await?
            .ok_or_else(|| anyhow::anyhow!("missing mint account for {}", mint))?;

        let owner = mint_account.owner;
        if owner == spl_token::id() || owner == spl_token_2022::id() {
            Ok(owner)
        } else {
            Err(anyhow::anyhow!(
                "unsupported token program {} for mint {}",
                owner,
                mint
            ))
        }
    }

    fn derive_associated_token_account(
        owner: &Pubkey,
        mint: &Pubkey,
        token_program_id: &Pubkey,
    ) -> Pubkey {
        get_associated_token_address_with_program_id(owner, mint, token_program_id)
    }

    fn token_program_id() -> Pubkey {
        Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap()
    }

    fn is_wrapped_sol_mint(mint: &Pubkey) -> bool {
        mint.to_string() == "So11111111111111111111111111111111111111112"
    }

    fn derive_orca_oracle(whirlpool: &Pubkey) -> Pubkey {
        let whirlpool_program = Self::orca_program_id();
        Pubkey::find_program_address(&[b"oracle", whirlpool.as_ref()], &whirlpool_program).0
    }

    fn orca_program_id() -> Pubkey {
        Pubkey::from_str("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc").unwrap()
    }

    fn parse_orca_tick_spacing(data: &[u8]) -> Result<u16> {
        if data.len() < 43 {
            return Err(anyhow::anyhow!("invalid Orca pool account length: {}", data.len()));
        }

        let offset = 8 + 32 + 1;
        Ok(u16::from_le_bytes([data[offset], data[offset + 1]]))
    }

    fn parse_token_account_amount(data: &[u8]) -> Result<u64> {
        let account = spl_token::state::Account::unpack(data)
            .map_err(|e| anyhow::anyhow!("invalid token account data: {}", e))?;
        Ok(account.amount)
    }

    fn estimate_setup_lamports(setup_instructions: &[Instruction]) -> u64 {
        let ata_program =
            Pubkey::from_str("ATokenGPvbdGVxr1h7eSLv1n7nNg7Yt7X6qUu2y4fS6u").unwrap();
        let token_account_rent =
            solana_sdk::rent::Rent::default().minimum_balance(spl_token::state::Account::LEN);

        setup_instructions
            .iter()
            .map(|instruction| {
                if instruction.program_id == ata_program {
                    token_account_rent
                } else if instruction.program_id == solana_sdk::system_program::id() {
                    if instruction.data.len() >= 12 {
                        let mut discriminator = [0u8; 4];
                        discriminator.copy_from_slice(&instruction.data[..4]);
                        if u32::from_le_bytes(discriminator) == 2 {
                            let mut lamports = [0u8; 8];
                            lamports.copy_from_slice(&instruction.data[4..12]);
                            return u64::from_le_bytes(lamports);
                        }
                    }
                    0
                } else {
                    0
                }
            })
            .sum()
    }

    fn execution_key(&self, opp: &Opportunity) -> String {
        let route_key = opp
            .route
            .legs
            .iter()
            .map(|leg| {
                format!(
                    "{}:{}:{}:{}",
                    leg.pool,
                    leg.token_in,
                    leg.token_out,
                    leg.a_to_b
                )
            })
            .collect::<Vec<_>>()
            .join("|");

        format!("{}:{}", opp.input_token, route_key)
    }

    async fn should_execute(&self, execution_key: &str) -> bool {
        let now = Instant::now();
        let execution_cooldown =
            Duration::from_secs(self.config.execution.opportunity_cooldown_secs);
        let mut recent = self.recent_executions.write().await;
        recent.retain(|_, seen_at| now.duration_since(*seen_at) < execution_cooldown);

        if recent.contains_key(execution_key) {
            return false;
        }

        recent.insert(execution_key.to_string(), now);
        true
    }

    async fn should_execute_by_rate_limit(&self) -> bool {
        let target = self.config.execution.target_trades_per_minute;
        if target == 0 {
            return true;
        }

        let now = Instant::now();
        let window = Duration::from_secs(60);
        let mut recent = self.recent_trade_timestamps.write().await;

        while let Some(front) = recent.front() {
            if now.duration_since(*front) >= window {
                recent.pop_front();
            } else {
                break;
            }
        }

        recent.len() < target as usize
    }

    async fn mark_trade_executed(&self) {
        let now = Instant::now();
        let window = Duration::from_secs(60);
        let mut recent = self.recent_trade_timestamps.write().await;
        recent.push_back(now);

        while let Some(front) = recent.front() {
            if now.duration_since(*front) >= window {
                recent.pop_front();
            } else {
                break;
            }
        }
    }

    async fn format_opportunity_summary(&self, opp: &Opportunity, status: &str) -> String {
        let pool_states = self.pool_states.read().await;
        let mut total_fee_bps = 0u64;
        let mut total_fee_amount = 0u64;
        let mut route_parts = Vec::new();

        for leg in &opp.route.legs {
            let fee_bps = pool_states
                .get(&leg.pool)
                .map(|pool| pool.fee_rate_bps as u64)
                .unwrap_or(0);
            let fee_amount = leg.amount_in.saturating_mul(fee_bps) / 10_000;
            total_fee_bps += fee_bps;
            total_fee_amount = total_fee_amount.saturating_add(fee_amount);

            route_parts.push(format!(
                "{} {}->{} fee={}bps in={} out={} pool={}",
                Self::pool_type_label(leg.pool_type),
                Self::short_token(&leg.token_in),
                Self::short_token(&leg.token_out),
                fee_bps,
                leg.amount_in,
                leg.expected_out,
                Self::short_pubkey(&leg.pool),
            ));
        }

        format!(
            "STATUS={} | TOKEN={} | SIZE={} | PROFIT={} | FEES~={} | ROUTE={}",
            status,
            Self::short_token(&opp.input_token),
            Self::format_amount_for_token(&opp.input_token, opp.input_amount),
            Self::format_bps(opp.profit_bps as i64),
            Self::format_bps(total_fee_bps as i64),
            route_parts.join(" => "),
        )
    }

    fn format_bps(bps: i64) -> String {
        format!("{:.2}%", bps as f64 / 100.0)
    }

    fn format_amount_for_token(token: &Pubkey, raw_amount: u64) -> String {
        match token.to_string().as_str() {
            "So11111111111111111111111111111111111111112" => {
                format!("{:.6} SOL", raw_amount as f64 / 1_000_000_000.0)
            }
            _ => raw_amount.to_string(),
        }
    }

    fn short_token(pubkey: &Pubkey) -> String {
        match pubkey.to_string().as_str() {
            "So11111111111111111111111111111111111111112" => "SOL".to_string(),
            value => Self::short_string(value),
        }
    }

    fn short_pubkey(pubkey: &Pubkey) -> String {
        Self::short_string(&pubkey.to_string())
    }

    fn short_string(value: &str) -> String {
        if value.len() <= 12 {
            return value.to_string();
        }

        format!("{}...{}", &value[..4], &value[value.len() - 4..])
    }

    fn pool_type_label(pool_type: bot_core::PoolType) -> &'static str {
        match pool_type {
            bot_core::PoolType::OrcaWhirlpool => "Orca",
            bot_core::PoolType::RaydiumClmm => "RaydiumCLMM",
            bot_core::PoolType::RaydiumAmm => "RaydiumAMM",
            bot_core::PoolType::Phoenix => "Phoenix",
            bot_core::PoolType::OpenBook => "OpenBook",
        }
    }
}
