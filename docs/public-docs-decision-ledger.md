# Opal Public Docs Decision Ledger

Last reviewed: 2026-08-11

This file consolidates product decisions made while defining the future public documentation at `docs.opalhq.xyz`. It is a review aid and handoff source, not the public documentation itself. Internal status badges in the other source documents remain authoritative for what is implemented today.

## Publication and audience

- The public documentation will live in its own Mintlify repository and deploy at `docs.opalhq.xyz`.
- It is a standalone protocol-learning site and does not depend on, embed, or assume access to the Opal application UI.
- Opal has no official Devnet program deployment while the MVP is incomplete. Existing development or test deployments are non-official and their program and authority addresses must not appear in public documentation.
- Publish deployment addresses only after the team designates the completed MVP deployment as official. The resulting deployment reference must identify the cluster, verified program address, upgradeability status, and authority policy.
- The official Devnet program remains upgradeable under an Opal-team-controlled single-signature authority so critical fixes can ship quickly while the MVP hardens.
- The Devnet deployment reference publishes the current upgrade-authority address and states that there is no timelock: the team can change the program without an advance onchain delay. It must not imply that verified code is immutable.
- The same Opal-team single-signature wallet is the official Devnet program upgrade authority, `ProtocolConfig.authority`, and owner of the configured USDC treasury token account. The trusted resolver remains a separate role-limited operational signer.
- Devnet has no admin or treasury multisig, timelock, withdrawal cap, withdrawal schedule, or program-mediated treasury spending policy. The wallet can move accumulated fee USDC at any time under the SPL Token program.
- The deployment reference discloses this centralized trust and publishes the exact admin-authority, treasury token-account, and treasury-owner addresses only after the completed deployment is officially designated. Treasury transfers remain publicly observable.
- The Mainnet upgrade-authority model is intentionally undecided and does not block the Devnet public documentation. The Devnet site must not promise a future Mainnet multisig, timelock, or immutable program before that policy is separately decided.
- Public documentation is written as though the Devnet MVP is live. It explains current Devnet behavior directly rather than exposing internal `[Built]` / `[MVP-target]` implementation gaps.
- The MagicBlock product documentation is the writing and information-design reference: concise overview pages, clear conceptual progression, focused guides, callouts, and direct navigation.
- Participant documentation explains what someone must know to assert, dispute, vote, claim, and withdraw safely. It avoids protocol internals unless they change participant behavior.
- Developer documentation explains how to integrate Opal. Architecture is included only where it helps an integrator use the protocol correctly; deep internals remain in this repository's `docs/` directory.
- Opal is presented as a universal optimistic oracle. A prediction market is the principal integration example, not the protocol's sole use case.
- There is no public SDK yet. The docs will contain a complete prediction-market demo written against a **Preview Integration API**: a documentation-only client-shaped sketch over the Solana program and Anchor IDL. It illustrates account reads, address derivation, and wallet-signed transaction construction; it is not an HTTP service, deployed API, runnable SDK, or independently authoritative protocol surface.
- Devnet parameters are presented as fixed facts. A notice may say Mainnet values can differ when Mainnet launches; participants are not told that they can configure protocol parameters.

## Assertion identity and permanence

- An Assertion contains a statement, a Resolution Spec, and a 500 USDC Participant Bond.
- A statement is single-line UTF-8 occupying 1–280 encoded bytes. It must not begin or end with Unicode whitespace and must not contain control characters, line/paragraph separators, or invisible bidirectional controls. Ordinary right-to-left letters remain valid.
- Statement length is measured in UTF-8 bytes, not characters. There is no arbitrary 20-character minimum; the program enforces structural validity but cannot prove grammatical or semantic quality.
- The statement bytes occupy the start of the fixed 280-byte onchain field and any unused suffix is zero-filled. A statement using all 280 bytes is valid without a trailing terminator. Both creating clients and `create_assertion` enforce the structural rules. See [ADR-0021](adr/0021-readable-statement-validation.md).
- After validation, clients and the program preserve the submitted UTF-8 bytes exactly. They do not normalize Unicode, trim, change case, or otherwise rewrite the statement. A signing interface displays the exact string it submits.
- JavaScript and TypeScript clients reject ill-formed strings before encoding and use `TextEncoder` byte length, never JavaScript `string.length`, for the 280-byte limit. Byte counting, signing preview, and Anchor transaction construction all use the same unchanged validated string. Account readers use fatal UTF-8 decoding so corrupt bytes fail instead of displaying replacement characters.
- Preview `validateStatement` reports ordinary input failure as structured data rather than throwing: `ok`, `utf8ByteLength`, and every applicable stable issue code. The codes are `ILL_FORMED_UNICODE`, `EMPTY`, `TOO_LONG`, `LEADING_WHITESPACE`, `TRAILING_WHITESPACE`, `CONTROL_CHARACTER`, `LINE_OR_PARAGRAPH_SEPARATOR`, and `BIDI_CONTROL_CHARACTER`; each appears at most once. Ill-formed JavaScript strings report a null byte length and only `ILL_FORMED_UNICODE`. `createAssertion` refuses to request a wallet signature when validation is not successful.
- The onchain instruction independently enforces the same statement acceptance rules so a custom client cannot bypass them. Invalid UTF-8 cannot deserialize as the Rust `String` instruction argument; the handler validates byte length and structure. Preview issue codes are stable client meanings, not promises about Anchor's numeric error codes.
- Statement validation uses one protocol-pinned character table in both clients and the program, not JavaScript `trim()` or runtime-dependent Unicode predicates as the source of truth. Control ranges `U+0000–U+001F` and `U+007F–U+009F`, line/paragraph separators `U+2028–U+2029`, and Unicode `Bidi_Control` characters are invalid anywhere. The first and last code point additionally cannot be in the Unicode `White_Space` set enumerated by ADR-0021 or be the invisible byte-order-mark character `U+FEFF`. Shared JavaScript/Rust conformance vectors cover every forbidden range and boundary case; changing the table is an explicit protocol change.
- Visually identical Unicode sequences may therefore remain byte-distinct. This is accepted because Assertions are identified by `assertionId`, not statement bytes or rendered text.
- The Resolution Spec is the source of truth for how the statement is judged. Opal resolves rubric-relative truth rather than absolute truth.
- Resolution Spec content is immutable and time-independent. It defines authoritative sources and their priority, key definitions, and ambiguity handling, but cannot impose an asserter-selected evidence cutoff, freeze a source version, exclude later corrections, or instruct the protocol to wait until a chosen date.
- Every Resolution Spec orders its authoritative sources and defines deterministic conflict and source-unavailability rules. A one-source spec satisfies this requirement automatically.
- A multi-source spec must state which source controls when current information conflicts and whether a lower-ranked source may substitute when a preferred source is unavailable. If its declared rules leave a material conflict or availability gap unresolved, the Assertion resolves `Unresolvable`; the resolver and voters cannot invent an unstated preference.
- Creating clients and integration services reject a multi-source spec whose ordering or conflict/unavailability behavior is missing or ambiguous. The onchain program cannot inspect this off-chain content, so a malformed spec created through a bypassing client remains possible and resolves `Unresolvable` when applied.
- Evidence timing is protocol-wide: every resolution action applies the unchanged spec to the latest relevant information available from its authoritative sources when that action occurs.
- A source correction while the Assertion is non-final may affect a later resolution layer. During voting it can influence wallets that have not yet voted, but cannot change an already accepted immutable Vote.
- Once the Assertion reaches a terminal truth outcome, later corrections never reopen or mutate it. An integrator needing a new judgment creates and references a new Assertion.
- Before signing Assertion creation, the creating client or integration service retrieves the uploaded Arweave object, verifies that it matches the committed identifier, and refuses to sign when retrieval or integrity verification fails. The Solana program cannot perform this network check.
- The asserter normally uploads the Resolution Spec and pays the external Arweave storage cost; an integrating application may sponsor it. Opal charges no storage fee.
- An existing upload may be reused without a duplicate storage payment only after the creating client retrieves it and verifies its bytes against the supplied immutable identifier.
- The canonical identifier is exactly `ar://<id>`, where `<id>` is a 43-character unpadded base64url Arweave transaction or data-item ID. It contains no gateway host, ArNS name, query, or fragment.
- The Preview Integration API exposes `resolutionSpecUri`, never a free-form `auxiliaryHash`. Both creating clients and the onchain `create_assertion` instruction require exactly the lowercase ASCII prefix `ar://` followed by 43 characters from the unpadded base64url alphabet. The target program stores the URI verbatim as `resolution_spec_uri: [u8; 48]`, with no variable length or padding.
- The current internal `auxiliary_hash: [u8; 128]` field is renamed and narrowed to `resolution_spec_uri: [u8; 48]` before the official Devnet deployment. A future identifier width requires an explicit versioned account change rather than consuming reserved bytes.
- Creating clients fail closed unless Arweave signature/data-root verification proves the retrieved bytes match the known ID. No separate raw content digest is required by the protocol.
- Resolution Specs use UTF-8 JSON validated against a published, versioned JSON Schema, not free-form Markdown. Devnet uses `schemaVersion: "1"`; future schema versions cannot reinterpret an existing v1 spec.
- V1 requires explicit definitions, a non-empty ordered source list in which each source declares unavailability behavior, `conflictBehavior: "highest-priority-wins"`, and non-empty ambiguity handling. Optional `rationale` text is human-readable and non-normative.
- Unknown fields are invalid. Creating clients also reject the cross-field case where the final source uses `next-source`.
- The exact UTF-8 JSON is limited to 16 KiB (16,384 bytes), checked before upload and again after retrieval before signing creation. This is an Opal application-level resource bound, not an Arweave protocol limit, upload tier, or pricing threshold.
- The creating client validates the exact JSON bytes before upload, then retrieves and revalidates those same bytes before signing Assertion creation. The onchain program stores only the URI and cannot parse the spec.
- Public docs publish the schema, a rendered field reference, and complete valid and invalid examples. Devnet does not require canonical JSON serialization: semantically equivalent byte sequences may have different Arweave IDs.
- Once creation is accepted, the statement, Resolution Spec URI, bond, and deadlines cannot be edited.
- An accepted Assertion cannot be cancelled, and its bond cannot be withdrawn early.
- Correcting a mistake requires a new Assertion; the original continues independently to resolution.
- Identical statement and Resolution Spec content may appear in several Assertions. Opal does not designate a latest or canonical replacement.
- `assertionId` is the sufficient public reference within the configured deployment. Public integrations do not need to construct or persist the underlying Assertion account address.
- The client generates a fresh random keypair for each Assertion ID. Its public key becomes `assertionId`, and the temporary keypair signs creation to prevent identifier front-running.
- The Assertion ID key grants no authority after creation. The client retains it only until creation confirms, then may discard it.
- Assertion IDs are unique and never reused within a deployment.
- The asserter normally pays Solana transaction and account-creation costs in addition to locking the bond. An integrator may sponsor network costs.
- Opal charges no separate upfront assertion fee. The Bond Fee is withheld only at settlement.

## Resolution lifecycle and terminology

- Assertions begin with the optimistic default answer `True`.
- The **Assertion Dispute Window** lasts seven days.
- A first valid dispute locks another 500 USDC bond and triggers the trusted LLM resolver.
- The LLM may return `True`, `False`, or `Unresolvable`; it cannot return `NoConsensus`.
- The **LLM Dispute Window** lasts seven days after the LLM verdict is posted.
- A second valid dispute locks a third 500 USDC bond and triggers Vote Initialization.
- **Vote Initialization** is an automatic, untimed technical transition, not a participant-facing window. The Devnet model assumes it succeeds and defines no abort or timeout outcome.
- The **Voting Window** lasts seven days and begins only after Vote Initialization and PER co-location complete.
- The three participant-facing windows are therefore Assertion Dispute, LLM Dispute, and Voting; all last seven days.
- A final Assertion is immutable. Later corrections use new Assertions and never mutate a resolved record.
- Expiry of a dispute or voting window does not execute code automatically. Anyone may submit a deterministic finalization transaction after the applicable deadline; the caller cannot choose the result or redirect settlement and normally pays the external network cost unless an integrator sponsors it.
- Until a finalization transaction lands, the Assertion remains in its eligible pre-final state. Integrators must not infer a terminal result from the clock alone.
- A trusted-resolver outage has an exceptional terminal `ResolverUnavailable` path so validly posted bonds cannot remain locked forever.
- The Resolver Deadline is 24 hours from acceptance of the first dispute. If no verdict has posted by then, the Assertion becomes eligible for `ResolverUnavailable`.
- `ResolverUnavailable` is an operational status, not a fifth Resolution Outcome. It keeps the outcome unset because no truth judgment occurred under the Resolution Spec.
- Anyone may invoke the deterministic timeout finalizer at or after the Resolver Deadline. The caller cannot choose an outcome or redirect funds and pays the external network cost unless an integrator sponsors it.
- Timeout finalization atomically refunds the full 500 USDC bond to both the asserter and LLM disputer through their public Solana token accounts. No Bond Fee, slashing, or reward applies; external transaction and account-creation costs are not reimbursed.
- Resolver verdicts are accepted only before the deadline and rejected afterward.
- Normal resolution produces one of `True`, `False`, `Unresolvable`, or `NoConsensus`. Public documentation frames resolver unavailability as an edge case; it does not publish `99.9%` as a measured guarantee unless production telemetry supports that exact claim.

## Outcomes

- `True`: the statement is verified under its Resolution Spec.
- `False`: the statement is contradicted under its Resolution Spec.
- `Unresolvable`: an affirmative finding that the statement cannot be decided under its Resolution Spec. It is fault-assigned and treats the asserter as incorrect.
- `NoConsensus`: a vote-only, no-fault outcome when quorum is not met or no eligible option reaches the supermajority.
- `NoConsensus` and `Unresolvable` are distinct even if an integrating prediction market chooses to void on either.
- A Vote Round with no Votes resolves `NoConsensus`.
- The canonical prediction-market demo maps `True` to YES winning and `False` to NO winning.
- It treats `Unresolvable`, `NoConsensus`, and `ResolverUnavailable` as invalid market results and executes one documented trader void/refund path while preserving the exact invalid reason in state, API responses, interface copy, and history.
- The shared market action does not merge their Opal meanings or economics: `Unresolvable` is fault-assigned with ordinary fees and slashing; `NoConsensus` is no-fault with ordinary fees; `ResolverUnavailable` is an operational failure with full fee-free bond refunds.
- Market refunds do not reimburse Opal participation fees or external transaction costs.

## Bonded roles and dispute races

- The Asserter, LLM Disputer, and Vote Disputer each post exactly 500 USDC on Devnet.
- The three bonded roles must be held by three distinct wallet addresses in one Assertion. This is a wallet-level constraint, not proof of three people.
- The first valid dispute transaction accepted before the applicable deadline becomes the bonded disputer.
- Competing or retried dispute transactions that land later fail without locking a bond or incurring an Opal Bond Fee. External execution costs may still apply.
- Any of the three bonded wallets may also vote in the final Vote Round.
- A bonded wallet may vote for any eligible outcome, even one that conflicts with the outcome favored by its bond.
- A wallet's bonded position and Vote position are separate and settle independently.

## Fees

- Every accepted Participant Bond pays a non-refundable 1% Bond Fee at normal settlement.
- `ResolverUnavailable` is the sole exception: both bonds refund in full because Opal failed to complete the resolution service.
- A 500 USDC bond therefore has 495 USDC remaining after its fee.
- The Bond Fee applies on every Resolution Outcome, including a correct undisputed Assertion and `NoConsensus`; `ResolverUnavailable` is an operational status rather than an outcome and follows its fee-free refund rule.
- Every accepted Vote position pays a non-refundable 0.1% Voting Fee at settlement.
- Voting Fee calculation rounds down to the nearest USDC atomic unit, so it never exceeds 0.1%.
- A failed or rejected Vote transaction creates no Vote position and incurs no Opal Voting Fee. External execution costs may still apply.
- Opal charges no protocol fee on Private Voting Balance deposits, payout claims, or withdrawals.
- External Solana, MagicBlock, or integration-provider execution costs are separate from Opal's fees.

## Pre-vote settlement

- An undisputed Assertion resolves `True`; its asserter receives 495 USDC after the Bond Fee.
- If an LLM verdict is not challenged, the correct bonded side receives its own remaining 495 USDC plus the incorrect side's remaining 495 USDC.
- This is **Winner Takes Remaining Bond**: a total payout of 990 USDC and 10 USDC in Bond Fees.
- Pre-vote settlement has a fixed recipient set and pays automatically to public Solana token accounts.

## Voting participation

- Voting uses USDC with linear weight: 1 originally staked USDC equals 1 vote.
- A Vote must stake at least 1 USDC. There is no maximum stake.
- Each wallet may cast exactly one immutable Vote per Vote Round.
- The selected outcome and stake cannot be changed, increased, cancelled, or withdrawn after acceptance.
- The one-Vote rule applies per wallet per round, not globally.
- A wallet may vote in several Vote Rounds concurrently if its unlocked Private Voting Balance covers every independently locked stake.
- A Vote counts only when accepted and recorded on the Voting PER before the Voting Window deadline. Signing or submitting before the deadline does not reserve it.
- The first accepted Vote for a wallet-and-round pair is authoritative. Later retries or competing submissions fail without changing it or locking more stake.

## Vote quorum and outcome

- Devnet Vote Quorum requires both at least 3,000 USDC in total Gross Voting Stake and at least 10 distinct voting wallet addresses.
- The three bonded wallets count toward the 10-wallet requirement when they Vote.
- Gross Voting Stake is the original locked stake before the Voting Fee.
- Quorum, voting weight, Reward Weight, and the supermajority calculation all use Gross Voting Stake.
- After quorum is met, `True`, `False`, or `Unresolvable` must reach an inclusive 67% of Gross Voting Stake.
- Exactly 67.000% is sufficient.
- If either quorum condition fails, or no eligible outcome reaches 67%, the result is `NoConsensus`.

## Vote-stage economics

- Before voting, all three bonds and all voting stake are co-located in one PER-resident Vote Settlement Vault.
- Each correct bonded position contributes 500 units of Reward Weight.
- Each winning Vote contributes Reward Weight equal to its original Gross Voting Stake.
- Incorrect bonds and losing voting stake, after ordinary fees, form one Slashed Pool.
- Correct bonded participants and winning voters receive fee-adjusted principal plus a pro-rata share of the Slashed Pool according to Reward Weight.
- Opal takes no additional treasury share from the Slashed Pool.
- `Unresolvable` settles like any other winning, fault-assigned vote outcome.
- `NoConsensus` slashes nobody, pays no rewards, and makes every bond and Vote stake claimable minus ordinary fees.

## Private Voting Balance

- Voting stake comes from a reusable, wallet-scoped Private Voting Balance on the deployment-wide MagicBlock Voting PER.
- Each deposit must be at least 1 USDC. Multiple deposits add to the same balance.
- Opal imposes no withdrawal minimum; any positive unlocked amount may be withdrawn.
- Only unlocked balance may fund a Vote or withdrawal.
- Active Vote stakes cannot be withdrawn.
- A finalized but unclaimed payout remains in the Vote Settlement Vault and is not part of available private balance.
- Once a payout claim confirms, the credited USDC is immediately unlocked for another Vote or withdrawal. There is no post-claim cooldown.
- A wallet may close its reusable Private Voting Balance only with its own signature and only when its balance, active locks, and outstanding claims are all zero.
- The balance, Vote positions, and claims remain bound to the original wallet. Opal has no admin recovery or destination override for a lost key.

## Deposit and withdrawal lifecycle

- Deposits and withdrawals are separate from voting.
- Opal charges no deposit or withdrawal fee; external execution costs may apply.
- Participant-facing cross-runtime status is `Pending`, `Completed`, or `Recovery Needed`.
- A transfer is `Completed` only after the destination balance confirms.
- Source-transaction confirmation alone never makes deposited funds usable or proves a withdrawal completed.
- Deposits and withdrawals are publicly visible on Solana and can leak timing or amount correlations.

## Vote privacy

- During the Voting Window, individual choices, stake amounts, private balances, and per-outcome totals are private inside the MagicBlock PER.
- The private tally prevents voters from following a visible leader.
- Finalization makes aggregate stake per outcome public immediately. Every wallet's selected outcome and stake become public through bounded post-finalization position publication.
- Each position's exact payout amount and `Claimable`, `Claimed`, or `Slashed` status becomes public with that position record.
- Every revealed Vote uses its own permanent public `VotePositionAccount`, uniquely keyed by the Vote Round and voter wallet. The round stores fixed-size aggregates, its immutable position count, publication progress, and a completion marker; it never stores a growable vector of voter positions.
- Every voter normally funds the SOL account-creation balance for its own permanent Vote position, including a bonded wallet that also Votes. An integration may sponsor this external cost, but Opal does not guarantee sponsorship.
- A setup that never produces an accepted Vote does not become a permanent record and can close safely with its lamports returned to the original payer. Once accepted, the position and its account-creation balance are permanent and non-recoverable.
- Preparing the base-layer position may reveal the wallet's intention to participate during the Voting Window, but not its selected outcome or stake.
- The truth outcome and payout formula finalize without enumerating every voter. Deterministic bounded batches publish position records afterward, and the public mapping is incomplete until `PositionPublicationComplete` is set.
- Position publication is permissionless and never expires. Any caller may publish still-missing valid records; the protocol derives their contents from authenticated finalized state and rejects alteration or duplicate progress.
- The publication caller normally pays the external transaction cost unless an integration sponsors it. Opal charges no publication fee, guarantees no sponsor, and pays no caller reward.
- Full-list consumers and auditors wait for position-publication completion. Integrations consuming only the final Assertion outcome do not; `Resolved` is independent of publication progress.
- A bonded position and Vote position remain separate records and entitlements when one wallet owns both.
- A wallet's total Private Voting Balance remains private.
- The MVP does not promise permanent voter anonymity.
- One deployment-wide Voting PER is fixed at deployment. Clients resolve its current endpoint through the MagicBlock router rather than choosing a validator per round or hard-coding a regional endpoint.

## Vote-stage payout claims

- Vote finalization freezes the outcome, fee totals, reward inputs, and exact payout entitlement for every position.
- Finalization does not pay an unbounded voter set in one transaction.
- A positive payout becomes `Claimable` immediately when finalization confirms; there is no cooldown or claim-opening transaction.
- A fully lost position becomes terminal `Slashed` and has no zero-value claim.
- The claimant must sign a claim transaction.
- A claim can only credit the original wallet's canonical Private Voting Balance. It cannot redirect to another private balance or a public token account.
- Claims never expire. Unclaimed USDC remains reserved for its owner in the Vote Settlement Vault.
- Claim amounts never change or accrue while unclaimed.
- Bond and Vote positions use separate claim records. If one wallet owns both, a client may bundle both claim instructions into one transaction and signature.
- Opal charges no claim fee. The claimant pays external execution costs unless an integrator sponsors them.
- `Resolved` means the truth outcome and payout formula are final; it does not mean every payout has been claimed.
- Repeating a successful claim cannot pay twice.

## Public records and cleanup

- Assertion, dispute, resolution, and public payout-status records remain readable indefinitely and cannot be closed for rent recovery.
- Their account-creation payer treats the storage cost as non-recoverable.
- Empty protocol-owned vaults and transient private accounts may close only after balance, lock, and liability checks reach zero.
- A Vote Settlement Vault cannot close while any non-expiring payout remains unclaimed.
- Every closable account records its original creation payer as its immutable Rent Refund Recipient.
- Reclaimed lamports return to the original payer. If Opal or an integrator sponsored creation, the sponsor receives the refund.
- Anyone may close an eligible protocol-owned vault; the caller cannot redirect rent or delete public history.
- Closing a Private Voting Balance requires the wallet's signature.

## Integration safety

- Irreversible downstream settlement must read the exact Assertion account directly at Solana `finalized` commitment and require its normal terminal `Resolved` state. A `processed`, `confirmed`, or indexer-only observation is insufficient.
- Indexers may support discovery and responsive UI, but they are not the source of truth for irreversible actions.
- The Preview Integration API separates observable states from action failures. Private-balance transfer `Pending` and `RecoveryNeeded` values and the terminal Assertion status `ResolverUnavailable` are returned by reads; they are not thrown errors.
- An action that cannot proceed rejects with a typed integration error containing a stable semantic string `code`, a stable `nextAction`, and an optional raw `cause` for diagnostics. Integrations branch only on the stable fields, never Anchor numeric codes, log text, RPC wording, or wallet-adapter messages. Unknown failures remain explicitly unknown rather than being guessed into a known semantic code. See [ADR-0022](adr/0022-stable-integration-errors-and-domain-statuses.md).
- The Preview layer conditionally reconciles a known duplicate rejection or ambiguous submission result. It returns success with `alreadyApplied: true` only after an authoritative record matches every immutable request field: dispute transition and wallet, Vote round/wallet/outcome/Gross Voting Stake, or claim position/owner/fixed payout/derived destination. A non-matching existing record is a conflict with `nextAction: "REFRESH_STATE"`; a missing record preserves the original failure. The onchain instructions still reject all duplicates before moving additional funds.
- The stable action mappings are `STATE_CONFLICT → REFRESH_STATE`, `DEADLINE_PASSED → REFRESH_STATE`, and `INSUFFICIENT_UNLOCKED_BALANCE → REVIEW_BALANCE`. A matching duplicate is reconciled success rather than an error; transfer and resolver statuses remain read data.
- `INSUFFICIENT_UNLOCKED_BALANCE` and its default diagnostic cause disclose no requested, unlocked, locked, or total Private Voting Balance amounts. A wallet-authenticated interface may retrieve and render those values locally through a separate private balance read, but typed errors and default telemetry omit them.
- Every onchain USDC amount is an unsigned `u64` count of atomic units.
- TypeScript client logic uses `bigint`; generated Anchor bindings may use `BN` only at their boundary, with exact conversion that never passes through a JavaScript `number`.
- JSON-shaped Preview Integration API inputs and outputs serialize amounts as unsigned base-10 integer strings in atomic units. They do not use JSON numbers, decimal points, exponents, grouping separators, signs, or unit suffixes.
- With USDC's six decimal places, 500 USDC is `500000000n` in TypeScript and `"500000000"` in JSON. Human-unit text such as `500 USDC` is display formatting only.
- Preview clients reject malformed amount strings and values outside the `u64` range. See [ADR-0020](adr/0020-integer-amounts-across-client-boundaries.md).

## Implementation boundaries the public docs must hide

- The current repository contains implementation gaps relative to this Devnet target, including the MagicBlock private vote, vote positions and staking vault, claim delivery, fixed bond/fee configuration, quorum logic, and signed Assertion IDs.
- Internal source documentation must continue marking those gaps accurately.
- The separate public Mintlify site presents the agreed Devnet MVP behavior as live and must not copy internal status warnings into participant or integration pages.
