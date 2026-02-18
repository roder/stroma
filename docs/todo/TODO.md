# Stroma Implementation Checklist

## Mayor Delegation Guide

**Project**: Stroma -- Privacy-first decentralized trust network  
**Architecture**: Rust Static Binary | Signal Protocol | Freenet (Dark) | ZK-Proofs  
**Last Updated**: 2026-02-08

### Objective: UAT-Capable Bot

The mock-layer logic (trust model, mesh optimization, persistence, proposals) is **complete** across 79 files and ~31,600 lines with 502+ tests passing. The bot cannot be tested against real Signal or Freenet networks because the integration layers are entirely stubbed. The objective is to wire the mock layer to real infrastructure so UAT can begin.

**UAT Definition**: A human operator can link the bot to a real Signal account, create a group, invite members, run the admission flow, execute proposals, and verify persistence -- all against live Signal and Freenet networks.

### Critical Path to UAT

Work must be delegated in this order. Each step depends on the one above it.

```
Step 1: Rig Presage        Wire presage fork to real Signal (E2E validation)
   |                        ✅ Polls (create/vote/terminate) UAT-validated
   |                        ⬜ Group CRUD (create_group, add/remove member) -- separate convoy
   |
Step 2: Rig Stroma         Wire LibsignalClient to presage Manager APIs
   |                        Implement StromaStore wrapper (replaces StromaProtocolStore)
   |                        Implement passphrase management (BIP-39)
   |                        Add register CLI command
   |                        Implement link_secondary_device()
   |
Step 3: Rig Stroma         Wire EmbeddedKernel to real Freenet APIs
   |                        Integrate Freenet state stream in run() loop
   |
Step 4: Rig Stroma         Fix broken admission flow paths:
   |                        - handle_reject_intro() (broken)
   |                        - handle_status() (stub)
   |                        - Eliminate divergent matchmaker methods
   |
Step 5: UAT                Bot links, creates group, runs full admission flow
                            against live Signal + Freenet
```

Steps 1-2 can be parallelized (presage E2E validation is independent of stroma wiring). Step 3 can partially overlap with Step 2. Step 4 can be done in parallel with Steps 1-3 (works against mocks).

### Status Summary

| Area | Status | Blocks UAT? |
|------|--------|-------------|
| **Mock-Layer Logic** | COMPLETE (502+ tests) | No |
| **Presage Fork E2E** | ✅ Polls UAT-validated. ✅ Group create/add-member functionally tested via bootstrap flow. | **No** -- core group ops working |
| **LibsignalClient** | Partial -- send_message, send_group_message, create_poll, terminate_poll, receive_messages wired to presage. create_group, add/remove_member working via bootstrap.rs. | **Partial** -- Step 2 |
| **StromaStore** | ✅ DONE -- wraps encrypted SqliteStore with SQLCipher, no-ops message persistence | No |
| **Passphrase Management** | ✅ DONE -- BIP-39 24-word, --passphrase-file delivery | No |
| **Register CLI** | ✅ DONE -- `stroma register` command | No |
| **link_secondary_device** | ✅ DONE -- working, QR code linking tested | No |
| **Bootstrap Flow** | ✅ FUNCTIONALLY TESTED -- /create-group creates Signal group, /add-seed adds members, triangle vouching initializes trust state | No |
| **EmbeddedKernel** | Custom mock (HashMap fallback), not real Freenet. Blocked on upstream visibility fix (st-ko618). | **Yes** -- Step 3 |
| **Freenet state stream** | Commented out in run() | **Yes** -- Step 3 |
| **handle_reject_intro** | BROKEN (blank state, no-op) | **Yes** -- Step 4 |
| **handle_status** | Hardcoded placeholder | **Yes** -- Step 4 |
| **CI/CD Coverage** | Disabled (87%) | No (post-UAT) |
| **MutualAI Convergence** | ✅ Vision documented (1100+ lines). Rust CLI running (`mutualai suggest`). Beads tracking initialized. | No -- parallel track |

### Codebase Statistics

| Module | Files | Lines | Status |
|--------|-------|-------|--------|
| `src/signal/` | 16 | 7,637 | Complete (mock layer) |
| `src/persistence/` | 12 | 6,538 | Complete (mock layer) |
| `src/matchmaker/` | 6 | 4,425 | Complete |
| `src/gatekeeper/` | 5 | 2,418 | Complete |
| `src/freenet/` | 7 | 2,422 | Complete (mock layer) |
| `src/stark/` | 6 | 1,002 | Complete (simplified proofs) |
| `src/crypto/` | 2 | 838 | Complete |
| `src/federation/` | 2 | 552 | Complete |
| `src/cli/` | 7 | 531 | Complete |
| `src/identity.rs` | 1 | 332 | Complete |
| `src/serialization/` | 1 | 131 | Complete |
| `tests/` | 6 | 3,817 | 20 tests `#[ignore]` |
| `benches/` | 6 | 913 | Complete |
| **Total** | **79** | **~31,600** | |

### Security Audit: PASS

Verified in `phase2-security-audit.md`:
- No cleartext Signal IDs in logs (0 violations)
- Transient mapping correctly implemented
- GAP-02 vote privacy compliant (no individual votes persisted)

### Performance Benchmarks: ALL TARGETS MET

- DVR calculation: 0.192ms @ 1000 members (target: <1ms) -- 5.2x faster
- Cluster detection: 0.448ms @ 500 members (target: <1ms) -- 2.2x faster
- Blind Matchmaker: 0.120ms @ 500 members (target: <200ms) -- 1,667x faster

---

## Security Constraints (ALL Rigs MUST Follow)

**Canonical Source**: `.beads/security-constraints.bead`

**The Eight Absolutes (NEVER)** -- Violations block merge:
1. NEVER store Signal IDs in cleartext
2. NEVER persist message history
3. NEVER bypass ZK-proof verification
4. NEVER add grace periods for ejection
5. NEVER make Signal source of truth
6. NEVER restrict vouching to Validators only
7. NEVER commit without Co-authored-by (AI agents)
8. NEVER trust persistence peers

**The Eight Imperatives (ALWAYS)**:
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

## Completed Work Archive

All mock-layer logic, trust model, mesh optimization, and persistence modules are complete. This section provides a summary for reference. The code itself is the source of truth.

### Phase 0: Foundation -- COMPLETE

All 25 deliverables implemented. All GAP-07, GAP-08 requirements met.

| Track | Module | Key Files | Tests |
|-------|--------|-----------|-------|
| HMAC Identity Masking | `src/identity.rs` (332 lines) | `derive_identity_masking_key()`, `mask_identity()`, `SensitiveIdentityData` with ZeroizeOnDrop | 6 unit + 3 proptest |
| STARK Circuits | `src/stark/` (6 files, 1,002 lines) | `VouchAir`, `prove_vouch_claim()`, `verify_vouch_proof()` | 14 unit + 4 proptest + 10 integration |
| Freenet Integration | `src/freenet/` (7 files, 2,422 lines) | `FreenetClient` trait, `TrustNetworkState`, `EmbeddedKernel`, `StateStream` | 45+ unit + 5 proptest |
| Signal Integration | `src/signal/` (16 files, 7,637 lines) | `SignalClient` trait, `MockSignalClient`, `StromaBot`, `PollManager` | 60+ unit |
| CBOR Serialization | `src/serialization/mod.rs` (131 lines) | `to_cbor()`, `from_cbor()` | Roundtrip tests |
| CLI | `src/cli/` (7 files, 531 lines) | `stroma link-device`, `run`, `status`, `verify`, `backup-store`, `version` | 16 tests (`#[ignore]`) |

**Note on STARK proofs**: The prover in `src/stark/prover.rs` generates simplified hash-based commitments, not full winterfell FRI proofs. The verifier checks structural consistency only. Upgrading to real STARK verification is tracked as hardening work below.

### Phase 1: Bootstrap & Core Trust -- COMPLETE

All 7 GAP remediations (GAP-01 through GAP-10) closed. All integration test scenarios implemented.

| Feature | Module | Status |
|---------|--------|--------|
| Bootstrap State Machine | `src/signal/bootstrap.rs` (601 lines) | Complete -- `/create-group`, `/add-seed`, triangle vouching |
| Trust Operations | `src/signal/bot.rs` (1,666 lines), `src/signal/pm.rs` (1,763 lines) | Complete -- `/invite`, `/vouch`, `/flag`, `/status`, `/reject-intro` |
| Vetting Flow | `src/signal/vetting.rs` (549 lines), `src/signal/matchmaker.rs` (384 lines) | Complete -- VettingSessionManager, assessor selection |
| Ejection Protocol | `src/gatekeeper/ejection.rs` (512 lines) | Complete -- immediate, no grace periods, retry with backoff |
| Health Monitoring | `src/gatekeeper/health_monitor.rs` (784 lines) | Complete -- real-time Freenet state stream |
| Rate Limiting (GAP-03) | `src/gatekeeper/rate_limiter.rs` (614 lines) | Complete -- 5-tier progressive cooldown, 16 tests |
| Audit Trail (GAP-01) | `src/gatekeeper/audit_trail.rs` (490 lines) | Complete -- immutable append-only log, 12 tests |
| Member Resolution | `src/signal/member_resolver.rs` (351 lines) | Complete -- ephemeral hash-to-name mapping |

### Phase 2: Mesh Optimization -- COMPLETE (mock layer)

All deliverables implemented. GAP-11 (cluster announcement) closed. 4 integration tests still `#[ignore]`.

| Feature | Module | Status |
|---------|--------|--------|
| DVR Calculation | `src/matchmaker/dvr.rs` (586 lines) | Complete -- 9 unit + 3 proptest |
| Graph Analysis | `src/matchmaker/graph_analysis.rs` (1,249 lines) | Complete -- Tarjan's bridges, Union-Find, 15 unit + 8 proptest |
| Cluster Detection | `src/matchmaker/cluster_detection.rs` (1,166 lines) | Complete -- Bridge Removal, GAP-11 announcement, 12 unit + 7 proptest |
| Strategic Introductions | `src/matchmaker/strategic_intro.rs` (1,008 lines) | Complete -- 3-phase algorithm (DVR / MST / cluster bridging), 13 unit + 5 proptest |
| Display | `src/matchmaker/display.rs` (388 lines) | Complete -- transient name resolution, 12 unit |
| Proposal Command | `src/signal/proposals/command.rs` (346 lines) | Complete -- `/propose config`, `/propose stroma`, timeout validation |
| Proposal Lifecycle | `src/signal/proposals/lifecycle.rs` (382 lines) | Complete -- create, monitor via state stream, terminate, execute |
| Proposal Executor | `src/signal/proposals/executor.rs` (412 lines) | Complete -- config changes with audit entries (federation deferred) |
| Poll Manager | `src/signal/polls.rs` (348 lines) | Complete -- create, vote, terminate, outcome check |

**Note**: `graph_analysis.rs` and `cluster_detection.rs` both implement bridge removal. Consider consolidating.

### Phase 2.5: Persistence -- COMPLETE

All 7 critical gaps resolved. 16/16 proptests passing.

| Feature | Module | Status |
|---------|--------|--------|
| Replication Health | `src/persistence/health.rs` (604 lines) | Complete -- 4-state model, 14 unit tests |
| Attestation | `src/persistence/attestation.rs` (806 lines) | Complete -- HMAC-SHA256 receipts, 5 proptest |
| Chunk Distribution | `src/persistence/distribution.rs` (768 lines) | Complete -- parallel distribution, attestation verification |
| Encryption | `src/persistence/encryption.rs` (702 lines) | Complete -- AES-256-GCM, version chain, Merkle root, 10 unit |
| Registry | `src/persistence/registry.rs` (625 lines) | Complete -- BTreeSet, tombstones, epoch tracking, 10 unit |
| Write-Blocking | `src/persistence/write_blocking.rs` (621 lines) | Complete -- 4-state machine (Provisional/Active/Degraded/Isolated), 12 unit |
| Chunking | `src/persistence/chunks.rs` (488 lines) | Complete -- 64KB chunks, AES-256-GCM, 8 unit |
| Rendezvous Hashing | `src/persistence/rendezvous.rs` (419 lines) | Complete -- deterministic holder selection, 10 unit |
| Chunk Storage | `src/persistence/chunk_storage.rs` (327 lines) | Complete -- contract-based storage, 5 unit |
| Recovery | `src/persistence/recovery.rs` (711 lines) | Complete -- full crash recovery, 3 integration |
| Property Tests | `src/persistence/proptests.rs` (432 lines) | Complete -- 16 proptests (encryption 8, chunking 3, rendezvous 5) |

---

## Remaining Work: Rig Stroma (Steps 2-4)

### Step 2: Real Signal Integration

**Beads**: `.beads/signal-integration.bead` (trait, store, linking roadmap), `.beads/technology-stack.bead` (presage/libsignal crate usage), `.beads/operator-cli.bead` (CLI commands)

**Problem**: `src/signal/client.rs` (`LibsignalClient`) has all 8 async methods returning `Err(SignalError::NotImplemented)`. The entire bot operates against `MockSignalClient` only.
**Depends on**: Step 1 (Presage E2E validation).

**Deliverables**:

- [X] Create `StromaStore` wrapper (`src/signal/stroma_store.rs`) around encrypted `SqliteStore`:
  - Wraps `presage-store-sqlite::SqliteStore` with SQLCipher encryption
  - No-ops message/sticker persistence (server seizure protection)
  - Persists protocol state, groups, profiles (restart recovery)
  - Supersedes `StromaProtocolStore` (which never implemented presage's `Store` trait)
- [X] Remove old `StromaProtocolStore` (`src/signal/store.rs`) and update imports across codebase
- [X] Implement passphrase management:
  - 24-word BIP-39 recovery phrase generated at link/register time
  - Delivery via `--passphrase-file` (container-native), stdin prompt, or env var (fallback)
- [X] Add `stroma register` CLI command (`src/cli/register.rs`) for new phone number registration
- [X] Wire `LibsignalClient` to presage `Manager` APIs:
  - `send_message()` -> `manager.send_message()`
  - `send_group_message()` -> `manager.send_message_to_group()`
  - `create_group()` -> `manager.create_group()` (blocked on GV2 Group CRUD convoy)
  - `add_group_member()` / `remove_group_member()` -> (blocked on GV2 Group CRUD convoy)
  - `create_poll()` -> `manager.send_poll()` (API exists in presage fork)
  - `terminate_poll()` -> `manager.terminate_poll()` (API exists in presage fork)
  - `receive_messages()` -> `manager.receive_messages()`
- [X] Implement vote aggregate + HMAC'd voter dedup persistence in PollManager (zeroize on outcome)
- [X] Implement `link_secondary_device()` in `src/signal/linking.rs` (currently stubbed)
- [X] Lift `presage-store-sqlite` ban in `deny.toml` and `security.yml` (REQUIRES HUMAN APPROVAL)
- [X] Enable 16 CLI integration tests in `tests/cli_integration.rs` (all `#[ignore]` with reason "presage dependency")
- [X] End-to-end manual test: bot registers/links, receives messages, creates group, restarts without losing state

**Testing Strategy**:

| Level | Tool | Purpose |
|-------|------|---------|
| Unit | `MockSignalClient` | All logic tests (existing, passing) |
| Integration | `MockSignalClient` + recording proxy | Verify message flow |
| E2E | Real Signal account (manual) | Device linking, group creation, polls |

### Step 3: Real Freenet Integration

**Beads**: `.beads/freenet-integration.bead` (embedded node, state stream, entry points), `.beads/freenet-contract-design.bead` (contract schema, delta commutativity)

**Problem**: `src/freenet/embedded_kernel.rs` uses a custom in-memory `HashMap` mock, not the actual Freenet `Executor` or `NodeConfig` APIs. The `freenet` and `freenet-stdlib` crate dependencies compile but are never called.
**Depends on**: Can start in parallel with Step 2.

**Deliverables**:

- [ ] Wire `EmbeddedKernel` to actual Freenet APIs:
  - Replace custom mock with `Executor::new_mock_in_memory()` for integration tests
  - Implement `NodeConfig::build()` path for production
  - Wire `subscribe()` to real Freenet state change events (currently returns `stream::empty()`)
- [ ] Integrate Freenet state stream in `StromaBot::run()` (TODO noted in `src/signal/bot.rs`)
- [ ] Test with real Freenet node (UAT)

**Testing Strategy**:

| Level | Tool | Purpose |
|-------|------|---------|
| Unit | `MockFreenetClient` | All logic tests (existing, passing) |
| Integration | `Executor::new_mock_in_memory()` | In-process contract execution |
| UAT | Real Freenet node | Dark mode, actual network |

### Step 4: Admission Flow Completion

**Beads**: `.beads/vetting-protocols.bead` (full admission lifecycle, assessor selection, ejection triggers), `.beads/blind-matchmaker-dvr.bead` (DVR-optimized selection algorithm, exclusion list, re-selection), `.beads/cross-cluster-requirement.bead` (cross-cluster enforcement, single-cluster exception)

**Problem**: The admission vetting flow (`/invite` -> assessor selection -> `/vouch` -> admission) is architecturally complete but has critical implementation gaps in the re-selection path and run loop integration. See `privacy-first_admission_flow_90400c89.plan.md` for the full design.
**Depends on**: Works against mock layer now (can parallelize with Steps 1-3). Must be done before UAT.

**Audit Summary** (2026-02-08):

| Component | File | Status | Gap |
|-----------|------|--------|-----|
| `select_validator()` | `signal/matchmaker.rs` | Working | Uses real TrustGraph + DVR-optimal selection |
| `select_validator_with_exclusions()` | `signal/matchmaker.rs` | Phase 0 stub | No TrustGraph, no DVR, takes `first()` -- divergent from `select_validator()` |
| `VettingSession` / `VettingSessionManager` | `signal/vetting.rs` | Complete | `excluded_candidates`, `Stalled` variant, all methods, privacy-safe messages |
| `MemberResolver` | `signal/member_resolver.rs` | Complete | Bidirectional, zeroizing, transient |
| `/reject-intro` parsing | `signal/pm.rs` | Complete | Parsed and routed to bot handler |
| `handle_invite()` | `signal/bot.rs` | ~90% | Invitee ServiceId is string placeholder; excluded set always empty on first call |
| `handle_vouch()` | `signal/bot.rs` | Complete | Full flow: cross-cluster check, STARK proof, admission, session cleanup |
| `handle_reject_intro()` | `signal/bot.rs` | ~50% BROKEN | Uses blank `TrustNetworkState::new()`, Phase 0 matcher, never assigns/PMs new validator |
| `run()` loop | `signal/bot.rs` | Partial | Signal-only; Freenet state stream commented out as TODO |
| `handle_status()` | `signal/pm.rs` | Stub | Returns hardcoded placeholder text |
| `handle_propose()` | `signal/pm.rs` | Stub | Parses args but never creates poll |
| Bootstrap seed notifications | `signal/bootstrap.rs` | Partial | MemberHash->ServiceId mapping TODO for notify + add-to-group |
| `add_member()` Freenet check | `signal/group.rs` | Missing | TODO: verify member is vetted in Freenet before Signal add |

**Critical Deliverables** (functional completeness against mock layer):

- [ ] **Fix `handle_reject_intro()`** in `src/signal/bot.rs` (BROKEN):
  - Replace `TrustNetworkState::new()` with actual Freenet state query (`self.freenet.get_state()`)
  - Replace `select_validator_with_exclusions()` call with `select_validator()` (the real DVR-optimal one)
  - After finding new validator: resolve via `MemberResolver`, call `assign_assessor()`, send privacy-safe PM, update session status
  - Currently lines 714-725 are all TODOs after the match succeeds

- [ ] **Eliminate `select_validator_with_exclusions()`** in `src/signal/matchmaker.rs`:
  - Merge its exclusion-list parameter into `select_validator()` (which already accepts `excluded: &HashSet<MemberHash>`)
  - The Phase 0 approximation method is divergent (no TrustGraph, no DVR) and only creates bugs when called alongside the real version
  - Remove `are_cross_cluster()` Phase 0 approximation if no longer used after merge

- [ ] **Integrate Freenet state stream in `StromaBot::run()`** in `src/signal/bot.rs`:
  - Uncomment and implement the `tokio::select!` block at lines 89-94
  - Handle `StateChange::ProposalExpired` and membership events from Freenet in real-time

- [ ] **Fix `handle_status()`** in `src/signal/pm.rs`:
  - Query Freenet for caller's actual standing, vouchers, and flags
  - Replace hardcoded placeholder text with real data
  - Resolve MemberHash to display names via `MemberResolver` (ephemeral)

**Lower-Priority Admission Deliverables**:

- [ ] Add Freenet verification pre-check in `src/signal/group.rs::add_member()` (TODO at line 25)
- [ ] Fix bootstrap seed notification: resolve MemberHash -> ServiceId for `/add-seed` responses (`src/signal/bootstrap.rs` lines 203-205, 222-224)
- [ ] Auto-populate `MemberResolver` from Signal group roster on bot startup (method `rebuild_from_roster()` exists but is never called from `StromaBot::new()`)
- [ ] Add session timeout/expiry for `Stalled` vetting sessions (variant exists in `VettingStatus` but no timer logic)
- [ ] Remove dead stubs `handle_invite()` / `handle_vouch()` in `src/signal/pm.rs` (lines 276-318) -- these are intercepted by `StromaBot::handle_message()` and never reached

### UAT Acceptance Criteria

**Beads**: `.beads/signal-integration.bead` (Step 5 test script), `.beads/vetting-protocols.bead` (admission lifecycle), `.beads/proposal-system.bead` (proposal flow)

When Steps 1-4 are complete, UAT is ready. Verify by running through this manual test script:

1. **Link bot**: `stroma link-device --name "test-bot"` -- scan QR code with Signal app, bot links as secondary device
2. **Create group**: Send `/create-group "UAT Test Group"` as PM to bot -- bot creates Signal group
3. **Bootstrap**: Send `/add-seed @alice` and `/add-seed @bob` -- bot creates triangle vouching, Freenet contract initialized
4. **Invite**: Alice sends `/invite @carol "met at conference"` -- bot selects assessor from different cluster, PMs them
5. **Reject-intro**: Selected assessor sends `/reject-intro @carol` -- bot selects a new assessor and PMs them
6. **Vouch**: New assessor sends `/vouch @carol` -- bot verifies cross-cluster, generates STARK proof, adds carol to group
7. **Status**: Carol sends `/status` -- bot returns her real standing, vouchers, flags from Freenet
8. **Propose**: Any member sends `/propose stroma --key min_vouches --value "2" --value "3" --timeout 1h --proposal "Change min vouches?"` -- Signal poll created, timeout monitored, result executed
9. **Persistence**: Restart bot -- trust state recoverable from Freenet, Signal session resumes from encrypted `StromaStore`, group config and vote state survive restart

All steps must complete without `NotImplemented` errors. All Signal messages must flow through real Signal network. All trust state must persist in Freenet contract.

---

### Post-UAT: Multi-Option Proposal UX

**Beads**: `.beads/proposal-system.bead` (full design: command syntax, config key registry, plurality voting, data model changes, roadmap)

**Problem**: The proposal system only supports binary Approve/Reject polls (hardcoded in `src/signal/proposals/lifecycle.rs:88`). Signal Protocol v8 polls support N options, which is needed for config changes where the question is "what value?" not "should we change?"

**New Command Syntax**:

```
/propose config --key disappearing_messages --value "1 day" --value "7 days" --value "30 days" --timeout 24h --proposal "What should our disappearing message timer be?"

/propose stroma --key min_vouches --value "2" --value "3" --value "4" --timeout 48h --proposal "Adjust minimum vouch threshold"
```

**Configuration Key Registry**:

The proposal system must expose all configurable keys from both Signal's group API and Stroma's `GroupConfig`. Each key needs type validation and allowed values.

Stroma Config Keys (from `src/freenet/trust_contract.rs:70-98`):

| Key | Type | Default | Range/Values | Description |
|-----|------|---------|--------------|-------------|
| `min_vouches` | `u32` | 2 | 1-10 | Minimum vouches for full standing |
| `max_flags` | `u32` | 3 | 1-10 | Maximum flags before ejection |
| `open_membership` | `bool` | false | true/false | Whether new members can join |
| `default_poll_timeout_secs` | `u64` | 172800 (48h) | 3600-604800 (1h-168h) | Default timeout for proposals |
| `config_change_threshold` | `f32` | 0.70 | 0.50-1.00 | Fraction of votes needed to pass |
| `min_quorum` | `f32` | 0.50 | 0.25-1.00 | Fraction of members who must vote |

Signal Group Config Keys (from libsignal-service-rs `GroupV2` API):

| Key | Type | Values | Description |
|-----|------|--------|-------------|
| `disappearing_messages` | duration | off, 1d, 7d, 14d, 30d, 90d | Message expiration timer |
| `group_name` | string | 1-32 chars | Display name of the group |
| `group_description` | string | 0-480 chars | Group description |
| `add_members` | permission | all_members, only_admins | Who can add new members |
| `edit_attributes` | permission | all_members, only_admins | Who can edit group info |
| `announcements_only` | bool | true/false | Only admins can send messages |

**Deliverables**:

- [ ] `src/signal/proposals/command.rs` -- Extend parsing:
  - Support `--key` flag for config key name
  - Support `--value` flag repeated N times (N >= 2)
  - Support `--proposal` flag for custom question text
  - Validate values against `ConfigKeyRegistry`

- [ ] `src/signal/proposals/config_registry.rs` -- New module:
  - `ConfigKeyRegistry` struct with validation rules per key
  - Distinguish Signal config keys from Stroma config keys
  - Reject unknown keys with helpful error listing valid options
  - Validate value types and ranges before creating poll

- [ ] `src/signal/polls.rs` -- Extend for multi-option:
  - `VoteAggregate` changes from `approve/reject` to `votes_per_option: Vec<u32>`
  - `PollOutcome` gains `WinnerSelected` and `Tie` variants
  - `process_vote()` maps option index to corresponding value
  - `check_poll_outcome()` implements plurality voting with quorum

- [ ] `src/signal/proposals/lifecycle.rs` -- Update poll creation:
  - Generate poll options from `--value` flags (not hardcoded Approve/Reject)
  - Use `--proposal` text as poll question
  - Store option-to-value mapping for execution

- [ ] `src/signal/proposals/executor.rs` -- Update execution:
  - Map winning option back to original `--value` string
  - For Signal keys: call presage API to change Signal group setting
  - For Stroma keys: update `GroupConfig` in Freenet contract (existing logic)

**Data Model Changes**:

```rust
pub struct ProposeArgs {
    pub subcommand: ProposalSubcommand,
    pub timeout: Option<Duration>,
    pub custom_question: Option<String>,  // --proposal flag
}

pub enum ProposalSubcommand {
    Config {
        key: String,
        values: Vec<String>,  // multiple --value flags
    },
    Stroma {
        key: String,
        values: Vec<String>,
    },
}

pub struct VoteAggregate {
    pub votes_per_option: Vec<u32>,  // votes[i] = count for option i
    pub total_members: u32,
}

pub enum PollOutcome {
    WinnerSelected {
        winning_option: u32,
        winning_value: String,
        vote_count: u32,
        total_votes: u32,
    },
    Tie {
        tied_options: Vec<u32>,
    },
    QuorumNotMet {
        participation_rate: f32,
        required_quorum: f32,
    },
}
```

**UX Flow**:

```
User: /propose config --key disappearing_messages --value "1 day" --value "7 days" --value "30 days" --timeout 24h --proposal "What should our disappearing message timer be?"

Bot creates Signal poll:
  Question: "What should our disappearing message timer be?"
  Options: 1 day | 7 days | 30 days
  Timeout: 24h | Quorum: 50%

After 24h, bot terminates poll:
  Results: 1 day (3), 7 days (8), 30 days (4)
  Winner: "7 days" (plurality, 8 of 15 votes)
  Quorum: 75% (met)

  Bot announces: "Proposal PASSED: Disappearing messages set to 7 days (8 of 15 votes)"
  Bot executes: Changes Signal group disappearing_messages to 7 days
```

### Post-UAT: Test Enablement & Coverage

**Beads**: `.beads/testing-standards.bead` (100% coverage mandate, property test requirements, testing pyramid)

**Problem**: 20 integration tests are `#[ignore]`, 100% coverage enforcement is disabled in CI (currently ~87%).

**Deliverables**:

- [ ] Enable 4 Phase 2 integration tests in `tests/phase2_integration.rs`:
  - `test_scenario_1_dvr_and_cluster_detection`
  - `test_scenario_2_blind_matchmaker`
  - `test_scenario_3_proposal_lifecycle`
  - `test_scenario_4_proposal_quorum_fail`
- [ ] Fix pre-existing rendezvous proptest failure (st-7wpro)
- [ ] Restore 100% coverage enforcement in `.github/workflows/ci.yml` (currently `enforce-100: false`)
- [ ] Achieve 100% coverage (close ~13% gap)

### Post-UAT: Security Fixes

**Beads**: `.beads/security-constraints.bead` (§1 Anonymity-First: never store/log/display Signal IDs in cleartext)

- [ ] Fix `ServiceId` `Display` trait cleartext exposure (st-hhzd4) -- `ServiceId(pub String)` in `src/signal/traits.rs:11` currently exposes raw identifiers via `Display` / `Debug`

### Post-UAT: Crypto Module Reorganization

**Beads**: `.beads/rust-standards.bead` (module organization), `.beads/security-constraints.bead` (§1 HMAC-based masking pattern, ACI-derived key requirement)

**Problem**: Crypto-related code is scattered across four isolated top-level modules (`identity`, `stark`, `crypto`, `persistence/encryption`, `persistence/attestation`) with duplicate types, competing hashing strategies, and a misleading `crypto/` module name.

**Issues to resolve**:

1. **Duplicate `MemberHash` types** -- `freenet::contract::MemberHash` (private inner, ~28 import sites) and `stark::types::MemberHash` (public inner, used in `signal/bot.rs`) are two independent types for the same concept. Manual byte-level conversions in `signal/bot.rs` bridge them.

2. **~~Competing identity hashing~~** -- ✅ RESOLVED (2026-02-12): `freenet::contract::MemberHash::from_identity()` has been **removed entirely**. All callers now use `identity::mask_identity()` with the proper HMAC-SHA256 approach and mnemonic-derived keys via `StromaKeyring`.

3. **No type bridge between `MaskedIdentity` and `MemberHash`** -- `identity.rs` produces `MaskedIdentity([u8; 32])`, Freenet stores `MemberHash([u8; 32])`. No `From` impl exists; conversion is manual via raw bytes.

4. **`crypto/` name is misleading** -- contains only PSI-CA, while HMAC lives in `identity/`, STARKs in `stark/`, AES-256-GCM in `persistence/encryption`. A newcomer would look here for all crypto and find only federation-specific code.

5. **Bead documentation references "Kernel" module** -- `.beads/rust-standards.bead` and `.cursor/rules/rust-standards.mdc` reference "Kernel: Identity masking, HMAC logic" but no `src/kernel/` directory has ever existed.

**Deliverables**:

- [ ] Unify `MemberHash` into a single type shared by `freenet` and `stark` -- either extract to a shared `crate::types` module or make `stark` depend on `freenet::contract::MemberHash`
- [x] ~~Add `From<MaskedIdentity> for MemberHash` (and/or reverse) for type-safe conversion between `identity.rs` output and Freenet storage~~ -- ✅ DONE (2026-02-12): Added `From<MaskedIdentity> for freenet::contract::MemberHash` and `From<MaskedIdentity> for stark::types::MemberHash`
- [x] ~~Remove `freenet::contract::MemberHash::from_identity()` (plain SHA256 with pepper) in favor of routing through `identity::mask_identity()` (HMAC-HKDF)~~ -- ✅ DONE (2026-02-12): Method **removed entirely**, all callers migrated to `mask_identity()` + `From<MaskedIdentity>`
- [ ] Rename `src/crypto/` to `src/psi/` or move `psi_ca.rs` under `src/federation/` since PSI-CA is specifically for federation handshakes
- [ ] Update `.beads/rust-standards.bead` and `.cursor/rules/rust-standards.mdc` to reflect actual module paths (remove "Kernel" references, document actual layout)

### Post-UAT: Hardening

**Beads**: `.beads/persistence-model.bead` (version-locked distribution, fallback holders, write-blocking states), `.beads/mesh-health-metric.bead` (DVR calculation), `.beads/cryptography-zk.bead` (STARK circuit design)

- [ ] Upgrade STARK proofs from simplified hash-based commitments to real winterfell FRI verification (`src/stark/prover.rs`, `src/stark/verifier.rs`)
- [ ] Add `DistributionLock` for version-locked chunk distribution in `src/persistence/distribution.rs` -- see `persistence-model.bead` § Version-Locked Distribution
- [ ] Add `retry_failed_distribution()` with exponential backoff in `src/persistence/distribution.rs`
- [ ] Add `compute_fallback_holder()` when primary unavailable in `src/persistence/rendezvous.rs` -- see `persistence-model.bead` § Holder Redistribution on Unavailability
- [ ] Consolidate overlapping bridge removal implementations (`src/matchmaker/graph_analysis.rs` and `src/matchmaker/cluster_detection.rs`)
- [ ] Add `compute_betweenness_centrality()` (Brandes' algorithm) if needed for mesh optimization

---

## Remaining Work: Rig Presage (Step 1)

**Beads**: `.beads/technology-stack.bead` (presage fork, libsignal-service-rs fork, dependency chain), `.beads/signal-integration.bead` (Step 1 roadmap)

The presage fork at `github.com/roder/presage` (branch: `integration/protocol-v8-polls`) provides Signal protocol integration with v8 poll support. **Step 1 poll validation is complete** -- polls have been UAT-tested against live Signal production servers. Group CRUD (create_group, add/remove member) is tracked in a separate convoy.

**Fork Chain**:
```
Signal Protocol (spec)
    -> roder/libsignal-service-rs (branch: feature/protocol-v8-polls-rebased)
        -> roder/presage (branch: integration/protocol-v8-polls)
            -> Stroma Cargo.toml
```

### Completed

- [x] Protocol v8 poll support added to libsignal-service-rs fork (PollCreate, PollVote, PollTerminate, PinMessage, UnpinMessage)
- [x] Presage fork compiles with libsignal-service-rs fork
- [x] Stroma builds successfully with `presage = { git = "...", branch = "..." }` in Cargo.toml
- [x] **E2E Validation**: Poll create/vote/terminate tested against live Signal production servers
- [x] **Convenience Wrappers**: `send_poll()`, `vote_on_poll()`, `terminate_poll()` added to presage Manager
- [x] **GroupContextV2 Fix**: Poll messages now arrive in group context (was missing, caused DMs)
- [x] **CLI Consistency**: `--poll-author-uuid`, `-o` append mode, `--master-key` in all docs
- [x] **Integration branch**: 3 feature branches merged into `integration/protocol-v8-polls`

### Remaining

- [ ] **Group CRUD**: `create_group()`, `add_group_member()`, `remove_group_member()` -- separate convoy (GV2 Group CRUD)
- [ ] **Upstream PR**: Submit protocol v8 poll support PR to `whisperfish/libsignal-service-rs`
- [ ] **Upstream PR**: Submit presage convenience wrappers PR to `whisperfish/presage`
- [ ] **Fork Maintenance**: Keep branches rebased against upstream whisperfish as it evolves

---

## Remaining Work: Rig libsignal-service-rs (Post-UAT)

**Beads**: `.beads/technology-stack.bead` (fork chain, dependency management)

The libsignal-service-rs fork at `github.com/roder/libsignal-service-rs` (branch: `feature/protocol-v8-polls-rebased`) adds protocol v8 poll support fields. Not on the UAT critical path -- the fork works as-is.

### Completed

- [x] Added PollCreate, PollVote, PollTerminate, PinMessage, UnpinMessage to protobuf definitions
- [x] Tests for new message types
- [x] Rebased on post-websocket-migration commit (498c03d)

### Remaining

- [ ] **Upstream PR**: Submit PR to `whisperfish/libsignal-service-rs` for protocol v8 poll support
- [ ] **Fork Maintenance**: Keep branch rebased against upstream as it evolves
- [ ] **Documentation**: Update stale bead references that still mention old branch name `feature/protocol-v8-polls-fixed` (should be `feature/protocol-v8-polls-rebased`)

---

## Quality Gates

**Beads**: `.beads/testing-standards.bead` (100% coverage, property tests, testing pyramid), `.beads/git-standards.bead` (commit standards, Co-authored-by)

All rigs must pass these gates before merging to main.

### Pre-Push Checklist

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
cargo llvm-cov nextest --all-features   # 100% coverage required
cargo deny check                        # Supply chain security
```

### CI Pipeline (`.github/workflows/ci.yml`)

| Job | Description |
|-----|-------------|
| Format & Lint | `cargo fmt --check` + `cargo clippy` |
| Test Suite | `cargo nextest run --all-features` |
| Code Coverage | 100% enforcement (currently disabled, restoration tracked above) |
| Dependencies | `cargo-deny check` |

### If CI Fails on Main

1. File P0 bug: `bd create --title="CI BROKEN: <description>" --type=bug --priority=0`
2. Notify mayor: `gt mail send mayor/ -s "CI BROKEN"`
3. Do NOT push more code until fixed
4. Fix or revert within 15 minutes

---

## Open Issues

| Issue | Priority | Rig | Description |
|-------|----------|-----|-------------|
| handle_reject_intro BROKEN | P0 | Stroma | Uses blank state + Phase 0 matcher, never assigns/PMs new validator after re-selection |
| Divergent matchmaker methods | P0 | Stroma | `select_validator()` (real) and `select_validator_with_exclusions()` (Phase 0 stub) are inconsistent |
| Freenet state stream | P0 | Stroma | `StromaBot::run()` only polls Signal; Freenet `tokio::select!` is commented out |
| handle_status stub | P0 | Stroma | Returns hardcoded placeholder, never queries Freenet for real standing data |
| Duplicate MemberHash | P1 | Stroma | Two independent `MemberHash` types (`freenet::contract` and `stark::types`) with manual byte conversions |
| ~~Weak from_identity()~~ | ~~P1~~ | Stroma | ✅ RESOLVED (2026-02-12): Method **removed**, all callers migrated to `mask_identity()` with mnemonic-derived keys |
| st-hhzd4 | P1 | Stroma | ServiceId Display trait exposes cleartext identities |
| st-7wpro | P1 | Stroma | Pre-existing rendezvous proptest failure |
| Coverage | P1 | Stroma | 100% enforcement disabled in CI (~87% actual) |
| 16 CLI tests | P0 | Stroma | `tests/cli_integration.rs` all `#[ignore]` |
| 4 Phase 2 tests | P1 | Stroma | `tests/phase2_integration.rs` scenarios 1-4 `#[ignore]` |
| Upstream PR | P2 | libsignal | Protocol v8 polls PR to whisperfish |
| Upstream PR | P2 | Presage | Fork PR to whisperfish |