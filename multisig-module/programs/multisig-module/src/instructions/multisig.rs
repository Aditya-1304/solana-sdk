use anchor_lang::prelude::*;
use crate::{Multisig, MultisigError, MultisigCreated, MultisigPaused, CreateMultisig, EmergencyAction};


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