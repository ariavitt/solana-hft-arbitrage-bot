//! Main bot orchestration

use anyhow::Result;
use bot_core::config::BotConfig;
use bot_core::{Opportunity, ParsedPool};
use pool_poller::{PoolDiscovery, PoolPoller, PoolRegistry, PollerConfig};
use pricing_engine::PricingEngine;
use rpc_proxy::{ProxyConfig, RpcProxy};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
    signer::Signer,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use strategy::ArbitrageStrategy;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, error, info, warn};
use tx_builder::TxBuilder;

use crate::executor::Executor;

/// Main arbitrage bot
pub struct ArbBot {
    config: BotConfig,
    rpc: Arc<RpcProxy>,
    poller: Arc<PoolPoller>,
    pricing: Arc<PricingEngine>,
    strategy: Arc<ArbitrageStrategy>,
    tx_builder: Arc<TxBuilder>,
    executor: Arc<Executor>,
    keypair: Arc<Keypair>,
    dry_run: bool,
    
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
        let keypair_path = shellexpand::tilde("~/.config/solana/id.json").to_string();
        let keypair = Arc::new(read_keypair_file(&keypair_path)
            .map_err(|e| anyhow::anyhow!("Failed to load keypair: {}", e))?);
        info!("🔑 Loaded keypair: {}", keypair.pubkey());

        // Initialize RPC Proxy
        let proxy_config = ProxyConfig {
            rpc: config.rpc.clone(),
            redis_url: config.redis.url.clone(),
            cache_ttl_ms: config.redis.cache_ttl_ms,
        };
        let rpc = Arc::new(RpcProxy::new(proxy_config).await?);
        info!("✅ RPC Proxy initialized");

        // Health check
        rpc.health_check().await?;
        info!("✅ RPC connection verified");

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
            config.strategy.max_slippage_bps,
            config.strategy.max_hops,
        ));
        info!("✅ Strategy initialized");

        // Initialize TX Builder
        let tx_builder = Arc::new(TxBuilder::new(
            keypair.clone(),
            config.execution.clone(),
        ));
        info!("✅ TX Builder initialized");

        // Initialize Executor
        let executor = Arc::new(Executor::new(
            rpc.clone(),
            keypair.clone(),
            config.execution.use_jito,
            dry_run,
        ));
        info!("✅ Executor initialized");

        Ok(Self {
            config,
            rpc,
            poller,
            pricing,
            strategy,
            tx_builder,
            executor,
            keypair,
            dry_run,
            pool_states: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Initialize the aggregator contract
    pub async fn initialize_aggregator(&self) -> Result<()> {
        use tx_builder::{build_initialize_ix, get_config_pda};
        
        let (config_pda, _) = get_config_pda();
        info!("📝 Config PDA: {}", config_pda);
        
        // Check if already initialized
        if let Ok(Some(_)) = self.rpc.get_account(&config_pda).await {
            info!("⚠️ Aggregator already initialized");
            return Ok(());
        }

        // Build initialize instruction
        let ix = build_initialize_ix(
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
        Ok(())
    }

    /// Main bot loop
    pub async fn run(&self) -> Result<()> {
        info!("🚀 Bot starting main loop");
        
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

            // Find arbitrage opportunities
            match self.find_opportunities(&pools).await {
                Ok(opps) => {
                    opportunities_found += opps.len() as u64;
                    
                    for opp in opps {
                        if opp.expected_profit > 0 {
                            info!(
                                "💰 Found opportunity: profit={} bps, route_len={}",
                                opp.profit_bps, opp.route.legs.len()
                            );

                            // Execute if profitable
                            match self.execute_opportunity(&opp).await {
                                Ok(sig) => {
                                    trades_executed += 1;
                                    info!("✅ Trade executed: {}", sig);
                                }
                                Err(e) => {
                                    warn!("❌ Trade failed: {}", e);
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
                last_stats = Instant::now();
            }
        }
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

        // Calculate minimum profit in absolute terms
        let min_profit = self.tx_builder.calculate_min_profit(
            opp.input_amount,
            self.config.strategy.min_profit_bps,
        );

        // Get recent blockhash
        let blockhash = self.rpc.get_latest_blockhash().await?;

        // Build transaction
        let tx = self.tx_builder.build_arbitrage_tx(
            &opp.route,
            min_profit,
            self.config.strategy.max_slippage_bps,
            &self.get_base_token_account()?,
            blockhash,
            vec![], // remaining accounts - need to build for each DEX
        ).await?;

        // Execute
        let sig = self.executor.execute(&tx).await?;

        let elapsed = start.elapsed();
        metrics::histogram!("trade_latency_ms").record(elapsed.as_millis() as f64);
        metrics::counter!("trades_total").increment(1);

        Ok(sig)
    }

    /// Get base token account (e.g., WSOL)
    fn get_base_token_account(&self) -> Result<Pubkey> {
        // In production, derive the associated token account
        // For now, return a placeholder
        Ok(self.keypair.pubkey())
    }
}

