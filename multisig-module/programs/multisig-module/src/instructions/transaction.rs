use anchor_lang::{prelude::*};
use crate::{
    TransactionType, MultisigError,
    TransactionProposed, TransactionApproved, TransactionExecuted,
    ProposeTransaction, ApproveTransaction, ExecuteTransaction,
    calculate_instruction_complexity
};

pub fn propose_transaction(
        ctx: Context<ProposeTransaction>,
        instruction_data: Vec<u8>,
        nonce: u64,
        transaction_type: TransactionType,
        expires_in_hours: Option<u8>,
    ) -> Result<()> {
        let multisig = &mut ctx.accounts.multisig;
        let transaction = &mut ctx.accounts.transaction;
        let proposer = &ctx.accounts.proposer;

        let clock = Clock::get()?;
        require!(
            clock.slot > multisig.last_proposal_slot + 10,
            MultisigError::RateLimitExceeded
        );

        require!(nonce == multisig.nonce, MultisigError::InvalidNonce);
        multisig.nonce = multisig.nonce.checked_add(1).ok_or(MultisigError::NonceOverflow)?;

        require!(!multisig.paused, MultisigError::MultisigPaused);
        multisig.validate_state()?;

        require!(!instruction_data.is_empty(), MultisigError::EmptyTransaction);
        require!(instruction_data.len() <= 1000, MultisigError::TransactionTooLarge);

        let complexity_score = calculate_instruction_complexity(&instruction_data)?;
        require!(complexity_score <= 100, MultisigError::TransactionTooComplex);

        let is_owner = multisig.owners.iter().any(|owner| owner == proposer.key);
        require!(is_owner, MultisigError::OwnerNotFound);

        let current_transaction_id = multisig.transaction_count;
        let expiration_hours = expires_in_hours.unwrap_or(72);
        let expires_at = clock.unix_timestamp + (expiration_hours as i64 * 3600);

        transaction.multisig = multisig.key();
        transaction.proposer = proposer.key();
        transaction.instruction_data = instruction_data.clone();
        transaction.transaction_id = current_transaction_id;
        transaction.executed = false;
        transaction.created_at = clock.unix_timestamp;
        transaction.expires_at = expires_at;
        transaction.transaction_type = transaction_type.clone();
        transaction.approvals = vec![false; multisig.owners.len()];
        transaction.created_slot = clock.slot;

        multisig.transaction_count = multisig.transaction_count
            .checked_add(1)
            .ok_or(MultisigError::TransactionCountOverflow)?;
        multisig.last_proposal_slot = clock.slot;

        emit!(TransactionProposed {
          multisig: multisig.key(),
          transaction: transaction.key(),
          proposer: proposer.key(),
          transaction_id: current_transaction_id,
          transaction_type: transaction_type.clone(),
          expires_at,
          created_at: clock.unix_timestamp,
        });

        msg!(
            "Transaction {} of type {:?} proposed by {} with {} bytes of data, expires at {}", 
            current_transaction_id,
            transaction_type,
            proposer.key(),
            instruction_data.len(),
            expires_at
        );
        Ok(())
    }
    
    pub fn approve_transaction(
        ctx: Context<ApproveTransaction>,
        transaction_id: u64,
    ) -> Result<()> {
        let approver = &ctx.accounts.approver;
        let multisig = &ctx.accounts.multisig;
        let transaction = &mut ctx.accounts.transaction;

        require!(!multisig.paused, MultisigError::MultisigPaused);
        multisig.validate_state()?;
        transaction.validate_state(multisig)?;
        require!(!transaction.executed, MultisigError::AlreadyExecuted);
        require!(transaction.transaction_id == transaction_id ,MultisigError::InvalidTransactionId
        );

        let owner_index = multisig.owners
            .iter()
            .position(|owner| owner == approver.key)
            .ok_or(MultisigError::OwnerNotFound)?;

        require!(
            owner_index < transaction.approvals.len(),
            MultisigError::ApprovalArrayMismatch
        );
        require!(
            transaction.approvals.len() == multisig.owners.len(),
            MultisigError::ApprovalArrayMismatch
        );

        require!(!transaction.approvals[owner_index], MultisigError::AlreadyApproved);

        transaction.approvals[owner_index] = true;

        let approval_count = transaction.approvals.iter().filter(|&&approved| approved).count();


        emit!(TransactionApproved {
          multisig: multisig.key(),
          transaction: transaction.key(),
          approver: approver.key(),
          transaction_id,
          approval_count: approval_count as u8,
          required_approvals: multisig.threshold,
        });

        msg!(
        "Transaction {} approved by {}. Approvals: {}/{}",
        transaction_id,
        approver.key,
        approval_count,
        multisig.threshold
        );Ok(())
    }


    pub fn execute_transaction(
        ctx: Context<ExecuteTransaction>,
        transaction_id: u64,
    ) -> Result<()> {
        let executor = &ctx.accounts.executor;
        let multisig = &ctx.accounts.multisig;
        let transaction = &mut ctx.accounts.transaction;

        
        require!(!multisig.paused, MultisigError::MultisigPaused);
        require!(!transaction.executed, MultisigError::AlreadyExecuted);
        multisig.validate_state()?;
        transaction.validate_state(multisig)?;
        require!(transaction.transaction_id == transaction_id, MultisigError::InvalidTransactionId);

        let clock = Clock::get()?;
        require!(
            clock.slot > transaction.created_slot + 1,
            MultisigError::SameSlotExecution
        );

        let is_owner = multisig.owners.iter().any(|owner| owner == executor.key);
        require!(is_owner, MultisigError::OwnerNotFound);

        let required_approvals = match transaction.transaction_type {
            TransactionType::AdminAction => multisig.admin_threshold,
            _ => multisig.threshold,
        };

        let approval_count = transaction.approval_count() as u8;

        
        require!(approval_count >= required_approvals, MultisigError::NotEnoughApprovals);

        transaction.executed = true;

        emit!(TransactionExecuted {
          multisig: multisig.key(),
          transaction: transaction.key(),
          executor: executor.key(),
          transaction_id,
          transaction_type: transaction.transaction_type.clone(),
          approval_count,
          executed_at: clock.unix_timestamp,
        });

        msg!(
            "Transaction {} of type {:?} executed by {}. Had {}/{} approvals",
            transaction_id,
            transaction.transaction_type,
            executor.key,
            approval_count,
            required_approvals
        );

        Ok(())
    }