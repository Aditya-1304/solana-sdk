use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenAccount;
use crate::errors::*;
use crate::events::*;
use crate::state::*;
use crate::utils::*;

pub fn create_proposal(
    ctx: Context<CreateProposal>,
    title: String,
    description: String,
    proposal_type: ProposalType,
    instruction_data: Vec<u8>,
) -> Result<()> {
    let governance = &mut ctx.accounts.governance;
    let proposal = &mut ctx.accounts.proposal;
    let clock = Clock::get()?;

    // Check governance state
    require!(!governance.paused, GovernanceError::GovernancePaused);
    require!(
        governance.active_proposals < governance.config.max_active_proposals,
        GovernanceError::ProposalQueueFull
    );

    // Validate inputs
    require!(title.len() <= Proposal::MAX_TITLE_LEN, GovernanceError::TitleTooLong);
    require!(description.len() <= Proposal::MAX_DESC_LEN, GovernanceError::DescriptionTooLong);
    require!(!instruction_data.is_empty(), GovernanceError::EmptyInstruction);
    require!(instruction_data.len() <= Proposal::MAX_INSTRUCTION_LEN, GovernanceError::InstructionTooLarge);

    // Check proposer voting power
    let proposer_voting_power = get_voting_power(
        &ctx.accounts.proposer.key(),
        &governance.config,
        ctx.accounts.proposer_token_account.as_ref(),
    )?;

    require!(
        proposer_voting_power >= governance.config.min_voting_power_to_propose,
        GovernanceError::InsufficientVotingPower
    );

    // Validate proposal type
    validate_proposal_type(&proposal_type, &instruction_data)?;

    // Initialize proposal
    proposal.governance = governance.key();
    proposal.proposal_id = governance.proposal_count;
    proposal.proposer = ctx.accounts.proposer.key();
    proposal.title = title;
    proposal.description = description;
    proposal.proposal_type = proposal_type;
    proposal.instruction_data = instruction_data;
    
    proposal.created_at = clock.unix_timestamp;
    proposal.voting_start_time = clock.unix_timestamp;
    proposal.voting_end_time = clock.unix_timestamp + (governance.config.voting_period_hours as i64 * 3600);
    proposal.execution_delay_hours = governance.config.execution_delay_hours as u64;
    
    proposal.votes_for = 0;
    proposal.votes_against = 0;
    proposal.votes_abstain = 0;
    proposal.unique_voters = 0;
    
    proposal.status = ProposalStatus::Active;
    proposal.executed = false;
    proposal.execution_time = None;
    proposal.cancellation_reason = None;
    proposal.bump = ctx.bumps.proposal;

    // Update governance state
    governance.proposal_count += 1;
    governance.active_proposals += 1;

    emit!(ProposalCreated {
        governance: governance.key(),
        proposal: proposal.key(),
        proposal_id: proposal.proposal_id,
        proposer: proposal.proposer,
        title: proposal.title.clone(),
        proposal_type: proposal.proposal_type.clone(),
    });

    msg!("Proposal '{}' created with ID: {}", proposal.title, proposal.proposal_id);
    Ok(())
}

pub fn cancel_proposal(
    ctx: Context<CancelProposal>,
    _proposal_id: u64,
    reason: String,
) -> Result<()> {
    let governance = &ctx.accounts.governance;
    let proposal = &mut ctx.accounts.proposal;

    require!(
        ctx.accounts.canceller.key() == governance.authority || 
        ctx.accounts.canceller.key() == proposal.proposer,
        GovernanceError::Unauthorized
    );

    require!(
        proposal.status == ProposalStatus::Active,
        GovernanceError::ProposalNotActive
    );

    require!(reason.len() <= Proposal::MAX_CANCELLATION_REASON_LEN, GovernanceError::ReasonTooLong);

    proposal.status = ProposalStatus::Cancelled;
    proposal.cancellation_reason = Some(reason.clone());

    emit!(ProposalCancelled {
        governance: governance.key(),
        proposal: proposal.key(),
        proposal_id: proposal.proposal_id,
        canceller: ctx.accounts.canceller.key(),
        reason: reason.clone(),
    });

    msg!("Proposal {} cancelled: {}", proposal.proposal_id, reason);
    Ok(())
}

#[derive(Accounts)]
#[instruction(title: String, description: String)]
pub struct CreateProposal<'info> {
    #[account(mut)]
    pub proposer: Signer<'info>,

    #[account(mut)]
    pub governance: Account<'info, Governance>,

    #[account(
        init,
        payer = proposer,
        space = Proposal::space(&title, &description),
        seeds = [b"proposal", governance.key().as_ref(), &governance.proposal_count.to_le_bytes()],
        bump,
    )]
    pub proposal: Account<'info, Proposal>,

    pub proposer_token_account: Option<InterfaceAccount<'info, TokenAccount>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(proposal_id: u64)]
pub struct CancelProposal<'info> {
    pub canceller: Signer<'info>,

    #[account(mut)]
    pub governance: Account<'info, Governance>,

    #[account(
        mut,
        seeds = [b"proposal", governance.key().as_ref(), &proposal_id.to_le_bytes()],
        bump = proposal.bump,
    )]
    pub proposal: Account<'info, Proposal>,
}