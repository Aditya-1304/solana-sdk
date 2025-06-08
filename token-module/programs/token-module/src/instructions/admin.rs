use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, FreezeAccount, ThawAccount};
use crate::{  TokenError, TokenAccountFrozen, TokenAccountThawed, InitializeTokenAuthority, FreezeTokenAccount, ThawTokenAccount};


pub fn initialize_token_authority(ctx: Context<InitializeTokenAuthority>) -> Result<()> {
    let authority = &mut ctx.accounts.token_authority;
    authority.admin = ctx.accounts.admin.key();
    authority.bump = ctx.bumps.token_authority;
    
    msg!("Token authority initialized with admin: {}", authority.admin);
    Ok(())
}

pub fn freeze_token_account(ctx: Context<FreezeTokenAccount>) -> Result<()> {
    require!(
        ctx.accounts.admin.key() == ctx.accounts.token_metadata.admin,
        TokenError::UnauthorizedFreezeAuthority
    );

    let authority_seeds = &[
        b"token_authority".as_ref(),
        &[ctx.accounts.token_authority.bump]
    ];

    token_interface::freeze_account(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            FreezeAccount {
                account: ctx.accounts.token_account.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                authority: ctx.accounts.token_authority.to_account_info(),
            },
            &[authority_seeds]
        )
    )?;

    emit!(TokenAccountFrozen {
        mint: ctx.accounts.mint.key(),
        token_account: ctx.accounts.token_account.key(),
        admin: ctx.accounts.admin.key(),
    });

    msg!("Frozen token account {}", ctx.accounts.token_account.key());
    Ok(())
}

pub fn thaw_token_account(ctx: Context<ThawTokenAccount>) -> Result<()> {
    require!(
        ctx.accounts.admin.key() == ctx.accounts.token_metadata.admin,
        TokenError::UnauthorizedFreezeAuthority
    );

    let authority_seeds = &[
        b"token_authority".as_ref(),
        &[ctx.accounts.token_authority.bump]
    ];

    token_interface::thaw_account(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            ThawAccount {
                account: ctx.accounts.token_account.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                authority: ctx.accounts.token_authority.to_account_info(),
            },
            &[authority_seeds]
        )
    )?;

    emit!(TokenAccountThawed {
        mint: ctx.accounts.mint.key(),
        token_account: ctx.accounts.token_account.key(),
        admin: ctx.accounts.admin.key(),
    });

    msg!("Thawed token account {}", ctx.accounts.token_account.key());
    Ok(())
}