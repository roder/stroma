# Stroma Developer Guide

**For Contributors & Technical Audience**

This guide explains Stroma's architecture, technical stack, and development workflow.

**Terminology**: See `.beads/terminology.bead` for canonical definitions of all terms used in this document.

## Architecture Overview

### Three-Layer Design

```
┌─────────────────────────────────────┐
│  Layer 1: Signal (User Interface)  │  ← Human-facing, E2E encrypted
├─────────────────────────────────────┤
│  Layer 2: Rust Bot (Trust Logic)   │  ← Gatekeeper, Matchmaker, Monitor
├─────────────────────────────────────┤
│  Layer 3: Freenet (State Storage)  │  ← Decentralized, anonymous, mergeable
└─────────────────────────────────────┘
```

### Core Innovation: Recursive ZK-Vouching

**Problem**: Traditional trust networks must reveal social graph to scale

**Solution**: Zero-knowledge proofs verify trust without revealing who vouched

**Technologies:**
- **Embedded Freenet Kernel** (freenet-stdlib) - In-process, not external service
- **STARKs** (winterfell) - No trusted setup, post-quantum secure
- **On-Demand Merkle Trees** - Generated from BTreeSet for ZK-proofs (not stored)
- **ComposableState** - Freenet trait for mergeable state with summary-delta sync
- **Vouch Invalidation** - Logical consistency (can't both trust and distrust)
- **Minimum Spanning Tree** - Optimal mesh topology with maximum anonymity (see [ALGORITHMS.md](ALGORITHMS.md))

## Technical Stack

### Core Technologies

| Component | Library/Version | Purpose |
|-----------|----------------|---------|
| **Language** | Rust 1.93+ | musl 1.2.5, improved DNS, memory safety |
| **Embedded Kernel** | [freenet-stdlib](https://docs.rs/freenet-stdlib) v0.1.30+ | In-process Freenet kernel |
| **Contract Framework** | [freenet-scaffold](https://github.com/freenet/freenet-scaffold) v0.2+ | ComposableState utilities |
| **ZK-Proofs** | winterfell | STARKs (no trusted setup) |
| **Identity Hashing** | ring (HMAC-SHA256) | Group-scoped masking |
| **Memory Hygiene** | zeroize | Immediate buffer purging |
| **Signal Integration** | libsignal-service-rs | Protocol-level Signal |
| **Async Runtime** | tokio 1.35+ | Event-driven execution |
| **CLI Framework** | clap 4+ | Operator interface |
| **Supply Chain** | cargo-deny, cargo-crev | Security audits |

### Why Rust 1.93+

- **musl 1.2.5**: Major DNS resolver improvements ([InfoWorld article](https://www.infoworld.com/article/4120988/rust-1-93-updates-bundled-musl-library-to-boost-networking.html))
- **Static linking**: No dynamic library vulnerabilities
- **Networking**: More reliable for Signal and freenet-core
- **DNS handling**: Better with large records and recursive name servers

### Why STARKs (not PLONK)

| Feature | STARKs | PLONK |
|---------|--------|-------|
| **Trusted Setup** | ❌ None | ✅ Required (ceremony) |
| **Post-Quantum** | ✅ Secure | ❌ Vulnerable |
| **Transparency** | ✅ Fully transparent | ⚠️ Depends on setup |
| **Proof Size** | ⚠️ Larger (~100KB) | ✅ Smaller (~1KB) |
| **Verification** | ✅ Constant time | ✅ Constant time |

**Decision**: STARKs for trustlessness and post-quantum security (proof size acceptable)

## Module Structure (Federation-Ready)

```
src/
├── main.rs                          # Event loop, CLI entry point
├── kernel/                          # Identity Masking
│   ├── mod.rs
│   ├── hmac.rs                      # HMAC-based hashing with group pepper
│   └── zeroize_helpers.rs           # Immediate buffer purging
├── freenet/                         # Freenet Integration
│   ├── mod.rs
│   ├── node.rs                      # freenet-core node management
│   ├── contract.rs                  # Wasm contract deployment
│   └── state_stream.rs              # Real-time state monitoring
├── signal/                          # Signal Integration
│   ├── mod.rs
│   ├── bot.rs                       # Bot authentication & commands
│   ├── group.rs                     # Group management (add/remove)
│   └── pm.rs                        # 1-on-1 PM handling
├── crypto/                          # ZK-Proofs & Trust Verification
│   ├── mod.rs
│   ├── stark_circuit.rs             # STARK circuit for vouching
│   ├── proof_generation.rs          # Generate proofs (spawn_blocking)
│   └── proof_verification.rs        # Verify proofs
├── gatekeeper/                      # Admission & Ejection Protocol
│   ├── mod.rs
│   ├── admission.rs                 # Vetting & admission logic
│   ├── ejection.rs                  # Immediate ejection (two triggers)
│   └── health_monitor.rs            # Continuous standing checks
├── matchmaker/                      # Internal Mesh Optimization
│   ├── mod.rs
│   ├── graph_analysis.rs            # Topology analysis (Union-Find, centrality)
│   ├── cluster_detection.rs         # Identify internal clusters (connected components)
│   └── strategic_intro.rs           # MST optimization (see ALGORITHMS.md)
├── config/                          # Group Configuration
│   ├── mod.rs
│   └── group_config.rs              # GroupConfig struct (Freenet contract)
└── federation/                      # Federation Logic (DISABLED IN MVP)
    ├── mod.rs                       # Feature flag: #[cfg(feature = "federation")]
    ├── shadow_beacon.rs             # Social Anchor Hashing (Phase 4+)
    ├── psi_ca.rs                    # Private Set Intersection (Phase 4+)
    ├── diplomat.rs                  # Federation proposals (Phase 4+)
    └── shadow_handover.rs           # Bot identity rotation (Phase 4+)
```

**Key Design**: `federation/` exists but is disabled via feature flag in MVP (validates architecture scales).

**See**: [ALGORITHMS.md](ALGORITHMS.md) for detailed MST algorithm, PSI-CA protocol, and complexity analysis.

### Future: Shadow Handover (Phase 4+)

The `shadow_handover.rs` module will implement cryptographic bot identity rotation:

- **Purpose**: Allow bot to rotate Signal phone number while preserving trust context
- **Mechanism**: Succession Document signed by old bot key, validated by Freenet contract
- **Use Cases**: Signal ban recovery, periodic rotation, operational security

See `.beads/federation-roadmap.bead` for protocol specification.

## Freenet Contract Design

### ComposableState Requirement

Freenet contracts must implement `ComposableState` trait for summary-delta synchronization:

```rust
pub trait ComposableState {
    type ParentState;
    type Summary;
    type Delta;
    type Parameters;
    
    fn verify(&self, parent: &Self::ParentState, params: &Self::Parameters) 
        -> Result<(), String>;
    fn summarize(&self, parent: &Self::ParentState, params: &Self::Parameters) 
        -> Self::Summary;
    fn delta(&self, parent: &Self::ParentState, params: &Self::Parameters, old: &Self::Summary) 
        -> Option<Self::Delta>;
    fn apply_delta(&mut self, parent: &Self::ParentState, params: &Self::Parameters, delta: &Option<Self::Delta>) 
        -> Result<(), String>;
}
```

### Mergeable State Structures

❌ **NOT Mergeable:**
```rust
pub struct TrustNetworkState {
    members: MerkleTree<MemberHash>,  // Two different trees = conflict
    vouches: Vec<VouchProof>,         // Order matters in Vec
}
```

✅ **Mergeable (Use These):**
```rust
pub struct TrustNetworkState {
    members: BTreeSet<MemberHash>,                      // Set union
    vouches: HashMap<MemberHash, BTreeSet<MemberHash>>, // Map union
    flags: HashMap<MemberHash, BTreeSet<MemberHash>>,   // Map union
}
```

### Stroma Contract Schema

```rust
use freenet_scaffold_macro::composable;
use std::collections::{BTreeSet, HashMap};

#[composable]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TrustNetworkState {
    // Field order matters! Dependencies go later.
    config: GroupConfigV1,        // No dependencies
    members: MemberSet,            // Depends on config
    vouches: VouchGraph,           // Depends on members
    flags: FlagGraph,              // Depends on members
    
    // Federation hooks (Phase 4+, disabled in MVP)
    #[cfg(feature = "federation")]
    federation_contracts: FederationSet,
}
```

**See**: `.cursor/rules/freenet-contract-design.mdc` for complete patterns and examples

### On-Demand Merkle Trees

**Key Insight**: Don't store Merkle Trees - generate on demand for ZK-proof verification

```rust
impl TrustNetworkState {
    /// Generate Merkle Tree from current member set
    pub fn generate_merkle_tree(&self) -> MerkleTree<MemberHash> {
        let sorted: Vec<_> = self.members.active.iter().cloned().collect();
        MerkleTree::from_leaves(sorted)
    }
    
    /// Get Merkle root for ZK-proof verification
    pub fn merkle_root(&self) -> Hash {
        self.generate_merkle_tree().root()
    }
}
```

**Performance Target**: < 100ms for 1000 members (validated in Spike Week)

## Trust Model Implementation

### Vouch Invalidation Logic

**Critical Rule**: If a voucher flags a member, that vouch is invalidated

```rust
pub fn calculate_effective_state(&self, member: &MemberHash) -> (usize, i32) {
    let vouchers = self.vouches.get(member).cloned().unwrap_or_default();
    let flaggers = self.flags.get(member).cloned().unwrap_or_default();
    
    // Find vouchers who also flagged (contradictory)
    let voucher_flaggers: HashSet<_> = vouchers
        .intersection(&flaggers)
        .collect();
    
    // Effective vouches = total vouches - voucher_flaggers
    let effective_vouches = vouchers.len() - voucher_flaggers.len();
    
    // Regular flags = flags from non-vouchers
    let regular_flags = flaggers.len() - voucher_flaggers.len();
    
    // Standing = effective_vouches - regular_flags
    let standing = effective_vouches as i32 - regular_flags as i32;
    
    (effective_vouches, standing)
}
```

**See**: [VOUCH-INVALIDATION-LOGIC.md](VOUCH-INVALIDATION-LOGIC.md) for detailed examples

### Ejection Protocol (Two Independent Triggers)

```rust
pub fn should_eject(&self, member: &MemberHash) -> bool {
    let (effective_vouches, standing) = self.calculate_effective_state(member);
    
    // Trigger 1: Standing < 0 (too many regular flags)
    if standing < 0 {
        return true;
    }
    
    // Trigger 2: Effective vouches < min_vouch_threshold
    if effective_vouches < self.config.min_vouch_threshold {
        return true;
    }
    
    false
}
```

## Bot Architecture

### 1:1 Bot-to-Group Relationship

**Architecture**: One bot process per Stroma group

```rust
pub struct StromaBot {
    signal_client: PresageManager,   // One Signal connection
    freenet_node: FreenetClient,     // One embedded Freenet kernel
    group_id: GroupId,                // Single group only
    group_name: String,               // "Mission Control", "Activists-NYC"
    config: GroupConfig,              // Group-specific configuration
}
```

**Deployment Model:**
- Each Stroma group = separate bot instance
- Each bot instance = separate systemd service
- Each bot instance = separate Freenet contract
- Scale: <100 groups = <100 processes

**Why 1:1:**
- Simpler state management (each bot owns one contract)
- Isolation (one group's issues don't cascade to others)
- Clear identity (bot phone number = group identity)
- Easier debugging (logs, state, errors per group)
- Federation clarity (1 bot = 1 mesh)

**See**: `.beads/bot-deployment-model.bead`

### Signal Integration: Presage

**Use Presage (high-level API)** for Signal protocol:

```rust
use presage::Manager;
// ❌ DO NOT USE: use presage_store_sqlite::SqliteStore;
// Default SqliteStore stores ALL messages - server seizure risk!

use stroma::store::StromaProtocolStore;  // ✅ Custom minimal store

// Registration (done via provisioning tool)
let store = StromaProtocolStore::new()?;
let manager = Manager::with_store(store, options).await?;

// Send messages
manager.send_message(recipient, message, timestamp).await?;
manager.send_message_to_group(master_key, message, timestamp).await?;

// Receive messages
let messages = manager.receive_messages().await?;
```

**CRITICAL SECURITY REQUIREMENT:**

Never use `presage_store_sqlite::SqliteStore` - it persists ALL messages to disk. If the bot server is seized, the adversary would get:
- ❌ Complete vetting conversation history
- ❌ Relationship context ("Great activist from...")
- ❌ Contact database linking to Signal IDs

**Required:** Implement custom `StromaProtocolStore` that stores ONLY:
- ✅ Signal protocol state (sessions, pre-keys) - ~100KB encrypted file
- ✅ In-memory ephemeral message processing (never written to disk)

See "Bot Storage (CRITICAL)" section below for implementation.

**When Presage insufficient**, drop to libsignal-service-rs:

```rust
use presage::libsignal_service::proto::DataMessage;
use presage::libsignal_service::proto::data_message::PollCreate;

// Custom protobuf messages
let poll = DataMessage {
    poll_create: Some(PollCreate {
        question: Some("Proposal question?".to_string()),
        options: vec!["Approve".to_string(), "Reject".to_string()],
        ..Default::default()
    }),
    ..Default::default()
};
```

**See**: `.beads/technology-stack.bead`, `.beads/security-constraints.bead` § 10

### Poll Support (Protocol v8)

**Fork Strategy:**
- Use forked libsignal-service-rs with protocol v8 poll support
- Submit PR to upstream Whisperfish
- Don't wait for merge - use fork immediately

**Cargo.toml:**
```toml
[dependencies]
presage = { git = "https://github.com/whisperfish/presage" }

# ❌ DO NOT ADD: presage-store-sqlite (server seizure risk)
# Use custom StromaProtocolStore instead

[patch.crates-io]
libsignal-service = {
    git = "https://github.com/roder/libsignal-service-rs",
    branch = "feature/protocol-v8-polls"
}
```

**IMPORTANT:** Never add `presage-store-sqlite` as a dependency. It stores complete message history, violating our server seizure protection model.

**See**: `.beads/poll-implementation-gastown.bead`, `.beads/voting-mechanism.bead`

### Event-Driven Design with Embedded Kernel

```rust
#[tokio::main]
async fn main() -> Result<(), Error> {
    // Parse CLI arguments
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Run { config, .. } => {
            run_bot_service(config).await?;
        }
        Commands::Bootstrap { .. } => {
            // Bootstrap handled separately
        }
        // ... other commands
    }
    
    Ok(())
}

async fn run_bot_service(config_path: PathBuf) -> Result<(), Error> {
    let config = load_config(&config_path)?;
    
    // Initialize embedded Freenet kernel (in-process, not external service)
    let kernel = FreenetKernel::builder()
        .mode(NetworkMode::Dark)  // Anonymous routing
        .data_dir(&config.freenet.data_dir)
        .build()
        .await?;
    
    // Load existing contract
    let contract_key = config.freenet.contract_key;
    
    // Initialize Signal bot
    let signal = SignalBot::authenticate(&config.signal).await?;
    
    // Subscribe to contract state stream from embedded kernel
    let mut state_stream = kernel.subscribe_to_contract(contract_key).await?;
    
    // Event loop (single process handles both Freenet and Signal)
    loop {
        tokio::select! {
            // Freenet state changes (from embedded kernel)
            Some(state_change) = state_stream.next() => {
                handle_state_change(state_change, &signal).await?;
            }
            
            // Signal messages
            Some(message) = signal.recv_message() => {
                handle_signal_command(message, &kernel, contract_key).await?;
            }
            
            // Periodic health check
            _ = health_check_interval.tick() => {
                check_all_trust_standings(&kernel, &signal, contract_key).await?;
            }
        }
    }
}
```

### State Stream Monitoring (NOT Polling)

```rust
// ✅ REQUIRED PATTERN (Real-time stream)
async fn monitor_state_changes(freenet: &FreenetClient, signal: &SignalClient) {
    let mut stream = freenet.subscribe_to_state_changes().await.unwrap();
    
    while let Some(change) = stream.next().await {
        match change {
            StateChange::MemberVetted(hash) => {
                signal.add_member(hash).await?;
            },
            StateChange::MemberRevoked(hash) => {
                signal.remove_member(hash).await?;
            },
        }
    }
}

// ❌ FORBIDDEN PATTERN (Polling)
async fn poll_state() {
    loop {
        let state = freenet.get_state().await.unwrap();
        // ...
        tokio::time::sleep(Duration::from_secs(1)).await; // ❌ Wasteful
    }
}
```

### Proposal System & Anonymous Voting

**Governance**: Bot is Signal admin (technical) but execute-only (no decision power)

**All group decisions flow through `/propose` system:**

```rust
pub struct Proposal {
    id: ProposalId,
    proposer: Hash,
    proposal_type: ProposalType,
    
    // Configuration (per-proposal)
    timeout: Duration,              // Configurable per poll
    threshold: f32,                 // From GroupConfig (not per-proposal)
    
    // Execution
    action: FreenetAction,
    
    // Timestamps
    created_at: Timestamp,
    expires_at: Timestamp,
}

pub enum ProposalType {
    ConfigChange { key: String, value: String },      // Signal group settings
    StromaConfig { key: String, value: String },      // Stroma trust settings
    Federation { group_id: String },                  // Federation proposal
}

pub enum FreenetAction {
    UpdateSignalGroupSetting { key: String, value: String },
    UpdateStromaConfig { key: String, value: String },
    InitiateFederation { group_id: String },
}
```

**Create Proposal with Signal Poll:**

```rust
use presage::libsignal_service::proto::DataMessage;
use presage::libsignal_service::proto::data_message::PollCreate;

async fn create_proposal(
    manager: &Manager,
    group_master_key: &[u8],
    proposal: &Proposal,
) -> Result<String> {
    // Format proposal as message + poll
    let poll_message = DataMessage {
        body: Some(format_proposal_details(proposal)),
        poll_create: Some(PollCreate {
            question: Some(format_proposal_question(proposal)),
            allow_multiple: Some(false),
            options: vec!["👍 Approve".to_string(), "👎 Reject".to_string()],
        }),
        timestamp: Some(now()),
        ..Default::default()
    };
    
    let message_id = manager.send_message_to_group(
        group_master_key,
        poll_message,
        now(),
    ).await?;
    
    // Store in Freenet contract
    freenet.record_active_proposal(proposal, message_id).await?;
    
    Ok(message_id)
}
```

**Monitor Poll Results (After Timeout):**

```rust
async fn check_proposal_results(
    manager: &Manager,
    proposal: &ActiveProposal,
) -> Result<ProposalResult> {
    // Fetch aggregated poll results from Signal
    // NOTE: Signal provides only vote counts, NOT who voted what
    let poll_data = manager.get_poll_results(
        proposal.poll_timestamp,
    ).await?;
    
    let approve_count = poll_data.options[0].vote_count;  // 👍
    let reject_count = poll_data.options[1].vote_count;   // 👎
    let total_votes = approve_count + reject_count;
    
    if total_votes == 0 {
        return Ok(ProposalResult::NoVotes);
    }
    
    let approval_ratio = approve_count as f32 / total_votes as f32;
    
    Ok(ProposalResult {
        approved: approval_ratio >= proposal.threshold,
        approve_count,
        reject_count,
        approval_ratio,
        // NO individual votes - preserves anonymity
    })
}
```

**Execute Approved Actions:**

```rust
async fn execute_proposal(
    proposal: &Proposal,
    signal: &SignalClient,
    freenet: &FreenetClient,
) -> Result<()> {
    // ALWAYS verify Freenet approval first
    if !freenet.is_proposal_approved(proposal.id).await? {
        return Err("Proposal not approved in Freenet contract");
    }
    
    // Execute action (bot uses Signal admin power here)
    match &proposal.action {
        FreenetAction::UpdateSignalGroupSetting { key, value } => {
            signal.update_group_setting(key, value).await?;
        },
        FreenetAction::UpdateStromaConfig { key, value } => {
            freenet.update_config(key, value).await?;
        },
        FreenetAction::InitiateFederation { group_id } => {
            freenet.establish_federation(group_id).await?;
        },
    }
    
    // Record execution in contract
    freenet.record_execution(proposal.id, now()).await?;
    
    Ok(())
}
```

**Proposal Monitoring (Real-Time Stream - NOT Polling):**

```rust
// ✅ REQUIRED PATTERN: Real-time state stream
// See: .cursor/rules/security-guardrails.mdc "State Management Violations"
async fn proposal_monitoring_stream(
    manager: &Manager,
    freenet: &FreenetClient,
) {
    // Subscribe to real-time state changes (NOT polling)
    let mut state_stream = freenet.subscribe_to_state_changes().await.unwrap();
    
    while let Some(change) = state_stream.next().await {
        match change {
            StateChange::ProposalExpired(proposal_id) => {
                // Fetch proposal details
                let proposal = freenet.get_proposal(proposal_id).await?;
                
                // Fetch poll results (anonymous)
                let result = check_proposal_results(manager, &proposal).await?;
                
                // Execute if approved
                if result.approved {
                    execute_proposal(&proposal.proposal, manager, freenet).await?;
                }
                
                // Mark as checked
                freenet.mark_proposal_checked(proposal_id).await?;
            },
            // Handle other state changes...
            _ => {}
        }
    }
}

// ❌ FORBIDDEN PATTERN: Polling
// async fn poll_monitoring_loop(...) {
//     loop {
//         sleep(Duration::from_secs(60)).await;  // ❌ NEVER USE POLLING
//         let state = freenet.get_state().await?; // ❌ NEVER POLL
//     }
// }
```

**Anonymity Guarantee:**
- Bot sees: Total approve, total reject, approval ratio
- Bot does NOT see: Who voted, how they voted
- Members see: Aggregate counts only (Signal's poll UI)

**See**: `.beads/proposal-system.bead`, `.beads/voting-mechanism.bead`

### Event-Driven Design with Embedded Kernel

### Anonymity-First Design

#### Identity Masking (MANDATORY)

```rust
use ring::hmac;
use zeroize::Zeroize;

pub fn mask_identity(signal_id: &str, group_pepper: &[u8]) -> Hash {
    // Use HMAC-SHA256 (NOT deterministic hashing)
    let key = hmac::Key::new(hmac::HMAC_SHA256, group_pepper);
    let tag = hmac::sign(&key, signal_id.as_bytes());
    
    Hash::from_bytes(tag.as_ref())
    
    // signal_id is borrowed, but owned data must be zeroized:
    // signal_id_owned.zeroize();
}
```

**Critical**: Different groups → different hashes for same person (enables PSI-CA privacy)

#### Immediate Zeroization (REQUIRED)

```rust
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(ZeroizeOnDrop)]
struct SensitiveData {
    signal_id: String,
    pepper: Vec<u8>,
}

fn process_sensitive_data(mut data: SensitiveData) -> Hash {
    let hash = mask_identity(&data.signal_id, &data.pepper);
    
    // Explicit zeroization
    data.signal_id.zeroize();
    data.pepper.zeroize();
    
    hash
    // data dropped here, ZeroizeOnDrop ensures cleanup
}
```

#### Bot Storage (CRITICAL - Server Seizure Protection)

**Problem**: Default Presage SqliteStore persists ALL messages

**Threat**: Server seizure reveals vetting conversations and relationship context

**Solution**: Custom minimal ProtocolStore

```rust
use presage::Store;

pub struct StromaProtocolStore {
    // In-memory only (ephemeral)
    sessions: HashMap<ServiceId, Session>,
    pre_keys_cache: HashMap<u32, PreKey>,
    identity_keys: IdentityKeyPair,
    
    // Minimal encrypted file for restart (~100KB)
    encrypted_protocol_state: PathBuf,
    passphrase: SecureString,
    
    // NO message history
    // NO contact database
    // NO conversation content
}

impl Store for StromaProtocolStore {
    // Implement ONLY protocol requirements:
    // - get_session(), save_session()
    // - get_pre_key(), save_pre_key()
    // - get_identity_key()
    
    // DO NOT implement:
    // - save_message() ← Not needed
    // - get_messages() ← Not needed
    // - save_contact() ← Not needed
}
```

**Server Seizure Result:**
- Adversary gets: ~100KB encrypted file (protocol state only)
- Adversary does NOT get: Messages, conversations, Signal IDs, context

**Implementation:**
```rust
// ❌ FORBIDDEN
use presage_store_sqlite::SqliteStore;
let store = SqliteStore::open_with_passphrase(...).await?;

// ✅ REQUIRED
let store = StromaProtocolStore::new(encrypted_file, passphrase)?;
let manager = Manager::with_store(store, options).await?;
```

**Why This Wasn't Caught Earlier:**

Our security-guardrails.mdc focused on:
- Identity masking (application layer)
- Zeroization (memory layer)
- Operator privileges (access control layer)

**But missed:**
- Signal client's persistence layer
- Message history storage threat
- Difference between "protocol state" vs "message content"

**Gap**: Rules said "don't store Signal IDs" but didn't extend to "don't store message content" or specifically address what bot's Signal client persists.

**This gap is now fixed** in:
- `.beads/security-constraints.bead` section 10
- `.beads/technology-stack.bead`
- `.cursor/rules/security-guardrails.mdc`
- This document

### Threat Model

**Primary Threat**: Trust map seizure by state-level adversary or compromised operator

**Adversary Goal**: Obtain trust map to identify group members and their relationships

#### Attack Vectors & Defenses

**1. Trust Map Seizure Attempts**

**Attack**: Adversary compromises bot server, captures memory dump, or coerces operator to export trust map

**Three-Layer Defense**:

| Layer | Defense Mechanism | Result if Compromised |
|-------|------------------|----------------------|
| **No Centralized Storage** | Trust map in Freenet (distributed) | Adversary needs to seize multiple peers |
| **Cryptographic Privacy** | HMAC-hashed IDs, zeroization, minimal store | Memory/disk contain only hashes + protocol state |
| **Metadata Isolation** | 1-on-1 PMs, operator least-privilege, no message persistence | No conversations, operator can't export |

**Result**: Even if adversary compromises bot or server, they only get:
- Small encrypted file (~100KB) with Signal protocol state
- Hashes (not identities)
- Group size and topology (not relationship details)
- NO message history, NO vetting conversations, NO relationship context

**2. Compromised Operator**
   - Defense: Operator least privilege (service runner only)
   - Defense: All actions approved by Freenet contract
   - Defense: No access to cleartext Signal IDs
   - Defense: Cannot manually export or query trust map
   - Defense: No message history to access (minimal store)

**3. Server Seizure**
   - Defense: Custom minimal ProtocolStore (protocol state only, ~100KB)
   - Defense: NO message history persisted
   - Defense: NO vetting conversations stored
   - Defense: Encrypted protocol state file
   - Result: Adversary gets encrypted protocol state, NO conversation content

**3. Signal Metadata Analysis**
   - Defense: All operations in 1-on-1 PMs (no group chat metadata)
   - Defense: HMAC-hashed identifiers (different hashes per group)
   - Defense: No announcement of who vouched for whom

**4. Freenet Network Analysis**
   - Defense: Anonymous routing (dark mode, no IP exposure)
   - Defense: Encrypted state storage
   - Defense: Distributed storage (no single node has full map)

**5. State-Level Adversaries**
   - Defense: ZK-proofs (verify trust without revealing vouchers)
   - Defense: Post-quantum secure (STARKs, no trusted setup)
   - Defense: Decentralized (no single target to compromise)
   - Defense: Three-layer defense prevents useful seizure

**6. Sybil Attacks**
   - Defense: 2-vouch requirement from members in DIFFERENT CLUSTERS (cross-cluster mandatory)
   - Defense: Same-cluster vouches rejected (not optimization, hard requirement)
   - Defense: Immediate ejection if flagged

#### Out of Scope (Assumed Secure)

1. **Signal Protocol Compromise**: Assume Signal's E2E encryption is secure
2. **Freenet Protocol Vulnerabilities**: Assume Freenet's anonymous routing works
3. **Quantum Computing**: STARKs are post-quantum secure, HMAC-SHA256 is not (acceptable for now, can upgrade to SHA3)
4. **Physical Device Seizure**: Assume members protect their own Signal devices

## Performance Targets

### Scalability
- **Target**: 10²-10³ (100x to 1000x)
- **Method**: Federation (Phase 4+)
- **Per Group**: Up to Signal's limit (~1000 members)

### Latency
- **Philosophy**: Security > Speed
- **STARK Generation**: < 10 seconds
- **Merkle Tree Generation**: < 100ms for 1000 members
- **State Updates**: < 1 second
- **Ejection**: < 1 second (immediate)

### Proof Sizes
- **STARK Proofs**: < 100KB (validated in Spike Week)
- **Merkle Proofs**: < 10KB
- **Contract State**: Grows linearly with members (sets are efficient)

## Development Workflow

### Build

```bash
# Development build
cargo build

# Production build (static MUSL binary)
cargo build --release --target x86_64-unknown-linux-musl

# Check binary size
ls -lh target/x86_64-unknown-linux-musl/release/stroma
```

### Testing

```bash
# Run all tests
cargo nextest run

# Run with coverage
cargo llvm-cov nextest

# Property-based tests
cargo test --features proptest

# Security-specific tests
cargo test security::
```

### Linting & Formatting

```bash
# Format code
cargo fmt

# Clippy (strict)
cargo clippy -- -D warnings

# Check for common mistakes
cargo clippy -- -W clippy::all
```

### Security Audits

```bash
# Check dependencies for known vulnerabilities
cargo deny check

# Verify crate authenticity (web of trust)
cargo crev verify

# Audit specific dependency
cargo crev review ring
```

### Benchmarking

```bash
# Benchmark Merkle Tree generation (Q3)
cargo bench --bench merkle_tree

# Benchmark STARK proofs (Q1)
cargo bench --bench stark_proofs

# Profile with flamegraph
cargo flamegraph --bench merkle_tree
```

## Testing Requirements

### Unit Tests (100% Coverage Goal)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_vouch_invalidation() {
        let mut state = TrustNetworkState::new();
        
        // Add members
        state.add_member(alice);
        state.add_member(bob);
        
        // Alice vouches for Bob
        state.add_vouch(bob, alice);
        state.add_vouch(bob, carol);
        assert_eq!(state.calculate_effective_vouches(&bob), 2);
        
        // Alice flags Bob (invalidates her vouch)
        state.add_flag(bob, alice);
        assert_eq!(state.calculate_effective_vouches(&bob), 1); // Vouch invalidated
        assert!(state.should_eject(&bob)); // Ejected (Trigger 2)
    }
}
```

### Property-Based Tests (Cryptographic Guarantees)

```rust
#[cfg(test)]
mod property_tests {
    use proptest::prelude::*;
    
    proptest! {
        #[test]
        fn test_different_pepper_different_hash(
            signal_id in ".*",
            pepper1 in prop::collection::vec(any::<u8>(), 16..32),
            pepper2 in prop::collection::vec(any::<u8>(), 16..32),
        ) {
            let hash1 = mask_identity(&signal_id, &pepper1);
            let hash2 = mask_identity(&signal_id, &pepper2);
            
            // Same ID with different pepper MUST produce different hashes
            if pepper1 != pepper2 {
                assert_ne!(hash1, hash2);
            }
        }
    }
}
```

### Integration Tests (Async Behavior)

```rust
#[tokio::test]
async fn test_admission_flow() {
    let freenet = MockFreenetClient::new();
    let signal = MockSignalClient::new();
    
    // Invite
    process_invite(alice, bob, &freenet, &signal).await.unwrap();
    
    // First vouch recorded
    assert_eq!(freenet.vouch_count(bob), 1);
    
    // Second vouch
    process_vouch(bob, carol, &freenet, &signal).await.unwrap();
    
    // Should trigger admission
    assert_eq!(freenet.vouch_count(bob), 2);
    assert!(signal.is_member(bob).await);
}
```

### Security Tests (Memory Hygiene)

```rust
#[test]
fn test_no_cleartext_in_memory_dump() {
    let signal_id = "alice_signal_id";
    let pepper = b"secret_pepper";
    
    let hash = mask_identity(signal_id, pepper);
    
    // Simulate memory dump
    let memory_dump = capture_memory_dump();
    
    // MUST NOT contain cleartext Signal ID
    assert!(!memory_dump.contains(signal_id));
    
    // Should contain only hash
    assert!(memory_dump.contains(&hash.to_string()));
}
```

## CI/CD Pipeline

### Required Checks (Must Pass)

```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.93
      - run: cargo test --all-features
      
  clippy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.93
      - run: cargo clippy -- -D warnings
      
  security:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo install cargo-deny
      - run: cargo deny check
      
  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo install cargo-llvm-cov
      - run: cargo llvm-cov nextest --html
```

## Outstanding Questions (Spike Week)

These 5 questions MUST be answered before Phase 0:

### Q1: STARK Verification in Wasm
**Question**: Can we verify STARK proofs in contract `verify()` without performance issues?

**Test Plan**:
- Compile winterfell to wasm32-unknown-unknown
- Measure verification time in Wasm context
- Target: < 100ms per proof

**Decision**: Client-side vs contract-side verification

### Q2: Proof Storage Strategy
**Question**: Should we store STARK proofs in contract state or just outcomes?

**Options**:
- A: Temporary storage (removed after verification)
- B: Permanent storage (audit trail)
- C: No storage (trust bot verification)

**Impact**: Storage costs, trustlessness, audit trail

### Q3: Merkle Tree Performance
**Question**: How expensive is on-demand Merkle Tree generation?

**Test Plan**:
- Benchmark with 10, 100, 500, 1000 members
- Target: < 100ms for 1000 members

**Decision**: On-demand vs caching Merkle root

### Q4: Conflict Resolution
**Question**: How does Freenet handle merge conflicts?

**Test Plan**:
- Create divergent states on two nodes
- Observe merge behavior
- Document conflict resolution semantics

**Impact**: May need vector clocks or causal ordering

### Q5: Custom Validation
**Question**: Can we enforce complex invariants in `verify()`?

**Test Plan**:
- Implement complex validation ("every member >= 2 vouches")
- Test if verify() can reject invalid states

**Decision**: Contract-enforced vs bot-enforced invariants

**See**: [Spike Week Briefing](SPIKE-WEEK-BRIEFING.md) for complete test plans

## Development Standards

### Code Style

Follow Rust community standards:
- **rustfmt**: Default configuration
- **clippy**: All warnings as errors
- **Documentation**: All public APIs documented
- **Testing**: 100% coverage goal (minimum 80%)

**See**: `.cursor/rules/rust-standards.mdc` for complete standards

### Async Patterns

Use tokio best practices:
- CPU-intensive work via `spawn_blocking` (STARK proofs)
- I/O operations async (Signal, Freenet)
- Graceful shutdown via tokio CancellationToken
- Structured concurrency (no detached tasks)

**See**: `.cursor/rules/rust-async.mdc` for async patterns

### Error Handling

```rust
// ✅ Use Result types
pub async fn add_member(member: Hash) -> Result<(), MembershipError> {
    // ...
}

// ✅ Define custom error types
#[derive(Debug, thiserror::Error)]
pub enum MembershipError {
    #[error("Member {0} already exists")]
    AlreadyExists(Hash),
    
    #[error("Insufficient vouches: {0} < {1}")]
    InsufficientVouches(usize, usize),
}

// ❌ Don't panic in production code
pub fn risky_operation() {
    let value = might_fail().unwrap(); // ❌ BAD
}
```

### Logging

```rust
use tracing::{info, warn, error, debug};

// ✅ Structured logging
info!(member = %member_hash, "Member admitted to group");

// ✅ Log hashes, not cleartext
warn!(member = %member_hash, standing = -1, "Member ejected");

// ❌ Never log cleartext Signal IDs
error!("Failed to add {}", signal_id); // ❌ BAD - leaks identity
```

## Git Workflow

### Commits by AI Agents

**All commits authored by Claude must include:**

```bash
git commit -m "$(cat <<'EOF'
Add vouch invalidation logic to trust model

- Voucher-flaggers invalidate their own vouches
- Prevents logical inconsistency
- Aligns with fluid identity philosophy

Co-authored-by: Claude <claude@anthropic.com>
EOF
)"
```

**See**: `.beads/security-constraints.bead` for complete git standards

### Branching

- `main` - Stable, deployable code
- `develop` - Integration branch
- `feature/*` - New features
- `fix/*` - Bug fixes
- `spike/*` - Experimental validation (Spike Week)

## Gastown Coordination

This project uses **Gastown** - multi-agent coordination with specialized roles:

- **Mayor**: Coordinates agents, assigns Beads (tasks)
- **Witness**: Reviews code for security violations
- **Specialists**: Domain-focused agents (Freenet, Signal, Crypto, etc.)

**See**: [AGENTS.md](../AGENTS.md) for complete agent coordination model

## Contributing

### Before Contributing

1. Read [TODO.md](TODO.md) for current tasks
2. Check Spike Week status (are Outstanding Questions answered?)
3. Review `.beads/` for immutable constraints
4. Review `.cursor/rules/` for development standards

### Making Changes

1. Create feature branch
2. Implement with tests (100% coverage)
3. Run security checks (`cargo deny check`)
4. Ensure no cleartext ID leakage
5. Add Co-authored-by if using Claude
6. Submit PR with detailed description

### PR Requirements

- ✅ All tests pass
- ✅ `cargo clippy` passes with no warnings
- ✅ `cargo deny check` passes
- ✅ `cargo fmt` applied
- ✅ Documentation updated
- ✅ No cleartext Signal IDs in code/tests

## Resources

### Internal Documentation
- [Algorithms](ALGORITHMS.md) - **Matchmaking algorithms & cryptographic protocols**
- [User Guide](USER-GUIDE.md) - For group members
- [Operator Guide](OPERATOR-GUIDE.md) - For bot administrators
- [Trust Model](TRUST-MODEL.md) - Mathematical details
- [Federation Roadmap](FEDERATION.md) - Phase 4+ vision
- [Spike Week Briefing](SPIKE-WEEK-BRIEFING.md) - Technology validation
- [TODO Checklist](TODO.md) - Implementation tasks

### Constraint Beads (Immutable)
- [Security Constraints](../.beads/security-constraints.bead)
- [Architecture Decisions](../.beads/architecture-decisions.bead)
- [Federation Roadmap](../.beads/federation-roadmap.bead)

### Development Rules
- [Rust Standards](../.cursor/rules/rust-standards.mdc)
- [Rust Async Patterns](../.cursor/rules/rust-async.mdc)
- [Freenet Contract Design](../.cursor/rules/freenet-contract-design.mdc)
- [User Roles & UX](../.cursor/rules/user-roles-ux.mdc)
- [Security Guardrails](../.cursor/rules/security-guardrails.mdc)

### External References
- [freenet-core](https://github.com/freenet/freenet-core) - State storage
- [freenet-scaffold](https://github.com/freenet/freenet-scaffold) - Contract utilities
- [winterfell](https://github.com/facebook/winterfell) - STARK proofs
- [libsignal-service-rs](https://github.com/whisperfish/libsignal-service-rs) - Signal integration

---

**Status**: Early development (Spike Week). Ready for technology validation phase.

**Last Updated**: 2026-01-27
