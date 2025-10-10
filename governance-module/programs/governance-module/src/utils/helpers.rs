use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenAccount;
use crate::errors::*;
use crate::state::*;

/// Calculate voting power based on governance configuration
pub fn get_voting_power(
    voter: &Pubkey,
    config: &GovernanceConfig,
    token_account: Option<&InterfaceAccount<TokenAccount>>,
) -> Result<u64> {
    match config.voting_type {
        VotingType::Equal => Ok(1),
        VotingType::TokenWeighted => {
            let token_account = token_account.ok_or(GovernanceError::InvalidTokenAccount)?;
            
            require!(
                token_account.owner == *voter,
                GovernanceError::InvalidTokenAccountOwner
            );

            if let Some(gov_token) = config.governance_token {
                require!(
                    token_account.mint == gov_token,
                    GovernanceError::InvalidGovernanceToken
                );
            }

            let balance = token_account.amount;
            
            if let Some(multiplier) = config.voting_power_multiplier {
                balance.checked_mul(multiplier)
                    .ok_or(GovernanceError::VotingPowerOverflow.into())
            } else {
                Ok(balance)
            }
        }
    }
}

/// Check if proposal has met quorum and update status
pub fn check_and_finalize_proposal(
    proposal: &mut Account<Proposal>,
    governance: &Account<Governance>,
) -> Result<()> {
    let total_votes = proposal.votes_for + proposal.votes_against + proposal.votes_abstain;

    let required_quorum = match governance.config.voting_type {
        VotingType::Equal => {
            governance.config.quorum_percentage as u64
        },
        VotingType::TokenWeighted => {
            (governance.total_voting_power * governance.config.quorum_percentage as u64) / 100
        }
    };

    if total_votes >= required_quorum.max(1) {
        if proposal.votes_for > proposal.votes_against {
            proposal.status = ProposalStatus::Passed;
            msg!("Proposal {} passed! For: {}, Against: {}", 
                 proposal.proposal_id, proposal.votes_for, proposal.votes_against);
        } else {
            proposal.status = ProposalStatus::Failed;
            msg!("Proposal {} failed! For: {}, Against: {}", 
                 proposal.proposal_id, proposal.votes_for, proposal.votes_against);
        }
    }
    Ok(())
}

/// Validate proposal type specific requirements
pub fn validate_proposal_type(
    proposal_type: &ProposalType,
    instruction_data: &[u8],
) -> Result<()> {
    match proposal_type {
        ProposalType::Treasury { amount, .. } => {
            require!(*amount > 0, GovernanceError::InvalidTreasuryAmount);
        },
        ProposalType::ConfigChange { parameter, new_value } => {
            require!(!parameter.is_empty(), GovernanceError::EmptyConfigParameter);
            require!(!new_value.is_empty(), GovernanceError::EmptyConfigValue);
        },
        ProposalType::Emergency { .. } => {
            // Emergency proposals are always valid
        },
        ProposalType::Custom { target_program, .. } => {
            require!(
                *target_program != Pubkey::default(),
                GovernanceError::InvalidProposalType
            );
        },
    }
    
    require!(!instruction_data.is_empty(), GovernanceError::EmptyInstruction);
    Ok(())
}