use anchor_lang::prelude::*;

declare_id!("HJzW17DkivXRYjirjDD56a3Pve6JFnKhmsfpswJQ3St4");

#[program]
pub mod governance_module {
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
    pub executed_at: Option<i64>,
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

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
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

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
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