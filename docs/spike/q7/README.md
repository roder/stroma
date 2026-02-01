# Q7: Freenet Bot Discovery

**Risk Level**: 🔴 BLOCKING  
**Status**: PENDING

---

## WHY This Question Matters

**The Core Problem**: Bots must FIND each other to exchange chunks for the Reciprocal Persistence Network.

Without discovery:
- Bots can't find persistence peers
- No chunk distribution possible
- Crashed bots can't recover
- Trust maps are lost

**Connection to Goal**: "A crashed bot recovers from adversarial peers" requires knowing WHO those peers are.

---

## The Question

**How do Stroma bots discover each other for persistence WITHOUT federation?**

### Key Constraints

1. **Pre-Federation**: Bots are adversaries, no trust relationship exists
2. **Dynamic Network**: Bots come and go (10 to 10,000 bots)
3. **Size Awareness**: Need network size for replication requirements (N >= 3)
4. **Stale Handling**: Dead bots must eventually be removed

---

## Test Scenarios

### Scenario 1: Dedicated Registry Contract

```rust
// Well-known contract address
const PERSISTENCE_REGISTRY: ContractHash = hash("stroma-persistence-registry-v1");

pub struct RegistryEntry {
    bot_pubkey: PublicKey,
    num_chunks: u32,              // Number of chunks for this bot's state
    size_bucket: SizeBucket,      // Small/Medium/Large
    registered_at: Timestamp,
    contract_hash: ContractHash,  // Bot's trust contract
}

impl PersistenceRegistry {
    /// Register this bot for persistence network
    pub fn register(&mut self, entry: RegistryEntry) -> Result<(), Error>;
    
    /// Discover all registered bots
    pub fn discover_bots(&self) -> Vec<RegistryEntry>;
    
    /// Get network size for replication requirements
    pub fn network_size(&self) -> usize;
    
    /// Remove this bot from registry (on shutdown)
    pub fn unregister(&mut self, bot_pubkey: &PublicKey) -> Result<(), Error>;
}
```

**Note**: No heartbeats required. Bots register once and unregister on clean shutdown.
Stale detection handled by chunk holders detecting missing bots during distribution.

**Pros**:
- Simple, well-understood
- Single source of truth
- Easy to enumerate all bots

**Cons**:
- Contract must be bootstrapped
- All bots must know registry address

### Scenario 2: DHT-Based Discovery

```rust
// Content-addressed discovery
const DISCOVERY_KEY: &str = "stroma-persistence-network-v1";

impl DhtDiscovery {
    /// Publish presence to DHT
    pub async fn announce(&self) -> Result<(), Error> {
        let key = hash(DISCOVERY_KEY);
        let value = self.bot_pubkey.to_bytes();
        freenet.put(key, value).await
    }
    
    /// Discover peers via DHT
    pub async fn discover(&self) -> Vec<PublicKey> {
        let key = hash(DISCOVERY_KEY);
        freenet.get_all(key).await
    }
}
```

**Pros**:
- No special contract needed
- Uses Freenet's native capabilities

**Cons**:
- DHT may not support "get all" efficiently
- Stale handling unclear

### Scenario 3: Hybrid Approach

```rust
// Registry for authoritative list + Bloom Filter for efficiency
impl HybridDiscovery {
    /// Check if bot is registered (fast, may false positive)
    pub fn is_registered_bloom(&self, pubkey: &PublicKey) -> bool;
    
    /// Get authoritative list from registry (slower, accurate)
    pub async fn get_all_registered(&self) -> Vec<RegistryEntry>;
}
```

---

## Test Cases

### Test 1: Basic Discovery

```rust
#[test]
async fn test_bot_a_discovers_bot_b() {
    let network = SimNetwork::new();
    
    let bot_a = network.spawn_bot("A").await;
    let bot_b = network.spawn_bot("B").await;
    
    // Bot B registers
    bot_b.register_for_persistence().await;
    
    // Bot A discovers
    let peers = bot_a.discover_persistence_peers().await;
    
    assert!(peers.contains(&bot_b.pubkey()));
}
```

### Test 2: Clean Unregistration

```rust
#[test]
async fn test_bot_unregistration() {
    let network = SimNetwork::new();
    
    let bot = network.spawn_bot("ephemeral").await;
    bot.register_for_persistence().await;
    
    // Verify registered
    let peers = registry.discover_bots().await;
    assert!(peers.iter().any(|p| p.pubkey == bot.pubkey()));
    
    // Clean shutdown
    bot.unregister_from_persistence().await;
    
    // Bot should be removed
    let peers = registry.discover_bots().await;
    assert!(!peers.iter().any(|p| p.pubkey == bot.pubkey()));
}
```

**Note**: No heartbeat mechanism. Bots unregister on clean shutdown.
Stale/crashed bots detected by chunk holders (failed distributions).

### Test 3: Network Size Calculation

```rust
#[test]
async fn test_network_size() {
    let network = SimNetwork::new();
    
    // Spawn 10 bots
    for i in 0..10 {
        let bot = network.spawn_bot(&format!("bot-{}", i)).await;
        bot.register_for_persistence().await;
    }
    
    let size = registry.network_size().await;
    assert_eq!(size, 10);
    
    // Replication requirements: 3 copies per chunk (N >= 3 bots needed)
    assert!(size >= 3);
}
```

### Test 4: Concurrent Registration

```rust
#[test]
async fn test_concurrent_registration() {
    let network = SimNetwork::new();
    
    // Multiple bots register simultaneously
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let network = network.clone();
            tokio::spawn(async move {
                let bot = network.spawn_bot(&format!("bot-{}", i)).await;
                bot.register_for_persistence().await
            })
        })
        .collect();
    
    // Wait for all
    for handle in handles {
        handle.await.unwrap();
    }
    
    // All should be registered
    assert_eq!(registry.network_size().await, 10);
}
```

---

## Success Criteria

| Criterion | Requirement |
|-----------|-------------|
| Discovery latency | < 5 seconds |
| Registration overhead | < 1 KB one-time |
| Network size accuracy | Exact count |
| Concurrent safety | No duplicate/lost entries |

**Note**: No heartbeat overhead (design decision from persistence-model.bead).
Replication Health based on write-time acknowledgment, not continuous monitoring.

---

## Fallback Strategy (NO-GO)

If automated discovery fails:

**Manual Bootstrap List**
```toml
# config.toml
[persistence]
bootstrap_peers = [
    "pubkey1...",
    "pubkey2...",
    "pubkey3...",
]
```

**Impact**: Acceptable for Phase 0. Operators manually configure known peers.

---

## Files

- `main.rs` - Test harness
- `RESULTS.md` - Findings (after spike)

## Related

- [SPIKE-WEEK-2-BRIEFING.md](../SPIKE-WEEK-2-BRIEFING.md) - Full context
- [Q8: Fake Bot Defense](../q8/README.md) - Depends on registry
- [Q9: Chunk Verification](../q9/README.md) - Uses discovered peers
