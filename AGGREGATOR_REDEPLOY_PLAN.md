# Aggregator Redeploy Plan

## Что это такое

`Aggregator program` - это твой on-chain контракт в сети Solana. Бот off-chain не делает multi-hop swap сам по себе: он только собирает транзакцию и отправляет ее в этот контракт, а уже контракт делает CPI-вызовы в DEX-пулы.

У каждой такой программы есть свой уникальный `program id`. Для текущего mainnet-simulate пути это:

- `DMCPSH38kwbcXxwyaHdXqEf4JCTdgdTwMJwcEMHPrEqK`

## Почему это важно

Локальные правки в `programs/arbitrage-aggregator/...` не меняют поведение программы, которая уже задеплоена в сети.

Пока бот отправляет транзакции в старый `program id`, он продолжает общаться со старой версией контракта, даже если локальный исходник уже исправлен.

Именно поэтому у нас появился честный разрыв:

- off-chain код уже продвинут дальше
- локальный исходник агрегатора уже исправляется
- но mainnet-simulate все еще упирается в поведение старой on-chain версии

## Что я уже подготовил

Теперь `program id` агрегатора больше не захардкожен только в `tx-builder`.

Он вынесен в runtime-конфиг:

- [mainnet-simulate.toml](/Users/polinababijcuk/Documents/New%20project/config/mainnet-simulate.toml)
- [mainnet.toml](/Users/polinababijcuk/Documents/New%20project/config/mainnet.toml)
- [devnet.toml](/Users/polinababijcuk/Documents/New%20project/config/devnet.toml)

Это значит, что после нового деплоя мы сможем:

1. получить новый `program id`
2. прописать его в конфиг
3. сразу переключить бота на новую on-chain версию без переписывания остального контура

## Что нужно для полного перехода

1. Проверить, можно ли upgrade-нуть текущую программу.
2. Если нельзя, задеплоить новую копию агрегатора.
3. Синхронизировать `declare_id!` в `programs/arbitrage-aggregator/src/lib.rs` с новым `program id`.
4. Обновить `Anchor.toml`.
5. Обновить `aggregator.program_id` в нужном runtime-конфиге.
6. Повторить `initialize_aggregator`, `prepare-accounts` и честную on-chain simulation.

## Разница между upgrade и redeploy

`Upgrade`:

- тот же `program id`
- новая версия кода поверх старой программы
- возможно только если программа была задеплоена как upgradeable и есть upgrade authority

`Redeploy`:

- новый `program id`
- по сути новая программа в сети
- потом бота надо переключить на этот новый адрес

## Почему это нам нужно прямо сейчас

Текущий следующий блокер:

- `ProgramAccountNotFound` на honest on-chain simulation

С высокой вероятностью это уже не просто off-chain wiring, а расхождение между тем, что исправлено локально, и тем, что реально живет в задеплоенной программе.
