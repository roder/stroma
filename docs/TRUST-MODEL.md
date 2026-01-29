# Stroma Trust Model

**Mathematical Details & Edge Cases**

This document explains how trust works in Stroma, including the vouch invalidation logic, ejection triggers, and role transitions.

## Key Terms

| Term | Meaning |
|------|---------|
| **Vouch** | A personal endorsement — you stake your reputation on someone |
| **Flag** | The opposite of a vouch — indicates you no longer trust someone |
| **Cluster** | A friend circle — people who know each other from the same social context |
| **Cross-cluster** | From *different* friend circles — required for admission and ongoing membership |
| **Effective Vouches** | Total vouches minus voucher-flaggers (vouchers who also flagged you) |
| **Regular Flags** | Flags from people who didn't vouch for you |
| **Standing** | Effective vouches minus regular flags (must stay ≥ 0) |

For general concepts, see [How It Works](HOW-IT-WORKS.md). For bot commands, see [User Guide](USER-GUIDE.md).

## Core Principle

**Trust Standing must remain ≥ 0 AND Effective Vouches must stay ≥ 2 for membership.**

**Cross-Cluster Requirement**: Vouches MUST come from as many distinct clusters as the member's vouch count (up to available clusters). Bridges (2 vouches) need 2 clusters; Validators (3+ vouches) need 3+ clusters. This prevents coordinated infiltration and ensures higher trust = more distributed verification.

**See**: `.beads/cross-cluster-requirement.bead` for full rationale

## Roles & Requirements

### Invitees (Leaf Nodes)
**Status**: OUTSIDE Signal group

- 1 vouch from member who invited them
- Being vetted (waiting for second vouch)
- Receives 1-on-1 PMs from bot
- Cannot vouch, flag, or vote

### Bridges
**Status**: IN Signal group (minimum requirement)

- Exactly 2 effective vouches from 2 different clusters
- Full member privileges
- At risk if voucher leaves OR flags
- Should build more connections for resilience

### Validators
**Status**: IN Signal group (high trust)

- 3+ effective vouches from 3+ different clusters (when available)
- Same privileges as Bridges
- More resilient to voucher changes
- Preferred by Blind Matchmaker for strategic introductions
- If only 2 clusters exist, must have vouches from both

**Note**: Validators have NO special permissions - only higher resilience and bot optimization preferences.

## Trust Standing Calculation

### The Formula

```
All_Vouchers = Set of members who vouched for you
All_Flaggers = Set of members who flagged you
Voucher_Flaggers = All_Vouchers ∩ All_Flaggers (contradictory)

Effective_Vouches = |All_Vouchers| - |Voucher_Flaggers|
Regular_Flags = |All_Flaggers| - |Voucher_Flaggers|
Standing = Effective_Vouches - Regular_Flags
```

### Vouch Invalidation (Critical Logic)

**Key Principle**: If a voucher flags a member, that vouch is invalidated.

**Rationale**: You can't simultaneously trust and distrust someone. This is a logical inconsistency that must be resolved by invalidating the vouch.

**Benefits:**
- Prevents "vouch bombing" attack (vouch then flag to manipulate standing)
- Aligns with fluid identity philosophy (trust is current state, not historical)
- Reflects relationship dynamics (trust can be revoked)
- Ensures vouches represent genuine ongoing trust

## Ejection Triggers (Two Independent Conditions)

### Critical Design Principle: NO UNILATERAL 2-POINT SWINGS

**The Rule**: No single member's action can cause another member's standing to drop by 2 points.

**Why This Matters**: Prevents weaponization. A voucher should be able to:
1. Invalidate their vouch (1-point consequence for their poor judgment)
2. File a flag to warn the group

...but these actions should NOT combine into automatic ejection through a single actor.

**How It Works**: Voucher-flaggers are excluded from BOTH vouch count AND flag count. Their flag doesn't add to the flag total; their action is treated as vouch invalidation only.

### Trigger 1: Standing < 0
**Condition**: `Effective_Vouches - Regular_Flags < 0`

**Meaning**: Too many regular flags (from non-vouchers) relative to effective vouches

**Example (Single Non-Voucher Can't Eject)**:
- Effective vouches: 3
- Regular flags: 1 (from non-voucher Carol)
- Standing: 3 - 1 = +2 ✅
- **Result**: STAYS (single flag isn't enough)

**Example (Multiple Independent Flags Can Eject)**:
- Effective vouches: 3
- Regular flags: 5 (from multiple non-vouchers)
- Standing: 3 - 5 = -2 ❌
- **Result**: EJECTED (but required multiple independent perspectives)

### Trigger 2: Effective_Vouches < min_vouch_threshold
**Condition**: `Effective_Vouches < 2` (default threshold)

**Meaning**: Dropped below minimum vouch requirement

**Causes:**
- Voucher left the group (vouch removed)
- Voucher flagged you (vouch invalidated)
- Both vouchers flagged you (both vouches invalidated)

**Example**:
- All vouches: 2 (Alice, Bob)
- All flags: 1 (Alice)
- Voucher-flaggers: 1 (Alice)
- Effective vouches: 2 - 1 = 1 ❌
- **Result**: EJECTED (even if standing ≥ 0)

## Detailed Examples

### Example 1: Simple Member (No Flags)
```
All Vouches: 2 (Alice, Bob)
All Flags: 0
Voucher-Flaggers: 0

Effective_Vouches = 2 - 0 = 2
Regular_Flags = 0 - 0 = 0
Standing = 2 - 0 = +2

Trigger 1: ✅ (2 ≥ 0)
Trigger 2: ✅ (2 ≥ 2)
Result: STAYS IN GROUP
```

### Example 2: Flagged by Non-Voucher
```
All Vouches: 2 (Alice, Bob)
All Flags: 1 (Carol)
Voucher-Flaggers: 0 (Carol didn't vouch)

Effective_Vouches = 2 - 0 = 2
Regular_Flags = 1 - 0 = 1
Standing = 2 - 1 = +1

Trigger 1: ✅ (+1 ≥ 0)
Trigger 2: ✅ (2 ≥ 2)
Result: STAYS IN GROUP
Note: Flag doesn't invalidate vouches from Alice or Bob
```

### Example 3: Flagged by One Voucher
```
All Vouches: 2 (Alice, Bob)
All Flags: 1 (Alice)
Voucher-Flaggers: 1 (Alice is in both sets)

Effective_Vouches = 2 - 1 = 1 ❌
Regular_Flags = 1 - 1 = 0
Standing = 1 - 0 = +1

Trigger 1: ✅ (+1 ≥ 0)
Trigger 2: ❌ (1 < 2)
Result: EJECTED
Reason: Alice's vouch is invalidated, leaving only 1 effective vouch
```

### Example 4: Three Vouches, One Voucher Flags
```
All Vouches: 3 (Alice, Bob, Carol)
All Flags: 1 (Alice)
Voucher-Flaggers: 1 (Alice)

Effective_Vouches = 3 - 1 = 2
Regular_Flags = 1 - 1 = 0
Standing = 2 - 0 = +2

Trigger 1: ✅ (+2 ≥ 0)
Trigger 2: ✅ (2 ≥ 2)
Result: STAYS IN GROUP
Note: Bob and Carol's vouches remain valid
```

### Example 5: Both Vouchers Flag
```
All Vouches: 2 (Alice, Bob)
All Flags: 2 (Alice, Bob)
Voucher-Flaggers: 2 (both in both sets)

Effective_Vouches = 2 - 2 = 0 ❌
Regular_Flags = 2 - 2 = 0
Standing = 0 - 0 = 0

Trigger 1: ✅ (0 ≥ 0) -- Edge case: exactly zero
Trigger 2: ❌ (0 < 2)
Result: EJECTED
Reason: Both vouchers flagged, invalidating both vouches
```

### Example 6: Many Flags, No Voucher-Flaggers
```
All Vouches: 3 (Alice, Bob, Carol)
All Flags: 5 (Dave, Eve, Frank, Grace, Hank)
Voucher-Flaggers: 0 (none of the flaggers vouched)

Effective_Vouches = 3 - 0 = 3
Regular_Flags = 5 - 0 = 5
Standing = 3 - 5 = -2 ❌

Trigger 1: ❌ (-2 < 0)
Trigger 2: ✅ (3 ≥ 2)
Result: EJECTED
Reason: Too many regular flags (standing negative)
```

### Example 7: Mixed Scenario (Both Triggers)
```
All Vouches: 2 (Alice, Bob)
All Flags: 3 (Alice, Carol, Dave)
Voucher-Flaggers: 1 (Alice)

Effective_Vouches = 2 - 1 = 1 ❌
Regular_Flags = 3 - 1 = 2
Standing = 1 - 2 = -1 ❌

Trigger 1: ❌ (-1 < 0)
Trigger 2: ❌ (1 < 2)
Result: EJECTED
Reason: BOTH triggers violated (worst case)
```

## Edge Cases

### Can Trigger 1 Pass While Trigger 2 Fails?
**Yes - Example 3 above**

- Standing = +1 (positive) ✅
- Effective vouches = 1 (below threshold) ❌
- Result: EJECTED via Trigger 2

### Can Trigger 2 Pass While Trigger 1 Fails?
**Yes - Example 6 above**

- Effective vouches = 3 (above threshold) ✅
- Standing = -2 (negative) ❌
- Result: EJECTED via Trigger 1

### What if Standing = 0 Exactly?
**Edge case**: Zero is NOT negative

- Standing = 0 → Trigger 1 PASSES (≥ 0)
- But if effective vouches < 2 → Trigger 2 FAILS
- Example 5 above: Standing = 0 but effective vouches = 0 → EJECTED via Trigger 2

### What if Someone Has Many Vouches but Also Many Flags?

**Depends on the balance:**

**Scenario A**: 10 vouches, 8 flags (none are voucher-flaggers)
- Effective vouches: 10
- Regular flags: 8
- Standing: +2 ✅
- Result: STAYS (both triggers pass)

**Scenario B**: 10 vouches, 12 flags (none are voucher-flaggers)
- Effective vouches: 10
- Regular flags: 12
- Standing: -2 ❌
- Result: EJECTED (Trigger 1)

**Scenario C**: 10 vouches, 9 flags (8 are voucher-flaggers)
- Effective vouches: 10 - 8 = 2
- Regular flags: 9 - 8 = 1
- Standing: 2 - 1 = +1 ✅
- Result: STAYS (but barely - at minimum threshold)

## Re-Entry After Ejection

### Process
1. Member invites you again (`/invite @You`)
2. Invitation counts as first vouch
3. Bot facilitates vetting with second member FROM A DIFFERENT CLUSTER
4. Second member vouches (must be cross-cluster from inviter)
5. Automatic admission when 2 cross-cluster vouches confirmed

### No Cooldown Period
You can re-enter immediately after securing 2 new vouches from different clusters.

### Previous History
**Question**: Do previous flags carry over?

**Answer**: TBD - needs design decision
- **Option A**: Fresh start (all previous vouches/flags cleared)
- **Option B**: Flags persist (but must get 2 new vouches to overcome them)

**Recommendation**: Option A for MVP (simpler, more forgiving)

## Network Topology

### Internal Clusters
Sub-communities within a single Stroma group based on social affinities.

**Example**: 50-person group with 3 clusters
- Cluster A: Artists (15 members)
- Cluster B: Engineers (20 members)
- Cluster C: Activists (15 members)

**Blind Matchmaker**: Bot suggests cross-cluster introductions for intersectional diversity

### Minimum Spanning Tree (MST)
Goal: Create fully connected mesh with minimal new interactions.

**Calculation**:
```
N = Count of Bridges (members with exactly 2 vouches)
I = Count of disconnected islands
Total introductions needed = N + I
```

**Example**: 20-person group
- 5 Bridges (need 1 more connection each)
- 2 disconnected islands (need 1 connection to main mesh)
- MST target: 5 + 2 = 7 strategic introductions

## Mesh Health Metrics

### Primary Metric: Distinct Validator Ratio (DVR)

Network health is measured by the **Distinct Validator Ratio** — a graph-theory-grounded metric that directly measures resilience against coordinated attacks.

**See**: `.beads/mesh-health-metric.bead` for full rationale.

### DVR Formula

```
DVR = Distinct_Validators / Max_Possible_Distinct_Validators

Where:
- Distinct_Validators = Validators with non-overlapping voucher sets
- Max_Possible = floor(N / 4)
- N = Total network members
```

**Why N/4?** Each distinct Validator requires ~4 members: themselves + 3 unique vouchers.

### DVR Calculation

```rust
fn calculate_distinct_validator_ratio(graph: &TrustGraph) -> f32 {
    let n = graph.member_count();
    if n < 4 { return 1.0; } // Bootstrap: too small to measure
    
    let max_possible = n / 4;
    let distinct_count = count_distinct_validators(graph);
    
    (distinct_count as f32 / max_possible as f32).min(1.0)
}

fn count_distinct_validators(graph: &TrustGraph) -> usize {
    // Greedy: select Validators whose voucher sets don't overlap
    let mut distinct = Vec::new();
    let mut used_vouchers = HashSet::new();
    
    for validator in validators_sorted_by_vouch_count_desc() {
        let vouchers = validator.voucher_set();
        if vouchers.is_disjoint(&used_vouchers) {
            distinct.push(validator);
            used_vouchers.extend(vouchers);
        }
    }
    distinct.len()
}
```

### Why DVR Instead of Density?

**Problem with density**: Arbitrary thresholds (30-60%) don't capture structure. A network can have high density but low resilience if all connections are within one cluster.

**DVR captures what matters**: How many members are verified by completely independent sets of vouchers? This directly measures resistance to coordinated infiltration.

| Metric | What It Measures | Limitation |
|--------|------------------|------------|
| **Density** | Total edges / possible edges | Structure-blind |
| **DVR** | Independent verification depth | Meaningful for security |

### Three-Tier Health Status (Thirds)

| Color | DVR Range | Status | Bot Behavior |
|-------|-----------|--------|--------------|
| 🔴 Red | 0% - 33% | **Unhealthy** | Actively suggest cross-cluster introductions |
| 🟡 Yellow | 33% - 66% | **Developing** | Suggest improvements opportunistically |
| 🟢 Green | 66% - 100% | **Healthy** | Maintenance mode |

### Example: 20-Member Network

```
Max possible distinct Validators: 20 / 4 = 5
Actual distinct Validators: 3

DVR = 3/5 = 60% → 🟡 Developing

Bot suggests: "Network health is developing. Consider
introducing [Bridge X] to [Validator Y from different cluster]
to build toward a fourth distinct Validator."
```

## Configuration Parameters

### GroupConfig Schema

```rust
pub struct GroupConfig {
    // Group identity
    group_name: String,                // "Mission Control" - changeable via consensus
    
    // Consensus thresholds
    config_change_threshold: f32,      // e.g., 0.70 (70%) - for all proposals
    default_poll_timeout: Duration,    // e.g., 48h - default if not specified

    // Federation parameters (Phase 4+)
    min_intersection_density: f32,     // e.g., 0.10-0.30
    validator_percentile: u32,         // e.g., 20 (top 20%)
    
    // Trust parameters
    min_vouch_threshold: usize,        // Default: 2
    
    // Metadata
    config_version: u64,
    last_updated: Timestamp,
}
```

### Configurable vs Fixed

**Configurable (via `/propose stroma <setting> <value>`):**
- `config_change_threshold`: Consensus required for changes (0.5-1.0)
- `min_vouch_threshold`: Minimum effective vouches to stay in group (≥2)
- `min_intersection_density`: Federation threshold (0.0-1.0)
- `validator_percentile`: Top % for validators (1-100)

**Fixed (Immutable in MVP):**
- Vouch invalidation logic (always enforced)
- Ejection triggers (two independent conditions)
- ANY Member can vouch (not restricted to Validators)
- Immediate ejection (no grace periods)

## Vouching Rules

### Who Can Vouch
**ANY Member** in the Signal group (Bridges and Validators)

**NOT restricted to Validators** - this is a critical design principle for non-hierarchical organization.

### Blind Matchmaker Optimization
Bot **prefers** Validators for strategic introductions because:
- They have more connections (better mesh topology)
- They're more resilient (less likely to leave soon)
- They create cross-cluster diversity

**But**: ANY Member can still vouch if they choose to.

### First Vouch = Invitation
Invitation itself counts as first vouch (no token exchange system).

**Flow**:
1. Member: `/invite @Friend "Context"`
2. Bot: "Invitation recorded as first vouch"
3. Bot selects second member from a DIFFERENT CLUSTER for vetting
4. Second member: `/vouch @Friend`
5. Automatic admission when 2 cross-cluster vouches confirmed

**Cross-Cluster Requirement** (CONTINUOUS): Members must maintain ≥2 vouches from different clusters at all times. Same-cluster vouches don't count toward this minimum but do count toward standing. See "Why Cross-Cluster Matters" in HOW-IT-WORKS.md.

## Flagging Rules

### Who Can Flag
**Only Members** (not Invitees)

**Rationale**: Invitees aren't in the group yet, so they can't flag members.

### Flag Effects

**If flagger is NOT a voucher**:
- Increases regular flags count
- Decreases standing
- May trigger ejection if standing < 0

**If flagger IS a voucher (voucher-flagger)**:
- Invalidates their vouch
- Decreases effective vouch count
- Decreases regular flags count (contradictory flag excluded)
- May trigger ejection if effective vouches < 2

### Re-Flagging
Can you flag someone multiple times?

**Answer**: TBD - needs design decision
- **Option A**: One flag per person (use set, not multiset)
- **Option B**: Multiple flags allowed (use multiset, counts accumulate)

**Recommendation**: Option A for MVP (simpler, prevents spam)

## Ejection Protocol

### Immediate Enforcement
- **No warnings** before ejection
- **No grace periods** to fix standing
- **No re-verification windows**
- **Automatic** - bot handles it

### Heartbeat Monitor
Bot checks trust standing every 60 minutes:
- Query Freenet for current state
- Calculate effective vouches and standing for all members
- Eject any member where either trigger is violated
- Send notifications (hashes, not names)

### Real-Time Monitoring
In addition to heartbeat, bot monitors Freenet state stream:
- Reacts to vouch/flag changes immediately
- Ejects within seconds of trigger violation
- No polling delay

### Notification Flow

**To Group**:
```
Bot → Stroma Group:
"A member's trust standing has dropped below the threshold and they 
have been automatically removed. They can re-enter by securing 2 new 
vouches from current members."
```

**To Ejected Member**:
```
Bot → Ejected Member (1-on-1 PM):
"Your trust standing has fallen below zero (Standing = -1).
- All vouches: 4
- All flags: 5
- Voucher-flaggers: 0
- Effective vouches: 4
- Regular flags: 5
- Standing: -1

You've been removed from the group. To re-enter, secure 2 new 
vouches from current members via /invite. No cooldown period."
```

**Privacy**: Uses hashes in group announcement, detailed stats only to affected member.

## Zero-Knowledge Proof Verification

### Purpose: Trust Map Protection

**Threat**: Adversary seizes bot server and captures trust map to identify members

**Defense**: ZK-proofs verify trust without exposing who vouched for whom

### What Gets Verified

For admission, bot proves:
```
Voucher_A ∈ Active_Members AND
Voucher_B ∈ Active_Members AND
Voucher_A ≠ Voucher_B AND
Cluster(Voucher_A) ≠ Cluster(Voucher_B) AND    // Cross-cluster requirement
Invitee ∉ Active_Members
```

**Cross-Cluster Enforcement**: The bot verifies that vouchers are from different clusters before admission. Same-cluster vouches are rejected with: "Second vouch must come from a different cluster than the inviter."

**Bootstrap Exception**: First 3-5 members exempt (only one cluster exists).

**Without revealing** (even if server seized):
- Who Voucher_A actually is (only hash)
- Who Voucher_B actually is (only hash)
- The complete social graph structure
- Relationship content or reasons

### STARK Proof

```rust
// Generate proof (bot-side, before submitting to Freenet)
let merkle_tree = state.generate_merkle_tree();
let voucher_a_proof = merkle_tree.generate_proof(voucher_a);
let voucher_b_proof = merkle_tree.generate_proof(voucher_b);

let stark_proof = winterfell::prove(StarkCircuit {
    merkle_root: merkle_tree.root(),
    voucher_a_proof,
    voucher_b_proof,
    invitee,
});

// Verify proof
let valid = winterfell::verify(stark_proof, public_inputs)?;
```

**Performance Targets**:
- Proof generation: < 10 seconds
- Proof size: < 100KB
- Verification: < 100ms (constant time)

## State Transitions

### Invitee → Bridge

```
State: Invitee (Outside Group)
Vouches: 1 (inviter)
Flags: 0
Effective Vouches: 1
Standing: +1

  ↓ [Second vouch received from different member]

State: Bridge (In Group)
Vouches: 2
Flags: 0
Effective Vouches: 2
Standing: +2
```

### Bridge → Validator

```
State: Bridge
Vouches: 2
Effective Vouches: 2

  ↓ [Additional vouch received]

State: Validator
Vouches: 3
Effective Vouches: 3
```

### Member → Ejected

```
State: Member
Vouches: 2
Flags: 0
Standing: +2

  ↓ [Voucher flags, invalidating their vouch]

Vouches: 2
Flags: 1
Voucher-Flaggers: 1
Effective Vouches: 1 ❌
Standing: +1
Result: EJECTED (Trigger 2)
```

### Ejected → Re-Entry

```
State: Ejected (Outside Group)

  ↓ [Member invites them back]

State: Invitee
Vouches: 1 (new invitation)

  ↓ [Second vouch received]

State: Bridge (In Group)
Vouches: 2 (fresh start)
Flags: 0 (or previous flags persist - TBD)
```

## Attack Scenarios

### Attack 1: Vouch Bombing
**Attack**: Vouch for someone, then immediately flag them to manipulate standing

**Example**:
- Attacker vouches for victim (victim now has 1 vouch)
- Attacker flags victim
- Vouch is invalidated (attacker's vouch doesn't count)
- Victim's standing doesn't change

**Defense**: Vouch invalidation prevents this attack

### Attack 2: Collusion to Eject
**Attack**: Multiple members flag someone to force ejection

**Example**:
- Victim has 3 effective vouches
- 5 members flag victim (none are vouchers)
- Effective vouches: 3 ✅
- Regular flags: 5
- Standing: 3 - 5 = -2 ❌
- Result: EJECTED (Trigger 1)

**Defense**: This is **intended behavior** - group consensus can eject members

### Attack 3: Sybil Attack
**Attack**: Attacker creates many fake identities to gain control

**Defense**:
- 2-vouch requirement from DIFFERENT CLUSTERS (cross-cluster enforced)
- Same-cluster vouches do NOT count toward admission
- Each vouch requires human interaction (vetting interview)
- STARKs verify vouchers are in Merkle Tree

**Difficulty**: Attacker must convince humans from multiple independent social contexts to vouch for fake identities — doesn't scale.

### Attack 4: Voucher Leaves After Admission
**Scenario**: Member has 2 vouches, one voucher leaves group

**Result**:
- Effective vouches: 2 - 1 = 1 ❌
- Ejection via Trigger 2

**Defense**: This is **intended behavior** - members must maintain 2 vouches at all times

**Mitigation**: Bot proactively suggests building 3+ connections (become Validator)

### Attack 5: Coordinated Infiltration
**Attack**: Bad actors rubber-stamp confederates to build an infiltration cluster

**Scenario**:
1. Alice (bad actor) joins legitimately with cross-cluster vouches
2. Alice invites confederate Bob, vouches for Bob
3. Alice's friend Carol (same cluster as Alice) tries to vouch for Bob

**Without Cross-Cluster Enforcement**:
- Bob admitted with 2 same-cluster vouches
- Repeat: infiltration cluster self-amplifies
- **Result**: Group compromised from within

**With Cross-Cluster Enforcement (CURRENT DESIGN)**:
- Carol's vouch REJECTED (same cluster as Alice)
- Bob needs vouch from member in DIFFERENT cluster
- That member has independent perspective on Bob
- **Result**: Infiltration requires deceiving multiple independent social contexts — doesn't scale

**See**: `.beads/cross-cluster-requirement.bead`

## Performance Characteristics

### Time Complexity

| Operation | Complexity | Notes |
|-----------|------------|-------|
| Calculate standing | O(V + F) | V = vouches, F = flags |
| Check ejection | O(V + F) | Two set intersections |
| Generate Merkle Tree | O(n log n) | n = members |
| STARK proof generation | O(n log n) | n = members |
| STARK verification | O(1) | Constant time |

### Space Complexity

| Data Structure | Size | Growth |
|----------------|------|--------|
| Member set | O(n) | Linear with members |
| Vouch graph | O(n²) worst case | Linear with actual vouches |
| Flag graph | O(n²) worst case | Linear with actual flags |
| Merkle Tree | O(n) | Temporary (not stored) |
| STARK Proof | ~100KB | Fixed size |

### Scalability Analysis

**Per Group**:
- 100 members: ~500 vouches (5 per member average)
- 500 members: ~2,500 vouches (5 per member average)
- 1000 members: ~5,000 vouches (5 per member average)

**Federation** (Phase 4+):
- Unlimited members across multiple groups
- Each group: up to Signal's limit (~1000)
- Scaling factor: 10²-10³ via federation

---

## See Also

- [Developer Guide](DEVELOPER-GUIDE.md) - Architecture, tech stack, workflow
- [Vouch Invalidation Logic](VOUCH-INVALIDATION-LOGIC.md) - Detailed explanation
- [Federation Roadmap](FEDERATION.md) - Phase 4+ vision
- [User Guide](USER-GUIDE.md) - For group members

---

**Last Updated**: 2026-01-27
