use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken,  token_interface::{TokenAccount, TokenInterface, Mint, MintTo, TransferChecked, Burn}};

declare_id!("DDnDEV5j1HkJzzV94sLaEi11e2CjXfTFRpQv1amgLxTr");

#[program]
pub mod token_module {
    use anchor_lang::system_program;
    use anchor_spl::token_interface;

    use super::*;

    pub fn initialize_token_authority(ctx: Context<InitializeTokenAuthority>) -> Result<()> {

        let authority = &mut ctx.accounts.token_authority;
        authority.admin = ctx.accounts.admin.key();
        authority.bump = ctx.bumps.token_authority;
        msg!("Token authority initialized with admin: {}", authority.admin);
        Ok(())
    }

    pub fn create_token(
        ctx: Context<CreateToken>,
        name: String,
        symbol: String,
        decimals: u8,
        uri: Option<String>,
        max_supply: Option<u64>,
    ) -> Result<()> {
        let token_metadata = &mut ctx.accounts.token_metadata;
        token_metadata.name = name.clone();
        token_metadata.symbol = symbol.clone();
        token_metadata.decimals = decimals;
        token_metadata.uri = uri;
        token_metadata.max_supply = max_supply;
        token_metadata.admin = ctx.accounts.admin.key();
        token_metadata.mint = ctx.accounts.mint.key();
        token_metadata.bump = ctx.bumps.token_metadata;

        msg!("Created new token {} ({})", name, symbol);
        Ok(())
    }

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

    pub fn mint_tokens(
        ctx: Context<MintTokens>,
        amount: u64,
    ) -> Result<()> {

        if let Some(max_supply) = ctx.accounts.token_metadata.max_supply {
            let current_supply = ctx.accounts.mint.supply;
            require!(
                current_supply.checked_add(amount).unwrap() <= max_supply,
                TokenError::ExceedsMaxSupply
            );
        }

        require!(
            ctx.accounts.admin.key() == ctx.accounts.token_metadata.admin || ctx.accounts.admin.key() == ctx.accounts.token_authority.admin,
            TokenError::UnauthorizedMintAuthority
        );

        let mint_key = ctx.accounts.mint.key();
        let authority_seeds = &[
            b"token_authority",
            mint_key.as_ref(),
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

        msg!("Minted {} tokens to {}", amount, ctx.accounts.destination.key());
        Ok(())
    }

    pub fn transfer_tokens(
        ctx: Context<TransferTokens>,
        amount: u64,
    ) -> Result<()> {
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

    pub fn burn_tokens(
        ctx: Context<BurnTokens>,
        amount: u64,
    ) -> Result<()> {
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

        msg!("Burned {} tokens from {}", amount, ctx.accounts.token_account.key());
        Ok(())
    }

}



#[derive(Accounts)]
pub struct InitializeTokenAuthority<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + 32 + 1,
        seeds = [b"token_authority"],
        bump,
    )]
    pub token_authority: Account<'info, TokenAuthority>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(name: String, symbol: String, decimals: u8)]
pub struct CreateToken<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        mint::decimals = decimals,
        mint::authority = token_authority,
        mint::freeze_authority = token_authority,
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = admin,
        space = 8 + 32 + 4 + name.len() + 4 + symbol.len() + 1 + 1 + 8 + 32 + 32 + 1,
        seeds = [b"token_metadata", mint.key().as_ref()],
        bump
    )]
    pub token_metadata: Account<'info, TokenMetadata>,

    ///CHECK: This is the token autority PDA
    #[account(
        seeds = [b"token_authority"],
        bump
    )]
    pub token_authority: Account<'info, TokenAuthority>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info,Rent>,

}

#[derive(Accounts)]
pub struct TransferSol<'info> {
    #[account(mut)]
    pub from: Signer<'info>,

    #[account(mut)]
    ///CHECK: Receiver account - system program handles validation 
    pub to: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MintTokens<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub destination: InterfaceAccount<'info, TokenAccount>,

    #[account(
        seeds = [b"token_metadata", mint.key().as_ref()],
        bump,
        constraint = token_metadata.mint == mint.key()
    )]
    pub token_metadata: Account<'info, TokenMetadata>, 

    #[account(
        seeds = [b"token_authority"],
        bump = token_authority.bump,
    )]
    pub token_authority: Account<'info, TokenAuthority>,

    pub token_program: Interface<'info, TokenInterface>,
}


#[derive(Accounts)]
pub struct TransferTokens<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    pub mint: InterfaceAccount<'info, Mint>,
    
    #[account(
        mut,
        constraint = from_account.mint == mint.key(),
        constraint = from_account.owner == owner.key(),
    )]
    pub from_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        constraint = to_account.mint == mint.key()
    )]
    pub to_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>
}

#[derive(Accounts)]
pub struct BurnTokens<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut, 
        constraint = token_account.mint == mint.key(),
        constraint = token_account.owner == owner.key()
    )]
    pub token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>
}

#[derive(Accounts)]
#[instruction(amount: u64, seed: [u8; 32])]
pub struct CreateEscrow<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        constraint = sender_token_account.mint == mint.key(),
        constraint = sender_token_account.owner == sender.key()
    )]
    pub sender_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init,
        payer = sender,
        space = 8 + 32 + 32 + 8 + 32 + 1 + 1 + 1, // Discriminator + keys + amount + seed + option<pubkey> + claimed + bump
        seeds = [b"token_escrow", sender.key().as_ref(), mint.key().as_ref(), &seed],
        bump
    )]
    pub escrow: Account<'info, Escrow>,

    ///CHECK: This is a PDA that will own the escrow token account
    #[account(
        seeds = [b"escrow_authority", escrow.key().as_ref()],
        bump
    )]
    pub escrow_authority: AccountInfo<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,

}
#[derive(Accounts)]
pub struct ReleaseEscrow<'info> {
    #[account(mut)]
    pub recipient: Signer<'info>,

    #[account(
        mut,
        constraint = !escrow.claimed,
    )]
    pub escrow: Account<'info, Escrow>,

    pub mint: InterfaceAccount<'info, Mint>,

    ///CHECK: This PDA owns the escrow token account
    #[account(
        seeds = [b"escrow_authority", escrow.key().as_ref()],
        bump
    )]
    pub escrow_authority: AccountInfo<'info>,

    #[account(
        mut,
        constraint = escrow_token_account.mint == mint.key(),
        constraint = escrow_token_account.owner == escrow_authority.key()
    )]
    pub escrow_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}

#[derive(Accounts)]
pub struct FreezeTokenAccount<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        constraint = token_account.mint == mint.key()
    )]
    pub token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        seeds = [b"token_metadata", mint.key().as_ref()],
        bump,
        constraint = token_metadata.mint == mint.key()
    )]
    pub token_metadata: Account<'info, TokenMetadata>,

    #[account(
        seeds = [b"token_authority"],
        bump = token_authority.bump,
    )]
    pub token_authority: Account<'info, TokenAuthority>,

    pub token_program: Interface<'info, TokenInterface>,
}

#[derive(Accounts)]
pub struct ThawTokenAccount<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        constraint = token_account.mint == mint.key()
    )]
    pub token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        seeds = [b"token_metadata", mint.key().as_ref()],
        bump,
        constraint = token_metadata.mint == mint.key()
    )]
    pub token_metadata: Account<'info, TokenMetadata>,

    #[account(
        seeds = [b"token_authority"],
        bump = token_authority.bump,
    )]
    pub token_authority: Account<'info, TokenAuthority>,

    pub token_program: Interface<'info, TokenInterface>,

}


#[account]
pub struct TokenAuthority {
    pub admin: Pubkey,
    pub bump: u8,
}

#[account]
pub struct TokenMetadata {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub uri: Option<String>,
    pub max_supply: Option<u64>,
    pub admin: Pubkey,
    pub mint: Pubkey,
    pub bump: u8,
}

#[account]
pub struct Escrow {
    pub sender: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub seed: [u8; 32],
    pub recipent: Option<Pubkey>,
    pub claimed: bool,
    pub bump:u8,

}

#[error_code]
pub enum TokenError {
    #[msg("Unauthorized mint authority")]
    UnauthorizedMintAuthority,

    #[msg("Unauthorized freeze authority")]
    UnauthorizedFreezeAuthority,

    #[msg("Unauthorized recipient for escrow")]
    UnauthorizedRecipient,

    #[msg("Escrow has already been claimed")]
    EscrowAlreadyClaimed,

    #[msg("Mint would exceed max supply")]
    ExceedsMaxSupply,
}