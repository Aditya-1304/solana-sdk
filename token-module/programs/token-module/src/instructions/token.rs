use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, MintTo, Burn};
use crate::{ TokenError, TokenCreated, TokenMinted, TokensBurned, CreateToken, MintTokens, BurnTokens};


pub fn create_token(
    ctx: Context<CreateToken>,
    name: String,
    symbol: String,
    decimals: u8,
    uri: Option<String>,
    max_supply: Option<u64>,
) -> Result<()> {
    // Validation
    require!(name.len() <= 32, TokenError::NameTooLong);
    require!(symbol.len() <= 10, TokenError::SymbolTooLong);
    if let Some(ref uri_str) = uri {
        require!(uri_str.len() <= 200, TokenError::UriTooLong);
    }

    // Initialize metadata
    let token_metadata = &mut ctx.accounts.token_metadata;
    token_metadata.name = name.clone();
    token_metadata.symbol = symbol.clone();
    token_metadata.decimals = decimals;
    token_metadata.uri = uri;
    token_metadata.max_supply = max_supply;
    token_metadata.admin = ctx.accounts.admin.key();
    token_metadata.mint = ctx.accounts.mint.key();
    token_metadata.bump = ctx.bumps.token_metadata;

    emit!(TokenCreated {
        mint: ctx.accounts.mint.key(),
        name: name.clone(),
        symbol: symbol.clone(),
        decimals,
        admin: ctx.accounts.admin.key(),
        max_supply,
    });

    msg!("Created new token {} ({})", name, symbol);
    Ok(())
}
pub fn mint_tokens(
    ctx: Context<MintTokens>,
    amount: u64,
) -> Result<()> {
    require!(amount>0, TokenError::ZeroAmount);

    if let Some(max_supply) = ctx.accounts.token_metadata.max_supply {
        let current_supply = ctx.accounts.mint.supply;
        // require!(
        //     current_supply.checked_add(amount).unwrap() <= max_supply,
        //     TokenError::ExceedsMaxSupply
        // );

        if let Some(new_supply) = current_supply.checked_add(amount) {
            require!(
                new_supply <= max_supply,
                TokenError::ExceedsMaxSupply
            )
        } else {
            return Err(error!(TokenError::ArithmeticOverflow));
        }
    }

    require!(
        ctx.accounts.admin.key() == ctx.accounts.token_metadata.admin,
        TokenError::UnauthorizedMintAuthority
    );

    msg!("Token Authority PDA for minting: {}", ctx.accounts.token_authority.key());
    msg!("Token Authority Bump for minting: {}", ctx.accounts.token_authority.bump);
    msg!("Mint account for minting: {}", ctx.accounts.mint.key());
    msg!("Mint authority according to mint account: {}", ctx.accounts.mint.mint_authority.unwrap());


    let _mint_key = ctx.accounts.mint.key();
    let authority_seeds = &[
        b"token_authority".as_ref(),
        &[ctx.accounts.token_authority.bump]
    ];

    token_interface::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.destination.to_account_info(),
                authority: ctx.accounts.token_authority.to_account_info()
            },
            &[authority_seeds]

        ),
        amount
    )?;

    emit!(TokenMinted {
        mint: ctx.accounts.mint.key(),
        destination: ctx.accounts.destination.key(),
        amount,
        new_supply: ctx.accounts.mint.supply,
    });

    msg!("Minted {} tokens to {}", amount, ctx.accounts.destination.key());
    Ok(())
}

pub fn burn_tokens(ctx: Context<BurnTokens>, amount: u64) -> Result<()> {
    require!(amount > 0, TokenError::ZeroAmount);
    require!(
        !ctx.accounts.token_account.is_frozen(),
        TokenError::AccountFrozen
    );

    token_interface::burn(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                mint: ctx.accounts.mint.to_account_info(),
                from: ctx.accounts.token_account.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(),
            },
        ),
        amount
    )?;

    emit!(TokensBurned {
        mint: ctx.accounts.mint.key(),
        token_account: ctx.accounts.token_account.key(),
        amount,
        new_supply: ctx.accounts.mint.supply,
    });

    msg!("Burned {} tokens from {}", amount, ctx.accounts.token_account.key());
    Ok(())
}