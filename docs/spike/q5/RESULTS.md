# Q5 Spike Results: Merkle Tree Performance

**Date**: 2026-01-30  
**Status**: COMPLETE  
**Decision**: GO - Generate on demand

## Summary

The spike benchmarked on-demand Merkle tree generation from BTreeSet at various member counts. Results show extremely fast performance, well below the threshold.

## Benchmark Results

| Members | Tree Build (ms) | Root Only (ms) | Proof Gen (ms) | Verify (ms) |
|---------|-----------------|----------------|----------------|-------------|
| 10 | 0.001 | 0.001 | 0.000 | 0.000 |
| 100 | 0.014 | 0.010 | 0.000 | 0.001 |
| 500 | 0.071 | 0.045 | 0.001 | 0.001 |
| 1000 | 0.140 | 0.090 | 0.002 | 0.001 |
| 2000 | 0.285 | 0.178 | 0.000 | 0.001 |
| 5000 | 0.723 | 0.447 | 0.007 | 0.001 |

## Decision Criteria Results

At **1000 members** (target benchmark):
- Root calculation: **0.090ms**
- Full tree build: **0.140ms**

**Threshold**: < 100ms = GO

**Result**: 0.090ms is **1000x faster** than the threshold.

## Decision: GO - Generate on Demand

### Rationale

1. **Extremely Fast**: Even at 5000 members, root calculation takes only 0.447ms
2. **Sub-millisecond**: All operations complete in under 1ms for typical group sizes
3. **No Caching Needed**: Performance is fast enough to regenerate on every request
4. **Stateless Design**: Simplifies bot architecture (no cache invalidation logic)

### Performance Scaling

- **100 → 1000 members**: 9.3x slowdown (expected O(n log n))
- **1000 → 5000 members**: 5.0x slowdown (sub-linear, good)

### Memory Overhead

Full tree memory usage is reasonable:
- 100 members: ~9 KB
- 1000 members: ~93 KB
- 5000 members: ~468 KB

For most operations, only the root hash (32 bytes) is needed.

## Architectural Implications

### Implementation Pattern

```rust
// For verification checks: calculate root on demand
let root = calculate_root(&members)?;

// For proof generation: build full tree only when needed
let tree = MerkleTree::from_btreeset(&members)?;
let proof = tree.generate_proof(&member_hash)?;
```

### Integration with Stroma

1. **Root Calculation**: On-demand for all membership verification
2. **Proof Generation**: On-demand when ZK-proof construction needed
3. **No Caching**: Stateless design, regenerate as needed
4. **Memory**: Full tree only held during proof operations, then dropped

## Fallback (Not Needed)

The caching fallback is **not needed** given the excellent performance.

If future requirements change (e.g., 100,000+ members), caching strategies include:
- Cache root hash, invalidate on membership change
- Incremental Merkle tree updates
- Store tree in bot memory between requests

## Files

- `merkle.rs` - Merkle tree implementation
- `main.rs` - Benchmark runner
- `README.md` - Spike overview
- `RESULTS.md` - This file

## Next Steps

1. Integrate Merkle tree module into Stroma trust verification
2. Use root calculation for membership proofs
3. Generate proofs on-demand for ZK circuit input
4. Consider cryptographically secure hash (ring::digest::SHA256) for production
