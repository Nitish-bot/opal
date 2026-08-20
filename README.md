# Opal

A Solana-native optimistic oracle for natural-language statements.

Opal treats assertions as true by default. If nobody disputes within a liveness window, the assertion resolves `True`. If disputed, it escalates through an LLM resolution round and (if challenged) a staked private vote.

Opal resolves **rubric-relative truth**, not absolute truth: every assertion ships its own Resolution Spec — the asserter's rubric for _how_ the statement should be judged — and Opal applies that spec rather than adjudicating any universal reality. The same statement text can resolve differently across assertions. See [ADR-0001](docs/adr/0001-rubric-relative-truth.md).

Target use case: prediction-market resolution. Statements like "Kanye West's Delhi concert got postponed" can't be answered by price feeds or APIs — they need a purpose-built oracle, and they need someone to declare _by what standard_ "postponed" is judged.

> **Status:** this README describes Opal's full **target design**. Feature status is tracked with badges — `[Built]` is implemented in the program today, `[MVP-target]` is committed but not yet built, `[Vision]` is post-MVP. Sections below are the design; see [Current State](#current-state) for exactly what's live.

## How It Works

1. **Assert** — someone posts a statement, a USDC bond, and a Resolution Spec (the rubric; stored off-chain on Arweave, its canonical `ar://` URI onchain)
2. **Wait** — liveness window where anyone can dispute
3. **Undisputed** — if no dispute, resolves `True`
4. **Disputed** — the first dispute triggers on-chain LLM resolution: a single trusted off-chain resolver posts the verdict via the resolver-gated `submit_llm_resolution` instruction `[Built]`; the off-chain service that makes the LLM call is `[MVP-target]` (the former 3-feed Switchboard council was removed per [ADR-0002](docs/adr/0002-trusted-llm-resolver.md))
5. **Challenged** — if the LLM verdict is challenged, escalate to a per-dispute staked vote, kept private during the voting window via a MagicBlock ephemeral rollup ([ADR-0003](docs/adr/0003-private-staked-voting.md))
6. **Resolved** — the final outcome is posted on-chain

The LLM layer is deliberately trusted, not trustless: a wrong verdict is challengeable into the staked vote, which is the real trust backstop.

## Resolution Outcomes

- `True` — verified under the spec
- `False` — contradicted under the spec
- `Unresolvable` — an affirmative finding that the statement cannot be decided under the spec. `[MVP-target]` The asserter is incorrect; in voting, `Unresolvable` must itself reach 67% and participates in ordinary slashing and rewards.
- `NoConsensus` — no voting option reached 67%. `[MVP-target]` Settles no-fault: every bond and voting stake is returned minus ordinary fees, nobody is slashed, and no rewards are paid. See [ADR-0007](docs/adr/0007-unresolvable-vs-no-consensus.md).

## Voting `[MVP-target]`

The final escalation is a private, per-dispute, USDC-staked vote (today `open_vote` sets up the round, but real staking, tallying, and MagicBlock privacy are not yet built):

- **Linear weight** — 1 staked USDC = 1 vote. Sybil-neutral; whale dominance is deterred by slashing, not by a weight curve.
- **Schelling-point slashing** — losing-side voters are slashed and winning-side voters are paid from the losing side, so the honest answer under the spec is the focal point.
- **Pre-funded** — voters deposit USDC into a reusable Private Voting Balance before casting; a vote cannot pull its stake directly from a public wallet balance.
- **One Voting PER** — every Devnet Vote Round and private balance uses the same deployment-wide MagicBlock validator, allowing balances to be reused across votes.
- **Co-located settlement** — before the seven-day Voting Window starts, all three Participant Bonds move into the same private rollup as voting stake. Opal creates and sponsors any missing private payout balances for the bonded participants. Every vote-stage refund and reward—including asserter and disputer payouts—must be claimed into a Private Voting Balance after finalization; withdrawing to the public Solana wallet is a separate manual action.
- **Private while open** — during the Voting Window, a MagicBlock PER hides private balances, choices, stakes, and per-outcome totals so voters cannot bandwagon. Deposits and withdrawals remain public on Solana. After settlement, aggregates and each wallet's selected outcome and stake are public.
- **Supermajority** — `True`, `False`, or `Unresolvable` must reach 67%, otherwise the vote resolves `NoConsensus`.

## Asset

USDC is the single protocol asset for all bonds, voting stake, rewards, slashing, and treasury fees. The mint is a config field (currently `pusd_mint` on-chain; renaming to `usdc_mint`) so localnet/devnet can use a test mint. See [ADR-0004](docs/adr/0004-single-asset-usdc.md).

## Quickstart

```bash
# Install deps
bun install

# Build the program
anchor build

# Run the logic suite on localnet
bun run test:local
```

**Requirements:** Solana CLI v3.1.12, Anchor 0.32.1, Bun, Rust 1.89.0

TypeScript integration tests use `@coral-xyz/anchor` with artifacts from `anchor build` (`target/idl/opal.json`, `target/types/opal.ts`).

### Web frontend

Privy social login + embedded Solana wallet (devnet). Assertion UI uses mock data until on-chain txs are wired:

```bash
cd web && cp .env.local.example .env.local && bun run check-env && bun run dev
```

See [web/README.md](web/README.md) and [Privy setup](web/docs/PRIVY_SETUP.md).

## Documentation

- [Web frontend](web/README.md) — Privy auth, devnet wallet, mock assertion flows
- [Privy setup](web/docs/PRIVY_SETUP.md) — dashboard checklist
- [Architecture](docs/architecture.md) — account model, state machine, instruction flow
- [Resolution](docs/resolution.md) — how statements move from assertion to final outcome
- [Tokenomics](docs/tokenomics.md) — bonds, slashing, rewards, fees
- [Glossary](docs/glossary.md) — shared vocabulary
- [Decision records](docs/adr/) — the rationale behind the locked design

## Testing

Tests are TypeScript integration tests run with `@coral-xyz/anchor`; there are no Rust unit tests. Targets split by environment ([ADR-0006](docs/adr/0006-codebase-organization.md)):

```bash
# [Built] Logic suite on a localnet validator
bun run test:local

# [MVP-target] Real integration against devnet, where the resolver service
# and MagicBlock ER actually exist: anchor test --skip-local-validator
anchor test
```

`bun run test:local` is an alias for `anchor test` today. Pointing `anchor test` at devnet — a funded devnet keypair, a deployed program, and `--skip-local-validator` — lands in the test-split follow-up PR per ADR-0006; until then it still spins up a local validator.

Current coverage: happy paths (undisputed, LLM resolution, full escalation), config validation, deadline violations, state guards, account mismatches, token balance assertions.

## Current State

The optimistic core is built: the account model, the six-state machine (`Asserted` → `PendingLLM` → `AssertedLLM` → `PendingVote` → `Voting` → `Resolved`), and the optimistic-resolution plus dispute plumbing all exist on-chain today. Integration tests drive the LLM round through `submit_llm_resolution` signed with a test resolver keypair.

These pieces are v1 targets, not yet shipped:

- **Trusted LLM resolver** — the on-chain half is built: `submit_llm_resolution` is gated on a dedicated `ProtocolConfig.resolver` key (separate from `authority`, so a leaked hot resolver key can only post a challengeable verdict) and accepts only `True`/`False`/`Unresolvable`. The off-chain service that makes the real LLM call and posts verdicts is the remaining target; the former 3-feed Switchboard council was removed per ADR-0002. On-chain LLM provenance (prompt/response/evidence hashing) is deferred to a `[Vision]` trust-minimized resolver.
- **Private staked voting** — opening a vote sets up the round, but real MagicBlock private voting (delegation, ER settlement) is the MVP target.
- **Resolution Spec on Arweave** — the onchain field exists as the legacy `auxiliary_hash: [u8; 128]`; canonical `ar://` validation, fail-closed verification, and the rename/narrowing to `resolution_spec_uri: [u8; 48]` are planned.
- **No Consensus settlement & pro-rata rewards** — `NoConsensus` no-fault settlement, equal 500 USDC Participant Bonds, per-bond/vote fees, and unified pro-rata vote rewards are planned; the current ratio, fee, and role-specific share fields are legacy implementation under the target model.
- **Field names** — state and config fields still carry the legacy `pusd` prefix; a later PR renames them to `usdc` to match the committed asset.

## Vision (post-MVP)

Directions recorded so they're not mistaken for current behavior:

- **OPAL token** — governance, reputation/staking, voter incentives. Dropped from the MVP; staked USDC and the `authority` keypair cover its jobs.
- **Trust-minimized LLM** — Switchboard On-Demand or TEE-attested (permissionless) inference replacing the trusted resolver; on-chain LLM provenance hashing, if any, lands here.
- **Proof-of-personhood** — enabling sub-linear/quadratic voting weight without Sybil collapse.
- **Stake-duration reputation** — long-term staking that accrues voter weight.
- **Timed resolution** — a possible future protocol-level lifecycle field preventing premature resolution; it would not be an asserter-selected cutoff inside the immutable Resolution Spec.

## Security

Not audited. Don't use in production.

## License

Not selected yet.
