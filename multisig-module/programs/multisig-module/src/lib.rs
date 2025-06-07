use anchor_lang::prelude::*;

declare_id!("8qzfg49CMM4u8UG6LaVhT4WuHC1CrgnrE8jYBzMFgvuZ");

#[program]
pub mod multisig_module {

    use super::*;

    pub fn create_multisig(
        ctx: Context<CreateMultisig>,
        owners: Vec<Pubkey>,
        threshold: u8,
    ) -> Result<()> {

        let multisig = &mut ctx.accounts.multisig;

        require!(!owners.is_empty(), MultisigError::NoOwners);
        require!(owners.len() <= 10, MultisigError::TooManyOwners);
        require!(threshold > 0, MultisigError::InvalidThreshold);
        require!(threshold <= owners.len() as u8, MultisigError::InvalidThreshold);

        for owner in &owners {
            require!(*owner != Pubkey::default(), MultisigError::InvalidOwner);
        }

        for i in 0..owners.len() {
            for j in i + 1..owners.len() {
                require!(owners[i] != owners[j], MultisigError::DuplicateOwners);
            }
        }

        multisig.owners = owners.clone();
        multisig.threshold = threshold;
        multisig.transaction_count = 0;
        multisig.bump = ctx.bumps.multisig;

        msg!("Multisig created with {} owners, threshold {}", owners.len(), threshold);
        Ok(())
    }

    pub fn change_threshold(
        ctx: Context<ChangeThreshold>,
        transaction_id: u64,
        new_threshold: u8,
    ) -> Result<()> {
        let multisig = &mut ctx.accounts.multisig;
        let transaction = &mut ctx.accounts.transaction;

        require!(transaction.executed, MultisigError::TransactionNotExecuted);
        require!(transaction.is_ready_to_execute(multisig.threshold), MultisigError::NotEnoughApprovals);

        require!(new_threshold > 0, MultisigError::InvalidThreshold);
        require!(new_threshold <= multisig.owners.len() as u8, MultisigError::InvalidThreshold);

        let old_threshold = multisig.threshold;
        multisig.threshold = new_threshold;
        msg!("Threshold changed from {} to {}", old_threshold, new_threshold);
        Ok(())
    }

    pub fn add_owner(
        ctx: Context<AddOwner>,
        transaction_id: u64,
        new_owner: Pubkey,
    ) -> Result<()> {
        let multisig = &mut ctx.accounts.multisig;
        let transaction = &ctx.accounts.transaction;


        require!(transaction.executed, MultisigError::TransactionNotExecuted);
        require!(transaction.is_ready_to_execute(multisig.threshold), MultisigError::NotEnoughApprovals);

        // Validate new owner
        require!(new_owner != Pubkey::default(), MultisigError::InvalidOwner);
        require!(multisig.owners.len() < 10, MultisigError::TooManyOwners);
        require!(!multisig.owners.contains(&new_owner), MultisigError::DuplicateOwners);

        multisig.owners.push(new_owner);

        msg!("Owner {} added. Total owners: {}", new_owner, multisig.owners.len());
        Ok(())
    }
    pub fn remove_owner(
        ctx: Context<RemoveOwner>,
        transaction_id: u64,
        owner_to_remove: Pubkey,
    ) -> Result<()> {
        let multisig = &mut ctx.accounts.multisig;
        let transaction = &ctx.accounts.transaction;

        require!(transaction.executed, MultisigError::TransactionNotExecuted);
        require!(transaction.is_ready_to_execute(multisig.threshold), MultisigError::NotEnoughApprovals);

        // Find and remove owner
        let owner_index = multisig.owners
            .iter()
            .position(|&owner| owner == owner_to_remove)
            .ok_or(MultisigError::OwnerNotFound)?;

        multisig.owners.remove(owner_index);

        // Ensure threshold is still valid
        require!(multisig.threshold <= multisig.owners.len() as u8, MultisigError::InvalidThreshold);
        require!(!multisig.owners.is_empty(), MultisigError::NoOwners);

        msg!("Owner {} removed. Total owners: {}", owner_to_remove, multisig.owners.len());
        Ok(())
    }
    pub fn emergency_pause(ctx: Context<EmergencyAction>) -> Result<()> {
        let multisig = &mut ctx.accounts.multisig;
        let caller = &ctx.accounts.caller;

        let is_owner = multisig.owners.iter().any(|owner| owner == caller.key);
        require!(is_owner, MultisigError::OwnerNotFound);

        multisig.paused = true;
        multisig.paused_by = caller.key();
        multisig.paused_at = Clock::get()?.unix_timestamp;

        msg!("Multisig paused by {}", caller.key);
        Ok(())
    }

    pub fn unpause(
        ctx: Context<UnpauseMultisig>,
        transaction_id: u64,
    ) -> Result<()> {
        let multisig = &mut ctx.accounts.multisig;
        let transaction = &ctx.accounts.transaction;

        require!(multisig.paused, MultisigError::NotPaused);
        require!(transaction.executed, MultisigError::TransactionNotExecuted);
        require!(transaction.is_ready_to_execute(multisig.threshold), MultisigError::NotEnoughApprovals);

        multisig.paused = false;
        multisig.paused_by = Pubkey::default();
        multisig.paused_at = 0;

        msg!("Multisig unpaused");
        Ok(())
    }

    pub fn propose_transaction(
        ctx: Context<ProposeTransaction>,
        instruction_data: Vec<u8>,
    ) -> Result<()> {

        let multisig = &mut ctx.accounts.multisig;
        let transaction = &mut ctx.accounts.transaction;
        let proposer = &ctx.accounts.proposer;

        require!(!multisig.paused, MultisigError::MultisigPaused);
        multisig.validate_state()?;

        require!(!instruction_data.is_empty(), MultisigError::EmptyTransaction);
        require!(instruction_data.len() <= 1000, MultisigError::TransactionTooLarge);

        let is_owner = multisig.owners.iter().any(|owner| owner == proposer.key);
        require!(is_owner, MultisigError::OwnerNotFound);

        let current_transaction_id = multisig.transaction_count;

        transaction.multisig = multisig.key();
        transaction.proposer = proposer.key();
        transaction.instruction_data = instruction_data.clone();
        transaction.transaction_id = current_transaction_id;
        transaction.executed = false;
        transaction.created_at = Clock::get()?.unix_timestamp;
        transaction.approvals = vec![false; multisig.owners.len()];

        multisig.transaction_count = multisig.transaction_count
            .checked_add(1)
            .ok_or(MultisigError::TransactionCountOverflow)?;

        msg!(
                "Transaction {} proposed by {} with {} bytes of data", 
                current_transaction_id,
                proposer.key(),
                instruction_data.len()
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

        require!(!transaction.executed, MultisigError::AlreadyExecuted);
        require!(transaction.transaction_id == transaction_id, MultisigError::InvalidTransactionId);

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

        require!(!transaction.executed, MultisigError::AlreadyExecuted);
        require!(transaction.transaction_id == transaction_id, MultisigError::InvalidTransactionId);

        let is_owner = multisig.owners.iter().any(|owner| owner == executor.key);
        require!(is_owner, MultisigError::OwnerNotFound);

        let approval_count = transaction.approvals.iter().filter(|&&approved| approved).count() as u8;

        require!(approval_count >= multisig.threshold, MultisigError::NotEnoughApprovals);

        transaction.executed = true;

        msg!(
        "Transaction {} executed by {}. Had {}/{} approvals",
        transaction_id,
        executor.key,
        approval_count,
        multisig.threshold
        );

        // TODO: In a real implementation, you would execute the actual instruction here
        // For now, we just mark it as executed and log it
        msg!("Instruction data to execute: {:?}", transaction.instruction_data);

        Ok(())
    }

}

#[derive(Accounts)]
#[instruction(owners: Vec<Pubkey>, threshold: u8)]  
pub struct CreateMultisig<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        init,
        payer = creator,
        space = 8 + Multisig::INIT_SPACE,
        seeds = [b"multisig", creator.key().as_ref()],
        bump,
    )]
    pub multisig: Account<'info, Multisig>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(instruction_data: Vec<u8>)]
pub struct ProposeTransaction<'info> {
    #[account(mut)]
    pub proposer: Signer<'info>,

    #[account(
        mut,
        // seeds = [b"multisig", multisig.owners[0].as_ref()],
        // bump = multisig.bump,

        // We will derive the multisig seed at client side
    )]
    pub multisig: Account<'info, Multisig>,

    #[account(
        init,
        payer = proposer,
        space = 8 + Transaction::INIT_SPACE,
        seeds = [
            b"transaction",
            multisig.key().as_ref(),
            &multisig.transaction_count.to_le_bytes()
        ],
        bump,
    )]
    pub transaction: Account<'info, Transaction>,

    pub system_program: Program<'info, System>,

}

#[derive(Accounts)]
#[instruction(transaction_id: u64)]
pub struct ApproveTransaction<'info> {
    #[account(mut)]
    pub approver: Signer<'info>,

    pub multisig: Account<'info, Multisig>,

    #[account(
        mut,
        seeds = [
            b"transaction",
            multisig.key().as_ref(),
            &transaction_id.to_le_bytes()
        ],
        bump,
        constraint = transaction.multisig == multisig.key() @ MultisigError::InvalidTransaction
    )]
    pub transaction: Account<'info, Transaction>,
}

#[derive(Accounts)]
#[instruction(transaction_id: u64)]
pub struct ExecuteTransaction<'info> {
    #[account(mut)]
    pub executor: Signer<'info>,

    pub multisig: Account<'info, Multisig>,

    #[account(
        mut,
        seeds = [
            b"transaction",
            multisig.key().as_ref(),
            &transaction_id.to_le_bytes()
        ],
        bump,
        constraint = transaction.multisig == multisig.key() @ MultisigError::InvalidTransaction
    )]
    pub transaction: Account<'info, Transaction>,
}

#[derive(Accounts)]
#[instruction(transaction_id: u64, new_threshold: u8)]
pub struct ChangeThreshold<'info> {
    #[account(mut)]
    pub multisig: Account<'info, Multisig>,
    
    #[account(
        constraint = transaction.multisig == multisig.key() @ MultisigError::InvalidTransaction,
        constraint = transaction.transaction_id == transaction_id @ MultisigError::InvalidTransactionId
    )]
    pub transaction: Account<'info, Transaction>,
}

#[derive(Accounts)]
pub struct EmergencyAction<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,
    
    #[account(mut)]
    pub multisig: Account<'info, Multisig>,
}

#[account]
#[derive(InitSpace)]
pub struct Multisig {
    #[max_len(10)]
    pub owners: Vec<Pubkey>,
    pub threshold: u8,
    pub transaction_count: u64,
    pub bump: u8,
    pub paused: bool,
    pub paused_by: Pubkey,
    pub paused_at: i64,
    pub created_at: i64,
}
impl Multisig {
    pub fn validate_state(&self) -> Result<()> {
        require!(!self.owners.is_empty(), MultisigError::NoOwners);
        require!(self.owners.len() <= 10, MultisigError::TooManyOwners);
        require!(self.threshold > 0, MultisigError::InvalidThreshold);
        require!(self.threshold <= self.owners.len() as u8, MultisigError::InvalidThreshold);
        Ok(())
    }
}

#[account]
#[derive(InitSpace)]
pub struct Transaction {
    pub multisig: Pubkey,
    pub proposer: Pubkey,
    #[max_len(1000)]
    pub instruction_data: Vec<u8>,
    #[max_len(10)]
    pub approvals: Vec<bool>,
    pub executed: bool,
    pub transaction_id: u64,
    pub created_at: i64,
}



impl Transaction {
    pub fn validate_state(&self, multisig: &Multisig) -> Result<()> {
        require!(
            self.approvals.len() == multisig.owners.len(),
            MultisigError::ApprovalArrayMismatch
        );
        require!(
            !self.instruction_data.is_empty(),
            MultisigError::EmptyTransaction
        );
        require!(
            self.instruction_data.len() <= 1000,
            MultisigError::TransactionTooLarge
        );
        Ok(())
    }

    pub fn approval_count(&self) -> usize {
        self.approvals.iter().filter(|&&approved| approved).count()
    }

    pub fn is_ready_to_execute(&self, threshold: u8) -> bool {
        self.approval_count() >= threshold as usize && !self.executed
    }
}

#[error_code]
pub enum MultisigError {
    #[msg("Invalid threshold: must be > 0 and <= number of owners")]
    InvalidThreshold,
    #[msg("No owners provided")]
    NoOwners,
    #[msg("Too many owners: maximum 10 allowed")]
    TooManyOwners,
    #[msg("Duplicate owners not allowed")]
    DuplicateOwners,
    #[msg("Invalid owner: cannot be default pubkey")]
    InvalidOwner,
    #[msg("Owner not found")]
    OwnerNotFound,
    #[msg("Already approved")]
    AlreadyApproved,
    #[msg("Not enough approvals")]
    NotEnoughApprovals,
    #[msg("Transaction already executed")]
    AlreadyExecuted,
    #[msg("Empty transaction data")]
    EmptyTransaction,
    #[msg("Transaction data too large")]
    TransactionTooLarge,
    #[msg("Invalid transaction ID")]
    InvalidTransactionId,
    #[msg("Invalid transaction")]
    InvalidTransaction,
    #[msg("Transaction count overflow")]
    TransactionCountOverflow,
    #[msg("Approval count overflow")]
    ApprovalCountOverflow,
    #[msg("Approval array length mismatch")]
    ApprovalArrayMismatch,
    #[msg("Clock unavailable")]
    ClockUnavailable,
    #[msg("Multisig is paused")]
    MultisigPaused,
    #[msg("Multisig is not paused")]
    NotPaused,
    #[msg("Transaction not executed")]
    TransactionNotExecuted,
    #[msg("Invalid multisig state")]
    InvalidMultisigState,
    #[msg("Invalid transaction state")]
    InvalidTransactionState,
}