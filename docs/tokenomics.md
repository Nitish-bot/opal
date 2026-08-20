# Opal Tokenomics

Opal uses a **single asset: USDC** `[MVP-target]`. The same token is collateral for assertion bonds, dispute bonds, voting stake, slashing, rewards, and treasury fees. There is no second protocol token in the MVP — governance is the `authority` keypair, and voting weight is staked USDC, not a governance token. See [ADR-0004](adr/0004-single-asset-usdc.md).

> **Field-name note.** The program structs still use the legacy `pusd` prefix (`pusd_mint`, `assertion_bond_min_pusd`, `*_bond_amount_pusd`). These name a USDC amount; a later PR renames `*_pusd → *_usdc` and `pusd_mint → usdc_mint`. Treat every `pusd` field as USDC.

For the lifecycle and state machine these economics drive, see [architecture.md](architecture.md) and [resolution.md](resolution.md); the shared vocabulary lives in [glossary.md](glossary.md).

## Asset: USDC `[MVP-target]`

USDC is used for assertion bonds, the first (LLM) dispute bond, the second (vote) dispute bond, voting stake, slashed collateral, disputer and voter rewards, and protocol treasury fees. The mint is a config field (`pusd_mint`, to become `usdc_mint`) so localnet/devnet can run against a test mint, but the protocol commits to USDC — we do **not** support "any USD-pegged stablecoin." See [ADR-0004](adr/0004-single-asset-usdc.md).

## Bond Model `[MVP-target]`

### Equal Participant Bonds

The asserter and both possible disputers each post the same **500 USDC Participant Bond** on Devnet. Equal bonds give every claim the same economic weight, make incorrect participation costly, and avoid arbitrary escalation ratios. Once accepted, a bond cannot be reduced or withdrawn early; it remains locked until its resolution path settles or, after voting, produces a claimable payout.

These three bonded roles must be held by three distinct wallet addresses within an Assertion. The LLM Disputer cannot be the Asserter, and the Vote Disputer can be neither the Asserter nor the LLM Disputer. This blocks wallet-level self-challenges and manufactured escalation, but it does not claim that distinct wallets represent distinct people.

Each dispute role is first-valid-transaction-wins. When several eligible wallets compete, only the first dispute accepted before the applicable deadline creates the role and locks 500 USDC. Later competing or retried transactions fail without locking a bond or incurring an Opal Bond Fee, although external execution costs may still apply.

The asserter's posted amount is stored on the assertion as `assertion_bond_amount_pusd`; each dispute account stores the same amount in `bond_amount_pusd`. These fields are renamed from `pusd` to `usdc` per [ADR-0004](adr/0004-single-asset-usdc.md).

### First Dispute Bond (LLM)

The LLM disputer posts 500 USDC to challenge the default optimistic `True` resolution and trigger LLM resolution. The posted amount is stored as `LlmDisputeAccount.bond_amount_pusd`.

### Second Dispute Bond (Vote)

The vote disputer posts 500 USDC to challenge the LLM verdict and open the private staked vote. The posted amount is stored as `VoteDisputeAccount.bond_amount_pusd`.

> **Implementation mismatch.** The current program still derives dispute bonds through `llm_dispute_bond_ratio_bps` and `vote_dispute_bond_ratio_bps`. Those legacy ratio fields must be removed before the Devnet MVP described here is deployed.

## Participation Fees `[MVP-target]`

Every 500 USDC Participant Bond pays a non-refundable 1% **Bond Fee** at normal settlement:

```text
bond_fee = 500 USDC × 1% = 5 USDC
unslashed_bond_refund = 500 USDC - 5 USDC = 495 USDC
```

Voting stake pays a smaller non-refundable 0.1% **Voting Fee**. Stake may use any USDC amount of at least 1 USDC; it need not be a whole-USDC amount. Because USDC has six decimal places, some percentage results are not exactly representable. Opal rounds the Voting Fee down to the nearest atomic unit:

```text
voting_fee_atomic = floor(gross_vote_stake_atomic × 10 ÷ 10,000)
```

The participant keeps any fractional remainder, so the effective fee never exceeds 0.1%. The difference is always less than one micro-USDC. Both fees apply to every Resolution Outcome, including the no-fault `NoConsensus` outcome. No-fault means no slashing; it does not mean fee-free participation.

The exceptional `ResolverUnavailable` operational path is the sole Bond Fee exception. If Opal's trusted resolver fails to post within 24 hours after the first dispute is accepted, the permissionless timeout finalizer returns both 500 USDC bonds in full. Opal charges no fee, slashes no position, and pays no reward because it did not complete the resolution service. External transaction and account-creation costs remain unreimbursed. See [ADR-0012](adr/0012-resolver-unavailable.md).

Only an accepted Vote position pays the Voting Fee at settlement. A voter also normally funds the SOL account-creation balance for its permanent public `VotePositionAccount`; an integration may sponsor this external cost, but Opal does not guarantee sponsorship. Once the Vote is accepted, that account-creation balance is non-recoverable because the record remains permanent. A rejected or failed Vote creates no permanent position and incurs no Opal Voting Fee. Any unused preparatory position account follows a safe close path that refunds its original payer, although other external execution costs may still apply.

Opal charges no protocol fee on Private Voting Balance deposits, payout claims, or withdrawals. External network or provider execution costs may still apply and are separate from the Bond Fee and Voting Fee.

Creating an Assertion has no separate upfront Opal fee. The asserter normally pays Solana transaction and account-creation costs, pays the external Arweave cost to upload the Resolution Spec, and locks the 500 USDC Participant Bond; an integrating application may sponsor the network or storage costs. Opal charges no Resolution Spec storage fee, and a verified existing upload can be reused without a duplicate upload. Account-creation costs for permanent Assertion, dispute, resolution, and public payout-status records are not recoverable through account closure. The 1% Bond Fee is withheld only at normal settlement.

On official Devnet, Bond Fees and Voting Fees transfer directly to a USDC treasury token account owned by the same Opal-team-controlled single-signature wallet that controls program upgrades and `ProtocolConfig.authority`. That wallet may move treasury USDC at any time; Devnet has no multisig, timelock, withdrawal cap, or program-governed spending policy. This is a disclosed Devnet trust choice, not a Mainnet commitment. Exact authority and treasury addresses are published only after the completed deployment is designated official. See [ADR-0018](adr/0018-devnet-single-admin-and-treasury-authority.md).

> **Implementation mismatch.** The current program uses `protocol_fee_bps = 250` in tests and applies it to settlement payouts. The Devnet MVP instead uses a 1% per-bond fee and a separate 0.1% voting fee; the program and configuration shape must be updated before deployment.

## Dispute Correctness `[Built]`

The first dispute always challenges the default `True` answer, so it does not store a challenged resolution. The second dispute records the LLM result it challenged (`VoteDisputeAccount.challenged_llm_resolution`).

Both dispute accounts carry a `settlement_resolution`, the outcome that settlement is judged against:

- First dispute: the LLM outcome if no vote happens, or the final vote outcome if the LLM verdict is challenged.
- Second dispute: always the final vote outcome.

A dispute is **correct** when the settled outcome contradicts the answer it challenged. `Unresolvable` is a fault-assigned outcome; only `NoConsensus` assigns no correctness:

```text
llm_dispute_correct  = settlement_resolution != True
vote_dispute_correct = settlement_resolution != challenged_llm_resolution
```

The first disputer is correct whenever the final outcome is `False` or `Unresolvable`; the asserter is correct only when it is `True`. A vote disputer is correct whenever the final outcome differs from the LLM verdict they challenged. `NoConsensus` overrides both rules and assigns no fault. See [ADR-0007](adr/0007-unresolvable-vs-no-consensus.md).

## Settlement Defaults

### Resolver Unavailable — Fee-Free Operational Failure `[MVP-target]`

If the trusted resolver does not post before its 24-hour deadline:

- the Assertion terminates with the exceptional `ResolverUnavailable` status and no Resolution Outcome,
- the asserter receives the full 500 USDC bond,
- the LLM disputer receives the full 500 USDC bond,
- no Bond Fee, slashing, or reward applies,
- anyone may invoke the deterministic finalizer, but cannot redirect either refund or select an outcome,
- external transaction and account-creation costs are not reimbursed.

This operational failure is distinct from `NoConsensus`, which is a normal vote outcome and still charges ordinary fees.

### Undisputed Assertion `[Built]`

If the liveness window expires with no dispute:

- the assertion resolves `True`,
- the asserter receives a 495 USDC refund from the original 500 USDC bond,
- the treasury receives the 5 USDC Bond Fee.

### No Consensus — No-Fault `[MVP-target]`

If none of `True`, `False`, or `Unresolvable` reaches the vote's 67% supermajority:

- every unslashed 500 USDC Participant Bond refunds 495 USDC after its Bond Fee,
- unslashed voting stake is returned after the 0.1% Voting Fee,
- no one is slashed; the ordinary fees still apply,
- no rewards are paid,
- the final outcome is `NoConsensus`.

`NoConsensus` reuses the retired `TooEarly` code (`2`) before Devnet deployment. `Unresolvable` remains code `3` and follows ordinary fault, slashing, and reward rules. See [ADR-0007](adr/0007-unresolvable-vs-no-consensus.md).

### First Dispute Settles Correct (`False`) `[Built]`

If the LLM disputer was correct:

- the LLM disputer wins,
- their own bond is refunded minus its Bond Fee,
- the incorrect asserter's remaining collateral is awarded to the LLM disputer after its Bond Fee,
- the LLM disputer receives 990 USDC in total: 495 USDC refunded principal plus a 495 USDC reward.

### First Dispute Settles Incorrect (`True`) `[Built]`

If the LLM disputer was incorrect:

- the asserter wins,
- their own bond is refunded minus its Bond Fee,
- the incorrect LLM disputer's remaining collateral is awarded to the asserter after its Bond Fee,
- the asserter receives 990 USDC in total: 495 USDC refunded principal plus a 495 USDC reward.

This bilateral settlement is the **Winner Takes Remaining Bond** rule. Opal retains the two 5 USDC Bond Fees; there are no role-based reward shares when the assertion resolves before voting.

### Vote Dispute Settlement `[MVP-target]`

If the vote disputer was correct (`settlement_resolution != challenged_llm_resolution`), their bond is refunded after its Bond Fee and contributes 500 units of Reward Weight. If incorrect, its remaining 495 USDC enters the Slashed Pool. `NoConsensus` assigns no correctness and follows the no-fault path.

## Pro-rata Vote Rewards `[MVP-target]`

Before the Voting Window starts, Vote Initialization moves/delegates all three Participant Bonds—1,500 USDC total on Devnet—out of the public Bond Vault and into a PER-resident **Vote Settlement Vault**. Opal also initializes and sponsors a Private Voting Balance for any bonded participant who lacks one. This setup adds no participant charge and does not change wallet authority. Every vote locks stake from a Private Voting Balance into the same vault. This co-location lets Opal apply one Slashed Pool and one pro-rata calculation without cross-runtime partial payouts.

At vote-stage settlement, fees are withheld first:

- each Participant Bond pays its 1% Bond Fee,
- each voting stake pays its 0.1% Voting Fee.

Correct bonded participants become entitled to their 495 USDC refunds. Winning voters become entitled to their stake minus the Voting Fee. Every positive vote-stage refund and reward, including a bonded participant's payout, must be claimed into the recipient's reusable Private Voting Balance; vote finalization does not automatically transfer it or withdraw it to the public Solana wallet. An incorrect bond or losing Vote is terminal `Slashed` and has no payout to claim. The remaining collateral from every incorrect bond and losing vote forms the **Slashed Pool**.

The Slashed Pool is distributed pro rata among every correct bonded participant and winning voter. Reward Weight is based on original capital at risk:

```text
bonded_participant_reward_weight = 500
voter_reward_weight = original_vote_stake

participant_reward =
  slashed_pool × participant_reward_weight ÷ total_reward_weight
```

The protocol takes no additional treasury share from the Slashed Pool; the Bond Fees and Voting Fees are its complete cut. An `Unresolvable` supermajority settles like any other winning voter outcome. `NoConsensus` creates no Slashed Pool and pays no rewards; every fee-adjusted bond and voting stake becomes claimable by its owner. A Vote Round also resolves `NoConsensus` when either quorum condition fails, including when no Votes are cast. In a zero-Vote round, all three bonds become claimable minus their Bond Fees and there is no Voting Fee because there is no voting stake. Every claim amount is fixed permanently at finalization and does not accrue while unclaimed. Claiming moves the payout into the owner's Private Voting Balance and carries no additional Opal fee, although the claimant pays any network execution cost not sponsored by an integrating application. Withdrawing any available private balance to Solana is another separate, manual action.

> **Implementation mismatch.** The current `ProtocolConfig` still contains role-specific reward-share fields (`llm_disputer_reward_share_bps`, `vote_disputer_reward_share_bps`, `voter_reward_share_bps`, and `treasury_share_bps`). Those fields are legacy under the unified pro-rata rule and must be removed before the Devnet MVP is deployed.

## Security Model `[MVP-target]`

Voting weight is **linear**: 1 staked USDC = 1 vote. Security does **not** come from a weight curve — no per-token curve resists both whales and Sybils, since they are the same dial. It comes from **slashing**: being on the losing side of a finalized vote costs money, which is Sybil-neutral and whale-deterring. With slashing as the backstop, weight stays linear and Sybil-neutral.

The minimum voting stake is 1 USDC and there is no maximum. The minimum prevents dust votes from creating needless state. A wallet-level maximum would not constrain influence because a voter could split stake across wallets.

A valid Devnet vote requires the dual **Vote Quorum**:

```text
gross_voting_stake = sum(original_locked_vote_stake)
stake_quorum_met = gross_voting_stake >= 3,000 USDC
wallet_quorum_met = distinct_voting_wallets >= 10
vote_quorum_met = stake_quorum_met AND wallet_quorum_met
```

The 67% Supermajority is evaluated against Gross Voting Stake only when both quorum conditions pass. The threshold is inclusive and uses checked integer cross-multiplication rather than division:

```text
supermajority_met = outcome_gross_stake × 10,000 >= gross_voting_stake × 6,700
```

Exactly 67.000% wins; any smaller share does not. If either quorum condition fails, settlement is `NoConsensus`. The 0.1% Voting Fee is deducted afterward during settlement; it never changes quorum, voting weight, or the final outcome. “Distinct voting wallets” means distinct wallet addresses with valid Votes; Opal does not claim they represent distinct people. A bonded wallet counts once when it Votes, so the Asserter and both Disputers can supply at most three of the required 10 wallets.

Each wallet may cast exactly one vote in a Vote Round. Once accepted, the selected outcome and staked amount are final: the voter cannot switch sides, add stake, or withdraw before settlement. A Vote counts only if its transaction is accepted and recorded on the Voting PER before the seven-day Voting Window deadline. Signing or submitting before the deadline does not reserve a Vote, and a transaction that lands afterward is rejected. The first accepted submission establishes the immutable Vote; later retries or competing submissions fail without changing it, locking additional stake, or incurring an Opal Voting Fee. The limit is per wallet per Vote Round, not global: a wallet may vote in multiple concurrent rounds when its unlocked Private Voting Balance covers each independently locked stake. This remains an enforceable wallet-level rule, not proof that one person has only one vote; a participant can still use multiple wallets.

The Asserter, LLM Disputer, and Vote Disputer may also use their wallet's one Vote. A Participant Bond does not count as that Vote or reduce its voting stake. A bonded wallet may select any outcome, including one that conflicts with its bonded position. Bond capital and voting stake settle independently: each correct position contributes its ordinary Reward Weight, and each incorrect position enters the Slashed Pool after fees. This permits hedging at the same wallet; prohibiting it would be trivially bypassed with another wallet.

Voting stake must come from a pre-funded **Private Voting Balance**, not a public USDC transfer performed as part of casting. Each deposit must be at least 1 USDC and adds to the same reusable balance. Funding is independent of any one Vote Round: a vote locks only its chosen stake, while the remaining unlocked balance can fund another vote or be withdrawn. Active voting stake cannot be withdrawn. After finalization, any fee-adjusted stake and rewards remain in the Vote Settlement Vault until the voter claims them into the Private Voting Balance. Once the claim confirms, those funds are immediately unlocked for another Vote or withdrawal; there is no post-claim cooldown. The voter may authorize a withdrawal of any positive amount up to the unlocked balance; Opal imposes no withdrawal minimum. The balance, positions, and claims remain bound to the original wallet, and Opal cannot recover or redirect them if the wallet's signing key is lost. Deposits and withdrawals are visible on Solana and can still leak timing or amount correlations. Each transfer is `Pending` until the destination balance confirms, then `Completed`; a transfer whose normal path fails becomes `Recovery Needed`. Source-transaction confirmation alone does not change the available balance.

The vote is private during the Voting Window: a MagicBlock Private Ephemeral Rollup hides individual choices, stake amounts, private balances, and per-outcome totals. Sealing removes the visible running tally, so voters can't bandwagon onto a leader and must coordinate on the only remaining focal point — the honest answer under the Resolution Spec. After settlement, the aggregate stake for each outcome, each wallet's selected outcome and stake, and each position's exact payout amount and `Claimable`, `Claimed`, or `Slashed` status become public; total Private Voting Balances remain private. Every voter mapping is stored in its own permanent public `VotePositionAccount`; the round stores only fixed-size aggregates and publication progress. Because positions are published in bounded batches, the outcome may be final before the public position list is complete. Publication is permissionless, and the caller normally pays its external transaction cost unless an integration sponsors it; Opal charges no fee and pays no reward for publishing. See [ADR-0017](adr/0017-per-voter-public-position-accounts.md). If the Vote Quorum is met, `True`, `False`, or `Unresolvable` must reach 67%; if quorum fails or none reaches 67%, the vote settles `NoConsensus`. See [ADR-0003](adr/0003-private-staked-voting.md) and [ADR-0007](adr/0007-unresolvable-vs-no-consensus.md).

## Vision (post-MVP) `[Vision]`

Recorded so the direction is clear and nobody mistakes these for current behaviour:

- **OPAL token** — a protocol token for governance, future staking/reputation, and voter participation incentives. Dropped from the MVP: staked USDC supplies voting weight and the `authority` keypair supplies governance. See [ADR-0004](adr/0004-single-asset-usdc.md).
- **Stake-duration reputation** — long-term staking that accrues voter weight/reputation.
- **Timed resolution** — a possible future protocol-level lifecycle field preventing premature resolution; it would not be an asserter-selected cutoff inside the immutable Resolution Spec.
- **Proof-of-personhood** — to enable sub-linear/quadratic weighting without Sybil collapse.
