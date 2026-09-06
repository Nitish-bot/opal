# Codebase organization and developer-workflow conventions

Two quality-of-life conventions, decided now and implemented in their own follow-up PRs (this docs PR only records them).

**Instruction modules group by the concept they act on.** `programs/opal/src/instructions/` moves from a flat list into concept subdirectories — `protocol/`, `assertion/`, `llm/`, `vote/` — so "where do I change X?" is obvious. The instruction that _acts on_ a thing lives with that thing even when it opens the next round: `dispute_assertion` → `assertion/` (you are disputing the assertion), `challenge_llm_resolution` → `llm/` (you are challenging the LLM result). Each subdir carries its own `mod.rs`; the `pub use` re-export paths in `lib.rs` update accordingly. No instruction is renamed — only relocated.

**Test targets split by environment.** Today `bun run test:local` aliases `anchor test`, and both run the localnet integration suite; LLM tests call resolver-gated `submit_llm_resolution` directly with a test resolver keypair, because the old `mock-llm` feature has been removed. A follow-up introduces an explicit Devnet integration target for the live trusted resolver ([ADR-0002](0002-trusted-llm-resolver.md)) and MagicBlock PER ([ADR-0003](0003-private-staked-voting.md)) while preserving the localnet logic suite.

**Why.** Past ~a dozen instructions a flat directory stops scaling, and concept-grouping makes the lifecycle legible. And the MVP's real dependencies can't run on a bare localnet, so logic tests and integration tests need different targets.

**Considered options / consequences.** Grouping by which round an instruction _opens_ was rejected as less discoverable. Replacing the local default with Devnet was rejected: contributors would need a funded keypair and deployed program for ordinary logic tests. The test-split PR instead adds a separately named Devnet command that skips the local validator and documents its prerequisites. Both changes are deferred to follow-up PRs.
