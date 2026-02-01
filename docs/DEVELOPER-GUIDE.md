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
- **Embedded Freenet Node** (`freenet` crate) - In-process, not external service
- **Contract Development** (`freenet-stdlib`) - ContractInterface trait for Wasm contracts
- **STARKs** (winterfell) - No trusted setup, post-quantum secure
- **On-Demand Merkle Trees** - Generated from BTreeSet for ZK-proofs (not stored)
- **Commutative Deltas** - Contract's responsibility (Q1 validated) - set-based state with tombstones
- **Contract Validation** - Trustless model (Q2 validated) - `update_state()` and `validate_state()` can reject invalid deltas/state
- **Vouch Invalidation** - Logical consistency (can't both trust and distrust)
- **Minimum Spanning Tree** - Optimal mesh topology with maximum anonymity (see [ALGORITHMS.md](ALGORITHMS.md))

## Technical Stack

### Core Technologies

| Component | Library/Version | Purpose |
|-----------|----------------|---------|
| **Language** | Rust 1.93+ | musl 1.2.5, improved DNS, memory safety |
| **Embedded Node** | [freenet](https://docs.rs/freenet/latest/freenet/) v0.1.107+ | In-process node (NodeConfig::build()) |
| **Contract Framework** | [freenet-stdlib](https://docs.rs/freenet-stdlib) v0.1.30+ | Wasm contracts (ComposableState trait) |
| **Contract SDK** | [freenet-stdlib](https://docs.rs/freenet-stdlib) v0.1+ | ContractInterface trait, Wasm contract development |
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
│   ├── hmac.rs                      # HMAC-based hashing with ACI-derived key
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
│   ├── graph_analysis.rs            # Topology analysis (Bridge Removal, centrality, DVR)
│   ├── cluster_detection.rs         # Identify internal clusters (Tarjan's algorithm, Q3 validated)
│   └── strategic_intro.rs           # DVR optimization + MST fallback (see ALGORITHMS.md, blind-matchmaker-dvr.bead)
├── config/                          # Group Configuration
│   ├── mod.rs
│   └── group_config.rs              # GroupConfig struct (Freenet contract)
├── persistence/                     # Reciprocal Persistence Network
│   ├── mod.rs                       # Public API
│   ├── encryption.rs                # AES-256-GCM, Ed25519 signatures
│   ├── chunking.rs                  # Split/join encrypted state into 64KB chunks
│   ├── registry.rs                  # Persistence peer discovery
│   ├── verification.rs              # Challenge-response verification (Q13)
│   ├── recovery.rs                  # State recovery from chunks
│   └── write_blocking.rs            # State machine (ACTIVE/DEGRADED/etc.)
└── federation/                      # Federation Logic (DISABLED IN MVP)
    ├── mod.rs                       # Feature flag: #[cfg(feature = "federation")]
    ├── shadow_beacon.rs             # Social Anchor Hashing (Phase 4+)
    ├── psi_ca.rs                    # Private Set Intersection (Phase 4+)
    ├── diplomat.rs                  # Federation proposals (Phase 4+)
    └── shadow_handover.rs           # Bot identity rotation (Phase 4+)
```

**Key Design**: `federation/` exists but is disabled via feature flag in MVP (validates architecture scales).
**Key Design**: `persistence/` ensures trust state durability even if Freenet data falls off.

**See**: [ALGORITHMS.md](ALGORITHMS.md) for detailed MST algorithm, PSI-CA protocol, and complexity analysis.

### Future: Shadow Handover (Phase 4+)

The `shadow_handover.rs` module will implement cryptographic bot identity rotation:

- **Purpose**: Allow bot to rotate Signal phone number while preserving trust context
- **Mechanism**: Succession Document signed by old bot key, validated by Freenet contract
- **Use Cases**: Signal ban recovery, periodic rotation, operational security

See `.beads/federation-roadmap.bead` for protocol specification.

## Two-Layer Architecture (Trust State + Persistence)

Stroma uses a two-layer architecture to ensure trust state durability:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  LAYER 1: TRUST STATE (Freenet-native)                                       │
│  ──────────────────────────────────────                                       │
│  Storage: BTreeSet (members), HashMap (vouches, flags) - mergeable           │
│  Sync: Native Freenet ComposableState (Q1 validated)                         │
│  Updates: Small deltas (~100-500 bytes) - INFREQUENT (human timescale)       │
│  Security: Contract validates via update_state() + validate_state() (Q2)     │
│                                                                              │
│  LAYER 2: PERSISTENCE CHUNKS (Reciprocal Persistence Network)               │
│  ──────────────────────────────────────────────────────────────              │
│  Purpose: Durability against Freenet data loss, server seizure protection   │
│  Method: Encrypt full state, chunk into 64KB pieces, replicate 3x each      │
│  Distribution: Deterministic per-chunk (rendezvous hashing, zero trust)     │
│  Frequency: Same as trust state updates (infrequent)                        │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Why Two Layers?

**Layer 1 Problem**: Freenet data falls off if no peers are subscribed.

**Layer 2 Solution**: Bots replicate each other's encrypted chunks so any bot can recover after crash.

### Key Persistence Types

```rust
/// Encrypted trust state ready for chunking
pub struct EncryptedTrustNetworkState {
    ciphertext: Vec<u8>,           // AES-256-GCM (key from Signal ACI)
    signature: Vec<u8>,            // Signed with Signal ACI identity key
    bot_pubkey: Vec<u8>,           // Signal ACI public key
    member_merkle_root: Hash,      // Public for ZK-proofs
    version: u64,                  // Monotonic, anti-replay
    previous_hash: Hash,           // Chain integrity
    timestamp: Timestamp,
}
// Note: No separate keypair file - uses Signal ACI identity from protocol store

/// A single chunk of encrypted state (Q12: 64KB constant)
pub const CHUNK_SIZE: usize = 64 * 1024; // 64KB

pub struct Chunk {
    data: Vec<u8>,                 // 64KB of encrypted data (CHUNK_SIZE)
    chunk_index: u32,              // Position in sequence
    chunk_hash: Hash,              // SHA-256 for integrity (Q9)
    version: u64,                  // Must match other chunks
}

/// Registry entry for bot discovery (Q7)
pub struct RegistryEntry {
    bot_pubkey: PublicKey,
    num_chunks: u32,               // state_size.div_ceil(CHUNK_SIZE)
    size_bucket: SizeBucket,
    registered_at: Timestamp,
    contract_hash: Hash,
    pow_proof: RegistrationProof,  // Difficulty 18 (Q8)
}
```

### Adversarial Peer Model

ALL persistence peers are treated as adversaries:
- Cannot read trust map (AES-256-GCM encrypted)
- Cannot reconstruct state (need ALL chunks + ACI key)
- Can compute whose chunks they hold (deterministic assignment)
- Security comes from encryption, not obscurity

### Subscription Layer: Two Separate Concerns

**CRITICAL:** Outbound (fairness) and Inbound (security) subscriptions are SEPARATE:

| Subscription Type | Purpose | Selection | Registry |
|-------------------|---------|-----------|----------|
| **OUTBOUND** | I hold others' fragments | Comparable-size (fairness) | PUBLIC accounting |
| **INBOUND** | Others hold MY fragments | RANDOM (security) | ENCRYPTED (only I decrypt) |

**Why Separate:**
- Bot-B (whose fragments I hold) ≠ holder of MY fragments
- Correlating these would leak network topology
- Maximum collusion resistance

### Contract Authority Models

| Contract Type | Authority Model | Rationale |
|--------------|-----------------|-----------|
| Trust Map | Single-writer (bot) | Core trust graph |
| Federation | Single-writer (each side) | Each group records own state |
| Replication | Single-writer + shared validation | Bot authority, peers validate |
| Registry | Shared (distributed) | Handles stale bots |

See [PERSISTENCE.md](PERSISTENCE.md) for full architecture and recovery procedures.

### Persistence Implementation Guidance (Spike Week 2)

**Validated Parameters and Protocols:**

#### Bot Discovery (Q7)
```rust
// Well-known registry contract address
const PERSISTENCE_REGISTRY: ContractHash =
    hash("stroma-persistence-registry-v1");

impl Bot {
    pub async fn register_for_persistence(&self) -> Result<(), Error> {
        let entry = RegistryEntry {
            bot_pubkey: self.pubkey,
            num_chunks: self.state_size / CHUNK_SIZE,
            size_bucket: self.compute_size_bucket(),
            registered_at: current_timestamp(),
            contract_hash: self.contract_hash,
            pow_proof: self.generate_pow_proof(DIFFICULTY_18), // Q8
        };

        freenet.update(PERSISTENCE_REGISTRY, |state| {
            state.register(entry)
        }).await
    }
}
```

**See**: [Q7 Results](spike/q7/RESULTS.md) for discovery protocol details

#### Proof of Work Registration (Q8)
```rust
// Production difficulty: 18 (requirement from Q8)
const POW_DIFFICULTY: u32 = 18;

pub struct RegistrationProof {
    nonce: u64,
    timestamp: u64,
    difficulty: u32,
}

impl RegistrationProof {
    pub fn generate(bot_pubkey: &PublicKey) -> Self {
        // ~100ms registration time on standard hardware
        // ~2 minutes for 1000 fake bots (Sybil defense)
        let mut nonce = 0;
        loop {
            let hash = hash(&format!("{:?}{}", bot_pubkey, nonce));
            if hash_meets_difficulty(&hash, POW_DIFFICULTY) {
                return Self { nonce, timestamp: now(), difficulty: POW_DIFFICULTY };
            }
            nonce += 1;
        }
    }

    pub fn verify(&self, bot_pubkey: &PublicKey) -> bool {
        let hash = hash(&format!("{:?}{}", bot_pubkey, self.nonce));
        hash_meets_difficulty(&hash, self.difficulty)
    }
}
```

**Combined Defense**: PoW + Reputation (7-day minimum) + Capacity verification (100MB)

**See**: [Q8 Results](spike/q8/RESULTS.md) for complete Sybil defense strategy

#### Chunk Verification (Q9)
```rust
// Challenge-response protocol for verifying chunk possession
pub struct VerificationChallenge {
    nonce: [u8; 32],        // Random, prevents replay
    offset: u32,            // Where to read in chunk
    length: u32,            // Sample size (typically 64 bytes)
    timestamp: u64,         // Unix timestamp for freshness
}

pub struct VerificationResponse {
    hash: [u8; 32],         // SHA-256(nonce || chunk[offset..offset+length])
    challenge: VerificationChallenge,
}

impl Bot {
    pub async fn verify_chunk_holder(
        &self,
        holder: &PublicKey,
        chunk_idx: u32
    ) -> Result<bool, Error> {
        let challenge = VerificationChallenge {
            nonce: random_nonce(),
            offset: random_offset(CHUNK_SIZE),
            length: 64,
            timestamp: now(),
        };

        let response = holder.send_challenge(challenge).await?;
        let expected = self.compute_expected_response(&challenge, chunk_idx);

        Ok(response.hash == expected)
    }
}
```

**Protocol overhead**: 128 bytes per challenge (48 challenge + 80 response)

**See**: [Q9 Results](spike/q9/RESULTS.md) for verification protocol details

#### Rendezvous Hashing (Q11)
```rust
/// Deterministic chunk holder selection via HRW (Highest Random Weight)
pub fn compute_chunk_holders(
    owner: &BotId,
    chunk_idx: u32,
    network_bots: &[BotId],
    epoch: Epoch,
    replicas: usize,
) -> Vec<BotId> {
    let mut scores: Vec<(BotId, u64)> = network_bots
        .iter()
        .map(|candidate| {
            let score = hash_score(owner, chunk_idx, candidate, epoch);
            (*candidate, score)
        })
        .collect();

    // Select top-N scoring candidates
    scores.sort_by(|a, b| b.1.cmp(&a.1));
    scores.into_iter().take(replicas).map(|(bot, _)| bot).collect()
}

fn hash_score(owner: &BotId, chunk_idx: u32, candidate: &BotId, epoch: Epoch) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    owner.hash(&mut hasher);
    chunk_idx.hash(&mut hasher);
    candidate.hash(&mut hasher);
    epoch.hash(&mut hasher);
    hasher.finish()
}
```

**Properties**: Deterministic, uniform distribution, minimal churn on bot join/leave

**See**: [Q11 Spike](spike/q11/main.rs) for algorithm validation

#### Chunk Size (Q12)
```rust
/// 64KB chunk size provides optimal balance
/// - Low overhead: 0.2% (vs 9.8% for 1KB)
/// - Acceptable distribution: 32% of network (100 bots)
/// - Simple bookkeeping: ~24 chunks per bot (512KB state)
pub const CHUNK_SIZE: usize = 64 * 1024; // 64KB

pub fn chunk_state(state: &[u8]) -> Vec<Vec<u8>> {
    state.chunks(CHUNK_SIZE).map(|c| c.to_vec()).collect()
}

pub fn num_chunks(state_size: usize) -> usize {
    state_size.div_ceil(CHUNK_SIZE)
}
```

**Alternative**: 16KB for high-security scenarios (82.5% distribution, 0.6% overhead)

**See**: [Q12 Results](spike/q12/RESULTS.md) for chunk size analysis

#### Spot Check Verification (Q13)
```rust
/// Verify 1% sample of holders before each write
pub async fn verify_before_write(
    owner: &Bot,
    chunks: &[Chunk]
) -> Result<()> {
    let all_holders: Vec<_> = chunks
        .iter()
        .flat_map(|chunk| chunk.get_holders())
        .collect();

    // Sample 1% (minimum 1)
    let sample_size = (all_holders.len() as f64 * 0.01).max(1.0) as usize;
    let sample = all_holders.choose_multiple(&mut rng, sample_size);

    for holder in sample {
        let challenge = ChunkChallenge::new(
            owner.id,
            holder.chunk_idx,
            CHUNK_SIZE
        );
        let response = holder.send_challenge(challenge).await?;

        if !response.verify(&challenge, &chunks[holder.chunk_idx]) {
            warn!("Holder {} failed verification", holder.id);
            mark_suspicious(holder.id);
        }
    }

    Ok(())
}
```

**Overhead**: ~0.16ms per write (negligible)

**See**: [Q13 Results](spike/q13/RESULTS.md) for fairness verification protocol

#### Chunk Distribution Protocol (Q14)
```rust
// Phase 0: Contract-based distribution (simple, proven)
pub async fn distribute_via_contract(
    holder: &BotId,
    chunk: &Chunk,
) -> Result<DistributionAttestation> {
    let chunk_contract = holder.chunk_contract_address(chunk.index);
    freenet.put(chunk_contract, chunk.data.clone()).await?;

    Ok(DistributionAttestation {
        holder: *holder,
        chunk_hash: hash(&chunk.data),
        timestamp: now(),
    })
}

// Phase 1+: Hybrid P2P + attestation (5x faster, 9x cheaper)
pub async fn distribute_hybrid(
    holder: &BotId,
    chunk: &Chunk,
) -> Result<DistributionAttestation> {
    // P2P transfer (bulk data)
    p2p_network.send_chunk(holder, chunk).await?;

    // Attestation write (small metadata)
    let attestation = DistributionAttestation {
        holder: *holder,
        chunk_hash: hash(&chunk.data),
        timestamp: now(),
    };
    freenet.put_attestation(&attestation).await?;

    Ok(attestation)
}
```

**Phase 0**: ~1.6s, 160 units per 512KB update (acceptable for infrequent updates)
**Phase 1+**: ~320ms, 18 units per 512KB update (9x cost reduction)

**See**: [Q14 Results](spike/q14/RESULTS.md) for protocol comparison

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
use freenet_stdlib::prelude::*;
use serde::{Serialize, Deserialize};
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TrustNetworkState {
    // Core membership (set-based, commutative)
    pub members: BTreeSet<MemberHash>,
    pub ejected: BTreeSet<MemberHash>,  // Can return (not permanent ban)
    
    // Trust graph (set-based, commutative)
    pub vouches: HashMap<MemberHash, HashSet<MemberHash>>,
    pub flags: HashMap<MemberHash, HashSet<MemberHash>>,
    
    // Configuration
    pub config: GroupConfigV1,
    pub schema_version: u64,
    
    // Federation hooks (Phase 4+, disabled in MVP)
    #[serde(default)]
    pub federation_contracts: Vec<ContractHash>,
}
```

**Note**: `freenet-scaffold` is outdated. Use `freenet-stdlib` for contract development.

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

### Blind Matchmaker: DVR-Optimized Algorithm

The bot suggests strategic introductions using a **hybrid algorithm**:

**Phase 0: DVR Optimization** (Priority)
- Tracks vouchers already used by existing distinct Validators
- Suggests vouchers NOT in any distinct Validator's voucher set
- Goal: Maximize Distinct Validator Ratio (independent verification)

**Phase 1: MST Fallback**
- If no DVR-optimal voucher available, use any cross-cluster Validator
- Still valid, just not optimal for network health

```rust
pub fn suggest_introduction(&self, bridge: Hash) -> Option<Introduction> {
    // Phase 0: Try DVR-optimal first
    if let Some(intro) = self.suggest_dvr_optimal(bridge) {
        return Some(intro);
    }
    
    // Phase 1: Fall back to MST
    self.suggest_mst_fallback(bridge)
}

fn suggest_dvr_optimal(&self, bridge: Hash) -> Option<Introduction> {
    let used_vouchers = self.collect_distinct_validator_vouchers();
    let bridge_cluster = self.find_cluster(bridge);
    
    // Find voucher that:
    // 1. Is in different cluster
    // 2. Hasn't been used by another distinct Validator
    self.validators()
        .filter(|v| self.find_cluster(*v) != bridge_cluster)
        .filter(|v| !used_vouchers.contains(v))
        .max_by_key(|v| self.centrality(*v))
        .map(|voucher| Introduction {
            person_a: bridge,
            person_b: voucher,
            reason: "Create distinct Validator (DVR optimization)",
            dvr_optimal: true,
        })
}
```

**See**: 
- `.beads/blind-matchmaker-dvr.bead` for full algorithm
- `.beads/mesh-health-metric.bead` for DVR metric
- `docs/ALGORITHMS.md` for mathematical details

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
    
    // Initialize embedded Freenet node (Q1 validated: use freenet crate)
    // See: spike/q1/RESULTS.md for entry point documentation
    let mut node_config = freenet::local_node::NodeConfig {
        should_connect: true,
        is_gateway: false,
        key_pair: load_or_generate_keypair(&config)?,
        network_listener_ip: "0.0.0.0".parse()?,
        network_listener_port: 0,  // OS assigns port
        ..Default::default()
    };
    node_config.add_gateway(config.freenet.gateway.clone());
    
    // Build node with client proxy for programmatic interaction
    let node = node_config.build([client_proxy]).await?;
    let shutdown = node.shutdown_handle();
    
    // Load existing contract
    let contract_key = config.freenet.contract_key;
    
    // Initialize Signal bot
    let signal = SignalBot::authenticate(&config.signal).await?;
    
    // Event loop (single process handles both Freenet and Signal)
    loop {
        tokio::select! {
            // Freenet state changes (via client proxy)
            Some(state_change) = state_stream.next() => {
                handle_state_change(state_change, &signal).await?;
            }
            
            // Signal messages
            Some(message) = signal.recv_message() => {
                handle_signal_command(message, &node).await?;
            }
            
            // Periodic health check
            _ = health_check_interval.tick() => {
                check_all_trust_standings(&node, &signal, contract_key).await?;
            }
            
            // Graceful shutdown
            _ = shutdown_signal() => {
                shutdown.shutdown();
                break;
            }
        }
    }
    Ok(())
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
use hkdf::Hkdf;
use sha2::Sha256;
use libsignal_protocol::IdentityKeyPair;

/// Derive HMAC key from Signal ACI identity (replaces group pepper)
fn derive_identity_masking_key(aci_identity: &IdentityKeyPair) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(
        Some(b"stroma-identity-masking-v1"),
        aci_identity.private_key().serialize().as_slice()
    );
    let mut key = [0u8; 32];
    hk.expand(b"hmac-sha256-key", &mut key).unwrap();
    key
}

pub fn mask_identity(signal_id: &str, aci_identity: &IdentityKeyPair) -> Hash {
    // Use HMAC-SHA256 with ACI-derived key (NOT deterministic hashing)
    let key_bytes = derive_identity_masking_key(aci_identity);
    let key = hmac::Key::new(hmac::HMAC_SHA256, &key_bytes);
    let tag = hmac::sign(&key, signal_id.as_bytes());
    
    Hash::from_bytes(tag.as_ref())
    
    // signal_id is borrowed, but owned data must be zeroized:
    // signal_id_owned.zeroize();
}
```

**Critical**: Different bots → different hashes for same person (enables PSI-CA privacy). All crypto keys derived from Signal ACI identity.

#### Immediate Zeroization (REQUIRED)

```rust
use zeroize::{Zeroize, ZeroizeOnDrop};
use libsignal_protocol::IdentityKeyPair;

#[derive(ZeroizeOnDrop)]
struct SensitiveData {
    signal_id: String,
}

fn process_sensitive_data(mut data: SensitiveData, aci_identity: &IdentityKeyPair) -> Hash {
    let hash = mask_identity(&data.signal_id, aci_identity);
    
    // Explicit zeroization
    data.signal_id.zeroize();
    
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
    use libsignal_protocol::IdentityKeyPair;
    
    proptest! {
        #[test]
        fn test_different_aci_different_hash(
            signal_id in ".*",
        ) {
            // Different ACI identities MUST produce different hashes
            let aci1 = IdentityKeyPair::generate(&mut rand::thread_rng());
            let aci2 = IdentityKeyPair::generate(&mut rand::thread_rng());
            
            let hash1 = mask_identity(&signal_id, &aci1);
            let hash2 = mask_identity(&signal_id, &aci2);
            
            // Same ID with different ACI identity MUST produce different hashes
            assert_ne!(hash1, hash2);
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
    let aci_identity = IdentityKeyPair::generate(&mut rand::thread_rng());
    
    let hash = mask_identity(signal_id, &aci_identity);
    
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

### Spike Week 1 (Q1-Q6) — ✅ ALL COMPLETE

All Spike Week 1 questions answered. Trust network primitives validated.

### Spike Week 2 (Q7-Q14) — ✅ ALL COMPLETE

All Spike Week 2 questions answered. Persistence network validated with concrete parameters:

**Validated Parameters:**
- **PoW Difficulty**: 18 (production), ~100ms registration
- **Chunk Size**: 64KB (0.2% overhead, 32% distribution)
- **Verification**: Challenge-response, 128 bytes overhead
- **Spot Check Rate**: 1% sample per write
- **Distribution**: Contract-based (Phase 0), Hybrid P2P (Phase 1+)
- **Holder Selection**: Rendezvous hashing (deterministic, uniform)

**Implementation Status**: Ready for Phase 0 development

**See**: [SPIKE-WEEK-2-BRIEFING.md](spike/SPIKE-WEEK-2-BRIEFING.md) for complete analysis

### Q1: Freenet Conflict Resolution — ✅ COMPLETE (GO)
**Answer**: Freenet applies all deltas via commutative set union. Use set-based state (BTreeSet) with tombstones.

**See**: [spike/q1/RESULTS.md](spike/q1/RESULTS.md)

### Q2: Contract Validation — ✅ COMPLETE (GO)
**Answer**: Contracts CAN enforce trust invariants via `update_state()` and `validate_state()`. Trustless model viable.

**See**: [spike/q2/RESULTS.md](spike/q2/RESULTS.md)

### Q3: Cluster Detection — ✅ COMPLETE (GO)
**Answer**: Bridge Removal algorithm (Tarjan's) distinguishes tight clusters. Standard Union-Find fails (1 cluster), Bridge Removal correctly separates A, B, and bridge Charlie.

**See**: [spike/q3/RESULTS.md](spike/q3/RESULTS.md)

### Q4: STARK Verification in Wasm — ✅ COMPLETE (PARTIAL)
**Answer**: winterfell Wasm is experimental. **Bot-side verification** for Phase 0 (native winterfell). Can migrate to contract-side when Wasm improves.

**See**: [spike/q4/RESULTS.md](spike/q4/RESULTS.md)

### Q5: Merkle Tree Performance — ✅ COMPLETE (GO)
**Answer**: 1000 members = **0.09ms** (1000x faster than threshold). **Generate on demand** — no caching needed.

| Members | Root (ms) |
|---------|-----------|
| 100 | 0.01 |
| 1000 | 0.09 |
| 5000 | 0.45 |

**See**: [spike/q5/RESULTS.md](spike/q5/RESULTS.md)

### Q6: Proof Storage Strategy — ✅ COMPLETE
**Answer**: **Store outcomes only** (not proofs). Proofs are ephemeral (10-100KB). Contract stores "Alice vouched for Bob", not the proof.

**See**: [spike/q6/RESULTS.md](spike/q6/RESULTS.md)

### Q7: Bot Discovery — ✅ COMPLETE (GO)
**Answer**: Registry contract at well-known address (`hash("stroma-persistence-registry-v1")`). Discovery < 1ms, registration ~100 bytes overhead.

**Implementation**: Single registry for < 10K bots, shard at scale.

**See**: [spike/q7/RESULTS.md](spike/q7/RESULTS.md)

### Q8: Fake Bot Defense — ✅ COMPLETE (GO)
**Answer**: Multi-layer defense: **PoW (difficulty 18)** + **Reputation (7-day minimum)** + **Capacity verification (100MB)**. Detection rate > 90%, false positive < 1%.

**Cost**: Attacker needs 7 days + 100GB storage + operational infrastructure for 1000 fake bots.

**See**: [spike/q8/RESULTS.md](spike/q8/RESULTS.md)

### Q9: Chunk Verification — ✅ COMPLETE (GO)
**Answer**: Challenge-response with SHA-256. Holder proves possession by computing `hash(nonce || chunk_sample)`. Protocol overhead: 128 bytes, latency < 1ms.

**Security**: Replay-resistant, content-private, cryptographically sound.

**See**: [spike/q9/RESULTS.md](spike/q9/RESULTS.md)

### Q11: Rendezvous Hashing — ✅ COMPLETE (GO)
**Answer**: HRW (Highest Random Weight) algorithm for deterministic holder selection. `score = hash(owner || chunk_idx || candidate || epoch)`. Select top-N scoring candidates.

**Properties**: Deterministic, uniform distribution, minimal churn, zero-trust.

**See**: [spike/q11/main.rs](spike/q11/main.rs)

### Q12: Chunk Size — ✅ COMPLETE (GO)
**Answer**: **64KB chunks** optimal balance. Coordination overhead 0.2%, distribution 32% of network (100 bots). Alternative: 16KB for 82.5% distribution, 0.6% overhead.

**Rationale**: Low overhead, simple bookkeeping, scales to large states.

**See**: [spike/q12/RESULTS.md](spike/q12/RESULTS.md)

### Q13: Spot Check Verification — ✅ COMPLETE (GO)
**Answer**: **1% sample rate** before each write. Challenge-response verification with 256-byte samples. Overhead ~0.16ms per write, detection rate > 95%, false positive < 1%.

**Phase 1+**: Add reputation scoring, increase to 5% if free-riding detected.

**See**: [spike/q13/RESULTS.md](spike/q13/RESULTS.md)

### Q14: Chunk Distribution Protocol — ✅ COMPLETE (GO)
**Answer**: **Contract-based for Phase 0** (simple, proven). **Hybrid P2P for Phase 1+** (5x faster, 9x cheaper).

**Phase 0**: ~1.6s, 160 units per 512KB update
**Phase 1+**: ~320ms, 18 units per 512KB update

**See**: [spike/q14/RESULTS.md](spike/q14/RESULTS.md)

### Summary: Spike Week 1 (Q1-Q6) - Proceed to Phase 0

| Question | Decision | Implementation |
|----------|----------|----------------|
| Q1: Conflict | GO | Set-based CRDT |
| Q2: Validation | GO | Trustless contract |
| Q3: Clusters | GO | Bridge Removal |
| Q4: STARK | PARTIAL | Bot-side |
| Q5: Merkle | GO | On-demand |
| Q6: Storage | Outcomes | No proof storage |

**See**: [Spike Week Briefing](spike/SPIKE-WEEK-BRIEFING.md) for Spike Week 1 analysis

### Summary: Spike Week 2 (Q7-Q14) - Persistence Network Ready

| Question | Decision | Implementation |
|----------|----------|----------------|
| Q7: Discovery | GO | Registry contract |
| Q8: Sybil Defense | GO | PoW 18 + Reputation |
| Q9: Verification | GO | Challenge-response |
| Q11: Hashing | GO | Rendezvous (HRW) |
| Q12: Chunk Size | GO | 64KB chunks |
| Q13: Spot Check | GO | 1% sample rate |
| Q14: Protocol | GO | Contract (Phase 0) |

**Validated Constants**:
- PoW difficulty: **18** (production)
- Chunk size: **64KB** (constant)
- Verification sample: **1%** per write
- Reputation age: **7 days** minimum
- Capacity: **100MB** minimum

**See**: [Spike Week 2 Briefing](spike/SPIKE-WEEK-2-BRIEFING.md) for persistence analysis

### Spike Week 2 (Q7-Q14) — Persistence Network

All persistence questions validated. Implementation ready:

| Question | Decision | Implementation Details |
|----------|----------|----------------------|
| Q7: Bot Discovery | ✅ GO | Registry contract at well-known address |
| Q8: Fake Bot Defense | ✅ GO | PoW (difficulty 18) + Reputation + Capacity verification |
| Q9: Chunk Verification | ✅ GO | Challenge-response with SHA-256 (128 bytes) |
| Q11: Rendezvous Hashing | ✅ GO | HRW algorithm (deterministic holder selection) |
| Q12: Chunk Size | ✅ GO | 64KB chunks (0.2% overhead, 32% distribution) |
| Q13: Spot Check | ✅ GO | 1% sample rate per write (0.16ms overhead) |
| Q14: Chunk Protocol | ✅ GO | Contract-based (Phase 0), Hybrid P2P (Phase 1+) |

**See**:
- [Q7 Results](spike/q7/RESULTS.md) - Registry-based discovery
- [Q8 Results](spike/q8/RESULTS.md) - Multi-layer Sybil defense
- [Q9 Results](spike/q9/RESULTS.md) - Challenge-response verification
- [Q11 Spike](spike/q11/main.rs) - Rendezvous hashing algorithm
- [Q12 Results](spike/q12/RESULTS.md) - Chunk size optimization
- [Q13 Results](spike/q13/RESULTS.md) - Fairness verification protocol
- [Q14 Results](spike/q14/RESULTS.md) - Distribution protocol comparison

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

Co-authored-by: Claude <noreply@anthropic.com>
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

1. Read [TODO.md](todo/TODO.md) for current tasks
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
- [Persistence](PERSISTENCE.md) - State durability & recovery
- [Federation Roadmap](FEDERATION.md) - Phase 4+ vision
- [Spike Week Briefing](spike/SPIKE-WEEK-BRIEFING.md) - Technology validation
- [Spike Week 2 Briefing](spike/SPIKE-WEEK-2-BRIEFING.md) - Persistence validation
- [TODO Checklist](todo/TODO.md) - Implementation tasks

### Constraint Beads (Immutable)
- [Security Constraints](../.beads/security-constraints.bead)
- [Architecture Decisions](../.beads/architecture-decisions.bead)
- [Persistence Model](../.beads/persistence-model.bead)
- [Contract Encryption](../.beads/contract-encryption.bead)
- [Discovery Protocols](../.beads/discovery-protocols.bead)
- [Federation Roadmap](../.beads/federation-roadmap.bead)

### Development Rules
- [Rust Standards](../.cursor/rules/rust-standards.mdc)
- [Rust Async Patterns](../.cursor/rules/rust-async.mdc)
- [Freenet Contract Design](../.cursor/rules/freenet-contract-design.mdc)
- [User Roles & UX](../.cursor/rules/user-roles-ux.mdc)
- [Security Guardrails](../.cursor/rules/security-guardrails.mdc)

### External References
- [freenet-core](https://github.com/freenet/freenet-core) - State storage and node embedding
- [freenet-stdlib](https://docs.rs/freenet-stdlib) - ContractInterface trait for Wasm contracts
- [winterfell](https://github.com/facebook/winterfell) - STARK proofs
- [libsignal-service-rs](https://github.com/whisperfish/libsignal-service-rs) - Signal integration

---

**Status**: Technology validation complete (Spike Week 1 & 2). Ready for Phase 0 implementation.

**Last Updated**: 2026-01-31
