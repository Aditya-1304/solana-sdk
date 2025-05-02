use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken,  token_interface::{self, TokenAccount, TokenInterface, Mint, MintTo, TransferChecked, Burn, FreezeAccount, ThawAccount}};
use anchor_lang::system_program;

declare_id!("DDnDEV5j1HkJzzV94sLaEi11e2CjXfTFRpQv1amgLxTr");

#[program]
pub mod token_module {
    
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
        require!(name.len() <= 32, TokenError::NameTooLong);
        require!(symbol.len() <= 10, TokenError::SymbolTooLong);
        if let Some(ref uri_str) = uri {
            require!(uri_str.len() <= 200, TokenError::UriTooLong);
        }
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

        emit!(TokenMinted {
            mint: ctx.accounts.mint.key(),
            destination: ctx.accounts.destination.key(),
            amount,
            new_supply: ctx.accounts.mint.supply,
        });

        msg!("Minted {} tokens to {}", amount, ctx.accounts.destination.key());
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

    pub fn burn_tokens(
        ctx: Context<BurnTokens>,
        amount: u64,
    ) -> Result<()> {
        require!(amount>0, TokenError::ZeroAmount);
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

    pub fn create_escrow(
        ctx: Context<CreateEscrow>,
        amount: u64,
        seed: [u8; 32]
    ) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;
        escrow.sender = ctx.accounts.sender.key();
        escrow.mint = ctx.accounts.mint.key();
        escrow.amount = amount;
        escrow.seed = seed;
        escrow.recipient = None;
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
            escrow: ctx.accounts.escrow.key(),
            sender: ctx.accounts.sender.key(),
            mint: ctx.accounts.mint.key(),
            amount,
            recipient: None,
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
        if escrow.recipient.is_none() {
            require!(
                escrow.sender == ctx.accounts.recipient.key() || 
                false,
                TokenError::UnauthorizedRecipient
            )
        }
        require!(!escrow.claimed, TokenError::EscrowAlreadyClaimed);

        escrow.claimed = true;

        let escrow_seeds = &[
            b"token_escrow",
            escrow.sender.as_ref(),
            escrow.mint.as_ref(),
            &escrow.seed,
            &[escrow.bump],
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
                &[escrow_seeds]
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

    pub fn freeze_token_account(ctx: Context<FreezeTokenAccount>) -> Result<()> {
        require!(
            ctx.accounts.admin.key() == ctx.accounts.token_metadata.admin,
            TokenError::UnauthorizedFreezeAuthority
        );

        let mint_key = ctx.accounts.mint.key();
        let authority_seeds = &[
            b"token_authority",
            mint_key.as_ref(),
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

        let mint_key = ctx.accounts.mint.key();
        let authority_seeds = &[
            b"token_authority",
            mint_key.as_ref(),
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
#[instruction(name: String, symbol: String, decimals: u8, uri: Option<String>)]
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
        space = 8 + // discriminator
            32 + // admin pubkey
            4 + name.len() + // string prefix + data
            4 + symbol.len() + // string prefix + data
            1 + // decimals
            1 + 4 + (uri.map_or(0, |s| s.len())) + // option prefix + string prefix + data
            9 + // option u64 for max supply
            32 + // mint pubkey
            1,   // bump
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


    #[account(
        init,
        payer = sender,
        associated_token::mint = mint,
        associated_token::authority = escrow_authority
    )]
    pub escrow_token_account: InterfaceAccount<'info, TokenAccount>,

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
        close = recipient,
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

    #[account(
        mut,
        constraint = recipient_token_account.mint == mint.key(),
        constraint = recipient_token_account.owner == recipient.key()
    )]
    pub recipient_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,

    pub system_program: Program<'info, System>,
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
    pub recipient: Option<Pubkey>,
    pub claimed: bool,
    pub bump:u8,

}

#[event]
pub struct TokenCreated {
    pub mint: Pubkey,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub admin: Pubkey,
    pub max_supply: Option<u64>,
}

#[event]
pub struct TokenMinted {
    pub mint: Pubkey,
    pub destination: Pubkey,
    pub amount: u64,
    pub new_supply: u64,
}

#[event]
pub struct TokensBurned {
    pub mint: Pubkey,
    pub token_account: Pubkey,
    pub amount: u64,
    pub new_supply: u64,
}

#[event]
pub struct TokenAccountFrozen {
    pub mint: Pubkey,
    pub token_account: Pubkey,
    pub admin: Pubkey,
}

#[event]
pub struct TokenAccountThawed {
    pub mint: Pubkey,
    pub token_account: Pubkey,
    pub admin: Pubkey,
}

#[event]
pub struct EscrowCreated {
    pub escrow: Pubkey,
    pub sender: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub recipient: Option<Pubkey>,
}

#[event]
pub struct EscrowReleased {
    pub escrow: Pubkey,
    pub recipient: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
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

    #[msg("Arithmetic overflow occurred")]
    ArithmeticOverflow,

    #[msg("Cannot perform operation on frozen account")]
    AccountFrozen,

    #[msg("Token name exceeds maximum length")]
    NameTooLong,

    #[msg("Token symbol exceeds maximum length")]
    SymbolTooLong,

    #[msg("Token URI exceeds maximum length")]
    UriTooLong,

    #[msg("Amount must be greater than zero")]
    ZeroAmount,
}