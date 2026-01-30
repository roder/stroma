# Q4: STARK Verification in Wasm

**Question**: Can winterfell compile to Wasm and verify proofs performantly?

**Status**: Testing

## Background

Stroma needs to verify ZK proofs for trust verification. The architectural decision is:

1. **Contract-side verification** (ideal): Freenet contract verifies proofs directly
2. **Bot-side verification** (fallback): Bot verifies before submitting to Freenet

Contract-side verification is more trustless but requires winterfell to compile to Wasm.

## Test Plan

1. **Compilation Test**: Try to compile winterfell verifier to `wasm32-unknown-unknown`
2. **Performance Test** (if compiles): Measure verification time for sample proof
3. **Document Issues**: Record any compilation or runtime issues

## Decision Criteria

| Result | Action |
|--------|--------|
| Compiles + < 500ms verify | GO: Contract-side verification |
| Compiles + > 500ms verify | PARTIAL: Client-side for now |
| Does not compile | NO-GO: Client-side only |

## Fallback Strategy

Bot verifies proofs before Freenet submission:

```rust
// Bot-side verification (if Wasm doesn't work)
fn verify_vouch_proof(proof: &StarkProof, claim: &VouchClaim) -> Result<bool> {
    winterfell::verify_proof(proof, claim)
}

// Submit only verified outcomes to Freenet
if verify_vouch_proof(&proof, &claim)? {
    freenet.submit_vouch(claim).await?;
}
```

## Files

- `verifier.rs` - STARK proof verification logic
- `main.rs` - Test runner (native and Wasm compilation attempt)
- `RESULTS.md` - Compilation and performance results

## Running the Spike

```bash
# Native test
cargo run --bin spike-q4

# Wasm compilation test
cargo build --target wasm32-unknown-unknown --release -p spike-q4
```

## References

- [winterfell crate](https://docs.rs/winterfell/latest/winterfell/)
- [winterfell GitHub](https://github.com/facebook/winterfell)
- [STARK Proofs](https://starkware.co/stark/)
- [Rust Wasm book](https://rustwasm.github.io/docs/book/)
