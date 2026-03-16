# Технологический стек

## Обзор

Полный список технологий, библиотек и SDK для production-уровня HFT бота на Solana.

---

## 1. On-chain разработка

### 1.1 Anchor Framework

**Назначение:** Написание смарт-контракта агрегатора.

```toml
[dependencies]
anchor-lang = "0.29.0"
anchor-spl = "0.29.0"
```

**Используется для:**
- Декларации программ
- CPI-вызовов к DEX
- ALT поддержки
- Безопасной работы с аккаунтами

### 1.2 Raw Solana SDK (без Anchor)

**Для низкоуровневой работы:**

```toml
[dependencies]
solana-program = "1.18"
solana-sdk = "1.18"
```

### 1.3 DEX IDL / Интерфейсы

#### Orca Whirlpools

```toml
[dependencies]
orca-whirlpools-core = "0.1"
orca-whirlpools-client = "0.1"
```

**GitHub:** https://github.com/orca-so/whirlpools

#### Raydium CLMM

```toml
# Нет официального Rust SDK
# Используем raw instructions + custom deserializers
```

**Репозиторий:** https://github.com/raydium-io/raydium-clmm

#### Phoenix DEX

```toml
[dependencies]
phoenix-sdk = "0.5"
```

**GitHub:** https://github.com/Ellipsis-Labs/phoenix-v1

#### OpenBook

```toml
[dependencies]
openbook-v2 = "0.1"
```

---

## 2. Off-chain (Rust Bot)

### 2.1 Solana Client Libraries

```toml
[dependencies]
# Основные
solana-sdk = "1.18"
solana-client = "1.18"
solana-rpc-client = "1.18"
solana-rpc-client-api = "1.18"
solana-transaction-status = "1.18"
solana-account-decoder = "1.18"

# Address Lookup Tables
solana-address-lookup-table-program = "1.18"
```

### 2.2 Async Runtime

```toml
[dependencies]
tokio = { version = "1.35", features = ["full"] }
tokio-util = "0.7"
futures = "0.3"
async-trait = "0.1"
```

### 2.3 Jito SDK

```toml
[dependencies]
jito-protos = "0.1"
jito-searcher-client = "0.1"
```

**GitHub:** https://github.com/jito-foundation/jito-solana

**Функции:**
- Отправка atomic bundles
- MEV protection
- Tip management
- Leader schedule watching

### 2.4 Сериализация

```toml
[dependencies]
# Borsh (Solana standard)
borsh = "1.3"
borsh-derive = "1.3"

# Serde (JSON/config)
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Zero-copy parsing
bytemuck = { version = "1.14", features = ["derive"] }
zerocopy = "0.7"
```

### 2.5 Big Numbers / Math

```toml
[dependencies]
# U256/U512 arithmetic
uint = "0.9"
primitive-types = "0.12"

# High precision
rug = "1.22"  # GMP bindings
num-bigint = "0.4"
num-traits = "0.2"

# Fixed point
fixed = "1.24"
```

### 2.6 Redis

```toml
[dependencies]
redis = { version = "0.24", features = ["tokio-comp", "connection-manager"] }
deadpool-redis = "0.14"
```

### 2.7 HTTP Client

```toml
[dependencies]
reqwest = { version = "0.11", features = ["json", "rustls-tls"] }
hyper = { version = "1.0", features = ["full"] }
```

### 2.8 WebSocket

```toml
[dependencies]
tokio-tungstenite = "0.21"
tungstenite = "0.21"
```

### 2.9 Configuration

```toml
[dependencies]
config = "0.14"
toml = "0.8"
dotenv = "0.15"
clap = { version = "4.4", features = ["derive"] }
```

### 2.10 Logging & Tracing

```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tracing-appender = "0.2"
```

### 2.11 Metrics

```toml
[dependencies]
metrics = "0.22"
metrics-exporter-prometheus = "0.13"
```

### 2.12 Error Handling

```toml
[dependencies]
thiserror = "1.0"
anyhow = "1.0"
```

### 2.13 Cryptography

```toml
[dependencies]
ed25519-dalek = "2.1"
curve25519-dalek = "4.1"
sha2 = "0.10"
bs58 = "0.5"
base64 = "0.21"
```

---

## 3. Полный Cargo.toml (Bot Core)

```toml
[package]
name = "solana-hft-bot"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"

[dependencies]
# === Solana ===
solana-sdk = "1.18"
solana-client = "1.18"
solana-rpc-client = "1.18"
solana-rpc-client-api = "1.18"
solana-transaction-status = "1.18"
solana-account-decoder = "1.18"

# === Async Runtime ===
tokio = { version = "1.35", features = ["full"] }
futures = "0.3"
async-trait = "0.1"

# === Jito ===
jito-protos = "0.1"
jito-searcher-client = "0.1"

# === Serialization ===
borsh = "1.3"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
bytemuck = { version = "1.14", features = ["derive"] }

# === Math ===
uint = "0.9"
num-bigint = "0.4"
num-traits = "0.2"

# === Redis ===
redis = { version = "0.24", features = ["tokio-comp", "connection-manager"] }
deadpool-redis = "0.14"

# === HTTP ===
reqwest = { version = "0.11", features = ["json", "rustls-tls"] }

# === WebSocket ===
tokio-tungstenite = "0.21"

# === Config ===
config = "0.14"
toml = "0.8"
clap = { version = "4.4", features = ["derive"] }

# === Logging ===
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# === Metrics ===
metrics = "0.22"
metrics-exporter-prometheus = "0.13"

# === Error Handling ===
thiserror = "1.0"
anyhow = "1.0"

# === Crypto ===
bs58 = "0.5"
base64 = "0.21"

[dev-dependencies]
tokio-test = "0.4"
criterion = "0.5"

[[bench]]
name = "deserialize_bench"
harness = false
```

---

## 4. On-chain Cargo.toml (Aggregator)

```toml
[package]
name = "swap-aggregator"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "lib"]
name = "swap_aggregator"

[features]
no-entrypoint = []
no-idl = []
no-log-ix-name = []
cpi = ["no-entrypoint"]
default = []

[dependencies]
anchor-lang = "0.29.0"
anchor-spl = "0.29.0"

# DEX integrations
orca-whirlpools-client = "0.1"
phoenix-sdk = "0.5"

[dev-dependencies]
anchor-client = "0.29.0"
```

---

## 5. DevOps инструменты

### 5.1 Docker

```dockerfile
# Dockerfile
FROM rust:1.75-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/hft-bot /usr/local/bin/
CMD ["hft-bot"]
```

### 5.2 Docker Compose

```yaml
version: '3.8'
services:
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    
  bot:
    build: .
    depends_on:
      - redis
    environment:
      - REDIS_URL=redis://redis:6379
      - RPC_URL=${RPC_URL}
    
  prometheus:
    image: prom/prometheus
    ports:
      - "9090:9090"
    
  grafana:
    image: grafana/grafana
    ports:
      - "3000:3000"
```

### 5.3 Мониторинг стек

| Tool | Purpose |
|------|---------|
| Prometheus | Metrics collection |
| Grafana | Visualization |
| Loki | Log aggregation |
| AlertManager | Alerting |

---

## 6. Security

### 6.1 Key Management

```toml
[dependencies]
# Hashicorp Vault
vaultrs = "0.7"

# YubiHSM
yubihsm = "0.42"
```

### 6.2 Secure Configuration

```bash
# .env (не коммитить!)
RPC_URL=https://private-rpc.example.com
JITO_AUTH_KEY=xxx
REDIS_PASSWORD=xxx
```

---

## 7. Рекомендуемые версии

| Component | Version | Notes |
|-----------|---------|-------|
| Rust | 1.75+ | MSRV |
| Solana CLI | 1.18.x | Latest stable |
| Anchor | 0.29.x | Latest |
| Node.js | 20.x | For Anchor tests |
| Redis | 7.x | Latest |
| Docker | 24.x | Latest |

---

## 8. Установка окружения

```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable
rustup component add clippy rustfmt

# Solana CLI
sh -c "$(curl -sSfL https://release.solana.com/v1.18.4/install)"

# Anchor
cargo install --git https://github.com/coral-xyz/anchor avm --locked
avm install latest
avm use latest

# Redis (local dev)
docker run -d --name redis -p 6379:6379 redis:7-alpine
```

