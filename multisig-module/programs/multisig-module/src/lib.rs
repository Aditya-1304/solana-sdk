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
        msg!("Proposer {} is proposing a transaction", proposer.key());
        Ok(())
    }
    pub fn approve_transaction(
        ctx: Context<ApproveTransaction>,
        transaction_id: u64,
    ) -> Result<()> {
        let approver = &ctx.accounts.approver;
        msg!("Approver {} is approving transaction {}", approver.key(), transaction_id);
        Ok(())
    }
    pub fn execute_transaction(
        ctx: Context<ExecuteTransaction>,
        transaction_id: u64,
    ) -> Result<()> {
        let executor = &ctx.accounts.executor;
        msg!("Executor {} is executing transaction {}", executor.key(), transaction_id);
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
pub struct ProposeTransaction<'info> {
    #[account(mut)]
    pub proposer: Signer<'info>,

}

#[derive(Accounts)]
pub struct ApproveTransaction<'info> {
    #[account(mut)]
    pub approver: Signer<'info>,
}

#[derive(Accounts)]
pub struct ExecuteTransaction<'info> {
    #[account(mut)]
    pub executor: Signer<'info>,
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
pub struct Transaction {
    pub multisig: Pubkey,
    pub instruction_data: Vec<u8>,
    pub approvals: Vec<bool>,
    pub executed: bool,
    pub transaction_id: u64,

}

#[error_code]
pub enum MultisigError {
    #[msg("Invalid threshold: must be > 0 and <= number of owners")]
    InvalidThreshold,
    #[msg("Too many owners: maximum 10 allowed")]
    NoOwners,
    #[msg("Duplicate owners not allowed")]
    TooManyOwners,
    #[msg("Owner not found")]
    DuplicateOwners,
    #[msg("Owner not found")]
    OwnerNotFound,
    #[msg("Already approved")]
    AlreadyApproved,
    #[msg("Not enough approvals")]
    NotEnoughApprovals,
    #[msg("Transaction already executed")]
    AlreadyExecuted,
}