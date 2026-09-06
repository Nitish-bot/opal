# Single-asset (USDC) for the MVP; OPAL deferred

The MVP uses a single asset, **USDC**, for all bonds, voting stake, rewards, slashing, treasury fees, and indivisible pro-rata dust. The mint stays a `ProtocolConfig` field (`usdc_mint`) so localnet/devnet can use a test mint, but the protocol commits to USDC and drops the "any USD-pegged stablecoin" framing. The **OPAL** token is removed from the MVP; governance is the `authority` keypair.

**Why.** Every job the docs assigned OPAL — voting weight, voter incentives, governance — is now done by staked USDC or the authority key (see [ADR-0003](0003-private-staked-voting.md) and [ADR-0008](0008-equal-bonds-fees-and-pro-rata-rewards.md)). Carrying a second asset and a governance-token narrative for something nothing uses is pure complexity; one asset makes bonds, rewards, and accounting trivial to reason about.

**Consequences.** The two-asset tokenomics narrative collapses to one asset. OPAL (governance, future reputation/staking) is documented as `Vision`. Code fields rename `*_pusd → *_usdc` and `pusd_mint → usdc_mint` (`anchor build` regenerates the IDL and TS types; the `web/` mock has its own naming and must be updated separately).
