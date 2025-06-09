use anchor_lang::prelude::*;

declare_id!("DDnDEV5j1HkJzzV94sLaEi11e2CjXfTFRpQv1amgLxTr");

// Import modules
pub mod instructions;
pub mod state;
pub mod events;
pub mod errors;

// ✅ FIXED: Import only Context structs and types, NOT functions
pub use state::*;           // Account structs and Context structs
pub use events::*;          // Event structs  
pub use errors::*;          // Error enums

// ✅ FIXED: Import instruction functions with module path to avoid conflicts
use instructions::{admin, token, escrow, transfer};

#[program]
pub mod token_module {
    use super::*;

    // Delegate to instruction handlers using module paths
    pub fn initialize_token_authority(ctx: Context<InitializeTokenAuthority>) -> Result<()> {
        admin::initialize_token_authority(ctx)
    }

    pub fn create_token(
        ctx: Context<CreateToken>,
        name: String,
        symbol: String,
        decimals: u8,
        uri: Option<String>,
        max_supply: Option<u64>,
    ) -> Result<()> {
        token::create_token(ctx, name, symbol, decimals, uri, max_supply)
    }

    pub fn transfer_sol(ctx: Context<TransferSol>, amount: u64) -> Result<()> {
        transfer::transfer_sol(ctx, amount)
    }

    pub fn mint_tokens(ctx: Context<MintTokens>, amount: u64) -> Result<()> {
        token::mint_tokens(ctx, amount)
    }

    pub fn transfer_tokens(ctx: Context<TransferTokens>, amount: u64) -> Result<()> {
        transfer::transfer_tokens(ctx, amount)
    }

    pub fn burn_tokens(ctx: Context<BurnTokens>, amount: u64) -> Result<()> {
        token::burn_tokens(ctx, amount)
    }

    pub fn create_escrow(
        ctx: Context<CreateEscrow>,
        amount: u64,
        seed: [u8; 32],
        recipient: Option<Pubkey>,
    ) -> Result<()> {
        escrow::create_escrow(ctx, amount, seed, recipient)
    }

    pub fn release_escrow(ctx: Context<ReleaseEscrow>) -> Result<()> {
        escrow::release_escrow(ctx)
    }

    pub fn freeze_token_account(ctx: Context<FreezeTokenAccount>) -> Result<()> {
        admin::freeze_token_account(ctx)
    }

    pub fn thaw_token_account(ctx: Context<ThawTokenAccount>) -> Result<()> {
        admin::thaw_token_account(ctx)
    }
}