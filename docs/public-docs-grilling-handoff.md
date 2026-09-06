# Handoff Prompt: Continue Opal Public-Docs Grilling

Copy the prompt below into a fresh LLM session opened in the Opal repository.

```text
You are continuing a product-definition and documentation-grilling session for Opal, a Solana optimistic oracle for natural-language statements.

Start by reading these files completely:

1. AGENTS.md
2. docs/public-docs-decision-ledger.md
3. docs/glossary.md
4. docs/resolution.md
5. docs/tokenomics.md
6. docs/architecture.md
7. docs/adr/0001-rubric-relative-truth.md through the latest ADR
8. docs/research/payout-delivery-model.md
9. docs/research/unicode-statement-handling.md

Use the grilling skill. If a question concerns MagicBlock or Solana behavior, read the applicable MagicBlock and Solana skills and primary documentation before asking. Facts must be researched from the repository or primary sources; do not ask the user questions that can be answered by inspection.

Goal:

- Finish resolving only the genuinely non-obvious product, protocol, integration, security, and participant-experience decisions needed for a complete public documentation site.
- The eventual public docs will be a separate Mintlify repository deployed at docs.opalhq.xyz.
- The public site is standalone and cannot depend on the Opal UI.
- It will describe the Devnet MVP as live.
- Participant pages explain behavior, risks, quirks, timing, costs, outcomes, claims, and recovery without teaching internal architecture.
- Developer pages explain integration, with only enough architecture to integrate correctly.
- Opal is universal; a complete prediction market built against a non-runnable Preview Integration API is the main example. Do not imply that an SDK exists.
- The MagicBlock product docs are the writing and information-design standard.

Interview rules:

- Ask exactly one question at a time and wait for the user's decision.
- Every question must include your recommended answer and the concrete consequence or failure mode it addresses.
- Ask only questions that materially change security, economics, finality, integration correctness, data availability, or participant expectations.
- Do not ask obvious questions, cosmetic preference questions, or questions already settled in docs/public-docs-decision-ledger.md.
- When a proposed answer creates a hidden edge case, explain that edge case before asking for confirmation.
- Maintain the distinction between a fact to research and a decision for the user.
- After the user confirms a decision, update the internal source-of-truth docs and docs/public-docs-decision-ledger.md, run `bun run format` and `git diff --check`, then ask the next question.
- Preserve unrelated working-tree changes. Do not implement program code and do not create the separate Mintlify repository until the user explicitly ends the grilling phase or asks you to build it.
- Do not reopen settled choices merely because you prefer a different design.
- Do not introduce public documentation about low-level arithmetic reconciliation that has no participant action or material consequence.

Decisions already settled:

- Treat docs/public-docs-decision-ledger.md as authoritative. Do not re-ask anything listed there.
- Opal has no official Devnet deployment while the MVP is incomplete. Do not publish or treat any development program or authority address as official; deployment identifiers are added only after the team designates the completed MVP deployment.
- At official Devnet launch, the program remains upgradeable under a disclosed Opal-team-controlled single-signature authority with no timelock guarantee. The deployment reference publishes the then-current authority address and does not imply immutability.
- That same Devnet single-signature wallet is the program upgrade authority, `ProtocolConfig.authority`, and owner of the configured USDC treasury token account; the resolver remains a separate operational signer. Devnet has no admin or treasury multisig, timelock, withdrawal cap, or program-mediated spending policy. The wallet can move fees at any time. Exact admin and treasury addresses are published only after the completed deployment is officially designated. See ADR-0018.
- The Mainnet upgrade-authority model is intentionally deferred and does not block Devnet documentation. Do not ask for or promise a Mainnet multisig, timelock, or immutable program during this grilling phase.
- In particular, do not revisit the fixed 500 USDC bonds, 1% Bond Fee, 0.1% Voting Fee, three seven-day windows, 67% inclusive supermajority, dual 3,000-USDC/10-wallet quorum, one immutable Vote per wallet per round, NoConsensus versus Unresolvable, claim-based vote payouts, private-balance rules, post-finalization public vote mapping, or permanent public records.
- Vote Initialization is assumed to complete on Devnet and has no abort or participant-facing timeout path.
- A stalled trusted resolver becomes eligible for the exceptional `ResolverUnavailable` operational status 24 hours after the first dispute is accepted rather than leaving bonds locked indefinitely. This is not a fifth truth outcome: normal resolution produces `True`, `False`, `Unresolvable`, or `NoConsensus`, while resolver failure keeps the outcome unset. Anyone may invoke the deterministic timeout finalizer; it atomically returns both 500 USDC bonds in full through their owners' public token accounts, charges no Bond Fee, and cannot redirect funds or select an outcome. Late resolver verdicts are rejected. Public docs frame this as an edge case and do not present `99.9%` as a measured guarantee without supporting telemetry.
- Irreversible integration actions require a direct Assertion-account read at Solana `finalized` commitment and the normal terminal `Resolved` state. Indexers are discovery aids, not settlement authority.
- Deterministic deadline finalization is permissionless. Window expiry alone does not execute code; until someone submits and finalizes the transaction, the Assertion remains in its pre-final state. The caller cannot choose the result or redirect settlement and normally pays the external network cost.
- Before signing Assertion creation, the creating client or service must retrieve the uploaded Arweave object and verify it matches the committed identifier. The onchain program cannot perform this check.
- Resolution Specs are immutable and time-independent. They define sources and priority, definitions, and ambiguity handling, but cannot impose an asserter-selected evidence cutoff, freeze a source version, exclude later corrections, or delay resolution until a spec-selected date. Each resolution action uses the latest relevant authoritative information available when it acts. Corrections can affect later non-final layers but cannot rewrite accepted immutable Votes or reopen a terminal Assertion. See ADR-0013.
- Every Resolution Spec orders its authoritative sources and defines deterministic conflict and source-unavailability behavior. One source satisfies the requirement automatically. A multi-source spec must state precedence and whether a lower-ranked source substitutes when a preferred source is unavailable; unresolved material ambiguity produces `Unresolvable`, and actors cannot invent a preference. See ADR-0014.
- The asserter or sponsoring integrator uploads the Resolution Spec and pays Arweave directly; Opal charges no storage fee. A verified existing upload may be reused without duplicate storage payment.
- The canonical Resolution Spec identifier is exactly `ar://<id>`, where `<id>` is a 43-character unpadded base64url Arweave transaction or data-item ID. The Preview API exposes `resolutionSpecUri`; both creating clients and the onchain creation instruction enforce the exact 48-byte shape. The target program stores those bytes verbatim in `resolution_spec_uri: [u8; 48]`, without padding. The current `auxiliary_hash: [u8; 128]` field is renamed and narrowed before Devnet. Clients fail closed on Arweave signature/data-root verification; raw digests, gateway URLs, ArNS names, queries, and fragments are invalid. A future identifier width requires an explicit versioned account change. See ADR-0015.
- Resolution Specs use UTF-8 JSON validated against a published, versioned JSON Schema rather than free-form Markdown. Devnet v1 requires `schemaVersion: "1"`, definitions, one or more ordered sources with unavailability behavior, `conflictBehavior: "highest-priority-wins"`, and non-empty ambiguity handling. Optional `rationale` is non-normative, unknown fields are invalid, and the final source cannot use `next-source`. The exact UTF-8 JSON is capped at 16 KiB (16,384 bytes), an Opal application resource bound rather than an Arweave restriction or tier. Clients enforce the limit and validate before upload, then repeat both checks on the retrieved exact bytes before signing creation. See ADR-0016 and `docs/schemas/resolution-spec-v1.schema.json`.
- Every revealed Vote uses one permanent public `VotePositionAccount` keyed by Vote Round and wallet; the Vote Round keeps only fixed-size aggregates, the frozen position count, publication progress, and a completion marker. Outcome finalization does not enumerate the unbounded voter set. Deterministic bounded batches publish the public positions, so full-list consumers wait for `PositionPublicationComplete` while outcome-only integrations may rely on the already-resolved Assertion. Bond and Vote positions remain separate. See ADR-0017.
- Every voter normally funds the SOL account-creation balance for its own permanent Vote position, including a bonded wallet that also Votes. An integration may sponsor it, but Opal does not guarantee sponsorship. A preparation that never produces an accepted Vote is safely closable and refunds the original payer; acceptance makes the record and its account-creation cost permanent. Base-layer position setup may reveal the wallet's intention to participate during the Voting Window, but not its selected outcome or stake.
- Bounded post-finalization Vote-position publication is permissionless and never expires. Any caller may publish still-missing records but cannot alter their authenticated contents or advance the count twice. The caller normally pays the external transaction cost unless an integration sponsors it; Opal charges no publication fee, guarantees no sponsor, and pays no caller reward.
- The canonical prediction-market demo maps `True` to YES and `False` to NO. It treats `Unresolvable`, `NoConsensus`, and `ResolverUnavailable` as invalid market results and uses one documented trader void/refund path while preserving the exact reason. This does not merge Opal economics: `Unresolvable` is fault-assigned with ordinary fees and slashing, `NoConsensus` is no-fault with ordinary fees, and `ResolverUnavailable` is an operational failure with full fee-free bond refunds. External transaction costs are not refunded. See ADR-0019.
- The Preview Integration API is a documentation-only client-shaped sketch over the Solana program and Anchor IDL, not an HTTP service, deployed API, runnable SDK, or separate protocol surface. It illustrates account reads, address derivation, and wallet-signed transaction construction; the program and IDL remain authoritative.
- Onchain USDC amounts are unsigned `u64` atomic-unit counts. TypeScript client logic uses `bigint`, with Anchor `BN` confined to the generated-binding boundary and no conversion through JavaScript `number`. JSON-shaped preview inputs and outputs use unsigned base-10 integer strings in atomic units. Human-readable USDC decimals are display formatting only. See ADR-0020.
- Statements are single-line UTF-8 occupying 1–280 encoded bytes, with no surrounding Unicode whitespace, control characters, line/paragraph separators, or Unicode `Bidi_Control` characters. Ordinary right-to-left letters remain valid. The onchain creation instruction enforces the rules; length is measured in bytes, there is no arbitrary minimum character count, unused field storage is zero-filled, and a full 280-byte statement needs no terminator. After validation, clients and the program preserve submitted bytes exactly without Unicode normalization or other rewriting; visually identical sequences may remain byte-distinct, and integrations use `assertionId` rather than text as identity. JavaScript and TypeScript clients reject ill-formed strings, measure `TextEncoder` byte length rather than `string.length`, and use the same unchanged validated value for byte counting, signing preview, and Anchor transaction construction. Readers use fatal UTF-8 decoding. Preview `validateStatement` returns structured `ok`, `utf8ByteLength`, and stable issue codes including `BIDI_CONTROL_CHARACTER` rather than throwing for ordinary invalid input; an ill-formed string has a null byte length and only `ILL_FORMED_UNICODE`. `createAssertion` refuses to request a signature for an invalid result. The onchain handler independently enforces the same protocol rules, while the Preview codes remain independent of raw Anchor numeric errors. Clients and the program use the exact character table pinned in ADR-0021 plus shared conformance vectors; JavaScript `trim()` and language-runtime Unicode versions do not define protocol behavior. Interfaces isolate each statement as one plain-text display unit instead of accepting author-supplied direction controls. The program does not attempt to grade grammar or meaning. See ADR-0021 and `docs/research/unicode-statement-handling.md`.
- The Preview Integration API separates observable status from failed actions. Transfer `Pending` and `RecoveryNeeded` and terminal Assertion status `ResolverUnavailable` are returned by reads, not thrown. Failed actions reject with a typed integration error containing stable semantic `code` and `nextAction` fields plus an optional raw diagnostic `cause`. Integrations never branch on Anchor numeric codes, logs, RPC wording, or wallet-adapter messages, and unknown failures are not guessed into known codes. See ADR-0022.
- Duplicate actions are conditionally idempotent only in the Preview layer. After a duplicate rejection or ambiguous submission result, an authoritative read must exactly match the requested immutable dispute, Vote, or claim fields before returning ordinary success with `alreadyApplied: true`. A mismatch is a conflict with `nextAction: "REFRESH_STATE"`; a missing record preserves the failure. Onchain instructions always reject duplicates before moving additional funds. See ADR-0022.
- Stable action mappings are `STATE_CONFLICT → REFRESH_STATE`, `DEADLINE_PASSED → REFRESH_STATE`, and `INSUFFICIENT_UNLOCKED_BALANCE → REVIEW_BALANCE`. The insufficient-balance error and its default diagnostic cause contain no private amounts; an authenticated interface obtains them through a separate private balance read and does not send them through default telemetry. See ADR-0022.

High-value unresolved queue:

Ask these one at a time only after verifying that later repository changes have not already answered them. Reorder when dependencies require it.

No queued question remains. Audit the current source docs for a genuinely unresolved decision with a material security, economic, finality, integration-correctness, data-availability, or participant-expectation consequence before asking anything else. Do not turn implementation hygiene or obvious mappings into user questions. If no such decision remains, say so and offer the decision ledger for review rather than inventing more questions.

Do not dump this queue on the user. Start with the highest-impact unanswered item, ask one question with a recommendation, and wait.

At natural checkpoints, offer the user a link to docs/public-docs-decision-ledger.md for review. When the user says the grilling is complete, reconcile the ledger against glossary/resolution/tokenomics/architecture/ADRs, report contradictions, and only then propose or build the separate Mintlify information architecture.
```
