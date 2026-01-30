# Q2 Spike: Contract Validation

**Question**: Can Freenet contracts reject invalid state transitions?

**Status**: COMPLETE

## Quick Start

```bash
# From project root
cargo run --bin spike-q2
```

## Background

This spike validates whether Freenet's `ContractInterface` can enforce trust invariants:

- **`validate_state()`** - Returns `ValidateResult::Invalid` to reject state
- **`update_state()`** - Returns `Err(ContractError::InvalidUpdate)` to reject delta

## Invariant Under Test

Every active member must have **>= 2 vouches from active members**.

This is Stroma's core admission requirement.

## Test Scenarios

| Test | Description | Expected |
|------|-------------|----------|
| 1 | Valid delta (2 vouches) | Accepted |
| 2 | Invalid delta (1 vouch) | Rejected by `update_state()` |
| 3 | Post-removal validation | `validate_state()` catches < 2 vouches |
| 4 | Tombstone rejection | Can't re-add removed member |
| 5 | Same-delta conflict | Voucher removed in same delta as addition |

## Key Files

- `contract.rs` - MemberState with validation logic
- `main.rs` - Test runner with all scenarios
- `RESULTS.md` - Detailed findings and decision

## Decision Criteria

| Result | Implication | Action |
|--------|-------------|--------|
| Contract rejects invalid deltas | Trustless | Use contract validation |
| Invalid deltas propagate | Not trustless | Hybrid: bot validates first |

## Results

See [RESULTS.md](RESULTS.md) for complete findings and GO/NO-GO decision.
