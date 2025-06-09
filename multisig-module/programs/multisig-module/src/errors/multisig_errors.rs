use anchor_lang::prelude::*;

#[error_code]
pub enum MultisigError {
    #[msg("Invalid threshold: must be > 0 and <= number of owners")]
    InvalidThreshold,
    #[msg("No owners provided")]
    NoOwners,
    #[msg("Too many owners: maximum 10 allowed")]
    TooManyOwners,
    #[msg("Duplicate owners not allowed")]
    DuplicateOwners,
    #[msg("Invalid owner: cannot be default pubkey")]
    InvalidOwner,
    #[msg("Owner not found")]
    OwnerNotFound,
    #[msg("Already approved")]
    AlreadyApproved,
    #[msg("Not enough approvals")]
    NotEnoughApprovals,
    #[msg("Transaction already executed")]
    AlreadyExecuted,
    #[msg("Empty transaction data")]
    EmptyTransaction,
    #[msg("Transaction data too large")]
    TransactionTooLarge,
    #[msg("Invalid transaction ID")]
    InvalidTransactionId,
    #[msg("Invalid transaction")]
    InvalidTransaction,
    #[msg("Transaction count overflow")]
    TransactionCountOverflow,
    #[msg("Approval count overflow")]
    ApprovalCountOverflow,
    #[msg("Approval array length mismatch")]
    ApprovalArrayMismatch,
    #[msg("Clock unavailable")]
    ClockUnavailable,
    #[msg("Multisig is paused")]
    MultisigPaused,
    #[msg("Multisig is not paused")]
    NotPaused,
    #[msg("Transaction not executed")]
    TransactionNotExecuted,
    #[msg("Invalid multisig state")]
    InvalidMultisigState,
    #[msg("Invalid transaction state")]
    InvalidTransactionState,
    #[msg("Transaction expired")]
    TransactionExpired,
    #[msg("Invalid nonce")]
    InvalidNonce,
    #[msg("Nonce overflow")]
    NonceOverflow,
    #[msg("Not enough admin approvals")]
    NotEnoughAdminApprovals,
    #[msg("Same slot execution not allowed")]
    SameSlotExecution,
    #[msg("Rate limit exceeded")]
    RateLimitExceeded,
    #[msg("Invalid transaction type")]
    InvalidTransactionType,
    #[msg("Invalid admin threshold")]
    InvalidAdminThreshold,
    #[msg("Transaction too complex")]
    TransactionTooComplex,
}