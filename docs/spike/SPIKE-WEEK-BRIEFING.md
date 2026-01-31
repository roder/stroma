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
| Q3: Cluster Detection | 🟡 RECOVERABLE | ✅ COMPLETE (GO) | Bridge Removal algorithm works |
| Q4: STARK in Wasm | 🟡 RECOVERABLE | ✅ COMPLETE (PARTIAL) | Bot-side verification |
| Q5: Merkle Tree Performance | 🟢 RECOVERABLE | ✅ COMPLETE (GO) | On-demand generation (0.09ms) |
| Q6: Proof Storage | ⚪ DECISION | ✅ COMPLETE | Store outcomes only |

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

## Q3: Cluster Detection (🟡 RECOVERABLE) — ✅ COMPLETE

**Question**: Does Union-Find distinguish tight clusters connected by bridges?

**Why Matters**: Cross-cluster vouching is mandatory. Same-cluster vouches are rejected. If detection fails, admission breaks.

### ✅ RESULT: GO — Bridge Removal Algorithm

**Finding**: Standard Union-Find fails (sees 1 cluster), but **Bridge Removal** (Tarjan's algorithm) successfully separates tight clusters.

**Test Results**:
| Algorithm | Clusters Found | Status |
|-----------|----------------|--------|
| Union-Find (baseline) | 1 | Expected |
| Mutual Vouch | 1 | NO-GO |
| Bridge Removal | 3 (A, B, Charlie) | **GO** |

**Recommended Algorithm**: Bridge Removal (Tarjan's)
- Identifies articulation edges (bridges)
- Removes bridges to find tight components
- Charlie becomes isolated bridge node

**Architectural Implications**:
- Use Bridge Removal for cross-cluster enforcement
- Bridge members can vouch but don't form tight cluster
- No fallback needed (algorithm works)

**See**: `q3/RESULTS.md` for full analysis.

---

## Q4: STARK Verification in Wasm (🟡 RECOVERABLE) — ✅ COMPLETE

**Question**: Can winterfell compile to Wasm and verify proofs performantly?

### ✅ RESULT: PARTIAL — Bot-Side Verification

**Finding**: winterfell Wasm support is experimental. Recommend **bot-side verification** for Phase 0.

**Rationale**:
1. winterfell designed for native (x86, ARM) with SIMD
2. Wasm support is not primary target
3. Bot-side verification is functional and secure
4. Can migrate to contract-side when Wasm improves

**Bot-Side Flow**:
```
Member generates proof → Bot receives → Bot verifies (native) → Submit outcome to Freenet
```

**Security Note**: Bot-side is NOT trustless. Acceptable for Phase 0; multi-bot consensus for Phase 1+.

**See**: `q4/RESULTS.md` for full analysis.

---

## Q5: Merkle Tree Performance (🟢 RECOVERABLE) — ✅ COMPLETE

**Question**: How fast is on-demand Merkle Tree generation from BTreeSet?

### ✅ RESULT: GO — Generate On Demand

**Benchmark Results** (1000 members):
| Operation | Time |
|-----------|------|
| Root calculation | 0.090ms |
| Full tree build | 0.140ms |

**Decision**: 0.090ms is **1000x faster** than the 100ms threshold.

**Scaling** (release build):
| Members | Root (ms) | Tree (ms) |
|---------|-----------|-----------|
| 100 | 0.010 | 0.014 |
| 1000 | 0.090 | 0.140 |
| 5000 | 0.447 | 0.723 |

**Architectural Implications**:
- Generate Merkle root on-demand (no caching needed)
- Build full tree only when proof generation required
- Stateless design (simpler bot architecture)

**See**: `q5/RESULTS.md` for full analysis.

---

## Q6: Proof Storage (⚪ DECISION) — ✅ COMPLETE

**Not a test** — design decision based on Q4 answer.

### ✅ RESULT: Store Outcomes Only

Since Q4 = Bot-side verification:

| Decision | Rationale |
|----------|-----------|
| Store outcomes only | Proofs are ephemeral, contract stores "Alice vouched for Bob" |
| Don't store proofs | Large (10-100KB), not needed after verification |

**Contract State**:
```rust
pub struct StromaContractState {
    members: BTreeSet<Hash>,           // Active members
    vouches: HashMap<Hash, HashSet<Hash>>,  // Vouch graph
    flags: HashMap<Hash, HashSet<Hash>>,    // Flag graph
    // No proof storage
}
```

**See**: `q6/RESULTS.md` for full analysis.

---

## Execution Plan — ✅ ALL COMPLETE

### Day 1: Q1 (BLOCKING) — ✅ COMPLETE
- **RESULT**: GO — commutative merges work with set-based state
- **Deliverable**: `q1/RESULTS.md`

### Day 2: Q2 (BLOCKING) — ✅ COMPLETE  
- **RESULT**: GO — contract can enforce invariants (trustless model viable)
- **Deliverable**: `q2/RESULTS.md`

### Day 3: Q3 (RECOVERABLE) — ✅ COMPLETE
- **RESULT**: GO — Bridge Removal algorithm works
- **Deliverable**: `q3/RESULTS.md`

### Day 3: Q5 (RECOVERABLE) — ✅ COMPLETE
- **RESULT**: GO — 0.09ms for 1000 members (on-demand OK)
- **Deliverable**: `q5/RESULTS.md`

### Day 4: Q4 (RECOVERABLE) — ✅ COMPLETE
- **RESULT**: PARTIAL — Bot-side verification for Phase 0
- **Deliverable**: `q4/RESULTS.md`

### Day 4: Q6 (DECISION) — ✅ COMPLETE
- **RESULT**: Store outcomes only (not proofs)
- **Deliverable**: `q6/RESULTS.md`

### Day 5: Documentation — ✅ COMPLETE
- All questions answered
- Architecture decisions documented
- Ready for Phase 0

**Signal Bot**: Not spiked. Presage is proven tech — proceed to Phase 0.

---

## Go/No-Go Criteria — ✅ PROCEED TO PHASE 0

### ✅ BLOCKING Questions: PASSED
- Q1: Freenet conflict resolution ✅ GO
- Q2: Contract validation ✅ GO

### ✅ RECOVERABLE Questions: ANSWERED
- Q3: Cluster detection ✅ GO (Bridge Removal)
- Q4: STARK Wasm ✅ PARTIAL (Bot-side)
- Q5: Merkle performance ✅ GO (On-demand)
- Q6: Proof storage ✅ DECIDED (Outcomes only)

### Architecture Adjustments Applied:
- Q4 → Bot-side verification (Phase 0), contract-side later
- Q6 → Store outcomes only (not proofs)

### ❌ No Blocking Issues Found
All BLOCKING questions passed. Architecture remains sound.

---

## Risks & Mitigations — ✅ ALL MITIGATED

| Risk | Result | Resolution |
|------|--------|------------|
| Freenet conflicts (Q1) | ✅ GO | Set-based CRDT works |
| Contract validation (Q2) | ✅ GO | Two-layer validation |
| Cluster detection (Q3) | ✅ GO | Bridge Removal algorithm |
| STARK Wasm (Q4) | PARTIAL | Bot-side verification (Phase 0) |
| Merkle performance (Q5) | ✅ GO | On-demand (0.09ms) |
| freenet-core maturity (Q1) | ✅ GO | SimNetwork works well |

**DVR Optimization**: Q3 cluster detection works, so Blind Matchmaker DVR can use optimal approach.

---

## Next Steps — ✅ SPIKE COMPLETE

**Spike Week is COMPLETE. Proceed to Phase 0.**

1. ✅ All questions answered (Q1-Q6)
2. ✅ Architecture decisions documented
3. Next: Complete **Pre-Gastown Audit** ([checklist](../todo/PRE-GASTOWN-AUDIT.md))
4. Next: Begin Phase 0 implementation

**Outcome**: Proceed to Phase 0 with:
- Trustless contract validation (Q2)
- Bridge Removal for cluster detection (Q3)
- Bot-side STARK verification (Q4, upgrade later)
- On-demand Merkle generation (Q5)
- Store outcomes only (Q6)

---

## Spike Validation Checklist (For Future Spikes)

Before marking a spike COMPLETE, validate against these questions:

### Architecture Alignment
- [ ] **User journey traced**: Start from Signal command → trace through bot → Freenet → outcome
- [ ] **UX boundary respected**: Members interact ONLY via Signal commands (no proof generation, no crypto)
- [ ] **Bot is sole crypto actor**: All ZK/STARK/Merkle operations happen inside bot
- [ ] **Cross-referenced USER-GUIDE.md**: Workflow matches documented user experience

### Technical Correctness
- [ ] **Who generates?** Confirm proof/tree/hash generation happens in bot, not member
- [ ] **Who verifies?** Confirm verification location (bot vs contract)
- [ ] **Who stores?** Confirm storage location (Freenet contract state)
- [ ] **Who transmits?** Confirm data flows (member→bot: commands only; bot→Freenet: outcomes)

### Constraint Verification
- [ ] **Reviewed security-guardrails.mdc**: No UX boundary violations
- [ ] **Reviewed architecture-objectives.mdc**: Aligns with Trust Logic Layer description
- [ ] **No new member requirements**: Members don't need new software, keys, or actions

**Rationale**: This checklist exists because Q4/Q6 spikes initially described members generating proofs — violating Stroma's core UX principle that Signal commands are the only member interface.
