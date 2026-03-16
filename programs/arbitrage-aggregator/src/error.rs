//! Custom error types for the aggregator

use anchor_lang::prelude::*;

#[error_code]
pub enum AggregatorError {
    /// Профит меньше минимального - транзакция отклонена
    #[msg("Insufficient profit - transaction reverted to protect from loss")]
    InsufficientProfit,

    /// Slippage превысил максимально допустимый
    #[msg("Slippage exceeded maximum allowed")]
    SlippageExceeded,

    /// Маршрут должен содержать минимум 2 свопа для арбитража
    #[msg("Invalid route - must have at least 2 legs for arbitrage")]
    InvalidRoute,

    /// Маршрут слишком длинный (лимит compute units)
    #[msg("Route too long - maximum 4 hops allowed")]
    RouteTooLong,

    /// Маршрут не замкнут (начальный токен != конечный)
    #[msg("Route must start and end with the same token")]
    RouteNotClosed,

    /// Арифметическое переполнение
    #[msg("Arithmetic overflow")]
    Overflow,

    /// Арифметическое переполнение вниз
    #[msg("Arithmetic underflow")]
    Underflow,

    /// Неавторизованный доступ
    #[msg("Unauthorized - only owner can perform this action")]
    Unauthorized,

    /// Программа приостановлена
    #[msg("Program is paused")]
    Paused,

    /// Неподдерживаемый DEX
    #[msg("Unsupported DEX type")]
    UnsupportedDex,

    /// Неверные аккаунты для свопа
    #[msg("Invalid accounts for swap")]
    InvalidSwapAccounts,

    /// Недостаточно remaining accounts
    #[msg("Not enough remaining accounts provided")]
    NotEnoughAccounts,

    /// CPI вызов не удался
    #[msg("CPI call to DEX failed")]
    CpiCallFailed,

    /// Неверный токен
    #[msg("Invalid token mint")]
    InvalidTokenMint,

    /// Недостаточный баланс
    #[msg("Insufficient balance")]
    InsufficientBalance,
}

