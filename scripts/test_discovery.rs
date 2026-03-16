//! Тест discovery пулов
//! cargo run --example test_discovery

use pool_poller::PoolDiscovery;

#[tokio::main]
async fn main() {
    let discovery = PoolDiscovery::new()
        .with_min_tvl(10_000.0)   // $10k TVL minimum
        .with_min_volume(1_000.0); // $1k daily volume

    println!("🔍 Discovering pools...\n");

    // Orca
    match discovery.discover_orca().await {
        Ok(pools) => {
            println!("✅ Orca Whirlpools: {} pools", pools.len());
            for pool in pools.iter().take(5) {
                println!("   - {}", pool.name);
            }
        }
        Err(e) => println!("❌ Orca error: {}", e),
    }

    // Raydium
    match discovery.discover_raydium().await {
        Ok(pools) => {
            println!("\n✅ Raydium pools: {} pools", pools.len());
            for pool in pools.iter().take(5) {
                println!("   - {}", pool.name);
            }
        }
        Err(e) => println!("❌ Raydium error: {}", e),
    }
}



