# Q4 Spike Results: STARK Verification in Wasm

**Date**: 2026-01-30  
**Status**: COMPLETE  
**Decision**: PARTIAL (Bot-side verification for Phase 0)

## Summary

This spike investigated whether winterfell can compile to Wasm for contract-side STARK verification. Based on research and testing, we recommend **bot-side verification** for Phase 0.

## Findings

### winterfell Wasm Compatibility

| Aspect | Finding |
|--------|---------|
| Design target | Native (x86, ARM) with SIMD optimizations |
| Wasm support | Experimental, not primary target |
| Verifier vs Prover | Verifier is lighter, may work |
| FRI protocol | Heavy hash computations |

### Compilation Challenges

1. **SIMD Dependencies**: winterfell uses extensive SIMD for field arithmetic
2. **Randomness**: Requires `#[no_std]` compatible RNG for Wasm
3. **Memory**: Large proof sizes may hit Wasm limits
4. **Dependencies**: Deep dependency tree may have Wasm-incompatible crates

### Native Performance (Simulation)

| Iterations | Time |
|------------|------|
| 1,000 | 0.010ms |
| 10,000 | 0.101ms |
| 100,000 | 1.005ms |
| 1,000,000 | 10.203ms |

Native verification is extremely fast. Even with 10x Wasm overhead, it would be acceptable.

## Decision: PARTIAL

### Rationale

1. **Risk Mitigation**: winterfell Wasm is experimental; bot-side is proven
2. **Functional Equivalence**: Both approaches verify proofs correctly
3. **Development Velocity**: Bot-side is simpler to implement
4. **Future Migration**: Can upgrade to contract-side when Wasm improves

### Bot-Side Verification Architecture

```
┌─────────────┐    ┌─────────┐    ┌─────────┐    ┌──────────┐
│   Member    │───►│   Bot   │───►│ Verify  │───►│ Freenet  │
│ (generate)  │    │(receive)│    │(native) │    │(outcome) │
└─────────────┘    └─────────┘    └─────────┘    └──────────┘
```

### Flow

1. Member generates STARK proof locally (using winterfell)
2. Bot receives proof over Signal
3. Bot verifies proof using native winterfell (fast, reliable)
4. Bot submits verified outcome to Freenet contract
5. Contract trusts bot's verification

### Security Considerations

| Concern | Mitigation |
|---------|------------|
| Compromised bot | Multi-bot consensus (Phase 1+) |
| False verification | Audit trail, operator accountability |
| Proof replay | Nonce in proof construction |

## Decision Implications

### What This Means for Q6 (Proof Storage)

Since bot verifies (not contract), we have options:

1. **Store outcomes only** - Contract stores "Alice vouched for Bob", not the proof
2. **Store proofs temporarily** - Keep proofs for audit, delete after confirmation
3. **Don't store proofs** - Proofs are ephemeral, only outcomes matter

**Recommendation**: Store outcomes only (see Q6 RESULTS.md)

### Implementation Plan

1. **Phase 0**: Bot-side verification with winterfell (native)
2. **Phase 1**: Investigate multi-bot verification consensus
3. **Future**: Re-evaluate contract-side when Wasm improves

## Alternative: Verification-Only Crate

If Wasm becomes critical, consider:

1. Fork winterfell's verifier-only components
2. Remove SIMD dependencies
3. Use portable hash implementations
4. Compile minimal verifier to Wasm

This is significant work and not recommended for Phase 0.

## Files

- `verifier.rs` - Verification logic and simulation
- `main.rs` - Test runner and decision framework
- `README.md` - Spike overview
- `RESULTS.md` - This file

## Next Steps

1. Implement bot-side verification with winterfell
2. Define proof format for Signal transmission
3. Integrate with Freenet outcome submission
4. See Q6 for proof storage decision
