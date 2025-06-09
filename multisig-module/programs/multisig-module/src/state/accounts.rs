use anchor_lang::prelude::*;
use crate::{ MultisigError};

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

