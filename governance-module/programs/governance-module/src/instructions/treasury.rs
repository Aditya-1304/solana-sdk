use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};
use anchor_spl::token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked};
use crate::errors::*;
use crate::events::*;
use crate::state::*;

pub fn initialize_treasury(
    ctx: Context<InitializeTreasury>,
) -> Result<()> {
    let treasury = &mut ctx.accounts.treasury;
    let governance = &ctx.accounts.governance;

    treasury.governance = governance.key();
    treasury.authority_bump = ctx.bumps.treasury_authority;
    treasury.initialized = true;

    emit!(TreasuryInitialized {
        governance: governance.key(),
        treasury: treasury.key(),
        token_mint: None,
    });

    msg!("Treasury initialized for governance: {}", governance.key());
    Ok(())
}

pub fn execute_treasury_sol_transfer(
    ctx: Context<ExecuteTreasurySolTransfer>,
    _proposal_id: u64,
    amount: u64,
) -> Result<()> {
    let proposal = &ctx.accounts.proposal;
    let governance = &ctx.accounts.governance;

    // Validate proposal
    require!(proposal.executed, GovernanceError::ProposalNotExecuted);
    require!(proposal.status == ProposalStatus::Passed, GovernanceError::ProposalNotPassed);

    // Validate proposal type
    if let ProposalType::Treasury { recipient, amount: proposal_amount, token_mint } = &proposal.proposal_type {
        require!(token_mint.is_none(), GovernanceError::InvalidProposalType);
        require!(*recipient == ctx.accounts.recipient.key(), GovernanceError::InvalidRecipient);
        require!(*proposal_amount == amount, GovernanceError::InvalidTreasuryAmount);
    } else {
        return Err(GovernanceError::InvalidProposalType.into());
    }

    // Verify treasury has sufficient funds
    let treasury_balance = ctx.accounts.treasury_authority.lamports();
    require!(treasury_balance >= amount, GovernanceError::InsufficientTreasuryFunds);

    // Transfer SOL from treasury to recipient
    let governance_key = governance.key();
    let signer_seeds: &[&[&[u8]]] = &[&[
        b"treasury_authority",
        governance_key.as_ref(),
        &[ctx.accounts.treasury.authority_bump],
    ]];

    transfer(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.treasury_authority.to_account_info(),
                to: ctx.accounts.recipient.to_account_info(),
            },
            signer_seeds,
        ),
        amount,
    )?;

    emit!(TreasuryWithdrawal {
        governance: governance.key(),
        proposal_id: proposal.proposal_id,
        recipient: ctx.accounts.recipient.key(),
        amount,
        token_mint: None,
    });

    msg!("Transferred {} SOL from treasury to {}", amount, ctx.accounts.recipient.key());
    Ok(())
}

pub fn execute_treasury_token_transfer(
    ctx: Context<ExecuteTreasuryTokenTransfer>,
    _proposal_id: u64,
    amount: u64,
) -> Result<()> {
    let proposal = &ctx.accounts.proposal;
    let governance = &ctx.accounts.governance;
    let token_mint = &ctx.accounts.token_mint;

    // Validate proposal
    require!(proposal.executed, GovernanceError::ProposalNotExecuted);
    require!(proposal.status == ProposalStatus::Passed, GovernanceError::ProposalNotPassed);

    // Validate proposal type
    if let ProposalType::Treasury { recipient, amount: proposal_amount, token_mint: mint } = &proposal.proposal_type {
        require!(mint.is_some(), GovernanceError::InvalidProposalType);
        require!(mint.unwrap() == token_mint.key(), GovernanceError::InvalidTreasuryToken);
        require!(*proposal_amount == amount, GovernanceError::InvalidTreasuryAmount);
        require!(
            ctx.accounts.recipient_token_account.owner == *recipient,
            GovernanceError::InvalidRecipient
        );
    } else {
        return Err(GovernanceError::InvalidProposalType.into());
    }

    // Verify treasury has sufficient funds
    require!(
        ctx.accounts.treasury_token_account.amount >= amount,
        GovernanceError::InsufficientTreasuryFunds
    );

    // Transfer tokens from treasury to recipient
    let governance_key = governance.key();
    let signer_seeds: &[&[&[u8]]] = &[&[
        b"treasury_authority",
        governance_key.as_ref(),
        &[ctx.accounts.treasury.authority_bump],
    ]];

    transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.treasury_token_account.to_account_info(),
                mint: token_mint.to_account_info(),
                to: ctx.accounts.recipient_token_account.to_account_info(),
                authority: ctx.accounts.treasury_authority.to_account_info(),
            },
            signer_seeds,
        ),
        amount,
        token_mint.decimals,
    )?;

    emit!(TreasuryWithdrawal {
        governance: governance.key(),
        proposal_id: proposal.proposal_id,
        recipient: ctx.accounts.recipient_token_account.owner,
        amount,
        token_mint: Some(token_mint.key()),
    });

    msg!("Transferred {} tokens from treasury to {}", amount, ctx.accounts.recipient_token_account.owner);
    Ok(())
}

#[derive(Accounts)]
pub struct InitializeTreasury<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    pub governance: Account<'info, Governance>,

    #[account(
        init,
        payer = authority,
        space = Treasury::SPACE,
        seeds = [b"treasury", governance.key().as_ref()],
        bump,
    )]
    pub treasury: Account<'info, Treasury>,

    /// CHECK: Treasury authority PDA
    #[account(
        seeds = [b"treasury_authority", governance.key().as_ref()],
        bump,
    )]
    pub treasury_authority: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(proposal_id: u64)]
pub struct ExecuteTreasurySolTransfer<'info> {
    pub executor: Signer<'info>,

    pub governance: Account<'info, Governance>,

    #[account(
        seeds = [b"proposal", governance.key().as_ref(), &proposal_id.to_le_bytes()],
        bump = proposal.bump,
    )]
    pub proposal: Account<'info, Proposal>,

    #[account(
        seeds = [b"treasury", governance.key().as_ref()],
        bump,
    )]
    pub treasury: Account<'info, Treasury>,

    /// CHECK: Treasury authority PDA that holds the funds
    #[account(
        mut,
        seeds = [b"treasury_authority", governance.key().as_ref()],
        bump = treasury.authority_bump,
    )]
    pub treasury_authority: AccountInfo<'info>,

    /// CHECK: Recipient account for SOL transfer
    #[account(mut)]
    pub recipient: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(proposal_id: u64)]
pub struct ExecuteTreasuryTokenTransfer<'info> {
    pub executor: Signer<'info>,

    pub governance: Account<'info, Governance>,

    #[account(
        seeds = [b"proposal", governance.key().as_ref(), &proposal_id.to_le_bytes()],
        bump = proposal.bump,
    )]
    pub proposal: Account<'info, Proposal>,

    #[account(
        seeds = [b"treasury", governance.key().as_ref()],
        bump,
    )]
    pub treasury: Account<'info, Treasury>,

    /// CHECK: Treasury authority PDA
    #[account(
        seeds = [b"treasury_authority", governance.key().as_ref()],
        bump = treasury.authority_bump,
    )]
    pub treasury_authority: AccountInfo<'info>,

    #[account(mut)]
    pub treasury_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(mut)]
    pub recipient_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_mint: InterfaceAccount<'info, Mint>,

    pub token_program: Interface<'info, TokenInterface>,
}