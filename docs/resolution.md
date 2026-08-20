# Opal Resolution

Resolution is the process that turns the optimistic default answer into a final answer. An assertion starts in `Asserted`, where the statement is treated as `True` by default; disputes move it through an LLM resolution round and, if that is challenged, a private staked USDC vote.

Throughout, "truth" means **rubric-relative truth**: the answer is judged against the assertion's own Resolution Spec, not against universal reality. See the [glossary](glossary.md) for vocabulary and [ADR-0001](adr/0001-rubric-relative-truth.md) for the rationale.

## Assertion Requirements

Every assertion includes:

1. An `assertionId`: the public key of a fresh client-generated keypair that signs creation, preventing another wallet from claiming the identifier. The temporary key has no authority after creation confirms and may then be discarded. `[MVP-target]`
2. An onchain `statement`: a short natural-language sentence stored in a fixed 280-byte field. Storage and the current maximum check are `[Built]`; the complete 1–280-byte, single-line UTF-8 validation and exact-byte preservation rules are `[MVP-target]` per [ADR-0021](adr/0021-readable-statement-validation.md).
3. An onchain `resolution_spec_uri: [u8; 48]`: the exact canonical `ar://<id>` URI of the off-chain **Resolution Spec**. The current `auxiliary_hash: [u8; 128]` storage is `[Built]`; narrowing, renaming, exact validation, and the spec workflow are `[MVP-target]`.

Once the creation transaction is accepted, the statement, Resolution Spec URI, 500 USDC Participant Bond, and all resulting deadlines are immutable. The asserter cannot edit or cancel the Assertion or withdraw the bond early. Correcting a mistake requires creating a new Assertion; the original remains active and continues to its own final outcome. Identical statement and Resolution Spec content may appear in several Assertions. Each has its own unique Assertion ID and remains independent; Opal does not choose a latest or canonical replacement. Within a configured deployment, that ID is sufficient for public integrations to reference the Assertion.

The Assertion and its linked dispute, resolution, and public payout-status records remain permanently readable. They cannot be closed for rent recovery after resolution. Only empty vaults and transient private infrastructure may close, and only after all balances, locks, and payout liabilities are zero. See [ADR-0011](adr/0011-permanent-public-records.md).

### The Resolution Spec `[MVP-target]`

The Resolution Spec is the immutable, time-independent asserter-supplied rubric that defines _how_ the statement resolves: authoritative sources in priority order, deterministic conflict and source-unavailability rules, key definitions, and ambiguity handling. **It is the source of truth.** The LLM resolver and the voters _apply_ the spec; they do not adjudicate absolute reality, so the same statement text can resolve differently across assertions. A one-source spec satisfies the ordering rule automatically. If a multi-source hierarchy cannot settle a material conflict or availability gap, the result is `Unresolvable`; actors cannot invent an unstated preference. The spec cannot impose an asserter-selected evidence cutoff, freeze a source version, exclude later corrections, or instruct the protocol to wait until a chosen date. See [ADR-0001](adr/0001-rubric-relative-truth.md), [ADR-0013](adr/0013-protocol-time-evidence.md), and [ADR-0014](adr/0014-ordered-resolution-sources.md).

The spec lives off-chain on **Arweave** (permanent, content-addressed). Its uploaded bytes are UTF-8 JSON with `schemaVersion: "1"` and must validate against the published [Resolution Spec v1 JSON Schema](schemas/resolution-spec-v1.schema.json). V1 requires explicit definitions, at least one ordered source with source-unavailability behavior, `conflictBehavior: "highest-priority-wins"`, and non-empty ambiguity handling. The optional `rationale` is human-readable context only and cannot override a typed rule. Unknown fields are invalid, creating clients additionally reject `next-source` on the final source, and the exact encoded JSON cannot exceed 16 KiB (16,384 bytes). That size is an Opal application resource bound, not an Arweave protocol restriction, upload tier, or pricing threshold. A future schema version cannot reinterpret an existing v1 spec. See [ADR-0016](adr/0016-versioned-resolution-spec-json.md).

The program stores the spec's canonical `ar://<id>` URI, where the ID is an Arweave L1 transaction or ANS-104 data-item ID. Creating clients fail closed unless signature/data-root verification proves the retrieved bytes match that known ID. Raw digests, HTTP gateway URLs, mutable ArNS names, queries, and fragments are invalid. The onchain protocol cannot fetch Arweave or validate the content schema. If a bypassing client creates an Assertion with a malformed spec, applying it produces `Unresolvable` rather than an inferred source preference. Vetting the spec before trusting an outcome remains the integrator's responsibility. See [ADR-0015](adr/0015-canonical-resolution-spec-uri.md).

Before upload, the creating client or integration service checks the 16,384-byte limit and validates the exact JSON bytes against the declared schema and documented cross-field rules. Before an Assertion creation transaction is signed, it retrieves the uploaded Arweave object, verifies that its content matches `resolutionSpecUri`, and repeats the size and validation checks on those exact bytes. It must refuse creation when the size limit, retrieval, integrity verification, UTF-8 decoding, schema validation, or a cross-field check fails. The Solana program cannot enforce these checks onchain; consumers must still retrieve, verify, validate, and read the Resolution Spec independently before trusting the resulting outcome.

The asserter normally uploads the spec and pays the external Arweave storage cost before creation; an integrating application may sponsor that upload. Opal charges no storage fee. If an integration supplies an existing upload, the client reuses it without a duplicate storage payment only after the same retrieval and integrity checks pass.

Evidence timing is supplied by the protocol rather than the spec. Each resolution action uses the latest relevant information available from the spec's authoritative sources when that action occurs. A source correction during the LLM Dispute Window can motivate a challenge, and a correction during the Voting Window can influence wallets that have not yet voted. Accepted Votes remain immutable, so the correction cannot rewrite earlier positions. After a terminal outcome, later corrections never reopen the Assertion; an integrator that needs a new judgment creates a new Assertion.

## Outcome Rules

The final `outcome: u8` is one of:

**`True` (0)** `[Built]`
The evidence, applied through the spec, verifies the statement.

**`False` (1)** `[Built]`
The evidence, applied through the spec, contradicts the statement.

**`Unresolvable` (3)** `[MVP-target]`
An affirmative finding that the statement cannot be decided under the spec: source priority is unclear, evidence conflicts or is unavailable, the statement is ambiguous, the spec is too weak, or the truth does not exist yet. The asserter is incorrect. In voting, `Unresolvable` must itself reach the 67% supermajority to win.

**`NoConsensus` (2)** `[MVP-target]`
A vote-only outcome used when none of `True`, `False`, or `Unresolvable` reaches 67%. It assigns no fault: every bond and voting stake is returned minus ordinary fees, and no rewards are paid. It reuses the legacy `TooEarly` code before Devnet deployment. See [ADR-0007](adr/0007-unresolvable-vs-no-consensus.md).

**`None` (255)** `[Built]` — sentinel for unset; the value of `outcome` until `state == Resolved`.

These four normal terminal outcomes—`True`, `False`, `Unresolvable`, and `NoConsensus`—cover the expected resolution path. If the trusted resolver fails to post within 24 hours after the first dispute is accepted, the Assertion instead becomes eligible for the exceptional terminal status `ResolverUnavailable` `[MVP-target]`. That status is not a fifth truth outcome: it records an operational failure, keeps `outcome = None`, and must not be interpreted as a judgment under the Resolution Spec. Public documentation presents this as an edge case, not as part of ordinary outcome selection; an exact `99.9%` completion claim is published only if production telemetry supports it.

The canonical prediction-market integration resolves YES on `True` and NO on `False`. It treats `Unresolvable`, `NoConsensus`, and `ResolverUnavailable` as invalid market results and runs one documented trader void/refund path while preserving the exact reason. That market-level convergence does not change Opal economics: `Unresolvable` is fault-assigned with ordinary fees, slashing, and rewards; `NoConsensus` is no-fault but still charges ordinary fees; `ResolverUnavailable` charges no Bond Fee and returns both accepted bonds in full. See [ADR-0019](adr/0019-prediction-market-invalid-result-mapping.md).

## Current Answer By State

`AssertionAccount` does not store a tentative-resolution field. The current non-final answer is inferred from `state` and the linked round accounts:

- `Asserted` (0): current non-final answer is the optimistic default `True`.
- `PendingLLM` (1): the default `True` has been disputed; no LLM verdict posted yet.
- `AssertedLLM` (2): current non-final answer is `LlmResolutionRound.outcome`.
- `PendingVote` (3): the LLM verdict has been challenged; the vote is being initialized immediately. This is a transient state, not a participant-facing window.
- `Voting` (4): final answer is pending vote settlement.
- `Resolved` (5): final answer is `AssertionAccount.outcome`.

## Lifecycle

The six-state normal-resolution machine and its plumbing — account structs, transitions, and the optimistic + dispute flow — are `[Built]`. The trusted resolver, exceptional `ResolverUnavailable` terminal path, and private vote that ride on top are `[MVP-target]`; the former non-mock resolution path (a Switchboard council) was removed per [ADR-0002](adr/0002-trusted-llm-resolver.md); verdicts are posted via the resolver-gated `submit_llm_resolution` (see below).

### 1. Asserted `[Built]`

When an assertion is created (`create_assertion`):

- the fresh Assertion ID keypair signs creation; its public key becomes the permanent `assertionId` but grants no authority after confirmation
- `[MVP-target]` the statement is single-line UTF-8 occupying 1–280 bytes, without the boundary-whitespace, control-character, line/paragraph-separator, or bidirectional-control code points pinned in ADR-0021; ordinary right-to-left letters remain valid. The program rejects anything outside those structural rules and preserves accepted bytes without normalization or rewriting. JavaScript and TypeScript clients reject ill-formed strings, measure `TextEncoder` byte length rather than `string.length`, and use the same unchanged validated value for the signing preview and Anchor transaction. Client and program checks share conformance vectors against the pinned table rather than relying on JavaScript `trim()` or runtime Unicode versions
- the asserter posts the 500 USDC Participant Bond (`assertion_bond_amount_pusd`; the field is renamed `*_usdc` in a later PR per [ADR-0004](adr/0004-single-asset-usdc.md))
- the asserter normally pays Solana transaction and account-creation costs; an integrating application may sponsor them, but Opal charges no separate upfront assertion fee
- `state = Asserted`
- `outcome = None` (255)
- `liveness_deadline` marks the end of the seven-day Assertion Dispute Window
- `[MVP-target]` `resolution_spec_uri` contains the exact 48-byte canonical `ar://<id>` for the off-chain Resolution Spec, enforced by the program (the current field is still named `auxiliary_hash`)
- the statement, Resolution Spec URI, bond, and deadlines become immutable; there is no cancellation or early bond withdrawal

The 1% Bond Fee is withheld only when the Assertion settles; it is not an additional creation-time charge.

While `Asserted`, the statement is treated as `True` by default. The seven-day **Assertion Dispute Window** is the only time a first dispute can be filed; if it expires undisputed, the assertion can be finalized `True` (`finalize_undisputed`).

> **Implementation mismatch.** The current instruction accepts a caller-supplied `assertion_id` without requiring that key to sign, which permits identifier front-running. It also enforces only maximum lengths for the statement and `auxiliary_hash`; the complete statement rules and exact Resolution Spec URI shape are not yet checked onchain, and the URI field is still 128 rather than 48 bytes. The Devnet target requires the ID signer, field narrowing, and target validation per [ADR-0010](adr/0010-signed-assertion-identifiers.md), [ADR-0015](adr/0015-canonical-resolution-spec-uri.md), and [ADR-0021](adr/0021-readable-statement-validation.md).

### 2. PendingLLM `[Built]` plumbing / `[MVP-target]` resolver

When the first dispute is filed (`dispute_assertion`):

- the LLM disputer posts the same 500 USDC Participant Bond as the asserter
- the LLM disputer's wallet must differ from the asserter's wallet
- `LlmDisputeAccount` is created
- `LlmResolutionRound` is created
- `state = PendingLLM`
- `dispute_count = 1`

If several wallets submit a first dispute, the first valid transaction accepted before the Assertion Dispute Window deadline becomes the LLM Disputer. Later competing or retried transactions fail without locking a Participant Bond or incurring an Opal Bond Fee; external execution costs may still apply.

The on-chain half is `[Built]`: `submit_llm_resolution` posts the verdict and is gated on a dedicated `ProtocolConfig.resolver` key — deliberately separate from `authority`, so a leaked hot resolver key can only post a challengeable verdict, not act as governance. It accepts only `True`, `False`, or `Unresolvable`; `NoConsensus` is produced only by voting. The off-chain **trusted resolver** service that makes the single LLM call and submits through this instruction is `[MVP-target]`. The LLM layer does not need to be trustless because the staked vote backstops it (a wrong verdict is challengeable). Binding LLM provenance (prompt/response/evidence hashes) on-chain for auditability is **not part of the MVP**; it is deferred to a `[Vision]` trust-minimized/permissionless resolver.

On localnet, integration tests call `submit_llm_resolution` directly with a test resolver keypair; the former `mock-llm` feature and `submit_mock_llm_resolution` were removed when the real instruction landed. (An earlier instruction of the same name belonged to the 3-feed Switchboard council — `set_council_feeds`, `council_feeds`, `switchboard_*`, and `*_hash` fields — removed per [ADR-0002](adr/0002-trusted-llm-resolver.md); it was compiled but never operationally stood up.)

The Devnet target adds an exceptional terminal `ResolverUnavailable` path when no resolver verdict arrives within 24 hours after the first dispute is accepted. Verdicts are accepted only before the Resolver Deadline. At or after it, anyone may invoke a deterministic timeout finalizer that atomically returns the full 500 USDC bond to both the asserter and LLM disputer. The caller cannot choose an outcome or redirect either refund. No Bond Fee, slashing, or reward applies; the caller pays the external transaction cost unless an integrator sponsors it. This path exists only to prevent permanent bond lock during a resolver outage and does not produce a Resolution Outcome. See [ADR-0012](adr/0012-resolver-unavailable.md).

### 3. AssertedLLM `[Built]`

When the LLM verdict is posted:

- `LlmResolutionRound.outcome` is set
- `state = AssertedLLM`
- the seven-day **LLM Dispute Window** opens (`llm_challenge_deadline`)

The current non-final answer is now the LLM result. If no second dispute is filed before `llm_challenge_deadline`, the assertion is finalized with the LLM outcome (`finalize_llm_resolution`).

### 4. PendingVote `[Built]` plumbing / `[MVP-target]` vote

When the LLM verdict is challenged (`challenge_llm_resolution`):

- the vote disputer posts the same 500 USDC Participant Bond as the asserter and first disputer
- the vote disputer's wallet must differ from both the asserter's and LLM disputer's wallets
- `VoteDisputeAccount` is created, recording `challenged_llm_resolution` (the LLM outcome being contested)
- `VoteResolutionRound` is created
- `state = PendingVote`
- `dispute_count = 2`

If several wallets challenge the LLM verdict, the first valid transaction accepted before the LLM Dispute Window deadline becomes the Vote Disputer. Later competing or retried transactions fail without locking a Participant Bond or incurring an Opal Bond Fee; external execution costs may still apply.

**Vote Initialization** begins immediately. The third 500 USDC Participant Bond joins the other two in the public Bond Vault, then the full 1,500 USDC vault balance is moved/delegated into a Vote Settlement Vault on the deployment-wide **Voting PER**. Opal creates and sponsors any missing Private Voting Balances for the asserter and either disputer; sponsorship does not grant Opal token authority and adds no participant fee. Other participants fund their reusable balance with deposits of at least 1 USDC each before voting. The protocol verifies that the vault, private vote state, and private participant balances share the Voting PER's router-resolved endpoint. `PendingVote` represents this transient automatic work; it is not a separate timed window for participants. The Devnet protocol model assumes initialization completes successfully and defines no timeout or abort outcome.

### 5. Voting `[MVP-target]`

When `open_vote` is called:

- Vote Initialization and PER co-location must already be complete
- the seven-day **Voting Window** is set on `VoteResolutionRound` (`voting_starts_at`, `voting_deadline`)
- `state = Voting`

Before casting, a voter must have enough USDC in a reusable **Private Voting Balance** funded through MagicBlock's private-payment/eSPL flow. Funding happens separately from voting. The voter also normally funds the SOL account-creation balance for its permanent public position record; an integration may sponsor that external cost, but Opal does not guarantee sponsorship. An accepted Vote makes the record and its account-creation balance permanent. If position setup occurs but the Vote is never accepted, a safe unused-setup close returns the lamports to the original payer. Preparing this base-layer record may reveal that the wallet intends to participate, but not its selected outcome or stake. The base-layer USDC deposit is also public, while the resulting balance is private; casting locks the chosen stake from that balance inside the PER, while unlocked funds remain available for another Vote Round or withdrawal. When the vote finalizes, fee-adjusted stake refunds and rewards become claimable from the Vote Settlement Vault. A voter must claim them into the Private Voting Balance before reusing or withdrawing them.

The vote is a private, per-dispute, USDC-staked Schelling-point vote with **linear weight** (1 staked USDC = 1 vote). During the Voting Window, a **MagicBlock Private Ephemeral Rollup** `[MVP-target]` hides individual choices, stake amounts, private balances, and per-outcome totals, preventing the bandwagon collapse a public running tally would cause. After settlement, the aggregate stake for each outcome, every wallet's selected outcome and stake, and each position's exact payout amount and `Claimable`, `Claimed`, or `Slashed` status become public; total Private Voting Balances remain private. Each revealed Vote uses a permanent public `VotePositionAccount` keyed by the Vote Round and wallet. The round stores fixed-size aggregates and publication progress rather than a growable voter vector. See [ADR-0017](adr/0017-per-voter-public-position-accounts.md).

Before an outcome can win, the Vote Round must contain at least 3,000 USDC in total **Gross Voting Stake**—the original stake locked before fees—and Votes from at least 10 distinct wallet addresses. Once both quorum conditions are met, `True`, `False`, or `Unresolvable` must reach an inclusive 67% of Gross Voting Stake; exactly 67.000% wins. The 0.1% Voting Fee is applied only during settlement and cannot change quorum, voting weight, or the outcome. Otherwise the vote resolves `NoConsensus`. See [ADR-0003](adr/0003-private-staked-voting.md) and [ADR-0007](adr/0007-unresolvable-vs-no-consensus.md).

A Vote counts only when its transaction is accepted and recorded on the Voting PER before the Voting Window deadline. Signing or submitting before the deadline is insufficient if the transaction lands after it; late transactions are rejected. The first accepted submission for a wallet-and-round pair establishes its immutable Vote. Any later retry or competing submission fails onchain without changing the outcome or stake, locking more USDC, or incurring an Opal Voting Fee. After a duplicate rejection or ambiguous submission result, the Preview layer returns reconciled success only if an authoritative record matches the requested round, wallet, outcome, and Gross Voting Stake exactly; a different immutable Vote remains a conflict. External execution costs may still apply.

The Asserter, LLM Disputer, and Vote Disputer may each cast their wallet's one Vote. Their existing bonded role does not consume that Vote, and they may select any outcome even if it conflicts with the outcome that makes their bond correct. Each bonded wallet with a valid Vote also counts once toward the 10-wallet Vote Quorum. The Participant Bond and Vote are classified independently during settlement: each correct position contributes its ordinary Reward Weight, while each incorrect position is slashed. A single wallet can therefore lose its bond while winning its Vote, or win its bond while losing its Vote. The one-Vote limit applies independently to each wallet-and-round pair; the same wallet may vote in concurrent Vote Rounds when its unlocked Private Voting Balance covers every stake.

> Today `open_vote` is permissionless and sets `delegated = BOOL_TRUE` without performing real MagicBlock delegation; the MagicBlock ER integration is the MVP's critical-path build. This is a build-toward target, not the intended end state.

### 6. Resolved `[Built]`

When a resolution path finalizes:

- `state = Resolved`
- `outcome` is set
- `finalized_at` is set
- dispute settlement fields (`settlement_resolution` on the dispute accounts) are populated
- pre-vote paths redistribute bonds immediately; vote resolution freezes claimable payouts in the Vote Settlement Vault

Three paths lead to `Resolved`:

1. **Undisputed:** `finalize_undisputed` sets `outcome = True`.
2. **LLM resolution:** `finalize_llm_resolution` sets `outcome = LlmResolutionRound.outcome`.
3. **Vote resolution:** `finalize_vote_resolution_placeholder` sets the final outcome. It currently takes a mock outcome argument; real vote tallying from the MagicBlock aggregate is `[MVP-target]`.

Window expiry alone does not execute a Solana program. After the applicable deadline, anyone may submit the deterministic finalization transaction and normally pays its external network cost. The caller cannot select the outcome or redirect settlement. Until a finalization transaction lands, the Assertion remains in its eligible pre-final state and integrators must not treat it as final.

## Settlement Logic

Settlement decides who is paid and who is slashed. The unit of account is USDC. At normal settlement, the Devnet MVP withholds a 1% non-refundable fee from every Participant Bond and a 0.1% non-refundable fee from voting stake. At vote stage, the remaining slashed collateral is distributed through the unified capital-weighted pro-rata rule. The exceptional `ResolverUnavailable` path refunds both bonds in full because Opal did not complete the resolution service. See [ADR-0004](adr/0004-single-asset-usdc.md) for the single-asset model, [ADR-0008](adr/0008-equal-bonds-fees-and-pro-rata-rewards.md) for settlement economics, and [ADR-0012](adr/0012-resolver-unavailable.md) for the resolver-failure exception.

### Resolver Unavailable — operational failure `[MVP-target]`

If the resolver posts no verdict before the 24-hour Resolver Deadline, a permissionless timeout finalizer atomically returns the full 500 USDC bond to both the asserter and LLM disputer through their public Solana token accounts. Neither bond pays the 1% Bond Fee, nobody is slashed, and no reward is paid. This is the sole exception to ordinary per-bond fees because Opal did not complete the resolution service. External transaction and account-creation costs are not reimbursed. The Assertion terminates as `ResolverUnavailable` with `outcome = None`; it does not enter `Resolved` with one of the four Resolution Outcomes.

### Undisputed (`True`) `[Built]`

If the Assertion Dispute Window expires with no dispute:

- assertion resolves `True`
- the 500 USDC asserter bond refunds 495 USDC after the 1% Bond Fee
- treasury receives the fee

### No Consensus — no-fault `[MVP-target]`

If the Vote Quorum is not met, or quorum is met but no voting option reaches 67%, the assertion resolves `NoConsensus` and settles no-fault:

- each owner may claim 495 USDC from an unslashed 500 USDC Participant Bond after the 1% Bond Fee
- each voter may claim unslashed voting stake after the 0.1% Voting Fee
- **no one is slashed** — not the asserter, not any disputer, not any voter; ordinary fees still apply
- no rewards are paid

In a zero-Vote round, all three Participant Bonds therefore refund 495 USDC each. No Voting Fee is collected because no voting stake exists.

`NoConsensus` says the vote did not establish a valid winning side; it does not claim that the assertion itself was affirmatively unresolvable. See [ADR-0007](adr/0007-unresolvable-vs-no-consensus.md).

### First Dispute (LLM Resolution) — fault-assigned outcome `[MVP-target]`

If the LLM verdict is not challenged, settlement follows whether the assertion resolved `True`:

- if the outcome is `False` or `Unresolvable`, the LLM disputer was correct because the disputed default `True` was overturned:
  - the LLM disputer receives 495 USDC refunded principal plus the asserter's remaining 495 USDC bond
- if the outcome is `True`, the LLM disputer was incorrect:
  - the asserter receives 495 USDC refunded principal plus the LLM disputer's remaining 495 USDC bond

This is the **Winner Takes Remaining Bond** rule: the single correct participant receives 990 USDC, and Opal retains 10 USDC across the two Bond Fees.

### Second Dispute (Vote Resolution) `[MVP-target]`

If the Vote Quorum is met and `True`, `False`, or `Unresolvable` reaches 67%, that vote outcome is final and settlement runs in two parts.

**Bond settlement** mirrors the first dispute, decided by the final outcome:

- if the final outcome is `False` or `Unresolvable`, the LLM disputer is correct; if it is `True`, the asserter is correct
- the vote disputer is correct when the final outcome differs from `challenged_llm_resolution`
- each correct bonded participant may claim their 495 USDC refund into a Private Voting Balance and contributes 500 units of Reward Weight
- each incorrect bond contributes its remaining 495 USDC to the Slashed Pool

**Voter settlement (Schelling-point slashing):**

- winning voters receive their stake after the 0.1% Voting Fee and contribute one unit of Reward Weight per originally staked USDC
- losing voting stake enters the Slashed Pool after the 0.1% Voting Fee
- a fully slashed bond or Vote has no payout and requires no claim
- when the Voting Fee is not exactly representable in USDC atomic units, it rounds down to the nearest micro-USDC and the participant keeps the remainder
- the Slashed Pool is distributed pro rata across every correct bonded participant and winning voter according to Reward Weight
- Opal takes no additional treasury share beyond Bond Fees and Voting Fees
- security comes from this slashing, not from any weight curve

Every vote-stage payout—bonded participant refund, voting-stake refund, or reward—becomes claimable as soon as Vote Round finalization confirms. There is no cooldown or separate claim-opening transaction. The recipient must submit a wallet-authorized claim to credit it to their Private Voting Balance. Once the claim confirms, the USDC is immediately unlocked for another Vote or withdrawal. After an already-claimed rejection or ambiguous result, the Preview layer returns reconciled success only when the authenticated position, owner, fixed payout, and protocol-derived destination prove that this claim was already applied. The claim itself is still not a withdrawal to the public Solana wallet. Settlement paths that finish before voting continue to pay public Solana token accounts automatically.

Outcome finalization does not enumerate or publish an unbounded voter set in one transaction. It freezes the immutable position count, then deterministic bounded batches publish one permanent `VotePositionAccount` per voter. Publication is permissionless and does not expire: any caller can publish a still-missing valid batch, cannot alter its records or advance progress twice, and normally pays the external transaction cost unless an integration sponsors it. Opal charges no publication fee and promises no caller reward. The Vote Round exposes publication progress and a completion marker. Until that marker is set, the public wallet-to-Vote mapping is incomplete even though the Assertion outcome and payout formula are final. A full position-list consumer must wait for publication completion; an integration using only the final Assertion outcome does not. A wallet's bonded and Vote positions remain independent records when it holds both roles.

If quorum is met and `Unresolvable` reaches 67%, it is the winning voter side: `True` and `False` voting stake is slashed, and `Unresolvable` voters participate in pro-rata rewards. If either quorum condition fails, or quorum is met but no option reaches 67%, `NoConsensus` follows the no-fault path and nobody receives rewards. See [ADR-0003](adr/0003-private-staked-voting.md) and [ADR-0007](adr/0007-unresolvable-vs-no-consensus.md).

> Pro-rata voter payouts are not yet distributed in code. The current role-specific `*_reward_share_bps` fields and placeholder vote finalization are legacy implementation under this target model.

## Vision (post-MVP)

Recorded so the direction is clear and nobody mistakes these for current behavior:

- **Trust-minimized / permissionless LLM** — Switchboard On-Demand feed(s), TEE-attested, or otherwise permissionless inference replacing the trusted resolver; on-chain LLM provenance hashing, if any, would land here. (The 3-feed Switchboard "council" and its `switchboard_*` fields were removed per [ADR-0002](adr/0002-trusted-llm-resolver.md).)
- **OPAL token** — governance and future voter incentives; dropped from the MVP, where governance is the `authority` keypair and stake is USDC. See [ADR-0004](adr/0004-single-asset-usdc.md).
- **Timed resolution** — a possible future protocol-level lifecycle field preventing premature resolution. It would not be content or an asserter-selected cutoff inside the immutable Resolution Spec.
- **Proof-of-personhood / quadratic weighting** and **stake-duration reputation** — Sybil-resistant sub-linear voting and long-term voter reputation.
- **On-chain commit-reveal** — the considered-and-rejected alternative to MagicBlock for private voting, recorded so the trade-off isn't relitigated.
