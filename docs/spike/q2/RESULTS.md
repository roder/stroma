# Q2: Contract Validation - Spike Results

**Date**: 2026-01-30
**Branch**: spike/q1/freenet_conflict_resolution
**Status**: COMPLETE - GO DECISION

## Executive Summary

**GO/NO-GO: ✅ GO**

Freenet contracts CAN enforce trust invariants. Both `update_state()` and `validate_state()` provide validation hooks that reject invalid deltas/states. Stroma can use a **trustless model** where the contract enforces the "2 vouches minimum" invariant.

---

## Test Results

### Test 1: Valid Delta Accepted

```
Initial state: {"Alice", "Bob", "Carol"}
Delta: Add Dave with vouches from Alice and Bob

✅ ACCEPTED: Delta applied successfully
   New state: {"Alice", "Bob", "Carol", "Dave"}
   Dave's vouch count: 2
```

**Finding**: Deltas with valid vouch counts are accepted.

### Test 2: Invalid Delta Rejected

```
Initial state: {"Alice", "Bob", "Carol"}
Delta: Add Dave with only 1 vouch (Alice)
Expected: REJECTED (need >= 2 vouches)

✅ REJECTED by update_state(): Member Dave has only 1 valid vouches (need >= 2)
   Dave NOT in active: true
```

**Finding**: `update_state()` can reject invalid deltas BEFORE they're applied. The state is not modified.

### Test 3: Post-Removal Validation

```
After adding Dave: {"Alice", "Bob", "Carol", "Dave"}
Dave's vouch count: 2

Scenario: Alice is removed (Dave's voucher)
After removing Alice:
   Active: {"Bob", "Carol", "Dave"}
   Dave's vouch count: 1 (was 2, now should be 1)

Calling validate_state() on merged state...
✅ INVALID detected by validate_state():
   Reason: Member Bob has only 1 vouches (need >= 2)
```

**Finding**: `validate_state()` catches invariant violations in the merged state. This demonstrates that removing a member affects the vouch counts of ALL remaining members who were vouched by the removed member. The validation correctly identifies that Bob now has only 1 vouch (Carol remains, but Alice is gone).

### Test 4: Tombstone Rejection

```
After removing Alice:
   Active: {"Bob", "Carol"}
   Removed (tombstones): {"Alice"}

Attempting to re-add Alice (with valid vouches)...
✅ REJECTED: Cannot add tombstoned member: Alice
   Tombstone enforcement works!
```

**Finding**: Tombstones are enforced. Once removed, a member cannot be re-added (matching Q1's remove-wins semantics).

### Test 5: Voucher Removal in Same Delta

```
Initial state: {"Alice", "Bob", "Carol"}
Delta: Add Dave (vouched by Alice, Bob) AND remove Alice

❌ Delta REJECTED: InvalidState("Delta created invalid state: Member Bob has only 1 vouches (need >= 2)")
   update_state() detected the issue
```

**Finding**: When a delta both adds a member and removes their voucher:
- `update_state()` applies the delta, then calls `validate_state()` internally
- Post-application validation catches the invariant violation
- Delta is rejected BEFORE being committed

This is even better than expected — the two-layer defense works within `update_state()` itself, not just between separate calls.

---

## Architectural Insights

### Two-Layer Validation Model

```
┌─────────────────────────────────────────────────────┐
│  Bot: Creates deltas, handles Signal, UX           │
│       Pre-validates for better error messages      │
└───────────────────┬─────────────────────────────────┘
                    │ submit delta
                    ▼
┌─────────────────────────────────────────────────────┐
│  Contract update_state():                          │
│  • Validates delta (>= 2 vouches, not tombstoned)  │
│  • Returns Err(InvalidUpdate) if invalid           │
│  • First line of defense                           │
└───────────────────┬─────────────────────────────────┘
                    │ if valid
                    ▼
┌─────────────────────────────────────────────────────┐
│  Contract validate_state():                        │
│  • Validates final merged state                    │
│  • Returns Invalid if invariants violated          │
│  • Catches edge cases like same-delta conflicts    │
│  • Second line of defense                          │
└─────────────────────────────────────────────────────┘
```

### When Each Validation Fires

| Hook | When Called | What It Checks |
|------|-------------|----------------|
| `update_state()` | On delta submission | Delta validity against current state |
| `validate_state()` | Before caching/propagating | Final state validity after merge |

### Trustless vs Hybrid Model

| Aspect | Trustless (Contract) | Hybrid (Bot + Contract) |
|--------|---------------------|------------------------|
| Invariant enforcement | Contract | Contract (with bot pre-check) |
| Error messages | Generic | Rich (bot can explain) |
| UX quality | Lower | Higher |
| Attack surface | Smaller | Larger (bot could be compromised) |

**Recommendation**: Use **Trustless with UX Enhancement**:
- Contract enforces invariants (authoritative)
- Bot pre-validates for better error messages (convenience)
- If bot is compromised, contract still rejects invalid deltas

---

## Answers to Original Questions

### Q2: Can contracts reject invalid state transitions?

**Answer: YES**

Two mechanisms available:

1. **`update_state()`** returns `Err(ContractError::InvalidUpdate)`:
   - Rejects delta before it's applied
   - State is NOT modified
   - Immediate rejection at submission time

2. **`validate_state()`** returns `ValidateResult::Invalid`:
   - Rejects final merged state
   - State is NOT cached or propagated
   - Catches edge cases that `update_state()` misses

### Does this allow trustless enforcement?

**YES** - Contract can enforce:
- Minimum vouch count (>= 2)
- Tombstone permanence (can't re-add removed members)
- No member in both active and removed sets
- Any other invariant we define

### What about edge cases?

| Edge Case | Handled By |
|-----------|------------|
| Invalid delta (< 2 vouches) | `update_state()` |
| Tombstoned re-addition | `update_state()` |
| Voucher removed in same delta | `validate_state()` |
| Post-merge vouch count drop | `validate_state()` |
| Active + removed conflict | `validate_state()` |

---

## Implications for Stroma Architecture

### Contract Design

```rust
#[contract]
impl ContractInterface for StromaContract {
    fn update_state(...) -> Result<UpdateModification<'static>, ContractError> {
        // Layer 1: Pre-check delta validity
        for addition in &delta.additions {
            if vouch_count < 2 {
                return Err(ContractError::InvalidUpdate(...));
            }
            if tombstoned {
                return Err(ContractError::InvalidUpdate(...));
            }
        }
        
        // Apply delta
        state.apply_delta(&delta);
        
        Ok(UpdateModification::valid(state))
    }
    
    fn validate_state(...) -> Result<ValidateResult, ContractError> {
        // Layer 2: Post-merge state validation
        for member in &state.active {
            if vouch_count(member) < 2 {
                return Ok(ValidateResult::Invalid);
            }
        }
        Ok(ValidateResult::Valid)
    }
}
```

### Bot Role (Still Needed)

Even with contract validation, the bot is needed for:

1. **Pre-validation with rich errors**: "You need 1 more vouch from a member in a different cluster"
2. **Trust recalculation**: Computing effective vouches, standing, etc.
3. **Signal synchronization**: Adding/removing members from Signal group
4. **UX**: Commands, notifications, status messages
5. **Mesh optimization**: Blind Matchmaker suggestions

### Security Improvement

| Before (Hybrid) | After (Trustless) |
|-----------------|-------------------|
| Bot validates, contract stores | Contract validates AND stores |
| Compromised bot = invalid state possible | Compromised bot = rejected by contract |
| Single point of failure | Defense in depth |

---

## Next Steps

1. **Q3 Spike**: Cluster detection for cross-cluster vouching requirement
2. **Update architecture docs**: Document two-layer validation model
3. **Design production contract**: Full `ContractInterface` implementation
4. **Consider cross-cluster validation**: Can contract enforce different-cluster vouches?

---

## Files Modified

- `contract.rs` - MemberState with validation logic
- `main.rs` - Test runner with all scenarios
- `README.md` - Spike documentation
- `RESULTS.md` - This file (complete results)
- `Cargo.toml` - Added `spike-q2` binary
