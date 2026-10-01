# Mainnet Launch Plan

## Что у нас уже готово

- Живой devnet aggregator уже задеплоен и инициализирован
- Runtime-переключение `aggregator.program_id` уже работает
- Есть отдельный торговый кошелек:
  - `config/wallets/trading-mainnet.json`
- Есть отдельный deployer-кошелек для mainnet:
  - `config/wallets/deployer-mainnet.json`
  - pubkey: `EpV63UBtyw4ZwEGG4t2AKwEJzuN4ULY4XuwuwLa3Gkv7`
- Есть отдельный deploy-конфиг:
  - [config/mainnet-deploy.toml](/Users/polinababijcuk/Documents/New%20project/config/mainnet-deploy.toml)

## Зачем разделять deployer и trading wallet

Это нужно по двум причинам:

1. `upgrade authority` программы и торговый кошелек не должны быть одним и тем же ключом
2. деплой и инициализация программы - это редкие административные операции, а торговля - постоянная операционная деятельность

Так безопаснее и проще сопровождать.

## Что еще нужно до mainnet deploy

### 1. Пополнить mainnet deployer

Кошелек:

- `EpV63UBtyw4ZwEGG4t2AKwEJzuN4ULY4XuwuwLa3Gkv7`

Практический минимум:

- около `2.0 SOL` на сам deploy
- лучше держать `2.3-2.5 SOL`, чтобы спокойно покрыть:
  - rent на программу / буфер
  - комиссии
  - повторный деплой при неудачной первой попытке
  - инициализацию программы

### 2. Создать mainnet program keypair

После пополнения мы создаем отдельный `program keypair` для mainnet-деплоя и получаем новый `program id`.

### 3. Синхронизировать новый `program id`

После получения адреса новой программы нужно обновить:

- `declare_id!`
- `Anchor.toml`
- `config/mainnet.toml`
- `config/mainnet-simulate.toml`
- `config/mainnet-deploy.toml`

Часть этой работы уже автоматизирована через:

- [scripts/sync_aggregator_program_id.sh](/Users/polinababijcuk/Documents/New%20project/scripts/sync_aggregator_program_id.sh)

## Порядок запуска

1. Пополнить deployer-кошелек
2. Создать mainnet `program keypair`
3. Синхронизировать `program id`
4. Собрать программу
5. Задеплоить на mainnet
6. Инициализировать агрегатор через `config/mainnet-deploy.toml`
7. Переключить `mainnet-simulate` на новый `program id`
8. Прогнать честную on-chain simulation
9. Только потом возвращаться к торговому контуру

## Что считать успехом

Mainnet launch считается подготовленным, когда:

1. программа существует в mainnet
2. `initialize_aggregator` проходит
3. `mainnet-simulate` больше не падает сразу на `ProgramAccountNotFound`
