# Spike Week: Technology Risk Validation

**Duration**: 5 days  
**Objective**: Validate core technologies and answer blocking questions before Phase 0 implementation

## Context

Freenet contracts use **ComposableState** trait with summary-delta synchronization for eventual consistency. Contracts must define mergeable state structures, not simple key-value storage. This affects our entire contract design.

---

## Risk Classification

| Question | Risk Level | Status | Fallback Strategy |
|----------|-----------|--------|-------------------|
| Q1: Freenet Conflict Resolution | 🔴 BLOCKING | ✅ COMPLETE (GO) | IPFS/iroh, custom P2P, or wait for maturity |
| Q2: Contract Validation | 🔴 BLOCKING | ✅ COMPLETE (GO) | Hybrid bot/contract validation |
| Q3: Cluster Detection | 🟡 RECOVERABLE | ⏳ Pending | Proxy: "vouchers can't vouch for each other" |
| Q4: STARK in Wasm | 🟡 RECOVERABLE | ⏳ Pending | Client-side verification |
| Q5: Merkle Tree Performance | 🟢 RECOVERABLE | ⏳ Pending | Cache in bot |
| Q6: Proof Storage | ⚪ DECISION | ⏳ Pending | Depends on Q4 answer |

**Test Priority**: BLOCKING questions first. If they fail, architecture changes fundamentally.

---

## Q1: Freenet Conflict Resolution (🔴 BLOCKING) — ✅ COMPLETE

**Question**: How does Freenet merge conflicting state updates?

**Why Critical**: If merge semantics produce unpredictable or invalid states, entire Freenet approach is infeasible.

**Test Scenario**:
```
Simultaneous updates:
- Node A: Add member X (vouched by A, B)
- Node B: Remove member A (X's voucher)

Result: Does merge produce valid state?
```

### ✅ RESULT: GO

**Finding**: Freenet requires commutativity, but it's the **contract's responsibility** to implement it.

**Observed Behavior** (Scenario 1: Both Applied):
- X was added AND A was removed (set union of deltas)
- States are identical regardless of delta application order
- This is correct CRDT behavior

**Design Pattern** (set-based state with tombstones):
```rust
fn apply_delta(&mut self, delta: &Delta) {
    // 1. Apply removals first (tombstone)
    for hash in &delta.removed {
        self.active.remove(hash);
        self.removed.insert(hash.clone());
    }
    // 2. Apply additions (only if not tombstoned)
    for hash in &delta.added {
        if !self.removed.contains(hash) {
            self.active.insert(hash.clone());
        }
    }
}
```

**Architectural Implications**:
- Freenet merge = commutative set union (correct)
- Trust semantics (vouch invalidation) = Stroma's responsibility
- Bot must recalculate trust standing after every state change
- Tombstones are permanent (re-entry requires new identity hash)

**See**: `q1/RESULTS.md` for full analysis.

**Entry Points Clarified**:
| Use Case | Entry Point |
|----------|-------------|
| Spike testing | `freenet::dev_tool::SimNetwork` |
| Unit testing | `freenet::local_node::Executor::new_mock_in_memory()` |
| Production | `freenet::local_node::NodeConfig::build()` |

**Alternatives if NO-GO**: IPFS (rust-ipfs/iroh), custom P2P solution, or wait for Freenet maturity.

---

## Q2: Contract Validation (🔴 BLOCKING) — ✅ COMPLETE

**Question**: Can contracts reject invalid state transitions?

**Why Critical**: Determines if contract can enforce trust invariants (trustless) or if bot must validate (less trustless).

### ✅ RESULT: GO

**Finding**: Freenet contracts CAN enforce trust invariants through two validation hooks:

1. **`update_state()`** - Returns `Err(ContractError::InvalidUpdate)` to reject delta BEFORE application
2. **`validate_state()`** - Returns `ValidateResult::Invalid` to reject merged state

**Observed Behavior**:
```
Test: Add member with 1 vouch (invalid)
✅ REJECTED by update_state(): Member Dave has only 1 valid vouches (need >= 2)
   Dave NOT in active: true

Test: Tombstoned member re-addition
✅ REJECTED: Cannot add tombstoned member: Alice

Test: Post-removal validation
✅ INVALID detected by validate_state(): Member Bob has only 1 vouches (need >= 2)
```

**Two-Layer Validation Architecture**:
```
┌─────────────────────────────────────────────────────┐
│  Bot: Creates deltas, handles Signal, UX           │
│       Pre-validates for better error messages      │
└───────────────────┬─────────────────────────────────┘
                    │ submit delta
                    ▼
┌─────────────────────────────────────────────────────┐
│  Contract update_state(): (Layer 1)                │
│  • Validates delta (>= 2 vouches, not tombstoned)  │
│  • Returns Err(InvalidUpdate) if invalid           │
└───────────────────┬─────────────────────────────────┘
                    │ if valid
                    ▼
┌─────────────────────────────────────────────────────┐
│  Contract validate_state(): (Layer 2)              │
│  • Validates final merged state                    │
│  • Returns Invalid if invariants violated          │
└─────────────────────────────────────────────────────┘
```

**Architectural Implications**:
- **Trustless model viable**: Contract enforces invariants
- Both `update_state()` and `validate_state()` can reject
- Bot pre-validation optional (for better UX/error messages)
- Defense in depth: compromised bot → contract still rejects

**See**: `q2/RESULTS.md` for full analysis.

---

## Q3: Cluster Detection (🟡 RECOVERABLE)

**Question**: Does Union-Find distinguish tight clusters connected by bridges?

**Why Matters**: Cross-cluster vouching is mandatory. Same-cluster vouches are rejected. If detection fails, admission breaks.

**The Bridge Problem**:
```
Cluster A (tight):  Alice, Bob, Carol (all vouch each other)
Bridge:             Charlie (vouched by Carol + Dave)
Cluster B (tight):  Dave, Eve, Frank (all vouch each other)

Union-Find result: 1 cluster (all connected)
Expected: 2 clusters (A and B are distinct)

Impact: George gets vouches from Alice + Bob → REJECTED (same cluster)
```

**Test Setup**:
- 7 members as shown above
- Run: `detect_clusters(graph)`
- Check: 1 cluster or 2?

**Decision Criteria**:
| Result | Action |
|--------|--------|
| 2 clusters | ✅ Use Union-Find |
| 1 cluster | ❌ Use fallback |

**Fallback**: "Vouchers must not have vouched for each other directly"
- Simpler: no cluster algorithm needed
- Effective: blocks obvious same-cluster vouching
- Trade-off: May reject valid cases where vouchers know each other
- Effort: 30 minutes

---

## Q4: STARK Verification in Wasm (🟡 RECOVERABLE)

**Question**: Can winterfell compile to Wasm and verify proofs performantly?

**Test**:
1. Compile winterfell verifier to `wasm32-unknown-unknown`
2. If compiles: measure verification time (10KB proof)

**Decision Criteria**:
| Result | Action |
|--------|--------|
| Compiles + < 500ms | ✅ Contract-side verification |
| Compiles + > 500ms | ⚠️ Client-side for now |
| Does not compile | ❌ Client-side only |

**Fallback**: Bot verifies proofs before Freenet submission. Less trustless but functional.

---

## Q5: Merkle Tree Performance (🟢 RECOVERABLE)

**Question**: How fast is on-demand Merkle Tree generation from BTreeSet?

**Test**:
```rust
let members: BTreeSet<Hash> = (0..1000).map(|i| hash(i)).collect();
let start = Instant::now();
let root = generate_merkle_root(&members);
println!("1000 members: {:?}", start.elapsed());
```

**Decision Criteria**:
| Time | Action |
|------|--------|
| < 100ms | ✅ Generate on demand |
| 100-500ms | ⚠️ Cache in bot |
| > 500ms | ❌ Optimize or cache in contract |

**Fallback**: Cache Merkle root in bot, invalidate on membership changes.

---

## Q6: Proof Storage (⚪ DECISION)

**Not a test** — design decision based on Q4 answer.

| Q4 Result | Q6 Decision |
|-----------|-------------|
| Contract verifies | Don't store proofs (verify in contract) |
| Client verifies | Store outcomes only (bot verifies) |

---

## Execution Plan

### Day 1: Q1 (BLOCKING) — ✅ COMPLETE
- ~~Install and run freenet-core~~
- ~~Deploy simple contract~~
- ~~Test conflict resolution~~
- **RESULT**: GO — commutative merges work with set-based state

**Deliverable**: Q1 answer ✅ (see `q1/RESULTS.md`)

### Day 2: Q2 (BLOCKING) — ✅ COMPLETE
- ~~Implement validation contract with MemberState + VouchGraph~~
- ~~Test invalid delta rejection (1 vouch → rejected)~~
- ~~Test post-removal validation (vouch count drop → detected)~~
- ~~Test tombstone enforcement (re-add blocked)~~
- **RESULT**: GO — contract can enforce invariants (trustless model viable)

**Deliverable**: Q2 answer ✅ (see `q2/RESULTS.md`)

### Day 2 (continued): Q3 (RECOVERABLE)
Afternoon: Q3
- Implement Union-Find in pure Rust
- Test 7-member bridge scenario
- If fails: implement fallback

**Deliverable**: Q3 answer (4-6 hours)

### Day 3: Q5 + Q4
Morning: Q5
- Benchmark Merkle Tree generation

Afternoon: Q4
- Compile winterfell to Wasm
- Measure verification time

**Deliverable**: Q5 and Q4 answers (4-6 hours)

### Day 4: Integration + Q6
- Implement prototype Stroma contract:
  - MemberSet (BTreeSet + tombstones)
  - VouchGraph (HashMap<Hash, BTreeSet<Hash>>)
  - Validation (based on Q2)
- Deploy and test end-to-end
- Decide Q6 based on Q4

**Deliverable**: Working prototype, Q6 decision (6-8 hours)

### Day 5: Documentation
- Write Go/No-Go Decision Report
- Update `.beads/architecture-decisions.bead`
- Document architectural changes
- Prepare Pre-Gastown Audit handoff

**Deliverable**: Complete spike documentation (4-8 hours)

**Signal Bot**: Not spiked. Presage is proven tech — move to Phase 0.

---

## Go/No-Go Criteria

### ✅ Proceed to Phase 0 if:
**BLOCKING passes**:
- Q1: Freenet conflict resolution is workable
- Q2: verify() can enforce basic invariants

**RECOVERABLE answered** (fallback OK):
- Q3: Cluster detection works OR fallback chosen
- Q4: STARK Wasm status known
- Q5: Merkle performance acceptable OR caching planned

**Documentation complete**:
- All questions answered
- Prototype contract works
- Architecture decisions documented

### ⚠️ Adjust Architecture if:
- Q2 → hybrid validation (bot + contract)
- Q3 → simpler proxy (not full cluster detection)
- Q4 → client-side verification
- Q5 → caching strategy

These adjustments are **acceptable** — document and proceed.

### ❌ Evaluate Alternatives if:
- Q1: Freenet conflict resolution is broken
- freenet-core is too immature for basic operations
- Multiple BLOCKING issues compound

**Action**: Document issues, evaluate IPFS/iroh, custom P2P solution, or wait for Freenet maturity.

---

## Risks & Mitigations

| Risk | If Fails | Mitigation |
|------|----------|------------|
| Freenet conflicts (Q1, BLOCKING) | Architecture changes | Design around behavior OR switch backends |
| verify() validation (Q2, BLOCKING) | Less trustless | Hybrid: bot validates, contract stores |
| Cluster detection (Q3, RECOVERABLE) | Admission logic changes | Use simpler proxy |
| STARK Wasm (Q4, RECOVERABLE) | Less trustless | Client-side verification |
| Merkle performance (Q5, RECOVERABLE) | Bot complexity | Cache root in bot |
| freenet-core immature (Q1, BLOCKING) | Project delay | Evaluate alternatives |

**DVR Optimization Note**: Blind Matchmaker DVR (`.beads/blind-matchmaker-dvr.bead`) depends on Q3. If cluster detection fails, DVR falls back to MST algorithm (still valid, not optimal).

---

## Next Steps

After spike completion:
1. Update `.beads/architecture-decisions.bead` with Q1-Q6 answers
2. Update contract schema based on findings
3. Complete **Pre-Gastown Audit** ([checklist](../todo/PRE-GASTOWN-AUDIT.md))
4. Proceed to Phase 0 implementation

**Expected Outcome**: Phase 0 with adjustments (hybrid validation, simpler cluster detection, client-side STARK). Architecture remains sound.
