# Opal architecture gap audit — 2026-08-14

## Question

Are the reported gaps in Opal's Devnet MVP architecture valid, are any material
gaps missing, and in what order should the open decisions be closed?

## Executive verdict

The reported review is directionally correct. Every substantive product gap is
real. Two claims need narrower wording:

1. MagicBlock's current documented authorization model is the blocker, not a
   general claim that all permissions in every version are identical. Its
   authorization guide says that a permission currently implies read access and
   that a read/write split may be added later. No documented capability was
   found that lets an arbitrary voter mutate a shared sensitive account without
   reading it.
2. `ephemeral-rollups-sdk = 0.8.0` is demonstrably incompatible with Opal's
   described current ER-local `EphemeralPermission` flow, but `0.14+` should not
   be stated as the proven minimum supported stack. `0.16.2` is the current
   published SDK and the official private-counter example is the reference flow
   to target.

The highest-risk unknown is whether MagicBlock can implement the required vote
privacy at all without adding a relay or a cryptographic ballot scheme. SDK
upgrading should follow, not precede, that feasibility decision.

## Verification of the reported gaps

### 1. Private-voting feasibility — confirmed, P0

Opal requires voters to write votes and locked stake while hiding individual
choices, amounts, and per-outcome totals from those voters
([architecture.md](../architecture.md), [tokenomics.md](../tokenomics.md)).
MagicBlock's authorization documentation says a permission currently implies
read access. The access-control flags govern authority and transaction metadata
visibility; they do not document a write-without-read role for raw account
state.

A shared tally or settlement-vault account therefore cannot be called private
from ordinary voters merely by adding every voter as a member. The design also
does not explain how an unbounded voter set fits into a permission member list.

Required proof before implementation:

- An arbitrary wallet can submit a vote without being granted read access to
  the tally, aggregate vault balance, other ballots, or sensitive transaction
  metadata.
- The same transaction atomically locks stake and records exactly one immutable
  ballot.
- A failed or duplicate submission cannot lock funds without a matching ballot.
- Membership/account sizes and transaction limits support the intended voter
  cardinality.
- The client verifies an application-approved TEE measurement, not merely that
  some TDX quote is structurally valid.

If the spike fails, the direct shared-tally architecture must be replaced by
either a disclosed trusted submission relay or a cryptographic ballot design
such as commit-reveal. The former is the smaller Devnet change but adds
censorship, privacy, and liveness trust.

### 2. Cross-runtime settlement — confirmed, P0

The docs say that public bonds are moved/delegated to a PER-resident Vote
Settlement Vault, private claims occur there, public positions are later
published, Voting Fees go to a public treasury, and the base Assertion becomes
terminal. They do not define an account placement matrix, commit group, or
finality boundary.

The missing contract must cover at least:

- `AssertionAccount`, disputes, the round, tally, ballots, private balances,
  both vaults, fee liability, claims, public positions, and treasury;
- base-only, delegated, and ER-only location for every account;
- authority and router/validator checks for every transition;
- which accounts commit together and which remain delegated for claims;
- how a PER terminal result authorizes a Solana Assertion update;
- how claim state and public position state stay consistent;
- how PER fee liability becomes a confirmed base-treasury receipt.

Recommended seam: keep assertions, disputes, treasury, and permanent public
receipts on Solana; keep ballots, live tallies, reusable private balances, and
the settlement vault on one PER. Commit-and-undelegate a dedicated
program-owned terminal outcome projection, then use a permissionless,
idempotent base instruction to finalize the Assertion from that projection.
Treat treasury sweeping and position publication as explicit asynchronous
workflows, not part of a fictional cross-runtime atomic transaction.

Magic Actions can be an optimization, but the protocol still needs idempotent
recovery. The official documentation says an action failure reverts that commit
attempt; base-layer fees, compute, account locks, and account lists remain
failure surfaces.

### 3. Failure recovery — confirmed, P0

The omission is explicit in [resolution.md](../resolution.md): Vote
Initialization is assumed to complete and has no timeout or abort outcome.
`Pending`/`Recovery Needed` labels do not define a recovery protocol.

Each cross-runtime transition needs:

- a durable operation ID and authoritative evidence;
- an idempotent retry instruction;
- a permissionless or named retry actor;
- a deadline and terminal fallback;
- an identified fee payer, budget, and exhaustion behavior;
- partial deposit, delegation, commit, undelegation, withdrawal, publication,
  and treasury-sweep handling.

The current MagicBlock guidance also identifies a default ten-commit sponsorship
allowance unless the application supplies a delegated payer and fee vault. A
deployment-wide, long-lived voting PER therefore needs an explicit funding and
top-up policy.

Recommended product addition: `VotingUnavailable`, distinct from
`NoConsensus`, after a bounded initialization or final-settlement SLA. It should
represent operational failure and provide fee-free recovery of every principal
that can be safely returned. Devnet admin-assisted recovery is acceptable only
if it is explicit and auditable; indefinite lock is not.

### 4. Pro-rata rounding — confirmed, P1

Independent per-recipient floors can leave residual atomic units:

```text
sum(floor(pool * weight_i / total_weight)) <= pool
```

The docs neither assign that residue nor explain how a settlement vault reaches
zero before closure. This conflicts with the statement that the treasury takes
no extra share.

Smallest MVP rule: floor each reward independently, assign the bounded residual
to the treasury, and permit sweeping only after all payout liabilities are
claimed or otherwise discharged. If zero treasury dust is non-negotiable, use a
deterministic ordered/telescoping allocation, but claims cannot all be final
until the bounded winner pass completes.

### 5. Resolver behavior — confirmed, P1

"One LLM call" is not an operational resolver specification. Missing items
include model and prompt-policy versions, deterministic source fetching,
redirect/size/type/timeout and SSRF controls, prompt-injection handling,
classification of source failure versus resolver failure, retry/idempotency,
and evidence retention.

Recommended policy:

- fetch and sanitize authoritative sources before inference;
- treat retrieved content as untrusted data, never instructions;
- pin the resolver-policy version when the LLM dispute starts;
- classify rubric-defined ambiguity/unavailability as `Unresolvable`;
- classify transport, provider, refusal, or malformed-output failures as
  retryable operational errors that can end in `ResolverUnavailable`;
- retain a signed, content-addressed, non-binding receipt containing policy and
  model versions, fetch timestamps/statuses/digests, verdict, and rationale.

### 6. Resolution Spec schema — confirmed, P1

`minLength: 1` accepts whitespace-only strings. The schema does not prevent
duplicate definitions or duplicate source identities, and `format: "uri"`
requires deliberate validator support under JSON Schema Draft 2020-12. Semantic
conflicts and raw duplicate JSON object keys also need checks outside ordinary
schema keywords.

Required hardening:

- non-whitespace normative strings;
- a duplicate-key-detecting JSON parser;
- unique normalized definition terms and source identities;
- explicit allowed schemes and a network-safe URL parser;
- configured format assertion, not an assumed validator default;
- shared conformance vectors for creators, integrators, and the resolver.

### 7. Document contradictions — confirmed, P1

- Truth consumers should require `hasTruthOutcome`, while the canonical market
  workflow should require `isTerminal` and void on `ResolverUnavailable`.
- `open_vote` authorization remains explicitly undecided.
- ADR-0002 still describes removed Switchboard code.
- ADR-0006 still describes the removed mock feature and stale test topology.
- Some short `NoConsensus` definitions omit failed quorum.
- `Recovery Needed` and `RecoveryNeeded` are inconsistent.

Recommended predicates:

```text
is_terminal = state == Resolved || state == ResolverUnavailable
has_truth_outcome = state == Resolved
```

Make `open_vote` permissionless only after all initialization and co-location
preconditions are deterministic and verifiable.

### 8. Implementation gap — confirmed, with version qualification

The repo pins SDK `0.8.0`, and the dependency is unused by the program. The
program has no real delegation, PER permission lifecycle, private ballot,
private token lock, tally, claim, bounded publication, resolver timeout, or
cross-runtime finalization implementation. `open_vote` only flips a flag, and
the vote-finalization placeholder accepts caller-supplied resolution data.

The current published SDK is `0.16.2`; for Anchor 0.32 the published feature
matrix points to `anchor-compat`, while `anchor` targets Anchor 1.x. The official
private-counter example creates `EphemeralPermission` on the ER after delegating
the protected data account. This establishes a current target, not a proven
minimum version. Do not retain the unverified phrase "requires v0.14+."

### 9. Test baseline — confirmed, with cause qualification

The local baseline reproduced 17 tests: 12 passed and 5 deadline-related tests
failed. The failures are consistent with tests using short JavaScript sleeps
while the local Surfpool runtime does not advance `Clock` as those tests assume:
three expected finalizers still saw an open deadline, and two supposedly late
transactions were accepted.

This is not evidence that the deadline predicates themselves are wrong. The
explicit toolchain warning is Anchor library `0.32.1` versus Anchor CLI `1.1.2`.
Installed Solana `3.1.13` and Rust `1.89` match the repo pins. Formatting,
`git diff --check`, and JSON syntax checks passed.

## Additional material gaps found independently

### Privacy and trust contract

The docs do not state privacy against whom: public RPC users, other voters,
Opal's Devnet administrator, the MagicBlock operator/host, or the TEE program.
Upgrade authority also matters: an upgradeable program cannot promise privacy
from its own administrator. Define the threat model and approved TEE
measurement/upgrade policy before selecting accounts.

### eSPL model and atomic vote lock

"Private Payments/eSPL" conflates separate surfaces. The hosted Payments API
returns unsigned deposit/transfer/withdraw transactions and cannot be assumed to
be Opal's accounting layer. Opal must choose one eSPL account/custody model and
prove that its program can atomically debit a voter's delegated token balance,
credit program-controlled settlement custody, and persist the ballot in one PER
transaction.

### Fee aggregation

The Voting Fee is floored per accepted vote, so gross outcome aggregates cannot
reconstruct exact fee totals:

```text
sum(floor(stake_i * 10 / 10_000))
    != floor(sum(stake_i) * 10 / 10_000) in general
```

Calculate the fee at cast time, store it with the private position, and maintain
`fee_total_by_outcome` (or an equivalent exact aggregate).

### Reward-weight units

The prose uses a bond weight of `500` while the protocol requires integer USDC
atomic units. The canonical weight must be `500_000_000` for a 500 USDC bond;
vote weight is `gross_stake_atomic`.

### Publication authenticity and completeness

The docs describe bounded public position publication but do not define the
cryptographic or account-level proof that prevents omission or alteration of
private positions. A frozen count is insufficient. Use committed
program-owned per-position accounts or a terminal Merkle commitment with
inclusion/index rules and an explicit completeness marker.

### Resolver key lifecycle

The resolver is a hot key, but `ProtocolConfig.resolver` has no rotation or
revocation path; the current initializer notes that a bad key permanently
bricks resolution. Devnet needs authority-gated immediate rotation, an audit
event/record, and a decision about pending disputes.

### Security budget and liveness incentives

Fixed 500 USDC bonds and a 3,000 USDC voting quorum do not bound downstream
value secured. A low-turnout challenge can also force the no-fault invalid path,
while voters still pay a fee if quorum fails. Define a Devnet exposure cap or
security tier, and describe the observer/indexer assumption required to notice
bad assertions and resolver verdicts before their deadlines.

### Deployment-wide PER failure domain

A single long-lived Voting PER puts all private balances and vote rounds behind
one availability, operator, funding, and upgrade domain. Define validator/TEE
rotation, outage behavior, migration, monitoring, and recovery before calling it
a durable custody layer.

## Recommended closure order

1. Define the privacy adversary and acceptable Devnet trust.
2. Run the write-without-read and atomic token-lock feasibility spike.
3. Select the exact eSPL integration model.
4. Freeze the base/PER account and authority matrix.
5. Define lifecycle states, idempotency, deadlines, and `VotingUnavailable`.
6. Choose rounding, fee aggregation, and reward units.
7. Specify resolver and Resolution Spec validation policy.
8. Reconcile docs and only then upgrade/build against the selected SDK stack.

## Primary sources

- [MagicBlock Authorization](https://docs.magicblock.gg/pages/private-ephemeral-rollups-pers/introduction/authorization)
- [MagicBlock Access Control](https://docs.magicblock.gg/pages/private-ephemeral-rollups-pers/how-to-guide/access-control)
- [Official private-counter Anchor example](https://github.com/magicblock-labs/magicblock-engine-examples/tree/main/private-counter/anchor)
- [Magic Actions overview](https://docs.magicblock.gg/pages/ephemeral-rollups-ers/magic-actions/overview)
- [Magic Actions troubleshooting](https://docs.magicblock.gg/pages/ephemeral-rollups-ers/magic-actions/troubleshooting)
- [Private Payments API introduction](https://docs.magicblock.gg/pages/private-ephemeral-rollups-pers/api-reference/per/introduction)
- [Private Payments transfer API](https://docs.magicblock.gg/api-reference/per-api/transferAmount)
- [`ephemeral-rollups-sdk` 0.16.2](https://docs.rs/crate/ephemeral-rollups-sdk/0.16.2)
- [`ephemeral-rollups-sdk` 0.16.2 feature flags](https://docs.rs/crate/ephemeral-rollups-sdk/0.16.2/features)
- [JSON Schema Draft 2020-12 validation](https://json-schema.org/draft/2020-12/json-schema-validation)

## Local evidence

- [Architecture](../architecture.md)
- [Resolution](../resolution.md)
- [Tokenomics](../tokenomics.md)
- [Resolution Spec v1 schema](../schemas/resolution-spec-v1.schema.json)
- [MagicBlock voting ADR](../adr/0003-private-staked-voting.md)
- [Payout-delivery research](payout-delivery-model.md)
- [`programs/opal/Cargo.toml`](../../programs/opal/Cargo.toml)
- [`open_vote.rs`](../../programs/opal/src/instructions/open_vote.rs)
- [`initialize_protocol_config.rs`](../../programs/opal/src/instructions/initialize_protocol_config.rs)
