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

### Bot-Side Proof Generation & Verification Architecture

```
┌─────────────┐    ┌─────────┐    ┌─────────┐    ┌──────────┐
│   Member    │───►│   Bot   │───►│ Generate│───►│ Freenet  │
│  (command)  │    │(process)│    │& Verify │    │(outcome) │
└─────────────┘    └─────────┘    └─────────┘    └──────────┘
      │                                               │
      │         Signal commands only                  │
      │         (/vouch, /flag, etc.)                 │
      └───────────────────────────────────────────────┘
                  User never sees crypto
```

### Flow

1. Member sends command via Signal (e.g., `/vouch @Alice`)
2. Bot processes command, validates preconditions
3. Bot generates STARK proof (proves vouch validity without revealing voucher)
4. Bot verifies proof using native winterfell (fast, reliable)
5. Bot submits verified outcome to Freenet contract

**Key UX Principle**: Members interact ONLY through Signal commands. All cryptographic operations (STARK proof generation, verification, Merkle proofs) happen inside the bot. Technical complexity is abstracted - users never generate proofs.

### Security Considerations

| Concern | Mitigation |
|---------|------------|
| Compromised bot | Multi-bot consensus (Phase 1+) |
| False proof generation | Audit trail, operator accountability |
| Proof replay | Nonce in proof construction |
| Bot impersonation | Signal protocol authentication |

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

## Why Not Signal Attestation?

**Evaluated**: Using Signal's built-in message authentication to prove members sent commands.

**What Signal Provides**:
- `SenderCertificate` validates sender UUID against Signal's trust root
- Messages are authenticated (can't forge sender identity)
- `sealed_sender_decrypt_to_usmc()` validates certificate chain

**Why It Doesn't Help**:
| What Signal Proves | What We Need |
|-------------------|--------------|
| "Alice sent *a message*" | "Alice sent `/vouch @Bob`" |
| Sender identity is valid | Message content is accurate |
| Envelope is authentic | Bot didn't modify content |

**The Gap**: Signal authenticates the sender, not the content. After decryption, there's no cryptographic binding between the plaintext command and the sender's signature. A compromised bot could claim Alice's message was `/vouch @Bob` when she actually sent `/vouch @Carol`.

**Attacks Signal Attestation Would Defend**:
- Bot fabricating messages entirely (weak attacker model)

**Attacks Signal Attestation Would NOT Defend**:
- Bot modifying message content (realistic threat)
- Compromised member device (out of scope)
- Replay attacks (bot controls timing)

**Conclusion**: Signal attestation adds implementation complexity without defending against realistic threats. The right path to trustlessness is multi-operator federation, not cryptographic message binding.

**See**: `.beads/architecture-decisions.bead` section "Trustlessness Analysis"

## Files

- `verifier.rs` - Verification logic and simulation
- `main.rs` - Test runner and decision framework
- `README.md` - Spike overview
- `RESULTS.md` - This file

## Next Steps

1. Implement bot-side proof generation and verification with winterfell
2. Define ZK circuit for vouch validity (voucher ∈ members, cross-cluster, etc.)
3. Integrate with Freenet outcome submission
4. See Q6 for proof storage decision
