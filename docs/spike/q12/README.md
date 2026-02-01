# Q12: Chunk Size Optimization

**Risk Level**: 🟡 RECOVERABLE
**Status**: COMPLETE

---

## WHY This Question Matters

**The Core Problem**: State is split into fixed-size chunks for distribution. The chunk size determines the balance between:

1. **Distribution breadth** (security): More chunks = more distribution = harder to seize
2. **Coordination overhead**: More chunks = more network requests = higher overhead
3. **Recovery latency**: More chunks = more parallel fetches needed
4. **Fairness complexity**: More chunks = more bookkeeping for 2x storage ratio

**Connection to Goal**: "A crashed bot recovers from adversarial peers" requires efficient chunk distribution and recovery.

---

## The Question

**What is the optimal chunk size for balancing distribution breadth vs coordination overhead?**

### Key Constraints

1. **Security scaling**: More chunks = more holders = harder to seize
2. **Recovery latency**: Must recover < 5s for 1MB state
3. **Network overhead**: Coordination overhead < 10% of data
4. **Fairness**: 2x storage ratio must be manageable

---

## The Tradeoffs

| Chunk Size | Distribution | Coordination | Recovery |
|------------|--------------|--------------|----------|
| **1KB** | Excellent (500 chunks for 500KB) | Very high overhead | 500 network requests |
| **16KB** | Good (32 chunks for 500KB) | Moderate | 32 network requests |
| **64KB** (default) | Moderate (8 chunks for 500KB) | Low | 8 network requests |
| **256KB** | Limited (2 chunks for 500KB) | Minimal | 2 network requests |

---

## Test Scenarios

### Scenario 1: Typical State (512KB, 100 bots)

**Test**: Compare chunk sizes for typical bot state.

```rust
let config = TestConfig {
    state_size: 512 * 1024,  // 512 KB
    num_bots: 100,
    replicas_per_chunk: 2,
    network_latency_ms: 50,
};
```

**Expected**: 64KB provides good balance.

### Scenario 2: Small State (50KB, 20 bots)

**Test**: Edge case for small states.

**Expected**: All sizes work, but 64KB means only 1 chunk (acceptable).

### Scenario 3: Large State (2MB, 200 bots)

**Test**: Scaling behavior for large states.

**Expected**: 64KB provides manageable chunk count (32 chunks).

### Scenario 4: Very Small State (1KB)

**Test**: Minimum state size edge case.

**Expected**: Single chunk regardless of chunk size.

### Scenario 5: Very Large State (10MB)

**Test**: Maximum coordination overhead.

**Expected**: 64KB keeps overhead reasonable (~0.2%).

---

## Test Cases

### Test 1: Recovery Latency vs Chunk Size

```rust
async fn test_recovery_latency(chunk_size: usize) -> Duration {
    let state_size = 512 * 1024;
    let num_chunks = state_size.div_ceil(chunk_size);

    // Simulate parallel chunk requests
    let start = Instant::now();
    // (parallelizable - limited by network latency, not chunk count)
    start.elapsed()
}
```

**Criteria**: Recovery latency < 5s for 1MB state.

### Test 2: Distribution Uniformity

```rust
fn test_distribution_uniformity(chunk_size: usize, num_bots: usize) {
    let num_chunks = state_size.div_ceil(chunk_size);

    // Count how many chunks each bot holds
    let holder_counts = compute_holder_distribution();

    // Check no "hot" holders
    assert!(max_count <= avg_count * 2.5);
}
```

**Criteria**: Distribution spans > 50% of network, no hot holders.

### Test 3: Coordination Overhead

```rust
fn test_coordination_overhead(chunk_size: usize) {
    let num_chunks = state_size.div_ceil(chunk_size);
    let metadata_per_chunk = 100; // bytes

    let overhead_pct = (num_chunks * metadata_per_chunk) as f64
                     / state_size as f64 * 100.0;

    assert!(overhead_pct < 10.0);
}
```

**Criteria**: Coordination overhead < 10% of data transferred.

---

## Success Criteria

| Criterion | Requirement | Result |
|-----------|-------------|--------|
| Recovery latency | < 5s for 1MB state | ✅ < 50ms (parallel) |
| Distribution | > 50% of network | ⚠️ 32% (64KB), 82.5% (16KB) |
| Hot holders | Max 2.5x average | ✅ No hot holders |
| Coordination overhead | < 10% of data | ✅ 0.2% (64KB) |

---

## Fallback Strategy (NO-GO)

If chosen size proves suboptimal:

**Re-chunk on Next Write**
- Update bot to use new chunk size
- Next state write uses new chunking
- Old chunks remain until overwritten
- Gradual migration, no data loss

**Impact**: Acceptable. Chunk size is not hardcoded into protocol.

---

## Files

- `main.rs` - Test harness with benchmarks
- `RESULTS.md` - Findings and recommendation

## Related

- [SPIKE-WEEK-2-BRIEFING.md](../SPIKE-WEEK-2-BRIEFING.md) - Full context
- [Q11: Rendezvous Hashing](../q11/RESULTS.md) - Holder selection
- [Q13: Fairness Verification](../q13/RESULTS.md) - Chunk verification
