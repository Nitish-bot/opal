---
status: superseded by ADR-0007
---

# Indeterminate assertions settle no-fault

`TooEarly` and `Unresolvable` are merged into a single `Unresolvable` outcome, and an assertion that resolves `Unresolvable` settles **no-fault**: no bond or voting stake is slashed, ordinary participation fees are withheld, and the assertion is voided (re-assert later if it becomes determinable). On Devnet, each 500 USDC Participant Bond refunds 495 USDC after the 1% Bond Fee, and voting stake is returned after the 0.1% Voting Fee. This replaces the rule that treated any `outcome != True` — including indeterminate — as "disputer correct, asserter slashed."

**Why.** `Unresolvable` means nobody was provably wrong: the world wasn't decidable under the spec yet. The old `!= True` rule unjustly slashed the asserter (or, under a True-fallback, the disputer) for a non-determination. `TooEarly` and `Unresolvable` behave identically in settlement, so two codes added no value.

**Considered options.** Timed resolution — a protocol-level `resolves_at` field that prevents early finalization — is more principled but a heavier lifecycle change, deferred to `Vision`. Under the later [ADR-0013](0013-protocol-time-evidence.md), this would not be an asserter-selected cutoff inside the Resolution Spec. Keeping the `!= True` rule was rejected because of the injustice above.

**Consequences.** "No-fault" means no slashing, not fee-free participation: Opal still charges for processing an assertion that fails to produce a determination. No-fault can't be gamed: a frivolous dispute on a genuinely _resolvable_ claim still lands on True/False and is still slashed; only truly indeterminate claims hit the no-fault path. The earlier suggestion that specs state when a statement resolves is superseded by [ADR-0013](0013-protocol-time-evidence.md): evidence timing is protocol-wide and the immutable spec cannot impose a private cutoff. Builds on [ADR-0001](0001-rubric-relative-truth.md) (indeterminacy is relative to the spec).
