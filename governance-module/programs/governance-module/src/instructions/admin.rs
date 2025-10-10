use anchor_lang::prelude::*;
use crate::errors::*;
use crate::events::*;
use crate::state::*;

pub fn emergency_pause(ctx: Context<EmergencyPause>) -> Result<()> {
    let governance = &mut ctx.accounts.governance;

    require!(
        ctx.accounts.caller.key() == governance.authority,
        GovernanceError::Unauthorized
    );

    governance.paused = true;

    emit!(GovernancePaused {
        governance: governance.key(),
        paused_by: ctx.accounts.caller.key(),
    });

    msg!("Governance paused by authority");
    Ok(())
}

pub fn unpause(ctx: Context<Unpause>) -> Result<()> {
    let governance = &mut ctx.accounts.governance;

    require!(
        ctx.accounts.caller.key() == governance.authority,
        GovernanceError::Unauthorized
    );

    governance.paused = false;

    emit!(GovernanceUnpaused {
        governance: governance.key(),
        unpaused_by: ctx.accounts.caller.key(),
    });

    msg!("Governance unpaused by authority");
    Ok(())
}

pub fn update_governance_config(
    ctx: Context<UpdateGovernanceConfig>,
    proposal_id: u64,
    new_config: GovernanceConfig,
) -> Result<()> {
    let governance = &mut ctx.accounts.governance;
    let proposal = &ctx.accounts.proposal;

    // Validate proposal
    require!(proposal.proposal_id == proposal_id, GovernanceError::InvalidProposal);
    require!(proposal.status == ProposalStatus::Passed, GovernanceError::ProposalNotPassed);
    require!(proposal.executed, GovernanceError::ProposalNotExecuted);
    require!(
        matches!(proposal.proposal_type, ProposalType::ConfigChange { .. }),
        GovernanceError::InvalidProposalType
    );

    // Validate new config
    require!(new_config.quorum_percentage <= 100, GovernanceError::InvalidQuorum);
    require!(new_config.voting_period_hours > 0, GovernanceError::InvalidVotingPeriod);
    require!(new_config.execution_delay_hours >= 1, GovernanceError::InvalidExecutionDelay);
    require!(
        new_config.max_active_proposals > 0 && new_config.max_active_proposals <= 100,
        GovernanceError::InvalidProposalType
    );

    let old_config = governance.config.clone();
    governance.config = new_config;

    emit!(GovernanceConfigUpdated {
        governance: governance.key(),
        proposal_id,
        old_config,
        new_config: governance.config.clone(),
    });

    msg!("Governance configuration updated via proposal {}", proposal_id);
    Ok(())
}

pub fn execute_proposal(
    ctx: Context<ExecuteProposal>,
    _proposal_id: u64,
) -> Result<()> {
    let governance = &ctx.accounts.governance;
    let proposal = &mut ctx.accounts.proposal;
    let clock = Clock::get()?;

    // Validate proposal state
    require!(proposal.status == ProposalStatus::Passed, GovernanceError::ProposalNotPassed);
    require!(!proposal.executed, GovernanceError::AlreadyExecuted);

    // Check execution delay
    let execution_time = proposal.voting_end_time + (proposal.execution_delay_hours as i64 * 3600);
    require!(
        clock.unix_timestamp >= execution_time,
        GovernanceError::ExecutionDelayNotMet
    );

    // Allow emergency proposals to bypass pause
    if !matches!(proposal.proposal_type, ProposalType::Emergency { .. }) {
        require!(!governance.paused, GovernanceError::GovernancePaused);
    }

    // Mark as executed
    proposal.executed = true;
    proposal.execution_time = Some(clock.unix_timestamp);

    emit!(ProposalExecuted {
        governance: governance.key(),
        proposal: proposal.key(),
        proposal_id: proposal.proposal_id,
        executor: ctx.accounts.executor.key(),
    });

    msg!("Proposal {} executed successfully", proposal.proposal_id);
    Ok(())
}

#[derive(Accounts)]
pub struct EmergencyPause<'info> {
    pub caller: Signer<'info>,

    #[account(mut)]
    pub governance: Account<'info, Governance>,
}

#[derive(Accounts)]
pub struct Unpause<'info> {
    pub caller: Signer<'info>,

    #[account(mut)]
    pub governance: Account<'info, Governance>,
}

#[derive(Accounts)]
#[instruction(proposal_id: u64)]
pub struct UpdateGovernanceConfig<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        constraint = authority.key() == governance.authority @ GovernanceError::Unauthorized
    )]
    pub governance: Account<'info, Governance>,

    #[account(
        seeds = [b"proposal", governance.key().as_ref(), &proposal_id.to_le_bytes()],
        bump = proposal.bump,
    )]
    pub proposal: Account<'info, Proposal>,
}

#[derive(Accounts)]
#[instruction(proposal_id: u64)]
pub struct ExecuteProposal<'info> {
    pub executor: Signer<'info>,

    pub governance: Account<'info, Governance>,

    #[account(
        mut,
        seeds = [b"proposal", governance.key().as_ref(), &proposal_id.to_le_bytes()],
        bump = proposal.bump,
    )]
    pub proposal: Account<'info, Proposal>,
}