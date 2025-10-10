use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;
use crate::errors::*;
use crate::events::*;
use crate::state::*;

pub fn create_governance(
    ctx: Context<CreateGovernance>,
    name: String,
    description: String,
    config: GovernanceConfig,
) -> Result<()> {
    let governance = &mut ctx.accounts.governance;
    let clock = Clock::get()?;

    // Validate inputs
    require!(name.len() <= Governance::MAX_NAME_LEN, GovernanceError::NameTooLong);
    require!(description.len() <= Governance::MAX_DESC_LEN, GovernanceError::DescriptionTooLong);
    require!(config.quorum_percentage <= 100, GovernanceError::InvalidQuorum);
    require!(config.voting_period_hours > 0, GovernanceError::InvalidVotingPeriod);
    require!(config.execution_delay_hours >= 1, GovernanceError::InvalidExecutionDelay);
    require!(config.max_active_proposals > 0 && config.max_active_proposals <= 100, GovernanceError::InvalidProposalType);

    // Validate governance token if specified
    if let Some(governance_token) = config.governance_token {
        require!(
            ctx.accounts.governance_token.is_some(),
            GovernanceError::MissingGovernanceToken
        );
        require!(
            ctx.accounts.governance_token.as_ref().unwrap().key() == governance_token,
            GovernanceError::InvalidGovernanceToken
        );
    }

    // Initialize governance
    governance.authority = ctx.accounts.authority.key();
    governance.name = name;
    governance.description = description;
    governance.config = config;
    governance.proposal_count = 0;
    governance.total_voting_power = 0;
    governance.active_proposals = 0;
    governance.paused = false;
    governance.created_at = clock.unix_timestamp;
    governance.bump = ctx.bumps.governance;

    emit!(GovernanceCreated {
        governance: governance.key(),
        authority: governance.authority,
        name: governance.name.clone(),
        config: governance.config.clone(),
    });

    msg!("Governance '{}' created successfully!", governance.name);
    Ok(())
}

#[derive(Accounts)]
#[instruction(name: String, description: String)]
pub struct CreateGovernance<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = Governance::space(&name, &description),
        seeds = [b"governance", authority.key().as_ref()],
        bump,
    )]
    pub governance: Account<'info, Governance>,

    pub governance_token: Option<InterfaceAccount<'info, Mint>>,
    pub system_program: Program<'info, System>,
}