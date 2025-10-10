use anchor_lang::prelude::*;

#[error_code]
pub enum GovernanceError {
    #[msg("Name too long")]
    NameTooLong,
    #[msg("Description too long")]
    DescriptionTooLong,
    #[msg("Invalid quorum percentage")]
    InvalidQuorum,
    #[msg("Invalid voting period")]
    InvalidVotingPeriod,
    #[msg("Invalid execution delay")]
    InvalidExecutionDelay,
    #[msg("Governance is paused")]
    GovernancePaused,
    #[msg("Title too long")]
    TitleTooLong,
    #[msg("Empty instruction")]
    EmptyInstruction,
    #[msg("Instruction too large")]
    InstructionTooLarge,
    #[msg("Insufficient voting power")]
    InsufficientVotingPower,
    #[msg("Proposal not active")]
    ProposalNotActive,
    #[msg("Voting period ended")]
    VotingPeriodEnded,
    #[msg("No voting power")]
    NoVotingPower,
    #[msg("Proposal not passed")]
    ProposalNotPassed,
    #[msg("Already executed")]
    AlreadyExecuted,
    #[msg("Execution delay not met")]
    ExecutionDelayNotMet,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Reason too long")]
    ReasonTooLong,
    #[msg("Invalid proposal")]
    InvalidProposal,
    #[msg("Proposal not executed")]
    ProposalNotExecuted,
    #[msg("Invalid proposal type")]
    InvalidProposalType,
    #[msg("Missing governance token")]
    MissingGovernanceToken,
    #[msg("Invalid governance token")]
    InvalidGovernanceToken,
    #[msg("Invalid token account")]
    InvalidTokenAccount,
    #[msg("Invalid token account owner")]
    InvalidTokenAccountOwner,
    #[msg("Voting power overflow")]
    VotingPowerOverflow,
    #[msg("Invalid treasury amount")]
    InvalidTreasuryAmount,
    #[msg("Empty config parameter")]
    EmptyConfigParameter,
    #[msg("Empty config value")]
    EmptyConfigValue,
    #[msg("Insufficient treasury funds")]
    InsufficientTreasuryFunds,
    #[msg("Delegation already exists")]
    DelegationAlreadyExists,
    #[msg("Delegation not found")]
    DelegationNotFound,
    #[msg("Cannot delegate to self")]
    CannotDelegateToSelf,
    #[msg("Invalid delegate")]
    InvalidDelegate,
    #[msg("Treasury not initialized")]
    TreasuryNotInitialized,
    #[msg("Invalid treasury token")]
    InvalidTreasuryToken,
    #[msg("Proposal queue full")]
    ProposalQueueFull,
}