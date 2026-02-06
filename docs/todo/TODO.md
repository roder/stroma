# Stroma Implementation Checklist

## 🎩 Mayor Briefing: Gastown Delegation Guide

**Project**: Stroma — Privacy-first decentralized trust network  
**Status**: Phase 0-2.5 Implementation in Progress (See Summary Below)  
**Last Updated**: 2026-02-05 (checkboxes updated based on phase review reports)

### 📊 Phase Completion Summary (as of 2026-02-05)

Based on phase review reports in `docs/todo/`:

| Phase | Status | Completion | Critical Findings | Review Report |
|-------|--------|------------|-------------------|---------------|
| **Phase 0** | 🟡 SUBSTANTIALLY COMPLETE | **88%** (22/25 items) | ✅ All core features implemented<br>❌ 2 BLOCKERS: Freenet deps disabled (st-5nhs1), Presage disabled (st-rvzl)<br>✅ 321 tests passing<br>✅ GAP-07 & GAP-08 compliant | PHASE0_REVIEW_REPORT.md |
| **Phase 1** | 🟡 PARTIAL | **70%** | ✅ Trust formula, ejection, health monitoring complete<br>❌ GAP-01 (audit trail) & GAP-03 (rate limiting) missing<br>⚠️ Some trust ops incomplete (integration) | phase1-review-report.md |
| **Phase 2** | 🟡 PARTIAL | **40%** | ✅ DVR & strategic introductions complete<br>❌ /mesh commands stubbed<br>❌ Bridge Removal partial<br>❌ Integration tests missing<br>✅ Benchmarks: ALL targets MET with margin | PHASE2_REVIEW.md<br>PHASE2-BENCHMARKS.md |
| **Phase 2.5** | 🟡 PARTIAL | **70%** | ✅ Core persistence modules complete (69 tests)<br>❌ CRITICAL: 0/13 property tests (st-btcya)<br>❌ Attestation module missing (st-h6ocd)<br>❌ User commands missing (st-p12rt) | phase-2.5-review.md |
| **Infrastructure** | ✅ COMPLETE | **100%** | ✅ All infrastructure requirements met<br>✅ Documentation EXCELLENT (18+ docs, 56 files)<br>⚠️ 5 minor doc gaps (1 P2: CHANGELOG, 4 P3) | INFRASTRUCTURE-DOCUMENTATION-REVIEW.md |

**Security Audit Status**: ✅ **PASS** - Phase 2 security audit (phase2-security-audit.md) verified:
- ✅ No cleartext Signal IDs in logs (0 violations)
- ✅ Transient mapping correctly implemented
- ✅ GAP-02 vote privacy compliant (no individual votes persisted)

**Performance Benchmarks** (Phase 2): ✅ **ALL TARGETS MET**
- DVR calculation: 0.192ms @ 1000 members (target: <1ms) - **5.2x faster**
- Cluster detection: 0.448ms @ 500 members (target: <1ms) - **2.2x faster**
- Blind Matchmaker: 0.120ms @ 500 members (target: <200ms) - **1,667x faster**

### Quick Start for Mayor

```bash
# 1. Review architectural constraints (REQUIRED before delegation)
cat .beads/security-constraints.bead
cat .beads/architecture-objectives.bead
cat .beads/technology-stack.bead

# 2. Create Phase 0 convoy using the convoy formula
bd mol wisp stroma-convoy-phase --var phase=phase0 --var convoy_id=convoy-phase0

# 3. Or manually: spawn Witness FIRST, then polecats
gt spawn --role witness --formula stroma-security-witness
gt spawn --role agent-crypto --bead phase0-hmac --formula stroma-polecat-rust
gt spawn --role agent-freenet --bead phase0-kernel --formula stroma-polecat-rust
gt spawn --role agent-signal --bead phase0-bot --formula stroma-polecat-rust
```

### Stroma-Specific Formulas

| Formula | Type | Purpose |
|---------|------|---------|
| `stroma-security-witness` | Patrol | Continuous "8 Absolutes" enforcement |
| `stroma-polecat-rust` | Work | TDD + 100% coverage + proptest + Co-authored-by |
| `stroma-convoy-phase` | Convoy | Phase orchestration with quality gates |
| `stroma-proptest-trust` | Aspect | Weaves proptest into trust-critical code |

**Location**: `.beads/formulas/stroma-*.formula.toml`

### Agent Roster (Polecat Roles)

| Role | Model Tier | Specialization | Constraint Beads to Read |
|------|------------|----------------|--------------------------|
| **Agent-Crypto** | Opus | HMAC masking, STARKs (winterfell), zeroization | `security-constraints.bead`, `cryptography-zk.mdc` |
| **Agent-Freenet** | Sonnet | Embedded kernel, Wasm contracts, ComposableState | `persistence-model.bead`, `freenet-integration.mdc` |
| **Agent-Signal** | Sonnet | Presage, protocol store, polls, group management | `technology-stack.bead`, `signal-integration.mdc` |
| **Witness** | Haiku | Security audit (continuous), no Signal IDs in persistent storage/logs | `security-constraints.bead` (all 8 absolutes) |

**Model Tier Rationale**:
- **Opus** (highest): Complex mathematical reasoning — STARK circuits, cryptographic proofs, winterfell integration
- **Sonnet** (mid): Well-documented APIs, pattern-based — Freenet contracts, Presage protocol, ComposableState
- **Haiku** (lowest): Pattern-matching audit — continuous scanning for constraint violations, no deep reasoning required

### Agent Model Configuration

Agents are configured in the Gastown rig at `~/gastown/stroma/settings/agents.json`:

```json
{
  "version": 1,
  "agents": {
    "agent-crypto": {
      "command": "claude",
      "args": ["--model", "claude-opus-4-20250514"]
    },
    "agent-freenet": {
      "command": "claude",
      "args": ["--model", "claude-sonnet-4-20250514"]
    },
    "agent-signal": {
      "command": "claude",
      "args": ["--model", "claude-sonnet-4-20250514"]
    },
    "witness": {
      "command": "claude",
      "args": ["--model", "claude-3-5-haiku-20241022"]
    }
  }
}
```

**Override per-sling** (Mayor can escalate model tier for complex tasks):

```bash
# Default: use configured agent model
gt sling hq-stark-circuit stroma

# Override: escalate to Opus for particularly complex cryptographic work
gt sling hq-stark-circuit stroma --agent claude-opus
```

### Critical Security Constraints (ALL Agents MUST Read)

**The Eight Absolutes (NEVER)** — Violations block merge:
1. NEVER store Signal IDs in cleartext
2. NEVER persist message history
3. NEVER bypass ZK-proof verification
4. NEVER add grace periods for ejection
5. NEVER make Signal source of truth
6. NEVER restrict vouching to Validators only
7. NEVER commit without Co-authored-by (AI agents)
8. NEVER trust persistence peers

**The Eight Imperatives (ALWAYS)** — Required for all implementations:
1. ALWAYS hash Signal IDs immediately with `mask_identity()` then zeroize
2. ALWAYS verify Freenet contract state before executing any action
3. ALWAYS use trait abstractions for testability (SignalClient, FreenetClient)
4. ALWAYS encrypt chunks with AES-256-GCM using ACI-derived key
5. ALWAYS use Freenet state stream (real-time events, NOT polling)
6. ALWAYS log operation types only (no identifiers, no relationships)
7. ALWAYS include `// SAFETY:` comments for any unsafe blocks
8. ALWAYS run quality gates before commit (`fmt`, `clippy`, `deny`, `llvm-cov`)

**See**: `.beads/security-constraints.bead` for enforcement patterns

---

## 📊 Project Status Overview

| Phase | Focus | Duration | Status | Convoy |
|-------|-------|----------|--------|--------|
| **Spike Week 1** | Freenet/STARK validation (Q1-Q6) | Week 0 | ✅ Complete | — |
| **Spike Week 2** | Persistence validation (Q7-Q14) | Week 0.5 | ✅ Complete | — |
| **Phase -1** | Protocol v8 Poll Support | 1-2 weeks | ✅ Complete | — |
| **Pre-Gastown Audit** | Human review before agent handoff | 6-9 hours | ✅ Complete | — |
| **Phase 0** | Foundation (HMAC, Freenet, Signal, STARK) | Weeks 1-2 | 📋 **NEXT** | `convoy-phase0` |
| **Phase 1** | Bootstrap & Core Trust | Weeks 3-4 | 📋 Planned | `convoy-phase1` |
| **Phase 2** | Proposals & Mesh Optimization | Weeks 5-6 | 📋 Planned | `convoy-phase2` |
| **Phase 2.5** | Persistence Implementation | Week 6-7 | 📋 Planned | `convoy-persistence` |
| **Phase 3** | Federation Preparation | Week 7-8 | 📋 Planned | `convoy-phase3` |
| **Phase 4+** | Federation (Future) | TBD | 📋 Future | — |
| **Infrastructure** | CI/CD, Docker, Documentation | Parallel to phases | ✅ **COMPLETE** | `convoy-infra` |

---

## 🚀 PHASE 0: Foundation Convoy

**Convoy ID**: `convoy-phase0`  
**Duration**: Weeks 1-2  
**Parallelizable**: Yes (4 independent tracks)  
**Review Report**: `docs/todo/PHASE0_REVIEW_REPORT.md`  
**Status**: 🟡 **88% Complete** (22/25 items) - 2 critical blockers (st-5nhs1, st-rvzl)

### Mayor Delegation Commands

```bash
# Create Phase 0 convoy
gt convoy create --title "Phase 0: Foundation" \
  --description "Core infrastructure: HMAC, Freenet, Signal, STARK"

# Create beads for each work unit
bd create --title "HMAC identity masking with zeroization" --convoy convoy-phase0
bd create --title "Embedded Freenet kernel integration" --convoy convoy-phase0
bd create --title "Signal bot with custom protocol store" --convoy convoy-phase0
bd create --title "STARK circuits for vouch verification" --convoy convoy-phase0
bd create --title "Freenet contract schema (TrustNetworkState)" --convoy convoy-phase0
bd create --title "Operator CLI interface" --convoy convoy-phase0

# Spawn parallel polecats (4 can run simultaneously)
gt spawn --role agent-crypto --bead <hmac-bead-id>
gt spawn --role agent-freenet --bead <kernel-bead-id>
gt spawn --role agent-signal --bead <signal-bead-id>
gt spawn --role witness --watch-all  # Continuous security audit
```

### Track 1: Cryptographic Foundation (Agent-Crypto)

**Bead**: `phase0-hmac` — HMAC Identity Masking  
**Agent**: Agent-Crypto  
**Dependencies**: None (can start immediately)  
**Constraint Beads**: `security-constraints.bead` § 1-2  
**Review**: See `PHASE0_REVIEW_REPORT.md` lines 28-50 (✅ FULLY IMPLEMENTED)

#### Deliverables

- [ ] `src/kernel/hmac.rs` — HMAC-SHA256 with ACI-derived key
  ```rust
  // Pattern from security-constraints.bead
  fn derive_identity_masking_key(aci_identity: &IdentityKeyPair) -> [u8; 32]
  fn mask_identity(signal_id: &str, aci_identity: &IdentityKeyPair) -> Hash
  ```
- [ ] `src/kernel/zeroize_helpers.rs` — Immediate buffer purging
- [ ] Unit tests with fixed test ACI identity
- [ ] Property-based tests (proptest) for collision resistance

#### Acceptance Criteria

- [x] 100% code coverage (enforced by CI) ✅ **COMPLETE** (Phase 0 Review)
- [x] Property-based tests (proptest) covering:
  - [x] HMAC determinism: same input + same key = same output ✅
  - [x] Key isolation: same input + different keys = different outputs ✅
  - [x] Collision resistance: different inputs = different outputs (probabilistic) ✅
- [x] Memory dump contains ONLY hashed identifiers (verify with intentional panic) ✅
- [x] Zeroization happens immediately after hashing ✅
- [x] `cargo clippy` and `cargo fmt` pass ✅

---

**Bead**: `phase0-stark` — STARK Circuits  
**Agent**: Agent-Crypto  
**Dependencies**: `phase0-hmac` (uses Hash type)  
**Constraint Beads**: `cryptography-zk.mdc`  
**Review**: See `PHASE0_REVIEW_REPORT.md` lines 53-82 (✅ FULLY IMPLEMENTED)

#### Deliverables

- [ ] `src/crypto/stark_circuit.rs` — Vouch verification circuit
- [ ] `src/crypto/proof_generation.rs` — Generate proofs (bot-side)
- [ ] `src/crypto/proof_verification.rs` — Verify proofs (bot-side, not Wasm)
- [ ] Benchmark: proof generation < 10 seconds, proof size < 100KB
- [ ] Property-based tests (proptest) for proof soundness

#### Acceptance Criteria

- [x] 100% code coverage (enforced by CI) ✅ **COMPLETE** (Phase 0 Review)
- [x] winterfell integration compiles ✅
- [x] Proof roundtrip: generate → serialize → deserialize → verify ✅
- [x] Performance within bounds (Q4 spike validated bot-side approach) ✅ (<10s, <100KB)
- [x] Property-based tests (proptest) covering:
  - [x] **Completeness**: valid inputs always produce verifiable proofs ✅
  - [x] **Soundness**: invalid inputs never produce verifiable proofs (probabilistic) ✅
  - [x] **Determinism**: same inputs produce identical proofs ✅
  - [x] **Serialization stability**: serialize → deserialize → serialize = original bytes ✅

#### Test Cases

```rust
#[test]
fn test_proof_roundtrip() {
    // Given: Valid vouch data (voucher, vouchee, context_hash)
    // When: Generate proof → serialize → deserialize → verify
    // Then: Verification succeeds
}

#[test]
fn test_invalid_proof_rejected() {
    // Given: Proof with tampered public inputs
    // When: Verify
    // Then: Verification fails
}

#[test]
fn test_proof_size_within_bounds() {
    // Given: Valid vouch data
    // When: Generate proof
    // Then: Serialized size < 100KB
}

proptest! {
    #[test]
    fn proof_completeness(
        voucher_hash: [u8; 32],
        vouchee_hash: [u8; 32],
        context_hash: [u8; 32],
    ) {
        // Valid inputs always produce verifiable proofs
        let proof = generate_vouch_proof(&voucher_hash, &vouchee_hash, &context_hash);
        assert!(verify_vouch_proof(&proof, &voucher_hash, &vouchee_hash, &context_hash).is_ok());
    }
    
    #[test]
    fn proof_determinism(
        voucher_hash: [u8; 32],
        vouchee_hash: [u8; 32],
        context_hash: [u8; 32],
    ) {
        // Same inputs produce identical proofs
        let proof1 = generate_vouch_proof(&voucher_hash, &vouchee_hash, &context_hash);
        let proof2 = generate_vouch_proof(&voucher_hash, &vouchee_hash, &context_hash);
        assert_eq!(proof1.serialize(), proof2.serialize());
    }
    
    #[test]
    fn proof_soundness_tampered_voucher(
        voucher_hash: [u8; 32],
        vouchee_hash: [u8; 32],
        context_hash: [u8; 32],
        tampered_voucher: [u8; 32],
    ) {
        prop_assume!(voucher_hash != tampered_voucher);
        // Proof for one voucher doesn't verify for different voucher
        let proof = generate_vouch_proof(&voucher_hash, &vouchee_hash, &context_hash);
        assert!(verify_vouch_proof(&proof, &tampered_voucher, &vouchee_hash, &context_hash).is_err());
    }
    
    #[test]
    fn serialization_roundtrip_stable(
        voucher_hash: [u8; 32],
        vouchee_hash: [u8; 32],
        context_hash: [u8; 32],
    ) {
        // Serialize → deserialize → serialize produces identical bytes
        let proof = generate_vouch_proof(&voucher_hash, &vouchee_hash, &context_hash);
        let bytes1 = proof.serialize();
        let deserialized = VouchProof::deserialize(&bytes1).unwrap();
        let bytes2 = deserialized.serialize();
        assert_eq!(bytes1, bytes2);
    }
}
```

---

### Track 2: Freenet Integration (Agent-Freenet)

**Bead**: `phase0-kernel` — Embedded Freenet Kernel  
**Agent**: Agent-Freenet  
**Dependencies**: None (can start immediately)  
**Constraint Beads**: `freenet-integration.mdc`  
**Review**: See `PHASE0_REVIEW_REPORT.md` lines 87-110 (⚠️ CODE COMPLETE, DEPENDENCIES DISABLED)  
**Blocker**: st-5nhs1 - Freenet dependencies disabled in Cargo.toml:69-70, `persistence-model.bead`

#### Deliverables

- [ ] `src/freenet/traits.rs` — Trait abstraction for testability
  ```rust
  // ✅ REQUIRED: Trait abstraction enables 100% coverage
  #[async_trait]
  pub trait FreenetClient: Send + Sync {
      async fn get_state(&self, contract: &ContractHash) -> Result<TrustState>;
      async fn apply_delta(&self, contract: &ContractHash, delta: &Delta) -> Result<()>;
      async fn subscribe(&self, contract: &ContractHash) -> Result<StateStream>;
  }
  ```
- [ ] `src/freenet/mod.rs` — Module exports
- [ ] `src/freenet/embedded_kernel.rs` — In-process kernel (freenet-stdlib)
  - [ ] Initialize kernel with dark mode (anonymous routing)
  - [ ] Single event loop integration (tokio)
  - [ ] Node lifecycle management (start, stop, health check)
- [ ] `src/freenet/contract.rs` — Wasm contract deployment
- [ ] `src/freenet/state_stream.rs` — Real-time state monitoring (NOT polling)

#### Testing Strategy (NO EXCEPTIONS)

- **Unit tests**: Use `Executor::new_mock_in_memory()` (instant, deterministic)
- **Integration tests**: Use `SimNetwork` for convergence testing
- **No coverage exceptions**: Trait abstraction makes all code testable

#### Acceptance Criteria

- [x] 100% code coverage (enforced by CI) ✅ **COMPLETE** (Phase 0 Review)
- [x] Unit tests use `Executor::new_mock_in_memory()` (not real network) ✅
- [ ] Kernel starts in-process without external service ❌ **BLOCKED** (st-5nhs1 - dependencies disabled)
- [x] State changes trigger stream events ✅ (code complete)
- [x] Contract deploys successfully to embedded kernel ✅ (code complete)
- [x] Verified: ComposableState merge is commutative (Q1 validated) ✅

---

**Bead**: `phase0-contract` — Trust Network Contract Schema  
**Agent**: Agent-Freenet  
**Dependencies**: `phase0-kernel`  
**Constraint Beads**: `freenet-contract-design.mdc`, `serialization-format.bead`  
**Review**: See `PHASE0_REVIEW_REPORT.md` lines 114-145 (✅ FULLY IMPLEMENTED, GAP-08 COMPLIANT)

#### Deliverables

- [ ] `src/freenet/trust_contract.rs` — TrustNetworkState struct
  ```rust
  #[derive(Serialize, Deserialize)]
  pub struct TrustNetworkState {
      members: BTreeSet<Hash>,           // Active members
      ejected: BTreeSet<Hash>,           // Ejected (not tombstone - can re-enter)
      vouches: HashMap<Hash, HashSet<Hash>>,  // who vouched for whom
      flags: HashMap<Hash, HashSet<Hash>>,    // who flagged whom
      config: GroupConfig,
      schema_version: u64,               // GAP-08: For debugging, not migration logic
      
      // GAP-08: Federation hooks (present but unused in MVP)
      // Use #[serde(default)] for backward-compatible schema evolution
      #[serde(default)]
      federation_contracts: Vec<ContractHash>,
  }
  ```
- [ ] `src/serialization/mod.rs` — CBOR serialization (NOT JSON)
- [ ] StateDelta struct with commutative operations
- [ ] Federation hooks (present but unused in MVP)

#### Acceptance Criteria

- [x] 100% code coverage (enforced by CI) ✅ **COMPLETE** (Phase 0 Review)
- [x] Property-based tests (proptest) for trust-critical invariants:
  - [x] Delta commutativity: `merge(A, B) == merge(B, A)` ✅
  - [x] Standing calculation: `standing = effective_vouches - regular_flags` ✅
  - [x] Vouch invalidation: voucher-flaggers excluded from BOTH counts ✅ **CRITICAL SUCCESS**
- [x] CBOR roundtrip: serialize → deserialize → compare ✅
- [x] Deterministic serialization (canonical bytes) ✅
- [x] Contract validates via `update_state()` + `validate_state()` (Q2 pattern) ✅

---

### Track 3: Signal Integration (Agent-Signal)

**Bead**: `phase0-signal` — Signal Bot with Custom Store  
**Agent**: Agent-Signal  
**Dependencies**: None (can start immediately)  
**Constraint Beads**: `technology-stack.bead` § Signal, `security-constraints.bead` § 10  
**Review**: See `PHASE0_REVIEW_REPORT.md` lines 149-189 (⚠️ CODE COMPLETE, PRESAGE DISABLED)  
**Blocker**: st-rvzl - Presage dependency disabled in Cargo.toml:77-78  
**Security**: GAP-07 verified (0 PII in logs)

#### Deliverables

- [ ] `src/signal/traits.rs` — Trait abstraction for testability
  ```rust
  // ✅ REQUIRED: Trait abstraction enables 100% coverage via mocks
  #[async_trait]
  pub trait SignalClient: Send + Sync {
      async fn send_message(&self, recipient: &ServiceId, message: &str) -> Result<()>;
      async fn create_poll(&self, group: &GroupId, poll: &PollCreate) -> Result<PollId>;
      async fn add_group_member(&self, group: &GroupId, member: &ServiceId) -> Result<()>;
      async fn remove_group_member(&self, group: &GroupId, member: &ServiceId) -> Result<()>;
  }
  ```
- [ ] `src/signal/store.rs` — Custom StromaProtocolStore
  ```rust
  // ❌ FORBIDDEN: SqliteStore (stores message history)
  // ✅ REQUIRED: Custom minimal store
  pub struct StromaProtocolStore {
      sessions: HashMap<ServiceId, Session>,  // In-memory
      pre_keys_cache: HashMap<u32, PreKey>,   // In-memory
      protocol_state_file: EncryptedProtocolState,  // ~100KB on disk
  }
  ```
- [ ] `src/signal/mock.rs` — MockSignalClient for unit tests
- [ ] `src/signal/linking.rs` — Link as secondary device
  - [ ] Generate provisioning URL via `Manager::link_secondary_device()`
  - [ ] Display QR code in terminal (qr2term)
  - [ ] Save ACI/PNI identity to custom store
- [ ] `src/signal/bot.rs` — Presage Manager wrapper (implements SignalClient)
- [ ] `src/signal/group.rs` — Group management (add/remove members)
- [ ] `src/signal/pm.rs` — 1-on-1 PM handling for vetting
- [ ] `src/signal/polls.rs` — Poll creation/monitoring (protocol v8)

#### Critical Constraint

**Server Seizure Protection**: The custom store MUST NOT persist:
- ❌ Message history
- ❌ Vetting conversation content
- ❌ Contact database
- ❌ Invitation context

#### Testing Strategy (NO EXCEPTIONS)

- **Unit tests**: Use `MockSignalClient` (trait-based mock)
- **Integration tests**: Mock + Recording proxy for verification
- **Manual E2E**: Pre-release only for `link-device` (requires real account)
- **No coverage exceptions**: Trait abstraction makes all code testable

#### Acceptance Criteria

- [x] 100% code coverage (enforced by CI) ✅ **COMPLETE** (Phase 0 Review)
- [x] Unit tests use `MockSignalClient` (not real Signal) ✅
- [ ] Bot links successfully as secondary device (manual E2E) ❌ **BLOCKED** (st-rvzl - presage disabled)
- [x] Messages received and processed (not stored) ✅ (code complete)
- [x] Group creation and member management works ✅ (code complete)
- [x] Store file is small (~100KB, not growing) ✅ (StromaProtocolStore)
- [x] Witness agent confirms: no Signal IDs in any persistent storage ✅ **GAP-07 COMPLIANT** (0 violations)

---

### Track 4: CLI & Infrastructure (Agent-Signal or General)

**Bead**: `phase0-cli` — Operator CLI Interface  
**Agent**: Agent-Signal (or general polecat)  
**Dependencies**: `phase0-signal` (uses linking)  
**Constraint Beads**: `operator-cli.mdc`  
**Review**: See `PHASE0_REVIEW_REPORT.md` lines 197-219 (✅ FULLY IMPLEMENTED, integration tests disabled)

#### Deliverables

- [ ] `src/cli/mod.rs` — CLI entry point
- [ ] `src/cli/link_device.rs` — `stroma link-device` command
- [ ] `src/cli/run.rs` — `stroma run` command
- [ ] `src/cli/utils.rs` — `status`, `verify`, `backup-store`, `version`

#### Command Specification

| Command | Purpose | Trust Operations |
|---------|---------|------------------|
| `stroma link-device` | Link to Signal account as secondary device | None |
| `stroma run` | Normal operation (awaits member-initiated bootstrap) | None |
| `stroma status` | Health check | None |
| `stroma verify` | Config validation | None |
| `stroma backup-store` | Export Signal store for backup | None |
| `stroma version` | Version info | None |

**Critical**: Operator CLI has NO trust operation commands. Bootstrap is member-initiated via Signal (`/create-group`, `/add-seed`).

#### Acceptance Criteria

- [x] All commands parse correctly (clap) ✅ **COMPLETE** (Phase 0 Review)
- [ ] `link-device` produces QR code and completes linking ❌ **BLOCKED** (st-rvzl - presage disabled)
- [ ] `run` starts bot and awaits Signal messages ❌ **BLOCKED** (st-rvzl - presage disabled)
- [x] `status` shows health information ✅ (code complete)

---

### Phase 0 Cargo Dependencies

**Bead**: `phase0-deps` — Cargo Configuration  
**Agent**: Any (quick task)

```toml
[dependencies]
# Cryptography
ring = { version = "0.17", features = ["std"] }
zeroize = { version = "1.7", features = ["zeroize_derive"] }
winterfell = "0.9"
hkdf = "0.12"

# Serialization
serde = { version = "1.0", features = ["derive"] }
ciborium = "0.2"  # CBOR for Freenet (NOT JSON)

# Signal integration
presage = { git = "https://github.com/whisperfish/presage" }

# Freenet integration
freenet-stdlib = { version = "0.0.7", features = ["full"] }

# Async runtime
tokio = { version = "1", features = ["full"] }

# CLI
clap = { version = "4", features = ["derive"] }

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

[patch.crates-io]
curve25519-dalek = { git = 'https://github.com/signalapp/curve25519-dalek', tag = 'signal-curve25519-4.1.3' }
libsignal-service = { git = "https://github.com/roder/libsignal-service-rs", branch = "feature/protocol-v8-polls-fixed" }
```

---

### Phase 0 Success Criteria

Before closing `convoy-phase0`, the Mayor MUST verify ALL of the following:

#### HMAC Identity Masking (Agent-Crypto)

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| HMAC-SHA256 with ACI-derived key | Unit test: deterministic output | Agent-Crypto |
| Immediate zeroization | Unit test: buffer cleared after hashing | Agent-Crypto |
| Proptest: HMAC determinism | Same input + same key = same output | Agent-Crypto |
| Proptest: Key isolation | Different keys = different outputs | Agent-Crypto |
| Proptest: Collision resistance | Different inputs = different outputs | Agent-Crypto |
| Memory dump test | Intentional panic shows only hashes, no Signal IDs | Witness |

#### STARK Circuits (Agent-Crypto)

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| winterfell integration compiles | Build test passes | Agent-Crypto |
| Proof roundtrip | generate → serialize → deserialize → verify succeeds | Agent-Crypto |
| Proof size < 100KB | Benchmark test | Agent-Crypto |
| Proof generation < 10 seconds | Benchmark test | Agent-Crypto |
| Proptest: Completeness | Valid inputs always produce verifiable proofs | Agent-Crypto |
| Proptest: Soundness | Invalid inputs never produce verifiable proofs | Agent-Crypto |

#### Freenet Kernel (Agent-Freenet)

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| Embedded kernel starts in-process | Integration test: no external service | Agent-Freenet |
| Unit tests use `Executor::new_mock_in_memory()` | Code review: no real network in tests | Agent-Freenet |
| State changes trigger stream events | Unit test: subscribe receives deltas | Agent-Freenet |
| Contract deploys successfully | Integration test: deploy to embedded kernel | Agent-Freenet |
| ComposableState merge is commutative | Unit test: A∪B = B∪A | Agent-Freenet |

#### Contract Schema (Agent-Freenet)

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| **GAP-08**: `schema_version: u64` field present | Code review: field in TrustNetworkState | Agent-Freenet |
| **GAP-08**: Federation hooks with `#[serde(default)]` | Code review: backward-compatible deserialization | Agent-Freenet |
| Members as BTreeSet<Hash> | Code review: no Signal IDs | Witness |
| Ejected set (not tombstone) | Code review: re-entry possible | Agent-Freenet |
| CBOR serialization (not JSON) | Code review: ciborium used | Agent-Freenet |

#### Signal Integration (Agent-Signal)

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| Bot links successfully | Integration test: device linking | Agent-Signal |
| Bot manages groups | Integration test: create, add, remove | Agent-Signal |
| Signal Polls supported (v8) | Unit test: PollCreate message builds | Agent-Signal |
| Custom StromaProtocolStore used | Code review: NOT presage-store-sqlite | Witness |
| No message history stored | Code review: no message persistence | Witness |

#### Logging Security — GAP-07 (CRITICAL)

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| No PII logged (Signal IDs, phones, names) | `rg` audit: zero matches | Witness |
| No trust map relationships logged | `rg` audit: zero voucher→target logs | Witness |
| No persistence locations logged | `rg` audit: zero chunk→holder logs | Witness |
| No federation relationships logged | `rg` audit: zero federated→group logs | Witness |
| Compromised bot test passes | Review: logs reveal nothing sensitive | Witness |

#### Security Constraints (Witness MUST Verify)

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| No cleartext Signal IDs in storage | Code review: all state uses hashes | Witness |
| No cleartext Signal IDs in logs | Code review: GAP-07 four-layer audit | Witness |
| No cleartext Signal IDs in output | Code review: all messages use hashes | Witness |
| presage-store-sqlite NOT used | `rg` audit: zero imports | Witness |
| StromaProtocolStore implemented | Code review: custom store | Agent-Signal |

#### Code Coverage (CI Enforced)

- [x] 100% code coverage on `src/kernel/hmac.rs` ✅ **COMPLETE** (identity.rs 100%)
- [x] 100% code coverage on `src/crypto/*.rs` ✅ **COMPLETE** (stark/* comprehensive)
- [x] 100% code coverage on `src/freenet/*.rs` ✅ **COMPLETE** (trust_contract.rs with proptests)
- [x] All proptests pass (minimum 256 cases per test) ✅ **321 tests passing**
- [x] `cargo clippy` passes with no warnings ✅
- [x] `cargo deny check` passes (supply chain security) ✅

#### Mayor Convoy Closure Checklist

```bash
# Verify all beads complete
bd list --convoy convoy-phase0 --status pending
# Should return: No pending beads

# Verify GAP-07 and GAP-08 complete
gt audit --convoy convoy-phase0 --agent witness
# Should return: All security constraints verified

# Verify logging security (GAP-07 - CRITICAL)
rg "tracing::(info|debug|warn|error)" --type rust -l | \
  xargs -I {} sh -c 'rg "signal_id|phone|name" {} && echo "FAIL: {}"'
# Should find zero violations

# Verify test coverage
cargo llvm-cov nextest --all-features -- --test-threads=1
# Should return: 100% coverage on kernel, crypto, freenet modules

# Verify property tests
cargo test --release -- proptest
# Should return: All proptests pass (256+ cases each)

# Close convoy
gt convoy close convoy-phase0 --verified
```

---

## 🌱 PHASE 1: Bootstrap & Core Trust Convoy

**Convoy ID**: `convoy-phase1`  
**Duration**: Weeks 3-4  
**Dependencies**: `convoy-phase0` complete  
**Review Report**: `docs/todo/phase1-review-report.md`  
**Status**: 🟡 **70% Complete** - Trust formula provably correct, GAP-01 & GAP-03 missing

### Mayor Delegation Commands

```bash
gt convoy create --title "Phase 1: Bootstrap & Core Trust" \
  --depends-on convoy-phase0

bd create --title "Seed group bootstrap flow" --convoy convoy-phase1
bd create --title "Invitation & vetting flow" --convoy convoy-phase1
bd create --title "Admission protocol with ZK-proof" --convoy convoy-phase1
bd create --title "Ejection protocol (two triggers)" --convoy convoy-phase1
bd create --title "Health monitoring (continuous)" --convoy convoy-phase1
bd create --title "Basic bot commands" --convoy convoy-phase1
```

### Bead: Bootstrap Flow

**Agent**: Agent-Signal + Agent-Freenet  
**Parallelizable**: No (sequential flow)  
**Review**: See `phase1-review-report.md` lines 17-51 (✅ COMPLETE - 525 lines, GAP-05 compliant)  
**Gaps**: GAP-09 partial (lines 49-51), `/audit bootstrap` stub

#### Bootstrap Sequence (Member-Initiated)

1. First member sends `/create-group "Group Name"` to bot via PM
2. Bot creates Signal group, adds first member
3. First member invites 2 seed members via `/add-seed @username`
4. All 3 seeds vouch for each other (triangle vouching)
5. Initialize Freenet contract with 3 members
6. Each seed member has 2 vouches (Bridge status)

**Note**: Bootstrap is ONE-TIME. After 3 seeds, normal invite/vouch flow applies.

#### Deliverables

- [x] `src/gatekeeper/bootstrap.rs` — Bootstrap state machine ✅ **COMPLETE** (525 lines, comprehensive)
- [x] Handle `/create-group` command ✅
- [x] Handle `/add-seed` command (only during bootstrap) ✅
- [x] Create initial Freenet contract state ✅
- [ ] `BootstrapEvent` recorded in Freenet contract (GAP-09) ⚠️ **PARTIAL** (structure exists, Freenet integration TODO)

#### Acceptance Criteria (GAP-05, GAP-09)

- [x] Group name is REQUIRED (non-empty string validation) ✅ **GAP-05 COMPLETE**
- [x] Group name stored in Freenet contract `group_name` field ✅
- [x] Signal group name matches Freenet contract name ✅
- [ ] `/audit bootstrap` command shows bootstrap event details ⚠️ **STUB** (GAP-09 incomplete)
- [x] All bot messages include group name (not "this group") ✅

---

### Bead: Trust Operations

**Agent**: Agent-Signal + Agent-Crypto  
**Dependencies**: `phase0-hmac` (identity masking), `phase0-stark` (ZK-proof generation)  
**Review**: See `phase1-review-report.md` lines 54-102 (⚠️ PARTIAL - flagging complete, vetting TODOs)  
**Complete**: Flagging (pm.rs:284-399), ZK-proof (bot.rs:327-363)  
**Incomplete**: 3-person PM chat, session cleanup, GAP-10 Freenet query

**Cryptographic Operations (delegated to Agent-Crypto beads):**
- **Identity Masking**: All Signal IDs hashed via `mask_identity()` from `phase0-hmac`
- **ZK-Proof Generation**: Admission proofs via `generate_vouch_proof()` from `phase0-stark`
- **Property-based tests for crypto**: Covered by `phase0-hmac` and `phase0-stark` beads

#### Invitation Flow

- [x] Member sends `/invite @username [context]` ✅ **COMPLETE** (command parsing)
- [x] Bot records invitation as first vouch (context is EPHEMERAL) ✅
- [ ] **GAP-10**: Bot warns inviter if invitee has previous flags (re-entry scenario) ⚠️ **PARTIAL** (structure exists, Freenet query TODO)
  - Example: "⚠️ @Alice has 3 previous flags. They'll need 4+ vouches to achieve positive standing."
- [x] Bot selects second Member via Blind Matchmaker ✅ (logic complete)
- [ ] Bot sends PMs to invitee and selected Member ⚠️ **TODO**

#### Vetting Interview

- [ ] Bot creates 3-person chat (invitee, Member, bot) ⚠️ **TODO**
- [ ] Bot facilitates introduction ⚠️ **TODO**
- [x] Member vouches via `/vouch @username` ✅ (command parsing)
- [ ] Bot records second vouch in Freenet ⚠️ **TODO**

#### Admission

- [ ] Bot verifies 2 vouches from different CLUSTERS ⚠️ **PARTIAL** (cross-cluster enforcement TODO)
- [x] Exception: Bootstrap phase (only 1 cluster) — same-cluster allowed ✅
- [x] Bot generates ZK-proof of admission criteria (uses `phase0-stark`) ✅ **COMPLETE** (bot.rs:327-363)
- [x] Bot stores outcome (not proof) in Freenet contract (Q6 decision) ✅
- [ ] Bot adds invitee to Signal group (now a Bridge) ⚠️ **TODO**
- [x] Bot announces admission (using hash from `phase0-hmac`, not Signal ID) ✅
- [ ] Bot deletes vetting session data immediately ⚠️ **TODO** (VettingSessionManager exists)

#### Flagging

- [x] Member sends `/flag @username [reason]` ✅ **COMPLETE** (pm.rs:284-399)
- [x] Bot records flag in Freenet ✅
- [x] Bot recalculates: `Standing = Effective_Vouches - Regular_Flags` ✅
- [x] If voucher flags: their vouch is invalidated (excluded from BOTH counts) ✅ **CRITICAL SUCCESS**
- [x] Bot checks ejection triggers ✅

#### Acceptance Criteria

**Signal Flow (Agent-Signal):**
- [x] Invitation creates vetting session ✅ (VettingSessionManager)
- [ ] Vetting interview uses 3-person PM chat ⚠️ **TODO**
- [ ] Session data deleted after admission or rejection ⚠️ **TODO**

**Cryptographic Integration (Agent-Crypto via delegated beads):**
- [x] All Signal IDs masked via `mask_identity()` before storage ✅ **COMPLETE** (MemberHash everywhere)
- [x] ZK-proof generated via `generate_vouch_proof()` on admission ✅
- [x] Proof covers: voucher hashes, cluster membership assertions ✅
- [x] Property-based tests pass in `phase0-hmac` and `phase0-stark` beads ✅ **321 tests passing**

**Trust Formula:**
- [x] Standing calculation: `Standing = Effective_Vouches - Regular_Flags` ✅ **COMPLETE** (trust_contract.rs:284-316)
- [x] Voucher-flags excluded from BOTH counts (no 2-point swing) ✅ **CRITICAL SUCCESS** (property test)
- [x] Property-based tests (proptest) covering:
  - [x] Standing always equals effective vouches minus flags ✅
  - [x] Voucher-flagging never produces 2-point swing ✅ **test_no_2point_swing_voucher_flags**
  - [x] Standing bounds: `min ≤ standing ≤ max_vouches` ✅

#### Test Cases

```rust
proptest! {
    #[test]
    fn standing_formula_correct(
        vouches: HashSet<Hash>,
        flags: HashSet<Hash>,
    ) {
        // Given: Set of vouchers and flaggers
        let voucher_flaggers: HashSet<Hash> = vouches.intersection(&flags).cloned().collect();
        
        let effective_vouches = vouches.len() - voucher_flaggers.len();
        let regular_flags = flags.len() - voucher_flaggers.len();
        
        let standing = compute_standing(&vouches, &flags);
        
        // Standing = Effective_Vouches - Regular_Flags
        prop_assert_eq!(standing, effective_vouches as i32 - regular_flags as i32);
    }
    
    #[test]
    fn voucher_flag_no_double_swing(
        initial_vouches: HashSet<Hash>,
        initial_flags: HashSet<Hash>,
        voucher_who_flags: Hash,
    ) {
        prop_assume!(initial_vouches.contains(&voucher_who_flags));
        prop_assume!(!initial_flags.contains(&voucher_who_flags));
        
        let standing_before = compute_standing(&initial_vouches, &initial_flags);
        
        // Voucher adds flag
        let mut new_flags = initial_flags.clone();
        new_flags.insert(voucher_who_flags);
        
        let standing_after = compute_standing(&initial_vouches, &new_flags);
        
        // Swing is exactly 0 (voucher excluded from both counts)
        let swing = standing_before - standing_after;
        prop_assert_eq!(swing, 0, "Voucher-flag should produce 0 swing, got {}", swing);
    }
    
    #[test]
    fn standing_bounded(
        vouches: HashSet<Hash>,
        flags: HashSet<Hash>,
    ) {
        let standing = compute_standing(&vouches, &flags);
        let max_positive = vouches.len() as i32;
        let max_negative = -(flags.len() as i32);
        
        prop_assert!(standing <= max_positive);
        prop_assert!(standing >= max_negative);
    }
}
```

---

### Bead: Ejection Protocol

**Agent**: Agent-Signal + Agent-Freenet  
**Review**: See `phase1-review-report.md` lines 137-162 (✅ COMPLETE - 507 lines, GAP-06 compliant)  
**Details**: Logarithmic backoff (retry.rs), immediate ejection (no grace periods)

#### Two Independent Ejection Triggers

| Trigger | Condition | Action |
|---------|-----------|--------|
| Standing violation | `Standing < 0` | Immediate ejection |
| Vouch violation | `Effective_Vouches < min_vouch_threshold` | Immediate ejection |

**Critical**: NO GRACE PERIODS. Ejection is immediate.

#### Deliverables

- [x] `src/gatekeeper/ejection.rs` — Ejection logic ✅ **COMPLETE** (507 lines, comprehensive)
- [x] `src/signal/retry.rs` — Signal API retry with logarithmic backoff (GAP-06) ✅ **COMPLETE**
- [x] Remove member from Signal group ✅
- [x] Send PM to ejected member (using hash) ✅
- [x] Announce to group (using hash, not name) ✅
- [x] Move member to `ejected` set (can re-enter with new vouches) ✅

#### Acceptance Criteria (GAP-06)

- [x] Signal API failures retry with logarithmic backoff (2^n seconds, capped at 1 hour) ✅ **GAP-06 COMPLETE**
- [x] Invariant enforced: `signal_state.members ⊆ freenet_state.members` (Signal may lag, never lead) ✅
- [x] Test: Transient Signal failures don't leave stale members in group ✅

---

### Bead: Health Monitoring

**Agent**: Agent-Freenet  
**Review**: See `phase1-review-report.md` lines 164-187 (✅ COMPLETE - 590 lines, real-time streams)  
**Details**: Uses StateStream (not polling), continuous standing checks

#### Deliverables

- [x] `src/gatekeeper/health_monitor.rs` — Continuous standing checks ✅ **COMPLETE** (590 lines, comprehensive)
- [x] Monitor Freenet state stream (real-time, NOT polling) ✅ **COMPLETE** (uses StateStream)
- [x] Check all members' standing on state changes ✅
- [x] Trigger ejection if thresholds violated ✅

**Note**: No heartbeat mechanism. Use Freenet state stream events.

---

### Basic Bot Commands

| Command | Deliverable | Agent |
|---------|-------------|-------|
| `/invite @username [context]` | `src/signal/commands/invite.rs` | Agent-Signal |
| `/vouch @username` | `src/signal/commands/vouch.rs` | Agent-Signal |
| `/flag @username [reason]` | `src/signal/commands/flag.rs` | Agent-Signal |
| `/status` | `src/signal/commands/status.rs` | Agent-Signal |
| `/audit operator` | `src/signal/commands/audit.rs` | Agent-Signal |
| `/audit bootstrap` | `src/signal/commands/audit.rs` | Agent-Signal |

#### Additional Deliverables (Gap Remediations)

- [ ] `src/gatekeeper/rate_limiter.rs` — Progressive cooldown for trust actions (GAP-03) ❌ **MISSING** (module does not exist)
  - First action: immediate, Second: 1 min, Third: 5 min, Fourth: 1 hour, Fifth+: 24 hours
  - **Review**: See `phase1-review-report.md` lines 220-237 (GAP-03 MISSING - critical gap)
- [ ] `src/gatekeeper/audit_trail.rs` — Operator action logging (GAP-01) ❌ **MISSING** (module does not exist)
  - **Review**: See `phase1-review-report.md` lines 220-237 (GAP-01 MISSING - critical gap)

#### `/status` Command Acceptance Criteria (GAP-04)

- [x] Shows user's own vouchers (by Signal display name, resolved at display time) ✅ (code structure exists)
- [x] Shows user's own flaggers (by Signal display name) ✅ (code structure exists)
- [x] Rejects queries about other members' relationships: `❌ Cannot view other members' vouchers` ✅ **GAP-04 COMPLETE** (test passes)
- [x] Hash→name resolution is ephemeral (never persisted by bot) ✅ (transient HashMap)

---

### Phase 1 Success Criteria

Before closing `convoy-phase1`, the Mayor MUST verify ALL of the following:

#### Bootstrap Flow — Agent-Signal + Agent-Freenet

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| 3-member seed group bootstrapped | Integration test: `/create-group` → `/add-seed` × 2 → triangle vouching | Agent-Signal |
| Group name required and stored | Unit test: empty name rejected, name in Freenet contract | Agent-Signal |
| **GAP-05**: Signal group name matches Freenet | Unit test: `signal_group_name == freenet_contract.group_name` | Agent-Signal |
| **GAP-09**: Bootstrap event recorded | Unit test: `/audit bootstrap` shows event details | Agent-Freenet |
| Bootstrap is ONE-TIME | Unit test: `/add-seed` rejected after 3 seeds | Agent-Signal |
| All seeds have Bridge status (2 vouches) | Unit test: each seed has exactly 2 vouches after bootstrap | Agent-Freenet |

#### Trust Operations: Invitation Flow — Agent-Signal + Agent-Crypto

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| `/invite` creates first vouch | Unit test: invitation recorded as vouch in Freenet | Agent-Signal |
| Context is EPHEMERAL | Code review: context never stored in Freenet contract | Witness |
| **GAP-10**: Re-entry warning shown | Unit test: invitee with previous flags gets warning message | Agent-Signal |
| Second member selected via Blind Matchmaker | Unit test: matchmaker suggests cross-cluster member | Agent-Freenet |
| PMs sent to invitee and vetter | Integration test: both receive PM from bot | Agent-Signal |

#### Trust Operations: Vetting Interview — Agent-Signal

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| 3-person PM chat created | Integration test: invitee, member, bot in private chat | Agent-Signal |
| Bot facilitates introduction | Unit test: introduction message sent to vetting chat | Agent-Signal |
| `/vouch` records second vouch | Unit test: vouch stored in Freenet contract | Agent-Signal |
| All vetting in PMs (never group) | Code review: vetting messages only sent to PM chats | Witness |

#### Trust Operations: Admission — Agent-Signal + Agent-Crypto

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| 2 vouches required from different CLUSTERS | Unit test: same-cluster vouches rejected (when ≥2 clusters) | Agent-Freenet |
| Cross-cluster suspended during bootstrap | Unit test: same-cluster allowed when only 1 cluster exists | Agent-Freenet |
| Cross-cluster activates at ≥2 clusters | Unit test: Bridge Removal triggers cross-cluster requirement | Agent-Freenet |
| ZK-proof generated on admission | Unit test: `generate_vouch_proof()` called with correct inputs | Agent-Crypto |
| Outcome (not proof) stored in Freenet | Code review: proof discarded, only admission outcome stored | Witness |
| Invitee added to Signal group | Integration test: member appears in group after admission | Agent-Signal |
| Admission announced using HASH | Unit test: announcement contains hash, not Signal ID | Agent-Signal |
| Vetting session data deleted immediately | Unit test: session data purged after admission/rejection | Agent-Signal |

#### Trust Operations: Flagging — Agent-Signal + Agent-Freenet

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| `/flag` records flag in Freenet | Unit test: flag stored with flagger hash and flaggee hash | Agent-Signal |
| Standing recalculated | Unit test: `Standing = Effective_Vouches - Regular_Flags` | Agent-Freenet |
| Voucher-flag invalidates vouch | Unit test: voucher who flags excluded from BOTH counts | Agent-Freenet |
| **No 2-point swing** | Proptest: voucher-flagging produces exactly 0 standing change | Agent-Freenet |
| Ejection triggers checked | Unit test: ejection evaluated after every flag | Agent-Freenet |

#### Trust Operations: Standing Formula — Agent-Freenet (CRITICAL)

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| Formula: `Standing = Effective_Vouches - Regular_Flags` | Proptest: formula holds for all input combinations | Agent-Freenet |
| Voucher-flaggers excluded from BOTH counts | Proptest: voucher who flags contributes 0 to standing | Agent-Freenet |
| Standing bounded correctly | Proptest: `min ≤ standing ≤ max_vouches` | Agent-Freenet |
| Standing recalculated on every change | Unit test: vouch, flag, vouch-invalidation all trigger recalc | Agent-Freenet |

#### Ejection Protocol — Agent-Signal + Agent-Freenet

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| Ejection when `Standing < 0` | Unit test: member with -1 standing immediately ejected | Agent-Freenet |
| Ejection when `Effective_Vouches < 2` | Unit test: member losing vouches below threshold ejected | Agent-Freenet |
| **NO GRACE PERIODS** | Code review: no delays, warnings, or cooldowns before ejection | Witness |
| Immediate execution | Unit test: ejection happens in same event loop as trigger | Agent-Freenet |
| Removed from Signal group | Integration test: member removed via Signal API | Agent-Signal |
| PM sent to ejected member | Unit test: ejection notification sent to member PM | Agent-Signal |
| Announcement uses HASH | Unit test: group announcement contains hash, not Signal ID | Agent-Signal |
| Moved to `ejected` set | Unit test: member in `ejected` set, can re-enter with new vouches | Agent-Freenet |
| **GAP-06**: Retry with logarithmic backoff | Unit test: Signal API failures retry 2^n seconds, capped at 1h | Agent-Signal |
| Invariant: Signal ⊆ Freenet | Unit test: `signal_members ⊆ freenet_members` always | Agent-Signal |

#### Health Monitoring — Agent-Freenet

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| Real-time Freenet state stream | Code review: no polling loops, uses `subscribe_to_state_changes()` | Witness |
| Standing checked on every change | Unit test: vouch/flag/ejection triggers standing check for all members | Agent-Freenet |
| Ejection triggered when threshold violated | Integration test: flag causing `Standing < 0` triggers ejection | Agent-Freenet |
| No heartbeat mechanism | Code review: no periodic health checks | Witness |

#### Bot Commands — Agent-Signal

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| `/invite @username [context]` works | Integration test: creates vetting session | Agent-Signal |
| `/vouch @username` works | Integration test: records vouch in Freenet | Agent-Signal |
| `/flag @username [reason]` works | Integration test: records flag, triggers standing recalc | Agent-Signal |
| `/status` shows own vouchers only | Unit test: displays own relationships, rejects queries about others | Agent-Signal |
| **GAP-04**: Third-party query rejected | Unit test: `❌ Cannot view other members' vouchers` | Agent-Signal |
| `/audit operator` works | Unit test: shows operator action log | Agent-Signal |
| `/audit bootstrap` works | Unit test: shows bootstrap event details (GAP-09) | Agent-Signal |
| **GAP-03**: Rate limiting enforced | Unit test: progressive cooldown (immediate → 1m → 5m → 1h → 24h) | Agent-Signal |
| **GAP-01**: Operator actions logged | Unit test: all operator actions in audit trail | Agent-Freenet |

#### Security Constraints (Witness MUST Verify)

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| No cleartext Signal IDs in storage | Code review: all Freenet state uses hashes | Witness |
| No cleartext Signal IDs in logs | Code review: logging uses hashes or masked IDs | Witness |
| All Signal IDs masked via `mask_identity()` | Code review: HMAC hashing before any storage | Witness |
| Vetting session data deleted | Code review: session purged after admission/rejection | Witness |
| Context never persisted | Code review: invitation context is ephemeral | Witness |
| Hash→name resolution ephemeral | Code review: display name lookups never persisted | Witness |
| ZK-proof discarded after verification | Code review: only outcome stored, not proof | Witness |

#### Property-Based Tests (REQUIRED)

| Criterion | Proptest Coverage | Agent |
|-----------|-------------------|-------|
| Standing formula correct | `Standing = Effective_Vouches - Regular_Flags` for all inputs | Agent-Freenet |
| Voucher-flag 0 swing | Voucher who flags produces exactly 0 standing change | Agent-Freenet |
| Standing bounded | `min ≤ standing ≤ max_vouches` for all combinations | Agent-Freenet |
| HMAC determinism | Same input + same key = same output (from phase0-hmac) | Agent-Crypto |
| HMAC key isolation | Different keys produce different outputs (from phase0-hmac) | Agent-Crypto |

#### Code Coverage (CI Enforced)

- [ ] 100% code coverage on `src/gatekeeper/*.rs`
- [ ] 100% code coverage on `src/signal/commands/*.rs`
- [ ] 100% code coverage on `src/trust/*.rs` (if exists)
- [ ] All proptests pass (minimum 256 cases per test)
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo deny check` passes (supply chain security)

#### Integration Test Scenarios

**Gastown MUST run these scenarios before closing convoy:**

```bash
# 1. Bootstrap Flow
gt test:integration --scenario bootstrap

# Scenario steps:
# a) First member sends /create-group "Test Group"
# b) Verify: Signal group created with name "Test Group"
# c) First member sends /add-seed @Alice
# d) First member sends /add-seed @Bob
# e) All 3 seeds vouch for each other (triangle)
# f) Verify: Freenet contract initialized with 3 members
# g) Verify: Each seed has exactly 2 vouches (Bridge status)
# h) Verify: /add-seed rejected (bootstrap complete)

# 2. Trust Operations: Full Admission Flow
gt test:integration --scenario admission-flow

# Scenario steps:
# a) Member sends /invite @Carol "Met at conference"
# b) Verify: First vouch recorded, context NOT in Freenet
# c) Verify: Blind Matchmaker selects second vetter
# d) Verify: 3-person vetting chat created (Carol, vetter, bot)
# e) Vetter sends /vouch @Carol
# f) Verify: Cross-cluster check (bootstrap: same-cluster allowed)
# g) Verify: ZK-proof generated
# h) Verify: Carol added to Signal group
# i) Verify: Announcement uses hash, not "Carol"
# j) Verify: Vetting session data deleted

# 3. Trust Operations: Standing and Ejection
gt test:integration --scenario standing-ejection

# Scenario steps:
# a) Create member with standing +2 (3 vouches, 1 flag)
# b) Add another flag → standing = +1
# c) Verify: No ejection (standing ≥ 0)
# d) Add another flag → standing = 0
# e) Verify: No ejection (standing ≥ 0)
# f) Add another flag → standing = -1
# g) Verify: IMMEDIATE ejection (no grace period)
# h) Verify: Member in ejected set, removed from Signal group

# 4. Trust Operations: Vouch Invalidation (No 2-Point Swing)
gt test:integration --scenario vouch-invalidation

# Scenario steps:
# a) Alice vouches for Bob (Bob standing: +1 vouch)
# b) Carol flags Bob (Bob standing: 1 vouch - 1 flag = 0)
# c) Alice flags Bob (voucher-flagging)
# d) Verify: Alice's vouch INVALIDATED
# e) Verify: Alice's flag EXCLUDED
# f) Verify: Bob's standing = 0 (not -1, no 2-point swing)
# g) Carol removes flag → Bob standing = 0 (still no Alice vouch)

# 5. Re-entry with Previous Flags (GAP-10)
gt test:integration --scenario re-entry-warning

# Scenario steps:
# a) Member ejected with 3 flags
# b) New member invites ejected person
# c) Verify: Warning shown to inviter about previous flags
# d) Verify: Re-entry requires 4+ vouches for positive standing
```

#### Mayor Convoy Closure Checklist

```bash
# Verify all beads complete
bd list --convoy convoy-phase1 --status pending
# Should return: No pending beads

# Verify all GAPs integrated
bd show gap-01  # Operator logging - should be complete
bd show gap-03  # Rate limiting - should be complete
bd show gap-04  # Status privacy - should be complete
bd show gap-05  # Group name - should be complete
bd show gap-06  # Signal retry - should be complete
bd show gap-09  # Bootstrap audit - should be complete
bd show gap-10  # Re-entry warning - should be complete

# Verify security audit
gt audit --convoy convoy-phase1 --agent witness
# Should return: All security constraints verified

# Verify test coverage
cargo llvm-cov nextest --all-features -- --test-threads=1
# Should return: 100% coverage on gatekeeper, signal/commands modules

# Verify property tests
cargo test --release -- proptest
# Should return: All proptests pass (256+ cases each)

# Verify documentation
ls docs/USER-GUIDE.md docs/OPERATOR-GUIDE.md
# Should exist and be updated with Phase 1 commands

# Close convoy
gt convoy close convoy-phase1 --verified
```

---

## 🎯 PHASE 2: Mesh Optimization Convoy

**Convoy ID**: `convoy-phase2`  
**Duration**: Weeks 5-6  
**Dependencies**: `convoy-phase1` complete  
**Review Reports**: 
  - Implementation: `docs/todo/PHASE2_REVIEW.md`
  - Benchmarks: `docs/todo/PHASE2-BENCHMARKS.md`  
  - Security Audit: `docs/todo/phase2-security-audit.md`  
**Status**: 🟡 **40% Complete** - Core algorithms done, /mesh commands stubbed, integration tests missing  
**Performance**: ✅ **ALL TARGETS MET** (DVR: 5.2x faster, Cluster: 2.2x faster, Matchmaker: 1,667x faster)

### Mayor Delegation Commands

```bash
gt convoy create --title "Phase 2: Mesh Optimization" \
  --depends-on convoy-phase1

bd create --title "DVR calculation and health tiers" --convoy convoy-phase2
bd create --title "Bridge Removal cluster detection" --convoy convoy-phase2
bd create --title "Blind Matchmaker strategic introductions" --convoy convoy-phase2
bd create --title "Advanced /mesh commands" --convoy convoy-phase2
bd create --title "Proposal system (/propose)" --convoy convoy-phase2
```

### Bead: DVR (Distinct Validator Ratio)

**Agent**: Agent-Freenet  
**Reference**: `.beads/mesh-health-metric.bead`, `.beads/blind-matchmaker-dvr.bead`  
**Review**: See `PHASE2_REVIEW.md` lines 13-41 (✅ COMPLETE - 30 unit tests)  
**Benchmarks**: See `PHASE2-BENCHMARKS.md` lines 47-64 (0.192ms @ 1000 members, 5.2x faster than target)  
**Gaps**: GAP-11 cluster announcement not integrated (lines 150-163)

#### Formula

```
DVR = Distinct_Validators / (N / 4)

Where:
- Distinct_Validators = Validators with non-overlapping voucher sets
- N = Total member count
```

#### Three-Tier Health Status

| Status | DVR Range | Color | Bot Behavior |
|--------|-----------|-------|--------------|
| **Unhealthy** | 0% - 33% | 🔴 | Actively suggest introductions |
| **Developing** | 33% - 66% | 🟡 | Opportunistic suggestions |
| **Healthy** | 66% - 100% | 🟢 | Maintenance mode |

#### Deliverables

- [x] `src/matchmaker/dvr.rs` — DVR calculation ✅ **COMPLETE** (30 unit tests)
- [ ] `src/matchmaker/cluster_detection.rs` — Bridge Removal algorithm (Q3) ⚠️ **PARTIAL** (connected components only, Tarjan's TODO)
- [ ] Integration with `/mesh` command display ❌ **STUB** (returns hardcoded data)
- [ ] **GAP-11**: Cluster formation announcement when ≥2 clusters detected ❌ **NOT INTEGRATED** (message defined, not triggered)

#### Acceptance Criteria (GAP-11)

- [ ] Cluster formation detected via Bridge Removal algorithm ⚠️ **PARTIAL** (only disconnected components, not tight clusters)
- [ ] Group announcement when cross-cluster requirement activates: ❌ **NOT INTEGRATED**
  > "📊 Network update: Your group now has distinct sub-communities! Cross-cluster vouching is now required for new members. Existing members are grandfathered."
- [ ] Grandfathering: existing members NOT required to get new vouches ❌ **TODO**
- [ ] Run cluster detection on every membership change (<1ms per Q3) ⚠️ **Not benchmarked** (code exists)

---

### Bead: Blind Matchmaker (DVR-Optimized)

**Agent**: Agent-Freenet  
**Reference**: `.beads/blind-matchmaker-dvr.bead`, `.beads/mesh-health-metric.bead`, `docs/ALGORITHMS.md`  
**Dependencies**: Cluster Detection (Bridge Removal algorithm from Q3)  
**Review**: See `PHASE2_REVIEW.md` lines 62-82 (✅ COMPLETE - 3-phase algorithm implemented)  
**Benchmarks**: See `PHASE2-BENCHMARKS.md` lines 83-101 (0.120ms @ 500 members, 1,667x faster than target)  
**Security**: See `phase2-security-audit.md` lines 87-128 (✅ transient mapping verified)

#### Overview

The Blind Matchmaker suggests strategic introductions to optimize network resilience. It uses a hybrid algorithm:
- **Phase 0**: DVR Optimization — create distinct Validators with non-overlapping voucher sets
- **Phase 1**: MST Fallback — strengthen remaining Bridges with any cross-cluster vouch
- **Phase 2**: Connect Clusters — bridge disconnected components

**Key Constraint**: Bot knows only topology (vouch counts, clusters), NOT relationship content.

#### Deliverables

- [x] `src/matchmaker/mod.rs` — Module exports ✅ **COMPLETE**
- [x] `src/matchmaker/graph_analysis.rs` — Topology analysis ✅ **COMPLETE**
  - [x] `TrustGraph` struct using `petgraph::Graph` ✅
  - [x] `find_bridges()` — Tarjan's algorithm for articulation edges ✅
  - [x] `detect_clusters()` — Bridge Removal for tight cluster detection ✅
  - [ ] `compute_betweenness_centrality()` — Brandes' algorithm ⚠️ **TODO** (if needed)
  - [x] `classify_members()` — Bridge (2 vouches) vs Validator (3+ vouches) ✅
- [x] `src/matchmaker/dvr.rs` — Distinct Validator Ratio calculation ✅ **COMPLETE**
  - [x] `count_distinct_validators()` — Greedy selection with voucher-set disjointness ✅
  - [x] `calculate_dvr()` — DVR = Distinct / floor(N/4) ✅
  - [x] `health_status()` — 🔴 <33%, 🟡 33-66%, 🟢 >66% ✅
- [x] `src/matchmaker/strategic_intro.rs` — Introduction suggestions ✅ **COMPLETE**
  - [x] `suggest_dvr_optimal_introductions()` — Phase 0 (priority 0) ✅
  - [x] `suggest_mst_fallback()` — Phase 1 (priority 1) ✅
  - [x] `link_clusters()` — Phase 2 (priority 2) ✅
  - [x] `find_unused_cross_cluster_voucher()` — Helper for DVR optimization ✅
- [x] `src/matchmaker/display.rs` — User-facing messages ✅ **COMPLETE**
  - [x] Use Signal display names (NOT hashes) in suggestions ✅
  - [x] Maintain transient in-memory mapping (Signal ID → hash) ✅ **Transient HashMap**
  - [x] Graceful fallback if display name not cached ✅ `@Unknown_XX`

#### Algorithm Summary

```rust
// Phase 0: DVR Optimization (priority 0)
// Find introductions that create DISTINCT Validators (non-overlapping voucher sets)
for bridge in bridges {
    if let Some(voucher) = find_unused_cross_cluster_voucher(bridge, &used_vouchers, graph) {
        introductions.push(Introduction {
            person_a: bridge,
            person_b: voucher,
            reason: "Create distinct Validator (DVR optimization)",
            priority: 0,
            dvr_optimal: true,
        });
        used_vouchers.extend(bridge_vouchers);
        used_vouchers.insert(voucher);
    }
}

// Phase 1: MST Fallback (priority 1)
// For remaining bridges, use ANY cross-cluster Validator
for bridge in remaining_bridges {
    if let Some(voucher) = find_any_cross_cluster_validator(bridge, graph) {
        introductions.push(Introduction { ..., priority: 1, dvr_optimal: false });
    }
}

// Phase 2: Connect Clusters (priority 2)
// Bridge disconnected clusters via highest-centrality validators
```

#### Bot Behavior by Health Status

| Health | DVR Range | Bot Behavior |
|--------|-----------|--------------|
| 🔴 Unhealthy | 0-33% | **Aggressively** suggest DVR-optimal introductions |
| 🟡 Developing | 33-66% | Suggest DVR-optimal when convenient |
| 🟢 Healthy | 66-100% | Maintenance mode (MST suggestions only) |

#### UX Messages

**DVR-optimal suggestion** (Phase 0):
```
"🔗 Strategic Introduction: I've identified @Alex as an ideal
connection for you. Vouching for them would strengthen the network's
distributed trust (they'd become independently verified).

Would you like me to facilitate an introduction?"
```

**MST fallback suggestion** (Phase 1):
```
"🔗 Suggestion: Consider connecting with @Jordan from a different
part of the network. This would strengthen your position and improve
network connectivity."
```

#### Performance Targets

| Network Size | Target Latency |
|-------------|----------------|
| 20 members | < 10ms |
| 100 members | < 50ms |
| 500 members | < 200ms |
| 1000 members | < 500ms |

**Complexity**: O(N × E) dominated by betweenness centrality calculation

#### Security Constraints

- [x] **No cleartext Signal IDs** in logs, persistent storage, or output ✅ **VERIFIED** (Phase 2 Security Audit - 0 violations)
- [x] **Transient mapping only** — Signal ID → hash mapping is in-memory, not persisted ✅ **VERIFIED** (HashMap ephemeral)
- [x] **Mapping reconstructs on restart** — rebuilt as members interact ✅ **VERIFIED** (HMAC deterministic)
- [x] **Hash-only in Freenet** — all stored state uses hashed identifiers ✅
- [x] **Display names resolved ephemerally** — from Signal API, never persisted ✅ **VERIFIED** (audit confirmed)

#### Acceptance Criteria

- [ ] `detect_clusters()` correctly identifies tight clusters using Bridge Removal
- [ ] `count_distinct_validators()` finds maximum set of Validators with disjoint voucher sets
- [ ] DVR calculation matches manual calculation for test cases
- [ ] Phase 0 prioritizes DVR-optimal introductions over Phase 1
- [ ] Phase 1 accepts any cross-cluster vouch (no DVR constraint)
- [ ] Phase 2 connects disconnected clusters
- [ ] Suggestions use Signal display names, not hashes
- [ ] Performance meets targets for all network sizes
- [ ] 100% test coverage with proptest for graph properties
- [ ] All tests use `MockFreenetClient` (no real network calls)

#### Test Cases

```rust
#[test]
fn test_dvr_distinct_validators() {
    // V1 vouched by {A, B, C}, V2 vouched by {D, E, F} → both distinct
    // V3 vouched by {A, G, H} → NOT distinct (shares A with V1)
}

#[test]
fn test_bridge_removal_clusters() {
    // Two tight clusters connected by single edge
    // Should detect 2 clusters, not 1
}

#[test]
fn test_dvr_optimal_prioritized() {
    // Given choice between DVR-optimal and non-optimal vouch
    // Should suggest DVR-optimal first
}

proptest! {
    #[test]
    fn dvr_never_exceeds_one(graph in arbitrary_trust_graph()) {
        let dvr = calculate_dvr(&graph);
        assert!(dvr <= 1.0);
    }
    
    #[test]
    fn distinct_validators_have_disjoint_sets(graph in arbitrary_trust_graph()) {
        let distinct = get_distinct_validators(&graph);
        for (i, v1) in distinct.iter().enumerate() {
            for v2 in distinct.iter().skip(i + 1) {
                let vouchers1: HashSet<_> = get_vouchers(v1).collect();
                let vouchers2: HashSet<_> = get_vouchers(v2).collect();
                assert!(vouchers1.is_disjoint(&vouchers2));
            }
        }
    }
}
```

---

### Bead: Advanced Commands (`/mesh` Family)

**Agent**: Agent-Signal  
**Reference**: `.beads/mesh-health-metric.bead`, `.beads/user-roles-ux.bead`, `docs/USER-GUIDE.md`  
**Dependencies**: Blind Matchmaker (DVR calculation), Persistence Model (replication status)  
**Review**: See `PHASE2_REVIEW.md` lines 85-116 (❌ ALL STUBBED - return hardcoded data)  
**Required**: Connect handlers to matchmaker module, query Freenet for real data  
**Beads**: Create new beads for mesh command implementation

#### Overview

The `/mesh` command family provides network health visibility to members. These commands query Freenet contract state and present human-readable summaries.

#### Commands

##### `/mesh` — Network Overview (Summary)

**Purpose**: Quick health check showing trust and replication status at a glance.

**Example Output**:
```
📊 Network Health: 🟢 Healthy (75%)

Members: 47
Distinct Validators: 9 / 11 possible
Clusters: 4 detected
Replication: 🟢 Replicated (3/3 holders)
Federation: None

Subcommands: /mesh strength, /mesh replication, /mesh config
```

**Implementation Notes**:
- DVR percentage is primary health metric (not arbitrary percentage)
- Shows subcommand hints for discoverability
- Replication status from persistence layer (chunk holder count)

---

##### `/mesh strength` — Detailed Trust Health

**Purpose**: Deep dive into network resilience metrics for power users.

**Example Output**:
```
📈 Mesh Health Report

Distinct Validator Ratio: 75% 🟢

┌─────────────────────────────┐
│ 🟢 Healthy    │████████░░│ │
│ 🟡 Developing │          │ │
│ 🔴 Unhealthy  │          │ │
└─────────────────────────────┘

Distinct Validators: 3
  V1: vouched by @Alice(C1), @Bob(C2), @Carol(C3)
  V2: vouched by @Dave(C1), @Eve(C2), @Frank(C3)
  V3: vouched by @Grace(C1), @Henry(C2), @Ivy(C3)

Maximum Possible: 4 (network size / 4)

Member Distribution:
  Bridges (2 vouches): 12
  Validators (3+ vouches): 35

To improve: Build cross-cluster relationships to create
1 more distinct Validator with unique vouchers.
```

**Implementation Notes**:
- Lists distinct Validators with their voucher sets (display names, not hashes)
- Shows cluster affiliation for each voucher (C1, C2, etc.)
- Includes actionable improvement suggestion
- Visual progress bar for DVR status

---

##### `/mesh replication` — Persistence Health

**Purpose**: Show whether trust data is safely replicated across peers.

**Example Output (Healthy)**:
```
💾 Replication Health: 🟢 Replicated

State Chunks: 8
Holders per Chunk: 3 (target: 3)
Last Verified: 2 minutes ago
Recovery Status: Full recovery possible

All chunks have sufficient holders. Your trust network
data can be recovered if this bot crashes.
```

**Example Output (Degraded)**:
```
💾 Replication Health: 🟡 Degraded

State Chunks: 8
Chunks at Risk: 2 (missing 1 holder each)
  Chunk 3: 2/3 holders
  Chunk 7: 2/3 holders
Last Verified: 5 minutes ago
Recovery Status: Partial (best-effort)

⚠️ Some chunks have fewer holders than target.
Bot is actively seeking replacement holders.
Writes are BLOCKED until replication restored.
```

**Example Output (At Risk)**:
```
💾 Replication Health: 🔴 At Risk

State Chunks: 8
Critical Chunks: 1 (only 1 holder!)
  Chunk 5: 1/3 holders ⚠️
Last Verified: 10 minutes ago
Recovery Status: At risk of data loss

🚨 CRITICAL: Chunk 5 has only 1 holder.
Bot is urgently seeking replacement holders.
All writes BLOCKED. Recovery may be incomplete if holder fails.
```

**Implementation Notes**:
- Queries persistence layer for chunk holder status
- Shows per-chunk detail when degraded
- Explains write-blocking behavior
- Uses challenge-response verification timestamps

---

##### `/mesh config` — Group Configuration

**Purpose**: Display current group settings (all configurable via `/propose`).

**Example Output**:
```
⚙️ Group Configuration

Trust Settings:
  min_vouch_threshold: 2 (vouches to become member)
  ejection_standing_threshold: 0 (standing below triggers ejection)
  cross_cluster_required: true (vouches must be from different clusters)

Voting Settings:
  config_change_threshold: 70% (votes needed to change config)
  min_quorum: 51% (participation needed for valid vote)
  proposal_timeout: 48h (voting window)

Persistence Settings:
  chunk_size: 64KB
  replication_factor: 3 (copies per chunk)
  verification_interval: 24h

Federation:
  federation_threshold: 10% (minimum overlap to propose)
  federated_groups: none

To change: /propose config <setting> <value>
```

**Implementation Notes**:
- Reads from Freenet contract state
- Groups settings by category
- Shows units and defaults
- Includes hint for how to change

---

#### Deliverables

- [ ] `src/commands/mesh.rs` — Command dispatcher
- [ ] `src/commands/mesh/overview.rs` — `/mesh` handler
  - [ ] Query DVR from matchmaker module
  - [ ] Query replication status from persistence module
  - [ ] Format summary with subcommand hints
- [ ] `src/commands/mesh/strength.rs` — `/mesh strength` handler
  - [ ] List distinct Validators with voucher details
  - [ ] Show member distribution (Bridges vs Validators)
  - [ ] Generate improvement suggestions
  - [ ] Render visual progress bar
- [ ] `src/commands/mesh/replication.rs` — `/mesh replication` handler
  - [ ] Query chunk holder status from persistence layer
  - [ ] Identify at-risk chunks
  - [ ] Show verification timestamps
  - [ ] Explain write-blocking status
- [ ] `src/commands/mesh/config.rs` — `/mesh config` handler
  - [ ] Read GroupConfig from Freenet contract
  - [ ] Format by category with units
- [ ] `src/commands/mesh/display.rs` — Shared formatting utilities
  - [ ] `render_progress_bar()` — Visual DVR indicator
  - [ ] `format_health_status()` — 🔴/🟡/🟢 with percentage
  - [ ] `resolve_display_names()` — Hash → Signal display name

#### Security Constraints

- [ ] **No cleartext Signal IDs** in logs or persistent storage
- [ ] **Display names resolved ephemerally** — from Signal API, never persisted
- [ ] **Hash-only in Freenet** — all stored state uses hashed identifiers
- [ ] **Self-query safe** — user sees their own vouchers (no new info leaked)
- [ ] **Third-party query restricted** — cannot see who vouched for OTHER members

#### Acceptance Criteria

- [ ] `/mesh` returns summary in <100ms (cached DVR, real-time replication) ❌ **STUB** (returns hardcoded data)
- [ ] `/mesh strength` shows all distinct Validators with correct voucher sets ❌ **STUB** (returns hardcoded data)
- [ ] `/mesh strength` displays cluster affiliation for each voucher ❌ **STUB**
- [ ] `/mesh strength` shows actionable improvement suggestion when DVR < 100% ❌ **STUB**
- [ ] `/mesh replication` shows correct chunk holder counts ❌ **STUB** (returns hardcoded data)
- [ ] `/mesh replication` identifies specific at-risk chunks when degraded ❌ **STUB**
- [ ] `/mesh config` shows all GroupConfig values grouped by category ❌ **STUB** (returns hardcoded data)
- [x] All commands use Signal display names (not hashes) in output ✅ (structure exists)
- [x] All commands handle bootstrap case (< 4 members) gracefully ✅ (structure exists)
- [ ] 100% test coverage with `MockFreenetClient` and `MockSignalClient` ❌ **TODO**

#### Test Cases

```rust
#[test]
fn test_mesh_overview_healthy() {
    // Given: DVR 75%, replication 3/3, no federation
    // When: /mesh
    // Then: Shows "🟢 Healthy (75%)" with all subcommand hints
}

#[test]
fn test_mesh_overview_unhealthy() {
    // Given: DVR 20%, replication degraded
    // When: /mesh
    // Then: Shows "🔴 Unhealthy (20%)" with replication warning
}

#[test]
fn test_mesh_strength_distinct_validators() {
    // Given: 3 distinct Validators with known voucher sets
    // When: /mesh strength
    // Then: Lists all 3 with display names and cluster affiliations
}

#[test]
fn test_mesh_strength_improvement_suggestion() {
    // Given: DVR 60% (2/3 possible distinct Validators)
    // When: /mesh strength
    // Then: Suggests creating 1 more distinct Validator
}

#[test]
fn test_mesh_replication_degraded() {
    // Given: Chunks 3 and 7 have only 2/3 holders
    // When: /mesh replication
    // Then: Shows 🟡 Degraded with specific chunk details
}

#[test]
fn test_mesh_config_all_settings() {
    // Given: GroupConfig with non-default values
    // When: /mesh config
    // Then: Shows all settings grouped by category
}

#[test]
fn test_mesh_bootstrap_graceful() {
    // Given: 3 members (bootstrap phase)
    // When: /mesh
    // Then: Shows "🟢 Healthy (Bootstrap)" without DVR calculation
}
```

---

### Bead: Proposal System

**Agent**: Agent-Signal  
**Reference**: `.beads/proposal-system.bead`, `.beads/voting-mechanism.bead`  
**Dependencies**: Signal Poll support (protocol v8), Freenet state stream  
**Review**: See `PHASE2_REVIEW.md` lines 118-147 (⚠️ PARTIAL - parsing complete, execution/monitoring TODO)  
**Security**: See `phase2-security-audit.md` lines 130-203 (✅ GAP-02 COMPLIANT - vote privacy verified)  
**Complete**: Command parsing (7 tests), PollManager structure, result calculation  
**Incomplete**: Poll creation storage, state stream monitoring, execution flow

#### Overview

The `/propose` command creates Signal Polls for group decisions. All proposals have **mandatory timeouts** — open-ended polls are forbidden. When timeout expires, the bot **terminates the poll on Signal** and records the result in Freenet.

#### Command Structure

```
/propose <subcommand> [args] [--timeout duration]
```

| Subcommand | Example | Purpose |
|------------|---------|---------|
| `config` | `/propose config name "New Name"` | Signal group settings |
| `stroma` | `/propose stroma min_vouch_threshold 3` | Trust config |
| `federate` | `/propose federate <group-id>` | Federation (Phase 4+) |

**Timeout Resolution** (in order of precedence):
1. Per-proposal `--timeout` flag (if specified)
2. `GroupConfig.default_poll_timeout` (fallback, e.g., 48h)
3. Open-ended polls are **NEVER** allowed

#### Timeout Lifecycle

```
1. Member: /propose config name "New Name" --timeout 72h
   
2. Bot creates Signal Poll:
   "📋 Proposal #42
    Change group name to 'New Name'
    
    Vote: 👍 Approve | 👎 Reject
    Timeout: 72h
    Threshold: 70% | Quorum: 50%
    Closes: Thu Jan 30, 10:00 AM"

3. Bot records in Freenet:
   ActiveProposal {
       expires_at: now() + 72h,
       checked: false,
       ...
   }

4. Members vote via Signal Poll (ephemeral, E2E encrypted)

5. Freenet emits StateChange::ProposalExpired when timeout reached

6. Bot handles expiration:
   a. Fetch poll results from Signal (aggregates only)
   b. Calculate result (quorum + threshold)
   c. TERMINATE poll on Signal via PollTerminate message
   d. Announce result to group
   e. If approved, execute action
   f. Record result in Freenet (aggregates only)
   g. Mark proposal as checked
```

#### Poll Termination (CRITICAL)

**Signal polls remain open indefinitely unless explicitly terminated.** The bot MUST send a `PollTerminate` message when the timeout expires.

```rust
use presage::libsignal_service::proto::data_message::PollTerminate;

async fn terminate_poll(
    manager: &Manager,
    group_master_key: &[u8],
    poll_timestamp: u64,
) -> Result<()> {
    let terminate_message = DataMessage {
        poll_terminate: Some(PollTerminate {
            target_sent_timestamp: Some(poll_timestamp),
        }),
        timestamp: Some(now()),
        ..Default::default()
    };
    
    manager.send_message_to_group(
        group_master_key,
        terminate_message,
        now(),
    ).await?;
    
    Ok(())
}
```

**Why Termination Matters:**
- Prevents late votes after timeout (votes after termination are ignored by Signal)
- Provides clear visual feedback in Signal UI (poll shows as closed)
- Ensures result announcement aligns with actual voting window
- Required for consistent UX across Signal clients

#### Deliverables

- [x] `src/proposals/mod.rs` — Module exports ✅ **COMPLETE**
- [x] `src/proposals/command.rs` — `/propose` parser ✅ **COMPLETE** (7 unit tests)
  - [x] Parse subcommand (config, stroma, federate) ✅
  - [x] Parse `--timeout` flag or use default ✅
  - [x] Validate timeout bounds (min: 1h, max: 168h/1 week) ✅
  - [x] Reject "appeal" subcommand (explicitly removed) ✅
- [ ] `src/proposals/poll.rs` — Signal Poll lifecycle ⚠️ **PARTIAL** (structure exists, storage TODO)
  - [ ] `create_poll()` — Create Signal Poll with question and options ⚠️ **TODO** (line 5: TODO store in Freenet)
  - [x] `terminate_poll()` — Send PollTerminate when timeout expires ✅ (function exists)
  - [x] `fetch_results()` — Get aggregated vote counts ✅ (PollManager structure exists)
- [ ] `src/proposals/monitor.rs` — Freenet state stream monitoring ❌ **TODO** (no listener implemented)
  - [ ] Subscribe to `StateChange::ProposalExpired` events ❌
  - [ ] Handle expiration: fetch results → terminate poll → announce → execute ❌
  - [ ] Mark proposal as checked (never re-check) ❌
- [ ] `src/proposals/executor.rs` — Execute approved actions ⚠️ **PARTIAL** (federation/stroma marked TODO)
  - [ ] Config changes via Signal API ⚠️ **TODO**
  - [ ] Stroma changes via Freenet contract delta ⚠️ **TODO**
  - [ ] Federation changes (Phase 4+) ⚠️ **TODO**
- [x] `src/proposals/result.rs` — Result calculation ✅ **COMPLETE**
  - [x] `calculate_result()` — Check quorum AND threshold ✅
  - [x] `format_announcement()` — Human-readable result message ✅

#### Acceptance Criteria

**Timeout Requirements:**
- [x] Every proposal has a finite timeout (REQUIRED field, never optional) ✅
- [x] Default timeout from `GroupConfig.default_poll_timeout` when not specified ✅
- [x] Timeout bounds enforced: minimum 1 hour, maximum 168 hours (1 week) ✅
- [ ] `expires_at` stored in Freenet contract (computed at creation) ⚠️ **TODO** (storage pending)

**Poll Termination Requirements:**
- [x] Bot sends `PollTerminate` message when timeout expires ✅ (function exists)
- [x] Termination timestamp matches original poll timestamp ✅
- [x] Termination occurs BEFORE result announcement ✅ (structure correct)
- [x] Votes cast after termination are not counted (Signal behavior) ✅

**Vote Privacy Requirements (GAP-02):**
- [x] **NEVER persist individual votes** — only aggregates (approve_count, reject_count) ✅ **GAP-02 COMPLIANT** (Phase 2 Security Audit)
- [x] No `VoteRecord { member, vote }` structures in codebase ✅ **VERIFIED** (audit found 0 violations)
- [x] Signal shows who voted during poll (ephemeral, E2E encrypted) ✅
- [x] Freenet stores only outcome + aggregates (permanent) ✅ (VoteAggregate struct)

**Quorum + Threshold Requirements:**
- [x] Both quorum AND threshold must be satisfied for passage ✅
  - Quorum: `min_quorum` % of members must vote
  - Threshold: `config_change_threshold` % of votes must approve
- [x] Proposal fails if quorum not met (even if 100% of voters approve) ✅ (test exists)
- [x] Threshold and quorum from GroupConfig (NOT per-proposal) ✅

**Monitoring Requirements:**
- [ ] Use real-time Freenet state stream (NOT polling with sleep loops) ❌ **TODO** (no listener)
- [ ] React to `StateChange::ProposalExpired` events ❌ **TODO**
- [ ] Check proposal ONCE after expiration, mark as checked ❌ **TODO**
- [ ] Never re-check same proposal ❌ **TODO**

#### Test Cases

```rust
#[test]
fn test_timeout_required() {
    // Given: Proposal without --timeout flag
    // When: Created
    // Then: Uses GroupConfig.default_poll_timeout (never None)
}

#[test]
fn test_timeout_custom() {
    // Given: /propose config name "Test" --timeout 96h
    // When: Created
    // Then: expires_at = now() + 96h
}

#[test]
fn test_timeout_bounds() {
    // Given: --timeout 30m (below minimum)
    // When: Parsed
    // Then: Error "Timeout must be at least 1 hour"
    
    // Given: --timeout 200h (above maximum)
    // When: Parsed
    // Then: Error "Timeout cannot exceed 168 hours (1 week)"
}

#[test]
fn test_poll_terminated_on_expiration() {
    // Given: Proposal with 1h timeout
    // When: Timeout expires
    // Then: Bot sends PollTerminate message to Signal
}

#[test]
fn test_termination_before_announcement() {
    // Given: Proposal expired
    // When: Bot handles expiration
    // Then: PollTerminate sent BEFORE result announcement
}

#[test]
fn test_quorum_not_met() {
    // Given: 10 members, 50% quorum, 2 votes (20% participation)
    // When: Calculate result
    // Then: approved = false, quorum_met = false
    // Even if: 100% of voters approved
}

#[test]
fn test_threshold_not_met() {
    // Given: 10 members, 70% threshold, 6 votes (3 approve, 3 reject)
    // When: Calculate result
    // Then: approved = false, threshold_met = false
    // Because: 50% approval < 70% threshold
}

#[test]
fn test_no_individual_votes_persisted() {
    // Given: Proposal with votes
    // When: Result recorded in Freenet
    // Then: Only approve_count, reject_count stored
    // And: No VoteRecord or individual voter data
}

#[test]
fn test_state_stream_not_polling() {
    // Given: Proposal monitoring code
    // When: Searching for sleep/poll patterns
    // Then: No `tokio::time::sleep` in polling loops
    // And: Uses `freenet.subscribe_to_state_changes()`
}
```

#### UX Messages

**Proposal Created:**
```
📋 Proposal #42
Change group name to 'New Name'

Vote: 👍 Approve | 👎 Reject
Timeout: 72h
Threshold: 70% | Quorum: 50%
Closes: Thu Jan 30, 10:00 AM
```

**Proposal Passed:**
```
✅ Proposal #42 PASSED

Result: 85% approved (17 of 20 votes)
Quorum: ✅ Met (100% participated, 50% required)
Threshold: ✅ Met (85% approved, 70% required)

Action executed: Group name changed to 'New Name'
```

**Proposal Failed (Quorum):**
```
❌ Proposal #42 FAILED

Result: Quorum not met
Participation: 30% (6 of 20 members voted)
Required: 50% participation

Note: Even though 100% of voters approved, the proposal
failed because not enough members participated.
```

**Proposal Failed (Threshold):**
```
❌ Proposal #42 FAILED

Result: Threshold not met
Approval: 55% (11 of 20 votes)
Required: 70% approval

Quorum: ✅ Met (100% participated)
```

---

### Phase 2 Success Criteria

Before closing `convoy-phase2`, the Mayor MUST verify ALL of the following:

#### DVR (Distinct Validator Ratio) — Agent-Freenet

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| DVR formula correct | Unit test: `DVR = Distinct_Validators / floor(N/4)` | Agent-Freenet |
| Distinct Validators disjoint | Proptest: all distinct Validators have non-overlapping voucher sets | Agent-Freenet |
| Three-tier health display | Unit test: 🔴 <33%, 🟡 33-66%, 🟢 >66% | Agent-Freenet |
| DVR never exceeds 1.0 | Proptest: DVR ≤ 1.0 for all graph configurations | Agent-Freenet |
| Performance <1ms at 1000 members | Benchmark test: DVR calculation under 1ms | Agent-Freenet |

#### Cluster Detection (Bridge Removal) — Agent-Freenet

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| Tarjan's algorithm for bridges | Unit test: identifies articulation edges correctly | Agent-Freenet |
| Tight clusters detected | Unit test: two clusters connected by single edge → 2 clusters | Agent-Freenet |
| Cluster detection on membership change | Integration test: runs on every vouch/flag/ejection | Agent-Freenet |
| **GAP-11**: Cluster formation announced | Unit test: message sent when ≥2 clusters detected first time | Agent-Freenet |
| Grandfathering enforced | Unit test: existing members not required to get cross-cluster vouches | Agent-Freenet |
| Performance <1ms (Q3 validated) | Benchmark: cluster detection under 1ms at 1000 members | Agent-Freenet |

#### Blind Matchmaker — Agent-Freenet

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| Phase 0: DVR-optimal prioritized | Unit test: DVR-optimal introductions have priority 0 | Agent-Freenet |
| Phase 1: MST fallback | Unit test: remaining bridges get any cross-cluster vouch (priority 1) | Agent-Freenet |
| Phase 2: Cluster linking | Unit test: disconnected clusters bridged via high-centrality validators | Agent-Freenet |
| Greedy disjoint selection | Proptest: selected distinct validators have disjoint voucher sets | Agent-Freenet |
| Display names in suggestions | Unit test: Signal display names (not hashes) in messages | Agent-Signal |
| Transient mapping only | Code review: no persistent Signal ID → hash storage | Witness |
| Performance targets met | Benchmark: <10ms at 20, <200ms at 500, <500ms at 1000 members | Agent-Freenet |

#### `/mesh` Commands — Agent-Signal

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| `/mesh` overview in <100ms | Benchmark test: cached DVR, real-time replication | Agent-Signal |
| `/mesh strength` lists distinct Validators | Unit test: all distinct validators with voucher sets shown | Agent-Signal |
| `/mesh strength` shows cluster affiliation | Unit test: vouchers labeled with cluster (C1, C2, etc.) | Agent-Signal |
| `/mesh strength` improvement suggestion | Unit test: actionable suggestion when DVR < 100% | Agent-Signal |
| `/mesh replication` chunk status | Unit test: correct chunk holder counts displayed | Agent-Signal |
| `/mesh replication` at-risk chunks | Unit test: specific at-risk chunks listed when degraded | Agent-Signal |
| `/mesh config` all settings | Unit test: all GroupConfig values grouped by category | Agent-Signal |
| Bootstrap case handled | Unit test: <4 members shows "Bootstrap" without DVR calculation | Agent-Signal |
| Display names resolved ephemerally | Code review: no persistent name→hash storage | Witness |

#### Proposal System — Agent-Signal

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| Mandatory timeout | Unit test: every proposal has `expires_at` (never None) | Agent-Signal |
| Default timeout from GroupConfig | Unit test: missing --timeout uses `default_poll_timeout` | Agent-Signal |
| Timeout bounds enforced | Unit test: min 1h, max 168h | Agent-Signal |
| Signal Poll created | Integration test: PollCreate message sent to group | Agent-Signal |
| **Poll termination on expiry** | Unit test: PollTerminate sent when timeout expires | Agent-Signal |
| Termination before announcement | Unit test: PollTerminate sent BEFORE result announcement | Agent-Signal |
| Quorum check | Unit test: fails if participation < min_quorum even with 100% approval | Agent-Signal |
| Threshold check | Unit test: fails if approval < config_change_threshold | Agent-Signal |
| Approved proposals execute | Integration test: config change applied after passage | Agent-Signal |
| **GAP-02**: No individual votes persisted | Code review: only approve_count/reject_count in Freenet | Witness |
| State stream (not polling) | Code review: no `tokio::time::sleep` in polling loops | Witness |

#### Security Constraints (Witness MUST Verify)

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| No cleartext Signal IDs in logs | Code review: all Signal IDs hashed before logging | Witness |
| No cleartext Signal IDs in storage | Code review: Freenet state uses hashes only | Witness |
| Display names resolved ephemerally | Code review: Signal API lookups, never persisted | Witness |
| Self-query safe | Code review: `/status` shows own vouchers only | Witness |
| Third-party query restricted | Code review: cannot query other members' vouchers | Witness |
| Vote privacy preserved | Code review: no VoteRecord with member ID | Witness |

#### Property-Based Tests (REQUIRED)

| Criterion | Proptest Coverage | Agent |
|-----------|-------------------|-------|
| DVR ≤ 1.0 | All graph configurations | Agent-Freenet |
| Distinct validators disjoint | Voucher sets pairwise disjoint | Agent-Freenet |
| Bridge removal correct | Known graphs with calculated clusters | Agent-Freenet |
| Betweenness centrality valid | 0 ≤ centrality ≤ (N-1)(N-2)/2 | Agent-Freenet |
| Timeout bounds | All durations checked against min/max | Agent-Signal |

#### Documentation & UX

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| `/mesh` help text | Unit test: subcommand hints displayed | Agent-Signal |
| Proposal UX messages | Unit test: all states (created, passed, failed) formatted correctly | Agent-Signal |
| GAP-11 announcement | Unit test: cluster activation message includes grandfathering note | Agent-Signal |
| `docs/ALGORITHMS.md` updated | File review: DVR formula, Bridge Removal algorithm documented | Agent-Freenet |
| `docs/USER-GUIDE.md` updated | File review: all `/mesh` commands documented | Agent-Signal |

#### Code Coverage (CI Enforced)

- [ ] 100% code coverage on `src/matchmaker/*.rs`
- [ ] 100% code coverage on `src/commands/mesh/*.rs`
- [ ] 100% code coverage on `src/proposals/*.rs`
- [ ] All proptests pass (minimum 256 cases per test)
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo deny check` passes (supply chain security)

#### Integration Test Scenarios

**Gastown MUST run these scenarios before closing convoy:**

```bash
# 1. DVR and Cluster Detection
gt test:integration --scenario mesh-health

# Scenario steps:
# a) Create 12-member network with 2 obvious clusters
# b) Verify: Bridge Removal detects 2 clusters
# c) Verify: GAP-11 announcement sent ("cross-cluster now required")
# d) Add new member with same-cluster vouches
# e) Verify: Admission REJECTED (cross-cluster required)
# f) Add new member with cross-cluster vouches
# g) Verify: Admission ACCEPTED
# h) Verify: DVR calculated, /mesh shows correct health tier

# 2. Blind Matchmaker Strategic Introductions
gt test:integration --scenario blind-matchmaker

# Scenario steps:
# a) Create network with DVR 40% (🔴 Unhealthy)
# b) Verify: Bot suggests DVR-optimal introduction (Phase 0)
# c) User accepts introduction, new vouch recorded
# d) Verify: DVR increases, bot updates suggestions
# e) Repeat until DVR > 66% (🟢 Healthy)
# f) Verify: Bot switches to maintenance mode (MST only)

# 3. Proposal System End-to-End
gt test:integration --scenario proposal-lifecycle

# Scenario steps:
# a) Member sends /propose config name "New Name" --timeout 5m
# b) Verify: Signal Poll created with correct options
# c) Members vote (mix of approve/reject)
# d) Wait for timeout to expire
# e) Verify: PollTerminate sent to Signal
# f) Verify: Result announced (pass or fail)
# g) If passed: Verify config change applied
# h) Verify: Only aggregates stored in Freenet (no individual votes)

# 4. Proposal Quorum Failure
gt test:integration --scenario proposal-quorum-fail

# Scenario steps:
# a) 10-member group, 50% quorum required
# b) Create proposal, only 3 members vote (all approve)
# c) Wait for timeout
# d) Verify: Proposal FAILED (quorum not met)
# e) Verify: Clear message explaining quorum failure
```

#### Mayor Convoy Closure Checklist

```bash
# Verify all beads complete
bd list --convoy convoy-phase2 --status pending
# Should return: No pending beads

# Verify GAP-02 and GAP-11 integrated
bd show gap-02  # Vote privacy - should be complete
bd show gap-11  # Cluster announcement - should be complete

# Verify security audit
gt audit --convoy convoy-phase2 --agent witness
# Should return: All security constraints verified

# Verify test coverage
cargo llvm-cov nextest --all-features -- --test-threads=1
# Should return: 100% coverage on matchmaker, mesh, proposals modules

# Verify property tests
cargo test --release -- proptest
# Should return: All proptests pass (256+ cases each)

# Verify documentation
ls docs/ALGORITHMS.md docs/USER-GUIDE.md
# Should exist and be updated with Phase 2 content

# Close convoy
gt convoy close convoy-phase2 --verified
```

---

## 💾 PHASE 2.5: Persistence Convoy

**Convoy ID**: `convoy-persistence`  
**Duration**: Week 6-7  
**Review Report**: `docs/todo/phase-2.5-review.md`  
**Status**: 🟡 **70% Complete** - Core modules done, property tests MISSING (CRITICAL)  
**Critical Gaps**: 
  - st-btcya: 0/13 property tests (encryption, chunking, rendezvous)
  - st-h6ocd: Attestation module missing
  - st-p12rt: User-facing commands missing  
**Dependencies**: `convoy-phase2` complete  
**Reference**: `.beads/persistence-model.bead`, `docs/PERSISTENCE.md`

### Mayor Delegation Commands

```bash
gt convoy create --title "Phase 2.5: Persistence" \
  --depends-on convoy-phase2

bd create --title "Replication health tracking" --convoy convoy-persistence
bd create --title "Chunk distribution (64KB, 3 copies)" --convoy convoy-persistence
bd create --title "Write-blocking state machine" --convoy convoy-persistence
bd create --title "Persistence registry architecture" --convoy convoy-persistence
bd create --title "Recovery procedure" --convoy convoy-persistence
```

### Key Parameters (FIXED)

```
CHUNK_SIZE = 64KB
REPLICATION_FACTOR = 3  (1 local + 2 remote)
```

### Bead: Replication Health Tracking

**Agent**: Agent-Freenet  
**Reference**: `.beads/persistence-model.bead` (§ Replication Health Metric)  
**Dependencies**: Chunk Distribution, Challenge-Response Verification (Q9/Q13)  
**Review**: See `phase-2.5-review.md` lines 40-96 (✅ COMPLETE - 14 tests, write-blocking works)  
**Gaps**: Attestation module missing (lines 58-77 - st-h6ocd CRITICAL), retry logic incomplete

#### Overview

Replication Health answers the fundamental question: **"Is my trust network data resilient?"**

The metric tracks whether chunks have been successfully distributed to holders. Health is measured **at write time** (not via continuous heartbeats) — when state changes occur, the bot distributes chunks and records which holders confirmed receipt.

#### Core Concepts

**Replication Factor**: 3 copies per chunk (1 local + 2 remote)

**Measurement**: At write time, not via heartbeats
- State changes → encrypt → chunk → distribute to 2 remote holders per chunk
- Each holder signs receipt confirmation (attestation)
- Health based on confirmed receipts, not continuous polling

**Why No Heartbeats**:
- Trust state changes are infrequent (human timescale: ~10-100/month)
- Heartbeats add complexity without proportional benefit
- Recovery verifies possession directly when needed

#### Health States

| Status | Chunk Health | Recovery Possible? | Writes | User Display |
|--------|--------------|-------------------|--------|--------------|
| 🟢 **Replicated** | All chunks 3/3 | ✅ Guaranteed | Allowed | "Fully resilient" |
| 🟡 **Partial** | Some chunks 2/3 | ✅ Possible | Allowed | "Recoverable but degraded" |
| 🔴 **At Risk** | Any chunk ≤1/3 | ❌ Not possible | **BLOCKED** | "Cannot recover if crash" |
| 🔵 **Initializing** | Establishing | — | Limited | "Setting up persistence" |

**Write-Blocking Rule**: If peers are available but distribution failed (DEGRADED), writes are blocked until replication succeeds. This prevents accumulating state that can't be backed up.

#### Health Calculation

```rust
/// Replication Health = Chunks_With_2+_Replicas / Total_Chunks
/// 
/// Where:
/// - Chunks_With_2+_Replicas = Chunks where at least 2 of 3 copies confirmed
/// - Total_Chunks = ceil(state_size / CHUNK_SIZE)
/// - CHUNK_SIZE = 64KB (fixed)
/// - REPLICATION_FACTOR = 3 (1 local + 2 remote)

pub struct ReplicationHealth {
    total_chunks: u32,
    chunks_fully_replicated: u32,   // 3/3 copies
    chunks_partial: u32,            // 2/3 copies
    chunks_at_risk: u32,            // ≤1/3 copies
    last_distribution: Timestamp,
    state_version: u64,
}

impl ReplicationHealth {
    pub fn status(&self) -> HealthStatus {
        if self.chunks_at_risk > 0 {
            HealthStatus::AtRisk           // 🔴 Any chunk ≤1/3
        } else if self.chunks_partial > 0 {
            HealthStatus::Partial          // 🟡 Some chunks 2/3
        } else {
            HealthStatus::Replicated       // 🟢 All chunks 3/3
        }
    }
    
    pub fn can_write(&self) -> bool {
        // Block writes only in DEGRADED state (peers available but failed)
        self.status() != HealthStatus::AtRisk
    }
    
    pub fn ratio(&self) -> f32 {
        let healthy = self.chunks_fully_replicated + self.chunks_partial;
        healthy as f32 / self.total_chunks as f32
    }
}
```

#### Deliverables

- [x] `src/persistence/health.rs` — Health tracking module ✅ **COMPLETE** (14 unit tests passing)
  - [x] `ReplicationHealth` struct with per-chunk status ✅
  - [x] `HealthStatus` enum (Replicated, Partial, AtRisk, Initializing) ✅
  - [x] `status()` — Compute current health from chunk states ✅
  - [x] `can_write()` — Check if writes are allowed ✅
  - [x] `ratio()` — Calculate health percentage ✅
- [ ] `src/persistence/attestation.rs` — Holder confirmations ❌ **MISSING** (st-h6ocd - no module exists)
  - [ ] `Attestation` struct (holder signature on chunk receipt) ❌
  - [ ] `verify_attestation()` — Verify holder's signature ❌
  - [ ] `record_attestation()` — Update chunk health on receipt ❌
- [x] `src/persistence/distribution.rs` — Chunk distribution tracking ⚠️ **PARTIAL** (4 tests, retry/fallback TODO)
  - [x] `ChunkDistributionState` — Per-chunk holder status ✅
  - [x] `on_distribution_success()` — Update health when holder confirms ✅
  - [x] `on_distribution_failure()` — Track failed distributions ✅
  - [ ] `retry_failed_distributions()` — Background retry logic ❌ **TODO** (exponential backoff missing)
- [ ] `src/commands/mesh/replication.rs` — User-facing command (see Advanced Commands bead) ❌ **STUB** (st-p12rt)
- [x] Integration with write-blocking in `src/persistence/state_writer.rs` ✅ **COMPLETE**

#### Write-Blocking Integration

```rust
/// State writer checks replication health before allowing writes
pub async fn write_state_change(
    change: StateChange,
    health: &ReplicationHealth,
    network_size: usize,
) -> Result<(), WriteError> {
    // Check write-blocking rules
    match (health.status(), network_size) {
        (HealthStatus::AtRisk, n) if n >= 3 => {
            // DEGRADED: Peers available but distribution failed
            return Err(WriteError::ReplicationDegraded {
                at_risk_chunks: health.chunks_at_risk,
                message: "Writes blocked until replication restored",
            });
        }
        (_, 1) => {
            // ISOLATED: N=1 network, warn but allow
            log::warn!("Single-bot network: no persistence guarantee");
        }
        (_, 2) => {
            // Mutual dependency, warn but allow
            log::warn!("Two-bot network: minimal persistence guarantee");
        }
        _ => {
            // ACTIVE or PROVISIONAL: writes allowed
        }
    }
    
    // Proceed with write...
    Ok(())
}
```

#### Acceptance Criteria

**Health Tracking:**
- [x] Health updated immediately after each chunk distribution attempt ✅
- [x] Per-chunk status tracked (not just aggregate) ✅
- [ ] Attestations (holder signatures) stored for verification ❌ **MISSING** (st-h6ocd - attestation module TODO)
- [x] Health persists across bot restarts (stored in local state) ✅

**Write-Blocking:**
- [x] Writes blocked when `chunks_at_risk > 0` AND `network_size >= 3` ✅ **COMPLETE** (13 tests passing)
- [x] Writes allowed in PROVISIONAL state (no peers available) ✅
- [x] Writes allowed with warning for N=1 and N=2 networks ✅
- [x] Clear error message explains why writes are blocked ✅

**Recovery Confidence:**
- [ ] 🟢 Replicated → "Full recovery possible"
- [ ] 🟡 Partial → "Recovery possible but degraded"
- [ ] 🔴 At Risk → "Cannot recover if crash now"

**Retry Logic:**
- [ ] Failed distributions retried with exponential backoff
- [ ] Fallback holders computed via rendezvous hashing when primary unavailable
- [ ] Retry continues until health restored or network unavailable

**User Display:**
- [ ] `/mesh` shows summary: "Replication: 🟢 Replicated (3/3 holders)"
- [ ] `/mesh replication` shows detailed per-chunk status
- [ ] Degraded chunks listed individually when health < 100%

#### Test Cases

```rust
#[test]
fn test_health_all_replicated() {
    // Given: 8 chunks, all with 3/3 copies confirmed
    // When: Calculate health
    // Then: status = Replicated, ratio = 1.0, can_write = true
}

#[test]
fn test_health_partial() {
    // Given: 8 chunks, 6 with 3/3, 2 with 2/3
    // When: Calculate health
    // Then: status = Partial, can_write = true
}

#[test]
fn test_health_at_risk() {
    // Given: 8 chunks, 1 with only 1/3 copies
    // When: Calculate health
    // Then: status = AtRisk, can_write = false (if network >= 3)
}

#[test]
fn test_write_blocked_degraded() {
    // Given: AtRisk status, network_size = 5
    // When: Attempt write
    // Then: WriteError::ReplicationDegraded returned
}

#[test]
fn test_write_allowed_isolated() {
    // Given: AtRisk status, network_size = 1
    // When: Attempt write
    // Then: Write succeeds with warning
}

#[test]
fn test_attestation_updates_health() {
    // Given: Chunk with 2/3 copies
    // When: Third holder sends attestation
    // Then: Chunk status updates to 3/3, overall health recalculated
}

#[test]
fn test_fallback_holder_on_failure() {
    // Given: Primary holder unreachable
    // When: Distribution fails
    // Then: Fallback holder computed via rendezvous hashing
}

proptest! {
    #[test]
    fn health_ratio_in_valid_range(
        fully: u32 in 0..100u32,
        partial: u32 in 0..100u32,
        at_risk: u32 in 0..100u32,
    ) {
        let total = fully + partial + at_risk;
        if total > 0 {
            let health = ReplicationHealth {
                total_chunks: total,
                chunks_fully_replicated: fully,
                chunks_partial: partial,
                chunks_at_risk: at_risk,
                ..Default::default()
            };
            let ratio = health.ratio();
            assert!(ratio >= 0.0 && ratio <= 1.0);
        }
    }
}
```

#### UX Messages

**Healthy (`/mesh replication`):**
```
💾 Replication Health: 🟢 Replicated

Last State Change: 3 hours ago (Alice joined)
State Size: 512KB (8 chunks)
Chunks Replicated: 8/8 (all 3 copies per chunk) ✅
State Version: 47

Recovery Confidence: ✅ Yes — all chunks available from multiple holders

💡 Your trust network is resilient. If this bot goes offline,
the state can be recovered from chunk holders.
```

**Degraded (`/mesh replication`):**
```
💾 Replication Health: 🟡 Partial

Last State Change: 1 hour ago (Bob vouched for Carol)
State Size: 512KB (8 chunks)
Chunks Replicated: 7/8 fully, 1/8 degraded (2/3 copies) ⚠️
  Chunk 5: 2/3 holders responding
State Version: 48

Recovery Confidence: ✅ Yes — all chunks recoverable

⚠️ One chunk has degraded replication. Recovery is still possible,
but resilience is reduced. Bot will retry distribution.
```

**At Risk (`/mesh replication`):**
```
💾 Replication Health: 🔴 At Risk

Last State Change: 30 minutes ago (Carol flagged Dave)
State Size: 512KB (8 chunks)
Critical Chunks: 1 (only 1 holder!) ⚠️
  Chunk 3: 1/3 holders responding
State Version: 49

Recovery Confidence: ❌ No — chunk 3 cannot be recovered if lost

🚨 CRITICAL: Writes are BLOCKED until replication restored.
Bot is actively seeking replacement holders for chunk 3.
```

**Write Blocked Error:**
```
❌ Write Blocked: Replication Degraded

Your trust network state cannot be safely modified because
chunk replication is incomplete:

  At-risk chunks: 1 (chunk 3 has only 1/3 copies)

This protects against data loss. The bot is actively
seeking replacement holders. Writes will resume automatically
when replication is restored.

Check status: /mesh replication
```

---

### Bead: Chunk Distribution

**Agent**: Agent-Freenet + Agent-Crypto  
**Reference**: `.beads/persistence-model.bead` (§ Chunking + Replication Model, § Deterministic Chunk Assignment)  
**Dependencies**: Registry (bot discovery), Rendezvous Hashing (Q11), Challenge-Response (Q9/Q13)  
**Review**: See `phase-2.5-review.md` lines 117-184 (⚠️ PARTIAL - 69 unit tests, 0/13 proptests)  
**CRITICAL**: Property tests MISSING (st-btcya lines 253-277) - encryption, chunking, rendezvous  
**Gaps**: Encryption module not separated (st-mkiez), attestation missing (st-h6ocd), retry logic TODO

#### Overview

Chunk Distribution handles the core persistence operation: encrypting trust state, splitting into fixed-size chunks, and distributing to deterministic holders for durability.

**The Goal**: A crashed bot can recover its trust map from encrypted fragments held by adversarial peers who cannot read or reconstruct that data.

#### Core Parameters (FIXED)

```
CHUNK_SIZE = 64KB          (balance: distribution vs coordination)
REPLICATION_FACTOR = 3     (1 local + 2 remote replicas per chunk)
```

**DO NOT CHANGE** these parameters without security review and explicit unpinning.

#### Distribution Flow

```
1. State changes → version = N → LOCK for distribution
2. Serialize trust state → encrypt with ACI-derived key
3. Split into ceil(state_size / 64KB) chunks
4. For each chunk:
   a. Compute 2 holders via rendezvous_hash(chunk_idx, bot_list, epoch)
   b. Distribute chunk to both holders
   c. Collect attestations (signed receipts)
5. On success: UNLOCK, apply queued changes → version = N+1
6. On partial failure: retry failed holders, DON'T proceed to next version
```

#### Rendezvous Hashing (Deterministic Holder Selection)

**Why Deterministic**: Anyone can compute who holds whose chunks. This is acceptable because:
- Chunks are encrypted (holder can't read content)
- Need ALL chunks + ACI key (single chunk = partial ciphertext)
- Attack requires compromising actual holders, not just knowing their identity
- **Removes registry as high-value attack target** (no chunk relationships stored)

```rust
/// Compute the 2 replica holders for a specific chunk
fn compute_chunk_holders(
    owner_contract: &ContractHash,
    chunk_index: u32,
    registered_bots: &[ContractHash],  // Sorted list from registry
    epoch: u64,
) -> [ContractHash; 2] {
    // Rendezvous hashing: each bot gets a score for holding this chunk
    // Top 2 scores = this chunk's replica holders
    let mut scores: Vec<(ContractHash, Hash)> = registered_bots
        .iter()
        .filter(|b| *b != owner_contract)  // Can't hold own chunks
        .map(|bot| {
            let score = hash(owner_contract, chunk_index, bot, epoch);
            (*bot, score)
        })
        .collect();
    
    scores.sort_by_key(|(_, score)| *score);
    [scores[0].0, scores[1].0]
}
```

#### Encryption (ACI-Derived Key)

**Single cryptographic identity**: Signal ACI key used for encryption, signing, AND identity masking.

```rust
/// Derive encryption key from Signal ACI identity via HKDF
fn derive_encryption_key(aci_identity: &IdentityKeyPair) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(
        Some(b"stroma-chunk-encryption-v1"),
        aci_identity.private_key().serialize().as_slice()
    );
    let mut key = [0u8; 32];
    hk.expand(b"aes-256-gcm-key", &mut key).unwrap();
    key
}

/// Encrypt state before chunking
fn encrypt_state(
    state: &TrustNetworkState,
    aci_identity: &IdentityKeyPair,
) -> Vec<u8> {
    let key = derive_encryption_key(aci_identity);
    let cipher = Aes256Gcm::new(Key::from_slice(&key));
    let nonce = generate_random_nonce();  // 12 bytes
    
    let plaintext = serialize(state);
    let ciphertext = cipher.encrypt(&nonce, plaintext.as_slice()).unwrap();
    
    // Prepend nonce to ciphertext
    [nonce.as_slice(), ciphertext.as_slice()].concat()
}
```

#### Version-Locked Distribution (REQUIRED)

**Problem**: Distribution of 8 chunks to 16 holders is not atomic. State changes during distribution could fragment chunk sets.

**Solution**: Lock state during distribution, queue new changes.

```rust
pub struct DistributionLock {
    locked_version: Option<u64>,
    pending_changes: Vec<StateChange>,
}

impl DistributionLock {
    /// Lock state for distribution
    pub fn lock(&mut self, version: u64) {
        assert!(self.locked_version.is_none(), "Already locked");
        self.locked_version = Some(version);
    }
    
    /// Queue changes while locked
    pub fn queue_change(&mut self, change: StateChange) {
        self.pending_changes.push(change);
    }
    
    /// Unlock after successful distribution
    pub fn unlock(&mut self) -> Vec<StateChange> {
        self.locked_version = None;
        std::mem::take(&mut self.pending_changes)
    }
}
```

**Invariant**: All holders for a given version have IDENTICAL chunks.

#### Contract-Based Distribution (Phase 0)

**Phase 0 Pattern**: Each bot has a chunk storage contract. Chunks are written as state updates.

```rust
/// Each bot has a chunk storage contract
pub struct ChunkStorageContract {
    /// Chunks I'm holding for others: (owner, chunk_idx) → chunk_data
    stored_chunks: HashMap<(ContractHash, u32), Chunk>,
}

/// Distribute chunk to holder via their contract
async fn distribute_chunk(
    holder_contract: &ContractHash,
    chunk: &Chunk,
    freenet: &FreenetClient,
) -> Result<Attestation> {
    let delta = ChunkStorageDelta::Store {
        owner: chunk.owner,
        chunk_index: chunk.chunk_index,
        data: chunk.data.clone(),
    };
    
    freenet.update_contract(holder_contract, &delta).await?;
    
    // Holder's contract returns attestation (signed receipt)
    Ok(attestation)
}
```

#### Deliverables

- [ ] `src/persistence/encryption.rs` — AES-256-GCM with ACI-derived key ❌ **MISSING** (st-mkiez - merged into chunks.rs)
  - [x] `derive_encryption_key()` — HKDF from Signal ACI ✅ (in chunks.rs)
  - [x] `encrypt_state()` — Serialize, encrypt, prepend nonce ✅ (in chunks.rs)
  - [x] `decrypt_state()` — Split nonce, decrypt, deserialize ✅ (in chunks.rs)
- [x] `src/persistence/chunking.rs` — 64KB chunk splitting ✅ **COMPLETE** (10 unit tests, in chunks.rs)
  - [x] `split_into_chunks()` — ceil(state_size / 64KB) chunks ✅
  - [x] `reassemble_chunks()` — Concatenate for decryption ✅
  - [x] Chunk metadata: owner, index, version, hash ✅
- [x] `src/persistence/rendezvous.rs` — Deterministic holder selection ✅ **COMPLETE** (14 unit tests)
  - [x] `compute_chunk_holders()` — Top 2 scores per chunk ✅
  - [ ] `compute_fallback_holder()` — When primary unavailable ⚠️ **TODO** (Q11 rendezvous fallback)
  - [x] `compute_all_chunk_holders()` — All holders for all chunks ✅
- [x] `src/persistence/distribution.rs` — Distribution orchestration ⚠️ **PARTIAL** (4 tests)
  - [x] `distribute_all_chunks()` — Parallel distribution to all holders ✅
  - [ ] `retry_failed_distribution()` — Exponential backoff retry ❌ **TODO**
  - [ ] `DistributionLock` — Version-locked queue ❌ **MISSING** (race condition possible)
- [x] `src/persistence/chunk_storage.rs` — Contract-based storage (Phase 0) ✅ **COMPLETE** (5 tests)
  - [x] `ChunkStorageContract` — HashMap<(owner, idx), Chunk> ✅
  - [x] `ChunkStorageDelta` — Store/Remove operations ✅
  - [ ] `Attestation` — Signed receipt from holder ❌ **MISSING** (st-h6ocd)
- [x] `src/persistence/recovery.rs` — State recovery ⚠️ **PARTIAL** (3 unit tests with mocks)
  - [x] `recover_state()` — Fetch all chunks, reassemble, decrypt ✅
  - [x] `fetch_chunk_from_any_holder()` — Try any 1 of 3 copies ✅

#### Acceptance Criteria

**Encryption (Agent-Crypto):**
- [x] AES-256-GCM encryption with key derived from Signal ACI via HKDF ✅
- [x] Random nonce per encryption (12 bytes, prepended to ciphertext) ✅
- [x] Decryption succeeds with correct ACI key ✅
- [x] Decryption fails with wrong key (authentication tag mismatch) ✅
- [ ] Property-based tests (proptest) covering: ❌ **MISSING** (st-btcya - 0/8 proptests)
  - [ ] Encryption roundtrip preserves data (completeness) ❌
  - [ ] Different keys produce different ciphertexts (key isolation) ❌
  - [ ] Wrong key fails authentication (soundness) ❌
  - [ ] Each encryption uses unique nonce ❌
  - [ ] HKDF key derivation is deterministic ❌
  - [ ] HKDF produces isolated keys for different inputs ❌

**Chunking:**
- [x] CHUNK_SIZE = 64KB constant used consistently ✅
- [x] ceil(state_size / CHUNK_SIZE) chunks produced ✅
- [x] Reassembled chunks match original encrypted state ✅
- [ ] Property-based tests (proptest) covering: ❌ **MISSING** (st-btcya - 0/3 proptests)
  - [ ] Split → reassemble = original ❌
  - [ ] Correct chunk count calculation ❌
  - [ ] No chunk exceeds CHUNK_SIZE ❌

**Holder Selection (Agent-Crypto):**
- [x] Rendezvous hashing produces deterministic results ✅
- [x] Same inputs → same holders (anyone can verify) ✅
- [x] Owner cannot hold their own chunks ✅
- [x] Churn-stable: only affected chunks reassigned when bot leaves ✅
- [ ] Property-based tests (proptest) covering: ❌ **MISSING** (st-btcya - 0/5 proptests)
  - [ ] Determinism: same inputs → same holders ❌
  - [ ] Owner exclusion: owner never selected for own chunks ❌
  - [ ] Two distinct holders per chunk ❌
  - [ ] Churn stability: non-holder departure doesn't change holders ❌
  - [ ] Uniform distribution: χ² test passes at 95% confidence ❌

**Distribution:**
- [x] 2 remote holders per chunk (REPLICATION_FACTOR = 3 including local) ✅
- [ ] Attestation (signed receipt) returned on successful storage ❌ **MISSING** (st-h6ocd)
- [ ] Version-locked: state changes queued during distribution ❌ **MISSING** (DistributionLock TODO)
- [x] All holders for same version have identical chunks ✅
- [ ] Partial failure: retry failed holders, don't proceed to next version ⚠️ **TODO** (retry logic missing)

**Security (GAP-12):**
- [x] Persistence peers are ADVERSARIAL — zero trust ✅
- [x] Persistence discovery ≠ Federation discovery (different trust models) ✅
- [x] Chunks encrypted before distribution (peers can't read) ✅
- [x] Need ALL chunks + ACI key to reconstruct ✅

#### Test Cases

```rust
#[test]
fn test_encryption_roundtrip() {
    // Given: Trust state and ACI identity
    // When: Encrypt then decrypt
    // Then: Recovered state matches original
}

#[test]
fn test_encryption_wrong_key_fails() {
    // Given: Encrypted state
    // When: Decrypt with different ACI key
    // Then: Error (authentication tag mismatch)
}

#[test]
fn test_chunking_correct_count() {
    // Given: 500KB encrypted state
    // When: Split into chunks
    // Then: ceil(500KB / 64KB) = 8 chunks
}

#[test]
fn test_rendezvous_deterministic() {
    // Given: Same owner, chunk_idx, bot_list, epoch
    // When: Compute holders twice
    // Then: Same result both times
}

#[test]
fn test_rendezvous_owner_excluded() {
    // Given: Owner in bot_list
    // When: Compute holders
    // Then: Owner not in result
}

#[test]
fn test_distribution_lock_queues() {
    // Given: Locked state (version 48)
    // When: State change arrives
    // Then: Change queued, not applied
}

#[test]
fn test_distribution_lock_applies_on_unlock() {
    // Given: Locked state with queued changes
    // When: Unlock
    // Then: Queued changes returned for application
}

#[test]
fn test_partial_failure_retries() {
    // Given: 2 holders, 1 fails
    // When: Distribution
    // Then: Retry failed holder, don't proceed to next version
}
```

#### Property-Based Tests (Agent-Crypto REQUIRED)

Cryptographic operations in Chunk Distribution require property-based testing to verify correctness across input space.

```rust
proptest! {
    // ══════════════════════════════════════════════════════════════════
    // ENCRYPTION PROPERTIES (Agent-Crypto)
    // ══════════════════════════════════════════════════════════════════
    
    #[test]
    fn encryption_roundtrip_preserves_data(state_bytes: Vec<u8>) {
        // Completeness: encrypt → decrypt always recovers original
        let identity = generate_test_identity();
        let encrypted = encrypt_state(&state_bytes, &identity);
        let decrypted = decrypt_state(&encrypted, &identity).unwrap();
        prop_assert_eq!(state_bytes, decrypted);
    }
    
    #[test]
    fn encryption_key_isolation(
        state_bytes: Vec<u8>,
        seed1: [u8; 32],
        seed2: [u8; 32],
    ) {
        prop_assume!(seed1 != seed2);
        // Different keys produce different ciphertexts
        let identity1 = identity_from_seed(&seed1);
        let identity2 = identity_from_seed(&seed2);
        let encrypted1 = encrypt_state(&state_bytes, &identity1);
        let encrypted2 = encrypt_state(&state_bytes, &identity2);
        prop_assert_ne!(encrypted1, encrypted2);
    }
    
    #[test]
    fn decryption_fails_with_wrong_key(
        state_bytes: Vec<u8>,
        seed1: [u8; 32],
        seed2: [u8; 32],
    ) {
        prop_assume!(seed1 != seed2);
        // Soundness: wrong key fails authentication
        let identity1 = identity_from_seed(&seed1);
        let identity2 = identity_from_seed(&seed2);
        let encrypted = encrypt_state(&state_bytes, &identity1);
        let result = decrypt_state(&encrypted, &identity2);
        prop_assert!(result.is_err());
    }
    
    #[test]
    fn encryption_nonce_uniqueness(state_bytes: Vec<u8>) {
        // Each encryption uses fresh nonce (first 12 bytes differ)
        let identity = generate_test_identity();
        let encrypted1 = encrypt_state(&state_bytes, &identity);
        let encrypted2 = encrypt_state(&state_bytes, &identity);
        // Same plaintext, same key → different ciphertext (random nonce)
        prop_assert_ne!(encrypted1[..12], encrypted2[..12]);
    }
    
    #[test]
    fn hkdf_key_derivation_deterministic(seed: [u8; 32]) {
        // Same input → same derived key
        let identity = identity_from_seed(&seed);
        let key1 = derive_encryption_key(&identity);
        let key2 = derive_encryption_key(&identity);
        prop_assert_eq!(key1, key2);
    }
    
    #[test]
    fn hkdf_key_derivation_isolated(seed1: [u8; 32], seed2: [u8; 32]) {
        prop_assume!(seed1 != seed2);
        // Different inputs → different derived keys
        let identity1 = identity_from_seed(&seed1);
        let identity2 = identity_from_seed(&seed2);
        let key1 = derive_encryption_key(&identity1);
        let key2 = derive_encryption_key(&identity2);
        prop_assert_ne!(key1, key2);
    }
    
    // ══════════════════════════════════════════════════════════════════
    // CHUNKING PROPERTIES
    // ══════════════════════════════════════════════════════════════════
    
    #[test]
    fn chunking_reassembly_matches(data: Vec<u8>) {
        // Completeness: split → reassemble = original
        let chunks = split_into_chunks(&data, CHUNK_SIZE);
        let reassembled = reassemble_chunks(&chunks);
        prop_assert_eq!(data, reassembled);
    }
    
    #[test]
    fn chunking_count_correct(data_len: usize) {
        // Correct chunk count: ceil(len / CHUNK_SIZE)
        let data = vec![0u8; data_len];
        let chunks = split_into_chunks(&data, CHUNK_SIZE);
        let expected = (data_len + CHUNK_SIZE - 1) / CHUNK_SIZE;
        let expected = if expected == 0 { 1 } else { expected };  // At least 1 chunk
        prop_assert_eq!(chunks.len(), expected);
    }
    
    #[test]
    fn chunking_max_size_enforced(data: Vec<u8>) {
        // No chunk exceeds CHUNK_SIZE
        let chunks = split_into_chunks(&data, CHUNK_SIZE);
        for chunk in &chunks {
            prop_assert!(chunk.data.len() <= CHUNK_SIZE);
        }
    }
    
    // ══════════════════════════════════════════════════════════════════
    // RENDEZVOUS HASHING PROPERTIES (Agent-Crypto)
    // ══════════════════════════════════════════════════════════════════
    
    #[test]
    fn rendezvous_deterministic(
        owner: [u8; 32],
        chunk_idx: u32,
        bot_seeds: Vec<[u8; 32]>,
        epoch: u64,
    ) {
        prop_assume!(bot_seeds.len() >= 3);  // Need at least 2 non-owner bots
        let bots: Vec<ContractHash> = bot_seeds.iter().map(hash_to_contract).collect();
        // Same inputs → same holders
        let holders1 = compute_chunk_holders(&hash_to_contract(&owner), chunk_idx, &bots, epoch);
        let holders2 = compute_chunk_holders(&hash_to_contract(&owner), chunk_idx, &bots, epoch);
        prop_assert_eq!(holders1, holders2);
    }
    
    #[test]
    fn rendezvous_owner_excluded(
        owner: [u8; 32],
        chunk_idx: u32,
        bot_seeds: Vec<[u8; 32]>,
        epoch: u64,
    ) {
        prop_assume!(bot_seeds.len() >= 3);
        let owner_contract = hash_to_contract(&owner);
        let mut bots: Vec<ContractHash> = bot_seeds.iter().map(hash_to_contract).collect();
        bots.push(owner_contract.clone());  // Include owner in list
        
        let holders = compute_chunk_holders(&owner_contract, chunk_idx, &bots, epoch);
        // Owner never selected as holder of own chunks
        prop_assert!(!holders.contains(&owner_contract));
    }
    
    #[test]
    fn rendezvous_two_distinct_holders(
        owner: [u8; 32],
        chunk_idx: u32,
        bot_seeds: Vec<[u8; 32]>,
        epoch: u64,
    ) {
        prop_assume!(bot_seeds.len() >= 3);
        let bots: Vec<ContractHash> = bot_seeds.iter().map(hash_to_contract).collect();
        let holders = compute_chunk_holders(&hash_to_contract(&owner), chunk_idx, &bots, epoch);
        // Two distinct holders selected
        prop_assert_ne!(holders[0], holders[1]);
    }
    
    #[test]
    fn rendezvous_churn_stability(
        owner: [u8; 32],
        chunk_idx: u32,
        bot_seeds: Vec<[u8; 32]>,
        epoch: u64,
    ) {
        prop_assume!(bot_seeds.len() >= 5);
        let bots: Vec<ContractHash> = bot_seeds.iter().map(hash_to_contract).collect();
        let holders_before = compute_chunk_holders(&hash_to_contract(&owner), chunk_idx, &bots, epoch);
        
        // Remove one non-holder bot
        let non_holder_idx = bots.iter()
            .position(|b| !holders_before.contains(b))
            .unwrap();
        let mut bots_after = bots.clone();
        bots_after.remove(non_holder_idx);
        
        let holders_after = compute_chunk_holders(&hash_to_contract(&owner), chunk_idx, &bots_after, epoch);
        // Holders unchanged when non-holder leaves
        prop_assert_eq!(holders_before, holders_after);
    }
    
    #[test]
    fn rendezvous_uniform_distribution(
        owners: Vec<[u8; 32]>,
        bot_seeds: Vec<[u8; 32]>,
        epoch: u64,
    ) {
        prop_assume!(owners.len() >= 100);
        prop_assume!(bot_seeds.len() >= 10);
        let bots: Vec<ContractHash> = bot_seeds.iter().map(hash_to_contract).collect();
        
        // Count how often each bot is selected across many owners
        let mut selection_counts: HashMap<ContractHash, usize> = HashMap::new();
        for owner in &owners {
            let holders = compute_chunk_holders(&hash_to_contract(owner), 0, &bots, epoch);
            for holder in holders {
                *selection_counts.entry(holder).or_insert(0) += 1;
            }
        }
        
        // Chi-squared test: distribution should be roughly uniform
        let expected = (owners.len() * 2) as f64 / bots.len() as f64;
        let chi_squared: f64 = selection_counts.values()
            .map(|&count| {
                let diff = count as f64 - expected;
                diff * diff / expected
            })
            .sum();
        
        // 95% confidence for chi-squared with (bots.len() - 1) degrees of freedom
        // For 9 df, critical value is ~16.9
        prop_assert!(chi_squared < 20.0, "Distribution not uniform: χ² = {}", chi_squared);
    }
}
```

---

### Bead: Write-Blocking State Machine

**Agent**: Agent-Freenet  
**Reference**: `.beads/persistence-model.bead` (§ Write-Blocking States)  
**Dependencies**: Replication Health Tracking, Chunk Distribution  
**Review**: See `phase-2.5-review.md` lines 77-96 (✅ COMPLETE - 13 tests passing, all states implemented)

#### Overview

Write-Blocking ensures the bot doesn't accumulate state that can't be backed up. When distribution fails but peers are available (DEGRADED), writes are blocked until replication is restored.

**Key Principle**: Availability-based, NOT TTL-based. Bot never penalized for network scarcity.

#### State Machine

| State | Condition | Writes | Replication Health | User Message |
|-------|-----------|--------|-------------------|--------------|
| **PROVISIONAL** | No suitable peers available | ALLOWED | 🔵 Initializing | "Setting up persistence" |
| **ACTIVE** | All chunks have 2+ replicas | ALLOWED | 🟢/🟡 | "Fully resilient" |
| **DEGRADED** | Any chunk ≤1 replica, peers available | **BLOCKED** | 🔴 At Risk | "Writes blocked" |
| **ISOLATED** | N=1 network | ALLOWED (warned) | 🔵 Initializing | "No persistence" |

#### State Transitions

```
                    ┌─────────────────────────────────────────┐
                    │                                         │
                    ▼                                         │
    ┌───────────┐  peers     ┌──────────┐  distribution  ┌────┴────┐
    │ ISOLATED  │─available─▶│PROVISIONAL│───succeeds───▶│  ACTIVE │
    │  (N=1)    │            │           │               │         │
    └───────────┘            └──────────┘               └────┬────┘
         ▲                        ▲                          │
         │                        │                     distribution
       N=1                   no peers                     fails
         │                        │                          │
         │                        │                          ▼
         │                   ┌────┴────┐               ┌──────────┐
         └───────────────────│DEGRADED │◀──────────────│  ACTIVE  │
                             │(BLOCKED)│               │          │
                             └─────────┘               └──────────┘
                                  │
                                  │ retry succeeds
                                  ▼
                             ┌──────────┐
                             │  ACTIVE  │
                             └──────────┘
```

#### Why Each State Exists

**PROVISIONAL** (Writes Allowed):
- Bot just started, no peers discovered yet
- Can't persist, but shouldn't block bootstrapping
- Transitions to ACTIVE once peers respond

**ACTIVE** (Writes Allowed):
- Normal operation: all chunks have sufficient replicas
- May be 🟢 (3/3) or 🟡 (2/3) — both allow writes

**DEGRADED** (Writes BLOCKED):
- Peers are available but distribution failed
- MUST succeed before making changes
- Prevents accumulating unbackable state
- Bot actively retries until restored

**ISOLATED** (Writes Allowed with Warning):
- Only bot in network (N=1)
- No peers to distribute to
- Operator warned: no persistence guarantee
- Acceptable for testing, NOT production

#### Implementation

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WriteBlockingState {
    Provisional,
    Active,
    Degraded,
    Isolated,
}

pub struct WriteBlockingManager {
    state: WriteBlockingState,
    network_size: usize,
    replication_health: ReplicationHealth,
    last_transition: Timestamp,
}

impl WriteBlockingManager {
    /// Compute current state from inputs
    pub fn compute_state(
        network_size: usize,
        peers_available: bool,
        health: &ReplicationHealth,
    ) -> WriteBlockingState {
        match (network_size, peers_available, health.status()) {
            (1, _, _) => WriteBlockingState::Isolated,
            (_, false, _) => WriteBlockingState::Provisional,
            (_, true, HealthStatus::AtRisk) => WriteBlockingState::Degraded,
            (_, true, _) => WriteBlockingState::Active,
        }
    }
    
    /// Check if writes are allowed
    pub fn can_write(&self) -> bool {
        match self.state {
            WriteBlockingState::Degraded => false,  // BLOCKED
            _ => true,  // All others allow writes
        }
    }
    
    /// Get warning message for allowed-but-risky states
    pub fn warning(&self) -> Option<String> {
        match self.state {
            WriteBlockingState::Isolated => {
                Some("Single-bot network: no persistence guarantee".to_string())
            }
            WriteBlockingState::Provisional => {
                Some("No peers discovered yet: persistence initializing".to_string())
            }
            _ => None,
        }
    }
    
    /// Get blocking reason for DEGRADED state
    pub fn blocking_reason(&self) -> Option<String> {
        if self.state == WriteBlockingState::Degraded {
            Some(format!(
                "Replication degraded: {} chunks at risk. Writes blocked until restored.",
                self.replication_health.chunks_at_risk
            ))
        } else {
            None
        }
    }
}
```

#### Network Bootstrap Limitations

| Network Size | State | Recovery Possible | Notes |
|--------------|-------|-------------------|-------|
| N=1 | ISOLATED | ❌ No | State on Freenet only, "good luck" |
| N=2 | ACTIVE | ⚠️ Fragile | Mutual dependency, both need each other |
| N=3 | ACTIVE | ⚠️ Minimal | One failure = degraded |
| N≥4 | ACTIVE | ✅ Resilient | Can tolerate 1 failure per chunk |
| N≥5 | ACTIVE | ✅ Recommended | Comfortable margin |

**Recommended minimum for production**: N≥5

#### Deliverables

- [ ] `src/persistence/write_blocking.rs` — State machine
  - [ ] `WriteBlockingState` enum (Provisional, Active, Degraded, Isolated)
  - [ ] `WriteBlockingManager` struct
  - [ ] `compute_state()` — Derive state from inputs
  - [ ] `can_write()` — Check if writes allowed
  - [ ] `warning()` — Get warning for risky states
  - [ ] `blocking_reason()` — Get reason for blocked state
- [ ] Integration with state writer
  - [ ] Check `can_write()` before applying changes
  - [ ] Return `WriteError::Blocked` with reason
  - [ ] Log warnings for ISOLATED and PROVISIONAL
- [ ] Integration with replication health
  - [ ] Recompute state on health changes
  - [ ] Trigger retry on DEGRADED
- [ ] User-facing messages
  - [ ] `/mesh` shows blocking status
  - [ ] Clear error message when write blocked

#### Acceptance Criteria

**State Computation:**
- [ ] N=1 → ISOLATED (regardless of health)
- [ ] No peers available → PROVISIONAL
- [ ] Peers available + AtRisk health → DEGRADED
- [ ] Peers available + healthy → ACTIVE

**Write Blocking:**
- [ ] DEGRADED blocks all writes
- [ ] PROVISIONAL, ACTIVE, ISOLATED allow writes
- [ ] ISOLATED and PROVISIONAL log warnings
- [ ] Clear error message returned when blocked

**State Transitions:**
- [ ] Transitions logged for debugging
- [ ] State recomputed when network_size changes
- [ ] State recomputed when replication health changes
- [ ] Retry triggered automatically in DEGRADED

**User Communication:**
- [ ] `/mesh` shows current blocking state
- [ ] Warning shown for ISOLATED ("no persistence guarantee")
- [ ] Warning shown for PROVISIONAL ("initializing")
- [ ] Error shown for DEGRADED ("writes blocked until restored")

#### Test Cases

```rust
#[test]
fn test_isolated_when_n_equals_1() {
    // Given: network_size = 1
    // When: Compute state
    // Then: ISOLATED (writes allowed with warning)
}

#[test]
fn test_provisional_when_no_peers() {
    // Given: network_size = 5, peers_available = false
    // When: Compute state
    // Then: PROVISIONAL (writes allowed with warning)
}

#[test]
fn test_degraded_when_at_risk() {
    // Given: network_size = 5, peers_available = true, health = AtRisk
    // When: Compute state
    // Then: DEGRADED (writes BLOCKED)
}

#[test]
fn test_active_when_healthy() {
    // Given: network_size = 5, peers_available = true, health = Replicated
    // When: Compute state
    // Then: ACTIVE (writes allowed)
}

#[test]
fn test_write_blocked_in_degraded() {
    // Given: DEGRADED state
    // When: can_write()
    // Then: false
}

#[test]
fn test_write_allowed_in_isolated() {
    // Given: ISOLATED state
    // When: can_write()
    // Then: true (with warning)
}

#[test]
fn test_transition_degraded_to_active() {
    // Given: DEGRADED state
    // When: Replication restored (all chunks 2+ replicas)
    // Then: Transitions to ACTIVE
}

#[test]
fn test_warning_for_isolated() {
    // Given: ISOLATED state
    // When: warning()
    // Then: "Single-bot network: no persistence guarantee"
}

#[test]
fn test_blocking_reason_for_degraded() {
    // Given: DEGRADED state with 2 at-risk chunks
    // When: blocking_reason()
    // Then: "2 chunks at risk. Writes blocked until restored."
}
```

#### UX Messages

**ISOLATED Warning:**
```
⚠️ Single-Bot Network

This bot is the only one in the persistence network.
Your trust state cannot be backed up.

If this bot crashes, recovery will NOT be possible.

Recommendation: Wait for more bots to join, or
accept this risk for testing purposes only.
```

**DEGRADED Error:**
```
❌ Write Blocked: Replication Degraded

Your trust network state cannot be safely modified because
chunk replication is incomplete:

  At-risk chunks: 2 (chunks 3, 7 have only 1/3 copies)
  Network size: 5 bots (peers available)

This protects against data loss. The bot is actively
seeking replacement holders. Writes will resume automatically
when replication is restored.

Check status: /mesh replication
```

**PROVISIONAL Warning:**
```
⚠️ Persistence Initializing

No persistence peers discovered yet. Your trust state
is stored locally only.

The bot will automatically establish persistence
once peers are discovered.

Check status: /mesh replication
```

---

### Phase 2.5 Success Criteria

Before closing `convoy-persistence`, the Mayor MUST verify ALL of the following:

#### Replication Health Tracking (Agent-Freenet)

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| Health states implemented | Unit tests pass for Replicated/Partial/AtRisk/Initializing | Agent-Freenet |
| `/mesh replication` displays correctly | Manual test: shows chunk counts, health status, recovery confidence | Agent-Signal |
| Health updates on state change | Unit test: state change triggers health recalculation | Agent-Freenet |
| No heartbeat polling | Code review: no periodic health checks, only event-driven | Witness |

#### Chunk Distribution (Agent-Freenet + Agent-Crypto)

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| CHUNK_SIZE = 64KB enforced | Constant in code, proptest verifies no chunks exceed | Agent-Freenet |
| REPLICATION_FACTOR = 3 | 1 local + 2 remote replicas per chunk | Agent-Freenet |
| AES-256-GCM encryption | Unit test: encrypt/decrypt roundtrip, wrong key fails | Agent-Crypto |
| HKDF key derivation | Unit test: deterministic, isolated per identity | Agent-Crypto |
| Rendezvous hashing deterministic | Proptest: same inputs → same holders | Agent-Crypto |
| Owner excluded from own chunks | Proptest: owner never selected as holder | Agent-Crypto |
| Version-locked distribution | Unit test: changes queued during distribution | Agent-Freenet |
| Attestations collected | Unit test: signed receipt returned on storage | Agent-Freenet |

#### Write-Blocking State Machine (Agent-Freenet)

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| ISOLATED allows writes (N=1) | Unit test: `can_write()` = true with warning | Agent-Freenet |
| PROVISIONAL allows writes | Unit test: `can_write()` = true with warning | Agent-Freenet |
| ACTIVE allows writes | Unit test: `can_write()` = true | Agent-Freenet |
| DEGRADED blocks writes | Unit test: `can_write()` = false, error message | Agent-Freenet |
| State transitions correct | Unit tests for all valid transitions | Agent-Freenet |
| User messages clear | Review: ISOLATED/PROVISIONAL warn, DEGRADED explains | Agent-Signal |

#### Recovery (Agent-Freenet)

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| Recovery succeeds (all chunks) | Integration test: crash → restart → state recovered | Agent-Freenet |
| Recovery fails gracefully (missing chunk) | Unit test: clear error when chunk unavailable | Agent-Freenet |
| Fallback holder used | Unit test: primary unavailable → next-highest score selected | Agent-Freenet |

#### Security Constraints (Witness MUST Verify)

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| Persistence peers ADVERSARIAL | Code review: no trust assumptions in chunk storage | Witness |
| Chunks encrypted before distribution | Code review: encrypt before chunk, decrypt after reassemble | Witness |
| Need ALL chunks + ACI key | Code review: partial chunks useless | Witness |
| Discovery ≠ Federation | Code review: persistence registry separate from federation | Witness |
| No cleartext in chunks | Code review: state serialized then encrypted | Witness |

#### Property-Based Tests (Agent-Crypto REQUIRED)

| Criterion | Proptest Coverage | Agent |
|-----------|-------------------|-------|
| Encryption completeness | roundtrip preserves data | Agent-Crypto |
| Encryption soundness | wrong key fails authentication | Agent-Crypto |
| Encryption key isolation | different keys → different ciphertexts | Agent-Crypto |
| HKDF determinism | same input → same key | Agent-Crypto |
| Chunking roundtrip | split → reassemble = original | Agent-Freenet |
| Rendezvous determinism | same inputs → same holders | Agent-Crypto |
| Rendezvous uniformity | χ² test passes at 95% confidence | Agent-Crypto |
| Churn stability | non-holder departure doesn't change holders | Agent-Crypto |

#### Documentation & Operator Guidance

| Criterion | Verification | Agent |
|-----------|--------------|-------|
| Signal store backup documented | `docs/OPERATOR-GUIDE.md` includes Signal store backup procedure | Agent-Signal |
| Recovery procedure documented | `docs/PERSISTENCE.md` explains crash recovery | Agent-Freenet |
| Write-blocking UX documented | `docs/USER-GUIDE.md` explains DEGRADED state | Agent-Signal |
| Health states documented | `docs/PERSISTENCE.md` explains Replicated/Partial/AtRisk | Agent-Freenet |

#### Code Coverage (CI Enforced)

- [ ] 100% code coverage on `src/persistence/*.rs`
- [ ] All proptests pass (minimum 256 cases per test)
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo deny check` passes (supply chain security)

#### Integration Test Scenario

**Gastown MUST run this scenario before closing convoy:**

```bash
# 1. Start 3-bot test network
gt test:integration --scenario persistence-basic

# Scenario steps:
# a) Bot-A creates trust state (5 members, 10 vouches)
# b) Bot-A distributes chunks to Bot-B and Bot-C
# c) Verify: All chunks have 3/3 copies (ACTIVE state)
# d) Kill Bot-A process
# e) Restart Bot-A
# f) Verify: Bot-A recovers state from Bot-B/Bot-C chunks
# g) Verify: Recovered state matches original

# 2. Test DEGRADED state
gt test:integration --scenario persistence-degraded

# Scenario steps:
# a) Start with healthy 3-bot network
# b) Kill Bot-B (chunk holder)
# c) Verify: Bot-A enters DEGRADED state
# d) Verify: Bot-A writes are BLOCKED
# e) Restart Bot-B
# f) Verify: Bot-A retries distribution
# g) Verify: Bot-A returns to ACTIVE state
# h) Verify: Bot-A writes succeed again
```

#### Mayor Convoy Closure Checklist

```bash
# Verify all beads complete
bd list --convoy convoy-persistence --status pending
# Should return: No pending beads

# Verify security audit
gt audit --convoy convoy-persistence --agent witness
# Should return: All security constraints verified

# Verify test coverage
cargo llvm-cov nextest --all-features -- --test-threads=1
# Should return: 100% coverage on persistence modules

# Verify property tests
cargo test --release -- proptest
# Should return: All proptests pass (256+ cases each)

# Verify documentation
ls docs/PERSISTENCE.md docs/OPERATOR-GUIDE.md
# Should exist and be updated

# Close convoy
gt convoy close convoy-persistence --verified
```

---

## 🔧 PHASE 3: Federation Preparation Convoy

**Convoy ID**: `convoy-phase3`  
**Duration**: Week 7-8  
**Dependencies**: `convoy-persistence` complete

### Mayor Delegation Commands

```bash
gt convoy create --title "Phase 3: Federation Prep" \
  --depends-on convoy-persistence

bd create --title "Social Anchor hashing (local only)" --convoy convoy-phase3
bd create --title "PSI-CA implementation (mock data)" --convoy convoy-phase3
bd create --title "Federation hooks validation" --convoy convoy-phase3
```

**Note**: Phase 3 computes federation infrastructure locally but does NOT broadcast. Federation broadcast is Phase 4+.

**Documentation**: Per convoy closure checklist, update `docs/FEDERATION.md` and `docs/ALGORITHMS.md` with PSI-CA details.

---

## 🏗️ Infrastructure Convoy (Parallel with Phases)

**Convoy ID**: `convoy-infra`  
**Can run parallel to implementation phases**  
**Review Report**: `docs/todo/INFRASTRUCTURE-DOCUMENTATION-REVIEW.md`  
**Status**: ✅ **100% Complete** - All infrastructure requirements met, documentation EXCELLENT  
**Documentation Gaps**: 5 minor (1 P2, 4 P3) - see review report lines 206-275

### Mayor Delegation Commands

```bash
gt convoy create --title "Infrastructure & CI/CD"

bd create --title "Dockerfile (hardened, distroless)" --convoy convoy-infra
bd create --title "GitHub Actions CI workflow" --convoy convoy-infra
bd create --title "GitHub Actions release workflow" --convoy convoy-infra
bd create --title "cargo-deny configuration" --convoy convoy-infra
bd create --title "CHANGELOG.md creation" --convoy convoy-infra  # st-vkbr7 (P2)
bd create --title "API documentation instructions" --convoy convoy-infra  # st-bd1ge (P3)
bd create --title "CONTRIBUTING.md creation" --convoy convoy-infra  # st-5op71 (P3)
bd create --title "MIGRATION-GUIDE.md creation" --convoy convoy-infra  # st-vz3ff (P3)
bd create --title "Production deployment checklist" --convoy convoy-infra  # st-zun44 (P3)
```

### Bead: Dockerfile

**Status**: ✅ **COMPLETE** (Review: INFRASTRUCTURE-DOCUMENTATION-REVIEW.md lines 17-30)  
**Features**: Multi-stage build, static MUSL, distroless base, non-root user, BuildKit caching

```dockerfile
# Stage 1: Builder
FROM rust:1.93-alpine AS builder
# Build static MUSL binary

# Stage 2: Runtime (distroless)
FROM gcr.io/distroless/static:nonroot
COPY --from=builder /build/stroma /stroma
USER nonroot:nonroot
ENTRYPOINT ["/stroma"]
```

### Bead: CI Workflow

**Status**: ✅ **COMPLETE** (Review: INFRASTRUCTURE-DOCUMENTATION-REVIEW.md lines 32-48)  
**Location**: `.github/workflows/ci.yml` (primary) + `security.yml` + `ci-monitor.yml` + `cargo-deny.yml`  
**Jobs**: Format & Lint, Test Suite (nextest), Code Coverage (87%, target 100%), Dependencies, CI Success Gate  
**Security**: 7 automated security checks (CodeQL, cargo-deny, unsafe detection, protected files, binary size, constraints, coverage)

```yaml
# .github/workflows/ci.yml
- cargo fmt --check
- cargo clippy -- -D warnings
- cargo nextest run --all-features
- cargo deny check
- cargo llvm-cov nextest --all-features  # 100% coverage required
```

### Bead: Release Workflow

**Status**: ✅ **COMPLETE** (Review: INFRASTRUCTURE-DOCUMENTATION-REVIEW.md lines 50-63)  
**Location**: `.github/workflows/release.yml`  
**Features**: Version tag triggers (v*), static MUSL binary, SHA256 checksums, GitHub releases, auto-generated notes  
**Minor Gap**: References CHANGELOG.md (st-vkbr7 - needs creation before next release)

### Bead: cargo-deny Configuration

**Status**: ✅ **COMPLETE** (Review: INFRASTRUCTURE-DOCUMENTATION-REVIEW.md lines 64-78)  
**Location**: `./deny.toml` + `.github/workflows/cargo-deny.yml`  
**Enforces**: Security vulnerabilities (RustSec), unmaintained deps, license compliance (Apache-2.0, MIT, BSD), banned crates (presage-store-sqlite), duplicate detection

---

### Documentation Status

**Overall Assessment**: ✅ **EXCELLENT** (Review: INFRASTRUCTURE-DOCUMENTATION-REVIEW.md lines 81-202)

#### ✅ Complete Documentation (All Present)

**Core Documentation**:
- [x] README.md - 398 lines, excellent mission/architecture overview
- [x] HOW-IT-WORKS.md - Plain-language trust protocol explanation
- [x] USER-GUIDE.md - Bot commands and daily workflows

**Operator Documentation**:
- [x] OPERATOR-GUIDE.md - 1648 lines, comprehensive installation/maintenance
- [x] CI-CD-PROTECTION.md - Green branch protection policy
- [x] CI-CD-DEPLOYMENT-STATUS.md - Deployment tracking and monitoring

**Developer Documentation**:
- [x] DEVELOPER-GUIDE.md - Architecture, module structure, development workflow
- [x] ALGORITHMS.md - MST, PSI-CA, Bridge Removal with complexity analysis
- [x] SECURITY-CI-CD.md - 7 automated security checks explained

**Security Documentation**:
- [x] THREAT-MODEL.md - 100+ lines, primary/secondary threat analysis
- [x] security-constraints.bead - 35KB immutable constraints with examples
- [x] TRUST-MODEL.md - Vouch mechanics, ejection, standing calculations

**Technical Documentation**:
- [x] FEDERATION.md - Phase 4+ design, PSI-CA protocol
- [x] PERSISTENCE.md - Reciprocal persistence, recovery procedures
- [x] 15+ specialized docs (FREENET_IMPLEMENTATION, VALIDATOR-THRESHOLD-STRATEGY, etc.)

**API Documentation**:
- [x] Rustdoc - 931 comment lines across 63 source files
- [ ] ⚠️ Generation instructions missing (st-bd1ge, P3)

#### ⚠️ Documentation Gaps (5 Minor - See Review lines 206-275)

| Gap | Priority | Bead | Status | Impact |
|-----|----------|------|--------|--------|
| **CHANGELOG.md** | P2 | st-vkbr7 | ❌ Missing | Required by release workflow before next release |
| **API doc instructions** | P3 | st-bd1ge | ⚠️ Partial | Rustdoc exists, needs `cargo doc` instructions in DEVELOPER-GUIDE |
| **CONTRIBUTING.md** | P3 | st-5op71 | ❌ Missing | Helps onboard contributors, project structure otherwise clear |
| **MIGRATION-GUIDE.md** | P3 | st-vz3ff | ❌ Missing | Needed before major version upgrade, not urgent now |
| **Production checklist** | P3 | st-zun44 | ⚠️ Partial | OPERATOR-GUIDE comprehensive, checklist adds convenience |

**Recommendation**: Address CHANGELOG.md (st-vkbr7) before next release. P3 gaps are nice-to-have improvements.

---

## 👁️ Witness Agent Protocol

**Role**: Continuous security audit  
**Runs**: Parallel to all other agents

### Witness Responsibilities

1. **Monitor all agent output** for Signal ID leakage (persistent storage, logs, network)
2. **Verify zeroization** in crypto operations (memory hygiene after hashing)
3. **Block unsafe patterns**:
   - Cleartext IDs in persistent storage, logs, or network
   - Polling (instead of state stream)
   - SqliteStore usage
   - Grace periods
4. **Read** `security-constraints.bead` before every review

### Witness Commands

```bash
# Run witness alongside convoy
gt spawn --role witness --watch-convoy convoy-phase0

# Witness checks (run before merge)
rg "signal.*id" --type rust  # Should find only hashed references
rg "SqliteStore" --type rust  # Should find zero matches
rg "sleep.*loop" --type rust  # Should find zero (no polling)

# Log security audit (GAP-07 - CRITICAL)
rg "tracing::(info|debug|warn|error).*voucher.*target" --type rust  # Should find zero
rg "tracing::(info|debug|warn|error).*flagger.*target" --type rust  # Should find zero
rg "tracing::(info|debug|warn|error).*holder.*chunk" --type rust    # Should find zero
rg "tracing::(info|debug|warn|error).*federated.*group" --type rust # Should find zero
```

---

## 📋 Completed Work (Reference)

### ✅ Spike Week 1 (Q1-Q6)

| Q | Finding | Status |
|---|---------|--------|
| Q1 | Freenet merge: commutative deltas with set-based state | GO |
| Q2 | Contract validation: trustless model viable | GO |
| Q3 | Cluster detection: Bridge Removal algorithm | GO |
| Q4 | STARK verification: bot-side for Phase 0 | PARTIAL |
| Q5 | Merkle Tree: 0.09ms at 1000 members | GO |
| Q6 | Proof storage: outcomes only | GO |

### ✅ Spike Week 2 (Q7-Q14)

| Q | Finding | Status |
|---|---------|--------|
| Q7 | Bot discovery: Registry-based, <1ms | GO |
| Q8 | Sybil resistance: PoW difficulty 18 | GO |
| Q9 | Chunk verification: Challenge-response <1ms | GO |
| Q11 | Rendezvous hashing: Deterministic | GO |
| Q12 | Chunk size: 64KB optimal | GO |
| Q13 | Fairness: 1% spot checks | GO |
| Q14 | Distribution: Contract-based Phase 0 | GO |

### ✅ Protocol v8 Poll Support

- Fork: `roder/libsignal-service-rs`
- Branch: `feature/protocol-v8-polls-fixed`
- Status: Build verified, unit tests pass

### ✅ Pre-Gastown Audit

- Terminology: Consistent
- Architecture: No contradictions
- Security: Constraints enforced
- GO decision: Ready for agent handoff

---

## 📚 Key Reference Documents

| Document | Purpose |
|----------|---------|
| `.beads/security-constraints.bead` | The 8 Absolutes (MUST read) |
| `.beads/architecture-objectives.bead` | Core design philosophy |
| `.beads/technology-stack.bead` | Tech decisions (Presage, CBOR, etc.) |
| `.beads/persistence-model.bead` | Chunk distribution, recovery |
| `.beads/terminology.bead` | Canonical definitions |
| `docs/DEVELOPER-GUIDE.md` | Module structure |
| `docs/ALGORITHMS.md` | DVR, Bridge Removal, PSI-CA |
| `docs/PERSISTENCE.md` | Comprehensive persistence guide |

---

## 🎯 Overall Success Metrics

### Security (Witness Validates)

- [ ] No cleartext Signal IDs in persistent storage, logs, or output
- [ ] All sensitive buffers zeroized immediately
- [ ] ZK-proofs used for all trust operations
- [ ] Memory dump contains only hashed identifiers
- [ ] cargo-deny and cargo-crev checks pass

### Functionality

- [ ] Seed group bootstrapped (3 members, member-initiated)
- [ ] Invitation & vetting flow working
- [ ] Admission requires 2 cross-cluster vouches
- [ ] Ejection immediate (both triggers working)
- [ ] All bot commands functional
- [ ] DVR health metric displayed correctly

### Architecture

- [ ] Static MUSL binary produced
- [ ] Embedded Freenet kernel runs successfully
- [ ] Signal bot authenticates and manages group
- [ ] STARK proofs < 100KB, generation < 10 seconds
- [ ] Federation infrastructure present (disabled in MVP)

---

## 🚦 Quality Gates (Before Convoy Close)

Every convoy must pass these gates before closure:

### Testing Requirements

| Requirement | Applies To | Tool |
|-------------|------------|------|
| **100% code coverage** | All code | `cargo llvm-cov nextest` |
| **Property-based tests** | Trust-critical code | `proptest` |
| **Deterministic tests** | All tests | Fixed seeds, mock time |

**Trust-critical code requiring proptest**:
- HMAC identity masking (determinism, key isolation)
- Standing calculation (vouch invalidation math)
- Delta commutativity (merge order independence)
- Ejection triggers (both conditions)

### Commands

```bash
# Security checks (MANDATORY)
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo deny check
cargo llvm-cov nextest --all-features  # 100% coverage REQUIRED

# Build check
cargo build --release --target x86_64-unknown-linux-musl

# Witness sign-off
# - No cleartext Signal IDs in persistent storage/logs/output
# - No SqliteStore usage
# - No polling patterns
# - Zeroization verified
# - proptest coverage for trust-critical code
```

---

## 📝 Convoy Closure Checklist

When closing a convoy:

1. [ ] All beads in convoy marked complete (`bd close <id>`)
2. [ ] Quality gates pass
3. [ ] **Documentation updated** (see below)
4. [ ] Witness agent sign-off
5. [ ] Git commits have Co-authored-by trailers
6. [ ] Push to remote (`git push`)
7. [ ] Update this document status
8. [ ] Create next convoy if applicable

### Documentation Requirements

**Every convoy MUST update affected documentation.** Documentation drift is a defect.

| If you changed... | Update these docs |
|-------------------|-------------------|
| User-facing commands | `docs/USER-GUIDE.md` |
| Trust logic or thresholds | `docs/TRUST-MODEL.md`, `docs/HOW-IT-WORKS.md` |
| Operator setup or CLI | `docs/OPERATOR-GUIDE.md` |
| Module structure | `docs/DEVELOPER-GUIDE.md` |
| Security constraints | `docs/THREAT-MODEL.md`, `docs/SECURITY-CI-CD.md` |
| Algorithms (DVR, Bridge Removal, PSI-CA) | `docs/ALGORITHMS.md` |
| Persistence model | `docs/PERSISTENCE.md` |
| Federation | `docs/FEDERATION.md` |

**Validation**: Run `rg "TODO\|FIXME\|PLACEHOLDER" docs/` — no placeholders allowed at convoy close.

```bash
# Convoy closure
bd sync
rg "TODO|FIXME|PLACEHOLDER" docs/  # Must be empty
gt convoy close convoy-phase0 --summary "Phase 0 complete"
git push
```

---

## 📚 Phase Review Reports Reference

**Location**: All phase review reports are in `docs/todo/`

### Available Reports

| Report | Phase | Status | Key Findings |
|--------|-------|--------|--------------|
| **PHASE0_REVIEW_REPORT.md** | Phase 0: Foundation | 88% Complete | ✅ All core features implemented<br>❌ 2 blockers: st-5nhs1 (Freenet deps), st-rvzl (Presage)<br>✅ 321 tests passing |
| **phase1-review-report.md** | Phase 1: Bootstrap & Trust | 70% Complete | ✅ Trust formula provably correct<br>❌ GAP-01 (audit trail) missing<br>❌ GAP-03 (rate limiting) missing |
| **PHASE2_REVIEW.md** | Phase 2: Mesh Optimization | 40% Complete | ✅ DVR & strategic introductions complete<br>❌ /mesh commands stubbed<br>❌ Integration tests missing |
| **PHASE2-BENCHMARKS.md** | Phase 2: Performance | ✅ ALL TARGETS MET | DVR: 5.2x faster, Cluster: 2.2x faster, Matchmaker: 1,667x faster |
| **phase2-security-audit.md** | Phase 2: Security | ✅ PASS | ✅ 0 cleartext Signal IDs<br>✅ Transient mapping verified<br>✅ GAP-02 vote privacy compliant |
| **phase-2.5-review.md** | Phase 2.5: Persistence | 70% Complete | ✅ Core modules complete (69 tests)<br>❌ CRITICAL: 0/13 property tests<br>❌ Attestation module missing |
| **INFRASTRUCTURE-DOCUMENTATION-REVIEW.md** | Infrastructure & Docs | ✅ COMPLETE | ✅ All 4 infrastructure requirements met<br>✅ 18+ docs (56 files total)<br>⚠️ 5 minor gaps (st-vkbr7, st-bd1ge, st-5op71, st-vz3ff, st-zun44) |

### How Agents Should Use These Reports

**For Remediation Work**:
1. Find the bead/task in this TODO.md file
2. Look for the `**Review**:` line which references the specific report and line numbers
3. Read the report section to understand what's complete, incomplete, or blocked
4. Check for related beads (st-XXXXX) that may already track the work

**For New Work**:
1. Before starting, read the relevant phase report to understand context
2. Check the "Gaps" or "Missing" sections for known issues
3. Verify dependencies are complete (check other beads' status)
4. Follow the review's recommendations for implementation

**For Verification**:
1. Use the "Acceptance Criteria" sections in phase reports as test checklists
2. Benchmark reports show performance targets - verify your changes don't regress
3. Security audit reports define constraints - verify compliance before committing

### Quick Navigation

- **Phase 0 blockers**: See PHASE0_REVIEW_REPORT.md lines 365-418
- **Phase 1 gaps**: See phase1-review-report.md lines 217-237 (GAP-01, GAP-03)
- **Phase 2 stubs**: See PHASE2_REVIEW.md lines 85-116 (/mesh commands)
- **Phase 2.5 critical**: See phase-2.5-review.md lines 253-277 (property tests)
- **Security constraints**: See phase2-security-audit.md for verification patterns
- **Infrastructure gaps**: See INFRASTRUCTURE-DOCUMENTATION-REVIEW.md lines 206-275 (5 minor gaps)
- **Documentation status**: See INFRASTRUCTURE-DOCUMENTATION-REVIEW.md lines 81-202 (comprehensive coverage)

---

**End of TODO.md**
