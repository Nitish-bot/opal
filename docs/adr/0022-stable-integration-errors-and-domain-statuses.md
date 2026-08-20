# Separate domain statuses from stable integration errors

The Preview Integration API distinguishes observable protocol or transfer state from a failed action. `Pending`, `RecoveryNeeded`, and `ResolverUnavailable` are values returned by reads; they are not thrown exceptions. An action that cannot proceed rejects with a typed integration error containing a stable semantic string `code`, a stable `nextAction` value describing the appropriate participant or integrator response, and an optional diagnostic `cause` preserving the underlying Anchor, Solana RPC, wallet, or transport failure.

Stable integration codes and `nextAction` values are the public contract. Raw Anchor numeric codes, program log text, RPC messages, and wallet-adapter errors are diagnostic details only: integrations may log or display their cause, but cannot branch on them. The Preview layer derives a semantic error only when the program result and any required refreshed account state prove the mapping; an unknown underlying failure remains an explicit unknown operation error rather than being guessed into a known code.

Duplicate-action handling is conditionally idempotent in the Preview layer. After a known duplicate rejection or an ambiguous submission result, the client reads the authoritative record and compares every immutable field relevant to the request. An existing dispute is success only when it belongs to the requested role and wallet and matches the requested Assertion transition. An existing Vote is success only when its round, wallet, outcome, and Gross Voting Stake match exactly. An already-claimed payout is success only when the authenticated position, fixed payout, owner, and protocol-derived destination prove that the requested claim was applied. A match returns the ordinary success shape with `alreadyApplied: true`; a missing record preserves the underlying failure, and an existing non-matching record returns a stable conflict error with `nextAction: "REFRESH_STATE"`.

This reconciliation never changes program semantics. Onchain instructions reject every duplicate before moving additional bonds, voting stake, or payout funds. The Preview layer does not resubmit blindly and never treats a competing dispute or different immutable Vote as success.

The initial stable action mappings are:

| Condition                                       | Preview handling                                                       | `nextAction`     |
| ----------------------------------------------- | ---------------------------------------------------------------------- | ---------------- |
| matching dispute, Vote, or claim already exists | success with `alreadyApplied: true`                                    | —                |
| existing immutable record conflicts             | `STATE_CONFLICT`                                                       | `REFRESH_STATE`  |
| applicable action deadline passed               | `DEADLINE_PASSED`                                                      | `REFRESH_STATE`  |
| unlocked Private Voting Balance is insufficient | `INSUFFICIENT_UNLOCKED_BALANCE`                                        | `REVIEW_BALANCE` |
| transfer is still in flight                     | return transfer status `Pending`, not an error                         | —                |
| transfer requires its recovery flow             | return transfer status `RecoveryNeeded`, not an error                  | —                |
| resolver deadline ended without a verdict       | return Assertion status `ResolverUnavailable`, not an error or outcome | —                |

`INSUFFICIENT_UNLOCKED_BALANCE` contains no requested, unlocked, locked, or total balance amounts. Its default diagnostic cause is redacted as well. A wallet-authenticated client may separately read and display those private values locally, but they are excluded from the typed error and default telemetry. This keeps routine logging from disclosing a private balance or intended Vote stake.

**Why.** An in-progress private-balance transfer, a transfer requiring recovery, and a terminal resolver outage are states that callers must render and reconcile. Treating them as transient exceptions encourages blind retries and can duplicate transactions or misrepresent terminal protocol history. Conversely, action failures such as a passed deadline or insufficient unlocked balance need stable machine-readable meanings even if the program's internal error ordering or RPC wording changes.

**Consequences.** Public examples read and switch on domain statuses directly. Failed action examples catch the typed integration error and switch on its stable `code` or `nextAction`, never a raw program number or English substring. A diagnostic cause is optional, non-contractual, and privacy-redacted by default. Retrying after a lost RPC response converges on the confirmed immutable record without hiding a real conflict; callers may distinguish a newly submitted success from a reconciled success through `alreadyApplied`. A passed deadline is not blindly retried, and an insufficient-balance failure directs the authenticated participant to review their balance without putting private amounts into ordinary error telemetry. The Preview Integration API remains a documentation-only sketch, not a deployed service or SDK.
