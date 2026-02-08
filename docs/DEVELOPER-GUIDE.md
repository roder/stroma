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
- **Commutative Deltas** - Contract's responsibility (Q1 validated) - set-based state with ejected set
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
| **Signal Integration** | Presage + libsignal-service-rs | High-level API + protocol (custom store, no SqliteStore) |
| **Async Runtime** | tokio 1.35+ | Event-driven execution |
| **CLI Framework** | clap 4+ | Operator interface |
| **Supply Chain** | cargo-deny, cargo-crev | Security audits |

### Why Rust 1.93+

- **musl 1.2.5**: Major DNS resolver improvements ([InfoWorld article](https://www.infoworld.com/article/4120988/rust-1-93-updates-bundled-musl-library-to-boost-networking.html))
- **Static linking**: No dynamic library vulnerabilities
- **Networking**: More reliable for Signal and freenet-core
- **DNS handling**: Better with large records and recursive name servers

### Rust Toolchain Components

**Required components for development and CI:**

- **rustfmt**: Code formatting (enforced in CI)
- **clippy**: Linting with zero-warning policy
- **llvm-tools-preview**: Code coverage instrumentation

**Required targets:**

- **x86_64-unknown-linux-musl**: Static binary builds (release workflow)

**Installation:**

```bash
# Install components
rustup component add rustfmt clippy llvm-tools-preview

# Install musl target
rustup target add x86_64-unknown-linux-musl
```

**Note**: All CI workflows use `dtolnay/rust-toolchain` action for consistent toolchain setup across jobs.

### Why STARKs (not PLONK)

| Feature | STARKs | PLONK |
|---------|--------|-------|
| **Trusted Setup** | ❌ None | ✅ Required (ceremony) |
| **Post-Quantum** | ✅ Secure | ❌ Vulnerable |
| **Transparency** | ✅ Fully transparent | ⚠️ Depends on setup |
| **Proof Size** | ⚠️ Larger (~100KB) | ✅ Smaller (~1KB) |
| **Verification** | ✅ Constant time | ✅ Constant time |

**Decision**: STARKs for trustlessness and post-quantum security (proof size acceptable)

## Module Structure

```
src/
├── main.rs                          # Event loop, CLI entry point
├── lib.rs                           # Library root, module declarations
├── identity.rs                      # HMAC identity masking with ACI-derived key, zeroization
├── cli/                             # Operator CLI (service management only)
│   ├── mod.rs
│   ├── link_device.rs               # Link to Signal account (one-time)
│   ├── run.rs                       # Run bot service (awaits member-initiated bootstrap)
│   ├── status.rs                    # Bot status reporting
│   ├── verify.rs                    # Configuration verification
│   ├── backup_store.rs              # Protocol store backup
│   └── version.rs                   # Version information
├── stark/                           # ZK-STARK Vouch Verification
│   ├── mod.rs
│   ├── circuit.rs                   # VouchAir circuit (winterfell AIR)
│   ├── types.rs                     # VouchClaim, VouchProof, MemberHash
│   ├── prover.rs                    # prove_vouch_claim() (simplified Phase 0)
│   ├── verifier.rs                  # verify_vouch_proof()
│   └── proptests.rs                 # Property-based tests
├── freenet/                         # Freenet Integration
│   ├── mod.rs
│   ├── traits.rs                    # FreenetClient trait abstraction
│   ├── embedded_kernel.rs           # EmbeddedKernel implementation (currently mock)
│   ├── mock.rs                      # MockFreenetClient for tests
│   ├── contract.rs                  # Contract deployment helpers
│   ├── trust_contract.rs            # TrustNetworkState, GroupConfig, MemberHash
│   └── state_stream.rs              # Real-time state change events
├── signal/                          # Signal Integration
│   ├── mod.rs
│   ├── traits.rs                    # SignalClient trait abstraction
│   ├── client.rs                    # LibsignalClient (100% stubbed, wiring TODO)
│   ├── mock.rs                      # MockSignalClient for tests
│   ├── store.rs                     # StromaProtocolStore (minimal, security-focused)
│   ├── linking.rs                   # link_secondary_device() (stubbed)
│   ├── bot.rs                       # StromaBot: command dispatch, run loop
│   ├── group.rs                     # Group management (add/remove members)
│   ├── pm.rs                        # 1-on-1 PM command handling
│   ├── bootstrap.rs                 # /create-group, /add-seed bootstrap flow
│   ├── vetting.rs                   # VettingSessionManager (ephemeral sessions)
│   ├── matchmaker.rs                # BlindMatchmaker (cross-cluster assessor selection)
│   ├── member_resolver.rs           # Transient MemberHash <-> ServiceId mapping
│   ├── polls.rs                     # PollManager (create, vote, terminate)
│   ├── retry.rs                     # Retry logic with backoff
│   └── proposals/                   # Proposal system
│       ├── mod.rs
│       ├── command.rs               # /propose argument parsing
│       ├── lifecycle.rs             # Proposal creation, monitoring, execution
│       └── executor.rs              # Config change application
├── gatekeeper/                      # Admission & Ejection Protocol
│   ├── mod.rs
│   ├── ejection.rs                  # Immediate ejection (two triggers)
│   ├── health_monitor.rs            # Continuous standing checks via state stream
│   ├── rate_limiter.rs              # 5-tier progressive cooldown (GAP-03)
│   └── audit_trail.rs               # Immutable append-only log (GAP-01)
├── matchmaker/                      # Internal Mesh Optimization
│   ├── mod.rs
│   ├── graph_analysis.rs            # Bridge detection, Union-Find, centrality
│   ├── cluster_detection.rs         # Tarjan's algorithm, GAP-11 announcement
│   ├── dvr.rs                       # Degree-Vouch Ratio calculation
│   ├── strategic_intro.rs           # 3-phase: DVR / MST / cluster bridging
│   └── display.rs                   # Transient name resolution for output
├── persistence/                     # Reciprocal Persistence Network
│   ├── mod.rs                       # Public API
│   ├── encryption.rs                # AES-256-GCM, version chain, Merkle root
│   ├── chunks.rs                    # 64KB chunk split/join
│   ├── chunk_storage.rs             # Contract-based chunk storage
│   ├── distribution.rs              # Parallel chunk distribution
│   ├── rendezvous.rs                # Deterministic holder selection
│   ├── registry.rs                  # Peer discovery, tombstones, epochs
│   ├── attestation.rs               # HMAC-SHA256 receipts (Q9)
│   ├── health.rs                    # 4-state replication health model
│   ├── recovery.rs                  # Crash recovery from chunks
│   ├── write_blocking.rs            # State machine (Provisional/Active/Degraded/Isolated)
│   └── proptests.rs                 # Property-based tests
├── crypto/                          # Federation Cryptography
│   ├── mod.rs
│   └── psi_ca.rs                    # Private Set Intersection (federation discovery)
├── federation/                      # Federation Logic (Phase 4+)
│   ├── mod.rs
│   └── social_anchor.rs             # Social Anchor computation
└── serialization/                   # Wire Format
    └── mod.rs                       # CBOR via ciborium (to_cbor, from_cbor)
```

**Key Design**: `federation/` and `crypto/psi_ca.rs` exist for federation discovery (Phase 4+).
**Key Design**: `persistence/` ensures trust state durability even if Freenet data falls off.
**Key Design**: `identity.rs` is a single file (not a module directory) containing HMAC masking with zeroization.
**Key Design**: `stark/` contains ZK-STARK circuits; `crypto/` contains only PSI-CA for federation.

**See**: [ALGORITHMS.md](ALGORITHMS.md) for detailed MST algorithm, PSI-CA protocol, and complexity analysis.

### Future: Shadow Handover (Phase 4+)

Phase 4+ will add a `shadow_handover.rs` module for cryptographic bot identity rotation:

- **Purpose**: Allow bot to rotate Signal phone number while preserving trust context
- **Mechanism**: Succession Document signed by old bot key, validated by Freenet contract
- **Use Cases**: Signal ban recovery, periodic rotation, operational security

**Note**: Not yet implemented. See `.beads/federation-roadmap.bead` for protocol specification.

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
        Commands::LinkDevice { device_name, .. } => {
            link_secondary_device(device_name).await?;
        }
        Commands::Run { config, bootstrap_contact, .. } => {
            // Bot handles bootstrap via Signal commands (/create-group, /add-seed)
            // NOT via CLI - see .beads/bootstrap-seed.bead
            run_bot_service(config, bootstrap_contact).await?;
        }
        // ... other commands (status, verify, backup-store, version)
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

### Proposal System & Voting

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
    freenet: &FreenetClient,
    proposal: &ActiveProposal,
    config: &GroupConfig,
) -> Result<ProposalResult> {
    // Fetch aggregated poll results from Signal
    // NOTE: Signal provides only vote counts, NOT who voted what
    let poll_data = manager.get_poll_results(
        proposal.poll_timestamp,
    ).await?;
    
    let approve_count = poll_data.options[0].vote_count;  // 👍
    let reject_count = poll_data.options[1].vote_count;   // 👎
    let total_votes = approve_count + reject_count;
    
    // Get total members from Freenet (source of truth)
    let total_members = freenet.get_member_count().await?;
    
    if total_votes == 0 {
        return Ok(ProposalResult {
            approved: false,
            quorum_met: false,
            threshold_met: false,
            approve_count: 0,
            reject_count: 0,
            approval_ratio: 0.0,
            participation_rate: 0.0,
        });
    }
    
    // Quorum: % of members who voted (abstainers count against quorum)
    let participation_rate = total_votes as f32 / total_members as f32;
    let quorum_met = participation_rate >= config.min_quorum;
    
    // Threshold: % of votes cast that approved (abstainers don't affect threshold)
    let approval_ratio = approve_count as f32 / total_votes as f32;
    let threshold_met = approval_ratio >= config.config_change_threshold;
    
    // BOTH quorum AND threshold must be met
    Ok(ProposalResult {
        approved: quorum_met && threshold_met,
        quorum_met,
        threshold_met,
        approve_count,
        reject_count,
        approval_ratio,
        participation_rate,
        // NO individual votes - preserves anonymity
    })
}
```

**Result Messaging:**
- If `!quorum_met`: "Proposal failed: Quorum not met (X% participated, Y% required)"
- If `quorum_met && !threshold_met`: "Proposal failed: Threshold not met (X% approved, Y% required)"
- If `approved`: "Proposal passed: X% approved (quorum: Y%, threshold: Z%)"

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
                
                // Fetch poll results
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

### API Documentation

Stroma uses rustdoc for API reference documentation. The codebase includes 931 lines of rustdoc comments across 63 source files.

**Generate and view API docs:**

```bash
# Generate documentation and open in browser
cargo doc --no-deps --open

# Generate without opening
cargo doc --no-deps
```

**Documentation location:**

Generated docs are placed in `./target/doc/stroma/index.html`

**Module-level documentation:**

Each module includes high-level documentation explaining its purpose and key types. See `src/*/mod.rs` files for module overviews.

**Tips:**
- Use `--no-deps` to exclude dependency documentation (faster builds)
- Use `--document-private-items` to include private API documentation during development
- Regenerate docs after making changes to doc comments

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

**Canonical Source**: [SECURITY-CI-CD.md](SECURITY-CI-CD.md)

All PRs to `main` are automatically blocked if they violate security constraints. See SECURITY-CI-CD.md for complete workflow details.

### Required Checks (Must ALL Pass)

| Check | Tool | Requirement |
|-------|------|-------------|
| Supply chain | `cargo-deny` | No vulnerabilities, no banned crates |
| Static analysis | CodeQL | No high/critical findings |
| Linting | `cargo clippy` | Zero warnings (`-D warnings`) |
| Formatting | `cargo fmt` | No deviations |
| Test coverage | `cargo-llvm-cov` | **100% mandatory** |
| Binary size | musl build | No bloat (10% / 1MB limit) |
| Security constraints | grep patterns | No cleartext IDs, no SqliteStore |
| Unsafe blocks | grep patterns | All must have `// SAFETY:` comments |

### Running Locally (Before PR)

**Quick check (2-3 minutes):**
```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo deny check
```

**Full check (5-10 minutes):**
```bash
cargo llvm-cov nextest --all-features   # 100% coverage required
cargo build --release --target x86_64-unknown-linux-musl
```

### Critical Violations (Auto-Reject)

- ❌ Cleartext Signal IDs (use `mask_identity()` + zeroize)
- ❌ `presage-store-sqlite` (use `StromaProtocolStore`)
- ❌ Grace periods in ejection logic
- ❌ Unsafe blocks without `// SAFETY:` comments
- ❌ Test coverage below 100%

See [SECURITY-CI-CD.md](SECURITY-CI-CD.md) for fix patterns and detailed guidance.


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

**Canonical Source**: `.beads/architectural-decisions-open.bead` § 9 (Logging Verbosity & Security)

**Four-Layer Log Security** - Never log identifiers that could link to real-world identity:

| Layer | What's Protected | Logging Rule |
|-------|------------------|--------------|
| Layer 1 (PII) | Signal IDs, phone numbers, names | **NEVER log** |
| Layer 2 (Hashes) | Member hashes, group hashes | Log at DEBUG only |
| Layer 3 (Aggregates) | Counts, percentages, status | Log at INFO |
| Layer 4 (Errors) | Error types (no identifiers) | Log at ERROR/WARN |

```rust
use tracing::{info, warn, error, debug};

// ✅ Layer 3: Aggregates at INFO
info!("Member admitted");
info!("Network state: {} members, {} pending invites", count, pending);

// ✅ Layer 2: Hashes at DEBUG only
debug!(member = %member_hash, "Vouch recorded");

// ✅ Layer 4: Errors without identifiers
error!("Signal API failed: {}", error_type);

// ❌ Layer 1: NEVER log PII
error!("Failed to add {}", signal_id); // ❌ FORBIDDEN - leaks identity
debug!("Adding member: {}", phone_number); // ❌ FORBIDDEN
```

**Audit Question**: If logs were seized, could an adversary identify WHO is in the group? If YES, logging is too verbose.

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

**Last Updated**: 2026-02-08
