use anchor_lang::{prelude::*};
use crate::{
    MultisigError,
    ThresholdChanged, OwnerAdded, OwnerRemoved, MultisigUnpaused,
    ChangeThreshold, AddOwner, RemoveOwner, UnpauseMultisig
};

pub fn change_threshold(
        ctx: Context<ChangeThreshold>,
        _transaction_id: u64,
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

        emit!(ThresholdChanged {
          multisig: multisig.key(),
          transaction: transaction.key(),
          old_threshold,
          new_threshold,
          changed_at: Clock::get()?.unix_timestamp,
        });

        msg!("Threshold changed from {} to {}", old_threshold, new_threshold);
        Ok(())
    }

    pub fn add_owner(
        ctx: Context<AddOwner>,
        _transaction_id: u64,
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

        emit!(OwnerAdded {
          multisig: multisig.key(),
          transaction: transaction.key(),
          new_owner,
          total_owners: multisig.owners.len() as u8,
          added_at: Clock::get()?.unix_timestamp,
        });

        msg!("Owner {} added. Total owners: {}", new_owner, multisig.owners.len());
        Ok(())
    }

    pub fn remove_owner(
        ctx: Context<RemoveOwner>,
        _transaction_id: u64,
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

        transaction.executed = true; 

        emit!(OwnerRemoved {
          multisig: multisig.key(),
          transaction: transaction.key(),
          removed_owner: owner_to_remove,
          total_owners: multisig.owners.len() as u8,
          removed_at: Clock::get()?.unix_timestamp,
        });

        msg!("Owner {} removed. Total owners: {}", owner_to_remove, multisig.owners.len());
        Ok(())
    }

pub fn unpause(
        ctx: Context<UnpauseMultisig>,
        _transaction_id: u64,
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

        emit!(MultisigUnpaused {
          multisig:multisig.key(),
          transaction: transaction.key(),
          unpaused_at: Clock::get()?.unix_timestamp,
        });

        msg!("Multisig unpaused");
        Ok(())
    }