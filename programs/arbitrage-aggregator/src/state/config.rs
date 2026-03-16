//! Program configuration account

use anchor_lang::prelude::*;

/// Глобальная конфигурация программы
#[account]
pub struct ProgramConfig {
    /// Владелец программы (может менять настройки)
    pub owner: Pubkey,

    /// Комиссия протокола в базисных пунктах (100 = 1%)
    /// Берётся с каждого успешного арбитража
    pub fee_bps: u16,

    /// Treasury аккаунт для сбора комиссий
    pub treasury: Pubkey,

    /// Программа приостановлена?
    pub paused: bool,

    /// Счётчик успешных арбитражей
    pub total_arbitrages: u64,

    /// Общий профит (для статистики)
    pub total_profit: u64,

    /// Bump для PDA
    pub bump: u8,

    /// Reserved space for future upgrades (split into smaller arrays)
    pub _reserved1: [u8; 32],
    pub _reserved2: [u8; 32],
}

impl Default for ProgramConfig {
    fn default() -> Self {
        Self {
            owner: Pubkey::default(),
            fee_bps: 0,
            treasury: Pubkey::default(),
            paused: false,
            total_arbitrages: 0,
            total_profit: 0,
            bump: 0,
            _reserved1: [0u8; 32],
            _reserved2: [0u8; 32],
        }
    }
}

impl ProgramConfig {
    pub const LEN: usize = 8 + // discriminator
        32 + // owner
        2 + // fee_bps
        32 + // treasury
        1 + // paused
        8 + // total_arbitrages
        8 + // total_profit
        1 + // bump
        32 + // reserved1
        32; // reserved2

    pub const SEED: &'static [u8] = b"config";
}

/// Статистика по конкретному пользователю (опционально)
#[account]
#[derive(Default)]
pub struct UserStats {
    /// Владелец статистики
    pub authority: Pubkey,

    /// Количество арбитражей
    pub arbitrage_count: u64,

    /// Общий профит
    pub total_profit: u64,

    /// Последний арбитраж (slot)
    pub last_arbitrage_slot: u64,

    /// Bump
    pub bump: u8,
}

impl UserStats {
    pub const LEN: usize = 8 + 32 + 8 + 8 + 8 + 1;
    pub const SEED: &'static [u8] = b"user_stats";
}

