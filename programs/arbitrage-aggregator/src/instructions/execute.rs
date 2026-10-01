//! Execute arbitrage instruction - main business logic

use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

use crate::error::AggregatorError;
use crate::state::ProgramConfig;
use crate::{ArbitrageExecuted, DexType, SwapLeg};

/// Execute arbitrage accounts
#[derive(Accounts)]
pub struct ExecuteArbitrage<'info> {
    /// Authority executing the arbitrage
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Program config
    #[account(
        mut,
        seeds = [ProgramConfig::SEED],
        bump = config.bump,
    )]
    pub config: Account<'info, ProgramConfig>,

    /// Base token account (start and end of arbitrage)
    /// This is where we measure profit
    #[account(
        mut,
        constraint = base_token_account.owner == authority.key() @ AggregatorError::Unauthorized
    )]
    pub base_token_account: Account<'info, TokenAccount>,

    /// Token program
    pub token_program: Program<'info, Token>,

    /// System program (for potential account creation)
    pub system_program: Program<'info, System>,

    // Remaining accounts:
    // For each swap leg, we need:
    // - Pool account
    // - Token accounts (source, destination)
    // - DEX program
    // - Additional accounts required by DEX
}

pub fn handler<'info>(
    ctx: Context<'_, '_, '_, 'info, ExecuteArbitrage<'info>>,
    route: Vec<SwapLeg>,
    min_profit: u64,
    max_slippage_bps: u16,
) -> Result<()> {
    let config = &ctx.accounts.config;

    // Check if program is paused
    require!(!config.paused, AggregatorError::Paused);

    // Validate route
    require!(route.len() >= 2, AggregatorError::InvalidRoute);
    require!(route.len() <= 4, AggregatorError::RouteTooLong);

    // Verify route is closed (starts and ends with same token)
    let first_token = route.first().unwrap().token_in;
    let last_token = route.last().unwrap().token_out;
    require!(first_token == last_token, AggregatorError::RouteNotClosed);

    // ============================================
    // 1. Record balance BEFORE
    // ============================================
    let balance_before = ctx.accounts.base_token_account.amount;
    msg!("💰 Balance BEFORE: {}", balance_before);

    // ============================================
    // 2. Execute each swap leg
    // ============================================
    let remaining_accounts = ctx.remaining_accounts;
    let mut account_index = 0;

    for (i, leg) in route.iter().enumerate() {
        msg!(
            "🔄 Executing leg {}: {:?} | {} → {} | amount: {}",
            i,
            leg.dex,
            leg.token_in,
            leg.token_out,
            leg.amount_in
        );

        // Execute swap based on DEX type
        account_index = execute_swap_leg(
            &ctx,
            leg,
            remaining_accounts,
            account_index,
            max_slippage_bps,
        )?;
    }

    // ============================================
    // 3. Reload and check balance AFTER
    // ============================================
    ctx.accounts.base_token_account.reload()?;
    let balance_after = ctx.accounts.base_token_account.amount;
    msg!("💰 Balance AFTER: {}", balance_after);

    // ============================================
    // 4. Calculate profit
    // ============================================
    let profit = balance_after
        .checked_sub(balance_before)
        .ok_or(AggregatorError::Underflow)?;

    msg!("📊 Profit: {} (minimum required: {})", profit, min_profit);

    // ============================================
    // 5. CRITICAL: Profit check
    // If no profit → REVERT entire transaction
    // ============================================
    require!(
        profit >= min_profit,
        AggregatorError::InsufficientProfit
    );

    // ============================================
    // 6. Update statistics (done through mut borrow above)
    // ============================================
    // Note: Statistics update would be handled in production

    // ============================================
    // 7. Emit success event
    // ============================================
    let clock = Clock::get()?;
    emit!(ArbitrageExecuted {
        authority: ctx.accounts.authority.key(),
        profit,
        route_length: route.len() as u8,
        timestamp: clock.unix_timestamp,
        slot: clock.slot,
    });

    msg!("✅ Arbitrage successful! Profit: {}", profit);

    Ok(())
}

/// Execute a single swap leg via CPI
fn execute_swap_leg<'info>(
    ctx: &Context<'_, '_, '_, 'info, ExecuteArbitrage<'info>>,
    leg: &SwapLeg,
    remaining_accounts: &[AccountInfo<'info>],
    start_index: usize,
    _max_slippage_bps: u16,
) -> Result<usize> {
    match leg.dex {
        DexType::OrcaWhirlpool => {
            execute_orca_swap(ctx, leg, remaining_accounts, start_index)
        }
        DexType::RaydiumClmm => {
            execute_raydium_clmm_swap(ctx, leg, remaining_accounts, start_index)
        }
        DexType::RaydiumAmm => {
            execute_raydium_amm_swap(ctx, leg, remaining_accounts, start_index)
        }
        DexType::Phoenix => {
            execute_phoenix_swap(ctx, leg, remaining_accounts, start_index)
        }
        DexType::OpenBook => {
            execute_openbook_swap(ctx, leg, remaining_accounts, start_index)
        }
    }
}

// ============================================
// CPI implementations for each DEX
// ============================================

/// Orca Whirlpool swap via CPI
fn execute_orca_swap<'info>(
    ctx: &Context<'_, '_, '_, 'info, ExecuteArbitrage<'info>>,
    leg: &SwapLeg,
    remaining_accounts: &[AccountInfo<'info>],
    start_index: usize,
) -> Result<usize> {
    msg!("  → Orca Whirlpool swap: {} → min {}", leg.amount_in, leg.min_amount_out);
    
    crate::cpi::orca::swap_cpi(
        &ctx.accounts.token_program.to_account_info(),
        &ctx.accounts.authority.to_account_info(),
        remaining_accounts,
        start_index,
        leg.amount_in,
        leg.min_amount_out,
        leg.a_to_b,
    )
}

/// Raydium CLMM swap via CPI
fn execute_raydium_clmm_swap<'info>(
    _ctx: &Context<'_, '_, '_, 'info, ExecuteArbitrage<'info>>,
    leg: &SwapLeg,
    remaining_accounts: &[AccountInfo<'info>],
    start_index: usize,
) -> Result<usize> {
    msg!("  → Raydium CLMM swap: {} → min {}", leg.amount_in, leg.min_amount_out);

    crate::cpi::raydium::swap_cpi(
        remaining_accounts,
        start_index,
        leg.amount_in,
        leg.min_amount_out,
        leg.a_to_b,
    )
}

/// Raydium AMM (v4) swap via CPI
fn execute_raydium_amm_swap<'info>(
    _ctx: &Context<'_, '_, '_, 'info, ExecuteArbitrage<'info>>,
    leg: &SwapLeg,
    remaining_accounts: &[AccountInfo<'info>],
    start_index: usize,
) -> Result<usize> {
    const RAYDIUM_AMM_ACCOUNTS_NEEDED: usize = 17;
    
    require!(
        remaining_accounts.len() >= start_index + RAYDIUM_AMM_ACCOUNTS_NEEDED,
        AggregatorError::NotEnoughAccounts
    );

    msg!("  → Raydium AMM swap: {} → min {}", leg.amount_in, leg.min_amount_out);

    // TODO: Implement CPI to Raydium AMM

    Ok(start_index + RAYDIUM_AMM_ACCOUNTS_NEEDED)
}

/// Phoenix swap via CPI
fn execute_phoenix_swap<'info>(
    _ctx: &Context<'_, '_, '_, 'info, ExecuteArbitrage<'info>>,
    leg: &SwapLeg,
    remaining_accounts: &[AccountInfo<'info>],
    start_index: usize,
) -> Result<usize> {
    const PHOENIX_ACCOUNTS_NEEDED: usize = 10;
    
    require!(
        remaining_accounts.len() >= start_index + PHOENIX_ACCOUNTS_NEEDED,
        AggregatorError::NotEnoughAccounts
    );

    msg!("  → Phoenix swap: {} → min {}", leg.amount_in, leg.min_amount_out);

    // TODO: Implement CPI to Phoenix

    Ok(start_index + PHOENIX_ACCOUNTS_NEEDED)
}

/// OpenBook swap via CPI
fn execute_openbook_swap<'info>(
    _ctx: &Context<'_, '_, '_, 'info, ExecuteArbitrage<'info>>,
    leg: &SwapLeg,
    remaining_accounts: &[AccountInfo<'info>],
    start_index: usize,
) -> Result<usize> {
    const OPENBOOK_ACCOUNTS_NEEDED: usize = 12;
    
    require!(
        remaining_accounts.len() >= start_index + OPENBOOK_ACCOUNTS_NEEDED,
        AggregatorError::NotEnoughAccounts
    );

    msg!("  → OpenBook swap: {} → min {}", leg.amount_in, leg.min_amount_out);

    // TODO: Implement CPI to OpenBook

    Ok(start_index + OPENBOOK_ACCOUNTS_NEEDED)
}
