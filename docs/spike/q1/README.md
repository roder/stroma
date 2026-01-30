# Q1: Freenet Conflict Resolution Spike

**Branch**: `spike/q1/freenet_conflict_resolution`
**Status**: ✅ Complete - GO Decision
**Decision**: GO - Freenet's merge model works for Stroma

## Objective

Validate that Freenet's delta merge semantics are commutative - producing the same result regardless of the order deltas are applied.

## Key Finding

**Freenet requires commutativity, but it's the contract's responsibility to implement it.**

From Freenet docs:
> "Implementations must ensure that state delta updates are commutative.
>  When applying multiple delta updates to a state, the order in which these
>  updates are applied should not affect the final state."

## Conflict Scenario Tested

```
Time T (simultaneous):
- Node A: Add member X (vouched by A, B)
- Node B: Remove member A (X's voucher)

Question: What state results from the merge?
Answer: Both applied (set union) - X is active, A is tombstoned.
        Trust semantics (X's vouch count) are Stroma's responsibility.
```

## Run Spike

```bash
# Build and run
cargo run --bin spike-q1

# Or just build
cargo build --bin spike-q1

# Run tests
cargo test --package stroma --bin spike-q1
```

## Expected Output

```
╔══════════════════════════════════════════════════════════╗
║     Q1 SPIKE: FREENET CONFLICT RESOLUTION                ║
║     Testing Delta Commutativity for Trust Networks       ║
╚══════════════════════════════════════════════════════════╝

=== Test 1: Delta Commutativity ===
...
✅ PASS: Deltas are commutative (same result regardless of order)

=== Test 2: Vouch Invalidation Scenario ===
...
⚠️  SCENARIO: Both Applied (Set Union)
   IMPLICATION: Stroma's verify() or trust model MUST handle this.

=== Test 3: Tombstone Permanence ===
...
✅ Tombstones are permanent - once removed, cannot re-add

GO/NO-GO DECISION:
✅ GO - Freenet's model works for Stroma
```

## Design Pattern: Set-Based State with Tombstones

```rust
fn apply_delta(&mut self, delta: &Delta) {
    // 1. Apply removals first (add to tombstone set)
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

This guarantees commutativity because:
- Set union is commutative
- Tombstones block late additions (remove-wins)
- Final state depends only on the *set* of deltas, not their order

## Freenet Entry Points

| Use Case | Entry Point |
|----------|-------------|
| Spike testing | `freenet::dev_tool::SimNetwork` |
| Unit testing | `freenet::local_node::Executor::new_mock_in_memory()` |
| Production | `freenet::local_node::NodeConfig::build()` |

## Files

- `main.rs` - Spike test code
- `contract.rs` - SimpleMemberSet with commutative delta application
- `RESULTS.md` - Detailed findings and architectural implications

## Results

See [RESULTS.md](RESULTS.md) for complete analysis.

## Next Steps

1. Q2 Spike: Can `verify()` reject invalid states?
2. Update architecture docs with merge semantics findings
3. Design trust standing recalculation for post-merge states
