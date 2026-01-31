# Stroma Implementation Checklist

## 📊 Project Status Overview

**Last Updated**: 2026-01-30

### ✅ Completed (Architectural Foundation)
- [X] Git repository initialized (6 commits on signal-bot branch)
- [X] 7 architectural constraint beads created:
  - [X] bot-deployment-model.bead (1:1 bot-to-group)
  - [X] group-identity.bead (group names required)
  - [X] governance-model.bead (bot execute-only, operator no privileges)
  - [X] proposal-system.bead (/propose command structure)
  - [X] technology-stack.bead (Presage over libsignal-service-rs)
  - [X] voting-mechanism.bead (native polls for anonymity)
  - [X] poll-implementation-gastown.bead (Agent-Signal task)
- [X] All rules updated with architectural decisions (6 files)
- [X] All documentation updated (4 files)
- [X] Comprehensive UX specification (user-roles-ux.mdc)
- [X] Trust model with vouch invalidation
- [X] Mesh health score design (peaks at 30-60% density)
- [X] Technology stack finalized (Presage, forked libsignal-service-rs, STARKs)
- [X] Gastown workspace plan updated
- [X] Signal bot provisioning tool (Fish script)

### ✅ Completed: Protocol v8 Poll Support
**Objective**: Agent-Signal implements protocol v8 poll support in forked libsignal-service-rs

**Why Critical**: Native polls provide anonymous voting (reactions expose voters)

**Timeline**: 1-2 weeks ✅ **COMPLETED**

**Bead**: `.beads/poll-implementation-gastown.bead`

**Status**: Forked libsignal-service-rs with protocol v8 poll support (feature/protocol-v8-polls-fixed)

### ✅ Completed: Spike Week (Week 0 - Validation Phase)
**Objective**: Validate core technologies before Phase 0 implementation

**Decision**: **✅ GO — PROCEED TO PHASE 0**

All six outstanding questions answered:
- Q1 (Freenet Merge): ✅ GO — commutative deltas with set-based state
- Q2 (Contract Validation): ✅ GO — trustless model viable
- Q3 (Cluster Detection): ✅ GO — Bridge Removal algorithm
- Q4 (STARK Verification): ✅ PARTIAL — Bot-side for Phase 0
- Q5 (Merkle Tree): ✅ GO — 0.09ms at 1000 members
- Q6 (Proof Storage): ✅ Outcomes only (not proofs)

**See**: [SPIKE-WEEK-BRIEFING.md](spike/SPIKE-WEEK-BRIEFING.md) and [Outstanding Questions](../spike/SPIKE-WEEK-BRIEFING.md#outstanding-questions-status-tracking)

### 📋 Next Phase: Phase 0 (Weeks 1-2)
**Objective**: Foundation implementation with federation-ready design

**Next Actions**: 
1. ✅ Run Pre-Gastown Audit (see PRE-GASTOWN-AUDIT.md)
2. Begin Phase 0 implementation (HMAC, Freenet integration, Signal bot, STARK circuits, Contract schema)
3. Track progress in Beads (bd)

### 📋 Tracked for Implementation (Not Yet Started)
- [ ] Dockerfile (hardened container wrapping static binary)
- [ ] GitHub Actions release workflow (binary + container)
- [ ] GitHub Actions CI workflow (tests, security audits)
- [ ] All code modules (see Phase 0-3 below)

---

## 🚀 Immediate Actions

### Git & Workspace Setup
- [X] Complete git initialization
  - [X] Stage all existing files: `git add .`
  - [X] Create initial commit
  - [X] Optionally set up remote

### Constraint Beads (Immutable)
- [X] Create `.beads/security-constraints.bead`
  - [X] Anonymity-first design rules
  - [X] No cleartext Signal IDs
  - [X] Immediate ejection protocol
  - [X] HMAC hashing requirements
  - [X] Zeroization requirements
  - [X] Vouch permissions (ANY Member can vouch)
  - [X] Co-authored-by requirement for Claude commits
  
- [X] Create `.beads/architecture-decisions.bead`
  - [X] freenet-stdlib as embedded kernel (#9)
  - [X] STARKs (winterfell) for ZK-proofs
  - [X] Each bot runs own embedded kernel (no sharing)
  - [X] Performance targets
  - [X] Threat model
  - [X] Single binary, two distributions (#10)
  - [X] Operator CLI design
  
- [X] Create `.beads/federation-roadmap.bead`
  - [X] MVP = single group (no federation)
  - [X] Federation as north star
  - [X] Phase 4+ federation features
  - [X] Design principles

### Agent Structure (Gastown Coordination)
- [X] Define agent boundaries
  - [X] **Agent-Signal** (Priority): Presage integration, poll support, bot commands
  - [X] Agent-Freenet: Embedded Freenet kernel integration
  - [X] Agent-Crypto: STARKs + HMAC + zeroization
  - [X] Witness-Agent: Security audit (continuous)
  
- [X] Create architectural constraint beads (7 total)
  - [X] bot-deployment-model.bead (1:1 bot-to-group)
  - [X] group-identity.bead (group names required)
  - [X] governance-model.bead (bot execute-only)
  - [X] proposal-system.bead (/propose structure)
  - [X] technology-stack.bead (Presage layers)
  - [X] voting-mechanism.bead (native polls)
  - [X] poll-implementation-gastown.bead (Agent-Signal task)
  
- [X] Launch Agent-Signal for poll implementation
  - [X] Agent reads poll-implementation-gastown.bead
  - [X] Agent forks libsignal-service-rs
  - [X] Agent implements protocol v8 support (PollCreate, PollVote, PollTerminate, PinMessage, UnpinMessage)
  - [X] Agent bases branch on b48b42fbc (commit presage pins to)
  - [X] Agent configures Stroma Cargo.toml with patches:
        - patch libsignal-service → our fork (feature/protocol-v8-polls-fixed)
        - patch curve25519-dalek → Signal's fork (per libsignal-service-rs README)
  - [X] Build verified: Stroma + presage + freenet all build successfully
  - [X] Unit tests verify poll protobuf serialization
  - [ ] Validate polls work end-to-end with Stroma bot (during bot implementation, not Spike Week)
  - [ ] Submit PR to upstream whisperfish/libsignal-service-rs (DEPENDS ON validation)
  
  **Note**: Poll end-to-end testing deferred to Phase 1 (bot implementation). Not a Spike Week priority
  because: (1) protobuf definitions from official Signal-Desktop, (2) unit tests pass, (3) architectural
  risk is low. Spike Week focuses on Freenet/STARK unknowns.

### Outstanding Questions (Spike Week Progress) — ✅ ALL COMPLETE

**Track in Multiple Locations** (docs/todo/TODO.md, docs/spike/SPIKE-WEEK-BRIEFING.md, README.md):

1. ✅ **Q1: Freenet Conflict Resolution** — COMPLETE (GO)
   - Freenet applies deltas via commutative set union
   - Contract's responsibility to ensure commutativity
   - Use set-based state with tombstones (remove-wins)
   - See: `docs/spike/q1/RESULTS.md`

2. ✅ **Q2: Contract Validation** — COMPLETE (GO)
   - Can contracts reject invalid state transitions? **YES**
   - `update_state()` returns `Err(ContractError::InvalidUpdate)` to reject delta
   - `validate_state()` returns `ValidateResult::Invalid` to reject merged state
   - **Trustless model viable** — contract enforces invariants
   - See: `docs/spike/q2/RESULTS.md`

3. ✅ **Q3: Cluster Detection** — COMPLETE (GO)
   - Bridge Removal algorithm (Tarjan's) distinguishes tight clusters
   - Standard Union-Find fails (sees 1 cluster), Bridge Removal works
   - Charlie becomes isolated bridge node; A and B are distinct clusters
   - See: `docs/spike/q3/RESULTS.md`

4. ✅ **Q4: STARK Verification in Wasm** — COMPLETE (PARTIAL)
   - winterfell Wasm support is experimental
   - **Bot-side verification** for Phase 0 (native winterfell)
   - Can migrate to contract-side when Wasm improves
   - See: `docs/spike/q4/RESULTS.md`

5. ✅ **Q5: On-Demand Merkle Tree Performance** — COMPLETE (GO)
   - 1000 members: 0.09ms (1000x faster than threshold)
   - **Generate on demand** (no caching needed)
   - 5000 members: 0.45ms (still sub-millisecond)
   - See: `docs/spike/q5/RESULTS.md`

6. ✅ **Q6: Proof Storage Strategy** — COMPLETE
   - **Store outcomes only** (not proofs)
   - Proofs are ephemeral (10-100KB each)
   - Contract stores "Alice vouched for Bob", not the proof
   - See: `docs/spike/q6/RESULTS.md`

**Status**: ✅ ALL QUESTIONS COMPLETE — Proceed to Phase 0

## ✅ Phase -1: Protocol v8 Poll Support (COMPLETED)

**Duration**: 1-2 weeks  
**Assigned**: Agent-Signal  
**Bead**: `.beads/poll-implementation-gastown.bead`  
**Status**: COMPLETED

### Why Polls Are Critical

**Anonymity** (Non-Negotiable):
- ✅ Signal Polls: Votes are anonymous (Signal doesn't expose who voted what)
- ❌ Emoji Reactions: Public (everyone sees who reacted with what emoji)
- Stroma's philosophy: Privacy-first, non-hierarchical decision-making

**Better Decision Making:**
- Polls support multiple choice options
- Reactions are binary only (👍/👎)

### Completed Tasks

- [X] **Fork libsignal-service-rs**
  - Fork created at https://github.com/roder/libsignal-service-rs
  - Branch: `feature/protocol-v8-polls-fixed`

- [X] **Add poll protobuf definitions**
  - [X] Copy from Signal-Desktop `protos/SignalService.proto`
  - [X] Add to `protobuf/SignalService.proto`:
    - [X] `message PollCreate` (field 24)
    - [X] `message PollTerminate` (field 25)
    - [X] `message PollVote` (field 26)
    - [X] `message PinMessage` (field 27)
    - [X] `message UnpinMessage` (field 28)
  - [X] Update protocol version from v7 to v8

- [X] **Build and test**
  - [X] `cargo build` succeeds (libsignal-service-rs)
  - [X] 21 tests pass (16 existing + 5 new poll tests)
  - [X] Protobuf serialization roundtrip validated

- [X] **Push to fork**
  - [X] Commit: `feat: Add protocol v8 poll support` (91805d38c)
  - [X] Commit: `test: Add unit tests for protocol v8 poll types` (532a8d64e)
  - [X] Pushed to `feature/protocol-v8-polls-fixed`

- [X] **Update Stroma's Cargo.toml**
  - [X] Patch libsignal-service to use our fork
  - [X] Patch curve25519-dalek per upstream README (resolves version conflict)
  - [X] Stroma + presage + freenet all build successfully

- [ ] **Test in Stroma** (Phase 2.5: Validation)
  - [ ] Create example binary for poll lifecycle testing
  - [ ] Can create polls via presage
  - [ ] Can vote on polls
  - [ ] Vote anonymity verified with real Signal account

- [ ] **Submit PR to upstream** (BLOCKED on validation)
  - PR submission depends on Phase 2.5 successful validation
  - See `.beads/poll-implementation-gastown.bead` for PR template

**Deliverable**: Poll support available in Stroma via fork ✅ (awaiting end-to-end validation before upstream PR)

## 🔬 Spike Week (Week 0 - Validation Phase) — ✅ COMPLETE

**Objective**: Validate *risky technical unknowns* before committing to architecture

**Focus**: Freenet contracts, STARK proofs, ComposableState — NOT Signal integration

**Rationale**: Signal/Presage integration is low-risk (protobuf from official client, unit tests pass).
Poll end-to-end testing belongs in Phase 2.5 validation (when we have Stroma bot), not Spike Week.

### ✅ Day 1-2: Embedded Freenet Kernel & Contract Design (COMPLETE)
- [X] Test embedded Freenet kernel (in-process, not external service)
  - [X] **Q1 Spike**: Freenet merge conflicts — use commutative deltas with set-based state + tombstones
  - [X] **Q2 Spike**: Contract validation — `update_state()` and `validate_state()` can enforce invariants
  
- [X] Test ComposableState trait (CRITICAL - architectural risk)
  - [X] Implement simple ComposableState (e.g., counter)
  - [X] Test merge semantics (create two states, merge them)
  - [X] Verify merge is commutative (order-independent)
  
- [X] Test set-based membership (Stroma-specific)
  - [X] Implement MemberSet with BTreeSet (active + removed tombstones)
  - [X] Test adding members to set
  - [X] Test removing members (tombstone pattern)
  - [X] Test merging two divergent member sets
  - [X] Verify tombstones prevent re-addition
  
- [X] Test on-demand Merkle Tree generation
  - [X] **Q5 Spike**: On-demand Merkle generation — 0.09ms at 1000 members (GO)
  - [X] Benchmark: 10ms @ 100 members, 0.14ms @ 1000 members, 0.45ms @ 5000 members
  - [X] Measure tree generation time: **1000x faster than threshold** (< 100ms)

### ✅ Day 3-4: State Monitoring & Outstanding Questions (COMPLETE)
- [X] Test state stream monitoring
  - [X] Deploy contract to Freenet
  - [X] Subscribe to state changes (real-time, not polling)
  - [X] Submit state update from one node
  - [X] Verify other node receives update via stream
  
- [X] **Answer Outstanding Questions** (CRITICAL - these determine architecture)
  - [X] **Q1: Freenet Conflict Resolution** — GO — Use commutative deltas with set-based state + tombstones
  - [X] **Q2: Contract Validation** — GO — Trustless model viable (`update_state()` + `validate_state()`)
  - [X] **Q3: Cluster Detection** — GO — Bridge Removal algorithm (Tarjan's) for tight cluster separation
  - [X] **Q4: STARK Verification in Wasm** — PARTIAL — Bot-side verification for Phase 0 (Wasm experimental)
  - [X] **Q5: Merkle Tree Performance** — GO — 0.09ms for 1000 members (on-demand OK)
  - [X] **Q6: Proof Storage Strategy** — Store outcomes only (not proofs)
  
- [X] Document findings
  - [X] Merkle Tree approach: Store set, generate tree on-demand (Q5)
  - [X] ZK-proof strategy: Bot-side verification for Phase 0 (Q4, upgrade later)
  - [X] Cluster detection: Bridge Removal algorithm (Q3)
  - [X] Merge semantics: CRDT-like patterns with tombstones (Q1)
  - [X] Contract validation: Two-layer model (update_state + validate_state) (Q2)

### ✅ Day 5: STARK Proof Generation (Architectural Risk) (COMPLETE)
- [X] Set up winterfell library
  - [X] Research winterfell capabilities and Wasm support
  - [X] Review Wasm compilation challenges
  
- [X] Evaluate proof generation & verification
  - [X] **Q4 Spike**: STARK verification — winterfell Wasm is experimental (risk)
  - [X] **Q6 Decision**: Store outcomes only (not proofs) — simplifies contract state
  - [X] Performance: Native winterfell < 1ms, Wasm would be 10-100x slower
  
- [X] Measure performance (Simulated)
  - [X] Proof size: < 100KB per STARK proof
  - [X] Proof generation time: Fast on native, acceptable latency
  - [X] Verification time: Constant time (scalable)
  
- [X] Document findings
  - [X] winterfell practical for Phase 0 (native verification)
  - [X] STARKs viable (transparent, post-quantum, no trusted setup)
  - [X] Wasm verification deferred to Phase 4+ (when mature)

### ✅ Spike Week Deliverable (COMPLETE)
- [X] Create Go/No-Go decision report
  - [X] Freenet validation: ✅ GO (commutative merge works)
  - [X] Contract validation: ✅ GO (trustless model viable)
  - [X] STARK proofs: ✅ PARTIAL (bot-side verification for Phase 0)
  - [X] Recommendation: **✅ PROCEED TO PHASE 0**
  - [X] Identified risks and mitigations documented

**See [SPIKE-WEEK-BRIEFING.md](../spike/SPIKE-WEEK-BRIEFING.md)** for complete analysis with results for Q1-Q6

---

## 🔍 Pre-Gastown Audit (Final Human Review)

**Objective**: Systematic human audit before turning project over to Gastown agents

**Why Critical**: Gastown agents follow guidance literally. Any inconsistencies, ambiguities, or contradictions will cause incorrect implementations or require human intervention. This audit ensures clean handoff.

**Timing**: After Spike Week completes, before Phase 0 implementation begins

→ **[PRE-GASTOWN-AUDIT.md](../todo/PRE-GASTOWN-AUDIT.md)** - Complete audit checklist (6-9 hours estimated)

### Audit Phases
- [ ] **Phase 1**: Terminology sweep (1-2h)
  - [ ] Search all files for "cluster" vs "friend circles"
  - [ ] Categorize by audience (user-facing vs technical)
  - [ ] Decide on Option A, B, or C (see audit doc)
  - [ ] Update inconsistent files

- [ ] **Phase 2**: Architectural consistency (2-3h)
  - [ ] Review each bead against checklist
  - [ ] Cross-reference with rules and docs
  - [ ] Flag contradictions for resolution
  - [ ] Document decisions

- [ ] **Phase 3**: Security constraint verification (1-2h)
  - [ ] Review all "NEVER" rules in security-guardrails.mdc
  - [ ] Verify enforcement in contract design and bot architecture
  - [ ] Check that docs reflect security model
  - [ ] Test threat model against architecture

- [ ] **Phase 4**: Spike Week alignment (1h)
  - [ ] Review Q1-Q6 in SPIKE-WEEK-BRIEFING.md
  - [ ] Verify beads note dependencies
  - [ ] Check fallback strategies documented
  - [ ] Confirm no architectural decisions bypass Spike Week

- [ ] **Phase 5**: Final review (1h)
  - [ ] Read through all beads in sequence
  - [ ] Imagine you're a Gastown agent — is everything clear?
  - [ ] Flag any ambiguities or missing context
  - [ ] Update audit checklist with findings

### Go/No-Go Decision
- [ ] **GO Decision**: All audit criteria met, ready for agent handoff
  - [ ] Terminology consistent within each audience
  - [ ] No contradictions between beads, rules, and docs
  - [ ] Security constraints consistently enforced
  - [ ] Spike Week dependencies noted and fallbacks provided
  - [ ] Agent handoff feels confident

- [ ] **NO-GO Decision**: Critical issues identified, must fix before handoff
  - [ ] Document specific issues in PRE-GASTOWN-AUDIT.md
  - [ ] Create fix action items
  - [ ] Re-audit after fixes

---

## 📦 Phase 0: Foundation (Weeks 1-2)

**Prerequisites**: 
- ✅ Spike Week complete (Q1-Q6 answered)
- ✅ Pre-Gastown Audit passed (GO decision)
- ✅ All architectural guidance consistent and ready for agents

**Objective**: Core infrastructure with federation-ready design

### Module Structure
- [ ] Create `src/cli/` directory (Operator CLI interface)
  - [ ] `src/cli/mod.rs`
  - [ ] `src/cli/bootstrap.rs` - Bootstrap command (seed group)
  - [ ] `src/cli/run.rs` - Run command (normal operation)
  - [ ] `src/cli/utils.rs` - Status, verify, export-pepper, version
  
- [ ] Create `src/kernel/` directory
  - [ ] `src/kernel/mod.rs`
  - [ ] `src/kernel/hmac.rs` - HMAC-based hashing
  - [ ] `src/kernel/zeroize_helpers.rs` - Immediate buffer purging
  
- [ ] Create `src/freenet/` directory (Embedded kernel, not external service)
  - [ ] `src/freenet/mod.rs`
  - [ ] `src/freenet/embedded_kernel.rs` - In-process Freenet kernel (freenet-stdlib)
  - [ ] `src/freenet/contract.rs` - Wasm contract deployment to embedded kernel
  - [ ] `src/freenet/state_stream.rs` - Real-time state monitoring from embedded kernel
  
- [ ] Create `src/signal/` directory (Presage-based)
  - [ ] `src/signal/mod.rs`
  - [ ] `src/signal/bot.rs` - Presage Manager, authentication
  - [ ] `src/signal/group.rs` - Group management (add/remove)
  - [ ] `src/signal/pm.rs` - 1-on-1 PM handling
  - [ ] `src/signal/polls.rs` - Poll creation/monitoring (protocol v8)
  
- [ ] Create `src/crypto/` directory
  - [ ] `src/crypto/mod.rs`
  - [ ] `src/crypto/stark_circuit.rs` - STARK circuit for vouching
  - [ ] `src/crypto/proof_generation.rs` - Generate proofs
  - [ ] `src/crypto/proof_verification.rs` - Verify proofs
  
- [ ] Create `src/gatekeeper/` directory
  - [ ] `src/gatekeeper/mod.rs`
  - [ ] `src/gatekeeper/admission.rs` - Vetting & admission logic
  - [ ] `src/gatekeeper/ejection.rs` - Immediate ejection
  - [ ] `src/gatekeeper/health_monitor.rs` - Continuous standing checks
  
- [ ] Create `src/matchmaker/` directory
  - [ ] `src/matchmaker/mod.rs`
  - [ ] `src/matchmaker/graph_analysis.rs` - Topology analysis
  - [ ] `src/matchmaker/cluster_detection.rs` - Identify internal clusters
  - [ ] `src/matchmaker/strategic_intro.rs` - MST optimization
  
- [ ] Create `src/config/` directory
  - [ ] `src/config/mod.rs`
  - [ ] `src/config/group_config.rs` - GroupConfig struct
  
- [ ] Create `src/proposals/` directory
  - [ ] `src/proposals/mod.rs`
  - [ ] `src/proposals/command.rs` - /propose parser
  - [ ] `src/proposals/poll.rs` - Signal Poll creation/monitoring
  - [ ] `src/proposals/executor.rs` - Execute approved actions
  
- [ ] Create `src/federation/` directory (disabled in MVP)
  - [ ] `src/federation/mod.rs`
  - [ ] `src/federation/shadow_beacon.rs` - Social Anchor Hashing (unused)
  - [ ] `src/federation/psi_ca.rs` - PSI-CA (unused)
  - [ ] `src/federation/diplomat.rs` - Federation proposals (unused)

### Cargo Configuration
- [X] Update `Cargo.toml`
  - [X] Add freenet-stdlib with "full" features (embedded kernel)
  - [X] Add freenet-scaffold (ComposableState utilities)
  - [ ] Add ring (HMAC)
  - [ ] Add zeroize (memory hygiene)
  - [ ] Add winterfell (STARKs)
  - [ ] Add libsignal-service-rs
  - [ ] Add tokio (async runtime)
  - [ ] Add serde (serialization)
  - [ ] Add ciborium (CBOR for Freenet contracts)
  - [ ] Add tracing (structured logging)
  - [ ] Add clap (CLI argument parsing)
  
- [ ] Create `.cargo/config.toml`
  - [ ] Configure MUSL target: `x86_64-unknown-linux-musl`
  - [ ] Add linker configuration
  - [ ] Add rustflags for static linking
  
- [ ] Create `cargo-deny.toml`
  - [ ] Configure advisories (deny vulnerabilities)
  - [ ] Configure licenses (allow list)
  - [ ] Configure bans (deny multiple versions)
  - [ ] Configure sources (deny unknown registries)

### Distribution & Deployment Infrastructure (TRACK FOR FUTURE)

**Critical Principle**: Single binary artifact, two distribution methods (no security compromise)

#### Dockerfile Creation
- [ ] Create `Dockerfile` (hardened container wrapping static binary)
  - [ ] Multi-stage build pattern:
    ```dockerfile
    # Stage 1: Builder (build static MUSL binary)
    FROM rust:1.93-alpine AS builder
    # ... build stroma-x86_64-musl
    
    # Stage 2: Runtime (distroless - no shell, no package manager)
    FROM gcr.io/distroless/static:nonroot
    COPY --from=builder /build/stroma /stroma
    USER nonroot:nonroot
    ENTRYPOINT ["/stroma"]
    ```
  - [ ] Security features:
    - [ ] FROM scratch or distroless (minimal base)
    - [ ] Non-root user (UID 65532)
    - [ ] Read-only root filesystem
    - [ ] No shell, no package manager
    - [ ] Only contains the static binary
  - [ ] Document: Container uses SAME binary as standalone (no security compromise)
  
#### GitHub Actions Release Workflow
- [ ] Create `.github/workflows/release.yml` (triggered on git tags)
  - [ ] **Build Phase**:
    - [ ] Checkout code
    - [ ] Setup Rust 1.93 with x86_64-unknown-linux-musl target
    - [ ] Build static binary: `cargo build --release --target x86_64-unknown-linux-musl`
    - [ ] Output: `stroma-v$VERSION-x86_64-musl`
  - [ ] **Sign & Checksum Binary**:
    - [ ] Generate SHA256: `sha256sum stroma > stroma.sha256`
    - [ ] GPG sign binary: `gpg --detach-sign --armor stroma`
    - [ ] Output: `stroma.asc` (signature)
  - [ ] **Build Container Image** (from same binary):
    - [ ] Copy static binary into Dockerfile context
    - [ ] Build image: `docker build -t ghcr.io/roder/stroma:$VERSION`
    - [ ] Tag as `:latest` if main release
    - [ ] Sign image with cosign: `cosign sign ghcr.io/roder/stroma:$VERSION`
  - [ ] **Publish Artifacts**:
    - [ ] Publish to GitHub Releases:
      - `stroma-x86_64-musl` (binary)
      - `stroma.sha256` (checksum)
      - `stroma.asc` (GPG signature)
    - [ ] Push image to ghcr.io/roder/stroma
    - [ ] Push image signature (cosign)
  - [ ] **Verify Reproducible Build**:
    - [ ] Build twice, compare checksums
    - [ ] Document build environment
    - [ ] Enable users to verify binary matches source

#### GitHub Actions CI Workflow  
- [ ] Create `.github/workflows/ci.yml` (on push, PR)
  - [ ] **Test Phase**:
    - [ ] cargo test --all-features
    - [ ] cargo nextest run (if using nextest)
  - [ ] **Lint Phase**:
    - [ ] cargo clippy -- -D warnings
    - [ ] cargo fmt --check
  - [ ] **Security Audit Phase**:
    - [ ] cargo deny check (dependencies, licenses, advisories)
    - [ ] cargo audit (vulnerabilities)
    - [ ] Scan for cleartext Signal IDs (grep patterns)
  - [ ] **Coverage Phase** (optional):
    - [ ] cargo llvm-cov nextest
    - [ ] Upload to codecov or similar

#### Container Image Hardening Documentation
- [X] Document security analysis in `.beads/architecture-decisions.bead`
  - [X] Attack surface comparison: Standalone vs Container
  - [X] Mitigation: Same binary in both (no compromise)
  - [X] Justification: ~100KB runtime overhead acceptable for 80% ease gain
  - [X] Verification: Image signature with cosign
- [X] Document in `docs/OPERATOR-GUIDE.md`:
  - [X] Container deployment guide (docker-compose + standalone)
  - [X] Image verification steps (cosign)
  - [X] Security properties of distroless base
  - [X] Comparison table: attack surface vs ease of use

### Recent Architectural Changes (2026-01-27)

#### Change #1: Embedded Freenet Kernel
**Decision**: Embed Freenet kernel in-process (not external service)

**Updated Files:**
- [X] `.beads/architecture-decisions.bead` - Added decision #9
- [X] `.cursor/rules/freenet-integration.mdc` - Updated to reflect embedded kernel
- [X] `.cursor/rules/operator-cli.mdc` - Created new rule for CLI design
- [X] `docs/OPERATOR-GUIDE.md` - Updated installation, bootstrap, monitoring
- [X] `docs/DEVELOPER-GUIDE.md` - Updated module structure, event loop
- [X] `Cargo.toml` - Added freenet-stdlib dependency
- [X] `README.md` - Updated tech stack, getting started

**Implementation Status**: Design complete, tracked for Spike Week validation

#### Change #2: Single Binary, Two Distributions
**Decision**: Build ONE static binary, distribute via standalone + container

**Updated Files:**
- [X] `.beads/architecture-decisions.bead` - Added decision #10
- [X] `docs/OPERATOR-GUIDE.md` - Added 3-tier deployment guide
- [X] `README.md` - Updated getting started section

**Key Insight**: Container wraps same binary as standalone (no security compromise)

**Implementation Status**: Design complete, Dockerfile tracked for Phase 0

#### Change #3: Operator CLI Interface
**Decision**: CLI for service management only (no trust operations)

**Commands Defined:**
- `stroma bootstrap` - One-time seed group initialization
- `stroma run` - Normal operation (embedded kernel)
- `stroma status` - Health check
- `stroma verify` - Config validation
- `stroma export-pepper` - Backup pepper
- `stroma version` - Version info

**Updated Files:**
- [X] `.beads/architecture-decisions.bead` - Module structure updated
- [X] `.cursor/rules/operator-cli.mdc` - Created comprehensive CLI spec
- [X] `docs/OPERATOR-GUIDE.md` - CLI usage examples
- [X] `docs/todo/TODO.md` - Added cli/ module to Phase 0

**Implementation Status**: Design complete, tracked for Phase 0

#### Change #4: Mesh Health Score UX
**Decision**: Normalize density to peak at optimal 30-60% range

**Updated Files:**
- [X] `README.md` - Added Mesh Health Score section
- [X] `.cursor/rules/user-roles-ux.mdc` - Updated bot command examples
- [X] `.cursor/rules/freenet-contract-design.mdc` - Added helper methods
- [X] `.beads/architecture-decisions.bead` - Updated network capacity notes

**Key Formula**: Health score = 100/100 when density is 30-60% (not at 100% density)

**Implementation Status**: Design complete, tracked for Phase 2 (Blind Matchmaker)

#### Change #5: Shadow Handover Protocol (Phase 4+ Documentation)
**Decision**: Document bot identity rotation protocol as Phase 4+ feature

**Updated Files:**
- [X] `.beads/federation-roadmap.bead` - Added full protocol specification
- [X] `.beads/architecture-decisions.bead` - Added decision #12
- [X] `.cursor/rules/operator-cli.mdc` - Added future `rotate` command
- [X] `docs/OPERATOR-GUIDE.md` - Added to disaster recovery section
- [X] `docs/DEVELOPER-GUIDE.md` - Added shadow_handover.rs to module structure
- [X] `docs/FEDERATION.md` - Added Shadow Handover section
- [X] `README.md` - Added to Federation (Phase 4+) features
- [X] `docs/todo/TODO.md` - Added to Phase 4+ roadmap

**Key Concept**: Bot's Signal identity (phone number) is ephemeral; cryptographic identity (keypair) persists. Succession documents signed by old bot authorize new bot.

**MVP Workaround**: Operator manually handles Signal bans by re-registering with backup phone number.

**Implementation Status**: Documented, deferred to Phase 4+

#### Change #6: Validator Threshold Strategy (Phased Approach)
**Decision**: Fixed thresholds for MVP, configurable safeguards for Phase 2, percentage-based for Phase 4+

**Rationale**:
- **MVP (Now)**: Small groups (3-30 members) with fixed Bridge=2, Validator=3+
  - Simplest implementation
  - Most transparent to members
  - Lowest governance overhead
  - Status: ✅ Implement in MVP

- **Phase 2 Gate**: Add configurable `min_vouch_threshold` (if medium groups stabilize)
  - Trigger: Operator feedback indicates stable 30-50 member groups
  - Scope: Allow groups to choose 2 (easier) vs 3+ (harder)
  - Safety: Requires consensus, cannot retroactively eject
  - Status: 📋 Design, gate decision before Phase 2

- **Phase 4 Gate**: Add percentage-based `validator_percentile` (if large groups request)
  - Trigger: Multiple federated groups request percentage-based scaling
  - Scope: Percentage-based validator threshold (e.g., top 20%)
  - Safety: Elevated consensus (85%), quarterly limit, min >= 3
  - Status: 📋 Design, gate decision before Phase 4

**Updated Files:**
- [ ] Create `docs/VALIDATOR-THRESHOLD-STRATEGY.md` - Comprehensive phased approach
- [ ] Update `.beads/architecture-decisions.bead` - Add validator strategy decision
- [ ] Update `docs/todo/TODO.md` - Add Phase 2 and Phase 4 gates

**Implementation Status**: Design complete, gates tracked for Phase 2 and Phase 4+ reviews

**Key Success Metrics**:
- MVP: Small groups stable with fixed thresholds
- Phase 2: Medium groups benefit from configurable min_vouch_threshold
- Phase 4: Large/federated groups benefit from percentage-based validators

### Phase 0 Beads Issues
- [ ] Create Bead-01: Operator CLI interface
  - [ ] `bd create --title "Implement operator CLI commands"`
  - [ ] Specify: Bootstrap command (seed group initialization)
  - [ ] Specify: Run command (normal operation with embedded kernel)
  - [ ] Specify: Utility commands (status, verify, export-pepper, version)
  - [ ] Specify: NO trust operation commands (operator least privilege)
  - [ ] Use clap for argument parsing
  
- [ ] Create Bead-02: HMAC identity masking with zeroization
  - [ ] `bd create --title "Implement HMAC identity masking"`
  - [ ] Specify: HMAC-SHA256 with group-secret pepper
  - [ ] Specify: Zeroize buffers immediately
  - [ ] Specify: Unit tests with fixed test pepper
  
- [ ] Create Bead-03: Embedded Freenet kernel integration
  - [ ] `bd create --title "Integrate embedded Freenet kernel"`
  - [ ] Specify: Use freenet-stdlib (not external freenet-core service)
  - [ ] Specify: Initialize kernel in-process
  - [ ] Specify: Dark mode (anonymous routing)
  - [ ] Specify: Single event loop for kernel + Signal
  - [ ] `bd create --title "Integrate freenet-core node"`
  - [ ] Specify: Node lifecycle management
  - [ ] Specify: Wasm contract deployment (stub)
  - [ ] Specify: State stream monitoring (real-time)
  
- [ ] Create Bead-03: Signal bot authentication and commands
  - [ ] `bd create --title "Implement Signal bot"`
  - [ ] Specify: Bot registration
  - [ ] Specify: Group management
  - [ ] Specify: 1-on-1 PM handling
  - [ ] Specify: Command parsing
  
- [ ] Create Bead-04: STARK circuits for vouch verification
  - [ ] `bd create --title "Implement STARK circuits"`
  - [ ] Specify: Circuit design
  - [ ] Specify: Proof generation
  - [ ] Specify: Proof verification
  
- [ ] Create Bead-05: Contract schema with federation hooks
  - [ ] `bd create --title "Design Freenet contract schema"`
  - [ ] Specify: TrustNetworkState struct
  - [ ] Specify: Federation hooks (unused in MVP)
  - [ ] Specify: GroupConfig struct

### Phase 0 Success Criteria
- [ ] freenet-core node runs successfully
- [ ] STARK proof generated (< 100KB, < 10 seconds)
- [ ] Signal bot can manage group (add/remove members)
- [ ] HMAC masking works with immediate zeroization
- [ ] Contract schema supports federation hooks (present but unused)

## 🚪 PHASE 2 GATE: Medium Group Decisions (Before Weeks 5-6)

**Trigger Condition**: Operator feedback indicates stable 30-50 member groups

**Decision Point**:
- [ ] Review Phase 2 Gate Questions (see `docs/VALIDATOR-THRESHOLD-STRATEGY.md`)
  - Do small groups naturally reach 30-50 members?
  - What percentage become Validators at current fixed threshold?
  - Do operators request min_vouch_threshold changes?
  - Are there observed downsides to fixed thresholds?

**If Phase 2 Gate Opens (YES, need configurability)**:
- [ ] Implement configurable `min_vouch_threshold`
  - [ ] Add to GroupConfig (range 2-4)
  - [ ] Add `/propose stroma min_vouch_threshold` command
  - [ ] Require config_change_threshold consensus
  - [ ] Cannot retroactively eject (new threshold only)

**If Phase 2 Gate Remains Closed (NO, fixed thresholds sufficient)**:
- [ ] Continue with fixed Bridge=2, Validator=3+
- [ ] Revisit gate during Phase 3 or before Phase 4

---

## 🌱 Phase 1: Bootstrap & Core Trust (Weeks 3-4)

**Objective**: Seed group, vetting, admission, ejection

### Bootstrap Module
- [ ] Implement seed group bootstrap
  - [ ] Manually add 3 seed members to Signal group
  - [ ] Create initial triangle vouching (all vouch for each other)
  - [ ] Initialize Freenet contract with 3 members
  - [ ] Each seed member has 2 vouches
  
- [ ] Verify bootstrap
  - [ ] Confirm all 3 members in Freenet state
  - [ ] Confirm all 3 members in Signal group
  - [ ] Confirm vouch counts are correct

### Trust Operations
- [ ] Implement invitation flow
  - [ ] Member sends `/invite @username [context]`
  - [ ] Bot records invitation as first vouch
  - [ ] Bot selects second Member via Blind Matchmaker
  - [ ] Bot sends PMs to invitee and selected Member
  
- [ ] Implement vetting interview
  - [ ] Bot creates 3-person chat (invitee, Member, bot)
  - [ ] Bot facilitates introduction
  - [ ] Member vouches via `/vouch @username`
  - [ ] Bot records second vouch in Freenet
  
- [ ] Implement admission
  - [ ] Bot verifies 2 vouches from different Members
  - [ ] Bot generates ZK-proof
  - [ ] Bot stores proof in Freenet contract
  - [ ] Bot adds invitee to Signal group (now a Bridge)
  - [ ] Bot announces admission
  - [ ] Bot deletes vetting session data
  
- [ ] Implement flagging
  - [ ] Member sends `/flag @username [reason]`
  - [ ] Bot records flag in Freenet
  - [ ] Bot recalculates: `Standing = Effective_Vouches - Regular_Flags`
  - [ ] If voucher flags: their vouch is invalidated (excluded from BOTH counts)
  - [ ] Bot checks ejection triggers (Standing < 0 OR Effective_Vouches < 2)

### Ejection Protocol
- [ ] Implement ejection triggers
  - [ ] Trigger 1: `Standing < 0` (too many flags)
  - [ ] Trigger 2: `Vouches < min_vouch_threshold` (voucher left)
  
- [ ] Implement immediate ejection
  - [ ] Bot removes member from Signal group
  - [ ] Bot sends PM to ejected member
  - [ ] Bot announces to group (uses hash, not name)
  - [ ] No grace period
  
- [ ] Implement health monitoring
  - [ ] Run heartbeat every 60 minutes
  - [ ] Check all members' standing
  - [ ] Trigger ejection if thresholds violated

### Basic Commands
- [ ] Implement `/invite @username [context]`
  - [ ] Parse command
  - [ ] Validate inviter is Member
  - [ ] Record as first vouch
  - [ ] Start vetting process
  
- [ ] Implement `/vouch @username`
  - [ ] Parse command
  - [ ] Validate voucher is Member
  - [ ] Record vouch in Freenet
  - [ ] Check if admission threshold met
  
- [ ] Implement `/flag @username [reason]`
  - [ ] Parse command
  - [ ] Validate flagger is Member
  - [ ] Validate reason is provided
  - [ ] Record flag in Freenet
  - [ ] Check ejection triggers
  
- [ ] Implement `/status`
  - [ ] Show user's own trust standing
  - [ ] Show vouch count
  - [ ] Show flag count
  - [ ] Show role (Bridge/Validator)

### Phase 1 Success Criteria
- [ ] 3-member seed group bootstrapped successfully
- [ ] New member admitted after 2 vouches (ZK-proof verified)
- [ ] Member ejected when `Standing < 0`
- [ ] Member ejected when `Effective_Vouches < 2` (includes voucher-flagger invalidation)
- [ ] Vouch invalidation works correctly (voucher who flags = vouch invalidated)
- [ ] All vetting in 1-on-1 PMs (no group chat exposure)
- [ ] No cleartext Signal IDs stored anywhere

## 🎯 Phase 2: Proposals & Mesh Optimization (Weeks 5-6)

**Objective**: Anonymous voting system, graph analysis, strategic introductions, MST

### Blind Matchmaker
- [ ] Implement graph topology analysis
  - [ ] Build trust graph from Freenet state
  - [ ] Identify Bridges (2 vouches)
  - [ ] Identify Validators (3+ vouches)
  - [ ] Calculate vouch distribution
  
- [ ] Implement cluster identification
  - [ ] Detect internal clusters (sub-communities)
  - [ ] Find disconnected islands
  - [ ] Calculate cluster sizes
  
- [ ] Implement strategic introduction suggestions
  - [ ] Priority 1: Connect Bridges to Validators (different clusters)
  - [ ] Priority 2: Connect Validators across islands
  - [ ] Generate introduction recommendations
  
- [ ] Implement MST optimization
  - [ ] Calculate minimum new interactions needed
  - [ ] Suggest strategic introductions to Members
  - [ ] Track introduction acceptance rate

### Advanced Commands
- [ ] Implement `/mesh` (network overview)
  - [ ] Show total member count
  - [ ] Show mesh density percentage
  - [ ] Show federation status (if any)
  - [ ] Show user's position in network
  
- [ ] Implement `/mesh strength` (histogram)
  - [ ] Calculate mesh density: `(Actual Vouches / Max Possible) × 100`
  - [ ] Generate histogram of vouch distribution
  - [ ] Show Bridges count (2 vouches)
  - [ ] Show Validators count (3+ vouches)
  - [ ] Show ASCII visualization
  
- [ ] Implement `/mesh config` (configuration view)
  - [ ] Show `group_name`
  - [ ] Show `config_change_threshold`
  - [ ] Show `default_poll_timeout`
  - [ ] Show `min_intersection_density`
  - [ ] Show `validator_percentile`
  - [ ] Show `min_vouch_threshold`
  - [ ] Show `min_vouch_threshold`
  
### Proposal System (`/propose`)

- [ ] Implement `/propose` command parser
  - [ ] Parse subcommand: config, stroma, federate
  - [ ] Parse arguments and options
  - [ ] Parse `--timeout` flag (optional, uses config default)
  - [ ] Validate parameters
  
- [ ] Implement `config` subcommand (Signal group settings)
  - [ ] `/propose config name "New Name"`
  - [ ] `/propose config description "..."`
  - [ ] `/propose config disappearing_messages 24h`
  - [ ] Validate Signal setting keys
  
- [ ] Implement `stroma` subcommand (Stroma trust config)
  - [ ] `/propose stroma min_vouch_threshold 3`
  - [ ] `/propose stroma config_change_threshold 0.80`
  - [ ] `/propose stroma default_poll_timeout 72h`
  - [ ] Validate Stroma config keys
  
- [ ] Implement `federate` subcommand (Phase 3+ only)
  - [ ] `/propose federate <group-id> --timeout 96h`
  - [ ] Validate group ID format
  - [ ] Placeholder for federation logic

- [ ] Implement proposal creation
  - [ ] Create Proposal struct in Freenet contract
  - [ ] Store proposal with timeout, threshold, action
  - [ ] Create Signal Poll for voting (anonymous)
  - [ ] Poll options: "👍 Approve", "👎 Reject"
  
- [ ] Implement poll monitoring
  - [ ] Check every 60 seconds for expired proposals
  - [ ] Fetch aggregated poll results from Signal
  - [ ] Calculate approval ratio
  - [ ] Mark proposal as checked (never check again)
  
- [ ] Implement automatic execution
  - [ ] If approved: execute action (update config, etc.)
  - [ ] Record result in Freenet contract
  - [ ] Announce result to group
  - [ ] Log execution in audit trail
  
- [ ] Verify anonymity
  - [ ] Confirm bot receives only vote counts (not individuals)
  - [ ] Verify no individual votes stored
  - [ ] Test with multiple voters

### Operator Audit
- [ ] Implement `/audit operator` command
  - [ ] Show operator action history (last 30 days)
  - [ ] Show action types (ServiceStart, ServiceRestart)
  - [ ] Show timestamps
  - [ ] Note: No manual operations logged (bot is automatic)

### Phase 2 Success Criteria
- [ ] Graph topology correctly identifies Bridges and Validators
- [ ] Strategic introductions suggested for MST
- [ ] Mesh density histogram displayed correctly
- [ ] Configuration changes via Signal Poll (70% threshold)
- [ ] Operator audit trail queryable
- [ ] Bot proactively suggests mesh optimization

## 🔧 Phase 3: Federation Preparation (Week 7)

### Phase 3 Pre-Implementation Review
- [ ] Validator Threshold Strategy review
  - [ ] **If Phase 2 Gate was closed**: Continue with fixed Bridge=2, Validator=3+
  - [ ] **If Phase 2 Gate was open**: Review configurable min_vouch_threshold implementation
  - [ ] Document feedback: Did configurability help medium groups?

---

## 🚪 PHASE 4 GATE: Large Group Decisions (Before Q2 2026)

**Trigger Condition**: Multiple federated groups request percentage-based validator scaling

**Decision Point**:
- [ ] Review Phase 4 Gate Questions (see `docs/VALIDATOR-THRESHOLD-STRATEGY.md`)
  - How many groups exceed 200 members?
  - Do federated groups report scaling issues?
  - Is fixed 3+ validator threshold limiting MST optimization?
  - Would percentage-based validator selection improve bridge density?

**If Phase 4 Gate Opens (YES, need percentage-based validators)**:
- [ ] Implement percentage-based `validator_percentile`
  - [ ] Add to GroupConfig (formula: `max(3, group_size * validator_percentile / 100)`)
  - [ ] Add `/propose stroma validator_percentile` command
  - [ ] Require elevated consensus (85%+ threshold)
  - [ ] Limit changes to once per quarter
  - [ ] Cannot retroactively change existing validators

**If Phase 4 Gate Remains Closed (NO, configurable threshold sufficient)**:
- [ ] Continue with current approach (fixed or configurable min_vouch_threshold)
- [ ] Revisit if large federated networks emerge

---

## 🔧 Phase 3: Federation Preparation (Week 7)

**Objective**: Validate federation infrastructure (locally, no broadcast)

### Shadow Beacon (Compute Locally)
- [ ] Implement Social Anchor hashing
  - [ ] Calculate top-N validators (percentile-based)
  - [ ] Generate discovery URI from validator hashes
  - [ ] Store locally (DO NOT broadcast in MVP)
  
- [ ] Implement validator percentile calculation
  - [ ] Sort members by vouch count
  - [ ] Calculate percentile threshold
  - [ ] Identify top validators
  
- [ ] Implement discovery URI generation
  - [ ] Hash social anchor
  - [ ] Generate multiple URIs (10%, 20%, 30%, 50%)
  - [ ] Store for future Phase 4 use

### PSI-CA (Test Locally)
- [ ] Implement Bloom filter generation
  - [ ] Create Bloom filter from member hashes
  - [ ] Optimize filter size/false positive rate
  - [ ] Serialize filter
  
- [ ] Implement commutative encryption
  - [ ] Encrypt Bloom filter (double-blinding)
  - [ ] Test encryption is commutative
  - [ ] Prepare for anonymous handshake
  
- [ ] Implement intersection density calculation
  - [ ] Calculate overlap: `|A ∩ B|`
  - [ ] Calculate union: `|A ∪ B|`
  - [ ] Calculate density: `|A ∩ B| / |A ∪ B|`
  - [ ] Test with mock data (simulate two groups)

### Contract Schema Validation
- [ ] Test federation hooks (present but unused)
  - [ ] Verify `federation_contracts` field exists
  - [ ] Verify `validator_anchors` field exists
  - [ ] Confirm they're empty in MVP
  
- [ ] Verify identity hashes are re-computable
  - [ ] Test HMAC hashing with different peppers
  - [ ] Confirm PSI-CA can work with hashes
  - [ ] Validate privacy preservation

### Documentation
- [ ] Create federation design document
  - [ ] Emergent discovery protocol
  - [ ] PSI-CA handshake protocol
  - [ ] BidirectionalMin threshold evaluation
  - [ ] Cross-mesh vouching protocol
  
- [ ] Create Phase 4+ roadmap
  - [ ] Shadow Beacon broadcast
  - [ ] Bot-to-bot discovery
  - [ ] Federation voting
  - [ ] Cross-mesh vouching implementation
  - [ ] Shadow Handover Protocol (bot identity rotation)
    - [ ] Bot keypair in Freenet contract schema
    - [ ] Succession document structure
    - [ ] Signature verification in contract verify()
    - [ ] `stroma rotate` CLI command
    - [ ] Graceful Bot-Old shutdown
    - [ ] Bot-New startup with succession verification
    - [ ] Signal group membership transfer logic

### Phase 3 Success Criteria
- [ ] Social Anchor hash computed correctly
- [ ] PSI-CA overlap calculated locally (test with mock data)
- [ ] Federation hooks in contract validated
- [ ] Documentation complete for Phase 4
- [ ] MVP ready for production deployment

## 🚢 Launch Phase 0 Convoy

- [ ] Brief Mayor agent
  - [ ] Provide technology stack decisions
  - [ ] Provide security constraints (read Beads)
  - [ ] Provide implementation roadmap
  - [ ] Provide agent coordination strategy
  
- [ ] Launch convoy with parallel agents
  ```bash
  gt convoy start \
    --phase "Phase 0: Foundation" \
    --beads "Bead-01,Bead-02,Bead-03,Bead-04,Bead-05" \
    --agents "Agent-Signal,Agent-Freenet,Agent-Crypto" \
    --witness "Witness-Agent"
  ```
  
- [ ] Monitor convoy progress
  - [ ] Check agent status
  - [ ] Review Witness Agent security audits
  - [ ] Verify no cleartext IDs in code
  - [ ] Verify zeroization implemented correctly

## 📊 Overall Success Metrics

### Security
- [ ] No cleartext Signal IDs stored anywhere
- [ ] All sensitive buffers zeroized immediately
- [ ] ZK-proofs used for all trust operations
- [ ] Memory dump contains only hashed identifiers
- [ ] cargo-deny and cargo-crev checks pass

### Functionality
- [ ] Seed group bootstrapped (3 members)
- [ ] Invitation & vetting flow working
- [ ] Admission requires 2 vouches from different Members
- [ ] Ejection immediate (both triggers working)
- [ ] All bot commands functional
- [ ] Mesh density calculated correctly

### Architecture
- [ ] Static MUSL binary produced
- [ ] freenet-core node runs successfully
- [ ] Signal bot authenticates and manages group
- [ ] STARK proofs < 100KB, generation < 10 seconds
- [ ] Federation infrastructure present (disabled in MVP)
- [ ] Freenet contract uses ComposableState (mergeable state)
- [ ] Set-based membership with on-demand Merkle Tree generation

### Documentation
- [ ] README.md accurate and complete
- [ ] All .cursor/rules/ updated
- [ ] Beads created for all phases
- [ ] API documentation complete
- [ ] Federation roadmap documented

---

## 📋 Outstanding Questions Status Tracking

| Question | Status | Decision | Date Resolved |
|----------|--------|----------|---------------|
| Q1: Freenet Conflict Resolution | ✅ Complete | GO — commutative deltas with set-based state + tombstones | 2026-01-29 |
| Q2: Contract Validation | ✅ Complete | GO — trustless model viable (update_state + validate_state) | 2026-01-30 |
| Q3: Cluster Detection | ✅ Complete | GO — Bridge Removal algorithm distinguishes tight clusters | 2026-01-30 |
| Q4: STARK verification in Wasm | ✅ Complete | PARTIAL — Bot-side verification for Phase 0 (Wasm experimental) | 2026-01-30 |
| Q5: Merkle Tree performance | ✅ Complete | GO — 0.09ms for 1000 members (on-demand OK) | 2026-01-30 |
| Q6: Proof storage strategy | ✅ Complete | Store outcomes only (not proofs) | 2026-01-30 |

**✅ SPIKE WEEK COMPLETE — ALL QUESTIONS ANSWERED**

Proceed to Phase 0 implementation with:
- Trustless contract validation (Q2)
- Bridge Removal for cluster detection (Q3)
- Bot-side STARK verification (Q4, upgrade later)
- On-demand Merkle generation (Q5)
- Store outcomes only (Q6)
