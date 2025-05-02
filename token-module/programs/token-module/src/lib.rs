use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenInterface;

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
pub struct TransferSol<'info> {
    #[account(mut)]
    pub from: Signer<'info>,
    ///Check: Receiver account can be any account, not necessarily initialized
    #[account(mut)]
    pub to: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MintSplToken<'info> {
    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub recipent_token_account: InterfaceAccount<'info, TokenAccount>,

    pub sender_authority: Signer<'info>,
    pub token_program: Interface<'info, TokenInterface>
}


#[derive(Accounts)]
pub struct BurnSplToken<'info> {
    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub sender_token_account: InterfaceAccount<'info, TokenAccount>,

    pub sender_authority: Signer<'info>,
    pub token_program: Interface<'info, TokenInterface>
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