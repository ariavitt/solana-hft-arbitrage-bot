# Deploy Aggregator Checklist

## Что мы уже знаем

- Бот сейчас ходил в `DMCPSH38kwbcXxwyaHdXqEf4JCTdgdTwMJwcEMHPrEqK`
- Проверка mainnet RPC показала, что этого аккаунта в сети нет
- Значит текущий blocker `ProgramAccountNotFound` ожидаем: программы по этому адресу просто нет

## Что нужно сделать дальше

### Вариант 1. Новый деплой на devnet

Используется, если мы хотим сначала проверить честный on-chain path дешево и безопасно.

1. Создать program keypair
2. Получить его pubkey
3. Прописать pubkey в проект
4. Собрать программу
5. Задеплоить на devnet
6. Инициализировать конфиг программы
7. Переключить бота на `config/devnet.toml`

### Вариант 2. Новый деплой на mainnet

Используется, если мы хотим сразу проверить честный mainnet-simulate / mainnet path.

1. Создать program keypair
2. Получить pubkey
3. Синхронизировать `declare_id!`, `Anchor.toml` и runtime-конфиг
4. Собрать программу
5. Задеплоить
6. Инициализировать config PDA
7. Повторить `prepare-accounts`
8. Снова прогнать honest on-chain simulation

## Команды

### 1. Создать program keypair

```bash
mkdir -p target/deploy
solana-keygen new -o target/deploy/arbitrage_aggregator-keypair.json --no-bip39-passphrase
```

### 2. Получить будущий program id

```bash
solana address -k target/deploy/arbitrage_aggregator-keypair.json
```

### 3. Синхронизировать проект под новый program id

Mainnet:

```bash
./scripts/sync_aggregator_program_id.sh mainnet <PROGRAM_ID>
```

Devnet:

```bash
./scripts/sync_aggregator_program_id.sh devnet <PROGRAM_ID>
```

### 4. Собрать программу

```bash
anchor build
```

### 5. Деплой

Devnet:

```bash
anchor deploy --provider.cluster devnet
```

Mainnet:

```bash
anchor deploy --provider.cluster mainnet
```

## Зачем нужен sync script

У программы есть несколько мест, которые должны говорить про один и тот же `program id`:

- `declare_id!` в Rust-контракте
- `Anchor.toml`
- runtime-конфиг бота

Если хотя бы одно место останется старым, мы снова получим путаницу:

- локально собрана одна программа
- бот шлет в другую
- PDA считаются для третьей

`scripts/sync_aggregator_program_id.sh` нужен именно для того, чтобы не разъехались адреса.

## Что считать успешным результатом

После деплоя и инициализации должны выполниться все три пункта:

1. `initialize_aggregator` проходит
2. `prepare-accounts` проходит
3. `trade_onchain_simulation_started` больше не падает сразу на `ProgramAccountNotFound`
