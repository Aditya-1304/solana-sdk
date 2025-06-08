use anchor_lang::prelude::*;

declare_id!("8qzfg49CMM4u8UG6LaVhT4WuHC1CrgnrE8jYBzMFgvuZ");

#[program]
pub mod multisig_module {

    use super::*;

    pub fn create_multisig(
        ctx: Context<CreateMultisig>,
        owners: Vec<Pubkey>,
        threshold: u8,
        admin_threshold: Option<u8>,
    ) -> Result<()> {

        let multisig = &mut ctx.accounts.multisig;

        require!(!owners.is_empty(), MultisigError::NoOwners);
        require!(owners.len() <= 10, MultisigError::TooManyOwners);
        require!(threshold > 0, MultisigError::InvalidThreshold);
        require!(threshold <= owners.len() as u8, MultisigError::InvalidThreshold);

        let admin_thresh = admin_threshold.unwrap_or(threshold);
        require!(admin_thresh >= threshold, MultisigError::InvalidAdminThreshold);
        require!(admin_thresh <= owners.len() as u8, MultisigError::InvalidAdminThreshold);

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
        multisig.admin_threshold = admin_thresh;
        multisig.transaction_count = 0;
        multisig.bump = ctx.bumps.multisig;
        multisig.paused = false; 
        multisig.paused_by = Pubkey::default();
        multisig.paused_at = 0;
        multisig.created_at = Clock::get()?.unix_timestamp;
        multisig.nonce = 0;
        multisig.last_proposal_slot = 0;

        multisig.validate_state()?;

        msg!("Multisig created with {} owners, threshold {}, admin threshold {}", 
             owners.len(), threshold, admin_thresh);
        Ok(())
    }

    pub fn change_threshold(
        ctx: Context<ChangeThreshold>,
        transaction_id: u64,
        new_threshold: u8,
    ) -> Result<()> {
        let multisig = &mut ctx.accounts.multisig;
        let transaction = &mut ctx.accounts.transaction;

        require!(!multisig.paused, MultisigError::MultisigPaused);
        multisig.validate_state()?;
        transaction.validate_state(multisig)?;

        require!(!transaction.executed, MultisigError::AlreadyExecuted);
        require!(transaction.is_admin_ready_to_execute(multisig.admin_threshold), MultisigError::NotEnoughAdminApprovals);

        require!(new_threshold > 0, MultisigError::InvalidThreshold);
        require!(new_threshold <= multisig.owners.len() as u8, MultisigError::InvalidThreshold);

        let old_threshold = multisig.threshold;
        multisig.threshold = new_threshold; 
        transaction.executed = true; 

        msg!("Threshold changed from {} to {}", old_threshold, new_threshold);
        Ok(())
    }

    pub fn add_owner(
        ctx: Context<AddOwner>,
        transaction_id: u64,
        new_owner: Pubkey,
    ) -> Result<()> {
        let multisig = &mut ctx.accounts.multisig;
        let transaction = &mut ctx.accounts.transaction;

        require!(!multisig.paused, MultisigError::MultisigPaused);
        multisig.validate_state()?;
        transaction.validate_state(multisig)?;

        
        require!(!transaction.executed, MultisigError::AlreadyExecuted);
        require!(transaction.is_admin_ready_to_execute(multisig.admin_threshold), MultisigError::NotEnoughAdminApprovals);

        
        require!(new_owner != Pubkey::default(), MultisigError::InvalidOwner);
        require!(multisig.owners.len() < 10, MultisigError::TooManyOwners);
        require!(!multisig.owners.contains(&new_owner), MultisigError::DuplicateOwners);

        multisig.owners.push(new_owner); 
        transaction.executed = true; 

        msg!("Owner {} added. Total owners: {}", new_owner, multisig.owners.len());
        Ok(())
    }

    pub fn remove_owner(
        ctx: Context<RemoveOwner>,
        transaction_id: u64,
        owner_to_remove: Pubkey,
    ) -> Result<()> {
        let multisig = &mut ctx.accounts.multisig;
        let transaction = &mut ctx.accounts.transaction;

        require!(!multisig.paused, MultisigError::MultisigPaused);
        multisig.validate_state()?;
        transaction.validate_state(multisig)?;

        
        require!(!transaction.executed, MultisigError::AlreadyExecuted);
        require!(transaction.is_admin_ready_to_execute(multisig.admin_threshold), MultisigError::NotEnoughAdminApprovals);

        
        let owner_index = multisig.owners
            .iter()
            .position(|&owner| owner == owner_to_remove)
            .ok_or(MultisigError::OwnerNotFound)?;

        multisig.owners.remove(owner_index);

        require!(multisig.threshold <= multisig.owners.len() as u8, MultisigError::InvalidThreshold);
        require!(multisig.admin_threshold <= multisig.owners.len() as u8, MultisigError::InvalidAdminThreshold);
        require!(!multisig.owners.is_empty(), MultisigError::NoOwners);

        transaction.executed = true; // ✅ Mark as executed only once

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
        let transaction = &mut ctx.accounts.transaction;

        require!(multisig.paused, MultisigError::NotPaused);
        require!(!transaction.executed, MultisigError::AlreadyExecuted);
        require!(transaction.is_ready_to_execute(multisig.threshold), MultisigError::NotEnoughApprovals);

        multisig.paused = false;
        multisig.paused_by = Pubkey::default();
        multisig.paused_at = 0;
        transaction.executed = true;

        msg!("Multisig unpaused");
        Ok(())
    }

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

}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, InitSpace)]
pub enum TransactionType {
    Transfer,
    TokenTransfer,
    AdminAction,
    ChangeThreshold,
    AddOwner,
    RemoveOwner,
    Custom
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
        mut,
        constraint = transaction.multisig == multisig.key() @ MultisigError::InvalidTransaction,
        constraint = transaction.transaction_id == transaction_id @ MultisigError::InvalidTransactionId
    )]
    pub transaction: Account<'info, Transaction>,
}

#[derive(Accounts)]
#[instruction(transaction_id: u64, new_owner: Pubkey)]
pub struct AddOwner<'info> {
    #[account(mut)]
    pub multisig: Account<'info, Multisig>,
    
    #[account(
        mut,
        constraint = transaction.multisig == multisig.key() @ MultisigError::InvalidTransaction,
        constraint = transaction.transaction_id == transaction_id @ MultisigError::InvalidTransactionId
    )]
    pub transaction: Account<'info, Transaction>,
}

#[derive(Accounts)]
#[instruction(transaction_id: u64, owner_to_remove: Pubkey)]
pub struct RemoveOwner<'info> {
    #[account(mut)]
    pub multisig: Account<'info, Multisig>,
    
    #[account(
        mut,
        constraint = transaction.multisig == multisig.key() @ MultisigError::InvalidTransaction,
        constraint = transaction.transaction_id == transaction_id @ MultisigError::InvalidTransactionId
    )]
    pub transaction: Account<'info, Transaction>,
}

#[derive(Accounts)]
#[instruction(transaction_id: u64)]
pub struct UnpauseMultisig<'info> {
    #[account(mut)]
    pub multisig: Account<'info, Multisig>,
    
    #[account(
        mut,
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
    pub admin_threshold: u8,
    pub transaction_count: u64,
    pub bump: u8,
    pub paused: bool,
    pub paused_by: Pubkey,
    pub paused_at: i64,
    pub created_at: i64,
    pub nonce: u64,
    pub last_proposal_slot: u64,
}
impl Multisig {
    pub fn validate_state(&self) -> Result<()> {
        require!(!self.owners.is_empty(), MultisigError::NoOwners);
        require!(self.owners.len() <= 10, MultisigError::TooManyOwners);
        require!(self.threshold > 0, MultisigError::InvalidThreshold);
        require!(self.threshold <= self.owners.len() as u8, MultisigError::InvalidThreshold);
        require!(self.admin_threshold >= self.threshold, MultisigError::InvalidAdminThreshold);
        require!(self.admin_threshold <= self.owners.len() as u8, MultisigError::InvalidAdminThreshold);

        let mut sorted_owners = self.owners.clone();
        sorted_owners.sort();

        for i in 1..sorted_owners.len() {
            require!(sorted_owners[i-1] != sorted_owners[i], MultisigError::DuplicateOwners);
            require!(sorted_owners[i] != Pubkey::default(), MultisigError::InvalidOwner);
        }
        Ok(())
    }
}

#[account]
#[derive(InitSpace)]
pub struct Transaction {
    pub transaction_id: u64,
    pub created_at: i64,
    pub expires_at: i64,
    pub executed: bool,
    pub created_slot: u64,

    pub multisig: Pubkey,
    pub proposer: Pubkey,
    pub transaction_type: TransactionType,

    #[max_len(10)]
    pub approvals: Vec<bool>,
    #[max_len(1000)]
    pub instruction_data: Vec<u8>,
}



impl Transaction {
    pub fn is_expired(&self) -> Result<bool> {
        let clock = Clock::get()?;
        Ok(clock.unix_timestamp > self.expires_at)
    }

    pub fn validate_state(&self, multisig: &Multisig) -> Result<()> {
        require!(!self.is_expired()?, MultisigError::TransactionExpired);
        require!(
            self.approvals.len() == multisig.owners.len(),
            MultisigError::ApprovalArrayMismatch
        );
        require!(!self.instruction_data.is_empty(), MultisigError::EmptyTransaction);
        require!(self.instruction_data.len() <= 1000, MultisigError::TransactionTooLarge);
        Ok(())
    }

    pub fn approval_count(&self) -> usize {
        self.approvals.iter().filter(|&&approved| approved).count()
    }

    pub fn is_ready_to_execute(&self, threshold: u8) -> bool {
        if self.executed {
            return false;
        }

        let mut count = 0u8;
        for &approved in &self.approvals {
            if approved {
                count += 1;
                if count >= threshold {
                    return true;
                }
            }
        }
        false
    }


    pub fn is_admin_ready_to_execute(&self, admin_threshold: u8) -> bool {
        self.is_ready_to_execute(admin_threshold)
    }
}

fn calculate_instruction_complexity(data: &[u8]) -> Result<u32> {
    let mut complexity = 0u32;
    
    // Simple heuristic: 1 point per 10 bytes + bonus for certain patterns
    complexity += (data.len() as u32) / 10;
    
    // Add complexity for potential loops/calls (simplified)
    for chunk in data.chunks(4) {
        if chunk.len() == 4 {
            let value = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            if value > 1000000 { // Large numbers might indicate loops
                complexity += 10;
            }
        }
    }
    
    Ok(complexity)
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
    #[msg("Transaction expired")]
    TransactionExpired,
    #[msg("Invalid nonce")]
    InvalidNonce,
    #[msg("Nonce overflow")]
    NonceOverflow,
    #[msg("Not enough admin approvals")]
    NotEnoughAdminApprovals,
    #[msg("Same slot execution not allowed")]
    SameSlotExecution,
    #[msg("Rate limit exceeded")]
    RateLimitExceeded,
    #[msg("Invalid transaction type")]
    InvalidTransactionType,
    #[msg("Invalid admin threshold")]
    InvalidAdminThreshold,
    #[msg("Transaction too complex")]
    TransactionTooComplex,
    
}