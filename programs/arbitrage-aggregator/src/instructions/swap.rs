//! Single swap instruction for testing

use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

use crate::error::AggregatorError;
use crate::state::ProgramConfig;
use crate::DexType;

/// Single swap for testing
#[derive(Accounts)]
pub struct SingleSwap<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        seeds = [ProgramConfig::SEED],
        bump = config.bump,
    )]
    pub config: Account<'info, ProgramConfig>,

    /// Source token account
    #[account(
        mut,
        constraint = source_token_account.owner == authority.key() @ AggregatorError::Unauthorized
    )]
    pub source_token_account: Account<'info, TokenAccount>,

    /// Destination token account
    #[account(
        mut,
        constraint = destination_token_account.owner == authority.key() @ AggregatorError::Unauthorized
    )]
    pub destination_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn single_swap_handler<'info>(
    ctx: Context<'_, '_, '_, 'info, SingleSwap<'info>>,
    dex: DexType,
    amount_in: u64,
    min_amount_out: u64,
) -> Result<()> {
    require!(!ctx.accounts.config.paused, AggregatorError::Paused);
    require!(amount_in > 0, AggregatorError::InvalidSwapAccounts);

    let balance_before = ctx.accounts.destination_token_account.amount;

    msg!(
        "🔄 Single swap via {:?}: {} → min {}",
        dex,
        amount_in,
        min_amount_out
    );

    // Execute swap based on DEX
    match dex {
        DexType::OrcaWhirlpool => {
            msg!("  Executing Orca Whirlpool swap...");
            // CPI call would go here
        }
        DexType::RaydiumClmm => {
            msg!("  Executing Raydium CLMM swap...");
        }
        DexType::RaydiumAmm => {
            msg!("  Executing Raydium AMM swap...");
        }
        DexType::Phoenix => {
            msg!("  Executing Phoenix swap...");
        }
        DexType::OpenBook => {
            msg!("  Executing OpenBook swap...");
        }
    }

    // Reload and verify minimum output
    ctx.accounts.destination_token_account.reload()?;
    let balance_after = ctx.accounts.destination_token_account.amount;
    let received = balance_after.saturating_sub(balance_before);

    require!(
        received >= min_amount_out,
        AggregatorError::SlippageExceeded
    );

    msg!("✅ Swap complete. Received: {}", received);

    Ok(())
}

