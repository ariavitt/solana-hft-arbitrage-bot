# 📋 ПЛАН ПРОЕКТА: Solana HFT Arbitrage Bot

> Полный пошаговый план разработки высокочастотного арбитражного бота

---

## 📌 СОДЕРЖАНИЕ

1. [Обзор проекта](#1-обзор-проекта)
2. [Архитектура](#2-архитектура)
3. [Roadmap](#3-roadmap)
4. [ФАЗА 1: Инфраструктура](#4-фаза-1-инфраструктура-неделя-1-2)
5. [ФАЗА 2: Сервисы](#5-фаза-2-сервисы-неделя-3-8)
6. [ФАЗА 3: On-chain](#6-фаза-3-on-chain-неделя-9-12)
7. [ФАЗА 4: Operations](#7-фаза-4-operations-неделя-13-16)
8. [Быстрый старт](#8-быстрый-старт)

---

# 1. ОБЗОР ПРОЕКТА

## 🎯 Цель

Создание высокочастотного бота для арбитража на DEX биржах Solana:
- **Минимальная задержка:** < 200ms end-to-end
- **Атомарное исполнение:** через smart-contract aggregator
- **MEV защита:** через Jito bundles
- **Профитабельность:** > 10 bps на сделку

## 💰 Бюджет (ежемесячно)

| Статья | Стоимость |
|--------|-----------|
| Server (US East) | $200-500 |
| RPC (Helius/Triton) | $200-500 |
| Private RPC | $500-2000 |
| Jito tips | Variable |
| **Итого** | **$1000-3000** |

## ⚠️ Риски

| Риск | Вероятность | Митигация |
|------|-------------|-----------|
| RPC downtime | Средняя | Multiple fallbacks |
| MEV attacks | Высокая | Jito bundles |
| Strategy failure | Средняя | Circuit breakers |
| Key compromise | Низкая | HSM + rotation |

---

# 2. АРХИТЕКТУРА

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         INFRASTRUCTURE                                   │
│         Private RPC  │  Paid RPC (Helius/Triton)  │  US Server          │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                            SERVICES                                      │
│                                                                          │
│   POLLER ──→ PROXY ──→ REDIS ──→ DESERIALIZER ──→ PRICING              │
│                                                        │                 │
│                                                        ▼                 │
│   MEV_MON ──→ STRATEGY ←── SIMULATOR ←── SCHEDULER ←── SIGNER          │
│                   │                                                      │
│                   └──→ METRICS ──→ ALERTS                               │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                            ON-CHAIN                                      │
│                                                                          │
│   AGGREGATOR (CPI) ──→ Orca │ Raydium │ Phoenix                         │
│        │                                                                 │
│        └──→ JITO BUNDLE                                                 │
└─────────────────────────────────────────────────────────────────────────┘
```

## Компоненты

| Сервис | Назначение | Latency Budget |
|--------|------------|----------------|
| Poller | Опрос пулов DEX | 30ms |
| Proxy | RPC кэширование | 10ms |
| Redis | Кэш данных | 1ms |
| Deserializer | Парсинг пулов | 5ms |
| Pricing | Котировки | 2ms |
| Strategy | Поиск маршрутов | 5ms |
| Simulator | Проверка TX | 50ms |
| Signer | Подписание | 1ms |
| **Всего** | | **~105ms** |

---

# 3. ROADMAP

```
┌──────────────────────────────────────────────────────────────────────────┐
│                         20-НЕДЕЛЬНЫЙ ROADMAP                             │
│                      (локальная разработка)                              │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ФАЗА 1: INFRA (Неделя 1-2)                                             │
│  ████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░                        │
│  ✓ Rust + Solana CLI                                                     │
│  ✓ Cargo workspace                                                       │
│  • Redis + RPC setup                                                     │
│                                                                          │
│  ФАЗА 2: SERVICES (Неделя 3-8)                                          │
│  ░░░░░░░░████████████████████████████░░░░░░░░░░░░░░░░░░░░                │
│  • RPC Proxy + Poller                                                    │
│  • Pool Deserializers                                                    │
│  • Pricing Engine                                                        │
│  • Strategy + TX Builder                                                 │
│                                                                          │
│  ФАЗА 3: ON-CHAIN (Неделя 9-14)                                         │
│  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░████████████████████░░░░                │
│  • Anchor Aggregator                                                     │
│  • CPI integrations                                                      │
│  • Jito Bundle                                                           │
│                                                                          │
│  ФАЗА 4: OPS (Неделя 15-20)                                             │
│  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░████████               │
│  • Аренда сервера                                                        │
│  • Мониторинг                                                            │
│  • Production                                                            │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

## Milestones

| # | Milestone | Неделя | Критерий |
|---|-----------|--------|----------|
| M1 | Infrastructure Ready | 2 | `cargo build` работает |
| M2 | Data Pipeline | 4 | Пулы парсятся |
| M3 | Pricing Complete | 6 | 1000 quotes/sec |
| M4 | Devnet E2E | 8 | TX на devnet |
| M5 | On-chain Ready | 12 | Aggregator на devnet |
| M6 | Jito Integration | 14 | Bundle работает |
| M7 | Production | 20 | Первый профит |

---

# 4. ФАЗА 1: ИНФРАСТРУКТУРА (Неделя 1-2)

## ✅ Чеклист

- [x] Установить Rust
- [x] Установить Solana CLI  
- [x] Создать Cargo workspace
- [ ] Установить Docker
- [ ] Запустить Redis
- [ ] Настроить RPC
- [ ] Создать devnet кошелёк

---

## Шаг 1.1: Установка Rust ✅

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup default stable
```

**Проверка:**
```bash
rustc --version  # rustc 1.75+
cargo --version
```

---

## Шаг 1.2: Установка Solana CLI ✅

```bash
cargo install solana-cli --version 1.18.26
```

**Проверка:**
```bash
solana --version  # solana-cli 1.18.x
```

---

## Шаг 1.3: Установка Docker

### macOS (Docker Desktop)

1. Скачать: https://www.docker.com/products/docker-desktop/
2. Установить .dmg
3. Запустить Docker.app

### macOS (Homebrew)

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
echo 'eval "$(/opt/homebrew/bin/brew shellenv)"' >> ~/.zprofile
eval "$(/opt/homebrew/bin/brew shellenv)"
brew install --cask docker
open /Applications/Docker.app
```

**Проверка:**
```bash
docker --version
docker run hello-world
```

---

## Шаг 1.4: Запуск Redis

```bash
docker run -d --name redis -p 6379:6379 redis:7-alpine
```

**Проверка:**
```bash
docker exec -it redis redis-cli ping
# Ответ: PONG
```

---

## Шаг 1.5: Настройка RPC

### Бесплатные варианты (для разработки):

| Провайдер | URL | Лимит |
|-----------|-----|-------|
| Devnet (публичный) | `https://api.devnet.solana.com` | ~100 RPS |
| Helius (бесплатный) | `https://devnet.helius-rpc.com/?api-key=XXX` | 10 RPS |

### Регистрация на Helius:
1. Зайти на https://helius.dev
2. Создать аккаунт
3. Получить API key
4. Использовать: `https://devnet.helius-rpc.com/?api-key=YOUR_KEY`

### Обновить конфиг:

Отредактировать `config/devnet.toml`:
```toml
[rpc]
primary_url = "https://devnet.helius-rpc.com/?api-key=YOUR_KEY"
```

---

## Шаг 1.6: Создание devnet кошелька

```bash
# Создать кошелёк
solana-keygen new -o ~/.config/solana/devnet.json

# Настроить на devnet
solana config set --url devnet
solana config set --keypair ~/.config/solana/devnet.json

# Получить SOL для тестов
solana airdrop 5

# Проверить баланс
solana balance
```

---

## Шаг 1.7: Проверка проекта

```bash
cd "/Users/polinababijcuk/Новая папка"

# Сборка
cargo build

# Тесты
cargo test
```

---

# 5. ФАЗА 2: СЕРВИСЫ (Неделя 3-8)

## ✅ Чеклист

- [ ] RPC Proxy работает
- [ ] Cache layer (Redis) функционирует
- [ ] Poller опрашивает пулы
- [ ] Deserializer парсит Orca/Raydium
- [ ] Pricing engine считает котировки
- [ ] Route graph находит маршруты
- [ ] Strategy находит арбитраж
- [ ] TX Builder строит транзакции
- [ ] Simulator проверяет TX
- [ ] E2E тест на devnet

---

## Шаг 2.1: RPC Proxy (Неделя 3)

**Файл:** `crates/rpc-proxy/src/proxy.rs`

**Задачи:**
1. Реализовать подключение к primary RPC
2. Добавить fallback RPCs
3. Интегрировать Redis cache
4. Добавить метрики latency

**Тест:**
```bash
cargo test -p rpc-proxy
```

---

## Шаг 2.2: Pool Poller (Неделя 3-4)

**Задачи:**
1. Создать crate `pool-poller`
2. Реализовать polling loop (50-100ms interval)
3. Собрать список адресов пулов Orca/Raydium
4. Отправлять данные в deserializer

**Пулы для тестов (devnet):**
```
Orca USDC/SOL: 3ne4mWqdYuNiYrYZC9TrA3FcfuFdErghH97vNPbjicr1
```

---

## Шаг 2.3: Pool Deserializer (Неделя 4-5)

**Файлы:**
- `crates/pool-deserializer/src/orca.rs` ✅
- `crates/pool-deserializer/src/raydium.rs` ✅

**Задачи:**
1. ✅ Парсинг Orca Whirlpool state
2. ✅ Парсинг Raydium CLMM state
3. Добавить Phoenix deserializer
4. Unit тесты с реальными данными

---

## Шаг 2.4: Pricing Engine (Неделя 5-6)

**Файлы:**
- `crates/pricing-engine/src/amm.rs` ✅
- `crates/pricing-engine/src/engine.rs` ✅
- `crates/pricing-engine/src/graph.rs` ✅

**Задачи:**
1. ✅ AMM math для concentrated liquidity
2. ✅ Quote calculation
3. ✅ Route graph (DFS)
4. Benchmark: 1000 quotes/sec

**Формула CLMM:**
```
Δy = L × (√P_upper - √P_current)
Δx = L × (1/√P_current - 1/√P_lower)
```

---

## Шаг 2.5: Strategy (Неделя 6-7)

**Файл:** `crates/strategy/src/arbitrage.rs` ✅

**Задачи:**
1. ✅ Поиск circular routes (A → B → A)
2. ✅ Расчёт профита
3. Фильтрация по min_profit_bps
4. Сортировка по профитабельности

**Конфигурация:**
```toml
[strategy]
min_profit_bps = 10      # Минимум 0.1% профит
max_slippage_bps = 50    # Макс 0.5% slippage
max_hops = 3             # Макс 3 свопа
```

---

## Шаг 2.6: TX Builder & Simulator (Неделя 7-8)

**Файлы:**
- `crates/tx-builder/src/builder.rs` ✅
- `crates/tx-builder/src/simulator.rs` ✅

**Задачи:**
1. ✅ Сборка swap instructions
2. ✅ Добавление compute budget
3. ✅ Подписание транзакции
4. ✅ Симуляция через RPC
5. E2E тест на devnet

---

## Шаг 2.7: E2E тест на Devnet (Неделя 8)

```bash
# 1. Убедиться что Redis запущен
docker start redis

# 2. Проверить баланс
solana balance

# 3. Запустить бота в dry-run режиме
cargo run --release -- --config config/devnet.toml --dry-run
```

**Ожидаемый результат:**
- Бот находит арбитражные возможности
- Симуляция проходит успешно
- Логи показывают найденные маршруты

---

# 6. ФАЗА 3: ON-CHAIN (Неделя 9-14)

## ✅ Чеклист

- [ ] Anchor установлен
- [ ] Aggregator компилируется
- [ ] CPI к Orca работает
- [ ] CPI к Raydium работает
- [ ] Deploy на devnet
- [ ] Jito client подключается
- [ ] Bundle submission работает
- [ ] E2E тест на mainnet-beta

---

## Шаг 3.1: Установка Anchor (Неделя 9)

```bash
cargo install --git https://github.com/coral-xyz/anchor avm --locked
avm install latest
avm use latest
anchor --version
```

---

## Шаг 3.2: Создание Aggregator программы (Неделя 9-10)

```bash
cd "/Users/polinababijcuk/Новая папка"
mkdir -p programs/swap-aggregator
cd programs/swap-aggregator
anchor init . --name swap_aggregator
```

**Структура:**
```
programs/swap-aggregator/
├── Cargo.toml
└── src/
    ├── lib.rs           # Program entry
    ├── instructions/    # Handlers
    ├── cpi/             # DEX integrations
    └── error.rs         # Errors
```

---

## Шаг 3.3: CPI интеграции (Неделя 10-11)

**Orca Whirlpool:**
- Program ID: `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc`
- Swap discriminator: `[0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8]`

**Raydium CLMM:**
- Program ID: `CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK`
- Swap discriminator: `[0x2b, 0x04, 0xed, 0x0b, 0x1a, 0xc9, 0x1e, 0x62]`

---

## Шаг 3.4: Deploy на Devnet (Неделя 11)

```bash
# Build
anchor build

# Get program ID
solana address -k target/deploy/swap_aggregator-keypair.json

# Update lib.rs с новым ID
# declare_id!("YOUR_PROGRAM_ID");

# Rebuild
anchor build

# Deploy
anchor deploy --provider.cluster devnet
```

---

## Шаг 3.5: Jito Integration (Неделя 12-14)

**Зависимости:**
```toml
[dependencies]
jito-protos = "0.1"
jito-searcher-client = "0.1"
```

**Задачи:**
1. Подключение к Jito Block Engine
2. Bundle submission
3. Tip management
4. Leader schedule monitoring

---

# 7. ФАЗА 4: OPERATIONS (Неделя 15-20)

## ✅ Чеклист

- [ ] Сервер арендован (US East)
- [ ] Код задеплоен на сервер
- [ ] HSM/Vault настроен
- [ ] Мониторинг работает (Prometheus + Grafana)
- [ ] Алерты настроены
- [ ] Первые сделки на mainnet
- [ ] Положительный P&L

---

## Шаг 4.1: Аренда сервера (Неделя 15)

**Рекомендации:**

| Провайдер | Specs | Цена | Локация |
|-----------|-------|------|---------|
| Latitude.sh | 8 cores, 32GB | $150/mo | Ashburn, VA |
| Vultr | Bare Metal | $120/mo | New Jersey |
| AWS | c6i.xlarge | $170/mo | us-east-1 |

---

## Шаг 4.2: Deploy на сервер (Неделя 15-16)

```bash
# На сервере:
# 1. Установить зависимости
apt update && apt install -y build-essential pkg-config libssl-dev

# 2. Установить Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env

# 3. Установить Docker + Redis
curl -fsSL https://get.docker.com | sh
docker run -d --name redis --restart unless-stopped -p 6379:6379 redis:7-alpine

# 4. Клонировать и собрать проект
git clone <your-repo> /opt/hft-bot
cd /opt/hft-bot
cargo build --release

# 5. Запустить
./target/release/hft-bot --config config/mainnet.toml
```

---

## Шаг 4.3: Мониторинг (Неделя 16-17)

**Docker Compose:**
```yaml
version: '3.8'
services:
  prometheus:
    image: prom/prometheus
    ports: ["9090:9090"]
    
  grafana:
    image: grafana/grafana
    ports: ["3000:3000"]
```

**Метрики:**
- `trades_total` — количество сделок
- `profit_usd` — профит
- `rpc_latency_ms` — latency RPC
- `success_rate` — % успешных TX

---

## Шаг 4.4: Production Launch (Неделя 18-20)

### Pre-launch checklist:
- [ ] Backup ключей
- [ ] Алерты в Telegram/Discord
- [ ] Circuit breaker при убытках
- [ ] Runbook для инцидентов

### Launch:
1. Начать с $100 капитала
2. Мониторить 24 часа
3. Анализировать P&L
4. Постепенно увеличивать объём

---

# 8. БЫСТРЫЙ СТАРТ

## Текущий статус проекта

```
✅ Rust установлен
✅ Solana CLI установлен  
✅ Cargo workspace создан
✅ Базовые crates реализованы
⏳ Docker — в процессе установки
⏳ Redis — ожидает Docker
⏳ RPC — нужно настроить
```

## Следующие шаги (прямо сейчас)

1. **Установить Docker Desktop** (инструкция выше)
2. **Запустить Redis:**
   ```bash
   docker run -d --name redis -p 6379:6379 redis:7-alpine
   ```
3. **Настроить RPC в конфиге:**
   ```bash
   # Отредактировать config/devnet.toml
   # Добавить Helius API key
   ```
4. **Создать devnet кошелёк:**
   ```bash
   solana-keygen new -o ~/.config/solana/devnet.json
   solana config set --url devnet
   solana airdrop 5
   ```

## Полезные команды

```bash
# Сборка
cargo build

# Тесты
cargo test --all

# Запуск (когда всё настроено)
cargo run --release -- --config config/devnet.toml

# Проверка Redis
docker exec -it redis redis-cli ping

# Проверка баланса Solana
solana balance
```

---

## 📚 Документация

- [Архитектура](../docs/ARCHITECTURE.md)
- [Tech Stack](../docs/TECH_STACK.md)
- [API Spec](../docs/API_SPEC.md)
- [Implementation Plan](../docs/IMPLEMENTATION_PLAN.md)

---

*Последнее обновление: декабрь 2024*

