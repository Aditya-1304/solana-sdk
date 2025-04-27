use anchor_lang::prelude::*;

declare_id!("8qzfg49CMM4u8UG6LaVhT4WuHC1CrgnrE8jYBzMFgvuZ");

#[program]
pub mod multisig_module {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
