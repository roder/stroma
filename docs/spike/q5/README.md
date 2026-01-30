# Q5: Merkle Tree Performance Spike

**Question**: How fast is on-demand Merkle Tree generation from BTreeSet?

**Status**: Testing

## Background

Stroma stores members in a BTreeSet (CRDT-friendly, mergeable) but needs Merkle proofs for ZK verification. The architectural decision is whether to:

1. Generate Merkle trees on-demand (simpler, stateless)
2. Cache Merkle trees in the bot (more complex, requires invalidation)
3. Store Merkle trees in the contract (most complex, storage overhead)

## Test Approach

Benchmark Merkle tree operations at various member counts:

1. **Tree Generation**: Build complete Merkle tree from BTreeSet
2. **Root Calculation**: Just compute the root hash
3. **Proof Generation**: Generate membership proof for one member
4. **Proof Verification**: Verify a membership proof

## Decision Criteria

| Time (1000 members) | Action |
|---------------------|--------|
| < 100ms | GO: Generate on demand |
| 100-500ms | PARTIAL: Cache in bot, invalidate on change |
| > 500ms | NO-GO: Optimize or cache in contract |

## Fallback Strategy

Cache Merkle root in bot, invalidate on membership changes:

```rust
struct CachedMerkle {
    root: Hash,
    tree: MerkleTree,
    version: u64,  // Invalidate when membership changes
}
```

## Files

- `merkle.rs` - Merkle tree implementation
- `main.rs` - Benchmark runner
- `RESULTS.md` - Performance results and decision

## Running the Spike

```bash
cargo run --bin spike-q5
```

## References

- [Merkle Trees](https://en.wikipedia.org/wiki/Merkle_tree)
- [ring crate](https://docs.rs/ring/latest/ring/) - SHA-256 hashing
- [sha2 crate](https://docs.rs/sha2/latest/sha2/) - Alternative SHA-256
- `docs/ALGORITHMS.md` - Merkle proof generation for ZK verification
