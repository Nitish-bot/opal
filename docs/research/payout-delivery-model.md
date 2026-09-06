# Vote-stage payout delivery: automatic push or participant claim?

**Research date:** 2026-08-07
**Scope:** Delivery of finalized vote-stage USDC refunds and rewards from Opal's logical round settlement balance into participants' MagicBlock Private Voting Balances. Actual USDC custody remains in eSPL's shared per-mint global vault on Solana.

## Decision

The Devnet MVP uses **participant-claimed payouts**:

1. Finalize the Vote Round authoritatively on the PER and freeze its payout formula and per-outcome position counts without enumerating every recipient.
2. Expose a deterministic `claim_vote_payout` instruction for one payout position.
3. Require the payout owner's wallet signature.
4. Transfer the payout into the owner's existing Private Voting Balance and mark the position claimed in the same transaction.
5. Keep unclaimed USDC reserved in the logical Vote Settlement Vault balance without a forfeiture deadline.

Only positive payouts become `Claimable`. A fully slashed position is terminal `Slashed` and requires no zero-value claim.

The round's truth outcome is final before every payout is claimed. `Resolved` therefore means that the Solana-projected outcome and payout formula are immutable; it does not mean every participant has received USDC. Positive payouts become claimable when authoritative PER settlement freezes the round, potentially before the deterministic Solana `Resolved` projection, without a cooldown or claim-opening transaction. Their amounts never change afterward and do not accrue while unclaimed.

This choice favors a smaller protocol and removes payout-worker liveness and operating-cost dependencies. It also adds one wallet interaction before the already-separate action of withdrawing a Private Voting Balance to Solana.

## Source-backed constraints

### A Vote Round with an unbounded voter set cannot pay everyone in one transaction

Solana transactions are atomic but size-limited. Current official documentation states a maximum serialized transaction size of 1,232 bytes, at most 64 accounts under the active limit, at most 64 executed instructions, and a maximum compute budget of 1.4 million compute units per transaction. ([Solana transactions](https://solana.com/docs/core/transactions), [Solana fees](https://solana.com/docs/core/fees))

**Inference for Opal:** authoritative outcome finalization can be atomic within one PER transaction, and each individual claim must be atomic within one later same-PER transaction, but projecting to Solana and paying an unlimited voter set cannot be one cross-runtime action. Required claims distribute the necessary `O(number of positions)` work across recipients. The feasibility spike must verify these transaction boundaries for the selected ER/eSPL interface.

### Claims optimize protocol cost; pushes optimize recipient UX

The Solana Foundation's reward-distribution guide describes claim-based distribution as more efficient and push-based distribution as more expensive because push directly sends to every contributor. The same guide also documents an automatic rewards-crank model. ([Solana DePIN reward-distribution guide](https://solana.com/developers/cookbook/depin))

Every Solana transaction requires a fee payer. A claim shifts submission and fee responsibility toward the recipient unless an integrator sponsors it; an automatic push shifts them toward the protocol or its sponsor. ([Solana fees](https://solana.com/docs/core/fees))

**Inference for Opal:** claims do not eliminate per-recipient execution. They choose who initiates it and avoid an Opal-operated processor. An integrator can sponsor the transaction while still requiring the recipient to sign.

### Private-balance delivery has an explicit participant-owned prerequisite

MagicBlock's private eSPL documentation describes private token setup as initializing token accounts and Ephemeral ATAs, setting permissions, and depositing and delegating before transfer. ([MagicBlock Private eSPL flow](https://docs.magicblock.gg/api-reference/spl-api/introduction-per))

The hosted Private Payments API builds unsigned transactions and returns `requiredSigners`; clients sign the builder result before submission. ([MagicBlock Private Payments transfer API](https://docs.magicblock.gg/api-reference/per-api/transferAmount))

**Inference for Opal:** the generic Payments API is not the protocol accounting layer. `claim_vote_payout` uses a pinned onchain eSPL interface and an internal same-PER token transfer so the payout movement and Opal state update share one transaction. The payout owner signs the Opal claim, while Opal's PDA authorizes the logical settlement debit. Every voter already creates and pre-funds a Private Voting Balance. A bonded participant that did not vote uses a spike-proven direct initializer to create and delegate an empty canonical destination before claiming and pays its SOL setup costs; receiving the payout must not require a positive USDC deposit. Its missing destination cannot block finalization, and the non-expiring entitlement remains reserved. The claim validates but never creates or redirects the destination. Opal provides no protocol-funded sponsorship, although an external integration may voluntarily pay setup or transaction costs.

This authority path must be proven in a Devnet integration test. MagicBlock access permissions protect permissioned account visibility and have their own lifecycle; they must not be confused with Opal's token authority checks. ([MagicBlock PER access control](https://docs.magicblock.gg/pages/private-ephemeral-rollups-pers/how-to-guide/access-control), [Solana PDA signing through CPI](https://solana.com/docs/core/cpi/cpi-with-pda))

### Do not use the delayed private-transfer queue for same-PER claims

MagicBlock's pinned eSPL source removes a transfer-queue item once payout execution is scheduled, before the payout result is known. ([MagicBlock eSPL queue tick source](https://github.com/magicblock-labs/ephemeral-spl-token/blob/c789b93850e206f185ed321140df1b5cb81eed5a/e-token/src/processor/transfer_queue_tick.rs))

The Private Payments documentation likewise distinguishes source transaction confirmation from final queued payment completion. ([MagicBlock Private Payments introduction](https://docs.magicblock.gg/pages/private-ephemeral-rollups-pers/api-reference/per/introduction))

**Inference for Opal:** queued private transfers solve delayed or cross-runtime delivery. Opal's logical Vote Settlement Vault balance and Private Voting Balances are already co-located in one PER, and the MVP publishes wallet-to-vote mappings after finalization. A direct synchronous same-PER claim avoids queue callbacks, refund states, and a second asynchronous workflow. A participant's later withdrawal to Solana remains a separate wallet-authorized cross-runtime operation.

Before claiming, the claimant funds a fixed-size `ClaimReceipt` PDA on Solana and delegates it. The payout transfer, positive-unclaimed-position count, authoritative private claimed marker, and receipt write update in the same PER transaction. Commit-and-undelegate later restores the receipt for public synchronization; an event or ER-only Ephemeral Account is not sufficient. Public `VotePositionAccount` status is a later projection: registration may set it to `Claimed` only when it also validates the matching restored claim receipt. Otherwise it may expose stale `Claimable` until later synchronization. Temporary public lag cannot be the double-claim authority, and permissionless synchronization remains gated on proving the PDA authority.

## Proposed Devnet state and instruction model

### 1. Finalize the round once

The PER finalization instruction should atomically freeze:

- final outcome and quorum/supermajority evidence;
- final aggregate stake and position counts per outcome;
- all three Bond Fees, the accumulated sum of independently floored Voting Fees, and Slashed Pool totals;
- the immutable pro-rata numerator and denominator inputs;
- the immutable number of voter positions;
- the exact number of positive unclaimed positions.

It should not attempt to enumerate all recipients or claim to know the sum of independently floored payout liabilities from aggregate weight alone. Each successful positive claim decrements the count and records its exact amount. Once the count is zero, checked balance and claimed-total reconciliation classifies only a residual smaller than the rewarded-position count as pro-rata dust; a mismatch becomes `RecoveryNeeded`. Integrators still wait for the deterministic aggregate receipt to reach the exact Assertion at Solana `finalized` before consuming the truth result. The target representation uses one permanent public `VotePositionAccount` per Vote Round and voter wallet, with only aggregates and publication counters on the round; see [ADR-0017](../adr/0017-per-voter-public-position-accounts.md).

### 2. Claim each position deterministically

`claim_vote_payout(round, position, recipient_private_balance)` should:

- require a finalized round and the position owner's signature;
- validate that the position belongs to the round;
- derive the signer's canonical Private Voting Balance and validate that it uses USDC and belongs to the configured Voting PER; no arbitrary payout destination is accepted;
- derive the payout only from frozen round totals plus the position's original bond or Gross Voting Stake;
- require the derived payout to be positive and the position to be `Claimable`;
- reject a position that has already been claimed;
- transfer the payout from the logical Vote Settlement Vault balance;
- mark the position claimed and decrement the positive-unclaimed-position count in the same transaction;
- leave the credited USDC immediately unlocked for another Vote or withdrawal.

The selected ER/eSPL interface must prove in the Devnet spike that this is one same-PER transaction: a failed claim must leave the balance, claimed marker, count, and receipt unchanged and safe to retry. Base-layer Solana atomicity is not evidence for this separate runtime boundary.

A wallet that owns both a bonded position and a Vote position has two independent claim records. A client may bundle both claim instructions into one transaction and wallet signature after Devnet simulation establishes a safe account and compute budget, but batching is a client optimization rather than a second accounting model.

### 3. Preserve unclaimed liabilities

Expose two distinct facts:

- `RoundFinalized`: the protocol outcome and payout formula are immutable.
- `PositionPublicationComplete`: every per-voter public position record has been published through bounded batches.
- a public exact payout amount and `Claimable`, `Claimed`, or `Slashed` status per position;
- a private total Private Voting Balance for each wallet.

The logical Vote Settlement Vault record may close only when every positive payout entitlement is claimed and independent checks prove zero logical balance, active locks, positive-unclaimed-position count, and unstaged fee or pro-rata-dust liability. eSPL's shared global vault is not closed per round. All three vote-stage Bond Fees and the exact Voting Fee total use a `TreasuryFeeDelivery`; dust uses a distinct `TreasuryDustDelivery` only after the positive-unclaimed count reaches zero. Each delivery has its own nonce and one-way status. A generic settled flag is insufficient. Slashed positions require no claim. One invalid claim must not affect any other position. No forfeiture deadline is recommended for Devnet.

## Option comparison

| Model                                      | User action                        | Who normally funds execution | Unlimited voter set                  | Liveness                                          | MVP fit                                |
| ------------------------------------------ | ---------------------------------- | ---------------------------- | ------------------------------------ | ------------------------------------------------- | -------------------------------------- |
| **Required participant claim**             | Required for every payout position | Participant or sponsor       | Scales by distributing work          | No payout worker; inactive users remain unclaimed | **Chosen: smallest reliable protocol** |
| Operator-only automatic push               | None                               | Opal                         | Requires bounded `O(N)` transactions | Operator outage blocks delivery                   | Better UX, weaker recovery             |
| Automatic-first, permissionless processing | None normally                      | Opal normally                | Requires bounded `O(N)` transactions | Requires processor and reconciliation machinery   | Sound, but more MVP machinery          |

## Devnet acceptance tests

1. Finalize a round with more positions than fit in one transaction; the outcome becomes final without paying them all.
2. Claim positions in different orders; every final balance and vault total is identical.
3. Replay a successful claim; no second credit is possible.
4. Submit a claim with the wrong signer, wallet balance, mint, Voting PER, round, or forged position; each fails without changing state.
5. Leave claims outstanding; the positive-unclaimed count and funds remain reserved and the logical settlement record cannot close.
6. Exercise `NoConsensus`, each fault outcome, and a bonded wallet that also voted.
7. Verify a recipient can read the credited Private Voting Balance after authentication and later withdraw separately; the claim itself must not initiate a public-wallet withdrawal.
8. Claim every positive position, verify the positive-unclaimed count reaches zero, reconcile the remaining bounded dust, stage both treasury tranches, and close only after every type-specific check passes.

## Final decision

Adopt **required participant claims** for vote-stage Devnet payouts. Claims are bounded, atomic, retryable, and remove the need for an Opal-operated payout processor. Keep automatic processing as a future UX improvement if measured demand justifies its sponsorship and reconciliation machinery.
