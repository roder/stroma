---
name: Gastown Workspace Setup & Implementation Roadmap
overview: Complete Gastown workspace setup with concrete technology decisions (freenet-core, STARKs) and phased MVP roadmap. MVP = Option A (single group), with federation as north star for all decisions.
todos:
  - id: install-gastown
    content: Install Gastown CLI tool (gt) via Go and add to PATH
    status: completed
  - id: init-git
    content: Initialize git repository in project directory and create initial commit
    status: in_progress
  - id: setup-gastown-workspace
    content: Initialize Gastown workspace (town) in current project directory
    status: completed
  - id: spike-week
    content: Run spike week to validate core technologies (freenet-core, Signal bot, STARKs)
    status: pending
  - id: create-constraint-beads
    content: Create immutable architectural constraint Beads with federation-ready design
    status: pending
  - id: scaffold-rust-modules
    content: Create Rust module structure (kernel, freenet, signal, crypto) with federation hooks
    status: pending
  - id: create-phase-0-beads
    content: Create Phase 0 (Foundation) Beads issues for implementation
    status: pending
  - id: prepare-mayor-brief
    content: Prepare Mayor briefing with technology decisions and security constraints
    status: pending
  - id: setup-agent-structure
    content: Define agent boundaries (Signal, Freenet, Crypto, Witness) and responsibilities
    status: pending
isProject: false
---

# Gastown Workspace Setup & Implementation Roadmap for Stroma

## Project Vision

**Big Picture**: Build federations of human networks based on trust and anonymity.

**Technology Stack**:
- **Rust**: Static MUSL binary for minimal attack surface
- **Signal**: User interface (familiar, E2E encrypted)
- **freenet-core**: Decentralized state storage (https://github.com/freenet/freenet-core)
- **STARKs**: Zero-knowledge proofs (no trusted setup, post-quantum secure)

**Architecture Priorities**:
1. **Security First**: Trust network protection above all else
2. **Scalability**: Must scale orders of magnitude (10²-10³)
3. **Anonymity**: Privacy via cryptography, not trust in authority
4. **Federation as North Star**: All decisions optimize for future federation

**MVP Scope**: Option A (Single Group)
- Bootstrap seed group (3 members)
- Invitation & vetting flow
- Vouching & flagging
- Ejection enforcement (two triggers)
- Basic commands (`/invite`, `/vouch`, `/flag`, `/status`, `/mesh`)
- **NO federation in MVP** (but infrastructure must be federation-ready)

## Current State (Updated)

✅ **Completed:**

- Gastown workspace initialized (`.beads/`, `mayor/`, `deacon/` directories exist)
- Git repository initialized (but no initial commit yet)
- Basic Rust project structure (`Cargo.toml`, `src/main.rs`)
- Gastown-specific `.gitignore` configured
- `AGENTS.md` with workflow instructions exists
- Comprehensive UX specification in `.cursor/rules/user-roles-ux.mdc`
- All architecture rules updated with clear terminology and flows

📋 **Remaining:**

- **Spike Week**: Validate freenet-core, Signal bot, STARKs (1 week)
- Initial git commit with existing files
- Immutable architectural constraint Beads (federation-ready)
- Complete Rust module structure with federation hooks
- Phase 0 (Foundation) Beads issues
- Mayor briefing with technology decisions
- Agent structure definition (Signal, Freenet, Crypto, Witness)

## Technology Decisions (Finalized)

### Core Technologies

| Component | Technology | Rationale |
|-----------|-----------|-----------|
| **Rust Version** | 1.93+ | Bundled musl 1.2.5 with improved DNS resolver for reliable networking |
| **State Storage** | [freenet-core](https://github.com/freenet/freenet-core) | Rust-native, Wasm contracts, active development (v0.1.107) |
| **ZK-Proofs** | STARKs (winterfell) | No trusted setup, post-quantum secure, transparent |
| **Identity Masking** | HMAC-SHA256 (ring) | Group-scoped hashing with pepper, zeroization |
| **Signal Integration** | libsignal-service-rs | Protocol-level group management, bot commands |
| **Memory Hygiene** | zeroize | Immediate buffer purging for sensitive data |
| **Static Binary** | MUSL (x86_64-unknown-linux-musl) | Minimal attack surface, no dynamic linking |

### Key Architectural Decisions

1. **Each bot runs its own freenet-core node** (no shared nodes)
2. **STARKs over PLONK** (no trusted setup ceremony needed)
3. **Signal limits as upper bound** (~1000 members per group)
4. **Latency acceptable**: Seconds to hours for vetting (security > speed)
5. **Bot deployment**: Operator's choice (local/VPS/container)
6. **State recovery**: Bot can go offline and recover from freenet-core state
7. **ComposableState for contracts**: Set-based membership, summary-delta sync
8. **Merkle Trees on-demand**: Generate from BTreeSet for ZK-proofs (not stored)

## Immediate Next Steps

### 0. 🔬 Spike Week (CRITICAL - Before Full Implementation)

**Purpose**: Validate core technologies before committing to architecture.

**Duration**: 5 days (1 week)

**Spike 1: freenet-core Integration & ComposableState (2 days)**
- Install and run freenet-core node locally
- Install freenet-scaffold and implement ComposableState trait
- Test set-based membership (BTreeSet) with merge semantics
- Test on-demand Merkle Tree generation (benchmark with 10-1000 members)
- Deploy contract with ComposableState to freenet-core
- Test state stream monitoring (real-time updates)
- **Answer Outstanding Questions Q1-Q5** (CRITICAL):
  - Q1: Can we verify STARK proofs in contract verify()?
  - Q2: Should we store proofs or just outcomes?
  - Q3: How expensive is on-demand Merkle Tree generation?
  - Q4: How does Freenet handle merge conflicts?
  - Q5: Can we add custom validation beyond ComposableState?
- **Risk Mitigation**: If freenet-core too immature OR ComposableState doesn't fit, evaluate alternatives

**Spike 2: Signal Bot Registration (1 day)**
- Register bot account with Signal (phone number)
- Test group management (create group, add/remove members)
- Test 1-on-1 PM handling
- Measure: Can we automate admission/ejection?
- **Risk Mitigation**: If Signal bans bots frequently, need fallback strategy

**Spike 3: STARK Proof Generation (2 days)**
- Generate sample STARK proof with `winterfell`
- Proof circuit: "2 vouches from different members verified"
- Measure proof size (target: < 100KB)
- Measure proof generation time (target: < 10 seconds)
- **Risk Mitigation**: If proofs too large/slow, consider PLONK

**Deliverable**: Go/No-Go decision report with technology validation

### 1. ✅ Install Gastown CLI (COMPLETED)

- Gastown workspace structure exists (`.beads/`, `mayor/`, `deacon/`)
- Note: `gt` command may need PATH configuration if not accessible directly

### 2. Complete Git Initialization (IN PROGRESS)

- Git repository initialized but no commits yet
- **Action Required:**
  - Stage all existing files: `git add .`
  - Create initial commit: `git commit -m "Initial commit: Gastown workspace with UX specification"`
  - Optionally set up remote: `git remote add origin <url>`

### 3. ✅ Set Up Gastown Workspace (COMPLETED)

- Gastown town initialized in current directory
- Mayor configuration exists at `mayor/CLAUDE.md`
- Deacon directory exists for build monitoring
- Beads system initialized with default formulas

### 4. Create Immutable Architectural Constraint Beads

The `.beads/` directory exists with Gastown formulas, but we need **immutable, project-specific constraint Beads** that agents must read before writing code.

**Action Required:** Create constraint documents as **immutable Beads** (pinned, cannot be modified without explicit unpinning):

#### **`.beads/security-constraints.bead`** (IMMUTABLE)
```markdown
# Security Constraints (Immutable)

## Anonymity-First Design
- ❌ NEVER store Signal IDs in cleartext
- ✅ Use HMAC-SHA256 with group-secret pepper (not deterministic hashing)
- ✅ Zeroize all sensitive buffers immediately after use
- ✅ Memory dump must contain only hashed identifiers

## Trust Model
- ❌ NEVER add grace periods for ejection
- ❌ NEVER bypass ZK-proof verification
- ✅ Immediate ejection when: Standing < 0 OR Vouches < 2
- ✅ Re-entry always possible (no cooldown)

## Zero-Knowledge Architecture
- ❌ NEVER reveal social graph structure
- ❌ NEVER expose who vouched for whom
- ✅ All trust verification via ZK-proofs (STARKs)
- ✅ Recursive proofs for scalability

## State Management
- ❌ NEVER make Signal group the source of truth
- ❌ NEVER cache trust decisions without Freenet verification
- ✅ freenet-core is authoritative state
- ✅ Signal group state is derived from Freenet
- ✅ Monitor Freenet state stream in real-time (not polling)

## Vouch Permissions
- ❌ NEVER restrict vouching to only Validators
- ✅ ANY Member can vouch (Bridges and Validators)
- ✅ Bot suggests Validators for optimization (Blind Matchmaker)
- ✅ First vouch = invitation itself (no token exchange)

## Operator Least Privilege
- ❌ NEVER allow operator to bypass protocol
- ❌ NEVER allow operator manual membership changes
- ✅ Operator is service runner only (start/stop bot)
- ✅ All bot actions automatic based on Freenet contract
- ✅ Operator is just another Member (no special privileges)
```

#### **`.beads/architecture-decisions.bead`** (IMMUTABLE)
```markdown
# Architecture Decisions (Immutable)

## Core Technologies
- **State Storage**: freenet-core (https://github.com/freenet/freenet-core)
- **ZK-Proofs**: STARKs (winterfell library)
- **Identity Masking**: HMAC-SHA256 (ring crate)
- **Signal Integration**: libsignal-service-rs
- **Memory Hygiene**: zeroize crate
- **Static Binary**: x86_64-unknown-linux-musl (MUSL)

## Node Architecture
- Each bot runs its own freenet-core node (no shared nodes)
- Bot operator provides Signal credentials
- Bot can recover state after going offline

## Performance Targets
- **Scalability**: 10²-10³ (100x to 1000x) without graph exposure
- **Latency**: Seconds to hours acceptable (security > speed)
- **Upper Bound**: Signal group limits (~1000 members)

## Federation as North Star
- MVP = Single group (no federation)
- ALL design decisions optimize for future federation
- Contract schema must support federation hooks (unused in MVP)
- Identity hashing must be re-computable for PSI-CA

## Threat Model (In Scope)
- Compromised operator (least privilege defense)
- Signal metadata analysis (1-on-1 PMs only)
- Freenet network analysis (anonymous routing)
- State-level adversaries (ZK-proofs, no graph exposure)
```

#### **`.beads/federation-roadmap.bead`** (IMMUTABLE)
```markdown
# Federation Roadmap (North Star)

## MVP Scope (Phase 0-2)
- ✅ Single group only
- ✅ Bootstrap seed group (3 members)
- ✅ Invitation & vetting
- ✅ Vouching & flagging
- ✅ Ejection (two triggers)
- ✅ Internal mesh optimization (Blind Matchmaker)
- ❌ NO federation in MVP

## Phase 3: Federation (Future)
- Shadow Beacon (emergent discovery)
- PSI-CA (Private Set Intersection Cardinality)
- Federation voting (Signal Polls)
- Cross-mesh vouching
- Federated Merkle Trees

## Design Principles
- **Emergent Discovery**: No pre-shared keys, Social Anchor Hashing
- **Blind Rendezvous**: Bots find each other via shared validators
- **Mutual Consent**: Both groups vote before federation
- **Privacy First**: PSI-CA reveals only overlap count, not identities
```

**Note**: These Beads are **pinned/immutable** and can only be changed via explicit unpinning ceremony (requires all agents to acknowledge change).

### 5. Complete Rust Project Scaffolding (Federation-Ready)

**Current State:** Basic `Cargo.toml` exists with minimal content (name, version, edition).

**Action Required:**

#### **Update `Cargo.toml` with dependencies:**
```toml
[package]
name = "stroma"
version = "0.1.0"
edition = "2021"

[dependencies]
# Cryptography
ring = "0.17"                          # HMAC identity masking
zeroize = { version = "1.7", features = ["derive"] }  # Memory hygiene
winterfell = "0.9"                     # STARKs for ZK-proofs

# Freenet integration
freenet-core = { git = "https://github.com/freenet/freenet-core" }

# Signal integration
libsignal-service = { git = "https://github.com/whisperfish/libsignal-service-rs" }

# Async runtime
tokio = { version = "1.35", features = ["full"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

[dev-dependencies]
proptest = "1.4"                       # Property-based testing
mock_instant = "0.3"                   # Deterministic time mocking

[profile.release]
opt-level = "z"                        # Optimize for size
lto = true                             # Link-time optimization
strip = true                           # Strip symbols
```

#### **Create module structure in `src/` (federation-ready):**
```
src/
├── main.rs                            # Event loop, CLI entry point
├── kernel/                            # Identity Masking
│   ├── mod.rs
│   ├── hmac.rs                        # HMAC-based hashing
│   └── zeroize_helpers.rs             # Immediate buffer purging
├── freenet/                           # Freenet Integration
│   ├── mod.rs
│   ├── node.rs                        # freenet-core node management
│   ├── contract.rs                    # Wasm contract deployment
│   └── state_stream.rs                # Real-time state monitoring
├── signal/                            # Signal Integration
│   ├── mod.rs
│   ├── bot.rs                         # Bot authentication & commands
│   ├── group.rs                       # Group management (add/remove)
│   └── pm.rs                          # 1-on-1 PM handling
├── crypto/                            # ZK-Proofs & Trust Verification
│   ├── mod.rs
│   ├── stark_circuit.rs               # STARK circuit for vouching
│   ├── proof_generation.rs            # Generate proofs
│   └── proof_verification.rs          # Verify proofs
├── gatekeeper/                        # Admission & Ejection Protocol
│   ├── mod.rs
│   ├── admission.rs                   # Vetting & admission logic
│   ├── ejection.rs                    # Immediate ejection (two triggers)
│   └── health_monitor.rs              # Continuous standing checks
├── matchmaker/                        # Internal Mesh Optimization
│   ├── mod.rs
│   ├── graph_analysis.rs              # Topology analysis
│   ├── cluster_detection.rs           # Identify internal clusters
│   └── strategic_intro.rs             # MST optimization suggestions
├── config/                            # Group Configuration
│   ├── mod.rs
│   └── group_config.rs                # GroupConfig struct (Freenet contract)
└── federation/                        # Federation Logic (Unused in MVP)
    ├── mod.rs
    ├── shadow_beacon.rs               # Social Anchor Hashing (compute but don't broadcast)
    ├── psi_ca.rs                      # Private Set Intersection Cardinality
    └── diplomat.rs                    # Federation proposals (Phase 3+)
```

**Key Design**: `federation/` module exists but is **disabled in MVP** (feature flag or config). This ensures all infrastructure is ready for Phase 3.

#### **Create `.cargo/config.toml`:**
```toml
[build]
target = "x86_64-unknown-linux-musl"

[target.x86_64-unknown-linux-musl]
linker = "x86_64-linux-musl-gcc"
rustflags = ["-C", "target-feature=+crt-static"]
```

#### **Create `cargo-deny.toml`:**
```toml
[advisories]
db-path = "~/.cargo/advisory-db"
db-urls = ["https://github.com/rustsec/advisory-db"]
vulnerability = "deny"
unmaintained = "warn"

[licenses]
unlicensed = "deny"
allow = ["MIT", "Apache-2.0", "BSD-3-Clause"]

[bans]
multiple-versions = "warn"
wildcards = "deny"

[sources]
unknown-registry = "deny"
unknown-git = "warn"
```

### 6. Create Phase 0 (Foundation) Beads Issues

**Action Required:** Use `bd create` to create initial roadmap issues for **Phase 0: Foundation (Weeks 1-2)**:

#### **Bead-01: Kernel (Identity Masking)**
```bash
bd create --title "Implement HMAC identity masking with zeroization" \
  --description "
  - HMAC-SHA256 with group-secret pepper (not deterministic hashing)
  - Zeroize buffers immediately after hashing
  - Ephemeral session storage (deleted after admission)
  - Unit tests with fixed test pepper for determinism
  - Property tests: hash(id, pepper1) != hash(id, pepper2)
  "
```

#### **Bead-02: Freenet Integration (Node & State)**
```bash
bd create --title "Integrate freenet-core node and state monitoring" \
  --description "
  - Node lifecycle management (start/stop/recover)
  - Wasm contract deployment (stub for MVP)
  - State stream monitoring (real-time, not polling)
  - Event-driven architecture (subscribe to state changes)
  - Recovery logic (bot offline/online handling)
  "
```

#### **Bead-03: Signal Integration (Bot & Commands)**
```bash
bd create --title "Implement Signal bot authentication and commands" \
  --description "
  - Bot registration and authentication
  - Group management (add_member, remove_member)
  - 1-on-1 PM handling (/invite, /vouch, /flag, /status)
  - Command parsing and routing
  - Never log or store Signal IDs in cleartext
  "
```

#### **Bead-04: Crypto Layer (STARK Circuits)**
```bash
bd create --title "Implement STARK circuits for vouch verification" \
  --description "
  - Circuit: Verify 2 vouches from different members
  - Proof generation (target: < 10 seconds)
  - Proof verification (constant time)
  - Merkle Tree proof integration
  - Test with sample data (proof size < 100KB)
  "
```

#### **Bead-05: Contract Schema (ComposableState & Federation-Ready)**
```bash
bd create --title "Design Freenet contract schema with ComposableState" \
  --description "
  - Implement ComposableState trait for all state fields
  - MemberSet: BTreeSet-based (active + removed tombstones)
  - VouchGraph: HashMap<MemberHash, BTreeSet<MemberHash>> (mergeable)
  - FlagGraph: Similar to VouchGraph
  - GroupConfigV1: Last-Write-Wins with version field
  - TrustNetworkState: #[composable] macro for auto-composition
  - Helper methods: generate_merkle_tree(), calculate_standing(), should_eject()
  - Federation hooks (Phase 4+, feature-flagged)
  - Test: Merge semantics are commutative (order-independent)
  - Based on decisions from Outstanding Questions Q1-Q5
  "
```

**Note**: These issues are for **Phase 0 only**. Phase 1 (Bootstrap & Core Trust) and Phase 2 (Mesh Optimization) will be created after Phase 0 completion.

### 7. Define Agent Structure & Responsibilities

**Action Required:** Document agent boundaries and coordination strategy.

#### **Agent Hierarchy**
```
Mayor (Lead Agent)
├─ Agent-Signal (Signal protocol integration)
│  ├─ Bot authentication & registration
│  ├─ Group management (add/remove members)
│  ├─ Command parsing (/invite, /vouch, /flag, etc.)
│  ├─ 1-on-1 PM handling
│  └─ Constraints: NEVER log Signal IDs
│
├─ Agent-Freenet (freenet-core integration)
│  ├─ Node lifecycle (start/stop/recover)
│  ├─ Contract deployment & state management
│  ├─ State stream monitoring (real-time)
│  ├─ Event-driven architecture
│  └─ Constraints: Freenet is source of truth
│
├─ Agent-Crypto (ZK-STARKs + HMAC)
│  ├─ HMAC identity masking
│  ├─ STARK circuit design & implementation
│  ├─ Proof generation & verification
│  ├─ Memory hygiene (zeroization)
│  └─ Constraints: Immediate zeroization, no cleartext IDs
│
└─ Witness Agent (Security audit)
   ├─ Monitor all agents for Signal ID leakage
   ├─ Verify zeroization in crypto operations
   ├─ Block unsafe patterns (e.g., cleartext ID logging)
   └─ Read .beads/security-constraints.bead before each review
```

#### **Convoy Execution Strategy**
```bash
# Parallel development with Witness oversight
gt convoy \
  --agents "Agent-Signal,Agent-Freenet,Agent-Crypto" \
  --witness "Witness-Agent" \
  --beads ".beads/security-constraints.bead,.beads/architecture-decisions.bead"
```

#### **Context Preservation (Beads)**
- All agents must read constraint Beads before writing code
- Constraint Beads are **immutable** (pinned)
- Changes require explicit unpinning ceremony
- Mayor coordinates but does not override security constraints

### 8. Configure Mayor Briefing

**Action Required:** Create `mayor/BRIEFING.md` with comprehensive context.

**Contents**:
```markdown
# Mayor Agent Briefing: Stroma Project

## Mission
Build a privacy-first, decentralized trust network that scales to federated groups.

## Technology Stack (Finalized)
- State: freenet-core (https://github.com/freenet/freenet-core)
- ZK-Proofs: STARKs (winterfell)
- Identity: HMAC-SHA256 (ring) + zeroize
- UI: Signal (libsignal-service-rs)
- Target: x86_64-unknown-linux-musl (static MUSL binary)

## MVP Scope
- Single group (no federation)
- Bootstrap, invitation, vetting, vouching, flagging, ejection
- Basic commands: /invite, /vouch, /flag, /status, /mesh
- Federation infrastructure present but disabled

## Critical Constraints (Read .beads/security-constraints.bead)
1. NEVER store Signal IDs in cleartext
2. NEVER bypass ZK-proof verification
3. NEVER add grace periods for ejection
4. NEVER make Signal group source of truth
5. NEVER restrict vouching to Validators only

## Agent Coordination
- Agent-Signal: Signal protocol integration
- Agent-Freenet: freenet-core integration
- Agent-Crypto: ZK-STARKs + HMAC
- Witness-Agent: Security audit (continuous)

## Current Phase: Phase 0 (Foundation)
See .beads/issues.jsonl for Bead-01 through Bead-05

## Cursor Rules
All rules in .cursor/rules/ are authoritative.
Read user-roles-ux.mdc for complete UX specification.
```

### 9. Initialize First Convoy (After Spike Week)

**Prerequisites**: 
- ✅ Spike week completed (freenet-core, Signal, STARKs validated)
- ✅ Go/No-Go decision made
- ✅ Constraint Beads created
- ✅ Agent structure defined

**Action**:
```bash
# Launch Phase 0 convoy
gt convoy start \
  --phase "Phase 0: Foundation" \
  --beads "Bead-01,Bead-02,Bead-03,Bead-04,Bead-05" \
  --agents "Agent-Signal,Agent-Freenet,Agent-Crypto" \
  --witness "Witness-Agent"
```

**Deliverable**: Foundation modules complete (kernel, freenet, signal, crypto, contract schema)

## Files to Create/Update

**Already Exists:**

- ✅ `.gitignore` - Gastown-aware gitignore configured
- ✅ `Cargo.toml` - Basic structure exists (needs dependencies)
- ✅ `.beads/` - Gastown Beads system initialized

**Need to Create:**

1. `.cargo/config.toml` - MUSL target configuration
2. `cargo-deny.toml` - Supply chain security config
3. `src/kernel/mod.rs` - Identity masking module
4. `src/shadow_beacon/mod.rs` - Emergent discovery module
5. `src/gatekeeper/mod.rs` - Signal admin bot module
6. `src/diplomat/mod.rs` - Federation logic module
7. `.beads/constraints/` - Architectural constraint documents (or Beads issues)
8. `README.md` - Project overview and Gastown workflow
9. `mayor/BRIEFING.md` - Mayor agent system prompt and context

## Verification Steps

After completing remaining steps:

- ✅ Verify git repository: `git status` (should show clean working tree after initial commit)
- ✅ Verify Beads system: `bd list` (should show roadmap issues)
- ✅ Verify Rust project: `cargo check` (should compile with module structure)
- ✅ Verify MUSL target: `rustup target add x86_64-unknown-linux-musl`
- ✅ Verify cargo-deny: `cargo deny check` (should run without errors)
- ⚠️ Verify `gt` command: May need PATH configuration if not accessible

## Current Directory Structure

```
/Users/matt/src/github.com/roder/stroma/
├── .beads/              ✅ Gastown Beads system (formulas, issues, config)
├── .claude/             ✅ Claude agent context
├── .cursor/rules/       ✅ Project rules and standards
├── .gitignore           ✅ Gastown-aware gitignore
├── .gitattributes       ✅ Git attributes
├── AGENTS.md            ✅ Agent workflow instructions
├── Cargo.toml           ⚠️  Basic structure (needs dependencies)
├── mayor/               ✅ Mayor agent configuration
│   ├── CLAUDE.md        ✅ Mayor context
│   ├── daemon.json      ✅ Daemon config
│   └── rigs.json        ✅ Rig configuration
├── deacon/              ✅ Deacon build monitoring
├── settings/            ✅ Gastown settings
├── docs/                ✅ Documentation
└── src/
    └── main.rs          ⚠️  Placeholder (needs module structure)
```

## Implementation Roadmap (Detailed)

### **Phase 0: Foundation (Weeks 1-2)**
**Focus**: Core infrastructure with federation-ready design

```
├─ Kernel (Identity Masking)
│  ├─ HMAC hashing with group pepper
│  ├─ Zeroization immediately after hashing
│  └─ Ephemeral session storage
├─ Freenet Integration
│  ├─ freenet-core node management (start/stop/recover)
│  ├─ Contract deployment (stub)
│  └─ State stream monitoring (real-time)
├─ Signal Integration (Basic)
│  ├─ Bot authentication
│  ├─ Group management (add/remove)
│  └─ 1-on-1 PM handling
├─ Crypto Layer
│  ├─ STARK circuit for vouch verification
│  ├─ Proof generation/verification
│  └─ Merkle Tree integration
└─ Contract Schema (Federation-Ready)
   ├─ TrustNetworkState struct
   ├─ Federation hooks (unused in MVP)
   └─ GroupConfig struct
```

**Deliverable**: Working foundation modules, ready for Phase 1

### **Phase 1: Bootstrap & Core Trust (Weeks 3-4)**
**Focus**: Seed group, vetting, admission, ejection

```
├─ Bootstrap Module
│  ├─ Seed group (3 members)
│  ├─ Initial triangle vouching
│  └─ Freenet state initialization
├─ Trust Operations
│  ├─ Invitation (first vouch)
│  ├─ Second vouch via Blind Matchmaker
│  ├─ Admission (ZK-proof verification)
│  └─ Ejection (both triggers: Standing < 0 OR Vouches < 2)
├─ Basic Commands
│  ├─ /invite @user [context]
│  ├─ /vouch @user
│  ├─ /flag @user [reason]
│  └─ /status
└─ Health Monitoring
   ├─ Continuous state stream
   ├─ Standing recalculation
   └─ Automatic ejection
```

**Deliverable**: Working single-group bot with core trust mechanics

### **Phase 2: Internal Mesh Optimization (Weeks 5-6)**
**Focus**: Graph analysis, strategic introductions, MST

```
├─ Blind Matchmaker
│  ├─ Graph topology analysis
│  ├─ Cluster identification (internal)
│  ├─ Strategic introduction suggestions
│  └─ MST optimization (least new interactions)
├─ Advanced Commands
│  ├─ /mesh (network overview)
│  ├─ /mesh strength (histogram)
│  ├─ /mesh config
│  └─ /propose-config key=value
├─ Configuration Management
│  ├─ Signal Polls for voting
│  ├─ config_change_threshold enforcement
│  └─ Automatic config updates
└─ Operator Audit
   └─ /audit operator command
```

**Deliverable**: Optimized single-group bot with full command set

### **Phase 3: Federation Preparation (Week 7)**
**Focus**: Validate federation infrastructure (no broadcast)

```
├─ Shadow Beacon (Compute Locally)
│  ├─ Social Anchor hashing
│  ├─ Validator percentile calculation
│  └─ Discovery URI generation (not broadcast)
├─ PSI-CA (Test Locally)
│  ├─ Bloom filter generation
│  ├─ Commutative encryption
│  └─ Intersection density calculation
├─ Contract Schema Validation
│  ├─ Test federation hooks (unused but present)
│  └─ Verify re-computable identity hashes
└─ Documentation
   ├─ Federation design document
   └─ Phase 4+ roadmap
```

**Deliverable**: MVP ready for production + validated federation path

### **Phase 4+: Federation (Future)**
**Out of MVP Scope** - Documented as north star

```
├─ Emergent Discovery
│  ├─ Shadow Beacon broadcast
│  ├─ Multiple discovery URIs
│  └─ Anonymous bot-to-bot handshake
├─ PSI-CA Protocol
│  ├─ Overlap calculation (no identity reveal)
│  ├─ BidirectionalMin evaluation
│  └─ Federation proposal
├─ Federation Voting
│  ├─ Signal Poll to members
│  ├─ Mutual consent (both groups vote)
│  └─ Contract signing
└─ Cross-Mesh Vouching
   ├─ Federated Merkle Trees
   ├─ Shadow vouches
   └─ Expedited vetting
```

## Success Criteria

### Phase 0 (Foundation)
- [ ] freenet-core node runs successfully
- [ ] STARK proof generated (< 100KB, < 10 seconds)
- [ ] Signal bot can manage group (add/remove members)
- [ ] HMAC masking works with immediate zeroization
- [ ] Contract schema supports federation hooks

### Phase 1 (Bootstrap & Core Trust)
- [ ] 3-member seed group bootstrapped successfully
- [ ] New member admitted after 2 vouches (ZK-proof verified)
- [ ] Member ejected when Standing < 0
- [ ] Member ejected when Vouches < 2
- [ ] All vetting in 1-on-1 PMs (no group chat exposure)

### Phase 2 (Mesh Optimization)
- [ ] Graph topology correctly identifies Bridges and Validators
- [ ] Strategic introductions suggested for MST
- [ ] Mesh density histogram displayed correctly
- [ ] Configuration changes via Signal Poll (70% threshold)
- [ ] Operator audit trail queryable

### Phase 3 (Federation Prep)
- [ ] Social Anchor hash computed correctly
- [ ] PSI-CA overlap calculated locally (test with mock data)
- [ ] Federation hooks in contract validated
- [ ] Documentation complete for Phase 4

## Next Actions (Immediate)

1. ✅ **Complete Git Initialization**: Commit all existing files
2. 🔬 **Run Spike Week**: Validate freenet-core, Signal, STARKs (1 week)
3. 📋 **Create Constraint Beads**: Immutable security constraints
4. 🏗️ **Scaffold Modules**: Rust project structure with federation hooks
5. 🎯 **Create Phase 0 Beads**: Issues for foundation work
6. 📖 **Brief Mayor**: Technology decisions and roadmap
7. 🚀 **Launch Phase 0 Convoy**: Begin parallel agent development