use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, TransferChecked};
use crate::{TokenError, EscrowCreated, EscrowReleased, 
    CreateEscrow, ReleaseEscrow};

pub fn create_escrow(
        ctx: Context<CreateEscrow>,
        amount: u64,
        seed: [u8; 32],
        recipient: Option<Pubkey>,
    ) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;
        escrow.sender = ctx.accounts.sender.key();
        escrow.mint = ctx.accounts.mint.key();
        escrow.amount = amount;
        escrow.seed = seed;
        escrow.recipient = recipient;
        escrow.claimed = false;
        escrow.bump = ctx.bumps.escrow;

        token_interface::transfer_checked(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.sender_token_account.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.escrow_token_account.to_account_info(),
                    authority: ctx.accounts.sender.to_account_info(),
                },
            ),
            amount,
            ctx.accounts.mint.decimals
        )?;

        emit!(EscrowCreated {
            escrow: escrow.key(),
            sender: ctx.accounts.sender.key(),
            mint: ctx.accounts.mint.key(),
            amount,
            recipient: escrow.recipient,
        });

        msg!("Created escrow with {} tokens", amount);
        Ok(())

    }

pub fn release_escrow(ctx: Context<ReleaseEscrow>) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;

        if let Some(recipient) = escrow.recipient {
            require!(
                recipient == ctx.accounts.recipient.key(),
                TokenError::UnauthorizedRecipient
            );
        }
        else {
            require!(
                escrow.sender == ctx.accounts.recipient.key(),
                TokenError::UnauthorizedRecipient
            )
        }
        require!(!escrow.claimed, TokenError::EscrowAlreadyClaimed);

        escrow.claimed = true;

        let escrow_key = escrow.key();
        let escrow_authority_seeds = &[
                b"escrow_authority".as_ref(),
                escrow_key.as_ref(),
                &[ctx.bumps.escrow_authority],
            ];

        token_interface::transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.escrow_token_account.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.recipient_token_account.to_account_info(),
                    authority: ctx.accounts.escrow_authority.to_account_info(),
                },
                &[escrow_authority_seeds]
            ),
            escrow.amount,
            ctx.accounts.mint.decimals,
        )?;

        emit!(EscrowReleased {
            escrow: escrow.key(),
            recipient: ctx.accounts.recipient.key(),
            mint: ctx.accounts.mint.key(),
            amount: escrow.amount,
        });

        msg!("Released {} tokens from escrow to {}", escrow.amount, ctx.accounts.recipient.key());
        Ok(())
    }