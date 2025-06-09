use anchor_lang::prelude::*;

declare_id!("8qzfg49CMM4u8UG6LaVhT4WuHC1CrgnrE8jYBzMFgvuZ");

pub mod instructions;
pub mod state;
pub mod events;
pub mod errors;
pub mod utils;

pub use state::*;
pub use events::*;
pub use errors::*;
pub use utils::*;

use instructions::{multisig, transaction, admin};

#[program]
pub mod multisig_module {
    use super::*;

    // Multisig management functions
    pub fn create_multisig(
        ctx: Context<CreateMultisig>,
        owners: Vec<Pubkey>,
        threshold: u8,
        admin_threshold: Option<u8>,
    ) -> Result<()> {
        multisig::create_multisig(ctx, owners, threshold, admin_threshold)
    }

    pub fn emergency_pause(ctx: Context<EmergencyAction>) -> Result<()> {
        multisig::emergency_pause(ctx)
    }

    // Transaction functions
    pub fn propose_transaction(
        ctx: Context<ProposeTransaction>,
        instruction_data: Vec<u8>,
        nonce: u64,
        transaction_type: TransactionType,
        expires_in_hours: Option<u8>,
    ) -> Result<()> {
        transaction::propose_transaction(ctx, instruction_data, nonce, transaction_type, expires_in_hours)
    }

    pub fn approve_transaction(
        ctx: Context<ApproveTransaction>,
        transaction_id: u64,
    ) -> Result<()> {
        transaction::approve_transaction(ctx, transaction_id)
    }

    pub fn execute_transaction(
        ctx: Context<ExecuteTransaction>,
        transaction_id: u64,
    ) -> Result<()> {
        transaction::execute_transaction(ctx, transaction_id)
    }

    // Admin functions
    pub fn change_threshold(
        ctx: Context<ChangeThreshold>,
        transaction_id: u64,
        new_threshold: u8,
    ) -> Result<()> {
        admin::change_threshold(ctx, transaction_id, new_threshold)
    }

    pub fn add_owner(
        ctx: Context<AddOwner>,
        transaction_id: u64,
        new_owner: Pubkey,
    ) -> Result<()> {
        admin::add_owner(ctx, transaction_id, new_owner)
    }

    pub fn remove_owner(
        ctx: Context<RemoveOwner>,
        transaction_id: u64,
        owner_to_remove: Pubkey,
    ) -> Result<()> {
        admin::remove_owner(ctx, transaction_id, owner_to_remove)
    }

    pub fn unpause(
        ctx: Context<UnpauseMultisig>,
        transaction_id: u64,
    ) -> Result<()> {
        admin::unpause(ctx, transaction_id)
    }
}
