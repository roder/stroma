# Q11: Rendezvous Hashing for Chunk Assignment

## Question
Does rendezvous hashing provide deterministic, stable holder assignment that eliminates registry scalability bottlenecks while maintaining equivalent security?

## Decision
**GO** - Use rendezvous hashing (HRW algorithm) for deterministic chunk holder assignment.

## Key Findings

### Deterministic Assignment
- Same inputs (owner, chunk index, network bots, epoch) always produce the same holders
- No randomness or state required
- Reproducible across all network participants

### Stability Under Churn
- Only affected chunks are reassigned when bots leave the network
- When a non-holder bot leaves: 0 chunks reassigned
- When a holder bot leaves: Only chunks held by that bot are reassigned
- Graceful degradation with minimal network disruption

### Uniform Distribution
- Load balanced across all network bots
- No "hot holders" (max assignment ≤ 2.5× average)
- Natural variance similar to random assignment
- Verified with 50 owners × 8 chunks × 2 replicas across 100 bots

### No Registry Bloat
- Registry size reduced from O(N × chunks × replicas) to O(N)
- Only maintains network bot list
- No per-chunk holder metadata required
- Massive scalability improvement for large networks

### Security Equivalent to Random Assignment
- Holder identities are computable by anyone (not secret)
- BUT: Chunks remain encrypted with ACI keys
- Attacker must still compromise ALL holders AND obtain ACI key
- Removes central attack target (holder metadata registry)
- Net security improvement: no high-value centralized target

### Owner Cannot Game Assignment
- Assignment is cryptographically derived from hash function
- Owner cannot predict or influence which bots are selected
- Even malicious owners cannot bias holder selection

## Implementation

### Algorithm: HRW (Highest Random Weight)
```
For each candidate bot in network:
  score = hash(owner || chunk_idx || candidate || epoch)

Select top-N scoring candidates as holders
```

### Key Properties
- **Deterministic**: Hash function ensures reproducibility
- **Uniform**: Cryptographic hash provides even distribution
- **Stable**: Epoch-based coordination for network changes
- **Efficient**: O(N log N) computation per chunk (sort by score)

### Epoch Handling
- Epoch increments on network membership changes
- Provides coordination point for churn events
- All nodes compute same holders for same epoch

## Test Results

All test scenarios passed:

1. ✅ Assignment Determinism - Same inputs produce identical outputs
2. ✅ Distribution Uniformity - No hot holders detected
3. ✅ Churn Stability - Minimal reassignment on bot leave
4. ✅ Owner Cannot Game - Assignment based on hash, not choice
5. ✅ Chunk Independence - Different chunks get different holders
6. ✅ Security Analysis - Equivalent security, improved scalability

## Reference Implementation
See [main.rs](main.rs) for complete implementation and detailed test scenarios.

## Architectural Context
This decision supports the [Reciprocal Persistence Network](../../.beads/persistence-model.bead) architecture by:
- Eliminating registry scalability bottlenecks
- Maintaining deterministic holder assignment
- Preserving security through encryption
- Enabling efficient network churn handling
