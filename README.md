# Solana HFT Arbitrage Bot

Rust workspace for discovering arbitrage opportunities across Solana DEX pools, building swap transactions, and evaluating execution through simulation.

**Status:** experimental implementation. Orca Whirlpool and Raydium CLMM have CPI adapters; end-to-end execution depends on pool accounts and a compatible deployed aggregator. Raydium AMM, Phoenix, and OpenBook execution handlers are placeholders. No production readiness or profitability is claimed.

## Components

| Component | Responsibility |
| --- | --- |
| `arb-bot` | CLI, orchestration, execution, and action reports |
| `bot-core` | Shared types and configuration |
| `pool-poller` | Pool registry, discovery, and state updates |
| `pool-deserializer` | Orca and Raydium pool account parsing |
| `rpc-proxy` | RPC access, retries, and Redis caching |
| `pricing-engine` | Quotes and pool graph |
| `strategy` | Arbitrage route evaluation |
| `tx-builder` | Transaction assembly, simulation, and Jito client |
| `arbitrage-aggregator` | Anchor program with CPI routing and profit checks |

## Getting started

### Requirements

- Rust stable with Cargo, as configured in `rust-toolchain.toml`.
- Redis for the RPC cache.
- Solana CLI for creating a development keypair.
- An RPC endpoint and pool accounts for the selected network.

Build and run the workspace tests:

```bash
git clone https://github.com/ariavitt/solana-hft-arbitrage-bot.git
cd solana-hft-arbitrage-bot
cargo build --workspace --locked
cargo test --workspace --locked
```

Start a local Redis instance, for example with Docker:

```bash
docker run --name solana-arb-redis -d -p 127.0.0.1:6379:6379 redis:7-alpine
```

Create a development keypair at the path used by the devnet configuration:

```bash
mkdir -p config/wallets
solana-keygen new --outfile config/wallets/deployer-devnet.json
```

Review `config/devnet.toml`, especially the RPC endpoint, wallet path, and aggregator program ID. The configured program must exist on that network and match the local instruction layout. Pool discovery and configured pools must also match the network.

Start in simulation mode:

```bash
cargo run --locked --bin arb-bot -- --config config/devnet.toml --dry-run
```

`--dry-run` may fall back to local validation when on-chain simulation fails; a locally validated route does not confirm on-chain execution. Keep `execution.allow_stateful_setup_in_dry_run = false` to avoid sending account-setup transactions during simulation. A keypair is still required in this mode.

List all CLI options:

```bash
cargo run --locked --bin arb-bot -- --help
```

## Configuration

| File | Purpose |
| --- | --- |
| `config/devnet.toml` | Development and simulation on devnet |
| `config/mainnet-simulate.toml` | Mainnet simulation configuration; use with `--dry-run` |
| `config/mainnet.toml` | Mainnet execution configuration |
| `config/mainnet-deploy.toml` | Aggregator deployment configuration |
| `Anchor.toml` | Anchor network and program settings |

Execution settings control slippage, compute budget, Jito use, cooldowns, and trade frequency. Wallet files, local credentials, and generated reports are excluded from version control.

## Monitoring

The bot exposes Prometheus metrics at `http://localhost:9090/metrics` and writes action reports to `reports/`.

To view the included dashboard, run this command from the repository root:

```bash
python3 server.py
```

Open `http://localhost:8080/dashboard.html`. The Python server proxies metrics from the running bot.

## Repository layout

```text
crates/                         Rust services and shared libraries
programs/arbitrage-aggregator/   Anchor program and program tests
config/                         Network configurations
scripts/                        Setup and operational helpers
dashboard.html                  Metrics dashboard
server.py                       Dashboard server and metrics proxy
```

## License

[MIT](LICENSE).
