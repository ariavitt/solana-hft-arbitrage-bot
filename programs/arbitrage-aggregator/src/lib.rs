//! # Arbitrage Aggregator
//!
//! Смарт-контракт для атомарного выполнения арбитражных свопов
//! с защитой от проскальзывания и проверкой профита on-chain.
//!
//! ## Возможности
//!
//! - Multi-hop свопы через CPI к DEX (Orca, Raydium, Phoenix)
//! - Atomic profit check - откат при отсутствии профита
//! - Slippage protection на каждом шаге
//! - События для off-chain мониторинга

use anchor_lang::prelude::*;

pub mod cpi;
pub mod error;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("DMCPSH38kwbcXxwyaHdXqEf4JCTdgdTwMJwcEMHPrEqK");

#[program]
pub mod arbitrage_aggregator {
    use super::*;

    /// Инициализация конфигурации программы
    /// Вызывается один раз при деплое
    pub fn initialize(ctx: Context<Initialize>, fee_bps: u16) -> Result<()> {
        instructions::admin::initialize(ctx, fee_bps)
    }

    /// Обновление параметров конфигурации (только owner)
    pub fn update_config(
        ctx: Context<UpdateConfig>,
        new_fee_bps: Option<u16>,
        paused: Option<bool>,
    ) -> Result<()> {
        instructions::admin::update_config(ctx, new_fee_bps, paused)
    }

    /// Выполнение арбитражного маршрута
    ///
    /// # Параметры
    /// - `route` - список свопов для выполнения
    /// - `min_profit` - минимальный профит в базовом токене
    /// - `max_slippage_bps` - максимальный slippage в базисных пунктах (100 = 1%)
    ///
    /// # Логика
    /// 1. Записывает баланс базового токена ДО свопов
    /// 2. Выполняет все свопы через CPI
    /// 3. Проверяет баланс ПОСЛЕ
    /// 4. Если profit < min_profit → REVERT
    pub fn execute_arbitrage<'info>(
        ctx: Context<'_, '_, '_, 'info, ExecuteArbitrage<'info>>,
        route: Vec<SwapLeg>,
        min_profit: u64,
        max_slippage_bps: u16,
    ) -> Result<()> {
        instructions::execute::handler(ctx, route, min_profit, max_slippage_bps)
    }

    /// Простой своп через один DEX (для тестирования)
    pub fn single_swap<'info>(
        ctx: Context<'_, '_, '_, 'info, SingleSwap<'info>>,
        dex: DexType,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<()> {
        instructions::swap::single_swap_handler(ctx, dex, amount_in, min_amount_out)
    }
}

/// Тип DEX для маршрутизации
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum DexType {
    /// Orca Whirlpools (Concentrated Liquidity)
    OrcaWhirlpool,
    /// Raydium CLMM
    RaydiumClmm,
    /// Raydium AMM (Classic)
    RaydiumAmm,
    /// Phoenix (Order Book)
    Phoenix,
    /// OpenBook (Order Book)
    OpenBook,
}

/// Один шаг в арбитражном маршруте
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct SwapLeg {
    /// Тип DEX
    pub dex: DexType,
    /// Адрес пула
    pub pool: Pubkey,
    /// Входящий токен
    pub token_in: Pubkey,
    /// Исходящий токен
    pub token_out: Pubkey,
    /// Количество на входе
    pub amount_in: u64,
    /// Минимум на выходе (slippage protection)
    pub min_amount_out: u64,
    /// A to B direction (для CLMM)
    pub a_to_b: bool,
}

/// Событие успешного арбитража
#[event]
pub struct ArbitrageExecuted {
    /// Кто выполнил
    pub authority: Pubkey,
    /// Профит в базовом токене
    pub profit: u64,
    /// Количество хопов
    pub route_length: u8,
    /// Timestamp
    pub timestamp: i64,
    /// Slot
    pub slot: u64,
}

/// Событие неудачного арбитража (для аналитики)
#[event]
pub struct ArbitrageFailed {
    pub authority: Pubkey,
    pub expected_profit: u64,
    pub actual_profit: i64,
    pub reason: String,
}

