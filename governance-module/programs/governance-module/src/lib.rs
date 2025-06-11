use anchor_lang::prelude::*;

declare_id!("HJzW17DkivXRYjirjDD56a3Pve6JFnKhmsfpswJQ3St4");

#[program]
pub mod governance_module {
    use std::f32::consts::E;

    use super::*;

    pub fn create_governance(
        ctx: Context<CreateGovernance>,
        name: String,
        description: String,
        config: GovernanceConfig
    ) -> Result<()> {
        let governance = &mut ctx.accounts.governance;
        let clock = Clock::get()?;

        require!(name.len() <= 64, GovernanceError::NameTooLong);
        require!(description.len() <= 256, GovernanceError::DescriptionTooLong);
        require!(config.quorum_percentage <= 100, GovernanceError::InvalidQuorum);
        require!(config.voting_period_hours > 0, GovernanceError::InvalidVotingPeriod);

        governance.authority = ctx.accounts.authority.key();
        governance.name = name;
        governance.description = description;
        governance.config = config;
        governance.proposal_count = 0;
        governance.total_voting_power = 0;
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

        require!(!governance.paused, GovernanceError::GovernancePaused);

        require!(title.len() <= 128, GovernanceError::TitleTooLong);
        require!(description.len() <= 1024, GovernanceError::DescriptionTooLong);
        require!(!instruction_data.is_empty(), GovernanceError::EmptyInstruction);
        require!(instruction_data.len() <= 1232, GovernanceError::InstructionTooLarge);

        let proposer_voting_power = match governance.config.voting_type {
            VotingType::Equal => 1,
            VotingType::TokenWeighted => {

                governance.config.min_voting_power_to_propose.max(1)
            }
        };

        require!(
            proposer_voting_power >= governance.config.min_voting_power_to_propose,
            GovernanceError::InsufficientVotingPower
        );

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
        proposal.bump = ctx.bumps.proposal;

        governance.proposal_count += 1;

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


    pub fn vote(
        ctx: Context<Vote>,
        proposal_id: u64,
        vote_type: VoteType,
    ) -> Result<()> {
        let governance = &ctx.accounts.governance;
        let proposal = &mut ctx.accounts.proposal;
        let vote_record = &mut ctx.accounts.vote_record;
        let clock = Clock::get()?;

        require!(proposal.status == ProposalStatus::Active, GovernanceError::ProposalNotActive);
        require!(
            clock.unix_timestamp >= proposal.voting_start_time &&
            clock.unix_timestamp <= proposal.voting_end_time,
            GovernanceError::VotingPeriodEnded
        );

        require!(!governance.paused, GovernanceError::GovernancePaused);

        let voting_power = match governance.config.voting_type {
            VotingType::Equal => 1u64,
            VotingType::TokenWeighted => {


                10u64 // Placeholder for token-weighted voting power logic
            }
        };

        require!(voting_power > 0, GovernanceError::NoVotingPower);

        vote_record.governance = governance.key();
        vote_record.proposal_id = proposal_id;
        vote_record.voter = ctx.accounts.voter.key();
        vote_record.vote_type = vote_type.clone();
        vote_record.voting_power = voting_power as i64;
        vote_record.timestamp = clock.unix_timestamp;
        vote_record.bump = ctx.bumps.vote_record;

        match vote_type {
            VoteType::For => proposal.votes_for += voting_power,
            VoteType::Against => proposal.votes_against += voting_power,
            VoteType::Abstain => proposal.votes_abstain += voting_power,
        }

        proposal.unique_voters += 1;

        self::check_and_finalize_proposal(proposal, governance)?;

        emit!(VoteCast {
            governance: governance.key(),
            proposal: proposal.key(),
            voter: vote_record.voter,
            vote_type: vote_record.vote_type.clone(),
            voting_power: voting_power,
        });

        msg!("Vote cast: {:?} with power: {}", vote_type, voting_power);
        Ok(())
    }

    pub fn execute_proposal(
        ctx: Context<ExecuteProposal>,
        proposal_id: u64,
    ) -> Result<()> {
        let governance = &ctx.accounts.governance;
        let proposal = &mut ctx.accounts.proposal;
        let clock = Clock::get()?;

        require!(proposal.status == ProposalStatus::Passed, GovernanceError::ProposalNotPassed);
        require!(!proposal.executed, GovernanceError::AlreadyExecuted);

        let execution_time = proposal.voting_end_time + (proposal.execution_delay_hours as i64 * 3600);

        require!(
            clock.unix_timestamp >= execution_time,
            GovernanceError::ExecutionDelayNotMet
        );

        if !matches!(proposal.proposal_type, ProposalType::Emergency { .. }) {
            require!(!governance.paused, GovernanceError::GovernancePaused);
        }

        match &proposal.proposal_type {
            ProposalType::Treasury { recipient, amount, token_mint } => {
                msg!("Executing treasury proposal: {} tokens to {}", amount, recipient);
            },
            ProposalType::ConfigChange { parameter, new_value } => {
                msg!("Executing config change proposal: {} to {:?}", parameter, new_value);
            },
            ProposalType::Emergency { action } => {
                msg!("Executing emergency action: {:?}", action);
            },
            ProposalType::Custom { target_program, instruction_data } => {
                msg!("Executing custom proposal on program: {} with data: {:?}", target_program, instruction_data);
            },
        }

        proposal.executed = true;
        proposal.execution_time = Some(clock.unix_timestamp);

        emit!(ProposalExecuted {
            governance: governance.key(),
            proposal: proposal.key(),
            proposal_id: proposal.proposal_id,
            executor: ctx.accounts.executor.key(),
        });

        msg!("Proposal {} executed successfully", proposal_id);
        Ok(())
    }
}


#[account]
pub struct Governance {
    pub authority: Pubkey,
    pub name: String,
    pub description: String,
    pub config: GovernanceConfig,
    pub proposal_count: u64,
    pub total_voting_power: u64,
    pub paused: bool,
    pub created_at: i64,
    pub bump: u8,
}

#[account]
pub struct Proposal {
    pub governance: Pubkey,
    pub proposal_id: u64,
    pub proposer: Pubkey,
    pub title: String,
    pub description: String,
    pub proposal_type: ProposalType,
    pub instruction_data: Vec<u8>,


    //Timing
    pub created_at: i64,
    pub voting_start_time: i64,
    pub voting_end_time: i64,
    pub execution_delay_hours: u64,


    //Vote tracking
    pub votes_for: u64,
    pub votes_against: u64,
    pub votes_abstain: u64,
    pub unique_voters: u32,


    //Status
    pub status: ProposalStatus,
    pub executed: bool,
    pub execution_time: Option<i64>,
    pub cancellation_reason: Option<String>,
    pub bump: u8,
}

#[account]
pub struct VoteRecord {
    pub governance: Pubkey,
    pub proposal_id: u64,
    pub voter: Pubkey,
    pub vote_type: VoteType,
    pub voting_power: i64,
    pub timestamp: i64,
    pub delagated_from: Option<Pubkey>,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub struct GovernanceConfig {
    pub voting_type: VotingType,
    pub quorum_percentage: u8,
    pub voting_period_hours: u32,
    pub execution_delay_hours: u32,
    pub min_voting_power_to_propose: u64,
    pub proposal_deposit: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum VotingType {
    Equal,
    TokenWeighted,
}

#[derive(AnchorDeserialize, AnchorSerialize, Clone, PartialEq)]
pub enum ProposalType {
    Treasury {
        recipient: Pubkey,
        amount: u64,
        token_mint: Option<Pubkey>,
    },

    ConfigChange {
        parameter: String,
        new_value: Vec<u8>,
    },

    Emergency {
        action: EmergencyAction,
    },

    Custom {
        target_program: Pubkey,
        instruction_data: Vec<u8>,
    },
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Debug)]
pub enum EmergencyAction {
    Pause,
    Unpause,
    Cancel { proposal_id: u64 },
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum ProposalStatus {
    Active,
    Passed,
    Failed,
    Cancelled,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Debug)]
pub enum VoteType {
    For,
    Against,
    Abstain
}

#[derive(Accounts)]
#[instruction(name: String, description: String)]
pub struct CreateGovernance<'info> {
    #[account(
        mut
    )]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = 8 + 32 + 4 + name.len() + 4 + description.len() + 150 + 8 + 8 + 1 + 8 + 1, // Discriminator + keys + strings + config + counters + flags + timestamps + bump
        seeds = [b"governance", authority.key().as_ref()],
        bump,
    )]
    pub governance: Account<'info, Governance>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(title: String, description: String)]
pub struct CreateProposal<'info> {
    #[account(
        mut
    )]
    pub proposer: Signer<'info>,

    #[account(
        mut
    )]
    pub governance: Account<'info, Governance>,

    #[account(
        init,
        payer = proposer,
        space = 8 +
                32 + 
                8 +
                32 + 
                4 + 
                title.len() + 
                4 + 
                description.len() + 
                150 + 
                1000 + 
                8*4 +
                8*4 +
                4 +
                1+
                1 +
                9 +
                1 +
                4 +
                256 +
                1 ,// Discriminator + keys + strings + proposal type + instruction data + timing + vote tracking + status + execution flags + cancellation reason + bump
        seeds = [b"proposal", governance.key().as_ref(), &governance.proposal_count.to_le_bytes()],
        bump,
    )]
    pub proposal: Account<'info, Proposal>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(proposal_id: u64)]
pub struct Vote<'info> {
    #[account(
        mut
    )]
    pub voter: Signer<'info>,

    pub governance: Account<'info, Governance>,

    #[account(
        mut,
        seeds = [b"proposal", governance.key().as_ref(), &proposal_id.to_le_bytes()],
        bump,
    )]
    pub proposal: Account<'info, Proposal>,

    #[account(
        init,
        payer = voter,
        space = 8 +
                32 +
                8 +
                32 +
                1 +
                8 +
                8 +
                1 +
                32 +
                1,
        seeds = [b"vote", proposal.key().as_ref(), voter.key().as_ref()],
        bump,
    )]
    pub vote_record: Account<'info, VoteRecord>,

    pub system_program: Program<'info, System>,
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

#[derive(Accounts)]
#[instruction(proposal_id: u64)]
pub struct CancelProposal<'info> {
    pub canceller: Signer<'info>,

    pub governance: Account<'info, Governance>,

    #[account(
        mut,
        seeds = [b"proposal", governance.key().as_ref(), &proposal_id.to_le_bytes()],
        bump = proposal.bump,
    )]
    pub proposal: Account<'info, Proposal>,
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
    #[account(mut)]
    pub governance: Account<'info, Governance>,

    #[account(
        seeds = [b"governance", governance.key().as_ref(), &proposal_id.to_le_bytes()],
        bump = governance.bump,
    )]
    pub proposal: Account<'info, Proposal>,
}

#[event]
pub struct GovernanceCreated {
    pub governance: Pubkey,
    pub authority: Pubkey,
    pub name: String,
    pub config: GovernanceConfig,
}

#[event]
pub struct ProposalCreated {
    pub governance: Pubkey,
    pub proposal: Pubkey,
    pub proposal_id: u64,
    pub proposer: Pubkey,
    pub title: String,
    pub proposal_type: ProposalType,
}

#[event]
pub struct VoteCast {
    pub governance: Pubkey,
    pub proposal: Pubkey,
    pub voter: Pubkey,
    pub vote_type: VoteType,
    pub voting_power: u64,
}

#[event]
pub struct ProposalExecuted {
    pub governance: Pubkey,
    pub proposal: Pubkey,
    pub proposal_id: u64,
    pub executor: Pubkey,
}

#[event]
pub struct ProposalCancelled {
    pub governance: Pubkey,
    pub proposal: Pubkey,
    pub proposal_id: u64,
    pub canceller: Pubkey,
    pub reason: String
}

#[event]
pub struct GovernancePaused {
    pub governance: Pubkey,
    pub paused_by: Pubkey,
}

#[event]
pub struct GovernanceUnpaused {
    pub governance: Pubkey,
    pub unpaused_by: Pubkey,
}

#[event]
pub struct GovernanceConfigUpdated {
    pub governance: Pubkey,
    pub proposal_id: u64,
    pub old_config: GovernanceConfig,
    pub new_config: GovernanceConfig,
}

#[error_code]
pub enum GovernanceError {
    #[msg("Name too long")]
    NameTooLong,
    #[msg("Description too long")]
    DescriptionTooLong,
    #[msg("Invalid quorum percentage")]
    InvalidQuorum,
    #[msg("Invalid voting period")]
    InvalidVotingPeriod,
    #[msg("Governance is paused")]
    GovernancePaused,
    #[msg("Title too long")]
    TitleTooLong,
    #[msg("Empty instruction")]
    EmptyInstruction,
    #[msg("Instruction too large")]
    InstructionTooLarge,
    #[msg("Insufficient voting power")]
    InsufficientVotingPower,
    #[msg("Proposal not active")]
    ProposalNotActive,
    #[msg("Voting period ended")]
    VotingPeriodEnded,
    #[msg("No voting power")]
    NoVotingPower,
    #[msg("Proposal not passed")]
    ProposalNotPassed,
    #[msg("Already executed")]
    AlreadyExecuted,
    #[msg("Execution delay not met")]
    ExecutionDelayNotMet,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Reason too long")]
    ReasonTooLong,
    #[msg("Invalid proposal")]
    InvalidProposal,
    #[msg("Proposal not executed")]
    ProposalNotExecuted,
    #[msg("Invalid proposal type")]
    InvalidProposalType,
}