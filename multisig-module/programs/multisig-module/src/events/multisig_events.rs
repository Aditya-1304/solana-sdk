use anchor_lang::prelude::*;
use crate::TransactionType;

#[event]
pub struct MultisigCreated {
    pub multisig: Pubkey,
    pub creator: Pubkey,
    pub owners: Vec<Pubkey>,
    pub threshold: u8,
    pub admin_threshold: u8,
    pub created_at: i64,
}

#[event]
pub struct MultisigPaused {
    pub multisig: Pubkey,
    pub paused_by: Pubkey,
    pub paused_at: i64,
}

#[event]
pub struct MultisigUnpaused {
    pub multisig: Pubkey,
    pub transaction: Pubkey,
    pub unpaused_at: i64,
}

#[event]
pub struct TransactionProposed {
    pub multisig: Pubkey,
    pub transaction: Pubkey,
    pub proposer: Pubkey,
    pub transaction_id: u64,
    pub transaction_type: TransactionType,
    pub expires_at: i64,
    pub created_at: i64,
}

#[event]
pub struct TransactionApproved {
    pub multisig: Pubkey,
    pub transaction: Pubkey,
    pub approver: Pubkey,
    pub transaction_id: u64,
    pub approval_count: u8,
    pub required_approvals: u8,
}

#[event]
pub struct TransactionExecuted {
    pub multisig: Pubkey,
    pub transaction: Pubkey,
    pub executor: Pubkey,
    pub transaction_id: u64,
    pub transaction_type: TransactionType,
    pub approval_count: u8,
    pub executed_at: i64,
}

#[event]
pub struct ThresholdChanged {
    pub multisig: Pubkey,
    pub transaction: Pubkey,
    pub old_threshold: u8,
    pub new_threshold: u8,
    pub changed_at: i64,
}

#[event]
pub struct OwnerAdded {
    pub multisig: Pubkey,
    pub transaction: Pubkey,
    pub new_owner: Pubkey,
    pub total_owners: u8,
    pub added_at: i64,
}

#[event]
pub struct OwnerRemoved {
    pub multisig: Pubkey,
    pub transaction: Pubkey,
    pub removed_owner: Pubkey,
    pub total_owners: u8,
    pub removed_at: i64,
}