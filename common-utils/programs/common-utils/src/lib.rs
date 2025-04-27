use anchor_lang::prelude::*;

declare_id!("Ai4JnBGB2qmd5LJFcSTKwwwEp2tBYfXLbAt4pC54dGh6");

#[program]
pub mod common_utils {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
