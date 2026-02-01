# Q14: Chunk Communication Protocol

**Risk Level**: 🟡 RECOVERABLE
**Status**: COMPLETE

---

## WHY This Question Matters

**The Core Problem**: Chunks must be distributed to 16+ holders per state update. The communication mechanism affects cost, latency, and complexity of the persistence network.

**The Dilemma**:
```
Bot-A writes state update (512KB = 8 chunks, 2 replicas each)
  ↓
Must distribute 16 chunks to 16 different holders
  ↓
Option A: Use Freenet contracts for everything
  - Leverages existing Freenet infrastructure
  - But: High write cost, slower latency
  ↓
Option C: Hybrid (P2P + attestation)
  - Fast, low-cost P2P for chunk data
  - Freenet contracts only for small attestations
  - But: More complex, two mechanisms
```

**Connection to Goal**: "Chunks must REACH holders reliably." Without efficient distribution, persistence network has prohibitive overhead.

---

## The Question

**How do bots transmit chunks to holders?**

### Key Constraints

1. **Performance**: Distribution must complete in < 10s for 512KB state
2. **Cost**: Must be affordable for typical update frequency (~10-100/month)
3. **Reliability**: Attestations must prove chunk delivery
4. **Simplicity**: Prefer simple mechanisms for Phase 0

---

## Protocol Options

### Option A: Freenet Contract-Based

**Mechanism**: Each bot has a "chunk inbox" contract where holders receive chunks.

```rust
// Bot A writes chunk to Bot B's storage contract
async fn distribute_via_contract(receiver: &Bot, chunk: &Chunk) -> Attestation {
    freenet.put_contract(receiver.storage_contract, chunk).await;
    Attestation::new(receiver, chunk.hash())
}
```

**Pros**:
- Leverages Freenet primitives
- No separate P2P layer needed
- Contract storage is persistent
- Well-integrated with Freenet security model

**Cons**:
- Expensive (16 contract writes per state update)
- Slower latency (~100ms per write)
- Higher operational cost

### Option B: Direct P2P

**Mechanism**: Bots connect directly via Freenet network layer.

```rust
// Bot A sends chunk directly to Bot B
async fn distribute_p2p(receiver: &Bot, chunk: &Chunk) -> Attestation {
    let connection = freenet.connect_to_peer(receiver.address).await;
    connection.send(chunk).await;
    Attestation::new(receiver, chunk.hash())
}
```

**Pros**:
- Fast, efficient
- Low cost (no contract writes)

**Cons**:
- NAT traversal complexity
- Address discovery needed
- Connection management overhead
- Not investigated in this spike (complexity)

### Option C: Hybrid (P2P + Attestation)

**Mechanism**: Use P2P for chunk data, Freenet contracts for attestations only.

```rust
// Fast P2P transfer + small attestation write
async fn distribute_hybrid(receiver: &Bot, chunk: &Chunk) -> Attestation {
    // 1. Send chunk via P2P (fast, 64KB data)
    p2p_network.send(receiver.p2p_address, chunk).await;

    // 2. Write small attestation to contract (< 100 bytes)
    let attestation = Attestation::new(receiver, chunk.hash());
    freenet.put_contract(receiver.attestation_contract, attestation).await;

    attestation
}
```

**Pros**:
- Best of both worlds
- Fast transfer (P2P ~20ms vs contract ~100ms)
- Lower cost (16 small attestations vs 16 large chunks)
- Attestations are verifiable

**Cons**:
- Two mechanisms to implement
- More complex than Option A
- P2P address management

---

## Test Scenarios

### Test 1: Contract-Based Distribution

**Setup**: Simulate Freenet with 100ms write latency, cost 10 units per write.

```rust
let distributor = ChunkDistributor::new(bot_a, contract_store);
let chunk = create_test_chunk(64 * 1024); // 64KB

let start = Instant::now();
let attestation = distributor.distribute_via_contract(bot_b, chunk).await;
let latency = start.elapsed();
```

**Expected**: ~100ms latency, cost 10 units, attestation is valid.

### Test 2: Hybrid Distribution

**Setup**: P2P has 20ms latency, cost 1 unit per transfer.

```rust
let distributor = ChunkDistributor::new(bot_a, contract_store, p2p_network);
let chunk = create_test_chunk(64 * 1024); // 64KB

let start = Instant::now();
let attestation = distributor.distribute_hybrid(bot_b, chunk).await;
let latency = start.elapsed();
```

**Expected**: ~20ms latency (5x faster), cost 1 unit (10x cheaper), attestation is valid.

### Test 3: Full State Distribution Comparison

**Setup**: 512KB state = 8 chunks, 2 replicas each = 16 distributions.

```rust
let holders = vec![bot_2, bot_3]; // 2 replicas
let chunks: Vec<_> = (0..8).map(|_| create_test_chunk(64 * 1024)).collect();

// Option A: Contract-based
let (latency_a, cost_a) = distributor.distribute_state_update(
    holders.clone(), chunks.clone(), false
).await;

// Option C: Hybrid
let (latency_c, cost_c) = distributor.distribute_state_update(
    holders.clone(), chunks.clone(), true
).await;
```

**Expected**:
- Option A: ~1.6s latency (16 × 100ms), cost 160 units
- Option C: ~320ms latency (16 × 20ms), cost 16 units
- Hybrid is 5x faster and 10x cheaper

### Test 4: Parallel Distribution

**Setup**: Test parallel distribution to 16 holders.

**Expected**: With proper parallelization, latency approaches single-request latency (~100ms for contracts, ~20ms for P2P).

### Test 5: Attestation Verification

**Setup**: Test attestation validity and tamper detection.

```rust
let attestation = DistributionAttestation::new(bot_a, bot_b, &chunk);

assert!(attestation.is_valid()); // Within 1 hour
assert!(attestation.verify_chunk(&chunk)); // Hash matches

let tampered_chunk = modify_chunk(chunk);
assert!(!attestation.verify_chunk(&tampered_chunk)); // Detect tampering
```

**Expected**: Attestations correctly validate chunks and detect tampering.

### Test 6: Scalability Analysis

**Setup**: Test different state sizes and holder counts.

| Scenario | State Size | Holders | Chunks | Expected Latency (Hybrid) |
|----------|------------|---------|--------|---------------------------|
| Small | 50KB | 10 | 1 | ~200ms |
| Medium | 512KB | 16 | 8 | ~320ms |
| Large | 2MB | 32 | 32 | ~640ms |

**Expected**: All scenarios complete in < 10s requirement.

---

## Cost Analysis

### Option A: Freenet Contract-Based

**Per state update** (512KB = 8 chunks × 64KB, 2 replicas each = 16 distributions):

```
16 contract writes
  × 64KB per write
  × [Freenet write cost per KB]
= ~16 × 10 cost units (simulated)
= 160 cost units
```

**Latency** (with parallelization):
```
Max(16 parallel writes) ≈ 100ms
```

### Option C: Hybrid (P2P + Attestation)

**Per state update**:

```
Data transfer:
  16 P2P transfers
  × 64KB per transfer
  × [P2P cost per KB]
  = ~16 × 1 cost units (simulated)
  = 16 cost units

Attestations:
  16 attestation writes
  × ~100 bytes per attestation
  × [Freenet write cost per KB]
  = ~16 × 0.1 cost units (negligible)
  = 1.6 cost units

Total: ~18 cost units
```

**Latency** (with parallelization):
```
Max(16 parallel P2P transfers) ≈ 20ms
+ Max(16 parallel attestation writes) ≈ 100ms
= ~120ms (can overlap for further optimization)
```

**Cost Reduction**: 160 / 18 = **8.9x cheaper**
**Latency Improvement**: 100ms / 20ms = **5x faster** (data transfer)

---

## Success Criteria

| Criterion | Requirement | Option A | Option C |
|-----------|-------------|----------|----------|
| Distribution latency | < 10s for 512KB | ✅ ~1.6s | ✅ ~320ms |
| Cost | Acceptable for 10-100/month | ⚠️ High | ✅ Low |
| NAT traversal | Works across NAT | ✅ (via Freenet) | ⚠️ (needs testing) |
| Attestations | Verifiable | ✅ | ✅ |
| Implementation complexity | Simple for Phase 0 | ✅ Simple | ⚠️ Complex |

---

## Freenet API Analysis

### Message Passing Availability

**Question**: Does Freenet support message passing between bots?

**Investigation Required**:
1. Freenet network layer capabilities
2. Peer-to-peer connection APIs
3. NAT traversal support

**Assumed for Spike**: Freenet provides network layer abstraction. If not available, fall back to Option A (contract-only) for Phase 0.

---

## Recommendation

### Phase 0: Option A (Contract-Based)

**Rationale**:
- ✅ Simple implementation (single mechanism)
- ✅ Leverages Freenet primitives
- ✅ Meets performance requirements (< 10s)
- ✅ No P2P layer needed
- ⚠️ Higher cost acceptable for initial deployment

**Trade-off**: Accept higher cost and latency for simplicity during Phase 0 validation.

### Phase 1+: Option C (Hybrid)

**Rationale**:
- ✅ 5x faster, 9x cheaper
- ✅ Scales better for frequent updates
- ✅ Reduces Freenet operational costs
- ⚠️ Requires P2P layer implementation

**Migration Path**:
1. Implement P2P address discovery in registry
2. Add P2P transfer layer
3. Modify distribution to use hybrid approach
4. Monitor cost/latency improvements

---

## Fallback Strategy (NO-GO)

If Freenet contracts prove too expensive or slow:

**Manual Holder Selection**
- Operators manually select reliable holders
- Reduce replication factor (2 → 1)
- Batch updates (daily vs hourly)

**Impact**: Acceptable for Phase 0. Reduces network overhead at cost of availability.

---

## Files

- `main.rs` - Test harness with distribution protocols
- `RESULTS.md` - Findings and cost/latency analysis

## Related

- [SPIKE-WEEK-2-BRIEFING.md](../SPIKE-WEEK-2-BRIEFING.md) - Full context
- [Q7: Bot Discovery](../q7/RESULTS.md) - Registry for address lookup
- [Q12: Chunk Size](../q12/RESULTS.md) - 64KB chunk size
- [Q13: Fairness Verification](../q13/RESULTS.md) - Attestation verification
