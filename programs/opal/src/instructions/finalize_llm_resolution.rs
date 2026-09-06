use crate::{
    constants::{
        ASSERTION_SEED, ASSERTION_STATE_ASSERTED_LLM, ASSERTION_STATE_RESOLVED, BOND_VAULT_SEED,
        LLM_DISPUTE_SEED, LLM_ROUND_SEED, OUTCOME_TRUE, PROTOCOL_CONFIG_SEED,
    },
    errors::OpalError,
    state::{AssertionAccount, LlmDisputeAccount, LlmResolutionRound, ProtocolConfig},
    utils::{checked_bps, is_outcome_set, is_pubkey_default, is_timestamp_set},
};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct FinalizeLlmResolutionArgs {
    pub assertion_id: Pubkey,
}

#[derive(Accounts)]
#[instruction(args: FinalizeLlmResolutionArgs)]
pub struct FinalizeLlmResolution<'info> {
    pub finalizer: Signer<'info>,

    #[account(
        seeds = [PROTOCOL_CONFIG_SEED],
        bump,
    )]
    pub protocol_config: AccountLoader<'info, ProtocolConfig>,

    pub pusd_mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [ASSERTION_SEED, args.assertion_id.as_ref()],
        bump = assertion.load()?.bump,
    )]
    pub assertion: AccountLoader<'info, AssertionAccount>,

    #[account(
        mut,
        seeds = [LLM_DISPUTE_SEED, assertion.key().as_ref()],
        bump = llm_dispute.load()?.bump,
    )]
    pub llm_dispute: AccountLoader<'info, LlmDisputeAccount>,

    #[account(
        mut,
        seeds = [LLM_ROUND_SEED, assertion.key().as_ref()],
        bump = llm_resolution_round.load()?.bump,
    )]
    pub llm_resolution_round: AccountLoader<'info, LlmResolutionRound>,

    #[account(
        mut,
        seeds = [BOND_VAULT_SEED, args.assertion_id.as_ref()],
        bump,
        token::mint = pusd_mint,
        token::authority = assertion,
    )]
    pub bond_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = pusd_mint,
    )]
    pub asserter_pusd: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = pusd_mint,
    )]
    pub llm_disputer_pusd: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = pusd_mint,
    )]
    pub treasury_pusd: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(
    ctx: Context<FinalizeLlmResolution>,
    _args: FinalizeLlmResolutionArgs,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;

    let assertion = ctx.accounts.assertion.load()?;
    let llm_dispute = ctx.accounts.llm_dispute.load()?;
    let llm_round = ctx.accounts.llm_resolution_round.load()?;
    let protocol_config = ctx.accounts.protocol_config.load()?;

    require!(
        assertion.state == ASSERTION_STATE_ASSERTED_LLM,
        OpalError::InvalidState
    );
    require!(
        is_pubkey_default(&assertion.vote_dispute),
        OpalError::VoteDisputeAlreadyExists
    );
    require!(
        !is_outcome_set(llm_dispute.settlement_resolution),
        OpalError::AlreadySettled
    );
    require!(
        llm_dispute.assertion == ctx.accounts.assertion.key(),
        OpalError::AssertionLinkMismatch
    );
    require!(
        llm_round.assertion == ctx.accounts.assertion.key(),
        OpalError::AssertionLinkMismatch
    );
    require!(
        llm_round.dispute == ctx.accounts.llm_dispute.key(),
        OpalError::RoundLinkMismatch
    );

    require!(
        is_timestamp_set(llm_round.challenge_deadline),
        OpalError::MissingChallengeDeadline
    );
    require!(
        now >= llm_round.challenge_deadline,
        OpalError::DeadlineNotReached
    );

    require!(
        is_outcome_set(llm_round.outcome),
        OpalError::LlmOutcomeMissing
    );
    let final_outcome = llm_round.outcome;

    let assertion_bond = assertion.assertion_bond_amount_pusd;
    let llm_dispute_bond = llm_dispute.bond_amount_pusd;

    let required_vault = assertion_bond
        .checked_add(llm_dispute_bond)
        .ok_or(OpalError::MathOverflow)?;
    require!(
        ctx.accounts.bond_vault.amount >= required_vault,
        OpalError::InsufficientVaultBalance
    );

    require!(
        ctx.accounts.asserter_pusd.owner == assertion.asserter,
        OpalError::InvalidAsserterTokenAccount
    );
    require!(
        ctx.accounts.llm_disputer_pusd.owner == llm_dispute.disputer,
        OpalError::InvalidDisputerTokenAccount
    );
    require!(
        ctx.accounts.treasury_pusd.key() == protocol_config.treasury,
        OpalError::InvalidTreasuryAccount
    );

    // !TBD: Reserved outcomes (TooEarly, Unresolvable) need explicit economic handling.
    // Currently they are treated identically to False (disputer wins).
    let llm_dispute_correct = final_outcome != OUTCOME_TRUE;

    let (asserter_payout, llm_disputer_payout, treasury_fee) = if llm_dispute_correct {
        let fee = checked_bps(assertion_bond, protocol_config.protocol_fee_bps)?;
        let llm_payout = llm_dispute_bond
            .checked_add(
                assertion_bond
                    .checked_sub(fee)
                    .ok_or(OpalError::MathOverflow)?,
            )
            .ok_or(OpalError::MathOverflow)?;
        (0, llm_payout, fee)
    } else {
        let fee = checked_bps(llm_dispute_bond, protocol_config.protocol_fee_bps)?;
        let asserter_total = assertion_bond
            .checked_add(
                llm_dispute_bond
                    .checked_sub(fee)
                    .ok_or(OpalError::MathOverflow)?,
            )
            .ok_or(OpalError::MathOverflow)?;
        (asserter_total, 0, fee)
    };

    let assertion_id = assertion.id;
    let bump = assertion.bump;
    drop(assertion);
    drop(llm_dispute);
    drop(llm_round);
    drop(protocol_config);

    let signer_seeds: &[&[u8]] = &[ASSERTION_SEED, assertion_id.as_ref(), &[bump]];

    if asserter_payout > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.bond_vault.to_account_info(),
                    to: ctx.accounts.asserter_pusd.to_account_info(),
                    authority: ctx.accounts.assertion.to_account_info(),
                },
                &[signer_seeds],
            ),
            asserter_payout,
        )?;
    }

    if llm_disputer_payout > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.bond_vault.to_account_info(),
                    to: ctx.accounts.llm_disputer_pusd.to_account_info(),
                    authority: ctx.accounts.assertion.to_account_info(),
                },
                &[signer_seeds],
            ),
            llm_disputer_payout,
        )?;
    }

    if treasury_fee > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.bond_vault.to_account_info(),
                    to: ctx.accounts.treasury_pusd.to_account_info(),
                    authority: ctx.accounts.assertion.to_account_info(),
                },
                &[signer_seeds],
            ),
            treasury_fee,
        )?;
    }

    let mut llm_dispute = ctx.accounts.llm_dispute.load_mut()?;
    llm_dispute.settlement_resolution = final_outcome;

    let mut assertion = ctx.accounts.assertion.load_mut()?;
    assertion.state = ASSERTION_STATE_RESOLVED;
    assertion.outcome = final_outcome;
    assertion.finalized_at = now;

    Ok(())
}
