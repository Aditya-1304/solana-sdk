use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenAccount;
use crate::errors::*;
use crate::events::*;
use crate::state::*;
use crate::utils::*;

pub fn vote(
    ctx: Context<Vote>,
    _proposal_id: u64,
    vote_type: VoteType,
) -> Result<()> {
    let governance = &ctx.accounts.governance;
    let proposal = &mut ctx.accounts.proposal;
    let vote_record = &mut ctx.accounts.vote_record;
    let clock = Clock::get()?;

    // Validate proposal state
    require!(proposal.status == ProposalStatus::Active, GovernanceError::ProposalNotActive);
    require!(
        clock.unix_timestamp >= proposal.voting_start_time &&
        clock.unix_timestamp <= proposal.voting_end_time,
        GovernanceError::VotingPeriodEnded
    );
    require!(!governance.paused, GovernanceError::GovernancePaused);

    // Get voting power (includes delegated power)
    let mut voting_power = get_voting_power(
        &ctx.accounts.voter.key(),
        &governance.config,
        ctx.accounts.voter_token_account.as_ref(),
    )?;

    // Add delegated voting power
    let delegated_from = if let Some(delegation_account) = &ctx.accounts.delegation {
        if delegation_account.active && delegation_account.delegate == ctx.accounts.voter.key() {
            voting_power = voting_power.checked_add(delegation_account.amount)
                .ok_or(GovernanceError::VotingPowerOverflow)?;
            Some(delegation_account.delegator)
        } else {
            None
        }
    } else {
        None
    };

    require!(voting_power > 0, GovernanceError::NoVotingPower);

    // Record vote
    vote_record.governance = governance.key();
    vote_record.proposal_id = proposal.proposal_id;
    vote_record.voter = ctx.accounts.voter.key();
    vote_record.vote_type = vote_type.clone();
    vote_record.voting_power = voting_power;
    vote_record.timestamp = clock.unix_timestamp;
    vote_record.delegated_from = delegated_from;
    vote_record.bump = ctx.bumps.vote_record;

    // Update proposal vote counts
    match vote_type {
        VoteType::For => proposal.votes_for += voting_power,
        VoteType::Against => proposal.votes_against += voting_power,
        VoteType::Abstain => proposal.votes_abstain += voting_power,
    }

    proposal.unique_voters += 1;

    // Check if proposal can be finalized
    check_and_finalize_proposal(proposal, governance)?;

    emit!(VoteCast {
        governance: governance.key(),
        proposal: proposal.key(),
        voter: vote_record.voter,
        vote_type: vote_record.vote_type.clone(),
        voting_power,
    });

    msg!("Vote cast: {:?} with power: {}", vote_type, voting_power);
    Ok(())
}

#[derive(Accounts)]
#[instruction(proposal_id: u64)]
pub struct Vote<'info> {
    #[account(mut)]
    pub voter: Signer<'info>,

    pub governance: Account<'info, Governance>,

    #[account(
        mut,
        seeds = [b"proposal", governance.key().as_ref(), &proposal_id.to_le_bytes()],
        bump = proposal.bump,
    )]
    pub proposal: Account<'info, Proposal>,

    #[account(
        init,
        payer = voter,
        space = VoteRecord::SPACE,
        seeds = [b"vote", proposal.key().as_ref(), voter.key().as_ref()],
        bump,
    )]
    pub vote_record: Account<'info, VoteRecord>,

    pub voter_token_account: Option<InterfaceAccount<'info, TokenAccount>>,

    /// Optional delegation account if voter is voting with delegated power
    pub delegation: Option<Account<'info, VotingDelegation>>,

    pub system_program: Program<'info, System>,
}