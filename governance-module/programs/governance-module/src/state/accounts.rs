use anchor_lang::prelude::*;

#[account]
pub struct Governance {
    pub authority: Pubkey,
    pub name: String,
    pub description: String,
    pub config: GovernanceConfig,
    pub proposal_count: u64,
    pub total_voting_power: u64,
    pub active_proposals: u32,
    pub paused: bool,
    pub created_at: i64,
    pub bump: u8,
}

impl Governance {
    pub const MAX_NAME_LEN: usize = 64;
    pub const MAX_DESC_LEN: usize = 256;
    
    pub fn space(name: &str, description: &str) -> usize {
        8 + // discriminator
        32 + // authority
        4 + name.len() + // name (String = 4 bytes length + data)
        4 + description.len() + // description
        GovernanceConfig::SPACE + // config
        8 + // proposal_count
        8 + // total_voting_power
        4 + // active_proposals
        1 + // paused
        8 + // created_at
        1 // bump
    }
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
    
    // Timing
    pub created_at: i64,
    pub voting_start_time: i64,
    pub voting_end_time: i64,
    pub execution_delay_hours: u64,
    
    // Vote tracking
    pub votes_for: u64,
    pub votes_against: u64,
    pub votes_abstain: u64,
    pub unique_voters: u32,
    
    // Status
    pub status: ProposalStatus,
    pub executed: bool,
    pub execution_time: Option<i64>,
    pub cancellation_reason: Option<String>,
    pub bump: u8,
}

impl Proposal {
    pub const MAX_TITLE_LEN: usize = 128;
    pub const MAX_DESC_LEN: usize = 1024;
    pub const MAX_INSTRUCTION_LEN: usize = 1232;
    pub const MAX_CANCELLATION_REASON_LEN: usize = 256;
    
    pub fn space(title: &str, description: &str) -> usize {
        8 + // discriminator
        32 + // governance
        8 + // proposal_id
        32 + // proposer
        4 + title.len() + // title
        4 + description.len() + // description
        200 + // proposal_type (max enum size with all variants)
        4 + Self::MAX_INSTRUCTION_LEN + // instruction_data (vec length + max data)
        8 + // created_at
        8 + // voting_start_time
        8 + // voting_end_time
        8 + // execution_delay_hours
        8 + // votes_for
        8 + // votes_against
        8 + // votes_abstain
        4 + // unique_voters
        1 + // status enum
        1 + // executed
        1 + 8 + // execution_time (Option<i64>)
        1 + 4 + Self::MAX_CANCELLATION_REASON_LEN + // cancellation_reason (Option<String>)
        1 // bump
    }
}

#[account]
pub struct VoteRecord {
    pub governance: Pubkey,
    pub proposal_id: u64,
    pub voter: Pubkey,
    pub vote_type: VoteType,
    pub voting_power: u64,
    pub timestamp: i64,
    pub delegated_from: Option<Pubkey>,
    pub bump: u8,
}

impl VoteRecord {
    pub const SPACE: usize = 
        8 + // discriminator
        32 + // governance
        8 + // proposal_id
        32 + // voter
        1 + // vote_type
        8 + // voting_power
        8 + // timestamp
        1 + 32 + // delegated_from (Option<Pubkey>)
        1; // bump
}

#[account]
pub struct VotingDelegation {
    pub delegator: Pubkey,
    pub delegate: Pubkey,
    pub amount: u64,
    pub created_at: i64,
    pub active: bool,
    pub bump: u8,
}

impl VotingDelegation {
    pub const SPACE: usize = 
        8 + // discriminator
        32 + // delegator
        32 + // delegate
        8 + // amount
        8 + // created_at
        1 + // active
        1; // bump
}

#[account]
pub struct Treasury {
    pub governance: Pubkey,
    pub authority_bump: u8,
    pub initialized: bool,
}

impl Treasury {
    pub const SPACE: usize = 
        8 + // discriminator
        32 + // governance
        1 + // authority_bump
        1; // initialized
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub struct GovernanceConfig {
    pub voting_type: VotingType,
    pub governance_token: Option<Pubkey>,
    pub quorum_percentage: u8,
    pub voting_period_hours: u32,
    pub execution_delay_hours: u32,
    pub min_voting_power_to_propose: u64,
    pub proposal_deposit: u64,
    pub voting_power_multiplier: Option<u64>,
    pub total_supply: Option<u64>,
    pub max_active_proposals: u32,
}

impl GovernanceConfig {
    pub const SPACE: usize = 
        1 + // voting_type
        1 + 32 + // governance_token (Option<Pubkey>)
        1 + // quorum_percentage
        4 + // voting_period_hours
        4 + // execution_delay_hours
        8 + // min_voting_power_to_propose
        8 + // proposal_deposit
        1 + 8 + // voting_power_multiplier (Option<u64>)
        1 + 8 + // total_supply (Option<u64>)
        4; // max_active_proposals
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Debug)]
pub enum VotingType {
    Equal,
    TokenWeighted,
}

#[derive(AnchorDeserialize, AnchorSerialize, Clone, PartialEq, Debug)]
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

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Debug)]
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
    Abstain,
}