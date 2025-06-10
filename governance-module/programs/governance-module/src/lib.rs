use anchor_lang::prelude::*;

declare_id!("HJzW17DkivXRYjirjDD56a3Pve6JFnKhmsfpswJQ3St4");

#[program]
pub mod governance_module {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}


#[account]
pub struct Governance {
    pub authority: Pubkey,
    pub name: String,
    pub description: String,
    pub config: GovernanceConfig,
    pub proposal_count: u64,
    pub total_voting_power: u64,
    pub paused: bool,
    pub created_at: i64,
    pub bump: u8,
}

#[account]
pub struct Proposal {
    pub governance: Pubkey,
    pub proposal_id: u64,
    pub proposer: Pubkey,
    pub title: String,
    
}
