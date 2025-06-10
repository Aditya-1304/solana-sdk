use anchor_lang::prelude::*;

declare_id!("J5q2xymUufzUCvKT8gKwYpaNRqJQBz7xaAeuM29vxTw5");

#[program]
pub mod governanace_module {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
