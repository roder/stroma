# Q6 Spike Results: Proof Storage Decision

**Date**: 2026-01-30  
**Status**: COMPLETE  
**Decision**: Store outcomes only (not proofs)

## Summary

Based on Q4's decision (bot-side verification), we store **outcomes only** in the Freenet contract, not the STARK proofs themselves.

## Q4 Input

| Q4 Finding | Value |
|------------|-------|
| Wasm compilation | Experimental, not recommended |
| Verification approach | Bot-side (native winterfell) |
| Performance | Sub-millisecond native |

## Decision Matrix Applied

| Q4 Result | Q6 Decision | Applies? |
|-----------|-------------|----------|
| Contract verifies | Don't store proofs | No |
| **Bot verifies** | **Store outcomes only** | **Yes** |
| Partial (slow) | Store temporarily | No |

## Decision: Store Outcomes Only

### What We Store

```rust
// Freenet contract state
pub struct VouchOutcome {
    voucher_hash: Hash,      // HMAC(voucher_signal_id)
    vouchee_hash: Hash,      // HMAC(vouchee_signal_id)
    timestamp: u64,          // When vouch was verified
    // No proof field - proofs are ephemeral
}
```

### What We Don't Store

```rust
// NOT stored in contract
struct StarkProof {
    // ... hundreds of KB of proof data
}
```

### Rationale

1. **Space Efficiency**: STARK proofs are large (10-100KB each). Storing proofs for 1000 members = 10-100MB. Outcomes are tiny (~100 bytes each).

2. **Verification Location**: Since bot verifies (not contract), contract doesn't need the proof. It trusts the bot's verification.

3. **Privacy**: Proofs may contain information that could be used to correlate members. Storing only outcomes minimizes data exposure.

4. **Simplicity**: Contract state is simpler without proof storage. Merging, synchronization, and queries are easier.

5. **Audit Trail**: If audit is needed, proofs can be logged separately (outside contract) with retention policy.

## Alternative Considered: Temporary Storage

We considered storing proofs temporarily for audit:

```rust
// Alternative: Temporary storage (NOT chosen)
pub struct VouchOutcomeWithProof {
    outcome: VouchOutcome,
    proof: Option<StarkProof>,  // Deleted after N days
    verified_at: u64,
}
```

**Rejected because**:
- Complicates contract state management
- Deletion timing is hard in decentralized system
- Audit can be done outside contract if needed

## Architectural Implications

### Contract State

```rust
pub struct StromaContractState {
    members: BTreeSet<Hash>,           // Active members
    vouches: HashMap<Hash, HashSet<Hash>>,  // Vouch graph
    flags: HashMap<Hash, HashSet<Hash>>,    // Flag graph
    // Note: No proofs stored
}
```

### Verification Flow

```
1. Member sends Signal command: /vouch @Bob
2. Bot validates preconditions (member is active, cross-cluster, etc.)
3. Bot generates STARK proof (proves vouch validity without revealing voucher)
4. Bot verifies proof (native winterfell)
5. Bot extracts outcome: "Alice vouches for Bob"
6. Bot submits outcome to Freenet
7. Contract records outcome (no proof)
8. Proof discarded after submission
```

**Key UX Principle**: Members interact ONLY through Signal commands. All cryptographic operations happen inside the bot. Members never generate, sign, or see proofs.

### Audit Strategy

If proof audit is required:

1. **Option A**: Bot logs proofs to separate encrypted storage
2. **Option B**: Proof hashes stored in contract for verification
3. **Option C**: Multi-bot consensus provides redundant verification

For Phase 0, no audit mechanism is required.

## Security Considerations

| Concern | Impact | Mitigation |
|---------|--------|------------|
| Bot submits false outcome | Moderate | Multi-bot consensus (Phase 1) |
| No proof to verify later | Low | Outcomes are verifiable by effect |
| Proof replay | Low | Timestamps and nonces |

## Summary

| Aspect | Decision |
|--------|----------|
| Proof storage | No (outcomes only) |
| Contract state | Members, vouches, flags |
| Proof retention | Ephemeral (discard after verify) |
| Audit mechanism | Not required for Phase 0 |

## Next Steps

1. Implement outcome-only contract state
2. Define outcome submission protocol (bot → Freenet)
3. Define ZK circuit for vouch validity
4. Consider audit logging for Phase 1+
