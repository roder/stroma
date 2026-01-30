# Q3 Spike Results: Cluster Detection

**Date**: 2026-01-30  
**Status**: COMPLETE  
**Decision**: GO - Bridge Removal Algorithm

## Summary

The spike tested whether cluster detection algorithms can distinguish tight clusters connected by bridges. This is critical for Stroma's cross-cluster vouching requirement.

## Test Results

### The Bridge Problem Graph

```
Cluster A (tight):  Alice ←→ Bob ←→ Carol (all vouch each other)
Bridge:             Charlie (vouched by Carol + Dave, vouches back)
Cluster B (tight):  Dave ←→ Eve ←→ Frank (all vouch each other)

Graph Statistics:
  - 7 members
  - 8 undirected edges
  - 8 mutual edges
```

### Algorithm Results

| Algorithm | Clusters Found | Expected | Result |
|-----------|----------------|----------|--------|
| Union-Find (baseline) | 1 | 1 | Expected |
| Mutual Vouch Clustering | 1 | 2+ | NO-GO |
| Edge Density (0.5-0.9) | 2-3 | 2+ | GO |
| Bridge Removal | 3 | 2+ | GO |

### Algorithm Analysis

#### 1. Standard Union-Find (Expected: Fails)
- **Result**: 1 cluster
- **Explanation**: Union-Find finds connected components. All nodes are connected via Charlie, so it sees one cluster.
- **Conclusion**: Not suitable for tight cluster detection.

#### 2. Mutual Vouch Clustering
- **Result**: 1 cluster
- **Explanation**: Charlie has mutual vouches with both Carol and Dave, creating a path of mutual vouches.
- **Conclusion**: Not suitable when bridges have bidirectional connections.

#### 3. Edge Density Analysis
- **Result**: 2-3 clusters (depending on threshold)
- **Best Threshold**: 0.7 produces clean separation (A: Alice/Bob/Carol, B: Dave/Eve/Frank, Charlie: isolated)
- **Conclusion**: Viable, but threshold tuning required.

#### 4. Bridge Removal (Tarjan's Algorithm)
- **Result**: 3 clusters (A: Alice/Bob/Carol, B: Dave/Eve/Frank, Charlie: bridge)
- **Explanation**: Identifies articulation edges (bridges) and removes them, leaving tight components.
- **Conclusion**: Best solution - automatically detects bridges without threshold tuning.

### Control Tests

| Test | Expected | Actual | Status |
|------|----------|--------|--------|
| Fully Connected (5 nodes) | 1 cluster | 1 cluster | PASS |
| Isolated Nodes (3 nodes) | 3 clusters | 3 clusters | PASS |

## Decision: GO

**Recommended Algorithm**: Bridge Removal (Tarjan's Algorithm)

### Rationale

1. **Automatic Detection**: No threshold tuning required - bridges are detected mathematically
2. **Correct Semantics**: Bridges are members who connect otherwise-disconnected communities
3. **Predictable**: Same input always produces same output
4. **Well-Understood**: Tarjan's algorithm is O(V+E), well-documented, and widely used

### Bridge Handling

In Stroma context:
- Charlie (bridge) is placed in their own "cluster"
- For admission, bridge members can vouch but don't form a tight cluster with either side
- A new member vouched by Carol (Cluster A) + Charlie (bridge) would satisfy cross-cluster requirement

### Implementation Notes

```rust
// Bridge Removal provides clean cluster separation
let clusters = cluster_by_bridge_removal(&graph);

// Result: 
//   Cluster A: ["Alice", "Bob", "Carol"]
//   Cluster B: ["Dave", "Eve", "Frank"]  
//   Bridge:    ["Charlie"]
```

## Fallback (Not Needed)

The fallback "vouchers must not have vouched for each other directly" is **not needed** since Bridge Removal works correctly.

However, it remains available as a simpler alternative if implementation complexity becomes a concern.

## Architectural Implications

1. **Cluster Detection**: Use Bridge Removal algorithm for cross-cluster enforcement
2. **Bridge Members**: Bridges are valid vouchers, don't form tight cluster with either side
3. **Admission Check**: `Cluster(Voucher_A) != Cluster(Voucher_B)` using Bridge Removal
4. **Bootstrap**: First 3-5 members exempt (single cluster exists)

## Files

- `cluster.rs` - Implementation of all algorithms
- `main.rs` - Test runner with results
- `README.md` - Spike overview
- `RESULTS.md` - This file

## Next Steps

1. Integrate Bridge Removal into Stroma trust module
2. Add cluster detection to admission flow
3. Handle bootstrap exception (single cluster)
4. Add tests for edge cases (star topology, fully connected subgraphs)
