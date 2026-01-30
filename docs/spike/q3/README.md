# Q3: Cluster Detection Spike

**Question**: Does Union-Find distinguish tight clusters connected by bridges?

**Status**: Testing

## Background

Cross-cluster vouching is MANDATORY for Stroma admission. Same-cluster vouches are rejected to prevent coordinated infiltration. If cluster detection fails to distinguish tight clusters connected by bridges, the admission system breaks.

## The Bridge Problem

```
Cluster A (tight):  Alice ←→ Bob ←→ Carol (all vouch each other)
Bridge:             Charlie (vouched by Carol + Dave)
Cluster B (tight):  Dave ←→ Eve ←→ Frank (all vouch each other)

Standard Union-Find result: 1 cluster (all connected)
Expected for Stroma: 2 clusters (A and B are distinct social contexts)
```

## Test Approach

1. **Standard Union-Find**: Basic connected components - will fail (1 cluster)
2. **Edge Density Analysis**: Detect clusters based on internal edge density
3. **Bridge Detection**: Identify articulation points/bridges, separate at bridges

## Decision Criteria

| Result | Action |
|--------|--------|
| 2 clusters detected | GO: Use the algorithm that works |
| 1 cluster detected (all methods) | NO-GO: Use fallback proxy rule |

## Fallback Strategy

**Proxy rule**: "Vouchers must not have vouched for each other directly"

- Simpler implementation (no cluster algorithm needed)
- Blocks obvious same-cluster vouching
- Trade-off: May reject some valid edge cases where vouchers know each other

## Files

- `cluster.rs` - Cluster detection implementations
- `main.rs` - Test runner with 7-member bridge scenario
- `RESULTS.md` - GO/NO-GO decision with rationale

## Running the Spike

```bash
cargo run --bin spike-q3
```

## References

- [Union-Find Algorithm](https://en.wikipedia.org/wiki/Disjoint-set_data_structure)
- [Community Detection](https://en.wikipedia.org/wiki/Community_structure)
- [Louvain Algorithm](https://en.wikipedia.org/wiki/Louvain_method)
- `docs/ALGORITHMS.md` - Stroma graph theory foundations
- `.beads/cross-cluster-requirement.bead` - Full threat model
