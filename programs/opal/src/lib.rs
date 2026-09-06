use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod instructions;
pub mod state;
pub mod utils;

pub use instructions::*;

pub use instructions::challenge_llm_resolution::{
    ChallengeLlmResolution, ChallengeLlmResolutionArgs,
};
pub use instructions::create_assertion::{CreateAssertion, CreateAssertionArgs};
pub use instructions::dispute_assertion::{DisputeAssertion, DisputeAssertionArgs};
pub use instructions::finalize_llm_resolution::{FinalizeLlmResolution, FinalizeLlmResolutionArgs};
pub use instructions::finalize_undisputed::{FinalizeUndisputed, FinalizeUndisputedArgs};
pub use instructions::finalize_vote_resolution_placeholder::{
    FinalizeVoteResolutionPlaceholder, FinalizeVoteResolutionPlaceholderArgs,
};
pub use instructions::initialize_protocol_config::{
    InitializeProtocolConfig, InitializeProtocolConfigArgs,
};
pub use instructions::open_vote::{OpenVote, OpenVoteArgs};
pub use instructions::submit_llm_resolution::{SubmitLlmResolution, SubmitLlmResolutionArgs};

declare_id!("8NCcxyAzKiAHxJ9DMnADtxShYutS9w81wHcXqgCavTBy");

#[program]
pub mod opal {

    use super::*;

    pub fn initialize_protocol_config(
        ctx: Context<InitializeProtocolConfig>,
        args: InitializeProtocolConfigArgs,
    ) -> Result<()> {
        instructions::initialize_protocol_config::handler(ctx, args)
    }

    pub fn create_assertion(
        ctx: Context<CreateAssertion>,
        args: CreateAssertionArgs,
    ) -> Result<()> {
        instructions::create_assertion::handler(ctx, args)
    }

    pub fn finalize_undisputed(
        ctx: Context<FinalizeUndisputed>,
        args: FinalizeUndisputedArgs,
    ) -> Result<()> {
        instructions::finalize_undisputed::handler(ctx, args)
    }

    pub fn dispute_assertion(
        ctx: Context<DisputeAssertion>,
        args: DisputeAssertionArgs,
    ) -> Result<()> {
        instructions::dispute_assertion::handler(ctx, args)
    }

    pub fn submit_llm_resolution(
        ctx: Context<SubmitLlmResolution>,
        args: SubmitLlmResolutionArgs,
    ) -> Result<()> {
        instructions::submit_llm_resolution::handler(ctx, args)
    }

    pub fn finalize_llm_resolution(
        ctx: Context<FinalizeLlmResolution>,
        args: FinalizeLlmResolutionArgs,
    ) -> Result<()> {
        instructions::finalize_llm_resolution::handler(ctx, args)
    }

    pub fn challenge_llm_resolution(
        ctx: Context<ChallengeLlmResolution>,
        args: ChallengeLlmResolutionArgs,
    ) -> Result<()> {
        instructions::challenge_llm_resolution::handler(ctx, args)
    }

    pub fn open_vote(ctx: Context<OpenVote>, args: OpenVoteArgs) -> Result<()> {
        instructions::open_vote::handler(ctx, args)
    }

    pub fn finalize_vote_resolution_placeholder(
        ctx: Context<FinalizeVoteResolutionPlaceholder>,
        args: FinalizeVoteResolutionPlaceholderArgs,
    ) -> Result<()> {
        instructions::finalize_vote_resolution_placeholder::handler(ctx, args)
    }
}
