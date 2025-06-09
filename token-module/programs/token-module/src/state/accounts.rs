use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{TokenAccount, TokenInterface, Mint}
};

#[derive(Accounts)]
pub struct InitializeTokenAuthority<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + TokenAuthority::INIT_SPACE,
        seeds = [b"token_authority".as_ref()],
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
        seeds = [b"token_authority".as_ref()],
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
        seeds = [b"token_metadata".as_ref(), mint.key().as_ref()],
        bump,
        constraint = token_metadata.mint == mint.key()
    )]
    pub token_metadata: Account<'info, TokenMetadata>, 

    #[account(
        seeds = [b"token_authority".as_ref()],
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
#[instruction(amount: u64, seed: [u8; 32], receipient: Option<Pubkey>)]
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
        space = 8 + 32 + 32 + 8 + 32 + (1+32) + 1 + 1, // Discriminator + keys + amount + seed + option<pubkey> + claimed + bump
        seeds = [b"token_escrow".as_ref(), sender.key().as_ref(), mint.key().as_ref(), &seed],
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
        init_if_needed,
        payer = recipient,
        associated_token::mint = mint,
        associated_token::authority = recipient,
        constraint = recipient_token_account.mint == mint.key(),
        // constraint = recipient_token_account.owner == recipient.key()
    )]
    pub recipient_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,

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
        seeds = [b"token_metadata".as_ref(), mint.key().as_ref()],
        bump = token_metadata.bump,
        constraint = token_metadata.mint == mint.key()
    )]
    pub token_metadata: Account<'info, TokenMetadata>,

    #[account(
        seeds = [b"token_authority".as_ref()],
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
        seeds = [b"token_metadata".as_ref(), mint.key().as_ref()],
        bump,
        constraint = token_metadata.mint == mint.key()
    )]
    pub token_metadata: Account<'info, TokenMetadata>,

    #[account(
        seeds = [b"token_authority".as_ref()],
        bump = token_authority.bump,
    )]
    pub token_authority: Account<'info, TokenAuthority>,

    pub token_program: Interface<'info, TokenInterface>,

}


#[account]
#[derive(InitSpace)]
pub struct TokenAuthority {
    pub admin: Pubkey,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct TokenMetadata {
    #[max_len(32)]
    pub name: String,
    #[max_len(10)]
    pub symbol: String,
    pub decimals: u8,
    #[max_len(200)]
    pub uri: Option<String>,
    pub max_supply: Option<u64>,
    pub admin: Pubkey,
    pub mint: Pubkey,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Escrow {
    pub sender: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub seed: [u8; 32],
    pub recipient: Option<Pubkey>,
    pub claimed: bool,
    pub bump:u8,

}