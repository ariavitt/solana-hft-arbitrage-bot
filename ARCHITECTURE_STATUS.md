# Architecture Status

## Purpose

This document is the Step 1 baseline: a short, honest map of what the project is today versus the intended architecture.

Target architecture:
- low-latency Solana micro-arbitrage bot
- frequent small-profit swaps
- fast pool polling and quote calculation
- execution through a custom on-chain aggregator contract
- atomic multi-hop swaps in a single transaction
- optimized for trades-per-minute, not only profit-per-trade

## Status Summary

### Real

These parts are present in code and are clearly functional in the current repo shape.

| Area | Status | Evidence |
| --- | --- | --- |
| Pool polling and deserialization pipeline | Real | `pool-poller`, `pool-deserializer`, and `pricing-engine` are wired into the bot main loop in [crates/arb-bot/src/bot.rs](/Users/polinababijcuk/Documents/New%20project/crates/arb-bot/src/bot.rs). |
| Arbitrage route discovery | Real | Strategy computes 2-hop opportunities in [crates/strategy/src/arbitrage.rs](/Users/polinababijcuk/Documents/New%20project/crates/strategy/src/arbitrage.rs). |
| RPC proxy abstraction | Real | Proxy with fallback handling exists in [crates/rpc-proxy/src/proxy.rs](/Users/polinababijcuk/Documents/New%20project/crates/rpc-proxy/src/proxy.rs). |
| Optional Redis cache layer | Real | Cache layer exists and degrades gracefully if Redis is unavailable in [crates/rpc-proxy/src/cache.rs](/Users/polinababijcuk/Documents/New%20project/crates/rpc-proxy/src/cache.rs). |
| Config-driven bot behavior | Real | Strategy and execution knobs are configurable in [crates/bot-core/src/config.rs](/Users/polinababijcuk/Documents/New%20project/crates/bot-core/src/config.rs) and `config/*.toml`. |
| Human-readable and JSON action logs | Real | Reporter writes both `.jsonl` and `.log` in [crates/arb-bot/src/reporter.rs](/Users/polinababijcuk/Documents/New%20project/crates/arb-bot/src/reporter.rs). |
| Micro-arb oriented rate control | Real | Current bot has `target_trades_per_minute`, cooldown, and per-cycle limits in [crates/arb-bot/src/bot.rs](/Users/polinababijcuk/Documents/New%20project/crates/arb-bot/src/bot.rs). |
| On-chain aggregator program source code | Real | Anchor program exists in [programs/arbitrage-aggregator/src/lib.rs](/Users/polinababijcuk/Documents/New%20project/programs/arbitrage-aggregator/src/lib.rs). |

### Partial

These parts exist and are directionally aligned with the intended architecture, but they are not proven production-ready end-to-end.

| Area | Status | Why partial |
| --- | --- | --- |
| Fast quote-to-execution path | Partial | The architecture supports it, but current repo state does not yet prove the full end-to-end latency target from live quote to real execution. |
| Tx builder for aggregator execution | Partial | [crates/tx-builder/src/builder.rs](/Users/polinababijcuk/Documents/New%20project/crates/tx-builder/src/builder.rs) builds aggregator transactions, but the bot currently passes `vec![]` for `remaining_accounts`, which means the DEX account plumbing is not yet fully connected. |
| DEX CPI account builders | Partial | [crates/tx-builder/src/dex_accounts.rs](/Users/polinababijcuk/Documents/New%20project/crates/tx-builder/src/dex_accounts.rs) has builders for Orca and Raydium, but it still contains placeholders and hardcoded/example values. |
| Aggregator execute instruction | Partial | [programs/arbitrage-aggregator/src/instructions/execute.rs](/Users/polinababijcuk/Documents/New%20project/programs/arbitrage-aggregator/src/instructions/execute.rs) has the intended shape, but only Orca and Raydium CLMM CPI paths are wired, and other DEX types are still TODO or skeletons. |
| Dry-run simulation mode | Partial | There is an RPC-based simulation path in [crates/arb-bot/src/executor.rs](/Users/polinababijcuk/Documents/New%20project/crates/arb-bot/src/executor.rs), but current bot success in practice is still dominated by local dry-run validation rather than confirmed on-chain program execution. |
| Jito support | Partial | Jito client and bundle sender exist in [crates/arb-bot/src/executor.rs](/Users/polinababijcuk/Documents/New%20project/crates/arb-bot/src/executor.rs) and [crates/tx-builder/src/jito.rs](/Users/polinababijcuk/Documents/New%20project/crates/tx-builder/src/jito.rs), but this path is not yet the validated default execution mode. |
| Throughput-oriented strategy | Partial | The code now has throughput controls, but route ranking is still primarily profit-sorted in [crates/strategy/src/arbitrage.rs](/Users/polinababijcuk/Documents/New%20project/crates/strategy/src/arbitrage.rs), not yet explicitly optimized for repeatable low-latency fills per minute. |

### Stub / Gap

These parts are the biggest reasons the current implementation does not yet fully match the intended production architecture.

| Area | Status | Evidence |
| --- | --- | --- |
| Real end-to-end on-chain execution is not yet the default proven path | Gap | The bot still has local dry-run success logic in [crates/arb-bot/src/bot.rs](/Users/polinababijcuk/Documents/New%20project/crates/arb-bot/src/bot.rs), which means success can be recorded without a full on-chain aggregator execution. |
| Base token account handling is placeholder-level | Gap | [crates/arb-bot/src/bot.rs](/Users/polinababijcuk/Documents/New%20project/crates/arb-bot/src/bot.rs) still returns the signer pubkey from `get_base_token_account()` instead of a real token account. |
| `remaining_accounts` are not assembled at execution time | Gap | Bot calls tx builder with `vec![]` remaining accounts in [crates/arb-bot/src/bot.rs](/Users/polinababijcuk/Documents/New%20project/crates/arb-bot/src/bot.rs). |
| Several on-chain DEX CPI implementations are not complete | Gap | `RaydiumAmm`, `Phoenix`, and `OpenBook` CPI branches in [programs/arbitrage-aggregator/src/instructions/execute.rs](/Users/polinababijcuk/Documents/New%20project/programs/arbitrage-aggregator/src/instructions/execute.rs) are still TODO/skeleton code. |
| Single-swap test instruction is mostly a shell | Gap | [programs/arbitrage-aggregator/src/instructions/swap.rs](/Users/polinababijcuk/Documents/New%20project/programs/arbitrage-aggregator/src/instructions/swap.rs) logs intent but does not yet perform real DEX CPI logic. |
| Some DEX account definitions are placeholders | Gap | Example: `observation_state` and `tick_array_bitmap` in [crates/tx-builder/src/dex_accounts.rs](/Users/polinababijcuk/Documents/New%20project/crates/tx-builder/src/dex_accounts.rs) include placeholder addresses. |
| Production proof for private RPC / validator path is absent from repo state | Gap | The code supports custom RPCs, but there is no repo-level proof yet of validated deployment, latency benchmarks, or real server topology. |
| Human log fee estimates are still approximate | Gap | Logs are now readable, but fee output is derived heuristically from route data in [crates/arb-bot/src/bot.rs](/Users/polinababijcuk/Documents/New%20project/crates/arb-bot/src/bot.rs), not yet from a fully verified fee accounting model. |

## Current Reality vs Intended Reality

### Intended reality

- bot reads pool state quickly
- bot computes quotes and routes quickly
- bot sends one transaction to the custom aggregator
- aggregator performs multi-hop swaps atomically via CPI
- small but valid opportunities are executed frequently
- overall edge depends on speed, reliability, and throughput

### Current reality

- bot reads pool state and computes opportunities successfully
- bot can be tuned for micro-arb style frequency
- bot has logging, throttling, and dry-run instrumentation
- custom aggregator program source exists and matches the intended model
- however, the production execution path is still not fully connected and proven

## Main blockers to 100% alignment

1. Aggregator path is not yet the fully validated default execution path.
2. Real token accounts and `remaining_accounts` are not assembled end-to-end during execution.
3. Several DEX CPI paths are incomplete or placeholder-level.
4. Current success metrics can still be produced through local dry-run validation.
5. There is no hard proof yet in the repo of the intended latency target on live execution.

## Recommended next step

Proceed to Step 2:

- verify on-chain aggregator state and deployment
- remove ambiguity around config PDA and required accounts
- make one honest end-to-end on-chain simulation path succeed without local dry-run shortcuts
