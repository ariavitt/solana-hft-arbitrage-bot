# On-Chain Execution Status

## Goal of Step 2

Confirm whether the project can execute a real aggregator-based arbitrage path on-chain, and identify the exact blockers if it cannot.

## Current Verdict

The intended on-chain path is present in design, but it is **not yet fully wired and production-valid end-to-end**.

## What the intended path should be

1. Bot finds a profitable route.
2. Bot builds one transaction for the custom aggregator program.
3. Aggregator program receives a closed route.
4. Aggregator performs each swap leg through CPI.
5. Aggregator checks balance before/after.
6. Aggregator reverts if profit is below threshold.

## What exists today

### 1. Aggregator program source exists

Confirmed in:
- [programs/arbitrage-aggregator/src/lib.rs](/Users/polinababijcuk/Documents/New%20project/programs/arbitrage-aggregator/src/lib.rs)
- [programs/arbitrage-aggregator/src/instructions/execute.rs](/Users/polinababijcuk/Documents/New%20project/programs/arbitrage-aggregator/src/instructions/execute.rs)

This is real code, not a stub at the top level.

### 2. Tx builder knows how to target the aggregator

Confirmed in:
- [crates/tx-builder/src/builder.rs](/Users/polinababijcuk/Documents/New%20project/crates/tx-builder/src/builder.rs)
- [crates/tx-builder/src/aggregator.rs](/Users/polinababijcuk/Documents/New%20project/crates/tx-builder/src/aggregator.rs)

The bot can build an `execute_arbitrage` instruction in the intended format.

### 3. Program config PDA model exists

Confirmed in:
- [programs/arbitrage-aggregator/src/state/config.rs](/Users/polinababijcuk/Documents/New%20project/programs/arbitrage-aggregator/src/state/config.rs)
- [programs/arbitrage-aggregator/src/instructions/admin.rs](/Users/polinababijcuk/Documents/New%20project/programs/arbitrage-aggregator/src/instructions/admin.rs)
- [crates/tx-builder/src/aggregator.rs](/Users/polinababijcuk/Documents/New%20project/crates/tx-builder/src/aggregator.rs)

The program has a clear config account model and initialization flow.

## Exact blockers found

### Blocker 1. Base token account resolution is no longer a placeholder, but execution accounts are still not pre-provisioned

Evidence:
- [crates/arb-bot/src/bot.rs](/Users/polinababijcuk/Documents/New%20project/crates/arb-bot/src/bot.rs)

Problem:
- the bot now derives real ATA addresses instead of passing the signer pubkey
- but honest on-chain simulation still requires those accounts to already exist and, for `WSOL`, to already be funded
- latest run now fails with:
  - `account preparation required before honest on-chain simulation: 7 setup instructions pending`

Impact:
- the hot path is now smaller and cleaner, but honest simulation still cannot proceed until required token accounts are prepared outside the arbitrage transaction

### Blocker 2. `remaining_accounts` are now partially wired for `Orca -> Orca`, but only for a narrow vertical slice

Evidence:
- [crates/arb-bot/src/bot.rs](/Users/polinababijcuk/Documents/New%20project/crates/arb-bot/src/bot.rs)
- [crates/tx-builder/src/builder.rs](/Users/polinababijcuk/Documents/New%20project/crates/tx-builder/src/builder.rs)

Problem:
- the bot now derives `remaining_accounts` for `Orca -> Orca`
- this includes pool, user token accounts, vaults, tick arrays, and oracle
- but the path is still intentionally narrow and does not yet generalize to the broader route universe

Impact:
- one vertical slice is becoming real, but the system is still far from full route coverage

### Blocker 3. DEX account builders are not fully production-ready

Evidence:
- [crates/tx-builder/src/dex_accounts.rs](/Users/polinababijcuk/Documents/New%20project/crates/tx-builder/src/dex_accounts.rs)

Problem:
- Raydium CLMM account data still contains placeholder/example values such as:
  - `observation_state = 11111111111111111111111111111111`
  - `tick_array_bitmap = 11111111111111111111111111111111`
- Orca and Raydium builders exist, but they are not yet dynamically derived from live route state

Impact:
- account metas may be structurally correct but still invalid for live CPI execution

### Blocker 4. Account preparation inside the arbitrage transaction made the transaction too large, so setup was split out

Evidence:
- [crates/arb-bot/src/bot.rs](/Users/polinababijcuk/Documents/New%20project/crates/arb-bot/src/bot.rs)
- [crates/tx-builder/src/builder.rs](/Users/polinababijcuk/Documents/New%20project/crates/tx-builder/src/builder.rs)
- [programs/arbitrage-aggregator/src/cpi/orca.rs](/Users/polinababijcuk/Documents/New%20project/programs/arbitrage-aggregator/src/cpi/orca.rs)

Problem:
- when ATA creation and `WSOL` funding were included inside the same versioned transaction as `execute_arbitrage`, simulation failed with:
  - `VersionedTransaction too large: 1780 bytes`
- the code is now changed so that setup instructions are split out from the arbitrage transaction

Impact:
- this removes one fake blocker from the hot path
- but it also makes the real operational requirement explicit: token accounts and `WSOL` liquidity need to be prepared ahead of honest on-chain simulation or live execution

### Blocker 5. Not all DEX CPI branches are implemented

Evidence:
- [programs/arbitrage-aggregator/src/instructions/execute.rs](/Users/polinababijcuk/Documents/New%20project/programs/arbitrage-aggregator/src/instructions/execute.rs)
- [programs/arbitrage-aggregator/src/instructions/swap.rs](/Users/polinababijcuk/Documents/New%20project/programs/arbitrage-aggregator/src/instructions/swap.rs)

Problem:
- `RaydiumAmm`, `Phoenix`, and `OpenBook` execution branches are still TODO/skeleton
- single-swap instruction is also mostly structural and does not yet prove real DEX execution

Impact:
- route universe is narrower than intended
- test surface for execution is incomplete

### Blocker 6. Tests do not validate real on-chain arbitrage execution

Evidence:
- [programs/arbitrage-aggregator/tests/test_aggregator.rs](/Users/polinababijcuk/Documents/New%20project/programs/arbitrage-aggregator/tests/test_aggregator.rs)

Problem:
- current tests validate serialization and simple route logic only
- there is no real integration test that executes `initialize`
- there is no real integration test that executes `execute_arbitrage` with CPI accounts

Impact:
- the most important path is not currently protected by meaningful tests

### Blocker 7. Network/program mapping is inconsistent

Evidence:
- [Anchor.toml](/Users/polinababijcuk/Documents/New%20project/Anchor.toml)
- [crates/tx-builder/src/aggregator.rs](/Users/polinababijcuk/Documents/New%20project/crates/tx-builder/src/aggregator.rs)

Problem:
- `Anchor.toml` has:
  - `programs.devnet.arbitrage_aggregator = "AGGRxxxxxxxx..."`
  - `programs.mainnet.arbitrage_aggregator = "DMCPSH38..."`
- tx-builder hardcodes `DMCPSH38...` as the aggregator program id

Impact:
- devnet/mainnet behavior is ambiguous
- the builder is not network-aware for aggregator program targeting

## Practical meaning

Today the repo supports:
- route discovery
- tx shape construction
- aggregator contract design
- partial real account derivation for one `Orca -> Orca` slice

But it does **not yet prove**:
- honest on-chain simulation of a real multi-hop aggregator tx with pre-provisioned execution accounts
- honest live execution path from bot to aggregator to DEXes

## Definition of done for Step 2

Step 2 is done only when all of the following are true:

1. Bot resolves a real base SPL token account instead of a placeholder.
2. Bot assembles correct `remaining_accounts` per route leg.
3. Required execution accounts are prepared before simulation or live send.
4. At least one honest 2-hop route succeeds in on-chain simulation through the aggregator.
5. Program config PDA and initialization are verified for the target cluster.
6. There is at least one real integration test for aggregator execution path.

## Recommended next action

Move to Step 3 with a very narrow target:

- add a real account preparation stage for required token accounts and `WSOL`
- then make one honest `Orca -> Orca` route work end-to-end through the aggregator in simulation
- ignore broader DEX coverage until that single vertical slice is real
