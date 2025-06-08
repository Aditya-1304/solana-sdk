use anchor_lang::prelude::*;


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