//! Admin instructions for program configuration

use anchor_lang::prelude::*;
use crate::state::ProgramConfig;
use crate::error::AggregatorError;

/// Initialize program config
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init,
        payer = owner,
        space = ProgramConfig::LEN,
        seeds = [ProgramConfig::SEED],
        bump
    )]
    pub config: Account<'info, ProgramConfig>,

    /// CHECK: Treasury account, validated by owner
    pub treasury: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

/// Update config parameters
#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [ProgramConfig::SEED],
        bump = config.bump,
        has_one = owner @ AggregatorError::Unauthorized,
    )]
    pub config: Account<'info, ProgramConfig>,
}

pub fn initialize(ctx: Context<Initialize>, fee_bps: u16) -> Result<()> {
    let config = &mut ctx.accounts.config;
    
    config.owner = ctx.accounts.owner.key();
    config.treasury = ctx.accounts.treasury.key();
    config.fee_bps = fee_bps;
    config.paused = false;
    config.total_arbitrages = 0;
    config.total_profit = 0;
    config.bump = ctx.bumps.config;

    msg!("Aggregator initialized with fee: {} bps", fee_bps);
    
    Ok(())
}

pub fn update_config(
    ctx: Context<UpdateConfig>,
    new_fee_bps: Option<u16>,
    paused: Option<bool>,
) -> Result<()> {
    let config = &mut ctx.accounts.config;

    if let Some(fee) = new_fee_bps {
        require!(fee <= 1000, AggregatorError::Overflow); // Max 10%
        config.fee_bps = fee;
        msg!("Fee updated to {} bps", fee);
    }

    if let Some(p) = paused {
        config.paused = p;
        msg!("Paused set to {}", p);
    }

    Ok(())
}

