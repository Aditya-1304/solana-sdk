use anchor_lang::prelude::*;
use crate::state::*;

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
    pub reason: String,
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

#[event]
pub struct VotingPowerDelegated {
    pub delegator: Pubkey,
    pub delegate: Pubkey,
    pub amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct DelegationRevoked {
    pub delegator: Pubkey,
    pub delegate: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct TreasuryInitialized {
    pub governance: Pubkey,
    pub treasury: Pubkey,
    pub token_mint: Option<Pubkey>,
}

#[event]
pub struct TreasuryWithdrawal {
    pub governance: Pubkey,
    pub proposal_id: u64,
    pub recipient: Pubkey,
    pub amount: u64,
    pub token_mint: Option<Pubkey>,
}