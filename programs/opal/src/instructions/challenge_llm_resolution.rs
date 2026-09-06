use crate::{
    constants::{
        ASSERTION_SEED, ASSERTION_STATE_ASSERTED_LLM, ASSERTION_STATE_PENDING_VOTE,
        BOND_VAULT_SEED, BOOL_FALSE, LLM_ROUND_SEED, OUTCOME_NONE, PROTOCOL_CONFIG_SEED,
        TIMESTAMP_NONE, VOTE_DISPUTE_SEED, VOTE_ROUND_SEED,
    },
    errors::OpalError,
    state::{
        AssertionAccount, LlmResolutionRound, ProtocolConfig, VoteDisputeAccount,
        VoteResolutionRound,
    },
    utils::{checked_bps, is_outcome_set, is_pubkey_default, is_timestamp_set},
};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct ChallengeLlmResolutionArgs {
    pub assertion_id: Pubkey,
}

#[derive(Accounts)]
#[instruction(args: ChallengeLlmResolutionArgs)]
pub struct ChallengeLlmResolution<'info> {
    #[account(mut)]
    pub disputer: Signer<'info>,

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
        seeds = [LLM_ROUND_SEED, assertion.key().as_ref()],
        bump = llm_resolution_round.load()?.bump,
    )]
    pub llm_resolution_round: AccountLoader<'info, LlmResolutionRound>,

    #[account(
        init,
        payer = disputer,
        space = 8 + std::mem::size_of::<VoteDisputeAccount>(),
        seeds = [VOTE_DISPUTE_SEED, assertion.key().as_ref()],
        bump
    )]
    pub vote_dispute: AccountLoader<'info, VoteDisputeAccount>,

    #[account(
        init,
        payer = disputer,
        space = 8 + std::mem::size_of::<VoteResolutionRound>(),
        seeds = [VOTE_ROUND_SEED, assertion.key().as_ref()],
        bump
    )]
    pub vote_resolution_round: AccountLoader<'info, VoteResolutionRound>,

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
        token::authority = disputer,
    )]
    pub disputer_pusd: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<ChallengeLlmResolution>,
    _args: ChallengeLlmResolutionArgs,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;

    let assertion = ctx.accounts.assertion.load()?;
    let llm_round = ctx.accounts.llm_resolution_round.load()?;
    let protocol_config = ctx.accounts.protocol_config.load()?;

    require!(
        assertion.state == ASSERTION_STATE_ASSERTED_LLM,
        OpalError::InvalidState
    );
    require!(
        assertion.dispute_count == 1 && is_pubkey_default(&assertion.vote_dispute),
        OpalError::VoteDisputeAlreadyExists
    );
    require!(
        llm_round.assertion == ctx.accounts.assertion.key(),
        OpalError::AssertionLinkMismatch
    );

    require!(
        is_timestamp_set(llm_round.challenge_deadline),
        OpalError::MissingChallengeDeadline
    );
    require!(
        now < llm_round.challenge_deadline,
        OpalError::DeadlinePassed
    );

    require!(
        is_outcome_set(llm_round.outcome),
        OpalError::LlmOutcomeMissing
    );
    let challenged_llm_resolution = llm_round.outcome;

    let vote_bond_amount = checked_bps(
        assertion.assertion_bond_amount_pusd,
        protocol_config.vote_dispute_bond_ratio_bps,
    )?;
    drop(assertion);
    drop(llm_round);
    drop(protocol_config);
    require!(vote_bond_amount > 0, OpalError::InsufficientBondAmount);

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.disputer_pusd.to_account_info(),
                to: ctx.accounts.bond_vault.to_account_info(),
                authority: ctx.accounts.disputer.to_account_info(),
            },
        ),
        vote_bond_amount,
    )?;

    let vote_round_key = ctx.accounts.vote_resolution_round.key();

    let mut vote_dispute = ctx.accounts.vote_dispute.load_init()?;
    vote_dispute.assertion = ctx.accounts.assertion.key();
    vote_dispute.disputer = ctx.accounts.disputer.key();
    vote_dispute.challenged_llm_resolution_round = ctx.accounts.llm_resolution_round.key();
    vote_dispute.challenged_llm_resolution = challenged_llm_resolution;
    vote_dispute.bond_amount_pusd = vote_bond_amount;
    vote_dispute.created_at = now;
    vote_dispute.resolution_round = vote_round_key;
    vote_dispute.settlement_resolution = OUTCOME_NONE;
    vote_dispute.bump = ctx.bumps.vote_dispute;

    let mut vote_round = ctx.accounts.vote_resolution_round.load_init()?;
    vote_round.assertion = ctx.accounts.assertion.key();
    vote_round.dispute = ctx.accounts.vote_dispute.key();
    // !TBD: PLACEHOLDER — MagicBlock ER fields. Will be wired when delegation is implemented.
    vote_round.magicblock_validator = Pubkey::default();
    vote_round.permission_account = Pubkey::default();
    vote_round.delegated_vote_state = Pubkey::default();
    vote_round.delegated = BOOL_FALSE;
    vote_round.committed = BOOL_FALSE;
    vote_round.voting_starts_at = TIMESTAMP_NONE;
    vote_round.voting_deadline = TIMESTAMP_NONE;
    vote_round.reveal_deadline = TIMESTAMP_NONE;
    vote_round.total_valid_weight = 0;
    vote_round.aggregate_votes = Default::default();
    vote_round.final_outcome = OUTCOME_NONE;
    vote_round.bump = ctx.bumps.vote_resolution_round;

    let mut assertion = ctx.accounts.assertion.load_mut()?;
    assertion.state = ASSERTION_STATE_PENDING_VOTE;
    assertion.dispute_count = 2;
    assertion.vote_dispute = ctx.accounts.vote_dispute.key();
    assertion.vote_resolution_round = vote_round_key;

    Ok(())
}
