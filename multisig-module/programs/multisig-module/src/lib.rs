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

        require!(threshold > 0, MultisigError::InvalidThreshold);
        require!(threshold <= owners.len() as u8, MultisigError::InvalidThreshold);
        require!(owners.len() <= 10, MultisigError::TooManyOwners);
        require!(!owners.is_empty(), MultisigError::NoOwners);

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

    pub fn propose_transaction(
        ctx: Context<ProposeTransaction>,
        instruction_data: Vec<u8>,
    ) -> Result<()> {
        let proposer = &ctx.accounts.proposer;

        let multisig = &mut ctx.accounts.multisig;
        let transaction = &mut ctx.accounts.transaction;
        let proposer = &ctx.accounts.proposer;

        require!(!instruction_data.is_empty(), MultisigError::EmptyTransaction);
        require!(instruction_data.len() <= 1000, MultisigError::TransactionTooLarge);

        let is_owner = multisig.owners.iter().any(|owner| owner == proposer.key);
        require!(is_owner, MultisigError::OwnerNotFound);

        transaction.multisig = multisig.key();
        transaction.proposer = proposer.key();
        transaction.instruction_data = instruction_data.clone();
        transaction.transaction_id = multisig.transaction_count;
        transaction.executed = false;
        transaction.created_at = Clock::get()?.unix_timestamp;

        transaction.approvals = vec![false; multisig.owners.len()];

        multisig.transaction_count += 1;

        msg!(
                "Transaction {} proposed by {} with {} bytes of data", 
                transaction.transaction_id,
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

#[account]
#[derive(InitSpace)]
pub struct Multisig {
    #[max_len(10)]
    pub owners: Vec<Pubkey>,
    pub threshold: u8,
    pub transaction_count: u64,
    pub bump: u8,
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
}