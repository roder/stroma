# Vouch Invalidation Logic: Critical Trust Model Refinement

**Date**: 2026-02-01  
**Status**: Integrated into all architecture documentation  
**Impact**: Fundamental change to Standing calculation

**Canonical Sources**:
- `.beads/security-constraints.bead` - Trust model enforcement, ejection triggers
- `.beads/terminology.bead` - Trust calculation definitions
- `.beads/vetting-protocols.bead` - Ejection protocol overview
- `.beads/architectural-decisions-open.bead` - Decision rationale (Decision #10)

## The Problem

**Original Assumption**: `Standing = Total_Vouches - Total_Flags`

**Logical Flaw Discovered**: What happens if a voucher later flags the person they vouched for?

### Example Scenario
1. Alice vouches for Bob during vetting
2. Bob admitted to group (2 vouches total)
3. Months later, Alice flags Bob ("behavior changed, no longer trust them")

**Under Original Math**:
- Vouches: 2 (Alice, Carol)
- Flags: 1 (Alice)
- Standing: 2 - 1 = +1
- Result: **STAYS in group**

**Logical Inconsistency**: Alice is simultaneously saying:
- "I trust Bob" (vouch)
- "I don't trust Bob" (flag)

This is contradictory and undermines the trust model.

## The Solution: Vouch Invalidation

### Core Principle
**If a voucher flags a member, that vouch is invalidated.**

**Critical Constraint: NO UNILATERAL 2-POINT SWINGS**
- No single member can cause a 2-point standing change through their own action
- Flags from vouchers invalidate the vouch ONLY
- A voucher's flag is NOT counted as an additional regular flag

Rationale:
- Vouches represent **current trust**, not historical endorsement
- You can't simultaneously trust and distrust someone
- Aligns with "fluid identity" philosophy (trust is relational and dynamic) — see `.beads/philosophical-foundations.bead` Duality #5 (Fluidity vs Stability)
- Prevents weaponization: prevents single member from ejecting another unilaterally — see `.beads/philosophical-foundations.bead` Duality #3 (Individual Agency vs Collective Integrity)
- Ensures: standing reductions require independent perspectives

### Refined Calculation

```
All_Vouchers = Set of members who vouched for you
All_Flaggers = Set of members who flagged you
Voucher_Flaggers = All_Vouchers ∩ All_Flaggers (contradictory members)

Effective_Vouches = |All_Vouchers| - |Voucher_Flaggers|
Regular_Flags = |All_Flaggers| - |Voucher_Flaggers|
Standing = Effective_Vouches - Regular_Flags
```

**Why This Works**:
- Voucher-flaggers are excluded from BOTH vouch count AND flag count
- Their contradictory action is treated as "vouch revocation"
- Only consistent flags (from non-vouchers) affect standing
- Prevents double-counting contradictory actions

### Example Scenarios (Refined)

#### Scenario 1: Flag from Non-Voucher
```
All vouches: 2 (Alice from Cluster A, Bob from Cluster B)
All flags: 1 (Carol)
Voucher-flaggers: 0 (Carol didn't vouch)

Effective vouches: 2 - 0 = 2
Regular flags: 1 - 0 = 1
Standing: 2 - 1 = +1

Trigger 1 (Standing): +1 (≥ 0) ✅
Trigger 2 (Effective vouches): 2 (≥ 2) ✅
Trigger 3 (Cross-cluster): 2 clusters (≥ 2) ✅
Result: STAYS (healthy member, flagged by someone outside their vouchers)
```

**Note**: All scenarios assume cross-cluster requirement is satisfied (vouches from different clusters). If a voucher leaves and reduces cluster diversity below required threshold, that's a separate ejection trigger. See `.beads/cross-cluster-requirement.bead`.

#### Scenario 2: Flag from Voucher (Vouch Invalidation)
```
All vouches: 2 (Alice from Cluster A, Bob from Cluster B)
All flags: 1 (Alice)
Voucher-flaggers: 1 (Alice)

Effective vouches: 2 - 1 = 1 (Alice's vouch invalidated)
Regular flags: 1 - 1 = 0
Standing: 1 - 0 = +1

Trigger 1 (Standing): +1 (≥ 0) ✅
Trigger 2 (Effective vouches): 1 (< 2) ❌
Result: EJECTED (only 1 effective vouch remains)
```

#### Scenario 3: Multiple Voucher-Flaggers
```
All vouches: 3 (Alice from A, Bob from B, Carol from C)
All flags: 2 (Alice, Bob)
Voucher-flaggers: 2 (Alice, Bob)

Effective vouches: 3 - 2 = 1 (only Carol's vouch remains)
Regular flags: 2 - 2 = 0
Standing: 1 - 0 = +1

Trigger 1 (Standing): +1 (≥ 0) ✅
Trigger 2 (Effective vouches): 1 (< 2) ❌
Result: EJECTED (only 1 effective vouch remains)
```

#### Scenario 4: Sufficient Effective Vouches After Invalidation
```
All vouches: 4 (Alice from A, Bob from B, Carol from C, Dave from A)
All flags: 1 (Alice)
Voucher-flaggers: 1 (Alice)

Effective vouches: 4 - 1 = 3 (Bob, Carol, Dave remain)
Regular flags: 1 - 1 = 0
Standing: 3 - 0 = +3

Trigger 1 (Standing): +3 (≥ 0) ✅
Trigger 2 (Effective vouches): 3 (≥ 2) ✅
Trigger 3 (Cross-cluster): 3 clusters (B, C, A via Dave) ✅
Result: STAYS (still has 3 effective vouches from multiple clusters)
```

#### Scenario 5: Both Voucher-Flaggers and Regular Flags
```
All vouches: 3 (Alice from A, Bob from B, Carol from C)
All flags: 3 (Alice, Dave, Eve)
Voucher-flaggers: 1 (Alice)

Effective vouches: 3 - 1 = 2 (Bob, Carol)
Regular flags: 3 - 1 = 2 (Dave, Eve)
Standing: 2 - 2 = 0

Trigger 1 (Standing): 0 (≥ 0) ✅ (edge case)
Trigger 2 (Effective vouches): 2 (≥ 2) ✅
Trigger 3 (Cross-cluster): 2 clusters (B, C) ✅
Result: STAYS (edge case: zero standing but enough cross-cluster effective vouches)
```

## Benefits of Vouch Invalidation

### 1. Logical Consistency
- Prevents contradictory state (trust and distrust simultaneously)
- Vouches represent current trust, not historical endorsement
- Aligns with "fluid identity" philosophy

### 2. Security Against Gaming
**Attack Prevented**: "Vouch Bombing"
```
Attacker Strategy (Without Invalidation):
1. Vouch for 10 people (gain trust)
2. All 10 get admitted
3. Later, flag all 10 people
4. Result: All 10 ejected, but attacker's vouches still count

With Invalidation:
- Attacker's vouches for those 10 people are invalidated
- Attacker's effective vouch count drops
- If attacker had vouched for enough people who they later flagged,
  their own effective vouches may drop below threshold
- System is self-regulating
```

### 3. Reflects Relationship Dynamics
- Relationships change over time
- Trust can be revoked (via flagging)
- Vouches are not permanent endorsements
- System adapts to evolving social dynamics

### 4. Fair Ejection
- If all your vouchers flag you, you're immediately ejected (effective vouches = 0)
- If 1 of 2 vouchers flags you, you're immediately ejected (effective vouches = 1 < 2)
- If you have resilience (3+ vouchers from 3+ clusters), losing 1 to flagging doesn't eject you
- Incentivizes building multiple cross-cluster trust relationships

**Re-Entry**: Ejected members can re-enter through normal admission (2 new cross-cluster vouches), but flags persist. See `.beads/vetting-protocols.bead` and `.beads/philosophical-foundations.bead` Duality #4 (Accountability vs Forgiveness).

## Implementation Details

### Contract State (No Change)
The contract state remains the same (sets already support this logic):

```rust
pub struct VouchGraph {
    vouches: HashMap<MemberHash, BTreeSet<MemberHash>>,
}

pub struct FlagGraph {
    flags: HashMap<MemberHash, BTreeSet<MemberHash>>,
}
```

**Key**: Vouches and flags are separate sets. We calculate intersection at query time.

### Helper Methods (Updated)

```rust
impl TrustNetworkState {
    /// Calculate effective state considering voucher-flaggers
    pub fn calculate_effective_state(&self, member: &MemberHash) -> (usize, i32) {
        let vouchers = self.vouches.get(member).cloned().unwrap_or_default();
        let flaggers = self.flags.get(member).cloned().unwrap_or_default();
        
        // Find vouchers who also flagged (contradictory)
        let voucher_flaggers: HashSet<_> = vouchers
            .intersection(&flaggers)
            .collect();
        
        // Effective vouches = vouchers who haven't flagged
        let effective_vouches = vouchers.len() - voucher_flaggers.len();
        
        // Regular flags = flags from non-vouchers
        let regular_flags = flaggers.len() - voucher_flaggers.len();
        
        // Standing = effective_vouches - regular_flags
        let standing = effective_vouches as i32 - regular_flags as i32;
        
        (effective_vouches, standing)
    }
}
```

### Contract Verification (Updated)

```rust
impl ComposableState for TrustNetworkState {
    fn verify(&self, _parent: &Self::ParentState, _params: &Self::Parameters) -> Result<(), String> {
        for member in &self.members.active {
            let (effective_vouches, standing) = self.calculate_effective_state(member);
            
            // Check effective vouch count (after invalidation)
            if effective_vouches < self.config.min_vouch_threshold {
                return Err(format!("Member {} has only {} effective vouches", member, effective_vouches));
            }
            
            // Check standing
            if standing < 0 {
                return Err(format!("Member {} has negative standing", member));
            }
        }
        Ok(())
    }
}
```

## User Experience Impact

### Bot Messages (Updated)

#### When Voucher Flags
```
Carol: /flag @Bob "Behavior changed, no longer trust them"

Bot → Carol (1-on-1 PM):
"Flag recorded and verified on Freenet.

⚠️ Important: You previously vouched for @Bob. Flagging them 
invalidates your vouch. This is treated as a vouch revocation.

@Bob's trust standing:
- All vouches: 3 (Carol, Dave, Eve)
- All flags: 1 (Carol)
- Voucher-flaggers: 1 (Carol - vouch invalidated)
- Effective vouches: 2 (Dave, Eve remain)
- Regular flags: 0
- Standing: +2 (positive)

@Bob remains in the group but is now at minimum threshold."
```

#### When Member Checks Status
```
Member: /status

Bot → Member (1-on-1 PM):
"📊 Your Trust Status

Role: Bridge
All vouches: 3 (Alice, Bob, Carol)
All flags: 1 (Alice)
⚠️ Voucher-flaggers: 1 (Alice - vouch invalidated)
Effective vouches: 2 ✅ (Bob, Carol)
Regular flags: 0
Standing: +2 (positive)

💡 Note: Alice vouched for you but later flagged you, invalidating 
her vouch. You still have 2 effective vouches from Bob and Carol.

You're at minimum effective vouch threshold. Consider building more 
connections for resilience."
```

## Testing Requirements

### Unit Tests (Required)

```rust
#[test]
fn test_vouch_invalidation() {
    let mut state = TrustNetworkState::new();
    
    // Add Bob with vouches from Alice and Carol
    state.add_member(bob_hash, vec![alice_hash, carol_hash]);
    
    // Verify: 2 effective vouches
    let (effective, standing) = state.calculate_effective_state(&bob_hash);
    assert_eq!(effective, 2);
    assert_eq!(standing, 2);
    
    // Alice flags Bob
    state.add_flag(bob_hash, alice_hash);
    
    // Verify: Only 1 effective vouch (Alice's invalidated)
    let (effective, standing) = state.calculate_effective_state(&bob_hash);
    assert_eq!(effective, 1);  // Only Carol's vouch remains
    assert_eq!(standing, 1);   // No regular flags
    
    // Verify: Should be ejected (< 2 effective vouches)
    assert!(state.should_eject(&bob_hash));
}

#[test]
fn test_multiple_voucher_flaggers() {
    let mut state = TrustNetworkState::new();
    
    // Add Bob with 3 vouches
    state.add_member(bob_hash, vec![alice_hash, carol_hash, dave_hash]);
    
    // Alice and Carol both flag Bob
    state.add_flag(bob_hash, alice_hash);
    state.add_flag(bob_hash, carol_hash);
    
    // Verify: Only 1 effective vouch (Dave)
    let (effective, standing) = state.calculate_effective_state(&bob_hash);
    assert_eq!(effective, 1);  // Only Dave's vouch remains
    assert!(state.should_eject(&bob_hash));  // < 2 effective vouches
}

#[test]
fn test_sufficient_effective_vouches_after_invalidation() {
    let mut state = TrustNetworkState::new();
    
    // Add Bob with 4 vouches
    state.add_member(bob_hash, vec![alice_hash, bob_hash, carol_hash, dave_hash]);
    
    // Alice flags Bob
    state.add_flag(bob_hash, alice_hash);
    
    // Verify: Still has 3 effective vouches
    let (effective, standing) = state.calculate_effective_state(&bob_hash);
    assert_eq!(effective, 3);  // Bob, Carol, Dave remain
    assert!(!state.should_eject(&bob_hash));  // 3 >= 2, stays in group
}
```

### Property Tests (Required)

```rust
#[cfg(test)]
mod property_tests {
    use proptest::prelude::*;
    
    proptest! {
        #[test]
        fn vouch_invalidation_always_reduces_effective_vouches(
            vouchers in prop::collection::hash_set(any::<Hash>(), 0..10),
            flaggers in prop::collection::hash_set(any::<Hash>(), 0..10),
        ) {
            let voucher_flaggers = vouchers.intersection(&flaggers).count();
            let effective = vouchers.len() - voucher_flaggers;
            
            // Effective vouches never exceed total vouches
            assert!(effective <= vouchers.len());
            
            // If any voucher flags, effective < total
            if voucher_flaggers > 0 {
                assert!(effective < vouchers.len());
            }
        }
    }
}
```

## Security Benefits

### Attack Prevention: "Vouch Bombing"

**Attack Without Invalidation**:
1. Malicious actor vouches for many people (builds reputation)
2. All vouched people get admitted
3. Later, malicious actor flags everyone they vouched for
4. Mass ejection occurs
5. Malicious actor's standing unaffected (vouches still count)

**Defense With Invalidation**:
1. Malicious actor vouches for many people
2. All vouched people get admitted
3. Later, malicious actor flags everyone they vouched for
4. All those vouches are invalidated
5. Malicious actor's effective vouch count drops
6. If malicious actor vouched for enough people who they later flagged:
   - Their own effective vouches may drop below threshold
   - They could eject themselves
7. System is self-regulating

### Attack Prevention: "Fake Voucher"

**Attack Without Invalidation**:
1. Attacker vouches for confederate
2. Confederate admitted
3. Confederate misbehaves
4. Attacker flags confederate to "distance" themselves
5. Attacker's vouch still counts toward their standing

**Defense With Invalidation**:
- Attacker's vouch is invalidated when they flag confederate
- Attacker's effective vouch count drops
- System naturally punishes vouching for bad actors

## Edge Cases

### Edge Case 1: All Vouchers Flag You
```
All vouches: 2 (Alice from Cluster A, Bob from Cluster B)
All flags: 2 (Alice, Bob)
Voucher-flaggers: 2 (Alice, Bob)

Effective vouches: 2 - 2 = 0
Regular flags: 2 - 2 = 0
Standing: 0 - 0 = 0

Trigger 1 (Standing): 0 (≥ 0) ✅
Trigger 2 (Effective vouches): 0 (< 2) ❌
Result: EJECTED (no effective vouches remain)
```

**Interpretation**: Everyone who vouched for you has lost trust. You should be ejected. Note that both vouchers' flags only invalidate their vouches (net -2 standing points), not -4 as it would be under old math.

### Edge Case 2: Zero Standing but Enough Cross-Cluster Vouches
```
All vouches: 3 (Alice from A, Bob from B, Carol from C)
All flags: 3 (Dave, Eve, Frank — none are vouchers)
Voucher-flaggers: 0

Effective vouches: 3 - 0 = 3
Regular flags: 3 - 0 = 3
Standing: 3 - 3 = 0

Trigger 1 (Standing): 0 (≥ 0) ✅ (edge case — zero is NOT negative)
Trigger 2 (Effective vouches): 3 (≥ 2) ✅
Trigger 3 (Cross-cluster): 3 clusters ✅
Result: STAYS (controversial but logically consistent)
```

**Interpretation**: Your vouchers still trust you (didn't flag), but others don't. Zero standing is the boundary case — you stay in group because vouchers haven't revoked trust. The threshold is "Standing < 0" (strictly negative), not "Standing ≤ 0".

### Edge Case 3: Negative Standing from Regular Flags
```
All vouches: 2 (Alice from Cluster A, Bob from Cluster B)
All flags: 5 (Carol, Dave, Eve, Frank, Grace — none are vouchers)
Voucher-flaggers: 0

Effective vouches: 2 - 0 = 2
Regular flags: 5 - 0 = 5
Standing: 2 - 5 = -3

Trigger 1 (Standing): -3 (< 0) ❌
Trigger 2 (Effective vouches): 2 (≥ 2) ✅
Trigger 3 (Cross-cluster): 2 clusters ✅
Result: EJECTED (Trigger 1 failed: too many regular flags)
```

**Interpretation**: Your vouchers still trust you, but the broader community overwhelmingly doesn't. Negative standing triggers ejection regardless of vouch count or cluster diversity.

### Edge Case 4: Vouch Invalidation Causes Cross-Cluster Violation
```
All vouches: 2 (Alice from Cluster A, Bob from Cluster B)
All flags: 1 (Alice — a voucher)
Voucher-flaggers: 1 (Alice)

Effective vouches: 2 - 1 = 1 (only Bob remains)
Regular flags: 1 - 1 = 0
Standing: 1 - 0 = +1
Remaining clusters: 1 (only Cluster B)

Trigger 1 (Standing): +1 (≥ 0) ✅
Trigger 2 (Effective vouches): 1 (< 2) ❌
Trigger 3 (Cross-cluster): 1 cluster (< 2) ❌
Result: EJECTED (fails BOTH Trigger 2 AND Trigger 3)
```

**Interpretation**: When a voucher flags you, you lose not just the vouch count but potentially cluster diversity too. In this case, Alice's flag causes TWO trigger violations simultaneously. The member would need a new vouch from a different cluster (not Cluster B) to re-enter.

## Documentation Impact

### Canonical Sources (Beads)
These beads are the authoritative sources for vouch invalidation logic:

| Bead | Content |
|------|---------|
| `.beads/security-constraints.bead` | Vouch invalidation rules, 2-point swing prevention, ejection triggers |
| `.beads/terminology.bead` | Trust calculation definitions (Effective_Vouches, Regular_Flags, Standing) |
| `.beads/vetting-protocols.bead` | Ejection protocol overview, standing calculation |
| `.beads/architectural-decisions-open.bead` | Decision #10: Vouch invalidation rationale |

### Derived Documents Updated
1. ✅ `README.md` - Updated Standing math examples with voucher-flagger cases
2. ✅ `.cursor/rules/architecture-objectives.mdc` - Updated Trust Standing section
3. ✅ `.cursor/rules/freenet-contract-design.mdc` - Updated helper methods
4. ✅ `.cursor/rules/freenet-integration.mdc` - Updated contract verify() logic
5. ✅ `.cursor/rules/signal-integration.mdc` - Updated ejection protocol
6. ✅ `.cursor/rules/user-roles-ux.mdc` - Updated bot messages and examples
7. ✅ `.cursor/rules/vetting-protocols.mdc` - Added effective vouches terminology
8. ✅ `docs/todo/TODO.md` - Updated Phase 1 success criteria
9. ✅ `docs/spike/SPIKE-WEEK-BRIEFING.md` - Added trust model refinement
10. ✅ `docs/VOUCH-INVALIDATION-LOGIC.md` - This document (comprehensive explanation)



## Implementation Checklist

### Phase 1 (Bootstrap & Core Trust)
- [ ] Implement calculate_effective_state() in contract
- [ ] Update should_eject() to check ALL THREE triggers:
  - [ ] Standing < 0
  - [ ] Effective_Vouches < min_vouch_threshold
  - [ ] Cross-cluster violation (see `.beads/cross-cluster-requirement.bead`)
- [ ] Update contract verify() to check effective vouches
- [ ] Add unit tests for vouch invalidation scenarios
- [ ] Add property tests for voucher-flagger intersection logic

### Bot UX
- [ ] Update /status command to show voucher-flaggers
- [ ] Update /flag command to warn if flagger is also voucher
- [ ] Update ejection messages to explain vouch invalidation
- [ ] Show effective vouches vs total vouches in bot responses
- [ ] Show cluster diversity status in /status command

### Testing
- [ ] Test all 3 edge cases documented above
- [ ] Test vouch bombing attack prevention
- [ ] Test that vouch invalidation is commutative (merge order independent)
- [ ] Property test: effective_vouches <= total_vouches (always true)
- [ ] Test interaction between vouch invalidation and cross-cluster requirement

## Summary

**Critical Refinement**: Vouch invalidation when voucher flags is a fundamental improvement to the trust model.

**Key Formula**:
```
Effective_Vouches = Total_Vouches - |Vouchers ∩ Flaggers|
Regular_Flags = Total_Flags - |Vouchers ∩ Flaggers|
Standing = Effective_Vouches - Regular_Flags
```

**Ejection Triggers** (Three Independent):
1. **Standing < 0**: `Effective_Vouches - Regular_Flags < 0` (strictly less than zero)
2. **Effective_Vouches < min_vouch_threshold**: Default 2, configurable
3. **Cross-cluster violation**: Vouches from < 2 clusters (when 2+ clusters exist)

**Note**: This document focuses on Triggers 1 and 2. See `.beads/cross-cluster-requirement.bead` for Trigger 3 (cross-cluster) which operates independently of vouch invalidation.

**Result**: Logically consistent, secure against gaming, aligns with fluid identity philosophy, reflects relationship dynamics.

This refinement makes Stroma's trust model **stronger, more consistent, and more resilient**.
