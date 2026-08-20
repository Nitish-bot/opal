# Opal Glossary

Opal is a Solana optimistic oracle that resolves **rubric-relative truth**: every assertion ships its own Resolution Spec saying _how_ it should be judged, and Opal applies that spec rather than adjudicating any universal truth. Statements are treated as `True` by default unless an economically-incentivized disputer challenges them — so no external monitoring/bot layer is required.

This glossary is the shared vocabulary and domain model. Mechanics live in [architecture.md](architecture.md), [resolution.md](resolution.md), and [tokenomics.md](tokenomics.md); the rationale behind the big decisions lives in [adr/](adr/).

## Status legend

Every feature is tagged so you never have to guess what's real:

- **`[Built]`** — implemented in the program today.
- **`[MVP-target]`** — committed for the MVP, not yet built. Build toward this.
- **`[Vision]`** — post-MVP / aspirational. Do **not** build it yet.

## Core concepts

**Opal**
A Solana-native optimistic oracle for natural-language statements, resolved relative to each assertion's Resolution Spec. Default-`True`; disputes escalate through an LLM resolution round and, if challenged, a staked private vote.

**Assertion** — `AssertionAccount` `[Built]`
The on-chain object created when an asserter posts a statement, a USDC bond, and a Resolution Spec.

**Assertion ID** — `AssertionAccount.id` — the canonical public reference for an Assertion within the configured Opal deployment. The client generates a fresh random keypair and its public key becomes the ID; `create_assertion` requires the temporary key to sign, preventing another wallet from front-running the identifier. The client retains the secret only until creation confirms, after which it grants no authority and may be discarded. IDs are unique and never reused in one deployment. Public integrations accept and persist the ID; the deployment configuration supplies the network and program context, and the underlying Assertion account address is an implementation detail. See [ADR-0010](adr/0010-signed-assertion-identifiers.md).
_Avoid_: requiring public API users to construct an Assertion PDA; treating statement text as an identifier; implying the Assertion ID key controls the Assertion after creation.

**Assertion Immutability** — once the creation transaction is accepted, the Assertion's statement, Resolution Spec URI, Participant Bond, and deadlines cannot be edited. The asserter cannot cancel the Assertion or withdraw its bond early. A correction requires a new Assertion, and the original continues independently to resolution.
_Avoid_: “update an assertion,” “cancel an assertion,” or implying that a replacement removes the original.

**Duplicate Assertions** — Opal permits multiple Assertions with identical statement and Resolution Spec content. Every Assertion has its own Assertion ID, bond, windows, lifecycle, and outcome. The protocol does not designate a latest or canonical replacement; an integrator must select the exact Assertion ID it consumes.
_Avoid_: identifying an Assertion by statement text alone; “the latest assertion wins.”

**Statement** — `statement: [u8; 280]` `[Built]` storage / `[MVP-target]` validation
The short human-readable sentence whose truth is asserted. It must be single-line UTF-8 occupying 1–280 bytes, must not begin or end with the boundary-whitespace characters pinned by ADR-0021, and must not contain the pinned control-character ranges, line/paragraph separators, or invisible bidirectional controls. Ordinary right-to-left letters remain valid. Length is measured in encoded bytes, not characters. JavaScript and TypeScript clients reject ill-formed strings and use `TextEncoder` byte length rather than `string.length`. Client and program validation share conformance vectors against the fixed table instead of taking protocol behavior from JavaScript `trim()` or a runtime Unicode version. After validation, clients and the program preserve the submitted UTF-8 bytes without normalization, trimming, case conversion, or other rewriting. The exact bytes occupy the start of the onchain field and the unused suffix is zero-filled; a full 280-byte statement requires no terminator. The program enforces these structural rules but does not attempt to judge grammar or semantic quality. See [ADR-0021](adr/0021-readable-statement-validation.md).
_Avoid_: an arbitrary minimum character count; using JavaScript `string.length` for the byte limit; using `trim()` as the protocol definition; accepting embedded bidirectional controls; describing the field as always null-terminated; implying that the program can prove prose is meaningful English; silently normalizing or rewriting accepted text.

**Resolution Spec** — `resolution_spec_uri` + off-chain content `[MVP-target]`
The immutable, time-independent asserter-supplied rubric that defines _how_ the statement resolves: authoritative sources in priority order, deterministic conflict and source-unavailability rules, key definitions, and ambiguity handling. **It is the source of truth** — the LLM resolver and voters apply it; they do not judge absolute reality. A single-source spec satisfies the ordering rule automatically. If a multi-source hierarchy cannot settle a material conflict or availability gap, the result is `Unresolvable`; actors cannot invent an unstated preference. The spec cannot impose an asserter-selected evidence cutoff, freeze a source version, exclude later corrections, or tell the protocol to wait until a date. It lives off-chain on Arweave (permanent, content-addressed); the canonical `ar://` Resolution Spec URI is stored onchain so anyone can retrieve and verify it. The current program field is still named `auxiliary_hash`; it is renamed `resolution_spec_uri` before Devnet. See [ADR-0001](adr/0001-rubric-relative-truth.md), [ADR-0013](adr/0013-protocol-time-evidence.md), [ADR-0014](adr/0014-ordered-resolution-sources.md), and [ADR-0015](adr/0015-canonical-resolution-spec-uri.md).
_Avoid_: "auxiliary data" framed as optional hints; "evidence" — it is the spec, not a hint; spec-defined evidence cutoffs or mutable source snapshots.

**Resolution Spec Upload** `[MVP-target]` — the permanent Arweave upload completed before Assertion creation. The asserter or an integrating sponsor pays the external storage cost directly; Opal charges no storage fee. A previously uploaded spec may be reused without paying for a duplicate upload when the creating client retrieves it and verifies that its bytes match the supplied immutable identifier.
_Avoid_: implying that Opal hosts mutable spec content; charging an Opal storage fee; requiring duplicate uploads of verified identical content.

**Resolution Spec URI** — `resolutionSpecUri` `[MVP-target]` — the exact 48-byte canonical identifier `ar://<id>`, where `<id>` is a 43-character unpadded base64url Arweave transaction or data-item ID. Both clients and the onchain creation instruction enforce its lowercase prefix, exact length, and identifier alphabet; gateways, ArNS names, queries, fragments, and other free-form values are invalid. Clients use the known ID for fail-closed Arweave signature/data-root verification; the protocol does not require a separate content digest.
_Avoid_: `auxiliaryHash` in public APIs; HTTP gateway URLs; mutable ArNS names; accepting a raw digest that cannot locate the spec.

**Resolution Spec Schema v1** `[MVP-target]` — the published JSON Schema for a Resolution Spec's UTF-8 JSON. It requires `schemaVersion: "1"`, explicit definitions, one or more ordered sources with source-unavailability behavior, `conflictBehavior: "highest-priority-wins"`, and non-empty ambiguity handling. An optional `rationale` is human-readable but non-normative. Unknown fields are invalid, the final source cannot use `next-source`, and the exact encoded JSON cannot exceed 16 KiB (16,384 bytes). The size limit is an Opal application bound, not an Arweave tier or protocol limit. See [ADR-0016](adr/0016-versioned-resolution-spec-json.md) and the [v1 schema](schemas/resolution-spec-v1.schema.json).
_Avoid_: a free-form Markdown rubric; treating `rationale` as an override; interpreting a v1 spec under rules introduced by a later schema version; describing 16 KiB as an Arweave restriction.

**Evidence Timing** `[MVP-target]` — the protocol-wide rule that each resolution action applies the unchanged Resolution Spec to the latest relevant information available from its authoritative sources when that action occurs. A later source correction can affect a later resolution layer while the Assertion is non-final, but cannot change an already accepted immutable Vote or reopen a terminal Assertion. An integrator needing a judgment after finalization creates a new Assertion.
_Avoid_: implying that the asserter chooses an evidence cutoff; implying that final outcomes update when a source changes.

**Source Hierarchy** `[MVP-target]` — the Resolution Spec's deterministic ordering of authoritative sources and its rule for conflicts and source unavailability. A one-source hierarchy is valid. In a multi-source hierarchy, lower-ranked sources control only as the spec explicitly permits; a material conflict or availability gap not settled by the hierarchy produces `Unresolvable`.
_Avoid_: treating listed sources as an unordered menu; choosing whichever source supports a preferred answer.

**Rubric-relative truth** `[MVP-target]`
Opal's core stance: the same statement text can resolve differently across assertions because each carries its own spec. "True" means "true under this assertion's fine print," which the asserter declares as the source of truth for their use case — not a universal fact.
_Avoid_: "absolute truth", "ground truth".

**Final Outcome** — `outcome: u8` `[Built]`
The terminal result on the assertion; `OUTCOME_NONE` (255) until `state == Resolved`. Consumers should ignore it unless `state == Resolved`.

**Resolver Unavailable** `[MVP-target]` — an exceptional terminal operational status used only when the trusted LLM resolver fails to post within 24 hours after the first dispute is accepted. It is not a fifth Resolution Outcome and does not express truth under the Resolution Spec. Once eligible, anyone may invoke the deterministic timeout finalizer; it atomically returns the full 500 USDC Participant Bonds to the asserter and LLM disputer with no Bond Fee, slashing, or reward. The caller cannot choose an outcome or redirect either refund. Normal resolution produces one of `True`, `False`, `Unresolvable`, or `NoConsensus`; public documentation presents resolver unavailability as an edge case rather than a normal result. The exact `99.9%` expectation is not presented as a measured availability guarantee unless supported by production telemetry. See [ADR-0012](adr/0012-resolver-unavailable.md).
_Avoid_: listing `ResolverUnavailable` beside the four truth outcomes; mapping it to `Unresolvable` or `NoConsensus`; implying that an operational failure judged the statement.

**Permanent Protocol Record** — an Assertion, dispute, resolution, or public payout-status record that remains readable indefinitely and cannot be closed for rent recovery. Empty vaults and transient private infrastructure may close only after every balance, lock, and payout liability reaches zero. See [ADR-0011](adr/0011-permanent-public-records.md).

**Rent Refund Recipient** — the immutable creation payer recorded for a closable vault or transient account. When that account safely closes, its reclaimed lamports return to the original payer. If Opal or an integrator sponsored creation, the sponsor receives the refund; it cannot be redirected to the participant or treasury.

**Permissionless Vault Cleanup** — once a protocol-owned vault has zero token balance, active locks, and payout liabilities, anyone may invoke its close instruction. The caller cannot choose the rent refund recipient or delete any permanent public record.

## States & outcomes

**Assertion State** — raw `u8` `[Built]`

- `Asserted` (0) — liveness; default `True`; first dispute allowed.
- `PendingLLM` (1) — first dispute filed; awaiting the LLM verdict.
- `AssertedLLM` (2) — LLM verdict posted; challenge window open.
- `PendingVote` (3) — LLM verdict challenged; vote round initializing immediately. This is a transient state, not a participant-facing window.
- `Voting` (4) — staked private vote active/settling.
- `Resolved` (5) — terminal; `outcome` set.
- `ResolverUnavailable` `[MVP-target]` — exceptional terminal operational status after the 24-hour Resolver Deadline; `outcome` remains `None`. Its raw representation is fixed during implementation and is not exposed by the Preview Integration API.

**Resolution Outcome** — raw `u8`

- `True` (0) `[Built]` — verified under the spec.
- `False` (1) `[Built]` — contradicted under the spec.
- `NoConsensus` (2) `[MVP-target]` — vote-only outcome used when the Vote Quorum is not met or, after quorum is met, no voting option reaches the 67% supermajority. This includes a Vote Round with no Votes. Settles no-fault: no collateral is slashed, ordinary fees still apply, and no rewards are paid.
- `Unresolvable` (3) `[MVP-target]` — an affirmative finding that the statement cannot be decided under its Resolution Spec. The asserter is incorrect; an `Unresolvable` voting option must itself reach 67% to win voter settlement.
- `TooEarly` — removed from the target model; its legacy code value `2` is reassigned to `NoConsensus` before Devnet deployment.
- `None` (255) — sentinel for unset.

`ResolverUnavailable` is a terminal lifecycle status, not an outcome code. It preserves `outcome = None` because the protocol did not determine rubric-relative truth.

## Participation windows

**Assertion Dispute Window** — the seven days after an assertion is created during which participants can dispute its default `True` position.
_Avoid_: "initial liveness window" in participant-facing documentation.

**LLM Dispute Window** — the seven days after the LLM posts its verdict during which participants can dispute that verdict and escalate the assertion to voting.
_Avoid_: "LLM liveness window"; "LLM challenge window" in participant-facing documentation.

**Resolver Deadline** `[MVP-target]` — the 24-hour protocol deadline beginning when the first dispute is accepted. The trusted resolver must post its verdict before this deadline; otherwise the Assertion becomes eligible for the exceptional `ResolverUnavailable` terminal path. This is not one of the three participant-facing dispute or voting windows.
_Avoid_: calling this another dispute window; implying that a participant action extends it.

**Vote Initialization** `[MVP-target]` — the automatic, untimed transition after an LLM verdict is disputed. The third Participant Bond is added to the Bond Vault, all three bonds are moved/delegated into the deployment's Voting PER, and co-location is verified. This is a transient protocol state, not a participant window.
_Avoid_: "vote setup window"; implying that participants can vote before initialization succeeds.

**Voting PER** `[MVP-target]` — the single MagicBlock Private Ephemeral Rollup validator used by every Opal Vote Round and Private Voting Balance in one Devnet deployment. Its validator identity is fixed at deployment; clients discover its current RPC endpoint through the MagicBlock router. The Devnet protocol model assumes Vote Initialization completes successfully; it has no participant-facing timeout or abort outcome.
_Avoid_: selecting a different PER per Vote Round; hard-coding a regional ER RPC URL.

**Voting Window** — the seven days during which participants can cast private, USDC-staked votes. It begins only after Vote Initialization succeeds; there is no separate participant-facing setup window.

## Participants

**Asserter** — `asserter` `[Built]` — posts an assertion, its spec, and the USDC bond. The asserter normally pays Solana transaction and account-creation costs; an integrating application may sponsor them. Opal charges no separate upfront assertion fee.
**Disputer** — challenges the current answer by posting a USDC bond.
**LLM Disputer** `[Built]` — the first disputer; challenges the default `True` and triggers LLM resolution.
**Vote Disputer** `[Built]` — the second disputer; challenges the LLM verdict and triggers the staked vote.
**Dispute Race** — when several wallets try to dispute the same answer, the first valid transaction accepted before the applicable Dispute Window deadline becomes the bonded disputer. Later competing or retried transactions fail without locking a Participant Bond or incurring an Opal Bond Fee; external execution costs may still apply.
**Bonded Role Separation** `[MVP-target]` — the Asserter, LLM Disputer, and Vote Disputer for one Assertion must be three distinct wallet addresses. One wallet cannot occupy more than one bonded role in the same Assertion. This prevents wallet-level self-challenges and manufactured escalation; it does not prove that the wallets belong to distinct people.
_Avoid_: “three distinct participants,” which claims a person-level identity guarantee Opal does not provide.
**Voter** `[MVP-target]` — anyone who stakes USDC into a specific vote round; weight is linear in stake; wrong-side stake is slashed, right-side stake earns rewards.
**LLM Resolver** `[MVP-target]` — a trusted off-chain service that calls one LLM and posts the verdict via `submit_llm_resolution` `[Built]`, gated on the dedicated `ProtocolConfig.resolver` key. (On-chain LLM provenance hashing is deferred to `[Vision]`.) See [ADR-0002](adr/0002-trusted-llm-resolver.md).
_Avoid_: "council" for the resolver — the 3-feed Switchboard council was removed per ADR-0002 and was never the resolver design.
**Integrator** — any app consuming Opal outcomes; must read the Resolution Spec to judge whether an outcome is meaningful for its use case, and should require `state == Resolved` before irreversible settlement.

**Finalized Integration Read** — the Solana account read an integrator uses before an irreversible downstream action. It must use `finalized` commitment and verify the exact Assertion account directly rather than relying only on a faster indexer view. `processed` may roll back, while `confirmed` has weaker finality than `finalized`. See the [Solana commitment reference](https://solana.com/docs/rpc#configuring-state-commitment).

**Permissionless Finalization** — after an applicable protocol deadline, anyone may submit a deterministic finalization transaction for an undisputed Assertion, an unchallenged LLM verdict, a completed Vote Round, or an eligible `ResolverUnavailable` edge case. A deadline expiring does not execute code automatically: the Assertion remains eligible but unfinalized until a transaction lands. The caller cannot choose the result or redirect payouts and normally pays the external network cost unless an integrator sponsors it.

**Preview Integration API** `[MVP-target]` — a documentation-only, client-shaped sketch of how a future client will wrap Opal's Solana program and Anchor IDL: read accounts, derive addresses, and construct wallet-signed transactions. It is not an HTTP service, deployed API, runnable SDK, or second protocol surface. Its non-runnable examples may change before a client is stabilized; the program and IDL remain authoritative. See [ADR-0020](adr/0020-integer-amounts-across-client-boundaries.md).
_Avoid_: "preliminary API"; "Opal SDK" until a runnable SDK exists; implying that the preview is deployed or independently authoritative.

**Statement Validation Result** `[MVP-target]` — the non-throwing Preview Integration API result for ordinary statement-input validation. It contains `ok`, the UTF-8 byte length, and every applicable stable issue code; each code appears at most once. Ill-formed JavaScript strings report a null byte length and only `ILL_FORMED_UNICODE`, because encoding them would first replace invalid code units. Assertion creation refuses to request a wallet signature unless validation succeeds. The onchain instruction separately enforces the same protocol acceptance rules; Preview issue codes do not promise matching Anchor error numbers. See [ADR-0021](adr/0021-readable-statement-validation.md).
_Avoid_: using `string.length` as the byte count; throwing for ordinary invalid form input; parsing English or raw numeric program errors to identify a validation issue; treating client validation as an onchain security boundary.

**Integration Operation Error** `[MVP-target]` — a failed Preview Integration API action represented by a stable semantic string `code`, a stable `nextAction`, and an optional raw diagnostic `cause`. Integrations branch on the stable fields and never on Anchor numeric codes, program log text, RPC wording, or wallet-adapter messages. Observable transfer and Assertion statuses are returned by reads rather than converted into exceptions. See [ADR-0022](adr/0022-stable-integration-errors-and-domain-statuses.md).
_Avoid_: treating `Pending`, `RecoveryNeeded`, or `ResolverUnavailable` as thrown errors; guessing a known code from an ambiguous raw failure; presenting internal numeric error ordering as a public compatibility contract; sending an unredacted raw cause to telemetry by default.

**Conditionally Idempotent Action** `[MVP-target]` — Preview-layer reconciliation after a known duplicate rejection or ambiguous submission result. The client reads the authoritative record and returns success with `alreadyApplied: true` only when every immutable dispute, Vote, or claim field matches the request. A mismatch is a stable conflict requiring refreshed state; a missing record preserves the failure. Onchain instructions remain strictly duplicate-rejecting and never move funds twice. See [ADR-0022](adr/0022-stable-integration-errors-and-domain-statuses.md).
_Avoid_: blindly resubmitting after an unknown RPC result; treating a competing dispute or different Vote as success; describing the underlying program instruction itself as idempotent.

**Insufficient Unlocked Balance Error** `[MVP-target]` — `INSUFFICIENT_UNLOCKED_BALANCE` with `nextAction: "REVIEW_BALANCE"`. The error and its default diagnostic cause contain no requested, unlocked, locked, or total Private Voting Balance amounts. An authenticated wallet view retrieves and displays those values separately and locally; default error telemetry does not receive them. See [ADR-0022](adr/0022-stable-integration-errors-and-domain-statuses.md).
_Avoid_: embedding private amounts or intended Vote stake in ordinary errors, logs, or analytics events.

**Protocol Amount Representation** `[MVP-target]` — every onchain USDC amount is a `u64` count of atomic units. TypeScript client logic uses `bigint`, with Anchor `BN` permitted only at the generated-binding boundary; it never passes an amount through a JavaScript `number`. JSON-shaped Preview Integration API inputs and outputs use unsigned base-10 integer strings in atomic units, so 500 USDC is `500000000n` in TypeScript and `"500000000"` in JSON. Human-unit text such as `500 USDC` is display formatting, not a protocol value. See [ADR-0020](adr/0020-integer-amounts-across-client-boundaries.md).
_Avoid_: floating-point amounts; JSON numbers for USDC; human-unit decimal strings at the protocol boundary.

**Prediction-Market Invalid Result** `[MVP-target]` — the canonical demo's market-level handling of `Unresolvable`, `NoConsensus`, or `ResolverUnavailable`: choose neither YES nor NO, preserve the exact reason, and execute the market's documented void/refund path. This shared market behavior does not make the three Opal cases equivalent: their fault, fee, and settlement rules remain distinct. See [ADR-0019](adr/0019-prediction-market-invalid-result-mapping.md).
_Avoid_: mapping an invalid result to YES or NO; calling `ResolverUnavailable` an outcome; implying that trader refunds reproduce or reimburse Opal participant economics.

## Economics

**USDC** `[MVP-target]` — the single protocol asset, used for all bonds, voting stake, rewards, slashing, and treasury fees. The mint is set when a deployment is initialized (currently `pusd_mint` on-chain; renaming to `usdc_mint` per ADR-0004) so localnet/devnet can use a test mint. See [ADR-0004](adr/0004-single-asset-usdc.md).
_Avoid_: calling the asset "pusd" (it is USDC — `pusd` is only the legacy on-chain field prefix, renaming per ADR-0004); "any USD-pegged stablecoin" (we commit to USDC); "OPAL" (dropped from the MVP).

**Deployment Parameters** — protocol values fixed when a network deployment is initialized. Participant documentation treats the Devnet values as constants; a later Mainnet deployment may use different values.
_Avoid_: "configurable parameters" when describing choices available to participants.

**Devnet Admin Authority** `[MVP-target]` — the one Opal-team-controlled single-signature wallet used by the official Devnet deployment as program upgrade authority, `ProtocolConfig.authority`, and owner of the configured Treasury token account. It has no multisig or timelock. The trusted resolver remains a separate role-limited signer. Exact addresses are published only after the completed deployment is officially designated. See [ADR-0018](adr/0018-devnet-single-admin-and-treasury-authority.md).
_Avoid_: implying that Devnet is immutable, governed by a DAO, protected by a multisig, or representative of a future Mainnet authority policy.

**Treasury** — the configured USDC token account that receives all Bond Fees and Voting Fees. On official Devnet it is owned by the Devnet Admin Authority, which can transfer its balance at any time under the SPL Token program; Opal imposes no withdrawal cap, timelock, or spending policy. Treasury movements are public on Solana.

**Participant Bond** `[MVP-target]` — 500 USDC collateral posted on Devnet by an asserter or either disputer. Every assertion and dispute uses the same bond; the current on-chain ratio fields are legacy implementation and are removed for the Devnet MVP.
_Avoid_: separate "assertion bond" and "dispute bond" amounts; ratio-based dispute bonds.

**Slashing** `[MVP-target]` — loss of bond or staked vote weight for being on the wrong side of a finalized dispute or vote.

**Schelling-point vote** `[MVP-target]` — the staked vote is a coordination game on the truth: losing-side voters are slashed and winning-side voters are paid from the losing side, so the equilibrium is to vote the spec's honest answer. Security comes from this slashing, not from any weight curve. See [ADR-0003](adr/0003-private-staked-voting.md).

**Bond Fee** `[MVP-target]` — the non-refundable 1% fee withheld from every 500 USDC Participant Bond at normal settlement. Each unslashed bond therefore refunds 495 USDC, including when the assertion resolves `NoConsensus`. The exceptional `ResolverUnavailable` path charges no Bond Fee because Opal failed to complete the resolution service; both bonds refund in full.

**Voting Fee** `[MVP-target]` — the non-refundable 0.1% fee withheld from voting stake at settlement. It is applied after tallying and does not reduce the stake used for Vote Quorum, voting weight, or Supermajority calculations.
**Voting Fee Rounding** `[MVP-target]` — when 0.1% of a Vote cannot be represented in USDC's smallest unit, the fee rounds down to the nearest micro-USDC. The participant keeps the remainder, so the effective fee never exceeds 0.1%. Any USDC amount of at least 1 USDC remains a valid stake; stakes are not restricted to whole-USDC or 0.001-USDC increments.

**Winner Takes Remaining Bond** `[MVP-target]` — the settlement rule when a dispute resolves before voting: after Bond Fees are withheld, the single correct bonded participant receives their own 495 USDC refund plus the incorrect participant's remaining 495 USDC bond. Opal retains 10 USDC in total fees.
_Avoid_: "winner takes all," which incorrectly implies that fees are included in the reward.

**Slashed Pool** `[MVP-target]` — at vote-stage settlement, the remaining collateral from incorrect Participant Bonds and losing voting stake after their respective fees are withheld.

**Reward Weight** `[MVP-target]` — a correct participant's original capital at risk: 500 units for each correct Participant Bond, or one unit per USDC of winning voting stake. The Slashed Pool is distributed pro rata across all Reward Weight, without role-specific percentages or an additional treasury share.

**No-fault settlement** `[MVP-target]` — when a vote resolves `NoConsensus`, no bond or voting stake is slashed. Ordinary Bond Fees and Voting Fees still apply, so fee-adjusted principal becomes claimable and no rewards are paid. See [ADR-0007](adr/0007-unresolvable-vs-no-consensus.md).
_Avoid_: "fee-free"; using no-fault for `Unresolvable`.

**Pro-rata vote settlement** `[MVP-target]` — after fees and principal refunds, the Slashed Pool is distributed to all correct bonded participants and winning voters in proportion to their Reward Weight.
_Avoid_: fixed reward shares by participant role; an additional treasury share from the Slashed Pool.

## Voting `[MVP-target]`

**Vote Round** — `VoteResolutionRound` — the per-dispute staked vote that produces the final outcome when an LLM verdict is challenged.
**Vote** — one immutable stake placement by a wallet in a Vote Round. Once cast, its outcome and stake cannot change or increase, and the stake remains locked through finalization; any positive payout then remains in the Vote Settlement Vault until claimed. A Vote counts only when its transaction is accepted and recorded on the Voting PER before the Voting Window deadline; signing or submitting earlier is insufficient if the transaction lands after the deadline. The first accepted submission for a wallet-and-round pair wins; later retries or competing submissions fail without changing the Vote, locking additional stake, or incurring an Opal Voting Fee. External execution costs may still apply to a failed transaction. Opal does not claim to identify one person across multiple wallets.
**Concurrent Voting** `[MVP-target]` — one wallet may hold one Vote in each of several Vote Rounds at the same time, provided its unlocked Private Voting Balance covers every new stake. The one-Vote rule is scoped to a wallet-and-round pair, and each round locks its stake independently.
**Bonded Participant Voting** `[MVP-target]` — the Asserter, LLM Disputer, and Vote Disputer may each cast their wallet's one Vote in the same Vote Round. Holding a Participant Bond does not consume or disqualify that Vote, and the wallet may select any voting outcome even when it conflicts with its bonded position. Bond capital and voting stake are classified and settled independently: each correct position contributes Reward Weight, and each incorrect position is slashed.
**Private Voting Balance** — a reusable, wallet-scoped USDC balance pre-funded through MagicBlock's private-payment/eSPL flow before a vote is cast. Each deposit must be at least 1 USDC. Opal locks stake from this balance inside the PER; only unlocked funds are available for another Vote Round or withdrawal. Active voting stake cannot be withdrawn. A finalized payout remains in the Vote Settlement Vault and is not part of the available balance until claimed. The balance, positions, and claims remain bound to the original wallet; Opal provides no admin recovery or destination override for a lost key. The private balance and stake movement are hidden while voting is open, but the base-layer deposit and withdrawal transactions are public and may leak timing or amount correlations.
_Avoid_: describing a Private Voting Balance as an anonymous deposit, a per-vote deposit, or an Opal-issued token; implying Opal can recover a lost wallet key.
**Private Voting Balance Closure** — the wallet-authorized removal of its reusable balance account after its token balance, active locks, and outstanding claims are all zero. No administrator or third party can close it on the wallet's behalf. Any reclaimed lamports return to the balance's original creation payer.
**Private Balance Withdrawal** — a wallet-authorized transfer of any positive amount up to the wallet's unlocked Private Voting Balance back to its public Solana USDC account. Opal imposes no minimum withdrawal and charges no withdrawal fee. Active voting stake and unclaimed payouts are not part of the withdrawable balance. External network or provider execution costs may apply.
**Private Balance Deposit** — a public Solana USDC transfer into a wallet's reusable Private Voting Balance through the MagicBlock private-payment/eSPL flow. Each deposit must be at least 1 USDC; several deposits add to the same balance. Opal charges no deposit fee, although external network or provider execution costs may apply.
**Private Balance Transfer Status** — the participant-facing lifecycle for a deposit or withdrawal: `Pending` while cross-runtime delivery is unresolved, `Completed` only after the destination balance confirms, and `Recovery Needed` when the normal path did not complete and the recovery flow must be used. Source-transaction confirmation alone does not make funds available or prove a withdrawal completed.
**Sponsored Private Balance Initialization** — during Vote Initialization, Opal creates and pays the setup costs for any missing Private Voting Balance belonging to the asserter or either disputer. This adds no participant fee and gives Opal no authority to view or withdraw the recipient's funds; the wallet retains that authority.
_Avoid_: "Opal wallet"; implying that sponsorship makes the balance custodial.
**Vote-stage Payout Claim** `[MVP-target]` — the wallet-authorized action that transfers a positive finalized vote-stage refund or reward from the Vote Settlement Vault into the wallet's canonical Private Voting Balance. The destination is derived and validated by the protocol; a claimant cannot redirect the payout to another private balance or a public token account. As soon as a Vote Round's finalization confirms, a positive payout is `Claimable` without a cooldown or separate opening transaction; finalization does not credit the balance automatically. The claim amount is fixed permanently at finalization and earns no interest, yield, bonus, or time-based adjustment while unclaimed. Once the claim confirms, its USDC is immediately unlocked for another Vote or withdrawal. A fully slashed position is terminal `Slashed` and has nothing to claim. Opal charges no additional claim fee; the claimant pays any network execution cost unless an integrating application sponsors it. Withdrawal to a public Solana wallet is a separate action after claiming.
_Avoid_: saying a participant has been paid merely because the Vote Round finalized; offering a zero-value claim for a `Slashed` position; allowing a caller-supplied claim destination; conflating a payout claim with a public-wallet withdrawal.
**Public Payout Status** `[MVP-target]` — after Vote Round finalization, every bond and Vote position is assigned an exact payout amount and one of `Claimable`, `Claimed`, or `Slashed`. Each Vote status becomes publicly readable through bounded position publication; the full public set is not complete until `PositionPublicationComplete`. Publishing this status does not reveal the wallet's total Private Voting Balance.
**Public Vote Position** — `VotePositionAccount` `[MVP-target]` — the permanent public record for one wallet's Vote in one Vote Round, uniquely keyed by the round and wallet. It exposes the selected outcome, Gross Voting Stake, exact finalized payout, and `Claimable`, `Claimed`, or `Slashed` status, but not the wallet's total Private Voting Balance. The voter normally funds its non-recoverable Solana account-creation balance; an integration may sponsor it, but Opal does not guarantee sponsorship. An unused preparation that never produces an accepted Vote is safely closable with its lamports returned to the original payer. The round holds fixed-size aggregates and publication counters, never a growable position vector. See [ADR-0017](adr/0017-per-voter-public-position-accounts.md).
**Position Publication Complete** `[MVP-target]` — the Vote Round marker proving that bounded post-finalization publication has produced every permanent Public Vote Position. Before this marker is set, the Assertion's truth outcome may already be final but the public voter mapping is incomplete. An integrator enumerating or auditing positions must wait for this marker; an integrator consuming only the finalized Assertion outcome need not. Publication is permissionless and never expires: any caller may publish still-missing valid records, cannot alter their contents or double-count them, and normally pays the external transaction cost unless an integration sponsors it.
**Vote Settlement Vault** — the private, PER-resident escrow that co-locates all three Participant Bonds and every locked voting stake for one Vote Round. Fees, principal refunds, the Slashed Pool, and rewards are calculated from this single vault. Claimable vote-stage payouts remain there until their recipients claim them into Private Voting Balances.
_Avoid_: confusing the Vote Settlement Vault (all collateral before settlement) with the Slashed Pool (only losing collateral remaining after fees).
**Linear weight** — 1 staked USDC = 1 vote. Sybil-neutral; whale dominance is deterred by slashing, not by a curve.
**Voting stake limits** — a vote requires at least 1 USDC and has no maximum. Wallet-level maximums are avoided because stake can be split across wallets.
**Private vote (MagicBlock)** — voters pre-fund a Private Voting Balance, then Opal locks stake and seals the vote inside a MagicBlock Private Ephemeral Rollup. While voting is open, private balances, per-outcome totals, wallet choices, and stake amounts are hidden, preventing the bandwagon/beauty-contest collapse a public tally would cause. After settlement, aggregate totals and each wallet's selected outcome and stake are public. See [ADR-0003](adr/0003-private-staked-voting.md).
_Avoid_: implying that an MVP vote remains anonymous or permanently private after settlement.
**Supermajority** — the inclusive 67% weighted threshold (`supermajority_bps = 6700`) that `True`, `False`, or `Unresolvable` must reach on Devnet after Vote Quorum is met. Exactly 67.000% wins. Implementations compare cross-products in integer units—`outcome_stake × 10,000 >= total_gross_stake × 6,700`—instead of rounding a divided percentage. If no option reaches it, the Vote resolves `NoConsensus`.
**Gross Voting Stake** `[MVP-target]` — the original USDC amount locked by a Vote before the Voting Fee. It supplies the Vote's linear weight and is used for the stake quorum, Supermajority, and Reward Weight calculations.
_Avoid_: using fee-adjusted stake in a tally or quorum calculation.
**Vote Quorum** `[MVP-target]` — the minimum participation required before the Supermajority is evaluated. Devnet requires both at least 3,000 USDC in total Gross Voting Stake and at least 10 distinct voting wallet addresses. Every wallet with a valid Vote counts once, including the Asserter, LLM Disputer, and Vote Disputer when they Vote. If all three do, at least seven other wallets are still required. If either quorum condition fails, the Vote Round resolves `NoConsensus`. The wallet count is an on-chain address count, not proof of 10 distinct people.
_Avoid_: “10 voters” when it could imply proof of personhood; treating the stake and wallet thresholds as alternatives rather than two required conditions.

## Vision (post-MVP) `[Vision]`

Recorded so the direction is clear and nobody mistakes these for current behaviour:

- **OPAL token** — governance, future reputation/staking, voter incentives. Dropped from the MVP (USDC + the authority key replace it).
- **Trust-minimized / permissionless LLM** — Switchboard On-Demand feed(s) or TEE-attested inference, replacing the trusted resolver. (The 3-feed Switchboard "council" was removed per ADR-0002.)
- **Proof-of-personhood** — to enable sub-linear/quadratic weighting without Sybil collapse.
- **Stake-duration reputation** — long-term staking that accrues voter weight/reputation.
- **Timed resolution** — a possible future protocol-level lifecycle field preventing premature resolution. It would not be content or an asserter-selected cutoff inside the immutable Resolution Spec.
- **TWAV (time-weighted voting)** — considered and rejected; recorded so it isn't reintroduced.
- **Nosana inference**; **on-chain commit-reveal** (the alternative to MagicBlock for private voting).
