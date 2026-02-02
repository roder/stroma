# Persistence Architecture

**Status**: Validated via Spike Week 2 (Q7-Q14)
**Last Updated**: 2026-02-01

---

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Discovery](#discovery)
- [Security](#security)
- [Distribution](#distribution)
- [Recovery](#recovery)
- [Write-Blocking](#write-blocking)
- [Operator Guide](#operator-guide)
- [Implementation Notes](#implementation-notes)

---

## Overview

### What is the Reciprocal Persistence Network?

The Reciprocal Persistence Network is Stroma's solution to a fundamental problem: **Freenet does not guarantee persistence**. If no peers are subscribed to a contract, the data disappears. This is a core property of Freenet's architecture.

For Stroma, this is catastrophic. Trust maps represent relationships built over months or years. Loss of this data destroys the community.

### The Goal

> A Stroma bot must be able to crash, lose all local state, and fully recover its trust map from encrypted fragments held by adversarial peers who cannot read or reconstruct that data.

### How It Works

Stroma bots replicate each other's **encrypted state chunks** even though they are adversaries:

```
WITHOUT PERSISTENCE:
  Bot crashes → No peers subscribed → Trust map gone → Community destroyed

WITH RECIPROCAL PERSISTENCE:
  Bot State (512KB)
    → 8 chunks × 64KB each
    → 2 remote replicas per chunk (16 distributions)
    → Bot crashes
    → Collect ALL chunks from remote holders
    → Decrypt with ACI key
    → Community intact
```

### Security Through Distribution

Larger states = more chunks = more distribution = harder to seize:

| State Size | Chunks | Remote Holders | Attack Complexity |
|------------|--------|----------------|-------------------|
| 64KB | 1 | ~2 bots | Compromise 2 bots + ACI |
| 512KB | 8 | ~6-8 bots | Compromise holders of ALL 8 + ACI |
| 5MB | ~80 | ~80-160 bots | Compromise holders of ALL 80 + ACI |

**Key insight**: Without the ACI key, ciphertext chunks are useless. Encryption is the real barrier.

---

## Architecture

### Two-Layer Design

#### Layer 1: Trust State (Freenet-Native)

| Aspect | Specification |
|--------|---------------|
| **Storage** | BTreeSet (members), HashMap (vouches, flags) |
| **Sync** | Native Freenet ComposableState (validated in Q1 spike) |
| **Updates** | Small deltas (~100-500 bytes), INFREQUENT (human timescale) |
| **Security** | Contract validates via `update_state()` + `validate_state()` |

Freenet provides:
- Summary-Delta Sync: Trust state merges commutatively
- Subscription Trees: Bots subscribe to contract state changes
- Eventual Consistency: Trust state converges across network
- Small-World Topology: Efficient propagation of updates

#### Layer 2: Persistence Chunks (Reciprocal Persistence Network)

| Aspect | Specification |
|--------|---------------|
| **Purpose** | Durability against Freenet data loss, server seizure protection |
| **Method** | Encrypt full state, chunk by size, replicate each chunk 3x |
| **Distribution** | Deterministic assignment via rendezvous hashing |
| **Frequency** | Same as trust state updates (infrequent) |
| **Security** | Need ALL chunks + ACI key to reconstruct |

What Freenet does NOT provide (we add):

| Gap | Stroma's Solution |
|-----|-------------------|
| Persistence (data falls off) | Reciprocal Persistence Network |
| Encryption at rest | Application-level AES-256-GCM |
| Seizure resistance | Chunked fragments distributed across N bots |
| Member count privacy | Size buckets, encrypted attestations |

### Full Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    RECIPROCAL PERSISTENCE NETWORK                            │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │              PERSISTENCE PEER DISCOVERY LAYER                           │ │
│  │                                                                          │ │
│  │  Mechanism: Dedicated Freenet contract for Stroma bot discovery        │ │
│  │  All discovered bots are ADVERSARIES:                                   │ │
│  │  - Cannot decrypt each other's state                                    │ │
│  │  - Cannot learn trust graph structure                                   │ │
│  │  - Only hold encrypted fragments + signatures                           │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                    │                                         │
│                                    ▼                                         │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │              MINIMAL REGISTRY (O(N) storage)                            │ │
│  │                                                                          │ │
│  │  PUBLIC (deterministic assignment - no relationships stored):          │ │
│  │  - Network size (N bots exist)                                          │ │
│  │  - Bot membership (Contract-X is a Stroma bot)                          │ │
│  │  - Current epoch (monotonic, increments on >10% bot count change)      │ │
│  │  - Size buckets (Contract-X has ~50 members) - optional, for fairness  │ │
│  │                                                                          │ │
│  │  NOT STORED:                                                             │ │
│  │  - Chunk holder relationships → COMPUTED via rendezvous hashing        │ │
│  │  - No per-chunk records → scales to any network size                   │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                    │                                         │
│                                    ▼                                         │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │              ENFORCEMENT LAYER (FAIRNESS + SECURITY)                    │ │
│  │                                                                          │ │
│  │  Before Bot-A can WRITE to its contract:                                │ │
│  │  1. Query network size (N) from registry                                │ │
│  │  2. IF N >= 3: Verify all chunks have 2+ replicas confirmed             │ │
│  │     AND verify storing ~2x own state size (fairness)                    │ │
│  │  3. IF N = 2: Verify mutual replication (both directions)               │ │
│  │  4. IF N = 1: WARN, allow writes (operator accepts risk)                │ │
│  │  5. If not compliant: BLOCK writes until compliant                      │ │
│  │                                                                          │ │
│  │  FAIRNESS PRINCIPLE: "Give 2x what you take"                            │ │
│  │  - Each bot stores ~2x their state size in fragments from others        │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                    │                                         │
│                                    ▼                                         │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │              VERIFICATION & RECOVERY LAYER                              │ │
│  │                                                                          │ │
│  │  STATE VERSIONING:                                                      │ │
│  │  - Each state update has monotonic version number                       │ │
│  │  - Recovery requests specific version OR "latest"                       │ │
│  │  - Enables consistent recovery across multiple chunk holders            │ │
│  │                                                                          │ │
│  │  RECOVERY (deterministic computation):                                  │ │
│  │  1. Bot loads Signal identity from backup                               │ │
│  │  2. Fetch registry: get bot list + current epoch + my num_chunks        │ │
│  │  3. For each chunk: COMPUTE holders via rendezvous_hash(chunk_index)   │ │
│  │  4. Request ALL chunks from computed holders (any 1 of 3 per chunk)    │ │
│  │  5. Concatenate chunks, decrypt with ACI key, verify signature, resume │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Chunking and Replication Model

```
CHUNK_SIZE = 64KB          (validated in Q12: balance distribution vs coordination)
REPLICATION_FACTOR = 3     (1 local + 2 remote replicas per chunk)
```

#### How It Works

```
Bot-A's state = 512KB encrypted
  → Split into ceil(512KB / 64KB) = 8 chunks
  → Each chunk: 1 local copy + 2 remote replicas = 3 copies total
  → 2 remote replicas distributed to DIFFERENT bots via rendezvous hashing
  → Total remote placements: 8 × 2 = 16 placements across ~6-8 bots
```

#### Security Through Distribution

```
┌─────────────────────────────────────────────────────────────────┐
│                    CHUNK REPLICATION MODEL                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Bot-A's state (512KB) → 8 encrypted chunks                     │
│                                                                  │
│  Chunk[0]: Bot-A (local) + Bot-X + Bot-Y  (3 copies)           │
│  Chunk[1]: Bot-A (local) + Bot-Z + Bot-W  (3 copies)           │
│  Chunk[2]: Bot-A (local) + Bot-M + Bot-N  (3 copies)           │
│  ...                                                            │
│  Chunk[7]: Bot-A (local) + Bot-P + Bot-Q (3 copies)            │
│                                                                  │
│  RESILIENCE: Any 1 of 3 copies per chunk = recoverable         │
│  SECURITY: Need ALL 8 chunks + ACI key to reconstruct          │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Discovery

### Registry-Based Bot Discovery

**Validated in Q7 Spike** (`docs/spike/q7/RESULTS.md`): Registry provides <1ms lookup latency.

All Stroma bots register in a well-known Freenet contract for persistence peer discovery. These are **adversaries**, not trusted partners.

#### Why Sharding?

A single Freenet contract storing all bot registrations would become:
- **A bottleneck**: Every bot read/write hits the same contract
- **A scalability limit**: Contract state grows unbounded as bots join
- **A single point of failure**: One contract's availability affects the entire network

Sharding distributes the registry across multiple contracts, each holding a subset of bots. This provides:
- **Parallel queries**: Multiple shards can be queried simultaneously
- **Bounded shard size**: Each shard stays manageable (~2,000-5,000 bots max)
- **Graceful scaling**: New shards added as network grows

**Phase 0 (current)**: Single registry is sufficient for <5,000 bots. Sharding is designed but not needed yet.

#### Registry Architecture

Two contract types coordinate persistence:

**1. Registry Metadata Contract** (well-known, single instance):
```rust
/// Well-known address: sha256("stroma-registry-metadata-v1")
pub struct RegistryMetadata {
    version: u32,
    shard_count: u32,                   // Current shards (power of 2)
    global_epoch: u64,                  // All shards reference this
    total_bots: u64,                    // Approximate count
    migration_status: MigrationStatus,  // Stable or Splitting
}
```

**2. Per-Shard Registry Contracts** (one per shard):
```rust
pub struct PersistenceRegistry {
    bots: BTreeSet<RegistryEntry>,
    tombstones: BTreeSet<ContractHash>,
    clock: u64,                         // Vector clock for migrations
}

pub struct RegistryEntry {
    contract_hash: ContractHash,
    size_bucket: SizeBucket,
    num_chunks: u32,
    registered_at: Timestamp,
    clock: u64,
}
```

**What's stored**: Bot membership, network size, size buckets, migration state
**What's NOT stored**: Chunk holder relationships (computed via rendezvous hashing)

#### Fibonacci-Triggered Scaling

Shard splits are triggered at Fibonacci thresholds, with powers-of-2 shard counts for simple hash partitioning:

| Bot Count Threshold | Shard Count | Bots/Shard (at trigger) |
|---------------------|-------------|-------------------------|
| 0 - 4,999 | 1 | up to 5,000 |
| 5,000 | 2 | ~2,500 |
| 8,000 | 4 | ~2,000 |
| 13,000 | 8 | ~1,625 |
| 21,000 | 16 | ~1,312 |
| 34,000 | 32 | ~1,062 |
| 55,000 | 64 | ~859 |
| 89,000 | 128 | ~695 |
| 144,000 | 256 | ~562 |

**Phase 0**: Single registry (sufficient for <5K bots)
**Scale trigger**: First split at 5,000 bots

#### Migration Protocol

When splitting from N to 2N shards:
1. **Announce**: Metadata contract sets `MigrationStatus::Splitting`
2. **Transition** (24h window): Writes go to new shard, reads query both old and new
3. **Merge**: Vector clocks resolve conflicts (newer clock wins, tombstones win ties)
4. **Complete**: Metadata updates `shard_count`, bumps `global_epoch`

See `.beads/persistence-model.bead` for full migration protocol with vector clocks.

#### Performance

- **Discovery latency**: <1ms (100 bots)
- **Registration overhead**: ~100 bytes per bot
- **Network size query**: O(1)

---

## Security

### PoW Sybil Resistance

**Validated in Q8 Spike** (`docs/spike/q8/RESULTS.md`): PoW prevents Sybil attacks while remaining RPi-compatible.

#### Defense Strategy: Multi-Layered

| Layer | Mechanism | Cost to Attacker |
|-------|-----------|------------------|
| **PoW** | Difficulty 18 (~30s per registration) | 2 minutes for 1000 bots |
| **Capacity** | Must prove 100MB storage minimum | 100GB for 1000 fake bots |
| **Reputation** | 7-day minimum age + successful operations | 7+ days + operational complexity |
| **Challenge-Response** | Periodic spot checks (1%) | Must maintain actual chunks |

**Combined**: Attack transforms from "one-time registration" to "sustained operational infrastructure."

#### PoW Parameters

```rust
pub const POW_DIFFICULTY: u32 = 18;  // ~30 seconds on desktop, ~60s on RPi 4
```

**Rationale**:
- Creates meaningful computational cost for mass registration
- RPi 4 compatible (~60 seconds acceptable for one-time registration)
- Adjustable based on network conditions

#### Reputation Formula

```rust
trust_score = (success_rate * 0.5) + (age_factor * 0.3) + (activity_factor * 0.2)

where:
  success_rate = successful_returns / (successful_returns + failed_returns + 1)
  age_factor = min(age_days / 30.0, 1.0)
  activity_factor = min(chunks_held / 10.0, 1.0)

eligible = trust_score >= 0.3 && age_days >= 7
```

### Chunk Verification

**Validated in Q9 & Q13 Spikes** (`docs/spike/q9/RESULTS.md`, `docs/spike/q13/RESULTS.md`): Challenge-response proves chunk possession with <1ms latency.

#### Challenge-Response Protocol

```rust
/// Challenge: Prove you have chunk[X] by returning hash(chunk || nonce)
struct ChunkChallenge {
    owner: ContractHash,       // Whose chunk
    chunk_index: u32,          // Which chunk
    nonce: [u8; 32],           // Random nonce (prevents replay)
    offset: usize,             // Sample location (256 bytes)
    length: usize,             // Sample size
    timestamp: u64,            // Freshness check
}

struct ChunkResponse {
    proof: Hash,               // SHA-256(nonce || chunk_sample)
    responder: ContractHash,   // Holder's identity
}
```

#### Security Properties

| Property | Mechanism | Result |
|----------|-----------|--------|
| **Possession proof** | Must have actual bytes to compute correct hash | ✅ Holder cannot fake |
| **Replay resistance** | Nonce changes each challenge | ✅ Old responses invalid |
| **Deletion detection** | Missing chunk → cannot respond | ✅ Detected immediately |
| **Content privacy** | Hash reveals nothing about plaintext | ✅ Zero knowledge leak |
| **Freshness** | Timestamp window (1 hour) | ✅ Prevents stale proofs |

#### Enforcement: Spot Checks

**Phase 0**: 1% random sampling before writes
- 100 chunks distributed → verify 1 random holder
- Detection rate: 100% (eventually - probabilistic)
- False positive rate: 0%
- Latency: <10ms per write

**Phase 1+**: Reputation scoring
- Track successful/failed challenges
- Soft deprioritization of bad actors
- Hard exclusion after 10 consecutive failures

---

## Distribution

### Rendezvous Hashing for Deterministic Assignment

**Validated in Q11 Spike** (see `.beads/persistence-model.bead`): Provides deterministic, stable, uniform holder assignment.

#### Why Deterministic Assignment

| Aspect | Registry-Based (chunk mappings stored) | Deterministic (Rendezvous) |
|--------|----------------------------------------|---------------------------|
| Registry size | O(chunks × replicas) | O(N) bot list only |
| Assignment lookup | Query registry | Local computation |
| Registry breach impact | Reveals all chunk-holder mappings | Reveals bot list only (mappings computable anyway) |
| Churn handling | Update records | Recompute (graceful) |

**Security tradeoff accepted**: Anyone can compute who holds whose chunks. But:
- Chunks are encrypted (holder can't read)
- Need ALL chunks + ACI key (single chunk = partial ciphertext)
- Holders are adversaries (don't trust each other)
- Knowing holder identities doesn't help without compromising those holders AND obtaining ACI key

**Registry attack surface analysis**:

The registry still contains valuable information for attackers:
- ⚠️ List of all Stroma bot addresses (reveals network membership)
- ⚠️ Size buckets (approximate group sizes)
- ⚠️ Network topology (which bots exist)

What deterministic assignment **removes** from the registry:
- ✅ Per-chunk holder relationships (no O(chunks × replicas) records to breach)
- ✅ Correlation between specific chunks and specific holders

**Net effect**: The registry is a **lower-value** target than a registry storing explicit chunk-holder mappings, but it's not zero-value. An attacker who compromises the registry learns which bots exist but not the contents of any trust maps (chunks are still encrypted, distributed, and require the ACI key).

### Registry Availability Attacks (DDoS Threat Model)

The registry contract is a known attack surface for availability attacks. Unlike data exfiltration (which encryption defeats), availability attacks aim to **disrupt the persistence network's operation**.

#### Attack Vectors

| Attack | Mechanism | Cost to Attacker | Impact |
|--------|-----------|------------------|--------|
| **State Bloat** | Register thousands of fake bots | PoW cost (~30s each) | Registry contract grows, queries slow |
| **Contract Computation** | Malformed/expensive queries | Freenet node resources | Contract execution degrades |
| **Read Amplification** | Query registry millions of times | Network bandwidth | Freenet nodes hosting registry exhausted |
| **Shard-Targeted** | Focus attack on specific shard | Same as above, concentrated | Subset of bots affected |

#### Defense Layers

**Layer 1: Freenet Native Protections** (First Line)

Freenet provides baseline protections that apply to all contracts:

- **Redundancy**: Contracts hosted by multiple nodes (no single point of failure)
- **Rate limiting**: Nodes can rate-limit queries from specific sources
- **Replication**: Contract state replicated across network
- **Resource isolation**: Expensive contracts don't affect other contracts on same node

**Limitation**: These protections are general-purpose. A sustained, well-resourced attack can still degrade service.

**Layer 2: PoW Registration Cost** (Sybil Prevention)

PoW creates computational cost for fake registrations:

```rust
// Attack economics
const POW_DIFFICULTY: u32 = 18;  // ~30 seconds per registration

// Cost analysis:
// 1,000 fake bots = ~8 hours CPU time
// 10,000 fake bots = ~83 hours CPU time (botnet required)
// Registry bloat limited by attacker's compute budget
```

**Layer 3: Contract-Level Rate Limiting** (REQUIRED for Phase 1+)

The registry contract SHOULD implement:

```rust
pub struct RegistryRateLimits {
    /// Maximum queries per source identity per minute
    query_rate_limit: u32,           // Default: 60/min
    
    /// Maximum computation cycles per operation
    compute_budget_per_op: u64,      // Default: 10_000 cycles
    
    /// Circuit breaker: disable expensive ops when load high
    circuit_breaker_threshold: f32,  // Default: 0.8 (80% capacity)
}

impl PersistenceRegistry {
    fn handle_query(&mut self, source: &Identity, query: Query) -> Result<Response> {
        // Rate limit check
        if self.rate_limiter.is_exceeded(source) {
            return Err(Error::RateLimited);
        }
        
        // Compute budget check
        let estimated_cost = query.estimate_compute_cost();
        if estimated_cost > self.limits.compute_budget_per_op {
            return Err(Error::QueryTooExpensive);
        }
        
        // Circuit breaker
        if self.load_monitor.current_load() > self.limits.circuit_breaker_threshold {
            if !query.is_essential() {
                return Err(Error::CircuitBreakerOpen);
            }
        }
        
        self.execute_query(query)
    }
}
```

**Layer 4: Sharding Resilience**

Sharding provides natural DDoS resistance:

- Attack on one shard doesn't affect other shards
- Attacker must distribute resources across all shards
- Bots in attacked shard operate in degraded mode (not blocked)
- Recovery happens automatically as attack subsides

```
Attack distribution with 16 shards:
  Single-shard attack: 1/16 of bots affected
  Full-network attack: Attack power diluted 16x
  Recovery: Each shard recovers independently
```

#### Graceful Degradation Under Attack

**Design principle**: The persistence network should **degrade gracefully**, not fail catastrophically.

| Attack Intensity | System Behavior |
|-----------------|-----------------|
| **Low** (probing) | Normal operation, rate limiting activates |
| **Medium** (sustained) | Query latency increases, non-essential ops delayed |
| **High** (DDoS) | Circuit breaker activates, essential ops only |
| **Extreme** (state-level) | Shards fail independently, partial network operation |

**During attack, bots can still**:
- ✅ Use cached registry data (stale but usable)
- ✅ Compute chunk holders locally (rendezvous hashing)
- ✅ Contact known holders directly
- ✅ Operate with degraded persistence (write-blocking may activate)

**During attack, bots cannot**:
- ❌ Register new bots (PoW submission fails)
- ❌ Discover newly joined bots (registry queries fail)
- ❌ Update size buckets (registry writes fail)

#### Accepted Risk

**The registry is inherently a discoverable, queryable resource.** This is required for the persistence network to function—bots must be able to find each other.

**Defense goal**: Degrade gracefully, recover quickly, not "prevent all DDoS."

**Residual risk**: A well-resourced state-level adversary can likely disrupt the persistence network temporarily. The defense is:
1. Disruption is temporary (attack must be sustained)
2. Trust maps remain encrypted (disruption ≠ data breach)
3. Bots can operate offline using cached data
4. Network recovers automatically when attack subsides

**See**: `docs/THREAT-MODEL-AUDIT.md` for complete threat analysis

---

#### Rendezvous Hashing Algorithm

```rust
/// Compute the 2 replica holders for a specific chunk
fn compute_chunk_holders(
    owner_contract: &ContractHash,
    chunk_index: u32,
    registered_bots: &[ContractHash],
    epoch: u64,
) -> [ContractHash; 2] {
    // Rendezvous hashing: each bot gets a score for holding this chunk
    // Top 2 scores = this chunk's replica holders
    let mut scores: Vec<(ContractHash, Hash)> = registered_bots
        .iter()
        .filter(|b| *b != owner_contract)  // Can't hold own chunks
        .map(|bot| {
            // Include chunk_index so different chunks go to different holders
            let score = hash(owner_contract, chunk_index, bot, epoch);
            (*bot, score)
        })
        .collect();

    scores.sort_by_key(|(_, score)| *score);
    [scores[0].0, scores[1].0]
}
```

**Properties validated**:
- ✅ Deterministic (same inputs → same outputs)
- ✅ Uniform distribution (no "hot" holders)
- ✅ Stable under churn (minimal reassignment when bots join/leave)
- ✅ No owner influence (can't game the algorithm)

### Chunk Size: 64KB Optimal

**Validated in Q12 Spike** (`docs/spike/q12/RESULTS.md`): 64KB provides optimal balance.

| Chunk Size | Chunks (512KB) | Distribution | Coordination Overhead | Verdict |
|------------|----------------|--------------|----------------------|---------|
| **1KB** | 2048 | 100% | 9.8% | High security, high overhead |
| **16KB** | 128 | 82.5% | 0.6% | Good balance (high security) |
| **64KB** | 32 | 32% | 0.2% | ✅ **Recommended** |
| **256KB** | 8 | 8% | 0.04% | Too concentrated |

**Decision**: 64KB for Phase 0
- Low coordination overhead (0.2%)
- Acceptable distribution (32% of network for 512KB state)
- Simple fairness bookkeeping
- Scales well to large states

**Alternative**: 16KB for high-security scenarios (82.5% distribution, 0.6% overhead)

### Reciprocal Fairness: 2x Storage Ratio

**Fundamental Rule**: "Give 2x what you take from the network"

Each bot stores approximately **2x their own state size** in chunks from other bots.

**Why 2x?**
- **2 remote replicas** per chunk (replication factor 3 = 1 local + 2 remote)
- **Other bots store 2x my state** for me (total remote storage)
- **I store 2x my state** for others (reciprocal fairness)

#### Fairness Verification

**Validated in Q13 Spike** (`docs/spike/q13/RESULTS.md`): Spot checks effective.

**Enforcement Options**:
1. **Spot checks** (Phase 0): 1% random challenges before writes - 100% detection, 0% false positives
2. **Reputation scoring** (Phase 1+): Track challenge success rate
3. **Soft deprioritization**: Bad actors less likely as holders

### Contract-Based Communication

**Validated in Q14 Spike** (`docs/spike/q14/RESULTS.md`): Freenet contracts for Phase 0.

#### Phase 0: Contract-Based Distribution

**Approach**: Chunks written as state updates to holder contracts.

**Characteristics**:
- Latency: ~1.6s for 512KB state (16 distributions)
- Cost: 160 units per update
- Acceptable for 10-100 updates/month
- Simple, single mechanism

#### Phase 1+: Hybrid (P2P + Attestations)

**Benefits**:
- 5x faster data transfer (~320ms vs ~1.6s)
- 9x cheaper (18 units vs 160 units per update)
- Scales better for frequent updates

**Migration trigger**: When distribution cost exceeds implementation cost threshold.

---

## Recovery

### Recovery Requirements

**Your Signal protocol store IS your recovery identity.** No separate keypair file needed.

The bot uses the Signal account's **ACI (Account Identity) key** for:
- Chunk encryption (AES-256-GCM derived from ACI key via HKDF)
- State signatures (using ACI identity key)
- Persistence network identification

**Without Signal store backup**:
- Fragments are useless (can't derive decryption key)
- Trust map is permanently lost
- NO recovery path exists

### Recovery Flow

```rust
async fn recover_state() -> Result<State, Error> {
    // 1. Restore Signal protocol store from backup (REQUIRED)
    let signal_store = restore_signal_store_from_backup()?;
    let aci_identity = signal_store.get_identity_key_pair().await?;

    // 2. Query registry: get bot list, epoch, my num_chunks
    let registry = fetch_registry().await?;
    let my_entry = registry.get_my_entry(&aci_identity.public_key())?;
    let num_chunks = my_entry.num_chunks;

    // 3. Compute holders for each chunk, fetch ALL chunks
    let mut chunks = Vec::with_capacity(num_chunks as usize);
    for chunk_idx in 0..num_chunks {
        let holders = compute_chunk_holders(
            &my_entry.contract_hash,
            chunk_idx,
            &registry.bot_list,
            registry.epoch,
        );
        // Need any 1 of 3 copies (local copy was lost in crash)
        let chunk = fetch_chunk_from_any_holder(&holders, chunk_idx).await?;
        chunks.push(chunk);
    }

    // 4. Concatenate chunks, derive encryption key from ACI, decrypt, verify
    let encrypted_state = concatenate_chunks(&chunks);
    let encryption_key = derive_key_from_aci(&aci_identity);  // HKDF
    let decrypted = decrypt(&encrypted_state, &encryption_key)?;
    verify_signature(&decrypted, &aci_identity)?;
    Ok(decrypted)
}
```

### Recovery Verification

**Recovery uses signature verification only** — chain integrity is informational.

**What we verify**:
- ✅ Signature matches ACI identity (proves authorship)
- ✅ Decryption succeeds (proves we have correct key)
- ✅ All chunks present (proves complete state)

**What we DON'T require**:
- ❌ Chain verification (informational only)
- ❌ Historical state comparison (not available after crash)
- ❌ Version range check (any valid signed state is acceptable)

---

## Write-Blocking

### Write-Blocking States

| State | Condition | Writes | Replication Health |
|-------|-----------|--------|-------------------|
| **PROVISIONAL** | No suitable peers available | ALLOWED | 🔵 Initializing |
| **ACTIVE** | All chunks have 2+ replicas confirmed | ALLOWED | 🟢 Replicated or 🟡 Partial |
| **DEGRADED** | Any chunk has ≤1 replica, peers available | **BLOCKED** | 🔴 At Risk |
| **ISOLATED** | N=1 network | ALLOWED (warned) | 🔵 Initializing |

### Key Principle

**Availability-based, NOT TTL-based.** Bot never penalized for network scarcity.

### Network Bootstrap Limitations

| Network Size | Persistence Guarantee | Recommendation |
|--------------|----------------------|----------------|
| N=1 | ❌ None | Testing only |
| N=2 | ⚠️ Fragile | Temporary bootstrap |
| N=3-4 | 🟡 Minimal | Early adoption |
| N≥5 | ✅ Resilient | **Production minimum** |

### Replication Health Metric

**Replication Health** is measured at **write time** (not via heartbeats):

| Event | What Happens |
|-------|--------------|
| State changes | Bot creates snapshot → encrypts → chunks → distributes 2 replicas per chunk |
| All chunks fully replicated | 🟢 **Replicated** — fully resilient |
| Some chunks degraded (1/3) | 🟡 **Partial** — recoverable, but degraded |
| Any chunk has 0/3 copies | 🔴 **At Risk** — cannot recover that chunk if crash now |

**Formula**:
```
Replication Health = Chunks_With_2+_Replicas / Total_Chunks
```

---

## Operator Guide

### Critical: Backup Your Signal Protocol Store

**Your Signal protocol store is your recovery identity.**

**Backup procedure**:
```bash
# Backup Signal protocol store regularly
cp -r ~/.local/share/stroma-bot/signal_store ~/backups/signal_store-$(date +%Y%m%d)
```

**Store backups**:
- External drive (offline)
- Encrypted cloud storage (with strong passphrase)
- Hardware security module (production)

### Network Size Requirements

| Network Size | Resilience | Recommendation |
|--------------|-----------|----------------|
| N=1 | ❌ None | Testing only |
| N=2 | ⚠️ Fragile | Temporary bootstrap |
| N≥5 | ✅ Resilient | **Production minimum** |

### Replication Health Monitoring

Check replication health via:
```
/mesh replication
```

Expected output:
```
💾 Replication Health: 🟢 Replicated

Last State Change: 3 hours ago (Alice joined)
State Size: 512KB (8 chunks)
Chunks Replicated: 8/8 (all 3 copies per chunk) ✅
State Version: 47

Recovery Confidence: ✅ Yes — all chunks available from multiple holders
```

### Storage Requirements

| Your State Size | You Store for Others | Total Local Storage |
|----------------|---------------------|-------------------|
| 512KB | ~1MB | ~1.5MB |
| 2MB | ~4MB | ~6MB |
| 10MB | ~20MB | ~30MB |

### Registration Cost

**PoW registration**: ~30-60 seconds on RPi 4 (difficulty 18).

This is intentional (Sybil resistance). First registration may take up to 1 minute on low-power hardware.

---

## Implementation Notes

### Detailed Spike Results

| Spike | Component | Key Finding | Reference |
|-------|-----------|-------------|-----------|
| **Q7** | Bot Discovery | Registry-based discovery <1ms latency | `docs/spike/q7/RESULTS.md` |
| **Q8** | Sybil Resistance | PoW (difficulty 18) prevents fake bots | `docs/spike/q8/RESULTS.md` |
| **Q9** | Chunk Verification | Challenge-response <1ms, zero content leak | `docs/spike/q9/RESULTS.md` |
| **Q11** | Rendezvous Hashing | Deterministic, stable, uniform assignment | (See `.beads/persistence-model.bead`) |
| **Q12** | Chunk Size | 64KB optimal (0.2% overhead, 32% distribution) | `docs/spike/q12/RESULTS.md` |
| **Q13** | Fairness Verification | 1% spot checks: 100% detection, 0% false positives | `docs/spike/q13/RESULTS.md` |
| **Q14** | Communication Protocol | Contracts Phase 0, hybrid Phase 1+ | `docs/spike/q14/RESULTS.md` |

### Architecture Constraints

Detailed architectural rules are documented in:
- `.beads/persistence-model.bead` - Complete persistence model (includes Fibonacci scaling strategy)
- `.beads/security-constraints.bead` - Security requirements
- `.beads/architecture-decisions.bead` - Core architectural decisions
- `.beads/discovery-protocols.bead` - Discovery mechanisms (registry metadata, vector clocks)

### Key Parameters (FIXED)

```rust
pub const CHUNK_SIZE: usize = 64 * 1024;           // 64KB
pub const REPLICATION_FACTOR: usize = 3;           // 1 local + 2 remote
pub const POW_DIFFICULTY: u32 = 18;                // ~30s registration
pub const FAIRNESS_RATIO: f64 = 2.0;               // Store 2x own state
pub const CHALLENGE_SAMPLE_SIZE: usize = 256;      // 256 bytes per challenge
pub const SPOT_CHECK_RATE: f64 = 0.01;             // 1% sampling
pub const MINIMUM_REPUTATION_AGE_DAYS: u32 = 7;    // 7-day waiting period
pub const INITIAL_SHARD_COUNT: u32 = 1;            // Start unsharded
pub const FIRST_SPLIT_THRESHOLD: u32 = 5000;       // First Fibonacci threshold
pub const MIGRATION_WINDOW_HOURS: u32 = 24;        // Dual-read transition window
```

### Anti-Patterns (NEVER)

- ❌ Allow chunk owner to choose holders (collusion risk)
- ❌ Store chunks unencrypted (adversarial peers)
- ❌ Use single replica (need 3 copies for resilience)
- ❌ Store per-chunk relationships in registry (scaling, attack surface)
- ❌ Allow writes in DEGRADED state (unbackable changes)
- ❌ Skip Signal store backup in operator docs (unrecoverable)

### Always Required

- ✅ Use deterministic holder selection (rendezvous hashing per-chunk)
- ✅ Encrypt with key derived from Signal ACI before chunking
- ✅ Maintain 3 copies per chunk (1 local + 2 remote)
- ✅ Keep registry minimal (O(N) bot list only)
- ✅ Block writes until persistence confirmed (in DEGRADED)
- ✅ Document Signal store backup prominently
- ✅ Treat all persistence peers as adversaries

### Implementation Phases

**Phase 0** (Current):
- Single registry contract (<5K bots)
- Registry Metadata Contract for coordination
- Contract-based chunk distribution
- PoW (difficulty 18) + reputation + capacity verification
- 64KB chunks, 2 remote replicas
- 1% spot checks for fairness
- Write-blocking in DEGRADED state

**Phase 1+** (Future):
- Fibonacci-triggered sharding (5K → 8K → 13K → 21K... thresholds)
- Powers-of-2 shard counts (1 → 2 → 4 → 8 → 16...)
- Vector clock migration protocol (24h dual-read window)
- Hybrid distribution (P2P + attestations)
- Behavioral analysis for Sybil detection
- Increased challenge sampling (5%) if needed
- Hard exclusion after repeated failures

### Related Documentation

- **Spike Briefing**: `docs/spike/SPIKE-WEEK-2-BRIEFING.md`
- **Architecture Model**: `.beads/persistence-model.bead`
- **Security Constraints**: `.beads/security-constraints.bead`
- **Developer Guide**: `DEVELOPER-GUIDE.md`
- **Operator Guide**: `OPERATOR-GUIDE.md`

---

**The Reciprocal Persistence Network exists for ONE reason:**

> A Stroma bot must be able to crash, lose all local state, and fully recover its trust map from encrypted chunks held by adversarial peers who cannot read or reconstruct that data.

This enables "cows not pets" bot operations - bots are disposable, but trust networks survive.
