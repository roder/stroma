# Spike Week Briefing: Critical Validation Phase

**Date**: 2026-01-26  
**Duration**: 5 days (Week 0)  
**Objective**: Validate core technologies before committing to full implementation

## Why Spike Week is Critical

We discovered that Freenet contracts require **ComposableState** trait with summary-delta synchronization. This fundamentally changes our contract design from what we initially assumed.

**Initial Assumption (WRONG)**: Freenet is a simple key-value store where we store Merkle Trees.

**Reality (CORRECT)**: Freenet uses eventual consistency with mergeable state structures. Contracts define how to merge conflicting states across the network.

## 🚨 Outstanding Questions (MUST RESOLVE)

These 5 questions fundamentally affect contract architecture and MUST be answered before Phase 0:

### Q1: STARK Proof Verification in Wasm
**Question**: Can we verify STARK proofs in contract `verify()` method without performance issues?

**Why This Matters**:
- Determines client-side vs contract-side verification strategy
- Affects trustlessness (contract verification is more trustless)
- Impacts Wasm bundle size and execution time

**Test Plan**:
1. Attempt to compile winterfell to Wasm target
2. If possible, measure verification time in Wasm context
3. Deploy test contract with STARK verification to freenet-core
4. Benchmark with sample proof

**Target**: < 100ms per proof verification

**Decision Criteria**:
- ✅ If verification < 100ms: Use contract-side verification (more trustless)
- ❌ If verification > 100ms OR can't compile to Wasm: Use client-side verification (simpler)

**Impact**: Determines which verification approach we use in architecture

---

### Q2: Proof Storage Strategy
**Question**: Should we store STARK proofs in contract state, or just store outcomes?

**Options**:
- **A**: Store proofs temporarily (verified in verify(), removed in apply_delta)
- **B**: Store proofs permanently (complete audit trail)
- **C**: Don't store proofs at all (bot verifies client-side, contract validates invariants only)

**Why This Matters**:
- Storage costs (STARKs can be large, ~50-100KB each)
- Audit trail (can we verify historical vouches?)
- Trustlessness (contract verification vs bot verification)

**Recommendation**:
- MVP: Use Option C (simplest, smallest contract state)
- Phase 4: Evaluate Options A/B for federated trust verification

**Decision Dependencies**: Depends on Q1 answer

**Impact**: Contract state schema design

---

### Q3: On-Demand Merkle Tree Performance
**Question**: How expensive is generating Merkle Tree from BTreeSet on every ZK-proof verification?

**Why This Matters**:
- Determines if we cache Merkle root or regenerate on demand
- Affects bot performance (proof generation speed)
- May require contract state changes if caching needed

**Test Plan**:
1. Implement Merkle Tree generation from BTreeSet<MemberHash>
2. Benchmark with varying sizes:
   - 10 members (baseline)
   - 100 members (typical small group)
   - 500 members (medium group)
   - 1000 members (Signal upper limit)
3. Measure generation time on modern CPU
4. Test memory allocation (potential optimization)

**Target**: < 100ms for 1000 members

**Decision Criteria**:
- ✅ If generation < 100ms: Generate on demand (no caching, simpler)
- ⚠️ If generation 100-500ms: Cache Merkle root in bot, invalidate on member changes
- ❌ If generation > 500ms: Need optimized Merkle Tree implementation or caching in contract

**Impact**: Bot architecture (caching strategy) and contract schema

---

### Q4: Freenet Conflict Resolution Semantics
**Question**: How does Freenet handle conflicts when two nodes submit incompatible updates simultaneously?

**Example Conflict**:
```
Time T:
- Node A submits: Add member X with vouches (A, B)
- Node B submits: Remove member A (X's voucher is being removed)

These updates are incompatible - how does Freenet resolve this?
```

**Why This Matters**:
- Determines if we need causal ordering or vector clocks
- Affects ejection timing (can ejection and admission race?)
- May require additional conflict resolution logic
- Impacts federation (cross-mesh update conflicts)

**Test Plan**:
1. Set up two freenet-core nodes
2. Subscribe both to same contract
3. Submit conflicting state updates from each node:
   - Node A: Add member X
   - Node B: Remove member Y (where X depends on Y)
4. Observe which update wins or how they merge
5. Document Freenet's conflict resolution behavior

**Possible Outcomes**:
- **Last-Write-Wins**: Later timestamp wins (simple but may lose data)
- **Merge Both**: Both updates applied (may create invalid state)
- **Deterministic Order**: Some deterministic rule (e.g., hash ordering)

**Decision**:
- If Freenet handles conflicts well: Use default behavior
- If conflicts cause issues: Add vector clocks, causal ordering, or pessimistic validation

**Impact**: Contract design complexity, ejection protocol timing

---

### Q5: Custom Validation Beyond ComposableState
**Question**: Can we enforce complex invariants beyond the `verify()` method in ComposableState?

**Complex Invariants We Need**:
- "Every member must have ≥2 effective vouches from members in DIFFERENT CLUSTERS"
- "Same-cluster vouches do NOT count toward admission threshold"
- "Standing = Effective_Vouches - Regular_Flags must be ≥ 0 for all active members"
- "Voucher-flaggers excluded from both counts (no 2-point swings)"
- "Config changes require version increment"
- "Vouchers must be active members (not removed)"

**Why This Matters**:
- Determines if contract can enforce ALL trust invariants
- May need bot-side validation if contract can't express complex logic
- Affects trustlessness (contract enforcement is more trustless than bot enforcement)

**Test Plan**:
1. Review freenet-core contract API documentation
2. Check if there's a separate validation hook beyond verify()
3. Implement complex validation logic in verify() method
4. Test if verify() can reject invalid states
5. Measure performance impact of complex validation

**Decision Criteria**:
- If verify() is sufficient: Enforce all invariants in contract (most trustless)
- If verify() is limited: Use hybrid approach:
  - Basic invariants in contract (vouch count, standing)
  - Complex invariants in bot (graph analysis, cluster validation)

**Impact**: Trust model enforcement, contract vs bot responsibility split

---

### Q6: Cluster Detection for Cross-Cluster Vouching (NEW - High Risk)
**Question**: How do we reliably detect clusters for cross-cluster vouching enforcement, especially during bootstrap and small group phases?

**Why This Matters**:
- Cross-cluster vouching is now a HARD REQUIREMENT (not optimization)
- Same-cluster vouches are REJECTED — this can break admission if not handled correctly
- Bootstrap groups (3-5 members) start with ONE cluster — enforcement can't begin immediately
- Need clear transition: when does enforcement activate?

**Critical Edge Cases**:
1. **Bootstrap (3 members)**: All in same cluster (triangle). Cross-cluster impossible.
2. **Small Group (4-6 members)**: May still be 1 cluster. When do we have 2?
3. **Bridge Member**: If Alice bridges Cluster-A and Cluster-B, are they still 2 clusters?
4. **Cluster Merge**: If clusters merge after admission, does this affect existing members?
5. **Isolated Member**: What if someone has 0 vouches from one cluster? (orphaned after voucher departure)

**Test Plan**:
1. Implement Union-Find cluster detection
2. Test with various group topologies:
   - 3-member triangle (should be 1 cluster)
   - 6 members in two groups of 3 (should be 2 clusters)
   - 6 members with 1 bridge person (is this 1 or 2 clusters?)
   - Linear chain (A→B→C→D→E) — how many clusters?
3. Test transition detection:
   - When does group go from 1→2 clusters?
   - What triggers enforcement activation?
4. Test admission rejection:
   - Attempt same-cluster vouch — verify rejection
   - Attempt cross-cluster vouch — verify acceptance

**Bootstrap Exception Logic**:
```rust
fn should_enforce_cross_cluster(group: &TrustGraph) -> bool {
    let clusters = detect_clusters(group);
    clusters.len() >= 2  // Only enforce when 2+ clusters exist
}

fn can_admit(invitee: Hash, voucher_1: Hash, voucher_2: Hash, group: &TrustGraph) -> Result<()> {
    if !should_enforce_cross_cluster(group) {
        // Bootstrap exception: allow same-cluster during early phase
        return Ok(());
    }
    
    let cluster_1 = find_cluster(voucher_1, group);
    let cluster_2 = find_cluster(voucher_2, group);
    
    if cluster_1 == cluster_2 {
        return Err("Second vouch must come from a different cluster");
    }
    
    Ok(())
}
```

**Decision Criteria**:
- ✅ If Union-Find reliably detects clusters: Use standard algorithm
- ⚠️ If edge cases cause issues: Define cluster more strictly (e.g., "no mutual vouch path shorter than N")
- ❌ If cluster detection too complex: Consider simpler proxy (e.g., "vouchers must not have vouched for each other")

**Fallback Options**:
1. **Simpler proxy**: "Vouchers must not have vouched for each other directly" (prevents obvious same-cluster)
2. **Configurable threshold**: `min_cluster_count` parameter (default: 2)
3. **Manual bootstrap**: Operator declares when bootstrap phase ends

**Impact**: Core admission logic, bootstrap process, user experience for small groups

**See**: `.beads/cross-cluster-requirement.bead` for full threat model

---

## Day-by-Day Breakdown

### Day 1: Freenet Setup & ComposableState Basics
- [ ] Install freenet-core (cargo install --path crates/core)
- [ ] Install freenet-scaffold (add to test Cargo.toml)
- [ ] Run freenet-core node locally
- [ ] Implement simple ComposableState (e.g., counter or simple set)
- [ ] Test summary-delta sync with two nodes
- [ ] Verify merge is commutative

**Deliverable**: Working freenet-core setup with basic ComposableState understanding

### Day 2: Stroma Contract Design & Testing
- [ ] Implement MemberSet with ComposableState
  - active: BTreeSet<MemberHash>
  - removed: BTreeSet<MemberHash> (tombstones)
  - Test merge semantics
- [ ] Implement VouchGraph with ComposableState
  - vouches: HashMap<MemberHash, BTreeSet<MemberHash>>
  - Test merge semantics
- [ ] Test on-demand Merkle Tree generation (Q3)
  - Benchmark with 10, 100, 500, 1000 members
  - Measure generation time
- [ ] Test conflict resolution (Q4)
  - Create conflicting updates
  - Observe Freenet merge behavior
- [ ] Test custom validation (Q5)
  - Implement complex invariants in verify()
  - Test rejection of invalid states
- [ ] **Test cluster detection for cross-cluster vouching (Q6)** ⚠️ HIGH RISK
  - Implement Union-Find cluster detection
  - Test bootstrap exception (1 cluster → allow same-cluster)
  - Test enforcement activation (2+ clusters → reject same-cluster)
  - Test edge cases: bridge members, cluster merges, linear chains
  - Validate admission rejection message when same-cluster attempted
- [ ] Deploy Stroma contract to freenet-core
- [ ] Test state stream monitoring

**Deliverable**: Answers to Q3, Q4, Q5, Q6 + working Stroma contract prototype

### Day 3: Signal Bot (No Changes from Original Plan)
- [ ] Register bot account with Signal
- [ ] Test group management (create, add, remove)
- [ ] Test 1-on-1 PM handling
- [ ] Test command parsing

### Day 4-5: STARK Proofs & Wasm Integration
- [ ] Set up winterfell library
- [ ] Create sample STARK circuit
- [ ] Measure proof size and generation time
- [ ] **Test STARK verification in Wasm** (Q1)
  - Attempt to compile winterfell to wasm32-unknown-unknown
  - If successful, deploy to freenet-core contract
  - Measure verification time in Wasm context
- [ ] **Decide proof storage strategy** (Q2)
  - Based on Q1 answer
  - Document decision rationale

**Deliverable**: Answers to Q1, Q2 + STARK proof validation

## Expected Outcomes

### Success Criteria
- [ ] All 6 outstanding questions answered with documented decisions
- [ ] freenet-core node runs successfully
- [ ] ComposableState contract deploys and merges correctly
- [ ] On-demand Merkle Tree generation meets performance targets
- [ ] Signal bot can manage group
- [ ] STARK proofs meet size/performance targets
- [ ] Vouch invalidation logic validated (voucher-flaggers correctly handled)
- [ ] **Cluster detection validated for cross-cluster vouching (Q6)**
  - [ ] Bootstrap exception works (1 cluster → no enforcement)
  - [ ] Enforcement activates correctly (2+ clusters → enforce)
  - [ ] Same-cluster vouches rejected with clear message
  - [ ] Edge cases documented and handled

### Trust Model Refinement (From Spike Week Discovery)
**Critical Logic**: Vouch Invalidation

If a voucher flags a member, that vouch is invalidated (logical inconsistency).

**Calculation**:
```
Effective_Vouches = All_Vouches - Voucher_Flaggers
Regular_Flags = All_Flags - Voucher_Flaggers
Standing = Effective_Vouches - Regular_Flags
```

**Why This Matters**:
- Prevents logical inconsistency (can't both trust and distrust)
- Prevents "vouch bombing" attack
- Aligns with fluid identity philosophy (trust is current state)
- Ensures vouches represent genuine ongoing trust

### Go/No-Go Decision Report
**Document**:
1. Answer to each question (Q1-Q6)
2. Decision rationale for each
3. Performance benchmarks (Merkle Tree, STARK proofs)
4. Contract design approach (based on Q1-Q6 answers)
5. Cluster detection approach for cross-cluster enforcement (Q6 answer)
6. Identified risks and mitigations
7. Recommendation: Proceed to Phase 0 or adjust architecture

## Potential Risks & Mitigations

### Risk 1: winterfell doesn't compile to Wasm
**Mitigation**: Use client-side verification (Approach 1), validate in bot before Freenet submission

### Risk 2: On-demand Merkle Tree too slow
**Mitigation**: Cache Merkle root in bot, invalidate on member changes (add to bot state)

### Risk 3: Freenet conflict resolution incompatible
**Mitigation**: Add vector clocks or restrict concurrent updates (pessimistic locking)

### Risk 4: ComposableState too limiting
**Mitigation**: Hybrid validation (basic in contract, complex in bot)

### Risk 5: freenet-core too immature
**Mitigation**: Consider alternatives (e.g., Gun.js, OrbitDB) or wait for Freenet maturity

### Risk 6: Cluster detection unreliable for cross-cluster enforcement ⚠️ HIGH RISK
**Problem**: Cross-cluster vouching is now MANDATORY. If cluster detection is unreliable:
- Small groups may get stuck (can't admit new members if clusters not detected correctly)
- Bootstrap process breaks (enforcement activates too early)
- Edge cases cause admission failures (bridge members, cluster merges)

**Mitigation Options**:
1. **Simpler proxy**: Instead of full cluster detection, use "vouchers must not have vouched for each other directly"
2. **Configurable transition**: `min_members_for_enforcement` parameter (e.g., 6)
3. **Manual bootstrap flag**: Operator declares when bootstrap phase ends
4. **Hybrid approach**: Use simple proxy for small groups, full cluster detection for larger groups

**Decision Required**: If Union-Find cluster detection is too complex for edge cases, document the fallback approach in `.beads/cross-cluster-requirement.bead`

## Integration with Main Roadmap

After Spike Week completes:
- Update `.beads/architecture-decisions.bead` with Q1-Q5 answers
- Update contract schema in Bead-05 based on findings
- Proceed to Phase 0 with validated architecture
- Outstanding questions resolved, no architectural surprises

## Success Indicators

✅ **Proceed to Phase 0** if:
- All 6 questions answered satisfactorily
- Performance targets met (Merkle Tree < 100ms, STARK < 10s)
- Contract design validated (mergeable state works)
- Cluster detection works reliably for cross-cluster enforcement
- Bootstrap exception logic validated
- No show-stopping issues discovered

⚠️ **Adjust Architecture** if:
- Major performance issues (need caching, optimization)
- ComposableState limitations (need hybrid validation)
- Conflict resolution issues (need vector clocks)
- Cluster detection edge cases require fallback (simpler proxy)

❌ **Evaluate Alternatives** if:
- freenet-core too immature or buggy
- ComposableState fundamentally incompatible with our needs
- Performance issues unsolvable
- Cross-cluster enforcement fundamentally breaks small group UX (no viable fallback)

**Expected**: Proceed to Phase 0 with some minor adjustments based on Spike Week findings.
