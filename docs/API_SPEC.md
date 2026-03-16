# API Specification

## Внутренние API между сервисами

---

## 1. RPC Proxy API

### HTTP Endpoints

```
POST /rpc/getMultipleAccounts
POST /rpc/simulateTransaction
POST /rpc/sendTransaction
GET  /health
GET  /metrics
```

### Request: Get Multiple Accounts

```json
POST /rpc/getMultipleAccounts
Content-Type: application/json

{
    "pubkeys": [
        "So11111111111111111111111111111111111111112",
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
    ],
    "encoding": "base64",
    "commitment": "confirmed"
}
```

### Response

```json
{
    "accounts": [
        {
            "pubkey": "So11111111111111111111111111111111111111112",
            "data": "base64_encoded_data...",
            "owner": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
            "lamports": 1000000,
            "executable": false,
            "rent_epoch": 123
        }
    ],
    "slot": 123456789,
    "cache_hit": true,
    "latency_ms": 12
}
```

---

## 2. Pool Deserializer API

### Internal Trait Interface

```rust
pub trait PoolDeserializer: Send + Sync {
    /// Deserialize raw account data into ParsedPool
    fn deserialize(&self, data: &[u8]) -> Result<ParsedPool, DeserializeError>;
    
    /// Pool type identifier
    fn pool_type(&self) -> PoolType;
    
    /// Expected data length
    fn expected_size(&self) -> usize;
}
```

### ParsedPool Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPool {
    pub address: Pubkey,
    pub pool_type: PoolType,
    pub token_a: TokenInfo,
    pub token_b: TokenInfo,
    pub reserves: Reserves,
    pub fee_rate_bps: u16,
    pub tick_data: Option<TickData>,
    pub last_updated_slot: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub mint: Pubkey,
    pub vault: Pubkey,
    pub decimals: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reserves {
    pub amount_a: u64,
    pub amount_b: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickData {
    pub current_tick: i32,
    pub sqrt_price: u128,
    pub liquidity: u128,
}
```

---

## 3. Pricing Engine API

### Internal Interface

```rust
pub trait PricingEngine: Send + Sync {
    /// Get quote for a swap
    fn quote(
        &self,
        pool: &Pubkey,
        amount_in: u64,
        swap_direction: SwapDirection,
    ) -> Result<Quote, PricingError>;
    
    /// Find all routes between tokens
    fn find_routes(
        &self,
        token_in: &Pubkey,
        token_out: &Pubkey,
        amount: u64,
        max_hops: usize,
    ) -> Vec<Route>;
    
    /// Update pool state
    fn update_pool(&self, pool: ParsedPool);
}
```

### Quote Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub pool: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,
    pub price_impact_bps: u16,
    pub fee_amount: u64,
    pub minimum_out: u64,  // With slippage
}
```

### Route Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub legs: Vec<SwapLeg>,
    pub total_amount_in: u64,
    pub expected_amount_out: u64,
    pub price_impact_bps: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapLeg {
    pub pool: Pubkey,
    pub pool_type: PoolType,
    pub token_in: Pubkey,
    pub token_out: Pubkey,
    pub amount_in: u64,
    pub expected_out: u64,
}
```

---

## 4. Strategy API

### Internal Interface

```rust
pub trait Strategy: Send + Sync {
    /// Find arbitrage opportunities
    async fn find_opportunities(&self) -> Vec<Opportunity>;
    
    /// Evaluate single opportunity
    fn evaluate(&self, opportunity: &Opportunity) -> EvaluationResult;
}
```

### Opportunity Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opportunity {
    pub id: Uuid,
    pub route: Route,
    pub input_token: Pubkey,
    pub input_amount: u64,
    pub expected_profit: i64,  // Can be negative
    pub profit_bps: i16,
    pub timestamp: u64,
    pub expires_at: u64,
}

#[derive(Debug, Clone)]
pub struct EvaluationResult {
    pub should_execute: bool,
    pub confidence: f64,
    pub risk_factors: Vec<RiskFactor>,
}
```

---

## 5. Transaction Builder API

### Internal Interface

```rust
pub trait TxBuilder: Send + Sync {
    /// Build swap transaction from route
    async fn build_swap_tx(
        &self,
        route: &Route,
        config: TxConfig,
    ) -> Result<BuiltTransaction, BuildError>;
    
    /// Build Jito bundle
    async fn build_bundle(
        &self,
        routes: &[Route],
        tip_lamports: u64,
    ) -> Result<Bundle, BuildError>;
}
```

### TxConfig

```rust
#[derive(Debug, Clone)]
pub struct TxConfig {
    pub payer: Pubkey,
    pub slippage_bps: u16,
    pub compute_unit_price: u64,
    pub compute_unit_limit: u32,
    pub use_alt: bool,
}
```

### BuiltTransaction

```rust
#[derive(Debug, Clone)]
pub struct BuiltTransaction {
    pub transaction: VersionedTransaction,
    pub signers_needed: Vec<Pubkey>,
    pub compute_estimate: u32,
    pub accounts_used: Vec<Pubkey>,
}
```

---

## 6. Simulator API

### Internal Interface

```rust
pub trait Simulator: Send + Sync {
    /// Simulate transaction
    async fn simulate(
        &self,
        tx: &VersionedTransaction,
    ) -> Result<SimulationResult, SimulationError>;
    
    /// Batch simulate
    async fn simulate_batch(
        &self,
        txs: &[VersionedTransaction],
    ) -> Vec<Result<SimulationResult, SimulationError>>;
}
```

### SimulationResult

```rust
#[derive(Debug, Clone)]
pub struct SimulationResult {
    pub success: bool,
    pub compute_units_consumed: u64,
    pub logs: Vec<String>,
    pub return_data: Option<Vec<u8>>,
    pub accounts_changed: Vec<AccountDelta>,
}

#[derive(Debug, Clone)]
pub struct AccountDelta {
    pub pubkey: Pubkey,
    pub lamports_before: u64,
    pub lamports_after: u64,
    pub data_changed: bool,
}
```

---

## 7. Scheduler API

### Internal Interface

```rust
pub trait Scheduler: Send + Sync {
    /// Queue opportunity for execution
    fn queue(&self, opportunity: Opportunity) -> QueueResult;
    
    /// Get queue status
    fn status(&self) -> QueueStatus;
    
    /// Cancel queued item
    fn cancel(&self, id: &Uuid) -> bool;
}
```

### QueueStatus

```rust
#[derive(Debug, Clone, Serialize)]
pub struct QueueStatus {
    pub pending: usize,
    pub in_progress: usize,
    pub completed_last_minute: usize,
    pub failed_last_minute: usize,
    pub rate_limited: bool,
}
```

---

## 8. Signer API

### Internal Interface

```rust
#[async_trait]
pub trait Signer: Send + Sync {
    /// Get public key
    fn pubkey(&self) -> Pubkey;
    
    /// Sign message
    async fn sign(&self, message: &[u8]) -> Result<Signature, SignerError>;
    
    /// Sign transaction
    async fn sign_transaction(
        &self,
        tx: &mut VersionedTransaction,
    ) -> Result<(), SignerError>;
}
```

---

## 9. Metrics API

### HTTP Endpoints

```
GET /metrics           # Prometheus format
GET /health            # Health check
GET /stats             # JSON stats
```

### Prometheus Metrics

```prometheus
# HELP trades_total Total number of trades executed
# TYPE trades_total counter
trades_total{status="success"} 1234
trades_total{status="failed"} 56

# HELP profit_usd_total Total profit in USD
# TYPE profit_usd_total gauge
profit_usd_total 5678.90

# HELP rpc_latency_ms RPC request latency
# TYPE rpc_latency_ms histogram
rpc_latency_ms_bucket{endpoint="primary",le="10"} 100
rpc_latency_ms_bucket{endpoint="primary",le="50"} 950
rpc_latency_ms_bucket{endpoint="primary",le="100"} 999

# HELP pools_tracked Number of pools being tracked
# TYPE pools_tracked gauge
pools_tracked{dex="orca"} 500
pools_tracked{dex="raydium"} 300
```

### JSON Stats

```json
GET /stats

{
    "uptime_seconds": 86400,
    "trades": {
        "total": 1290,
        "success": 1234,
        "failed": 56,
        "success_rate": 0.956
    },
    "pnl": {
        "total_usd": 5678.90,
        "last_hour_usd": 123.45,
        "last_24h_usd": 2345.67
    },
    "latency": {
        "rpc_p50_ms": 25,
        "rpc_p99_ms": 85,
        "pipeline_p50_ms": 150,
        "pipeline_p99_ms": 280
    },
    "pools": {
        "total": 800,
        "by_dex": {
            "orca": 500,
            "raydium": 200,
            "phoenix": 100
        }
    }
}
```

---

## 10. Redis Cache Schema

### Keys

```
# Pool state (TTL: 200ms)
pool:{address}:state -> Borsh<ParsedPool>
pool:{address}:updated -> timestamp_ms

# ALT cache (TTL: 5min)
alt:{address} -> JSON<[Pubkey]>

# Price cache (TTL: 100ms)
price:{token_a}:{token_b} -> f64

# Route cache (TTL: 50ms)
route:{hash} -> JSON<Route>

# Metrics (no TTL)
metrics:trades:total -> i64
metrics:pnl:total -> f64
```

### Example Operations

```rust
// Set pool state
redis.set_ex(
    format!("pool:{}:state", pool.address),
    borsh::serialize(&pool)?,
    Duration::from_millis(200),
).await?;

// Get pool state
let data: Vec<u8> = redis.get(format!("pool:{}:state", address)).await?;
let pool: ParsedPool = borsh::deserialize(&data)?;
```

---

## 11. Configuration Schema

### config.toml

```toml
[general]
log_level = "info"
network = "mainnet-beta"  # devnet, testnet, mainnet-beta

[rpc]
primary_url = "https://private-rpc.example.com"
fallback_urls = [
    "https://api.helius.xyz/v0/?api-key=xxx",
    "https://triton.example.com"
]
timeout_ms = 5000
max_retries = 3

[redis]
url = "redis://localhost:6379"
pool_size = 10
cache_ttl_ms = 200

[strategy]
min_profit_bps = 10
max_slippage_bps = 50
max_hops = 3
min_liquidity_usd = 10000

[execution]
compute_unit_price = 1000
compute_unit_limit = 400000
use_jito = true
jito_tip_lamports = 10000

[pools]
# Pool addresses to monitor
include = [
    "HJPjoWUrhoZzkNfRpHuieeFk9WcZWjwy6PBjZ81ngndJ",  # Orca SOL/USDC
    "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2",  # Raydium SOL/USDC
]
exclude = []

[monitoring]
prometheus_port = 9090
health_port = 8080
```

---

## 12. Error Codes

```rust
#[derive(Debug, thiserror::Error)]
pub enum BotError {
    // RPC errors (1xxx)
    #[error("RPC connection failed: {0}")]
    RpcConnection(#[from] solana_client::client_error::ClientError) = 1001,
    
    #[error("RPC timeout")]
    RpcTimeout = 1002,
    
    #[error("Rate limited")]
    RateLimited = 1003,
    
    // Deserialization errors (2xxx)
    #[error("Invalid pool data")]
    InvalidPoolData = 2001,
    
    #[error("Unknown pool type")]
    UnknownPoolType = 2002,
    
    // Pricing errors (3xxx)
    #[error("Insufficient liquidity")]
    InsufficientLiquidity = 3001,
    
    #[error("Price impact too high")]
    PriceImpactTooHigh = 3002,
    
    // Simulation errors (4xxx)
    #[error("Simulation failed: {0}")]
    SimulationFailed(String) = 4001,
    
    #[error("Insufficient funds")]
    InsufficientFunds = 4002,
    
    // Execution errors (5xxx)
    #[error("Transaction failed: {0}")]
    TransactionFailed(String) = 5001,
    
    #[error("Slippage exceeded")]
    SlippageExceeded = 5002,
    
    #[error("Bundle rejected")]
    BundleRejected = 5003,
}
```

