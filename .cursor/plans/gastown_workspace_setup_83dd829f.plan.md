---
name: Gastown Workspace Setup & Stroma Implementation Roadmap
overview: Gastown workspace ready for agent implementation. All architectural decisions finalized, constraint beads created, landing zone prepared. Next: Agent-Signal implements protocol v8 poll support, then Phase 0 foundation modules.
todos:
  - id: install-gastown
    content: Install Gastown CLI tool (gt) via Go and add to PATH
    status: completed
  - id: init-git
    content: Initialize git repository and create commits with architectural foundation
    status: completed
  - id: setup-gastown-workspace
    content: Initialize Gastown workspace (town) in current project directory
    status: completed
  - id: architectural-beads
    content: Create 7 architectural constraint beads with design decisions
    status: completed
  - id: update-docs-rules
    content: Update all rules, documentation, and beads for architectural decisions
    status: completed
  - id: protocol-v8-poll
    content: Agent-Signal implements protocol v8 poll support in forked libsignal-service-rs
    status: pending
  - id: spike-week-1
    content: Spike Week 1 - Validate core technologies (Q1-Q6 ALL COMPLETE)
    status: completed
  - id: spike-week-2
    content: Spike Week 2 - Validate persistence network (Q7-Q14 PENDING)
    status: pending
  - id: scaffold-rust-modules
    content: Create Rust module structure (kernel, freenet, signal, crypto, proposals)
    status: pending
  - id: create-phase-0-beads
    content: Create Phase 0 (Foundation) Bead issues for implementation
    status: pending
  - id: launch-phase-0
    content: Launch Phase 0 convoy with specialized agents
    status: pending
isProject: false
---

# Gastown Workspace Setup & Stroma Implementation Roadmap

## Project Vision

**Big Picture**: Build a privacy-first, decentralized trust network that scales via federation.

**Technology Stack**:
- **Rust 1.93+**: Static MUSL binary, improved musl networking
- **Signal**: User interface via Presage (high-level Rust API)
- **Freenet (Dark)**: Decentralized state with ComposableState
- **STARKs**: Zero-knowledge proofs (no trusted setup, post-quantum)

**Architecture Priorities**:
1. **Privacy First**: Anonymous voting, no graph exposure
2. **1:1 Bot-to-Group**: One bot per group (simplicity over efficiency)
3. **Non-Hierarchical**: Bot executes only, never decides
4. **Scalability**: 10²-10³ scaling via recursive ZK-proofs

---

## Current State (2026-01-31)

### ✅ Completed

**Gastown Infrastructure:**
- Gastown workspace initialized (`.beads/`, `mayor/`, `deacon/`)
- Git repository with 5 commits on `signal-bot` branch
- Gastown-specific `.gitignore` configured
- `AGENTS.md` with workflow instructions

**Architectural Foundation:**
- ✅ **7 Architectural Constraint Beads** created:
  1. `bot-deployment-model.bead` - 1:1 bot-to-group relationship
  2. `group-identity.bead` - Group name required
  3. `governance-model.bead` - Bot as execute-only agent
  4. `proposal-system.bead` - `/propose` command structure
  5. `technology-stack.bead` - Presage over libsignal-service-rs
  6. `voting-mechanism.bead` - Native Signal Polls for anonymity
  7. `poll-implementation-gastown.bead` - Agent-Signal task (protocol v8)

**Documentation & Rules:**
- ✅ 6 rules files updated with architectural decisions
- ✅ 4 documentation files updated
- ✅ All planning documents removed (were temporary notes)
- ✅ Complete UX specification in `.cursor/rules/user-roles-ux.mdc`
- ✅ Security guardrails comprehensive

**Provisioning Tools:**
- ✅ Fish shell script for Signal bot provisioning (`utils/provision-signal-cli.fish`)
- ✅ Two-phase workflow (bypasses macOS TTY buffer limit)

### 📋 Remaining Tasks

**Immediate (Before Phase 0):**
1. **Protocol v8 Poll Support** (Agent-Signal task)
   - Fork libsignal-service-rs
   - Add poll protobuf definitions (fields 24-26)
   - Submit PR to Whisperfish
   - Use fork in Stroma immediately
   - Timeline: 1-2 weeks

2. **Spike Week** (Technology Validation)
   - Validate freenet-core with ComposableState
   - Test Presage group management and polls
   - Benchmark STARK proof generation
   - Answer outstanding technical questions
   - Timeline: 1 week

**Foundation Work:**
3. Scaffold Rust module structure
4. Create Phase 0 Bead issues
5. Launch Phase 0 convoy (parallel agent development)

---

## Architectural Decisions (Finalized 2026-01-28)

### Decision 1: 1:1 Bot-to-Group Relationship

**Architecture**: One bot process per Stroma group

```
1 Bot Instance = 1 Signal Group = 1 Freenet Contract = 1 Trust Mesh
```

**Multi-Group Deployment:**
- Use systemd service templates: `stroma-bot@<group-name>.service`
- Each group has separate config, phone number, state
- Scale: <100 groups = <100 processes (acceptable)

**Why**: Simplicity, isolation, clear identity, easier debugging

**See**: `.beads/bot-deployment-model.bead`

### Decision 2: Group Names Required

**Constraint**: Every Stroma group MUST have human-readable name

**Specified at**: Seed initialization (`--group-name "Mission Control"`)

**Usage**:
- Signal group name
- Bot invitations: "You've been invited to '{name}' on Stroma"
- Federation: "Federate '{our_name}' with '{their_name}'"
- Changeable via consensus: `/propose config name "New Name"`

**See**: `.beads/group-identity.bead`

### Decision 3: Non-Hierarchical Governance

**Principle**: Bot is Signal admin (technical) but execute-only (no decision power)

**Authority Model:**
```
Signal Admin Powers = Technical Capability (bot has)
Freenet Contract = Decision Authority (group has)
Operator = Service Runner (NO privileges)
```

**Bot Governance:**
- Bot MUST verify Freenet contract before EVERY action
- Bot NEVER makes autonomous decisions
- All membership changes require contract approval
- All config changes require group consensus

**Operator Role:**
- Start/stop service (systemd)
- Monitor logs
- NO manual commands
- NO override capability

**See**: `.beads/governance-model.bead`

### Decision 4: `/propose` Command System

**Unified interface** for all group decisions:

```
/propose <subcommand> [args] [--timeout duration]
```

**Subcommands:**
- `config <setting> <value>` - Signal group settings
- `stroma <setting> <value>` - Stroma trust config
- `federate <group-id>` - Federation proposal

**Configuration:**
- Timeout: Per-proposal configurable (default from GroupConfig)
- Threshold: From GroupConfig.config_change_threshold (e.g., 70%)

**NO Ejection Appeals:**
- Removed from system
- Re-entry via securing 2 new vouches + re-invite
- Prevents contentious public debates

**See**: `.beads/proposal-system.bead`

### Decision 5: Presage (Not signal-cli)

**Technology Stack:**
```
libsignal-service-rs (low-level Rust)
        ↓
Presage (high-level Rust convenience API)
        ↓
Stroma Bot
```

**Primary**: Use Presage for Signal integration
**Secondary**: Drop to libsignal-service-rs when Presage insufficient
**NOT Used**: signal-cli (separate Java tool, not in Stroma codebase)

**See**: `.beads/technology-stack.bead`

### Decision 6: Native Signal Polls (Anonymous Voting)

**CRITICAL**: Polls preserve voter anonymity (reactions expose voters)

**Why Polls:**
- ✅ Anonymous voting (Signal doesn't expose who voted what)
- ✅ Multiple choice options (better decision making)
- ✅ Native Signal UX (familiar interface)
- ❌ Reactions are public (everyone sees who reacted)

**Implementation Strategy:**
- Fork libsignal-service-rs
- Add protocol v8 poll support (fields 24-26 in DataMessage)
- Use fork immediately in Stroma
- Submit PR to Whisperfish (don't wait for merge)
- Migrate to upstream when merged

**See**: `.beads/voting-mechanism.bead`, `.beads/poll-implementation-gastown.bead`

---

## Technology Decisions (Finalized)

### Core Technologies

| Component | Technology | Version | Rationale |
|-----------|-----------|---------|-----------|
| **Rust** | Latest stable | 1.93+ | Musl 1.2.5 networking improvements |
| **State Storage** | Freenet Dark | v0.1.107+ | ComposableState, summary-delta sync |
| **Signal** | Presage | Latest git | High-level Rust API, wraps libsignal-service-rs |
| **Signal (low-level)** | libsignal-service-rs (FORK) | Protocol v8 | Poll support via our fork |
| **ZK-Proofs** | STARKs (winterfell) | 0.9+ | No trusted setup, post-quantum |
| **Identity** | HMAC-SHA256 (ring) | 0.17+ | Group-scoped hashing with pepper |
| **Memory** | zeroize | 1.7+ | Immediate buffer purging |
| **Binary** | MUSL | x86_64 | Static linking, minimal attack surface |

### Key Architectural Patterns

**1. ContractInterface for Freenet Contracts:**
```rust
// Note: freenet-scaffold is outdated. Use freenet-stdlib.
use freenet_stdlib::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TrustNetworkState {
    members: BTreeSet<Hash>,              // Naturally mergeable
    ejected: BTreeSet<Hash>,              // Can return (not permanent)
    vouches: HashMap<Hash, HashSet<Hash>>,  // Naturally mergeable
    flags: HashMap<Hash, HashSet<Hash>>,
    config: GroupConfig,
}

// Merkle Trees generated on-demand (not stored in contract)
impl TrustNetworkState {
    pub fn generate_merkle_tree(&self) -> MerkleTree {
        MerkleTree::from_leaves(self.members.iter())
    }
}
```

**2. Real-Time State Stream (Not Polling):**
```rust
async fn monitor_state_changes(freenet: &FreenetClient, signal: &SignalClient) {
    let mut stream = freenet.subscribe_to_state_changes().await?;
    
    while let Some(change) = stream.next().await {
        match change {
            StateChange::MemberVetted(hash) => signal.add_member(hash).await?,
            StateChange::MemberRevoked(hash) => signal.remove_member(hash).await?,
            // ...
        }
    }
}
```

**3. Anonymous Voting via Signal Polls:**
```rust
use presage::libsignal_service::proto::DataMessage;
use presage::libsignal_service::proto::data_message::PollCreate;

// Create proposal poll
let poll = DataMessage {
    poll_create: Some(PollCreate {
        question: Some("Approve config change?"),
        options: vec!["👍 Approve".to_string(), "👎 Reject".to_string()],
        allow_multiple: Some(false),
    }),
    ..Default::default()
};

// Bot sees only aggregates (approve_count, reject_count)
// Never sees individual votes (preserves anonymity)
```

---

## Implementation Roadmap (Updated)

### **Phase -1: Protocol v8 Poll Support (PRIORITY)**

**Duration**: 1-2 weeks  
**Assigned**: Agent-Signal  
**Bead**: `.beads/poll-implementation-gastown.bead`

**Tasks:**
1. Fork libsignal-service-rs on GitHub
2. Create branch: `feature/protocol-v8-polls`
3. Copy poll protobuf definitions from Signal-Desktop
4. Update protocol version to v8
5. Build and test
6. Push to fork
7. Update Stroma's Cargo.toml to use fork
8. Submit PR to Whisperfish upstream

**Deliverable**: Poll support available in Stroma immediately

**Critical Path**: This MUST be done before proposal system implementation

### **Phase 0: Spike Week (VALIDATION)**

**Duration**: 1 week  
**Purpose**: Validate core technologies before full implementation

**Spike 1: Freenet + ComposableState (2 days)**
- Run freenet-core node locally
- Implement ComposableState trait for test contract
- Test set-based membership merging
- Benchmark on-demand Merkle Tree generation
- Test state stream monitoring (real-time updates)

**Outstanding Questions (Spike Week 1 — ALL COMPLETE):**
- ✅ Q1: Freenet merge conflicts — GO (commutative deltas with set-based state + tombstones)
- ✅ Q2: Contract validation — GO (update_state + validate_state enforce invariants)
- ✅ Q3: Cluster detection — GO (Bridge Removal algorithm, Tarjan's)
- ✅ Q4: STARK verification — PARTIAL (bot-side for Phase 0, Wasm experimental)
- ✅ Q5: Merkle Tree performance — GO (0.09ms @ 1000 members)
- ✅ Q6: Proof storage — Store outcomes only (not proofs)

**Outstanding Questions (Spike Week 2 — PENDING):**
- ⏳ Q7-Q14: Persistence network validation (see SPIKE-WEEK-2-BRIEFING.md)

**Spike 2: Presage + Polls (2 days)**
- Test Presage group management
- Verify poll support in forked libsignal-service-rs
- Test poll creation and vote counting
- Verify vote anonymity (no individual votes exposed)

**Spike 3: STARK Proofs (1 day)**
- Generate sample STARK proof with winterfell
- Circuit: "2 vouches from different members verified"
- Measure proof size (target: < 100KB)
- Measure generation time (target: < 10 seconds)

**Deliverable**: Go/No-Go decision with validated tech stack

### **Phase 1: Foundation (Weeks 1-2)**

**Modules to Implement:**

```
src/
├── kernel/                   # Identity Masking
│   ├── hmac.rs               # HMAC-SHA256 with ACI-derived key (replaces group pepper)
│   └── zeroize_helpers.rs    # Immediate buffer purging
├── freenet/                  # Freenet Integration  
│   ├── node.rs               # Node lifecycle
│   ├── contract.rs           # Contract deployment
│   └── state_stream.rs       # Real-time monitoring
├── signal/                   # Signal Integration (Presage)
│   ├── bot.rs                # Bot authentication
│   ├── group.rs              # Group management
│   └── pm.rs                 # 1-on-1 PM handling
├── crypto/                   # ZK-Proofs
│   ├── stark_circuit.rs      # STARK circuit design
│   ├── proof_gen.rs          # Proof generation
│   └── proof_verify.rs       # Proof verification
└── proposals/                # Proposal System
    ├── command.rs            # /propose parser
    ├── poll.rs               # Poll creation/monitoring
    └── executor.rs           # Execute approved actions
```

**Bead Issues** (create with `bd create`):
- Bead-01-Kernel: HMAC identity masking
- Bead-02-Freenet: Node and state stream
- Bead-03-Signal: Presage integration and bot commands
- Bead-04-Crypto: STARK circuits
- Bead-05-Contract: ComposableState schema with GroupConfig

### **Phase 2: Core Trust Operations (Weeks 3-4)**

**Features:**
- Bootstrap seed group (3 members, group name required)
- Invitation & vetting flow
- Admission (after 2 effective vouches, ZK-proof verified)
- Ejection (two triggers: Standing < 0 OR Effective_Vouches < 2)
- Vouch invalidation: voucher-flaggers excluded from both counts
- Basic commands: `/invite`, `/vouch`, `/flag`, `/status`
- Health monitoring (real-time state stream, NOT polling)

**Bead Issues**:
- Bead-06-Bootstrap: Seed group initialization
- Bead-07-Vetting: Invitation and matching
- Bead-08-Admission: ZK-proof verification
- Bead-09-Ejection: Two-trigger enforcement
- Bead-10-Health: Continuous monitoring

### **Phase 3: Proposals & Mesh (Weeks 5-6)**

**Features:**
- `/propose` command (config, stroma, federate)
- Anonymous voting via Signal Polls
- Timeout enforcement
- Automatic execution of approved proposals
- Mesh topology analysis
- Strategic introductions (Blind Matchmaker)
- Advanced commands: `/mesh`, `/mesh strength`, `/mesh config`

**Bead Issues**:
- Bead-11-Proposals: Command parsing and poll creation
- Bead-12-Voting: Poll monitoring and result counting
- Bead-13-Graph: Topology analysis
- Bead-14-Matchmaker: Strategic introductions
- Bead-15-Config: Configuration management

### **Phase 4: Federation Prep (Week 7)**

**Features** (compute but don't broadcast):
- Social Anchor hashing
- PSI-CA implementation (test with mock data)
- Federation hooks validation
- Documentation for Phase 5+

**Bead Issues**:
- Bead-16-SocialAnchor: Validator hashing
- Bead-17-PSI: Overlap calculation
- Bead-18-FederationHooks: Contract validation
- Bead-19-Docs: Federation documentation

---

## Agent Structure

### Specialized Agents

**Agent-Signal** (Priority)
- **Current Task**: Implement protocol v8 poll support
- **Responsibilities**: 
  - Fork libsignal-service-rs
  - Presage integration
  - Bot commands and PM handling
  - Group management
  - Poll creation and monitoring
- **Constraints**: Never log Signal IDs

**Agent-Freenet**
- **Responsibilities**:
  - Freenet node lifecycle
  - ComposableState implementation
  - State stream monitoring
  - Contract deployment
- **Constraints**: Freenet is source of truth

**Agent-Crypto**
- **Responsibilities**:
  - HMAC identity masking
  - STARK circuit design
  - Proof generation/verification
  - Memory zeroization
- **Constraints**: Immediate zeroization, no cleartext IDs

**Witness Agent**
- **Responsibilities**:
  - Security audit of all agents
  - Verify no Signal ID logging
  - Check zeroization compliance
  - Block unsafe patterns
- **Reads**: All constraint beads before each review

### Coordination Strategy

**Use Gastown Convoy** for parallel development:
```bash
gt convoy \
  --agents "Agent-Signal,Agent-Freenet,Agent-Crypto" \
  --witness "Witness-Agent" \
  --beads ".beads/*.bead"
```

---

## Immediate Next Actions

### 1. **Agent-Signal: Implement Protocol v8 Polls**

**Bead**: `.beads/poll-implementation-gastown.bead`

**Steps:**
```fish
# Fork on GitHub
gh repo fork whisperfish/libsignal-service-rs --clone=true

# Create feature branch
cd libsignal-service-rs
git checkout -b feature/protocol-v8-polls

# Copy poll definitions from Signal-Desktop
# Update protobuf/SignalService.proto with fields 24-26

# Build and test
cargo build
cargo test

# Push and use in Stroma
git push origin feature/protocol-v8-polls

# Update Stroma's Cargo.toml
[patch.crates-io]
libsignal-service = {
    git = "https://github.com/roder/libsignal-service-rs",
    branch = "feature/protocol-v8-polls"
}

# Submit PR to upstream
gh pr create --repo whisperfish/libsignal-service-rs \
  --title "feat: Add Signal protocol v8 poll support"
```

**Timeline**: 1-2 weeks  
**Blocking**: Proposal system (Phase 3)

### 2. **Run Spike Week (Technology Validation)**

**Duration**: 5 days

**Validation Checklist:**
- [ ] freenet-core runs successfully
- [ ] ComposableState trait works with set-based state
- [ ] On-demand Merkle Tree generation is performant
- [ ] Presage can create/manage groups
- [ ] Poll support works in forked libsignal-service-rs
- [ ] STARK proofs generate in < 10 seconds
- [ ] Proof size < 100KB

**Deliverable**: Go/No-Go report

### 3. **Scaffold Rust Modules**

**Create module structure:**
```fish
mkdir -p src/{kernel,freenet,signal,crypto,proposals,gatekeeper,matchmaker,config}

# Create mod.rs files
touch src/kernel/mod.rs
touch src/freenet/mod.rs
touch src/signal/mod.rs
touch src/crypto/mod.rs
touch src/proposals/mod.rs
# ... etc
```

**Update `Cargo.toml`** with dependencies:
```toml
[dependencies]
# Presage (high-level Signal)
presage = { git = "https://github.com/whisperfish/presage" }
# ❌ DO NOT ADD: presage-store-sqlite (stores message history - server seizure risk)
# Implement custom StromaProtocolStore instead (see security-constraints.bead § 10)

# Freenet (node embedding + contracts)
# Note: freenet-scaffold is outdated. Use freenet + freenet-stdlib.
freenet = "0.1"
freenet-stdlib = { version = "0.1", features = ["contract", "net"] }

# Crypto
ring = "0.17"
zeroize = { version = "1.7", features = ["derive"] }
winterfell = "0.9"

# Async runtime
tokio = { version = "1.35", features = ["full"] }

# Serialization (CBOR for Freenet contracts)
serde = { version = "1.0", features = ["derive"] }

[patch.crates-io]
# Use forked libsignal-service with poll support
libsignal-service = {
    git = "https://github.com/roder/libsignal-service-rs",
    branch = "feature/protocol-v8-polls"
}

# Required for Presage
curve25519-dalek = {
    git = 'https://github.com/signalapp/curve25519-dalek',
    tag = 'signal-curve25519-4.1.3'
}
```

### 4. **Create Phase 0 Bead Issues**

**Use `bd create`** for each foundation module:

```fish
# Bead-01: Kernel
bd create --title "Implement HMAC identity masking with zeroization"

# Bead-02: Freenet  
bd create --title "Integrate Freenet node and state monitoring"

# Bead-03: Signal
bd create --title "Implement Presage integration and bot commands"

# Bead-04: Crypto
bd create --title "Implement STARK circuits for vouch verification"

# Bead-05: Contract
bd create --title "Design contract schema with ComposableState and GroupConfig"
```

### 5. **Launch Phase 0 Convoy**

**Prerequisites:**
- ✅ Architectural beads created
- ✅ Poll support implemented (Agent-Signal)
- ✅ Spike week validated
- ✅ Module structure scaffolded
- ✅ Phase 0 Bead issues created

**Launch Command:**
```bash
gt convoy start \
  --phase "Phase 0: Foundation" \
  --beads "Bead-01,Bead-02,Bead-03,Bead-04,Bead-05" \
  --agents "Agent-Signal,Agent-Freenet,Agent-Crypto" \
  --witness "Witness-Agent"
```

---

## Constraint Beads (Gastown Landing Zone)

All architectural constraints captured in beads for agent guidance:

### Architectural Constraints (Immutable)
- `.beads/bot-deployment-model.bead` - 1:1 bot-to-group
- `.beads/group-identity.bead` - Group names required
- `.beads/governance-model.bead` - Execute-only bot
- `.beads/proposal-system.bead` - `/propose` structure
- `.beads/technology-stack.bead` - Presage over libsignal-service-rs
- `.beads/voting-mechanism.bead` - Native polls for anonymity
- `.beads/security-constraints.bead` - Existing security rules

### Implementation Tasks (Gastown)
- `.beads/poll-implementation-gastown.bead` - Agent-Signal task
- `.beads/federation-roadmap.bead` - North star (existing)

### Rules (Always Applied)
- `.cursor/rules/core-standards.mdc`
- `.cursor/rules/security-guardrails.mdc`
- `.cursor/rules/architecture-objectives.mdc`
- `.cursor/rules/tech-stack.mdc`
- + 9 more module-specific rules

**Agents MUST read beads before implementation.**

---

## Success Criteria

### Architectural Foundation (Completed ✅)
- [x] 7 constraint beads created
- [x] All rules updated
- [x] Documentation consistent
- [x] Gastown landing zone ready

### Phase -1: Poll Support
- [ ] libsignal-service-rs forked
- [ ] Protocol v8 implemented
- [ ] PR submitted to upstream
- [ ] Stroma uses fork successfully

### Phase 0: Foundation
- [ ] Spike week completed (Go/No-Go)
- [ ] Module structure scaffolded
- [ ] 5 Bead issues created
- [ ] Foundation modules implemented

### Phase 1: Core Trust
- [ ] Seed group bootstrap works
- [ ] Admission after 2 vouches
- [ ] Ejection on both triggers
- [ ] All vetting in 1-on-1 PMs

### Phase 2: Proposals & Mesh
- [ ] `/propose` command works
- [ ] Anonymous voting via polls
- [ ] Automatic execution
- [ ] Mesh optimization

---

## Current Branch Status

**Branch**: `signal-bot`  
**Commits**: 5 commits with architectural foundation  
**Last Commit**: "Establish architectural foundation and Gastown landing zone"

**Files Ready:**
- 7 architectural constraint beads
- 6 updated rules files
- 4 updated documentation files
- 1 provisioning tool (Fish script)
- Complete security guardrails

**Next**: Agent-Signal implements protocol v8 polls per `.beads/poll-implementation-gastown.bead`

---

## References

**Core Documentation:**
- `docs/DEVELOPER-GUIDE.md` - Complete technical guide
- `docs/OPERATOR-GUIDE.md` - Deployment and multi-group setup
- `docs/TRUST-MODEL.md` - Trust standing and vouch invalidation

**Rules:**
- `.cursor/rules/architecture-objectives.mdc` - Core architecture
- `.cursor/rules/security-guardrails.mdc` - Security constraints
- `.cursor/rules/user-roles-ux.mdc` - Complete UX specification

**Beads:**
- `.beads/*.bead` - All architectural constraints
- Use `bd list` to see Bead issues when created

**External:**
- Presage: https://github.com/whisperfish/presage
- libsignal-service-rs: https://github.com/whisperfish/libsignal-service-rs
- Freenet: https://github.com/freenet/freenet-core
- Signal-Desktop (reference): https://github.com/signalapp/Signal-Desktop

---

**Status**: ✅ Ready for Gastown agent implementation  
**Next Action**: Agent-Signal implements protocol v8 poll support  
**Last Updated**: 2026-01-31
