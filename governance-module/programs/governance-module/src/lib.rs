use anchor_lang::prelude::*;

declare_id!("HJzW17DkivXRYjirjDD56a3Pve6JFnKhmsfpswJQ3St4");

#[program]
pub mod governance_module {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
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
