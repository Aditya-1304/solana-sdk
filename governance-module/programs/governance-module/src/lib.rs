use anchor_lang::prelude::*;

declare_id!("HJzW17DkivXRYjirjDD56a3Pve6JFnKhmsfpswJQ3St4");

pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;
pub mod utils;

pub use errors::*;
pub use events::*;
pub use instructions::*;
pub use state::*;
pub use utils::*;

#[program]
pub mod governance_module {
    use super::*;

    // ==================== GOVERNANCE ====================
    
    pub fn create_governance(
        ctx: Context<CreateGovernance>,
        name: String,
        description: String,
        config: GovernanceConfig,
    ) -> Result<()> {
        instructions::governance::create_governance(ctx, name, description, config)
    }

    // ==================== PROPOSALS ====================

    pub fn create_proposal(
        ctx: Context<CreateProposal>,
        title: String,
        description: String,
        proposal_type: ProposalType,
        instruction_data: Vec<u8>,
    ) -> Result<()> {
        instructions::proposal::create_proposal(ctx, title, description, proposal_type, instruction_data)
    }

    pub fn cancel_proposal(
        ctx: Context<CancelProposal>,
        proposal_id: u64,
        reason: String,
    ) -> Result<()> {
        instructions::proposal::cancel_proposal(ctx, proposal_id, reason)
    }

    pub fn execute_proposal(
        ctx: Context<ExecuteProposal>,
        proposal_id: u64,
    ) -> Result<()> {
        instructions::admin::execute_proposal(ctx, proposal_id)
    }

    // ==================== VOTING ====================

    pub fn vote(
        ctx: Context<Vote>,
        proposal_id: u64,
        vote_type: VoteType,
    ) -> Result<()> {
        instructions::voting::vote(ctx, proposal_id, vote_type)
    }

    // ==================== DELEGATION ====================

    pub fn delegate_voting_power(
        ctx: Context<DelegateVotingPower>,
        amount: u64,
    ) -> Result<()> {
        instructions::delegation::delegate_voting_power(ctx, amount)
    }

    pub fn revoke_delegation(
        ctx: Context<RevokeDelegation>,
    ) -> Result<()> {
        instructions::delegation::revoke_delegation(ctx)
    }

    // ==================== TREASURY ====================

    pub fn initialize_treasury(
        ctx: Context<InitializeTreasury>,
    ) -> Result<()> {
        instructions::treasury::initialize_treasury(ctx)
    }

    pub fn execute_treasury_sol_transfer(
        ctx: Context<ExecuteTreasurySolTransfer>,
        proposal_id: u64,
        amount: u64,
    ) -> Result<()> {
        instructions::treasury::execute_treasury_sol_transfer(ctx, proposal_id, amount)
    }

    pub fn execute_treasury_token_transfer(
        ctx: Context<ExecuteTreasuryTokenTransfer>,
        proposal_id: u64,
        amount: u64,
    ) -> Result<()> {
        instructions::treasury::execute_treasury_token_transfer(ctx, proposal_id, amount)
    }

    // ==================== ADMIN ====================

    pub fn emergency_pause(
        ctx: Context<EmergencyPause>,
    ) -> Result<()> {
        instructions::admin::emergency_pause(ctx)
    }

    pub fn unpause(
        ctx: Context<Unpause>,
    ) -> Result<()> {
        instructions::admin::unpause(ctx)
    }

    pub fn update_governance_config(
        ctx: Context<UpdateGovernanceConfig>,
        proposal_id: u64,
        new_config: GovernanceConfig,
    ) -> Result<()> {
        instructions::admin::update_governance_config(ctx, proposal_id, new_config)
    }
}