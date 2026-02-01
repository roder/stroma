# State Persistence in Stroma

This document explains how Stroma ensures trust network durability through the **Reciprocal Persistence Network**.

---

## The Problem: Freenet Data Falls Off

### Why This Matters to You

Your Stroma group represents months or years of building trust relationships. Members have vouched for each other, the network has grown organically, and your community depends on this trust map to function.

**What happens if this data disappears?**

- All membership information is lost
- Vouch relationships vanish
- Your community must rebuild from scratch
- The trust network that took years to build is gone in an instant

### The Technical Reality

Freenet is designed for privacy through ephemerality. If no peers are subscribed to a contract, the data eventually falls off the network. This is a feature, not a bug - it prevents data from persisting indefinitely without consent.

But for Stroma, this creates a problem:

```
WITHOUT PERSISTENCE:
  Bot crashes → No peers subscribed → Trust map gone → Community destroyed
```

Freenet's native subscription model provides some replication, BUT:

1. **Subscriptions are voluntary** - peers may unsubscribe
2. **Data falls off** when interest wanes  
3. **No guaranteed minimum replicas** for critical data
4. **Single-writer contracts** (like Stroma's) need explicit backup strategy

---

## The Solution: Reciprocal Persistence Network

### The Core Insight

Stroma bots can replicate each other's encrypted state **without trusting each other**.

```
WITH RECIPROCAL PERSISTENCE (Chunking Model):
  Bot state → encrypted → split into 64KB chunks → each chunk gets 3 copies
  Bot crashes → collect ALL chunks (any 1 of 3 per chunk) → decrypt with ACI key → Community intact
```

### How It Works (High Level)

1. **Your bot encrypts its trust state** using keys derived from your Signal identity (ACI key)
2. **The encrypted state is split into 64KB chunks** (e.g., 500KB state → 8 chunks)
3. **Each chunk gets 3 copies**: 1 local + 2 remote replicas
4. **Chunk holders are assigned deterministically** via rendezvous hashing per-chunk (not chosen by you)
5. **Those bots hold your chunks** (but can't read them - they're encrypted)
6. **You hold chunks for other bots** (reciprocally, ~2x your state size)
7. **If you crash, restore your Signal store, compute your holders per-chunk, and collect ALL chunks** to recover

**Key Simplification**: No separate keypair to manage. Your Signal account IS your identity.

### Why This Is Secure

| What Holders CAN Do | What Holders CANNOT Do |
|---------------------|------------------------|
| Store your encrypted chunks | Decrypt your trust map |
| Validate your signature | Learn who your members are |
| Compute whose chunk they hold | Read the chunk content |
| Return chunks on request | Reconstruct state (need ALL chunks + ACI key) |

**Key Point**: Holders are assumed to be adversaries. Anyone can compute who holds whose chunks (deterministic assignment), but chunks are encrypted - so knowing the holder identity doesn't help read the data.

**Security scales with state size**: Larger trust maps = more chunks = distributed across more bots = harder to seize. A 5MB trust map split into ~80 chunks would require compromising holders of ALL 80 chunks AND obtaining your ACI private key.

---

## How It Works

### Two-Layer Architecture

Stroma uses two layers for state management:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  LAYER 1: TRUST STATE (Freenet-native)                                       │
│  ──────────────────────────────────────                                       │
│  What: Members, vouches, flags                                               │
│  How: Native Freenet ComposableState with commutative deltas                │
│  Updates: Infrequent (human timescale - when members join/leave)            │
│                                                                              │
│  LAYER 2: PERSISTENCE CHUNKS (Reciprocal Persistence Network)               │
│  ──────────────────────────────────────────────────────────────              │
│  What: Encrypted backup of Layer 1                                          │
│  How: 64KB chunks, each with 3 copies (1 local + 2 remote)                 │
│  Purpose: Recovery if Freenet data falls off or bot crashes                 │
└─────────────────────────────────────────────────────────────────────────────┘
```

### What Freenet Provides vs What We Add

**Freenet Native Capabilities (Leveraged):**

| Capability | How Stroma Uses It |
|------------|-------------------|
| Summary-Delta Sync | Trust state merges commutatively |
| Subscription Trees | Bots subscribe to contract state changes |
| Eventual Consistency | Trust state converges across network |

**What Freenet Does NOT Provide (We Add):**

| Gap | Stroma's Solution |
|-----|-------------------|
| Persistence (data falls off) | Reciprocal Persistence Network |
| Encryption at rest | Application-level AES-256-GCM |
| Seizure resistance | Chunks distributed across N bots (deterministic assignment) |
| Member count privacy | Size buckets |

### Subscription Layer: TWO SEPARATE CONCERNS

**CRITICAL**: Outbound and Inbound subscriptions are SEPARATE concerns. They are assigned independently by the algorithm.

**1. OUTBOUND SUBSCRIPTIONS (Fairness - I hold others' chunks):**
- Algorithm assigns: "Which bots' chunks must I hold?"
- I subscribe to those bots' contracts
- I receive and store their chunks (~2x my state size total)
- Target: Store approximately 2x what others store for me

**2. INBOUND SUBSCRIPTIONS (Security - Others hold MY chunks):**
- Algorithm assigns per-chunk: "Who holds chunk[0]? chunk[1]? etc." (deterministic)
- Those bots subscribe to MY contract
- They receive and store my chunks (2 replicas per chunk)
- Assignment: DETERMINISTIC (rendezvous hashing per-chunk - verifiable by anyone)

**Why Separate (even with deterministic assignment):**
- Bot-B (whose chunks I hold) ≠ holder of MY chunks
- Different chunks may have different holders (spreads risk)
- Algorithm ensures no correlation between these sets
- Holders CAN compute assignments, but chunks are ENCRYPTED

```
Bot-A's perspective (500KB state = 8 chunks):
┌────────────────────────────────────────────────────────────────────┐
│  FAIRNESS: I hold chunks FOR Bot-B, Bot-C, ... (~2x my state size)│
│  SECURITY: ~8-16 bots hold MY chunks (deterministic per-chunk)    │
│                                                                    │
│  Chunk[0]: Bot-X, Bot-Y hold replicas                             │
│  Chunk[1]: Bot-Z, Bot-W hold replicas                             │
│  ...etc (different holders per chunk = more distribution)          │
│                                                                    │
│  Anyone CAN compute assignments (but chunks are encrypted)         │
│  Need ALL chunks + ACI key to reconstruct                          │
└────────────────────────────────────────────────────────────────────┘
```

### Chunk Distribution

When your trust state changes:

1. **Version**: State change applied, version incremented (e.g., v47 → v48)
2. **Lock**: Distribution locked — new changes queued until distribution completes
3. **Encrypt**: Full state encrypted with key derived from Signal ACI identity (AES-256-GCM)
4. **Chunk**: Split into 64KB chunks (e.g., 500KB → 8 chunks)
5. **Compute holders per-chunk**: For each chunk, `rendezvous_hash(my_contract, chunk_idx, bot_list, epoch)` → 2 remote holders
6. **Distribute**: Send 2 replicas of each chunk to the computed holders
7. **Verify**: Holders sign attestations confirming they have chunks
8. **Unlock**: Apply queued changes, repeat if needed

**Why version-locked distribution**: Ensures all holders for a given version have identical chunks. Prevents fragmented chunk sets across versions.

```
Your Bot's State (500KB)
     │
     ▼
┌─────────────┐
│  Encrypt    │  AES-256-GCM (key derived from Signal ACI identity)
└─────────────┘
     │
     ▼
┌─────────────┐
│   Chunk     │  Split into 64KB chunks
└─────────────┘
     │
     ├────────────┬────────────┬─────────────┬─────────────┐
     ▼            ▼            ▼             ▼             ▼
 Chunk[0]    Chunk[1]     Chunk[2]     ...          Chunk[7]
     │            │            │             │             │
     ▼            ▼            ▼             ▼             ▼
Bot-X,Y      Bot-Z,W      Bot-M,N      ...          Bot-P,Q
(computed)   (computed)   (computed)   (computed)   (computed)

← Rendezvous hashing assigns 2 holders per chunk
← Different chunks may have different holders (more distribution)
← Need ALL chunks + ACI key to reconstruct
```

### Recovery Process

When you need to recover (bot crash, server loss, etc.):

```rust
// Pseudocode for recovery
async fn recover_state() -> Result<TrustState> {
    // 1. Restore your Signal protocol store from backup
    let signal_store = restore_signal_store_from_backup()?;
    let aci_identity = signal_store.get_identity_key_pair().await?;
    
    // 2. Get registry info
    let (bot_list, epoch, num_chunks) = registry.get_my_info().await?;
    
    // 3. For each chunk, compute holders and fetch from any 1 of 3 copies
    let mut chunks = Vec::with_capacity(num_chunks);
    for chunk_idx in 0..num_chunks {
        let holders = rendezvous_hash(&my_contract, chunk_idx, &bot_list, epoch);
        // Try holders until we get the chunk (any 1 of 3 works)
        let chunk = fetch_from_any(&holders, chunk_idx).await?;
        chunks.push(chunk);
    }
    
    // 4. Concatenate chunks, decrypt, verify signature
    let encrypted_state = concatenate(&chunks);
    let encryption_key = derive_key_from_aci(&aci_identity);
    let state = decrypt(&encrypted_state, &encryption_key)?;
    verify_signature(&state, &aci_identity)?;
    
    Ok(state)
}
```

**Key Point**: Your Signal protocol store IS your recovery key. No separate keypair file needed.

### Replication Health: Is My Data Resilient?

Every member can check if their trust network data is safely replicated:

```
User → Bot: /mesh replication

Bot → User:
"💾 Replication Health: 🟢 Replicated

Last State Change: 3 hours ago (Alice joined)
State Size: 512KB (8 chunks)
Chunks Replicated: 8/8 fully (all 3 copies per chunk) ✅
State Version: 47

Recovery Confidence: ✅ Yes — all chunks available from multiple holders

💡 Your trust network is resilient. If this bot goes offline,
the state can be recovered from chunk holders."
```

#### Replication Health States

| Status | Meaning | Recovery | User Message |
|--------|---------|----------|--------------|
| 🟢 **Replicated** | All chunks have 3 copies | ✅ Guaranteed | "Fully resilient" |
| 🟡 **Partial** | Some chunks degraded (2/3 copies) | ✅ Possible | "Recoverable but degraded" |
| 🔴 **At Risk** | Any chunk has ≤1 copy | ❌ At risk | "Cannot recover if crash" |
| 🔵 **Initializing** | New bot, establishing | — | "Setting up persistence" |

#### How It Works

Replication Health is measured when state changes happen (not via continuous monitoring):

1. **State changes** (vouch, flag, member join)
2. **Bot encrypts and chunks** the new state
3. **Distributes 2 replicas per chunk** to computed holders (rendezvous hashing per-chunk)
4. **Records per-chunk success**
5. **Health = chunks with 2+ replicas / total chunks**

This is simple and doesn't require heartbeats — we just track whether the last replication worked.

### Write-Blocking States

Your bot has different states based on replication health:

| State | Meaning | Can Write? | Replication Health |
|-------|---------|------------|-------------------|
| **ACTIVE** | All chunks have 2+ replicas | Yes | 🟢 or 🟡 |
| **PROVISIONAL** | No suitable peers available yet | Yes (warned) | 🔵 |
| **DEGRADED** | Any chunk ≤1 replica, peers available | **No** | 🔴 |
| **ISOLATED** | Only bot in network (N=1) | Yes (at risk) | 🔵 |

**Why DEGRADED blocks writes**: If distribution failed and peers exist, the bot must succeed before making more changes. This prevents accumulating state that can't be backed up.

### Network Bootstrap Limitations

| Network Size | Persistence Guarantee | Notes |
|--------------|----------------------|-------|
| **N=1** | None | State pushed to Freenet only — "good luck" |
| **N=2** | Fragile | Mutual dependency — both need each other |
| **N=3** | Minimal | One failure = degraded |
| **N≥4** | Resilient | Can tolerate 1 failure per chunk |
| **N≥5** | Recommended | Comfortable margin for production |

### Partial Recovery

**If any chunk is permanently lost, full state is unrecoverable.**

This is a deliberate design decision:
- Encryption requires ALL chunks + ACI key
- Partial ciphertext is useless
- No graceful degradation possible

Mitigations: 3x replication per chunk, deterministic fallback holders, larger states = more distribution.

---

## Federation vs Persistence Timeline

Understanding when persistence and federation happen in a bot's lifecycle:

```
1. Bot-A joins network (500KB state = 8 chunks):
   - Registers with Persistence Registry (adds to bot list, num_chunks, epoch increments)
   - For each chunk: computes 2 holders via rendezvous_hash(A, chunk_idx, bot_list, epoch)
   - IF holders reachable: Distributes chunks, enters ACTIVE
   - IF no peers available: Enters PROVISIONAL (writes allowed with warning)
   - Bot-A can WRITE to Freenet ✓

2. Bot-A establishes chunk holders (SECURITY):
   - Algorithm assigns holders per-chunk (deterministic, different holders per chunk)
   - Anyone CAN compute who holds A's chunks (public algorithm)
   - BUT: Chunks are ENCRYPTED (holders can't read content)
   - Need ALL chunks + ACI key to reconstruct
   - Transitions PROVISIONAL → ACTIVE

3. Bot-A fulfills fairness obligations (SEPARATE CONCERN):
   - Computes: "Which bots' chunks must I hold?" (reverse query)
   - Bot-A holds chunks FOR Bot-B, Bot-C, ... (~2x state size total)
   - This is DIFFERENT from who holds Bot-A's chunks
   - Fairness ratio tracked: total_stored / my_state_size ≈ 2x

4. Later, Bot-A federates with Bot-D (OPTIONAL):
   - Shadow Beacon discovers shared validators
   - PSI-CA confirms overlap threshold
   - Groups vote to federate
   - Bot-A subscribes to Bot-D for ZK queries (ADDITIONAL subscription)
   - Federation is BONUS persistence, not primary

5. Result:
   - Bot-A has chunk holders distributed across ~8-16 bots (adversarial, security)
   - Bot-A has fairness coordination with comparable bots (accounting)
   - Bot-A MAY have federation peer (trusted, optional bonus)
   - These THREE relationships are SEPARATE
```

**Key Insights:**

1. **Federations are TRANSITORY** - they can dissolve if shared validators leave
2. **Persistence peers are the STABLE FOUNDATION** - doesn't depend on trust relationships
3. **Bot is never penalized for network scarcity** - only blocked when it COULD fix its replica set but hasn't

---

## For Operators

### Critical: Backup Your Signal Protocol Store

**Your Signal protocol store IS your recovery identity.** No separate keypair file or group pepper needed.

The bot uses your Signal account's **ACI (Account Identity) key** for:
- Chunk encryption (AES-256-GCM key derived from ACI via HKDF)
- State signatures (using ACI identity key)
- Identity masking (HMAC key derived from ACI via HKDF)
- Persistence network identification

Without your Signal store backup:
- You cannot decrypt recovered chunks
- You cannot verify identity hashes
- Your trust map is permanently lost
- There is NO recovery path

**Best Practices**:

```bash
# Signal protocol store location
/var/lib/stroma/signal-store/

# Backup to secure location
tar -czf /secure-backup/stroma-signal-store-$(date +%Y%m%d).tar.gz /var/lib/stroma/signal-store/

# Store backup:
# - Encrypted USB drive in safe location
# - Hardware security module (HSM)
# - Secure cloud backup (encrypted)
# - NOT on the same server as the bot
```

**What's in the Signal store:**
- ACI identity keypair (your cryptographic identity for persistence)
- PNI identity keypair (phone number identity)
- Session keys and pre-keys

**What's NOT stored (no backup needed):**
- Message history (ephemeral by design)
- Contact database (not used)

### Recovery Procedure

If your bot crashes and loses local state:

1. **Restore bot from backup** or install fresh
2. **Restore Signal protocol store** from secure backup
3. **Start bot** - it will automatically:
   - Load ACI identity from Signal store
   - Query registry for bot list, epoch, num_chunks
   - For each chunk, compute holders and fetch (any 1 of 3 per chunk)
   - Concatenate chunks, decrypt, verify state
   - Resume normal operation

```bash
# Example recovery
tar -xzf /secure-backup/stroma-signal-store-YYYYMMDD.tar.gz -C /var/lib/stroma/
stroma-bot recover
```

### Monitoring

Watch for these alerts:

| Alert | Meaning | Action |
|-------|---------|--------|
| `persistence.state=DEGRADED` | Some chunk has ≤1 replica | Bot will find new holder automatically |
| `persistence.state=PROVISIONAL` | No suitable peers | Wait for network growth |
| `persistence.chunk_degraded` | One or more chunks underreplicated | Check network connectivity |
| `persistence.verification_failed` | Holder may have deleted | Bot will find replacement |

### Configuration

```toml
# /etc/stroma/config.toml

[persistence]
# Chunk size in bytes (default: 64KB)
chunk_size = 65536

# Minimum replicas per chunk (default: 3 = 1 local + 2 remote)
replication_factor = 3

# Signal protocol store location (contains your identity)
signal_store_path = "/var/lib/stroma/signal-store"
```

**Note**: No separate keypair file needed — your Signal identity IS your persistence identity. No heartbeat mechanism required. Replication Health is measured at write time based on successful chunk distribution acknowledgments.

---

## For Developers

### Module Structure

```
src/
├── persistence/
│   ├── mod.rs              # Public API
│   ├── encryption.rs       # AES-256-GCM, Ed25519 signatures
│   ├── chunking.rs         # Split/join encrypted state into 64KB chunks
│   ├── registry.rs         # Persistence peer discovery
│   ├── verification.rs     # Challenge-response verification
│   ├── recovery.rs         # State recovery from chunks
│   └── write_blocking.rs   # State machine (ACTIVE/DEGRADED/etc.)
```

### Key Types

```rust
/// Encrypted trust state ready for chunking
pub struct EncryptedTrustState {
    ciphertext: Vec<u8>,           // AES-256-GCM encrypted (key from Signal ACI)
    signature: Signature,           // Signed with Signal ACI identity key
    bot_pubkey: PublicKey,         // Signal ACI public key for verification
    member_merkle_root: Hash,      // For ZK-proof verification
    version: u64,                  // Monotonic, anti-replay
    previous_hash: Hash,           // Chain integrity
    timestamp: Timestamp,
}

/// A single chunk
pub struct Chunk {
    data: Vec<u8>,                 // 64KB of encrypted data
    chunk_index: u32,              // Position in sequence
    chunk_hash: Hash,              // For verification
    version: u64,                  // Must match other chunks
}

/// Registry records (minimal - deterministic assignment)
pub struct RegistryEntry {
    contract_hash: ContractHash,   // Bot's trust contract address
    size_bucket: SizeBucket,       // For fairness estimation
    num_chunks: u32,               // For recovery
    registered_at: Timestamp,
}

/// Chunk holders computed via rendezvous hashing (not stored)
fn compute_chunk_holders(
    owner: &ContractHash,
    chunk_index: u32,
    bots: &[ContractHash],
    epoch: u64,
) -> [ContractHash; 2] {
    // Deterministic - anyone can compute
    // See persistence-model.bead for algorithm
}
```

### Integration Points

**When trust state changes** (vouch, flag, member join/leave):

```rust
// In trust state module
impl TrustState {
    pub async fn apply_delta(&mut self, delta: Delta) -> Result<()> {
        // 1. Apply to local state
        self.members.apply(&delta)?;
        
        // 2. Persist to Freenet (Layer 1)
        freenet.update_state(&delta).await?;
        
        // 3. Update persistence chunks (Layer 2)
        persistence.on_state_change(&self).await?;
        
        Ok(())
    }
}

// In persistence module
impl PersistenceManager {
    pub async fn on_state_change(&self, state: &TrustState) -> Result<()> {
        // Check write-blocking state
        if self.state == BotState::DEGRADED {
            return Err(Error::WriteBlocked);
        }
        
        // Encrypt and chunk
        let encrypted = self.encrypt(state)?;
        let chunks = self.chunk(&encrypted)?;
        
        // Distribute 2 replicas per chunk to computed holders
        self.distribute_chunks(chunks).await?;
        
        Ok(())
    }
}
```

### Testing

```rust
#[tokio::test]
async fn test_full_recovery_cycle() {
    // Setup
    let network = SimNetwork::new();
    let bot = network.spawn_bot("test").await;
    
    // Create some trust state
    bot.add_member("alice").await;
    bot.add_member("bob").await;
    bot.vouch("alice", "bob").await;
    
    // Verify chunks distributed
    assert!(bot.persistence().all_chunks_replicated());
    
    // Simulate crash (lose local state)
    let keypair = bot.keypair().clone();
    drop(bot);
    
    // Recover
    let recovered_bot = Bot::recover(&keypair, &network).await.unwrap();
    
    // Verify state matches
    assert!(recovered_bot.has_member("alice"));
    assert!(recovered_bot.has_member("bob"));
    assert!(recovered_bot.has_vouch("alice", "bob"));
}
```

---

## Security Model

### Threat Model

**Primary Threat**: Adversarial persistence peers

| Threat | Mitigation |
|--------|------------|
| Peer reads trust map | AES-256-GCM encryption with owner's key |
| Peer reconstructs state | Need ALL chunks + ACI private key |
| Peers collude | Chunks distributed across many bots, need ALL |
| Peer deletes chunk | Verification (challenge-response), 3 copies per chunk |
| Peer forges attestation | Challenge-response proves possession |

### What Adversary Learns

An adversarial chunk holder learns:

| Information | Leaked? |
|-------------|---------|
| A chunk exists | Yes (unavoidable) |
| Whose chunk it is | Yes (deterministic assignment) |
| Chunk contents | No (encrypted) |
| Other holders | Yes (deterministic, computable) |
| Trust map structure | No (fully encrypted, need ALL chunks + ACI) |
| Member identities | No (HMAC-hashed in trust state anyway) |

**Key insight**: Holder identities are computable (deterministic assignment), but chunks are encrypted. Security comes from encryption + needing ALL chunks + ACI key, not from hiding who holds what.

### Security Guarantees

1. **Confidentiality**: Chunk holders cannot read trust map
2. **Integrity**: Recovered state verified by signature chain
3. **Availability**: Any 1 of 3 copies per chunk sufficient
4. **Durability**: Chunks persist across bot crashes
5. **Distribution**: Larger states = more chunks = harder to seize

### Comparison to Alternatives

| Approach | Trust Requirement | Recovery | Privacy |
|----------|------------------|----------|---------|
| Centralized backup | Trust backup provider | Single point | Provider sees all |
| Federated peers only | Trust federation partners | Depends on federation | Partners may learn |
| **Reciprocal Persistence** | Zero trust (adversarial) | Any 1 of 3 per chunk | Cryptographic privacy |

---

## FAQ

### Why not just use cloud backup?

Cloud backup requires trusting the cloud provider with your unencrypted trust map. The Reciprocal Persistence Network provides the same durability with zero trust - holders cannot read your data.

### What if all chunk holders go offline?

Each chunk has 3 copies (1 local + 2 remote). For recovery, you need any 1 of the 3 copies per chunk. If ALL copies of a single chunk are unavailable, you cannot recover.

Mitigation: Deterministic assignment spreads chunks across many bots. With 8 chunks, you might have holders across ~8-16 bots. All of them being offline simultaneously is unlikely.

### Can I run a single bot without persistence?

Yes, but your state can fall off Freenet if no one is subscribed. The bot will warn you (`persistence.state=ISOLATED`). For production use, at least 3 bots in the persistence network is recommended.

### How much storage does holding chunks require?

Trust state is typically small (members + vouches + flags). For a 1000-member group:
- Trust state: ~50-100 KB
- Encrypted + chunked: ~64KB (1 chunk)
- 2x fairness: ~128KB stored for others

Storage burden is minimal even for resource-constrained operators. The 2x fairness ratio means you store about twice your own state size for other bots.

### What happens during network partition?

If your bot is partitioned from chunk holders:
- Local operations continue (state changes accumulate)
- Persistence updates queue until connectivity restored
- On reconnection, chunks are updated
- No data loss if partition is temporary

---

## Related Documentation

- [DEVELOPER-GUIDE.md](DEVELOPER-GUIDE.md) - Full development guide
- [OPERATOR-GUIDE.md](OPERATOR-GUIDE.md) - Running a Stroma bot
- [THREAT-MODEL-AUDIT.md](THREAT-MODEL-AUDIT.md) - Security analysis
- [FEDERATION.md](FEDERATION.md) - Federation (distinct from persistence)
- [Spike Week 2](spike/SPIKE-WEEK-2-BRIEFING.md) - Technical validation

---

## Summary

**The Reciprocal Persistence Network exists for ONE reason:**

> A Stroma bot must be able to crash, lose all local state, and fully recover its trust map from encrypted chunks held by adversarial peers who cannot read or reconstruct that data.

This enables "cows not pets" bot operations - bots are disposable, but trust networks survive.
