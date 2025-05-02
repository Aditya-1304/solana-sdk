use anchor_lang::prelude::*;
use anchor_spl::{token::Mint, token_interface::TokenInterface};

declare_id!("DDnDEV5j1HkJzzV94sLaEi11e2CjXfTFRpQv1amgLxTr");

#[program]
pub mod token_module {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
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