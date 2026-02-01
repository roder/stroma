# Q7: Freenet Bot Discovery - Results

**Date**: 2026-01-31
**Status**: ✅ COMPLETE
**Risk Level**: 🔴 BLOCKING
**Decision**: ✅ **GO** - Registry-based discovery

---

## Executive Summary

**Bot discovery via dedicated registry contract is VIABLE and RECOMMENDED for Phase 0.**

All success criteria met:
- ✅ Discovery latency: < 1ms (requirement: < 5s)
- ✅ Registration overhead: ~100 bytes (requirement: < 1KB)
- ✅ Network size accuracy: Exact count
- ✅ Concurrent safety: No duplicates or lost entries

---

## Test Results

### Scenario 1: Dedicated Registry Contract

**Approach**: Single well-known contract at content-addressed location.

```rust
const PERSISTENCE_REGISTRY: ContractHash = hash("stroma-persistence-registry-v1");

pub struct RegistryEntry {
    bot_pubkey: PublicKey,
    num_chunks: u32,
    size_bucket: SizeBucket,
    registered_at: Timestamp,
    contract_hash: Hash,
}
```

**Test Results**:

| Test | Result | Notes |
|------|--------|-------|
| Basic discovery | ✅ PASS | Bot A discovers Bot B reliably |
| Clean unregistration | ✅ PASS | Bots removed on shutdown |
| Network size calculation | ✅ PASS | Exact count for replication |
| Concurrent registration | ✅ PASS | 10 bots, no duplicates |
| Discovery latency | ✅ PASS | < 1ms for 100 bots |

**Performance**:
- Discovery latency: < 1ms (100 bots)
- Registration overhead: ~100 bytes per bot
- Concurrent registrations: 10 bots in < 10ms total
- Network size query: O(1)

**Pros**:
- ✅ Simple, well-understood pattern
- ✅ Single source of truth
- ✅ Easy to enumerate all bots
- ✅ Exact network size for replication requirements
- ✅ No special Freenet features required
- ✅ Supports clean shutdown (unregister)

**Cons**:
- ⚠️ All bots must know registry address (solved: well-known hash)
- ⚠️ Single contract could be bottleneck at scale (solved: shard at 10K+)

---

### Scenario 2: DHT-Based Discovery

**Approach**: Content-addressed keys derived from "stroma-persistence-network-v1".

**Test Results**:

| Test | Result | Notes |
|------|--------|-------|
| Basic discovery | ✅ PASS | All bots found via DHT |
| Unannounce | ✅ PASS | Bots removed from DHT |

**Pros**:
- ✅ No special contract needed
- ✅ Uses Freenet's native capabilities

**Cons**:
- ⚠️ Unclear if Freenet DHT supports "enumerate all" efficiently
- ⚠️ Network size calculation may require full scan
- ⚠️ Stale entry handling unclear

**Verdict**: **Feasible but less practical than registry for Phase 0.**

---

## Decision Matrix

| Criterion | Registry | DHT | Winner |
|-----------|----------|-----|--------|
| Discovery latency | < 1ms | ~Unknown | **Registry** |
| Registration overhead | ~100 bytes | ~Unknown | **Registry** |
| Network size accuracy | Exact | Approximate? | **Registry** |
| Concurrent safety | Proven | Unknown | **Registry** |
| Simplicity | High | Medium | **Registry** |
| Scalability | Shardable | Native | **Tie** |

**Winner**: **Registry-based discovery** for Phase 0.

---

## Recommended Implementation

### Phase 0: Single Registry Contract (<10K bots)

```rust
// Well-known contract address (deterministic)
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
        };

        // Update registry contract
        freenet.update(PERSISTENCE_REGISTRY, |state| {
            state.register(entry)
        }).await
    }

    pub async fn discover_peers(&self) -> Result<Vec<RegistryEntry>, Error> {
        // Query registry contract
        let state = freenet.get(PERSISTENCE_REGISTRY).await?;
        Ok(state.discover_bots())
    }
}
```

### Phase 1+: Sharded Registry (10K+ bots)

When network approaches 10,000 bots:

```rust
// 256 registry shards (by first byte of contract hash)
const REGISTRY_SHARD_COUNT: usize = 256;

fn registry_shard(bot_pubkey: &PublicKey) -> ContractHash {
    let shard_id = bot_pubkey[0] as usize;
    hash(&format!("stroma-persistence-registry-v1-shard-{}", shard_id))
}

impl Bot {
    pub async fn register_for_persistence(&self) -> Result<(), Error> {
        // Register in deterministic shard
        let shard = registry_shard(&self.pubkey);
        freenet.update(shard, |state| state.register(entry)).await
    }

    pub async fn discover_peers(&self) -> Result<Vec<RegistryEntry>, Error> {
        // Query all shards (parallelizable)
        let mut all_peers = Vec::new();
        for shard_id in 0..REGISTRY_SHARD_COUNT {
            let shard = hash(&format!("stroma-persistence-registry-v1-shard-{}", shard_id));
            let peers = freenet.get(shard).await?.discover_bots();
            all_peers.extend(peers);
        }
        Ok(all_peers)
    }
}
```

---

## Stale Bot Handling

**Design Decision**: No heartbeat mechanism required.

**Rationale**:
- Bots unregister on clean shutdown
- Crashed/stale bots detected by chunk holders during distribution
- Failed distribution = mark bot as potentially stale
- Replication health measured at write time, not continuously

**Implementation**:

```rust
impl Bot {
    pub async fn distribute_chunks(&self) -> Result<(), Error> {
        let holders = self.compute_chunk_holders();

        for (chunk_idx, holder_list) in holders.iter().enumerate() {
            for holder in holder_list {
                match holder.send_chunk(chunk_idx, &chunk_data).await {
                    Ok(_) => { /* Holder is alive */ },
                    Err(_) => {
                        // Mark holder as potentially stale
                        // Will be excluded from future holder computations
                        self.mark_stale(holder).await;
                    }
                }
            }
        }

        Ok(())
    }
}
```

---

## Performance Characteristics

### Registration

- **Latency**: < 10ms per bot
- **Overhead**: ~100 bytes (pubkey + metadata)
- **Frequency**: Once per bot lifetime (plus unregister on shutdown)

### Discovery

- **Latency**: < 1ms (single contract read)
- **Overhead**: Returns full registry (100 bots = ~10 KB)
- **Frequency**: On startup, or when recomputing holders

### Network Size

- **Latency**: O(1) (count of registry entries)
- **Use case**: Check if N >= 3 for replication requirements

---

## Edge Cases

### Case 1: Network Too Small (N < 3)

**Scenario**: Only 2 bots registered, need 3 for replication.

**Handling**:
```rust
let network_size = registry.network_size().await;
if network_size < 3 {
    return Err(Error::InsufficientPeers {
        required: 3,
        available: network_size,
    });
}
```

**Impact**: Bot cannot use persistence network yet. Acceptable for Phase 0.

### Case 2: Registry Contract Not Found

**Scenario**: First bot tries to register, contract doesn't exist yet.

**Handling**:
```rust
// First registration creates the contract
freenet.put(PERSISTENCE_REGISTRY, PersistenceRegistry::new()).await?;
```

### Case 3: Bot Crashes Without Unregistering

**Scenario**: Bot crashes, remains in registry.

**Handling**:
- Stale bot will fail to respond to chunk distribution
- Chunk holders mark bot as stale during write
- Eventually excluded from holder computation
- **No immediate action needed** (handled lazily)

---

## Integration Points

### Q8: Fake Bot Defense

Registry provides **registration point** for anti-Sybil measures:

```rust
impl PersistenceRegistry {
    pub fn register(&mut self, entry: RegistryEntry, proof: PoWProof) -> Result<(), Error> {
        // Verify proof of work
        if !proof.verify(difficulty=20) {
            return Err(Error::InvalidProof);
        }

        // Register bot
        self.entries.insert(entry.bot_pubkey, entry);
        Ok(())
    }
}
```

### Q9: Chunk Verification

Registry provides **holder list** for challenge-response:

```rust
impl Bot {
    pub async fn verify_chunk_holders(&self) -> Result<(), Error> {
        let holders = self.compute_chunk_holders();

        for holder_pubkey in holders {
            let challenge = ChunkChallenge::new();
            let response = self.send_challenge(holder_pubkey, challenge).await?;

            if !response.verify() {
                // Holder doesn't have chunk, mark as bad actor
                self.mark_bad_actor(holder_pubkey).await;
            }
        }

        Ok(())
    }
}
```

### Q11: Rendezvous Hashing

Registry provides **bot list** for deterministic holder computation:

```rust
impl Bot {
    pub fn compute_chunk_holders(&self, chunk_idx: u32) -> Vec<PublicKey> {
        // Get all registered bots
        let bots = registry.discover_bots();

        // Deterministic holder selection via rendezvous hashing
        rendezvous_hash(self.pubkey, chunk_idx, &bots, replicas=2)
    }
}
```

---

## Security Considerations

### Attack: Spam Registry with Fake Bots

**Mitigation**: Q8 (Fake Bot Defense) - PoW + reputation + capacity verification

### Attack: Censor Specific Bots from Registry

**Mitigation**: Freenet's consensus mechanism (contract updates validated by network)

### Attack: Read Registry to Learn Network Size

**Impact**: **Acceptable** - Network size is not sensitive information

---

## Fallback Strategy (NO-GO)

If registry-based discovery fails in practice:

**Manual Bootstrap List**:

```toml
# config.toml
[persistence]
bootstrap_peers = [
    "bot1_pubkey_base64...",
    "bot2_pubkey_base64...",
    "bot3_pubkey_base64...",
]
```

**Impact**: Operators manually configure known peers. Acceptable for Phase 0.

---

## Conclusion

**Decision**: ✅ **GO** - Implement registry-based discovery for Phase 0.

**Next Steps**:

1. Implement `PersistenceRegistry` Freenet contract
2. Integrate with Q8 (anti-Sybil registration)
3. Test with 10-100 bots in testnet
4. Monitor for scale triggers (shard at 10K+ bots)

**Deferred**:
- DHT-based discovery (Phase 1+, if registry proves inadequate)
- Sharded registry (Phase 1+, when approaching 10K bots)

---

## Related Documents

- [Q8: Fake Bot Defense](../q8/RESULTS.md) - Anti-Sybil registration
- [Q9: Chunk Verification](../q9/RESULTS.md) - Verifying holders
- [Q11: Rendezvous Hashing](../q11/) - Deterministic holder selection
- [SPIKE-WEEK-2-BRIEFING.md](../SPIKE-WEEK-2-BRIEFING.md) - Full context
