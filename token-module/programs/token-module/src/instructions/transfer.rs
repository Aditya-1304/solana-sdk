use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::token_interface::{self, TransferChecked};
use crate::{TokenError, TransferSol, TransferTokens};

pub fn transfer_sol(ctx: Context<TransferSol>, amount: u64) -> Result<()> {
        msg!("Transferring {} lamports from {} to {}", amount, ctx.accounts.from.key(), ctx.accounts.to.key());

        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.from.to_account_info(),
                    to: ctx.accounts.to.to_account_info(),
                },
            ),
            amount
        )?;

        msg!("SOL transfer successful");
        Ok(())
    }

pub fn transfer_tokens(
        ctx: Context<TransferTokens>,
        amount: u64,
    ) -> Result<()> {
        require!(amount>0, TokenError::ZeroAmount);
        require!(
            !ctx.accounts.from_account.is_frozen(),
            TokenError::AccountFrozen
        );
        token_interface::transfer_checked(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.from_account.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.to_account.to_account_info(),
                    authority: ctx.accounts.owner.to_account_info(),
                },
            ),
            amount,
            ctx.accounts.mint.decimals,
        )?;

        msg!("Transferred {} tokens from {} to {}", amount, ctx.accounts.from_account.key(), ctx.accounts.to_account.key());
        Ok(())
    }