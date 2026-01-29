# Spike Week: Technology Risk Validation

**Duration**: 5 days  
**Objective**: Validate core technologies and answer blocking questions before Phase 0 implementation

## Context

Freenet contracts use **ComposableState** trait with summary-delta synchronization for eventual consistency. Contracts must define mergeable state structures, not simple key-value storage. This affects our entire contract design.

---

## Risk Classification

| Question | Risk Level | Fallback Strategy |
|----------|-----------|-------------------|
| Q1: Freenet Conflict Resolution | 🔴 BLOCKING | IPFS/iroh, custom P2P, or wait for maturity |
| Q2: verify() Validation | 🔴 BLOCKING | Hybrid bot/contract validation |
| Q3: Cluster Detection | 🟡 RECOVERABLE | Proxy: "vouchers can't vouch for each other" |
| Q4: STARK in Wasm | 🟡 RECOVERABLE | Client-side verification |
| Q5: Merkle Tree Performance | 🟢 RECOVERABLE | Cache in bot |
| Q6: Proof Storage | ⚪ DECISION | Depends on Q4 answer |

**Test Priority**: BLOCKING questions first. If they fail, architecture changes fundamentally.

---

## Q1: Freenet Conflict Resolution (🔴 BLOCKING)

**Question**: How does Freenet merge conflicting state updates?

**Why Critical**: If merge semantics produce unpredictable or invalid states, entire Freenet approach is infeasible.

**Test Scenario**:
```
Simultaneous updates:
- Node A: Add member X (vouched by A, B)
- Node B: Remove member A (X's voucher)

Result: Does merge produce valid state?
```

**Test Steps**:
1. Deploy simple contract with `add_member` and `remove_member`
2. Submit conflicting updates from two clients
3. Observe merge behavior

**Decision Criteria**:
| Behavior | Action |
|----------|--------|
| Deterministic + valid state | ✅ Design around behavior |
| Invalid state, no detection | ⚠️ Add vector clocks |
| Unpredictable/random | ❌ Evaluate alternatives |

**Alternatives if NO-GO**: IPFS (rust-ipfs/iroh), custom P2P solution, or wait for Freenet maturity.

---

## Q2: Contract Validation (🔴 BLOCKING)

**Question**: Can `verify()` reject invalid state transitions?

**Why Critical**: Determines if contract can enforce trust invariants (trustless) or if bot must validate (less trustless).

**Test**:
```rust
fn verify(&self, delta: &Self::Delta) -> bool {
    if let Delta::AddMember { hash, vouches } = delta {
        return vouches.len() >= 2;  // Reject if < 2
    }
    true
}
```

**Test Steps**:
1. Implement contract with validation rule
2. Submit delta with 1 vouch (invalid)
3. Check: Does network reject it?

**Decision Criteria**:
| Result | Action |
|--------|--------|
| verify() or network rejects | ✅ Contract enforces invariants |
| Invalid delta applied | ❌ Hybrid: bot validates, contract stores |

**Fallback**: Bot-side validation. Less trustless but functional.

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

### Day 1: Q1 (BLOCKING)
- Install and run freenet-core
- Deploy simple contract
- Test conflict resolution
- **STOP if unpredictable** — evaluate alternatives

**Deliverable**: Q1 answer (4-6 hours)

### Day 2: Q2 + Q3
Morning: Q2 (BLOCKING)
- Implement validation contract
- Test invalid delta rejection
- Document verify() capabilities

Afternoon: Q3 (RECOVERABLE)
- Implement Union-Find in pure Rust
- Test 7-member bridge scenario
- If fails: implement fallback

**Deliverable**: Q2 and Q3 answers (6-8 hours)

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
3. Complete **Pre-Gastown Audit** ([checklist](PRE-GASTOWN-AUDIT.md))
4. Proceed to Phase 0 implementation

**Expected Outcome**: Phase 0 with adjustments (hybrid validation, simpler cluster detection, client-side STARK). Architecture remains sound.
