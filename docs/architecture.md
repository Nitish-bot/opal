# Opal Architecture

Opal is a Solana-native optimistic oracle for natural-language statements, resolved relative to each assertion's **Resolution Spec** (see [glossary.md](glossary.md) and [ADR-0001](adr/0001-rubric-relative-truth.md)). The protocol treats every new assertion as `True` by default during `Asserted`; disputers can challenge that default for direct economic upside, so the protocol design does not require an external monitoring or bot layer.

This document describes the target architecture for the prediction-market wedge. Resolution rules are detailed in [resolution.md](resolution.md), and economics are detailed in [tokenomics.md](tokenomics.md). Each section and feature is tagged `[Built]` (in the program today), `[MVP-target]` (committed, not yet built), or `[Vision]` (post-MVP); pure-`[Vision]` material is clustered at the end.

## Deployment Status and Public Identifiers

Opal does not yet have an official Devnet deployment while the MVP program is incomplete. Any onchain program used for development or testing is non-official and must not be presented as a participant or integration endpoint. Public documentation must not publish a Devnet program address or upgrade-authority address until the team designates the completed MVP deployment as official.

The official Devnet program remains upgradeable under one Opal-team-controlled single-signature wallet so critical fixes can ship quickly while the MVP hardens. That same wallet is `ProtocolConfig.authority` and owns the configured USDC treasury token account; the trusted resolver remains a separate role-limited operational signer. Devnet has no admin multisig, timelock, treasury-withdrawal cap, or program-mediated treasury governance. The public deployment reference must identify the cluster, verified program address, current admin/upgrade-authority address, treasury token-account and owner addresses, and the absence of a timelock. It must state plainly that the team can change the program or move treasury funds without an advance onchain delay; Devnet users must not infer immutability or decentralized treasury control from a verified address. See [ADR-0018](adr/0018-devnet-single-admin-and-treasury-authority.md).

The Mainnet upgrade-authority model is intentionally undecided and is not a prerequisite for the Devnet MVP or its documentation. Devnet documentation must not promise a future Mainnet multisig, timelock, or immutable deployment before that policy is separately decided.

## Design Invariants

- `AssertionAccount` does not store a tentative resolution field. `[Built]`
- The current non-final answer is inferred from state: `Asserted` means default `True`; `AssertedLLM` means read `LlmResolutionRound.outcome`; `PendingVote` and `Voting` mean that LLM answer is under challenge and no final answer exists yet. `[Built]`
- `outcome` is unset (`OUTCOME_NONE`) until `state == Resolved`. `[Built]`
- `Asserted` and `AssertedLLM` are liveness states: the current answer can still be challenged. `[Built]`
- `PendingLLM` and `PendingVote` are intermediary states: a dispute has been accepted, but the next resolution layer is not finished or active yet. `[Built]`
- `Resolved` is the normal terminal truth state: `outcome` is set and irreversible consumers can settle. `[Built]`
- `ResolverUnavailable` is a separate exceptional terminal operational status: `outcome` remains unset because no truth judgment occurred. `[MVP-target]`
- The statement lives onchain as a fixed-size byte array. The target creation rule requires single-line UTF-8 occupying 1–280 bytes, with no surrounding Unicode whitespace, control characters, line/paragraph separators, or invisible bidirectional controls; ordinary right-to-left letters remain valid. Accepted UTF-8 bytes are preserved without normalization or rewriting, and unused storage is zero-filled. Storage and the current maximum check are `[Built]`; complete validation is `[MVP-target]`. See [ADR-0021](adr/0021-readable-statement-validation.md).
- The Resolution Spec lives **off-chain on Arweave**; its exact canonical 48-byte `ar://<transaction-or-data-item-id>` URI is stored onchain in `resolution_spec_uri: [u8; 48]` so anyone can retrieve the spec and cryptographically verify it against the known ID. The current `auxiliary_hash: [u8; 128]` field is renamed and narrowed before Devnet. The immutable, time-independent spec **is the source of truth** — the LLM resolver and voters apply it, they do not adjudicate universal reality. It orders authoritative sources and defines conflict and unavailability behavior; any material ambiguity left by those rules produces `Unresolvable`. It cannot impose an asserter-selected evidence cutoff, freeze a source version, exclude later corrections, or delay resolution until a spec-selected date. Evidence timing is protocol-wide: each resolution action uses the latest authoritative information available when it acts. `[MVP-target]` (see [ADR-0001](adr/0001-rubric-relative-truth.md), [ADR-0013](adr/0013-protocol-time-evidence.md), [ADR-0014](adr/0014-ordered-resolution-sources.md), and [ADR-0015](adr/0015-canonical-resolution-spec-uri.md))
- **USDC** is the single collateral asset for bonds, slashing, rewards, and fees. The mint is a config field (`pusd_mint`, slated to rename to `usdc_mint`) so localnet/devnet can use a test mint, but the protocol commits to USDC. `[MVP-target]` (see [ADR-0004](adr/0004-single-asset-usdc.md))
- Governance is the `authority` keypair. There is no separate governance token in the MVP. `[MVP-target]`
- LLM resolution is a **single trusted off-chain resolver** that posts the verdict via `submit_llm_resolution`, gated on the dedicated `ProtocolConfig.resolver` key `[Built]`; the off-chain resolver service is `[MVP-target]`. The former 3-feed Switchboard council was removed per [ADR-0002](adr/0002-trusted-llm-resolver.md). Binding LLM provenance hashes onchain is not part of the MVP — it is deferred to `[Vision]`.
- A resolver that does not post within 24 hours after the first dispute is accepted makes the Assertion eligible for the exceptional terminal status `ResolverUnavailable` `[MVP-target]`, not a fifth truth outcome. Anyone may invoke its deterministic timeout finalizer; it atomically refunds both 500 USDC bonds in full, without Bond Fees, slashing, or rewards, and does not let the caller select an outcome or redirect funds. Normal resolution produces `True`, `False`, `Unresolvable`, or `NoConsensus`; resolver unavailability keeps `outcome = None` because no rubric-relative judgment occurred. See [ADR-0012](adr/0012-resolver-unavailable.md).
- Final escalation is a **private, USDC-staked vote on a MagicBlock ephemeral rollup** with linear weight (1 USDC = 1 vote) and Schelling-point slashing. `True`, `False`, or `Unresolvable` must reach 67%, otherwise the vote resolves `NoConsensus`. `[MVP-target]` (see [ADR-0003](adr/0003-private-staked-voting.md))
- `Unresolvable` is fault-assigned; only `NoConsensus` settles no-fault, making collateral claimable minus ordinary fees with no rewards. `[MVP-target]` (see [ADR-0007](adr/0007-unresolvable-vs-no-consensus.md))

> Note on field names: account fields still carry the legacy `pusd` prefix (`assertion_bond_amount_pusd`, `bond_amount_pusd`, `pusd_mint`, …). These are USDC; the `*_pusd → *_usdc` / `pusd_mint → usdc_mint` rename is a separate follow-up PR.

## Onchain Account Model

All state accounts are **zero-copy** with `#[repr(C, packed)]`. They contain only primitive types (`u8`, `i64`, `Pubkey`, `[u8; N]`, `u64`, `u16`, `u128`). No `Option<T>`, `bool`, or enums are used inside zero-copy accounts; sentinels (`Pubkey::default()`, `0`, `255`) represent unset fields. `[Built]`

### `AssertionAccount` `[Built]`

The primary PDA for an assertion. It intentionally stores enough summary fields that an integrator can read one account and know the current state, whether the assertion was disputed once or twice, and which round accounts contain the LLM and vote resolutions.

```rust
#[repr(C, packed)]
#[account(zero_copy(unsafe))]
pub struct AssertionAccount {
    pub id: Pubkey,
    pub asserter: Pubkey,
    pub statement: [u8; 280],
    pub auxiliary_hash: [u8; 128], // current; target: resolution_spec_uri: [u8; 48]
    pub bond_vault: Pubkey,
    pub state: u8,                  // ASSERTION_STATE_*
    pub liveness_deadline: i64,
    pub llm_challenge_deadline: i64,
    pub outcome: u8,                // OUTCOME_* (255 = unset)
    pub finalized_at: i64,          // 0 = unset
    pub dispute_count: u8,
    pub assertion_bond_amount_pusd: u64,
    pub llm_dispute: Pubkey,        // default = unset
    pub vote_dispute: Pubkey,       // default = unset
    pub llm_resolution_round: Pubkey,   // default = unset
    pub vote_resolution_round: Pubkey,  // default = unset
    pub bump: u8,
}
```

`auxiliary_hash: [u8; 128]` is the current, misleading field for the off-chain Resolution Spec identifier. Before Devnet it becomes `resolution_spec_uri: [u8; 48]`, matching the exact canonical `ar://<id>` URI without padding; resolving the Assertion means applying that spec, not judging absolute truth.

Implementation notes:

- On creation, `state = ASSERTION_STATE_ASSERTED`, `outcome = OUTCOME_NONE`, and `dispute_count = 0`.
- While `state = ASSERTED`, the current non-final answer is the optimistic default `True`.
- When the first dispute is filed, `dispute_count = 1`, `llm_dispute` is set, `llm_resolution_round` is set, and `state = PENDING_LLM`.
- When the LLM result is posted, `state = ASSERTED_LLM`; the current non-final answer is `LlmResolutionRound.outcome`.
- When the second dispute is filed, `dispute_count = 2`, `vote_dispute` is set, `vote_resolution_round` is set, and `state = PENDING_VOTE`.
- When voting is active, `state = VOTING`; the LLM result remains the challenged result until the vote resolves.
- When finalized, `state = RESOLVED` and `outcome` is set.

### Resolution Spec (Arweave + `resolution_spec_uri`) `[MVP-target]`

The Resolution Spec is the immutable, time-independent asserter-supplied rubric — authoritative sources in priority order, deterministic conflict and source-unavailability rules, key definitions, and ambiguity handling. It is the single most important artifact after the statement itself, because **it is the source of truth**: True means "true under this assertion's spec, correctly applied," not a universal fact (see [ADR-0001](adr/0001-rubric-relative-truth.md)).

- The spec lives off-chain on **Arweave** (permanent, content-addressed) and must stay retrievable and integrity-checkable for the whole assertion lifecycle.
- The asserter or an integrating sponsor uploads the spec and pays Arweave directly; Opal charges no storage fee. A verified existing upload may be reused without paying for duplicate storage.
- The canonical identifier is exactly `ar://<id>`, with a 43-character unpadded base64url Arweave transaction or data-item ID and no gateway, ArNS name, query, or fragment. Both the creating client and `create_assertion` enforce its exact 48-byte ASCII shape. The target `resolution_spec_uri: [u8; 48]` stores those bytes verbatim with no padding.
- Creating clients use the known ID for fail-closed Arweave signature/data-root verification. A separate raw content digest is not part of the protocol identifier.
- The current `AssertionAccount.auxiliary_hash: [u8; 128]` field is renamed and narrowed to `resolution_spec_uri: [u8; 48]` before Devnet; public integrations never expose `auxiliaryHash`.
- The uploaded content is UTF-8 JSON validated against a published, versioned schema. Devnet v1 requires `schemaVersion: "1"`, explicit definitions, one or more ordered sources with unavailability behavior, `conflictBehavior: "highest-priority-wins"`, and non-empty ambiguity handling. Optional `rationale` text is non-normative; unknown fields are rejected. The exact JSON is capped at 16 KiB (16,384 bytes) as an Opal application bound, not an Arweave tier or protocol limit. See [ADR-0016](adr/0016-versioned-resolution-spec-json.md) and the [v1 schema](schemas/resolution-spec-v1.schema.json).
- The LLM resolver and the voters **apply** the spec; disputes and votes are framed as "applying this spec, the answer is X," not "in reality it's X."
- A one-source spec satisfies the hierarchy automatically. A multi-source spec must state precedence and whether lower-ranked sources substitute when a preferred source is unavailable. Any material conflict or availability gap left unresolved by those rules produces `Unresolvable`; resolution actors cannot choose a convenient source.
- The spec cannot select an evidence cutoff, freeze a source version, exclude later corrections, or delay resolution until a date. Each resolution action applies the unchanged spec to the latest relevant information available from its authoritative sources when that action occurs. Accepted Votes remain immutable if a source changes later in the Voting Window.
- Vetting the spec is the integrator's responsibility: garbage spec in → garbage truth out, faithfully applied. An integrator must read the spec before trusting an outcome.

### `LlmDisputeAccount` `[Built]`

The first dispute account. It does not need to store the challenged resolution because the first dispute always challenges the default optimistic `True`.

```rust
#[repr(C, packed)]
#[account(zero_copy(unsafe))]
pub struct LlmDisputeAccount {
    pub assertion: Pubkey,
    pub disputer: Pubkey,
    pub bond_amount_pusd: u64,
    pub created_at: i64,
    pub resolution_round: Pubkey,
    pub settlement_resolution: u8,  // 255 = unset
    pub bump: u8,
}
```

`settlement_resolution` records the outcome this dispute settled against: if the LLM result is not challenged, it is the LLM result; if it is challenged, it is the final vote result. `True`, `False`, and `Unresolvable` assign fault and slash incorrect positions; `NoConsensus` assigns no fault and returns collateral minus ordinary fees. See [ADR-0007](adr/0007-unresolvable-vs-no-consensus.md) and [resolution.md](resolution.md). `[MVP-target]`

### `VoteDisputeAccount` `[Built]`

The second dispute account. It challenges the LLM result stored on `LlmResolutionRound`.

```rust
#[repr(C, packed)]
#[account(zero_copy(unsafe))]
pub struct VoteDisputeAccount {
    pub assertion: Pubkey,
    pub disputer: Pubkey,
    pub challenged_llm_resolution_round: Pubkey,
    pub challenged_llm_resolution: u8,  // OUTCOME_*
    pub bond_amount_pusd: u64,
    pub created_at: i64,
    pub resolution_round: Pubkey,
    pub settlement_resolution: u8,  // 255 = unset
    pub bump: u8,
}
```

`settlement_resolution` is the final vote outcome. `True`, `False`, and `Unresolvable` slash incorrect positions; `NoConsensus` settles no-fault `[MVP-target]`.

### `BondVault` `[Built]`

A PDA-controlled SPL token account that holds assertion and dispute collateral (USDC) until settlement. It is initialized alongside the assertion and uses the assertion's PDA as its authority.

On paths that resolve before voting, the Bond Vault remains on Solana and pays public token accounts. On vote escalation, the `[MVP-target]` Vote Initialization flow moves/delegates its full 1,500 USDC balance into a PER-resident Vote Settlement Vault and sponsors missing Private Voting Balances for the three bonded wallets. Paying initialization costs does not change the wallets' token authority. The Voting Window cannot begin until the vault, private vote state, and private participant balances are verified on the same router-resolved endpoint.

### `LlmResolutionRound` `[Built]`

The account tracking LLM resolution for the first dispute. The former council/Switchboard fields (`council_feeds`, the `switchboard_*` fields, and the four `*_hash` provenance fields) were removed with the council path per [ADR-0002](adr/0002-trusted-llm-resolver.md). On-chain LLM provenance hashing would only land with a future `[Vision]` trust-minimized resolver whose shape is undecided, so this doc does not commit to any specific hash set.

```rust
#[repr(C, packed)]
#[account(zero_copy(unsafe))]
pub struct LlmResolutionRound {
    pub assertion: Pubkey,
    pub dispute: Pubkey,
    pub outcome: u8,            // OUTCOME_* (255 = unset)
    pub requested_at: i64,
    pub resolved_at: i64,       // 0 = unset
    pub challenge_deadline: i64, // 0 = unset
    pub bump: u8,
}
```

`outcome` is an outcome code: `0 = True`, `1 = False`, `2 = NoConsensus`, `3 = Unresolvable`. Code `2` is currently the legacy `TooEarly` value and is reassigned before Devnet deployment. The resolver can post `True`, `False`, or `Unresolvable`; only vote settlement can produce `NoConsensus`. See [ADR-0007](adr/0007-unresolvable-vs-no-consensus.md).

### `VoteResolutionRound` `[Built]` (struct) / `[MVP-target]` (private vote)

The account tracking the private staked vote for the second dispute. The MagicBlock fields (`magicblock_validator`, `permission_account`, `delegated_vote_state`) and the `delegated` / `committed` lifecycle flags model the ephemeral-rollup delegation that the private vote runs on. In the Devnet target, `magicblock_validator` copies the single deployment-wide Voting PER validator identity rather than selecting a validator per round. In the current code these fields are wired structurally but the real ER vote is not yet implemented — `open_vote` advances the state machine and sets `delegated = BOOL_TRUE`, and `finalize_vote_resolution_placeholder` writes a supplied outcome. The MagicBlock private vote is the **MVP target**, not a permanent placeholder.

```rust
#[repr(C, packed)]
#[account(zero_copy(unsafe))]
pub struct VoteResolutionRound {
    pub assertion: Pubkey,
    pub dispute: Pubkey,
    pub magicblock_validator: Pubkey,
    pub permission_account: Pubkey,
    pub delegated_vote_state: Pubkey,
    pub delegated: u8,          // BOOL_TRUE/BOOL_FALSE
    pub committed: u8,          // BOOL_TRUE/BOOL_FALSE
    pub voting_starts_at: i64,  // 0 = unset
    pub voting_deadline: i64,   // 0 = unset
    pub reveal_deadline: i64,   // 0 = unset
    pub total_valid_weight: u128,
    pub aggregate_votes: VotesPerOutcome,
    pub final_outcome: u8,      // OUTCOME_* (255 = unset)
    pub bump: u8,
}
```

`aggregate_votes` (`VotesPerOutcome`) is designed to accumulate per-outcome Gross Voting Stake and `total_valid_weight` the gross weighted total `[MVP-target]`; today both are only initialized to zero and never written. The target round also counts distinct wallet addresses with valid Votes. Every valid Vote record increments that count once, including a Vote cast by any bonded wallet. The vote uses **linear weight** (1 originally staked USDC = 1 vote) and is settled by **Schelling-point slashing** — losing-side voters are slashed, winning-side voters are paid from the losing side. The MagicBlock ephemeral rollup hides per-outcome totals and wallet vote records during the Voting Window. Settlement makes the aggregates and each wallet's selected outcome and stake public. The Supermajority is evaluated only after the dual Devnet Vote Quorum is met: `total_valid_weight >= 3,000 USDC` and at least 10 distinct voting wallets. Both the threshold calculation and `total_valid_weight` use original locked stake before the Voting Fee; fees are deducted only during settlement. The inclusive threshold uses checked cross-products, `outcome_weight × 10,000 >= total_valid_weight × 6,700`, so exactly 67.000% wins without division rounding. `final_outcome` is `NoConsensus` if either quorum condition fails or, after quorum, no option reaches 67%. See [ADR-0003](adr/0003-private-staked-voting.md) and [ADR-0007](adr/0007-unresolvable-vs-no-consensus.md).

The three bonded wallets remain eligible to vote and may choose any outcome, including one opposed to their bonded position. The one-Vote-per-wallet-per-round constraint applies normally to each of them. A wallet may participate in multiple concurrent Vote Rounds when its unlocked Private Voting Balance covers each independently locked position. Bond and Vote positions are recorded, classified, and settled independently, so one may be correct while the other is slashed.

Voting Fees are calculated in USDC atomic units with checked multiplication and integer division: `floor(gross_stake × 10 / 10,000)`. Integer truncation deliberately rounds the fee down; quorum, tally, and Reward Weight continue using gross stake.

The MVP target adds a reusable **Private Voting Balance** for each wallet through MagicBlock's private-payment/eSPL flow. A voter deposits at least 1 USDC per deposit on the base layer before casting; the deposit is public, while the delegated USDC balance is permissioned in the PER. Vote Initialization sponsors missing balances for the bonded wallets and moves/delegates the three public Participant Bonds into a **Vote Settlement Vault** on that PER. The voter normally funds the SOL account-creation balance for its own permanent `VotePositionAccount`; an integration may sponsor it, but Opal does not guarantee sponsorship. An unused preparatory record is safely closable to its original payer if no Vote is accepted; acceptance makes the record permanent. The vote instruction locks stake from the private balance into the settlement vault and writes the private vote record. The private balance, Vote Settlement Vault, and vote state must be co-located because one ER transaction cannot mutate accounts on different runtimes or validators. The current program has no voter-position account, private balance integration, voting-stake escrow, sponsored balance initialization, or Bond Vault delegation; these remain `[MVP-target]`.

Private vote records are not committed as plaintext during the Voting Window. Vote finalization freezes fixed-size aggregate totals, payout inputs, the immutable voter-position count, and public-position publication progress. It does not append an unbounded voter vector to the round or publish every voter atomically. Deterministic bounded batches publish a separate permanent `VotePositionAccount` for each wallet-and-round pair containing the selected outcome, Gross Voting Stake, exact payout, and `Claimable` or `Slashed` status. Publication is permissionless and never expires. The instruction derives each record from authenticated finalized state, rejects altered or duplicate positions, and advances progress only for newly published records. The caller normally pays its external transaction cost unless an integration sponsors it; there is no Opal publication fee or caller reward. The round marks publication complete only when its published count equals its frozen position count. A successful claim publicly advances its position to `Claimed`; the wallet's total Private Voting Balance remains private. See [ADR-0017](adr/0017-per-voter-public-position-accounts.md).

Outcome finality is independent from publication completion. An integrator that enumerates or audits all positions must wait for the completion marker, while one consuming only the final Assertion outcome does not. Reserved claim liabilities stay in the Vote Settlement Vault. A recipient submits a wallet-authorized claim that transfers a positive payout to their Private Voting Balance inside the PER. Claims are independent bounded transactions rather than one transaction over an unbounded voter set; a `Slashed` position has no claim transaction. A later wallet-authorized withdrawal moves available USDC back to Solana through an observed cross-runtime workflow. Deposits and withdrawals expose `Pending`, `Completed`, and `Recovery Needed`; only destination-balance confirmation produces `Completed`, so source-transaction confirmation alone never changes usable balance or proves a withdrawal completed.

`PendingVote` exists so the protocol can create the vote round, move/delegate the Bond Vault, and verify PER co-location before starting the Voting Window and moving to `Voting`.

### `ProtocolConfig` `[Built]`

Singleton PDA containing deployment parameters. The struct below is the current implementation; several economic fields are legacy under the target equal-bond, per-fee, pro-rata settlement model.

```rust
#[repr(C, packed)]
#[account(zero_copy(unsafe))]
pub struct ProtocolConfig {
    pub authority: Pubkey,
    pub pusd_mint: Pubkey,      // USDC mint; slated to rename to usdc_mint
    pub treasury: Pubkey,
    pub resolver: Pubkey,
    pub assertion_bond_min_pusd: u64,
    pub llm_dispute_bond_ratio_bps: u16,
    pub vote_dispute_bond_ratio_bps: u16,
    pub protocol_fee_bps: u16,
    pub llm_disputer_reward_share_bps: u16,
    pub vote_disputer_reward_share_bps: u16,
    pub voter_reward_share_bps: u16,
    pub treasury_share_bps: u16,
    pub supermajority_bps: u16,
    pub liveness_window_seconds: i64,
    pub llm_challenge_window_seconds: i64,
    pub vote_setup_window_seconds: i64,
    pub voting_window_seconds: i64,
    pub bump: u8,
}
```

- `authority` is governance for the MVP (no separate governance token).
- `resolver` is the only key allowed to post LLM verdicts (`submit_llm_resolution`). It is deliberately separate from `authority`: the resolver is a hot key held by the off-chain service, and a leak exposes only a challengeable verdict, not governance.
- `supermajority_bps` is initialized to `6700` (67%) on Devnet: it is the weighted threshold `True`, `False`, or `Unresolvable` must reach, otherwise the outcome is `NoConsensus`.
- The Devnet target adds fixed deployment parameters for a 3,000 USDC minimum total voting stake and 10 minimum distinct voting wallets. Both Vote Quorum conditions must pass before `supermajority_bps` is evaluated.
- The bond-ratio and role-specific reward-share fields are removed for the Devnet MVP. All Participant Bonds are 500 USDC, and vote-stage rewards use unified Reward Weight.
- The Devnet target adds one `voting_per_validator` deployment parameter. Every `VoteResolutionRound.magicblock_validator` copies it; participants and individual Vote Rounds cannot override it. The validator identity is fixed, but clients resolve its current endpoint through the router.

### `Treasury` `[Built]`

An SPL token account owned by the same Opal-team single-signature wallet used as the official Devnet program upgrade authority and `ProtocolConfig.authority`. It is configured immutably in `ProtocolConfig.treasury`, and normal settlement transfers Bond Fees and Voting Fees directly to it. The wallet can transfer accumulated USDC at any time under the SPL Token program; Opal has no separate withdrawal instruction, onchain spending limit, multisig, or timelock on Devnet. Exact addresses are published only after the completed deployment is officially designated. See [ADR-0018](adr/0018-devnet-single-admin-and-treasury-authority.md).

## Instruction Flow

The instruction names below are stable for this PR.

1. `create_assertion` `[Built]`
   - `[MVP-target]` Requires a fresh client-generated Assertion ID keypair to sign; the public key becomes `AssertionAccount.id`, while the temporary secret grants no post-creation authority.
   - Uses the asserter as the normal Solana transaction and account-creation payer; an integrator may sponsor those network costs. Opal charges no separate upfront assertion fee.
   - `[MVP-target]` Rejects a statement unless it is single-line UTF-8 occupying 1–280 bytes, without surrounding Unicode whitespace, control characters, line/paragraph separators, or invisible bidirectional controls; stores accepted bytes exactly without normalization or rewriting. See [ADR-0021](adr/0021-readable-statement-validation.md).
   - Stores the statement and the canonical Resolution Spec URI (`resolution_spec_uri`; currently `auxiliary_hash`). `[MVP-target]` Rejects the URI unless it has the exact 48-byte canonical `ar://<id>` shape defined by [ADR-0015](adr/0015-canonical-resolution-spec-uri.md).
   - Locks the asserter's USDC bond.
   - Sets `state = ASSERTED`, `outcome = OUTCOME_NONE`, and `liveness_deadline`.
   - Permanently fixes the statement, Resolution Spec URI, bond, and deadlines; no edit, cancellation, or early bond-withdrawal instruction exists.

   > **Implementation mismatch.** The current `assertion_id` is an unsigned caller-supplied argument and can be front-run. It also checks only maximum lengths for statement and `auxiliary_hash`, accepts empty or structurally invalid statements and non-canonical Resolution Spec identifiers, and allocates 128 bytes for the target 48-byte URI. Require the ID signer, target validation, and field narrowing before Devnet deployment; see [ADR-0010](adr/0010-signed-assertion-identifiers.md), [ADR-0015](adr/0015-canonical-resolution-spec-uri.md), and [ADR-0021](adr/0021-readable-statement-validation.md).

2. `dispute_assertion` `[Built]`
   - Allowed while the assertion is `ASSERTED` and before `liveness_deadline`.
   - `[MVP-target]` Rejects the asserter's wallet; the first disputer must be a distinct wallet address.
   - Locks the first disputer's USDC bond.
   - Creates `LlmDisputeAccount`.
   - Creates `LlmResolutionRound`.
   - Sets `state = PENDING_LLM`, `dispute_count = 1`, and round/dispute pointers on `AssertionAccount`.
   - Uses first-valid-transaction-wins semantics: after one dispute is accepted, every competing or retried transaction fails before bond movement or Opal fee liability.

3. `submit_llm_resolution` `[Built]` _(instruction)_ / `[MVP-target]` _(off-chain resolver service)_
   - Gated to `protocol_config.resolver` — a dedicated key, separate from `authority`, so a leaked hot resolver key can only post a challengeable verdict.
   - Accepts only `True`, `False`, or `Unresolvable`; `NoConsensus` is vote-only per [ADR-0007](adr/0007-unresolvable-vs-no-consensus.md).
   - Sets `AssertionAccount.state = ASSERTED_LLM` and opens the LLM challenge deadline.
   - The `[MVP-target]` off-chain trusted resolver makes a single LLM call and posts the verdict through this instruction; on-chain provenance hashing is deferred to `[Vision]`. (The name previously belonged to the removed Switchboard council instruction — [ADR-0002](adr/0002-trusted-llm-resolver.md).)
   - `[MVP-target]` Accepts a verdict only before the 24-hour Resolver Deadline.

4. `finalize_resolver_unavailable` `[MVP-target]`
   - Is permissionless at or after the Resolver Deadline while the Assertion remains `PENDING_LLM`.
   - Sets the exceptional terminal status `ResolverUnavailable` while preserving `outcome = OUTCOME_NONE`.
   - Atomically refunds the full 500 USDC Participant Bond to the asserter and LLM disputer through their public Solana token accounts.
   - Charges no Bond Fee, slashes nobody, and pays no reward.
   - Derives both recipients; the caller cannot redirect funds or choose a Resolution Outcome.

5. `finalize_llm_resolution` `[Built]`
   - Allowed after the LLM challenge window if no vote dispute exists.
   - Sets `state = RESOLVED` and `outcome = LlmResolutionRound.outcome`.
   - Sets `LlmDisputeAccount.settlement_resolution` and settles bonds using Winner Takes Remaining Bond; `Unresolvable` assigns fault to the asserter.

6. `challenge_llm_resolution` `[Built]`
   - Allowed while the assertion is `ASSERTED_LLM` and before the LLM challenge deadline.
   - `[MVP-target]` Rejects both existing bonded wallets; the second disputer must differ from the asserter and first disputer.
   - Locks the second disputer's USDC bond.
   - Creates `VoteDisputeAccount` with `challenged_llm_resolution = LlmResolutionRound.outcome`.
   - Creates `VoteResolutionRound`.
   - Sets `state = PENDING_VOTE`, `dispute_count = 2`, and round/dispute pointers on `AssertionAccount`.
   - `[MVP-target]` Starts Vote Initialization: moves/delegates the complete Bond Vault balance into the Voting PER's Vote Settlement Vault and verifies co-location.
   - Uses first-valid-transaction-wins semantics: after one challenge is accepted, every competing or retried transaction fails before bond movement or Opal fee liability.

7. `open_vote` `[Built]` (state plumbing) / `[MVP-target]` (real ER delegation)
   - Requires Vote Initialization and PER co-location to be complete.
   - Sets the seven-day Voting Window on `VoteResolutionRound`; no voting time elapses while initialization is pending.
   - Moves the assertion from `PENDING_VOTE` to `VOTING`.
   - Sets `delegated = BOOL_TRUE`. Auth policy for completing/opening the vote is still to be finalized (currently permissionless for liveness).

8. `finalize_vote_resolution_placeholder` `[Built]` (state plumbing) / `[MVP-target]` (real tally)
   - Allowed after the voting window expires.
   - Sets `VoteResolutionRound.final_outcome` (currently from an argument; in the MVP from the private tally, applying the 67% threshold and producing `NoConsensus` when no option reaches it, followed by the MagicBlock commit/undelegate).
   - Sets `AssertionAccount.state = RESOLVED` and `AssertionAccount.outcome`.
   - Freezes fees, the Slashed Pool, reward inputs, and every claimable vote-stage payout without enumerating or paying every recipient.

9. `claim_vote_payout` `[MVP-target]`
   - Is callable immediately after Vote Round finalization confirms, with no cooldown or separate claim-opening instruction, and requires the payout owner's wallet signature.
   - Derives the permanently fixed claim amount from immutable finalization totals and the claimant's recorded bond or Vote position; elapsed time cannot change it.
   - Requires a positive `Claimable` amount; terminal `Slashed` positions cannot claim.
   - Derives and validates the wallet's canonical Private Voting Balance; no caller-selected destination is accepted.
   - Transfers the claim from the Vote Settlement Vault into the claimant's immediately available Private Voting Balance and marks that position claimed atomically; no post-claim lock or cooldown is created.
   - Rejects a repeated claim and never withdraws funds to the public Solana wallet.

10. `finalize_undisputed` `[Built]`
    - Allowed after `liveness_deadline` if no dispute exists.
    - Sets `state = RESOLVED` and `outcome = TRUE`.
    - Returns the asserter bond minus configured fees.

11. `initialize_protocol_config` `[Built]`
    - Authority-gated bootstrap of the `ProtocolConfig` singleton.

## State Machine `[Built]`

The six normal resolution states and their current constant values are `Asserted` (0), `PendingLLM` (1), `AssertedLLM` (2), `PendingVote` (3), `Voting` (4), and `Resolved` (5). The Devnet target adds the exceptional terminal `ResolverUnavailable` status; its raw representation is an implementation detail to fix when the path is built and is not part of the Preview Integration API.

```text
Asserted(default=True)
  | liveness expires with no dispute
  v
Resolved(True)

Asserted(default=True)
  | first dispute
  v
PendingLLM
  | LLM verdict posted (submit_llm_resolution, resolver-gated)
  v
AssertedLLM(LlmResolutionRound.outcome)
  | LLM challenge window expires
  v
Resolved(LlmResolutionRound.outcome)

AssertedLLM(LlmResolutionRound.outcome)
  | second dispute
  v
PendingVote
  | vote opened
  v
Voting
  | private vote finalized
  v
Resolved(VoteResolutionRound.final_outcome)

PendingLLM
  | 24-hour resolver deadline expires
  v
ResolverUnavailable(outcome=None, terminal operational failure) [MVP-target]
```

When a terminal outcome is `Unresolvable`, the asserter is incorrect and ordinary slashing/rewards apply. When it is `NoConsensus`, settlement is no-fault: no collateral is slashed, ordinary participation fees are withheld, and no rewards are paid. See [ADR-0007](adr/0007-unresolvable-vs-no-consensus.md).

## Integrator Contract

The **Preview Integration API** used by the future developer documentation is a non-runnable, client-shaped sketch over the Solana program and Anchor IDL. It illustrates account reads, address derivation, and wallet-signed transaction construction; it is not an HTTP service, deployed API, runnable SDK, or separate source of protocol truth. The underlying program and IDL are authoritative, and the preview's method names may change before a real client is stabilized.

The Preview layer separates read state from failed actions. Transfer `Pending` and `RecoveryNeeded` and terminal Assertion status `ResolverUnavailable` are returned as domain values. An action rejection exposes a stable semantic `code` and `nextAction`, with the raw Anchor, RPC, wallet, or transport failure retained only as an optional diagnostic cause. Integrations never branch on raw numeric codes or message text, and an unproven mapping remains an explicit unknown error. See [ADR-0022](adr/0022-stable-integration-errors-and-domain-statuses.md).

After a known duplicate rejection or ambiguous submission result, the Preview layer reads the authoritative immutable record before deciding whether the request succeeded. An exact dispute, Vote, or claim match returns the ordinary success result with `alreadyApplied: true`; a mismatch returns a conflict with `nextAction: "REFRESH_STATE"`, and a missing record preserves the failure. The onchain instruction still rejects duplicates before moving additional funds. This reconciliation makes transport retries idempotent without allowing a competing dispute or a Vote with different outcome or stake to masquerade as the requested action.

The stable action mappings are `STATE_CONFLICT → REFRESH_STATE`, `DEADLINE_PASSED → REFRESH_STATE`, and `INSUFFICIENT_UNLOCKED_BALANCE → REVIEW_BALANCE`. The insufficient-balance error and its default diagnostic cause omit requested, unlocked, locked, and total private-balance amounts. A wallet-authenticated interface may separately retrieve and display those values locally; ordinary error telemetry must not receive them.

Prediction markets and other consumers use `AssertionAccount.id` as the canonical public reference within their configured Opal deployment. The client derives the underlying Assertion account internally. Consumers should read the assertion ID, statement, Resolution Spec URI, state, final outcome, dispute count, dispute pointers, resolution round pointers, and finalized timestamp.

Integrator rules:

- Identify and consume the exact Assertion ID. Statement or Resolution Spec content is not unique: duplicate Assertions are allowed, remain independent, and have no protocol-designated latest or canonical replacement.
- Creation clients enforce the same structural statement rules as the program before asking the wallet to sign: single-line UTF-8 occupying 1–280 bytes, no surrounding Unicode whitespace, and no control characters, line/paragraph separators, or Unicode `Bidi_Control` characters. Ordinary right-to-left letters remain valid. JavaScript and TypeScript clients first reject ill-formed strings, measure `TextEncoder` byte length rather than `string.length`, and use the same unchanged validated value for byte counting, signing preview, and Anchor transaction construction. They submit and display the accepted string exactly, without Unicode normalization or other rewriting; account readers use fatal UTF-8 decoding. Preview `validateStatement` returns `ok`, `utf8ByteLength`, and stable issue codes as data; `createAssertion` refuses to request a signature when `ok` is false. The onchain handler independently enforces the same acceptance rules, while invalid UTF-8 fails during Rust `String` deserialization. Client and program classification follows the exact character table pinned in ADR-0021 and shared cross-language conformance vectors rather than JavaScript `trim()` or runtime Unicode versions. Interfaces isolate the statement as one plain-text display unit instead of trusting embedded direction controls. Preview issue codes are not aliases for raw Anchor numeric errors. The protocol does not use an arbitrary minimum character count or attempt to grade the prose's meaning. See [ADR-0021](adr/0021-readable-statement-validation.md).
- Preserve amount precision across every layer. Onchain USDC amounts are unsigned `u64` atomic-unit counts. TypeScript client logic uses `bigint`, permitting Anchor `BN` only at the generated-binding boundary and never converting through a JavaScript `number`. Any JSON-shaped Preview Integration API value uses an unsigned base-10 integer string in atomic units; human-unit decimals are display output only. See [ADR-0020](adr/0020-integer-amounts-across-client-boundaries.md).
- `resolutionSpecUri` is the exact 48-byte canonical `ar://` identifier for the off-chain Resolution Spec. Both the client and onchain creation instruction enforce its byte shape. An integrator **must retrieve and cryptographically verify the spec by its known ID, then read it** to judge whether an outcome is meaningful for its use case — outcomes are relative to the spec, not to absolute reality.
- Before upload, the creating client or service enforces the 16,384-byte limit and validates the exact UTF-8 JSON bytes against the declared schema and documented cross-field rules. Before signing Assertion creation, it retrieves the Arweave object, verifies it matches the committed identifier, repeats the size and validation checks on those exact bytes, and refuses to sign on any size, retrieval, integrity, encoding, schema, or cross-field failure. The onchain program cannot perform these checks.
- In `ASSERTED`, the current non-final answer is the optimistic default `True`.
- In `ASSERTED_LLM`, the current non-final answer is `LlmResolutionRound.outcome`.
- In `PENDING_VOTE` and `VOTING`, the LLM answer is under challenge; final answer is not available until `RESOLVED`.
- Irreversible market settlement must require `state == RESOLVED` from a direct account read at Solana `finalized` commitment; `processed`, `confirmed`, or an indexer-only observation is insufficient. See the [Solana commitment reference](https://solana.com/docs/rpc#configuring-state-commitment).
- Consumers should ignore `AssertionAccount.outcome` unless `state == RESOLVED`.
- `Unresolvable` means the claim was affirmatively found undecidable under the spec; `NoConsensus` means the vote reached no supermajority. Integrators may void on either, but must not conflate their participant economics.
- The canonical prediction-market demo resolves YES on `True`, NO on `False`, and uses its invalid-market void/refund path for `Unresolvable`, `NoConsensus`, or `ResolverUnavailable`. It preserves the exact invalid reason and does not treat the shared market action as equivalent Opal economics. See [ADR-0019](adr/0019-prediction-market-invalid-result-mapping.md).
- Consumers can inspect `dispute_count`, `llm_dispute`, `vote_dispute`, `llm_resolution_round`, and `vote_resolution_round` to understand whether the assertion was disputed once or twice and what each layer produced.
- A later correction requires a new assertion. It must not mutate a resolved assertion.
- While an Assertion remains non-final, a source correction can affect a later resolution action: it may motivate an LLM dispute or influence wallets that have not yet cast their immutable Votes. It cannot rewrite an accepted Vote. After terminal finalization, it cannot reopen or update the Assertion.

Finalization is permissionless when its result is completely determined by state and the applicable deadline. Window expiry does not execute the program automatically: an integrator, participant, or other caller must submit the transaction and normally pays its external network cost. The caller cannot choose an outcome or redirect settlement. Until that transaction reaches `finalized`, consumers must continue treating the Assertion as non-final. The current authority gate on `finalize_vote_resolution_placeholder` exists only because that placeholder accepts a caller-supplied outcome; the target tally-derived vote finalizer is permissionless.

## Account Lifecycle

Assertion, dispute, resolution, and per-voter public `VotePositionAccount` payout-status accounts are permanent protocol records. No participant or administrator can close or reuse them after completion; integrators may rely on the referenced history remaining readable. Their account-creation costs are non-recoverable. A Vote Round stores fixed-size aggregates and publication counters, not a growable vector of these records.

Token vaults and transient private infrastructure may close only after explicit zero-balance, zero-lock, and zero-liability checks. In particular, a Vote Settlement Vault remains open while any non-expiring payout is unclaimed. Every closable account records its creation payer as the immutable rent-refund recipient; reclaimed lamports return to that payer, including Opal or an integrator when it sponsored creation. A protocol-owned vault may be closed permissionlessly after all checks pass. A reusable Private Voting Balance may be closed only by its wallet and only when its balance, active locks, and outstanding claims are zero. Closing transient infrastructure must not remove the corresponding permanent public record. See [ADR-0011](adr/0011-permanent-public-records.md).

## External Systems

**LLM resolver** `[MVP-target]`
A single LLM call posted onchain by a trusted off-chain resolver through the resolver-gated `submit_llm_resolution` instruction `[Built]`. This layer does **not** need to be trustless because the staked vote is the trust backstop: a wrong or corrupt verdict is challengeable into the private vote. On-chain provenance hashing is deferred to a `[Vision]` trust-minimized resolver. The former non-mock path — a 3-feed Switchboard council — was removed per [ADR-0002](adr/0002-trusted-llm-resolver.md). Local integration tests call the instruction directly with a test resolver keypair instead of the live resolver service.

**MagicBlock (private ephemeral rollup)** `[MVP-target]`
The final escalation runs inside one deployment-wide MagicBlock **Voting PER**. Voters first deposit USDC into reusable Private Voting Balances through the private-payment/eSPL flow. Every balance, Vote Settlement Vault, and private vote record is pinned to the same validator so funds can be reused across Vote Rounds and settled together. Individual balances, choices, stake amounts, and per-outcome totals are sealed during the Voting Window, preventing the bandwagon/beauty-contest collapse a public running tally would cause. Settlement then publishes the aggregate totals and each wallet's selected outcome and stake ([ADR-0003](adr/0003-private-staked-voting.md)). The `VoteResolutionRound` struct carries the MagicBlock fields (validator, permission account, delegated vote state) and the `delegated`/`committed` lifecycle flags for this purpose. This is the MVP's critical-path dependency.

All vote-stage refunds and rewards—including bonded-participant payouts—become claimable when the Vote Round finalizes. Each recipient must claim into their Private Voting Balance. Only unlocked private balance may be withdrawn: active voting stake remains locked, and an unclaimed payout remains in the Vote Settlement Vault. A wallet may withdraw any positive amount up to its unlocked balance; Opal imposes no withdrawal minimum. Withdrawal is a separate wallet-authorized operation, so neither finalization nor a payout claim initiates a base-layer payout transaction.

Planned MagicBlock implementation requirements:

- Use dual connections: base layer for initialization/delegation, ER connection for operations on delegated vote state, commits, and undelegation.
- Read the fixed Devnet Voting PER validator identity from protocol configuration and resolve its current ER endpoint through the MagicBlock router; do not choose per-round validators or hard-code regional RPC URLs.
- Use the private-payment/eSPL flow for wallet deposits, private-balance reads, private transfers, and withdrawals; Opal remains responsible for voting authorization and settlement rules.
- Enforce a 1 USDC minimum on each Private Voting Balance deposit while permitting withdrawal of any positive unlocked amount.
- Charge no Opal protocol fee on deposits or withdrawals; treat external network and provider execution costs separately.
- Pin each participating Private Voting Balance, Vote Settlement Vault, and private vote state to the configured Voting PER and verify router co-location before casting.
- Move/delegate the complete public Bond Vault balance into the Voting PER during Vote Initialization; start the Voting Window only after co-location is confirmed.
- Initialize and sponsor any missing Private Voting Balance for the three bonded wallets before voting; preserve each wallet as the sole withdrawal authority and charge no extra participant fee.
- Keep balances, positions, and payout claims bound to their original wallet; expose no Opal admin recovery, reassignment, or destination override.
- Freeze every vote-stage entitlement against one Vote Settlement Vault, then let each recipient claim into their Private Voting Balance with double-claim protection.
- Treat deposit/withdrawal transaction construction, source confirmation, and final token settlement as distinct states; expose `Pending`, `Completed`, and `Recovery Needed`, with reconciliation and refund/recovery paths.
- Send delegation transactions to the base layer.
- Send vote-cast, reveal/settlement mutations, commit, and undelegate transactions to the ER connection after delegation.
- Use matching PDA seeds between account definitions and delegate calls.
- Check delegation status before accepting ER-side vote mutations.
- Accept a Vote only when its transaction executes on the Voting PER with `execution_time < voting_deadline`; submission time does not reserve a Vote.
- Create at most one position for each wallet-and-round pair; after the first accepted Vote, reject every retry or competing submission before any additional stake movement.
- Assess the Voting Fee only for an accepted position during settlement; failed or rejected Vote transactions create no Opal fee liability.
- Use `skipPreflight: true` for ER transactions where the client stack requires it.
- Wait for state propagation after delegation and undelegation in tests.

## Vision (post-MVP)

These are recorded so the direction is clear; none of them is current behavior. Do not build them yet.

**Trust-minimized / permissionless LLM** `[Vision]`
A future hardening path replaces the trusted resolver with trust-minimized or permissionless inference — Switchboard On-Demand feed(s) or TEE-attested inference; on-chain LLM provenance hashing, if any, would land here. A three-feed Switchboard "council" was once the non-mock resolution path, but was never operationally stood up (tests only ever ran the mock) and was **removed** for the MVP: it bought trust-minimization at a layer already backstopped by the vote, at the cost of operating live feeds. See [ADR-0002](adr/0002-trusted-llm-resolver.md).

**OPAL token & governance** `[Vision]`
A governance/reputation/staking token. Every job once assigned to it — voting weight, voter incentives, governance — is handled in the MVP by staked USDC and the `authority` keypair, so OPAL is deferred. See [ADR-0004](adr/0004-single-asset-usdc.md).

**Proof-of-personhood & sub-linear weighting** `[Vision]`
Proof-of-personhood would enable sub-linear/quadratic vote weighting without Sybil collapse, hardening against the residual 51%-of-stake attack. See [ADR-0003](adr/0003-private-staked-voting.md).

**Stake-duration reputation** `[Vision]`
Long-term staking that accrues voter weight/reputation over time.

**Timed resolution** `[Vision]`
A future protocol-level lifecycle field could prevent an Assertion from resolving before the underlying truth exists. If introduced, it belongs to the protocol account rather than the Resolution Spec; spec authors never choose private evidence cutoffs under [ADR-0013](adr/0013-protocol-time-evidence.md).

**Other considered-and-deferred mechanisms** `[Vision]`
Time-weighted voting (TWAV) — considered and **rejected** (weighting earlier votes empowers a first-moving attacker); recorded so it is not reintroduced. On-chain commit-reveal — the considered alternative to MagicBlock for private voting, rejected for the MVP. Nosana-powered inference — a possible operator-decentralization path for the LLM layer.
