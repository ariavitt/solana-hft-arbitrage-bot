//! Solana HFT Arbitrage Bot
//!
//! Main entry point that orchestrates all components:
//! - Pool Poller: fetches DEX pool data
//! - Pricing Engine: calculates quotes
//! - Strategy: finds arbitrage opportunities
//! - TX Builder: creates transactions
//! - Executor: sends transactions to the network

use anyhow::Result;
use bot_core::config::BotConfig;
use clap::Parser;
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod bot;
mod executor;
mod reporter;

use bot::ArbBot;

/// Solana Arbitrage Bot CLI
#[derive(Parser, Debug)]
#[command(name = "arb-bot")]
#[command(about = "High-frequency arbitrage bot for Solana DEXes")]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "config/devnet.toml")]
    config: PathBuf,

    /// Run in dry-run mode (simulate only, don't send transactions)
    #[arg(long, default_value = "false")]
    dry_run: bool,

    /// Verbose logging
    #[arg(short, long, default_value = "false")]
    verbose: bool,

    /// Initialize aggregator contract (run once)
    #[arg(long, default_value = "false")]
    init_aggregator: bool,

    /// Prepare required token accounts for the first supported route and exit
    #[arg(long, default_value = "false")]
    prepare_accounts: bool,

    /// Auto-discover pools from Orca/Raydium APIs
    #[arg(long, default_value = "false")]
    discover: bool,

    /// Minimum TVL (USD) for pool discovery
    #[arg(long, default_value = "50000")]
    min_tvl: u64,

    /// Minimum 24h volume (USD) for pool discovery  
    #[arg(long, default_value = "10000")]
    min_volume: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Setup logging
    let log_level = if args.verbose { Level::DEBUG } else { Level::INFO };
    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(true)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("🚀 Starting Solana Arbitrage Bot");
    info!("📁 Config: {:?}", args.config);
    info!("🧪 Dry run: {}", args.dry_run);

    // Load configuration
    let config = BotConfig::load(&args.config)?;
    info!("✅ Config loaded: network={}", config.general.network);

    // Setup metrics exporter
    setup_metrics()?;

    // Create and run bot
    let bot = ArbBot::new(
        config,
        args.dry_run,
        args.discover,
        args.min_tvl as f64,
        args.min_volume as f64,
    ).await?;

    if args.init_aggregator {
        info!("📝 Initializing aggregator contract...");
        bot.initialize_aggregator().await?;
        info!("✅ Aggregator initialized!");
        return Ok(());
    }

    if args.prepare_accounts {
        info!("🧰 Preparing execution accounts for the first supported route...");
        bot.prepare_accounts_for_next_route().await?;
        info!("✅ Account preparation finished");
        return Ok(());
    }

    // Run main loop
    info!("🔄 Starting main loop...");
    bot.run().await?;

    Ok(())
}

fn setup_metrics() -> Result<()> {
    // Setup Prometheus metrics exporter on port 9090
    let builder = metrics_exporter_prometheus::PrometheusBuilder::new();
    builder
        .with_http_listener(([0, 0, 0, 0], 9090))
        .install()?;
    
    info!("📊 Metrics available at http://localhost:9090/metrics");
    Ok(())
}
