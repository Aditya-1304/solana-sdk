use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenAccount;
use crate::errors::*;
use crate::events::*;
use crate::state::*;
use crate::utils::*;

pub fn delegate_voting_power(
    ctx: Context<DelegateVotingPower>,
    amount: u64,
) -> Result<()> {
    let delegator = &ctx.accounts.delegator;
    let delegate = &ctx.accounts.delegate;
    let delegation = &mut ctx.accounts.delegation;
    let governance = &ctx.accounts.governance;
    let clock = Clock::get()?;

    require!(
        delegator.key() != delegate.key(),
        GovernanceError::CannotDelegateToSelf
    );

    require!(amount > 0, GovernanceError::NoVotingPower);

    // Verify delegator has sufficient voting power
    let delegator_power = get_voting_power(
        &delegator.key(),
        &governance.config,
        ctx.accounts.delegator_token_account.as_ref(),
    )?;

    require!(
        delegator_power >= amount,
        GovernanceError::InsufficientVotingPower
    );

    delegation.delegator = delegator.key();
    delegation.delegate = delegate.key();
    delegation.amount = amount;
    delegation.created_at = clock.unix_timestamp;
    delegation.active = true;
    delegation.bump = ctx.bumps.delegation;

    emit!(VotingPowerDelegated {
        delegator: delegator.key(),
        delegate: delegate.key(),
        amount,
        timestamp: clock.unix_timestamp,
    });

    msg!("Delegated {} voting power from {} to {}", amount, delegator.key(), delegate.key());
    Ok(())
}

pub fn revoke_delegation(ctx: Context<RevokeDelegation>) -> Result<()> {
    let delegation = &mut ctx.accounts.delegation;
    let clock = Clock::get()?;

    require!(
        ctx.accounts.delegator.key() == delegation.delegator,
        GovernanceError::Unauthorized
    );

    delegation.active = false;

    emit!(DelegationRevoked {
        delegator: delegation.delegator,
        delegate: delegation.delegate,
        timestamp: clock.unix_timestamp,
    });

    msg!("Delegation revoked from {} to {}", delegation.delegator, delegation.delegate);
    Ok(())
}

#[derive(Accounts)]
pub struct DelegateVotingPower<'info> {
    #[account(mut)]
    pub delegator: Signer<'info>,

    /// CHECK: Delegate account
    pub delegate: AccountInfo<'info>,

    pub governance: Account<'info, Governance>,

    #[account(
        init,
        payer = delegator,
        space = VotingDelegation::SPACE,
        seeds = [b"delegation", delegator.key().as_ref(), delegate.key().as_ref()],
        bump,
    )]
    pub delegation: Account<'info, VotingDelegation>,

    pub delegator_token_account: Option<InterfaceAccount<'info, TokenAccount>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RevokeDelegation<'info> {
    #[account(mut)]
    pub delegator: Signer<'info>,

    #[account(
        mut,
        seeds = [b"delegation", delegator.key().as_ref(), delegation.delegate.as_ref()],
        bump = delegation.bump,
        close = delegator,
    )]
    pub delegation: Account<'info, VotingDelegation>,
}