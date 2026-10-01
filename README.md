# Solana HFT Arbitrage Bot

> A high-frequency trading, arbitrage, and swap bot for the Solana blockchain.

## Overview

A low-latency bot for arbitrage across Solana decentralized exchanges (DEXs), designed to execute trades atomically.

## Architecture

The project is organized into four layers:

1. **Infrastructure:** a private validator RPC endpoint, paid RPC providers such as Helius and Triton, and a US-based compute server.
2. **Services:** a pool poller, RPC proxy, Redis cache, pool deserializer, pricing engine, transaction signer, scheduler, simulator, strategy engine, MEV monitor, and metrics exported to Prometheus and Grafana.
3. **On-chain execution:** an aggregator program that routes swaps through Orca, Raydium CLMM, and Phoenix via cross-program invocations (CPI), with flash-loan and Jito bundle components.
4. **Operations:** HSM-backed key management, devnet backtesting, and alerts.

## Project structure

```text
solana-hft-bot/
├── docs/                    # Documentation
│   ├── ARCHITECTURE.md      # Detailed architecture
│   ├── IMPLEMENTATION_PLAN.md
│   ├── TECH_STACK.md
│   └── API_SPEC.md
├── project-plan/            # Implementation plan
│   ├── 00_PROJECT_OVERVIEW.md
│   ├── 01_PHASE_1_INFRA.md
│   ├── 02_PHASE_2_SERVICES.md
│   ├── 03_PHASE_3_ONCHAIN.md
│   ├── 04_PHASE_4_OPS.md
│   ├── 05_ROADMAP.md
│   └── 06_STARTING_GUIDE.md
├── crates/                  # Rust workspace
│   ├── bot-core/            # Bot core
│   ├── rpc-proxy/           # RPC proxy service
│   ├── pool-deserializer/   # Pool deserializer
│   ├── pricing-engine/      # Pricing engine
│   ├── strategy/            # Routing strategies
│   ├── tx-builder/          # Transaction builder
│   └── aggregator/          # On-chain aggregator (Anchor)
├── config/                  # Configuration
├── scripts/                 # Deployment scripts
└── docker/                  # Docker configuration
```

## Quick start

```bash
# Build the project
cargo build

# Run the tests
cargo test

# Run the bot on devnet
cargo run --bin hft-bot -- --config config/devnet.toml
```

## Technology stack

- **Language:** Rust
- **On-chain:** Anchor Framework
- **Off-chain:** Tokio, Solana SDK, Jito SDK
- **Cache:** Redis
- **Monitoring:** Prometheus and Grafana

## Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [Implementation plan](docs/IMPLEMENTATION_PLAN.md)
- [Technology stack](docs/TECH_STACK.md)

## License

MIT
