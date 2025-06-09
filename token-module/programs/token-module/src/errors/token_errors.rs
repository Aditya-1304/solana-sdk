use anchor_lang::prelude::*;

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