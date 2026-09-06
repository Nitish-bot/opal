use crate::{
    constants::{
        ASSERTION_SEED, ASSERTION_STATE_PENDING_VOTE, ASSERTION_STATE_VOTING, BOOL_TRUE,
        PROTOCOL_CONFIG_SEED, VOTE_ROUND_SEED,
    },
    errors::OpalError,
    state::{AssertionAccount, ProtocolConfig, VoteResolutionRound},
    utils::checked_add_i64,
};
use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct OpenVoteArgs {
    pub assertion_id: Pubkey,
}

#[derive(Accounts)]
#[instruction(args: OpenVoteArgs)]
pub struct OpenVote<'info> {
    pub authority: Signer<'info>,

    #[account(
        seeds = [PROTOCOL_CONFIG_SEED],
        bump,
    )]
    pub protocol_config: AccountLoader<'info, ProtocolConfig>,

    #[account(
        mut,
        seeds = [ASSERTION_SEED, args.assertion_id.as_ref()],
        bump = assertion.load()?.bump,
    )]
    pub assertion: AccountLoader<'info, AssertionAccount>,

    #[account(
        mut,
        seeds = [VOTE_ROUND_SEED, assertion.key().as_ref()],
        bump = vote_resolution_round.load()?.bump,
    )]
    pub vote_resolution_round: AccountLoader<'info, VoteResolutionRound>,
}

pub fn handler(ctx: Context<OpenVote>, _args: OpenVoteArgs) -> Result<()> {
    // This placeholder is permissionless for liveness. The target instruction becomes the
    // receipt-validated `publish_vote_ready` projection described in docs/architecture.md.
    let assertion = ctx.accounts.assertion.load()?;
    let vote_round = ctx.accounts.vote_resolution_round.load()?;
    require!(
        assertion.state == ASSERTION_STATE_PENDING_VOTE,
        OpalError::InvalidState
    );
    require!(
        vote_round.assertion == ctx.accounts.assertion.key(),
        OpalError::AssertionLinkMismatch
    );
    drop(assertion);
    drop(vote_round);

    let now = Clock::get()?.unix_timestamp;
    let protocol_config = ctx.accounts.protocol_config.load()?;
    let voting_starts_at = checked_add_i64(now, protocol_config.vote_setup_window_seconds)?;
    let voting_deadline = checked_add_i64(voting_starts_at, protocol_config.voting_window_seconds)?;

    let vote_round = &mut ctx.accounts.vote_resolution_round.load_mut()?;
    vote_round.voting_starts_at = voting_starts_at;
    vote_round.voting_deadline = voting_deadline;
    vote_round.reveal_deadline = voting_deadline;
    // !TBD: PLACEHOLDER — MagicBlock delegation is currently a no-op (delegated=true but no ER tx).
    vote_round.delegated = BOOL_TRUE;

    let assertion = &mut ctx.accounts.assertion.load_mut()?;
    assertion.state = ASSERTION_STATE_VOTING;

    Ok(())
}
