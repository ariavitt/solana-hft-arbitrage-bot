# План реализации

## Обзор фаз

```
┌────────────────────────────────────────────────────────────────────────────┐
│  PHASE 1: INFRA          │  PHASE 2: SERVICES        │  PHASE 3: ONCHAIN   │
│  (Неделя 1-2)            │  (Неделя 3-8)             │  (Неделя 9-12)      │
│  ─────────────────────   │  ─────────────────────    │  ─────────────────  │
│  • RPC setup             │  • Poller                 │  • Aggregator       │
│  • Server provisioning   │  • Proxy                  │  • DEX CPI          │
│  • Redis setup           │  • Deserializer           │  • Jito integration │
│  • Basic monitoring      │  • Pricing engine         │  • Testing          │
│                          │  • Strategy               │                     │
│                          │  • Simulator              │                     │
│                          │  • Scheduler              │                     │
│                          │  • Signer                 │                     │
└────────────────────────────────────────────────────────────────────────────┘
                                        │
                                        ▼
┌────────────────────────────────────────────────────────────────────────────┐
│                           PHASE 4: OPS (Неделя 13-16)                      │
│  ─────────────────────────────────────────────────────────────────────     │
│  • Key management (HSM)                                                    │
│  • Backtesting framework                                                   │
│  • Alerting system                                                         │
│  • Production deployment                                                   │
│  • Performance tuning                                                      │
└────────────────────────────────────────────────────────────────────────────┘
```

---

## Phase 1: Infrastructure (Неделя 1-2)

### Неделя 1: Базовая инфраструктура

#### День 1-2: RPC Setup

**Задачи:**
- [ ] Выбор и настройка приватного RPC
- [ ] Регистрация на Helius/Triton (fallback)
- [ ] Тестирование latency

**Результат:**
```toml
# config/rpc.toml
[primary]
url = "https://private-rpc.example.com"
timeout_ms = 5000

[fallback]
urls = [
    "https://api.helius.xyz/...",
    "https://triton.example.com/..."
]
```

#### День 3-4: Server Provisioning

**Задачи:**
- [ ] Заказ сервера (US East)
- [ ] Базовая настройка OS
- [ ] Установка Docker, Redis
- [ ] Firewall configuration

**Скрипт настройки:**
```bash
#!/bin/bash
# scripts/setup-server.sh

# Update system
apt update && apt upgrade -y

# Install dependencies
apt install -y build-essential pkg-config libssl-dev

# Install Docker
curl -fsSL https://get.docker.com | sh

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Start Redis
docker run -d --name redis -p 6379:6379 redis:7-alpine
```

#### День 5-7: Project Setup

**Задачи:**
- [ ] Инициализация Cargo workspace
- [ ] Базовая структура проекта
- [ ] CI/CD setup (GitHub Actions)
- [ ] Базовый мониторинг

---

### Неделя 2: Rust Workspace

#### Структура:

```
crates/
├── bot-core/           # Shared types, config
├── rpc-proxy/          # RPC proxy service
├── pool-deserializer/  # Pool parsing
├── pricing-engine/     # Quote calculation
├── strategy/           # Route finding
├── tx-builder/         # Transaction building
└── aggregator/         # On-chain program
```

#### Задачи:
- [ ] Создание всех crates
- [ ] Shared types и config
- [ ] Error types
- [ ] Basic logging setup

---

## Phase 2: Services (Неделя 3-8)

### Неделя 3-4: Data Pipeline

#### RPC Proxy (rpc-proxy)

```rust
// Основной интерфейс
pub struct RpcProxy {
    primary: RpcClient,
    fallbacks: Vec<RpcClient>,
    cache: RedisPool,
}

impl RpcProxy {
    pub async fn get_multiple_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> Result<Vec<Option<Account>>> {
        // 1. Check cache
        // 2. Batch request to RPC
        // 3. Update cache
        // 4. Return
    }
}
```

**Milestone:** Proxy возвращает данные < 50ms

#### Pool Deserializer (pool-deserializer)

```rust
pub trait PoolDeserializer {
    fn deserialize(&self, data: &[u8]) -> Result<ParsedPool>;
    fn pool_type(&self) -> PoolType;
}

// Implementations
pub struct OrcaWhirlpoolDeserializer;
pub struct RaydiumClmmDeserializer;
pub struct PhoenixDeserializer;
```

**Milestone:** Парсинг 1000 pools/sec

---

### Неделя 5-6: Pricing & Strategy

#### Pricing Engine (pricing-engine)

```rust
pub struct PricingEngine {
    pools: HashMap<Pubkey, ParsedPool>,
}

impl PricingEngine {
    pub fn quote(
        &self,
        pool: &Pubkey,
        amount_in: u64,
        direction: SwapDirection,
    ) -> Result<Quote> {
        // AMM math based on pool type
    }
    
    pub fn find_routes(
        &self,
        token_in: Pubkey,
        token_out: Pubkey,
        amount: u64,
    ) -> Vec<Route> {
        // Graph traversal
    }
}
```

#### Strategy (strategy)

```rust
pub struct ArbitrageStrategy {
    pricing: Arc<PricingEngine>,
    min_profit_bps: u16,
    max_hops: usize,
}

impl ArbitrageStrategy {
    pub async fn find_opportunities(&self) -> Vec<Opportunity> {
        // 1. Get all routes
        // 2. Calculate profit
        // 3. Filter by min_profit
        // 4. Sort by profit
    }
}
```

**Milestone:** Находит profitable routes в real-time

---

### Неделя 7-8: Execution Pipeline

#### Transaction Builder (tx-builder)

```rust
pub struct TxBuilder {
    signer: Keypair,
    rpc: RpcClient,
}

impl TxBuilder {
    pub async fn build_swap_tx(
        &self,
        route: &Route,
    ) -> Result<VersionedTransaction> {
        // 1. Build instructions
        // 2. Get ALT
        // 3. Get recent blockhash
        // 4. Create message
        // 5. Sign
    }
}
```

#### Simulator

```rust
pub async fn simulate(
    rpc: &RpcClient,
    tx: &VersionedTransaction,
) -> Result<SimulationResult> {
    let result = rpc.simulate_transaction(tx).await?;
    
    if let Some(err) = result.err {
        return Err(SimulationError::Failed(err));
    }
    
    Ok(SimulationResult {
        compute_units: result.units_consumed,
        logs: result.logs,
    })
}
```

**Milestone:** End-to-end тест на devnet

---

## Phase 3: On-chain (Неделя 9-12)

### Неделя 9-10: Aggregator Contract

#### Anchor Program

```rust
#[program]
pub mod swap_aggregator {
    use super::*;

    pub fn execute_swap(
        ctx: Context<ExecuteSwap>,
        route: SwapRoute,
        min_amount_out: u64,
    ) -> Result<()> {
        // Execute each leg
        for leg in route.legs {
            execute_leg(&ctx, &leg)?;
        }
        
        // Verify output
        let final_amount = get_token_balance(&ctx.accounts.output)?;
        require!(
            final_amount >= min_amount_out,
            ErrorCode::SlippageExceeded
        );
        
        Ok(())
    }
}
```

#### CPI Integrations

```rust
// Orca CPI
pub fn cpi_orca_swap(
    ctx: &Context<ExecuteSwap>,
    amount_in: u64,
    min_out: u64,
) -> Result<()> {
    // Build Orca swap instruction
    // Execute via CPI
}

// Raydium CPI
pub fn cpi_raydium_swap(...) -> Result<()> { ... }

// Phoenix CPI
pub fn cpi_phoenix_swap(...) -> Result<()> { ... }
```

**Milestone:** Работающий swap на devnet

---

### Неделя 11-12: Jito Integration

#### Bundle Submission

```rust
pub struct JitoClient {
    client: SearcherClient,
    tip_account: Pubkey,
}

impl JitoClient {
    pub async fn submit_bundle(
        &self,
        txs: Vec<VersionedTransaction>,
        tip_lamports: u64,
    ) -> Result<BundleId> {
        let bundle = Bundle {
            transactions: txs,
            tip: tip_lamports,
        };
        
        self.client.send_bundle(bundle).await
    }
}
```

**Milestone:** Успешный bundle на mainnet-beta

---

## Phase 4: Operations (Неделя 13-16)

### Неделя 13-14: Security & Key Management

#### HSM Integration

```rust
pub trait Signer {
    async fn sign(&self, message: &[u8]) -> Result<Signature>;
    fn pubkey(&self) -> Pubkey;
}

// File-based (dev)
pub struct FileSigner { keypair: Keypair }

// Vault (prod)
pub struct VaultSigner { client: VaultClient }

// HSM (high-security)
pub struct HsmSigner { hsm: YubiHsm }
```

### Неделя 15-16: Monitoring & Alerting

#### Prometheus Metrics

```rust
use metrics::{counter, gauge, histogram};

pub fn record_trade(profit: f64, latency_ms: f64) {
    counter!("trades_total").increment(1);
    gauge!("profit_last").set(profit);
    histogram!("trade_latency_ms").record(latency_ms);
}
```

#### Grafana Dashboards

- RPC latency
- Trades per minute
- P&L
- Error rates
- Pool coverage

---

## Milestones Summary

| Phase | Week | Milestone | Verification |
|-------|------|-----------|--------------|
| 1 | 1 | RPC working | `curl` test |
| 1 | 2 | Workspace setup | `cargo build` |
| 2 | 4 | Data pipeline | Unit tests |
| 2 | 6 | Pricing engine | Benchmark |
| 2 | 8 | E2E devnet | Integration test |
| 3 | 10 | Aggregator | Devnet deploy |
| 3 | 12 | Jito bundle | Mainnet test |
| 4 | 14 | Security | Audit checklist |
| 4 | 16 | Production | Live trading |

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| RPC downtime | Multiple fallbacks |
| Price staleness | Aggressive TTL |
| MEV attacks | Jito bundles |
| Key compromise | HSM + rotation |
| Strategy failure | Circuit breakers |

---

## Definition of Done

### MVP (Week 8)
- [ ] Finds arbitrage opportunities
- [ ] Simulates successfully
- [ ] Works on devnet

### Beta (Week 12)
- [ ] Executes real trades
- [ ] Jito integration
- [ ] Basic monitoring

### Production (Week 16)
- [ ] HSM key management
- [ ] Full alerting
- [ ] Documented runbook
- [ ] Performance optimized

