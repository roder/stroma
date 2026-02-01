# Spike Week 2: Reciprocal Persistence Network

**Duration**: 3-5 days  
**Objective**: Validate persistence layer technologies for the Reciprocal Persistence Network
**Continues from**: `SPIKE-WEEK-BRIEFING.md` (Q1-Q6 COMPLETE)

---

## Context: WHY This Spike Exists

### The Core Problem: Freenet Data Falls Off

**Freenet does not guarantee persistence.** If no peers are subscribed to a contract, the data disappears. This is a fundamental property of Freenet's architecture - it's a feature (privacy through ephemerality) not a bug.

**But Stroma's trust maps MUST persist.** The trust network (members, vouches, flags) represents relationships built over months or years. If a bot crashes and no peers hold the state, the entire trust network is lost - catastrophic for the community.

### The Solution: Reciprocal Persistence Network

Stroma bots replicate each other's **encrypted state chunks** even though they are adversaries:

```
WITHOUT PERSISTENCE:
  Bot crashes → No peers subscribed → Trust map gone → Community destroyed

WITH RECIPROCAL PERSISTENCE (Chunking Model):
  Bot state (500KB) → 8 encrypted chunks × 64KB each
  Each chunk → 3 copies (1 local + 2 remote replicas)
  Bot crashes → Collect ALL chunks from remote holders → Decrypt with ACI key → Community intact
```

**Security through distribution**: Larger states = more chunks = more distribution = harder to seize.

### Why Not Just Use Freenet's Native Replication?

Freenet's subscription model provides some replication, BUT:

1. **Subscriptions are voluntary** - peers may unsubscribe
2. **Data falls off** when interest wanes
3. **No guaranteed minimum replicas** for critical data
4. **Single-writer contracts** need explicit backup strategy

### This Spike Validates

The discovery, security, and verification mechanisms needed for the Reciprocal Persistence Network.

---

## Risk Classification

| Question | Risk Level | Status | Fallback Strategy |
|----------|-----------|--------|-------------------|
| Q7: Bot Discovery | 🔴 BLOCKING | PENDING | Manual bootstrap list |
| Q8: Fake Bot Defense | 🟡 RECOVERABLE | PENDING | Rate limiting + reputation |
| Q9: Chunk Verification | 🟡 RECOVERABLE | PENDING | Trust-on-first-verify |
| Q10: Federation Discovery Efficacy | 🟢 DEFERRABLE | DEFERRED | Top-N (current design) |
| Q11: Rendezvous Hashing for Chunks | 🟡 RECOVERABLE | PENDING | Registry-based fallback |
| Q12: Chunk Size Optimization | 🟡 RECOVERABLE | PENDING | 64KB default |
| Q13: Fairness Verification | 🟡 RECOVERABLE | PENDING | Soft reputation enforcement |
| Q14: Chunk Communication Protocol | 🟡 RECOVERABLE | PENDING | Freenet contracts (Option A) |

**Test Priority**: BLOCKING question first. If Q7 fails, persistence network requires manual configuration.

**Note**: Q10 is deferred. Phase 4 uses top-N discovery; revisit only if discovery proves inadequate.

**Persistence model**: State is split into 64KB chunks, each replicated 3x (1 local + 2 remote). Q12 validates chunk size, Q13 validates fairness verification, Q14 validates chunk transmission.

---

## Q7: Freenet Bot Discovery (🔴 BLOCKING)

**Question**: How do Stroma bots discover each other for persistence WITHOUT federation?

**Why Critical**: Before federation, bots are adversaries. They need to find each other to exchange fragments, but have no prior knowledge of each other's existence.

**Connection to WHY**: Bots must FIND each other to exchange fragments. Without discovery, no persistence network.

### Test Scenarios

1. **Single Registry Contract** (Phase 0, <10K bots)
   - All Stroma bots publish to a well-known contract address
   - Contract tracks: bot contract_hash, size bucket, registered_at, epoch
   - Pros: Simple, reliable
   - Cons: SPOF and bottleneck at scale
   - **Note**: No heartbeat mechanism (see persistence-model.bead)

2. **Sharded Registry** (Scale trigger: 10K+ bots)
   - 256 registry shards (by first byte of contract hash)
   - Each bot registers in its deterministic shard
   - Holder computation queries all shards (parallelizable)
   - Pros: No SPOF, scales to millions
   - Cons: More contracts to manage, slightly more complex queries
   - **Recommended for production**

3. **DHT-Based Discovery** (Alternative)
   - Bots publish to content-addressed keys derived from "stroma-persistence-network"
   - Discovery via Freenet's native DHT
   - Pros: No special contract needed
   - Cons: May be harder to enumerate all bots, probabilistic

**Recommendation**: Start with single registry (Phase 0), implement sharding when approaching 10K bots.

### Test Implementation

```rust
// Test 1: Can Bot A discover Bot B?
async fn test_discovery_basic() {
    let bot_a = spawn_bot("A").await;
    let bot_b = spawn_bot("B").await;
    
    // Bot B registers
    bot_b.register_for_persistence().await;
    
    // Bot A discovers
    let peers = bot_a.discover_persistence_peers().await;
    assert!(peers.contains(&bot_b.pubkey()));
}

// Test 2: Clean unregistration
async fn test_clean_unregistration() {
    let bot_a = spawn_bot("A").await;
    bot_a.register_for_persistence().await;
    
    // Clean shutdown
    bot_a.unregister_from_persistence().await;
    
    let registry = get_registry().await;
    assert!(!registry.contains(&bot_a.pubkey()));
}
// Note: Stale/crashed bots detected by chunk holders during distribution

// Test 3: Network size calculation
async fn test_network_size() {
    spawn_bots(10).await;
    
    let size = calculate_network_size().await;
    assert_eq!(size, 10);
}
```

### Success Criteria

- [ ] Bot A can discover Bot B without prior knowledge
- [ ] Clean unregistration on bot shutdown
- [ ] Network size calculable for replication requirements (N >= 3 check)
- [ ] Discovery works across Freenet network partitions (eventually)

**Note**: No heartbeat mechanism. Replication Health measured at write time via distribution acknowledgments.

### Deliverable

`q7/RESULTS.md` with:
- Chosen approach (registry, DHT, or hybrid)
- Performance characteristics (discovery latency, registration overhead)
- Edge cases (network partition, crashed bots)
- Architectural implications

---

## Q8: Fake Bot Defense (🟡 RECOVERABLE)

**Question**: How to prevent fake bot registration diluting the network?

**Why Matters**: Attacker could register thousands of fake bots to:
- Become chunk holders (denial of service on recovery)
- Skew network size calculations (forcing insufficient replication)
- Pollute the registry (making discovery slow/expensive)

**Connection to WHY**: Attackers mustn't become chunk holders. If they do, they can refuse to return chunks during recovery (DoS attack).

### Attack Scenario

```
Attacker registers 1000 fake "bots" (just pubkeys, no real state)
  ↓
Fake bots selected as chunk holders for real Bot-A
  ↓
Bot-A crashes, tries to recover
  ↓
Fake bots don't respond (no real chunk stored)
  ↓
Recovery fails → Trust map lost
```

### Test Scenarios

1. **Proof of Work**
   - Registration requires solving computational puzzle
   - Cost: CPU time per registration
   - Pros: Economic cost to Sybil attack
   - Cons: Penalizes legitimate RPi operators

2. **Stake Mechanism**
   - Registration requires burning tokens or locking collateral
   - Cost: Economic value per registration
   - Pros: Strong Sybil resistance
   - Cons: Requires token economy (complexity)

3. **Reputation Accumulation**
   - New bots start "untrusted" (not selected as chunk holders)
   - Trust builds over time via successful chunk returns
   - Pros: No upfront cost, organic trust
   - Cons: Slow bootstrap for new bots

4. **Size Verification**
   - Bot must prove it's actually holding state (challenge-response)
   - Bots with no real state can't pass verification
   - Pros: Directly addresses attack vector
   - Cons: Privacy concerns (reveals state existence)

### Test Implementation

```rust
// Test 1: PoW registration cost
async fn test_pow_registration() {
    let start = Instant::now();
    let proof = compute_registration_pow(difficulty=20);
    let elapsed = start.elapsed();
    
    // Should take ~1 second on average hardware
    assert!(elapsed > Duration::from_millis(500));
    assert!(elapsed < Duration::from_secs(5));
    
    // Verify proof
    assert!(verify_registration_pow(&proof));
}

// Test 2: Reputation-based selection
async fn test_reputation_selection() {
    let new_bot = spawn_bot("new").await;
    let established_bot = spawn_bot("established").await;
    
    // Established bot has history
    established_bot.record_successful_chunk_return().await;
    
    // New bot should NOT be selected as holder (yet)
    let holders = compute_chunk_holders(&bot_a, 0, &bots, epoch);
    // Reputation system may deprioritize new bots
}
```

### Success Criteria

- [ ] Cost of fake registration > benefit to attacker
- [ ] Legitimate bots not penalized (RPi can still register)
- [ ] Sybil attack bounded (attacker can't dominate network cheaply)
- [ ] Defense mechanism doesn't reveal trust map contents

### Deliverable

`q8/RESULTS.md` with:
- Chosen defense mechanism(s)
- Cost/benefit analysis for attacker
- Impact on legitimate small operators
- Integration with registry

---

## Q9: Chunk Verification (🟡 RECOVERABLE)

**Question**: How to verify a holder ACTUALLY has a chunk without revealing content?

**Why Matters**: Registry says "Bot X holds my chunk[3]" but we need cryptographic proof that:
1. Bot X actually stored the chunk
2. Bot X still has the chunk (didn't delete it)
3. The chunk is intact (not corrupted)

**Connection to WHY**: Must PROVE chunks exist before trusting recovery. Otherwise, bot might think it has chunk holders, but some deleted their chunks.

### Security Constraint

Holder CANNOT:
- Learn chunk content from verification challenge
- Forge attestation without actually having the chunk
- Replay old attestations for deleted chunks

### Test Scenarios

1. **Challenge-Response**
   - Owner sends random challenge (e.g., "hash bytes 100-200 with nonce X")
   - Holder must respond with correct hash
   - Pros: Proves possession, fresh (nonce prevents replay)
   - Cons: Reveals structure (holder learns chunk has bytes 100-200)

2. **Merkle Proof of Chunk**
   - Owner publishes Merkle root of chunk
   - Challenge: "prove you have leaf at index X"
   - Holder returns Merkle proof
   - Pros: Standard cryptographic technique
   - Cons: Requires pre-computed Merkle tree of chunk

3. **Periodic Attestation**
   - Holder signs "I have chunk[3] for Bot-A version 7" periodically
   - Owner verifies signature matches known holder pubkey
   - Pros: Simple, low overhead
   - Cons: Doesn't prove possession (holder could sign without having chunk)

4. **Proof of Retrievability (PoR)**
   - Cryptographic scheme designed for this exact problem
   - Proves file possession without revealing content
   - Pros: Academically proven secure
   - Cons: Complex to implement

### Test Implementation

```rust
// Test 1: Challenge-response verification
async fn test_challenge_response() {
    let chunk = create_test_chunk();
    let holder = spawn_holder(&chunk).await;
    
    // Create challenge
    let nonce = random_nonce();
    let offset = rand::gen_range(0..chunk.len() - 100);
    let challenge = Challenge { nonce, offset, length: 100 };
    
    // Holder responds
    let response = holder.respond_to_challenge(&challenge).await;
    
    // Verify
    let expected = hash(&chunk[offset..offset+100], &nonce);
    assert_eq!(response, expected);
}

// Test 2: Replay resistance
async fn test_replay_resistance() {
    let chunk = create_test_chunk();
    let holder = spawn_holder(&chunk).await;
    
    let challenge1 = Challenge::new();
    let response1 = holder.respond_to_challenge(&challenge1).await;
    
    // Same challenge should give same response (deterministic)
    let response1_again = holder.respond_to_challenge(&challenge1).await;
    assert_eq!(response1, response1_again);
    
    // Different nonce should give different response
    let challenge2 = Challenge::new(); // Different nonce
    let response2 = holder.respond_to_challenge(&challenge2).await;
    assert_ne!(response1, response2);
}

// Test 3: Deleted chunk detection
async fn test_deleted_detection() {
    let chunk = create_test_chunk();
    let holder = spawn_holder(&chunk).await;
    
    // Verify works initially
    assert!(verify_possession(&holder, &chunk).await);
    
    // Holder deletes chunk
    holder.delete_chunk().await;
    
    // Verification should fail
    assert!(!verify_possession(&holder, &chunk).await);
}
```

### Success Criteria

- [ ] Verify chunk possession with <1KB exchange
- [ ] Resistant to replay attack (nonce/freshness)
- [ ] No content leakage (holder learns nothing new)
- [ ] Detects deleted/corrupted chunks

### Deliverable

`q9/RESULTS.md` with:
- Chosen verification scheme
- Security analysis (what attacker learns)
- Protocol overhead (bytes per verification)
- Integration with registry attestations

---

## Q10: Federation Discovery Efficacy (🟢 DEFERRABLE)

**Question**: Does "any-N overlap" discovery find significantly more valid federation candidates than "top-N hash" discovery?

**Why Asked**: Current design uses content-addressed URIs with top-N validators (O(1) lookup). Alternative: Bloom Filter scan for any-N overlap (O(N) background scan). Unclear which finds more valid federation without empirical data.

**Risk Level**: 🟢 DEFERRABLE - Phase 4 uses top-N (proven scalable). Revisit only if federation discovery proves inadequate in practice.

### The Tension

| Approach | Discovery Breadth | Complexity | Scalability |
|----------|------------------|------------|-------------|
| **Top-N hash** | Finds groups with SAME top-3, top-5, etc. | Simple content-addressed lookup | O(1) per bucket |
| **Any-N Bloom** | Finds groups sharing ANY 3 validators | Bloom Filter scan + PSI-CA confirm | O(N) background scan |

**Top-N may miss**: Groups where overlap is in "peripheral" validators (validators 4-10, not top 3)
**Any-N overhead**: Scanning all groups at each bucket (background process, acceptable)

### Simulation Approach

If this spike is needed:

```rust
// Generate realistic social graphs
fn generate_social_graph(
    num_groups: usize,
    validators_per_group: Range<usize>,
    overlap_distribution: OverlapDistribution,
) -> Vec<Group> { ... }

// Compare discovery rates
fn compare_discovery_rates(graph: &[Group]) -> ComparisonResult {
    let top_n_discovered = discover_via_top_n(&graph);
    let any_n_discovered = discover_via_bloom(&graph);
    
    ComparisonResult {
        top_n_count: top_n_discovered.len(),
        any_n_count: any_n_discovered.len(),
        any_n_unique: any_n_discovered.difference(&top_n_discovered).count(),
        // Key question: is any_n_unique significant?
    }
}
```

### Decision Criteria

| Result | Action |
|--------|--------|
| Any-N finds <10% more | Stay with top-N (simpler, proven) |
| Any-N finds 10-50% more | Optional background Bloom scan |
| Any-N finds >50% more | Implement Bloom funnel |

### Current Status

**DEFERRED**: Phase 4 implements top-N (content-addressed URIs at Fibonacci buckets). Revisit if:
- Federation discovery rate is lower than expected
- Operators report "missing" obvious federation candidates
- Network topology suggests peripheral overlaps are common

**See**: `.beads/discovery-protocols.bead` section "DEFERRED DECISION"

---

## Q11: Rendezvous Hashing for Chunk Assignment (🟡 RECOVERABLE)

**Question**: Does deterministic chunk holder assignment (rendezvous hashing) provide equivalent security to registry-based random assignment?

**Why Asked**: Deterministic assignment eliminates O(N×chunks×replicas) registry records, removing a scalability bottleneck and attack target. But holder identities become computable by anyone.

**Risk Level**: 🟡 RECOVERABLE - If security concerns emerge, can fall back to registry-based model.

### The Scalability Problem (Solved by Deterministic Assignment)

| Model | Registry Size | Lookup | Attack Surface |
|-------|--------------|--------|----------------|
| Registry-based | O(5N) records | Query registry | Registry is high-value target |
| Deterministic | O(N) bot list | Local computation | No centralized metadata |

### Security Analysis

**What we lose**: Holder identities were previously "encrypted in registry, only owner decrypts."

**Why this is acceptable**:

| Concern | Analysis |
|---------|----------|
| Attacker can compute holders | True, but chunks are still encrypted |
| Attacker knows "Bot-X holds Bot-A's chunk[3]" | Still needs to compromise Bot-X AND all other chunk holders |
| Need ALL chunks to reconstruct | Must compromise holders of ALL chunks + obtain ACI key |
| Registry was central target | Now NO central target (net improvement) |

### Test Implementation

```rust
// Test 1: Assignment determinism
fn test_assignment_is_deterministic() {
    let bots = vec![bot_a, bot_b, bot_c, bot_d, bot_e];
    let epoch = 5;
    
    // Same inputs → same outputs (per-chunk)
    let holders1 = compute_chunk_holders(&bot_a, 0, &bots, epoch);
    let holders2 = compute_chunk_holders(&bot_a, 0, &bots, epoch);
    assert_eq!(holders1, holders2);
}

// Test 2: Assignment is unpredictable without knowing inputs
fn test_assignment_entropy() {
    let bots = generate_random_bots(100);
    let epoch = 5;
    
    // Each bot gets different holders per chunk
    let all_holders: HashSet<_> = bots.iter()
        .flat_map(|b| {
            (0..8).flat_map(|chunk_idx| compute_chunk_holders(b, chunk_idx, &bots, epoch))
        })
        .collect();
    
    // Good distribution (not clustering on a few bots)
    assert!(all_holders.len() > bots.len() / 2);
}

// Test 3: Churn stability (minimal reassignment)
fn test_churn_stability() {
    let mut bots = generate_random_bots(100);
    let epoch = 5;
    
    let holders_before = compute_chunk_holders(&bot_a, 0, &bots, epoch);
    
    // Remove one bot (not a holder of bot_a's chunk[0])
    bots.retain(|b| *b != bot_unrelated);
    let epoch = 6;
    
    let holders_after = compute_chunk_holders(&bot_a, 0, &bots, epoch);
    
    // Most holders should remain the same
    let unchanged = holders_before.iter()
        .filter(|h| holders_after.contains(h))
        .count();
    assert!(unchanged >= 1);  // At most 1 changed per chunk
}

// Test 4: Owner cannot influence assignment
fn test_owner_cannot_game() {
    let bots = vec![bot_a, bot_friend1, bot_friend2, bot_adversary];
    let epoch = 5;
    
    // bot_a cannot choose their chunk holders
    let holders = compute_chunk_holders(&bot_a, 0, &bots, epoch);
    
    // May include adversary (deterministic, not chosen)
    // This is fine because chunks are encrypted
}
```

### Success Criteria

- [ ] Assignment is deterministic (same inputs → same outputs)
- [ ] Distribution is uniform (no "hot" holders)
- [ ] Churn is graceful (minimal reassignment when bots join/leave)
- [ ] No owner influence (can't game the algorithm)
- [ ] Security equivalent (encrypted chunks still secure)

### Deliverable

`q11/RESULTS.md` with:
- Rendezvous hashing implementation
- Distribution analysis (is assignment uniform?)
- Churn analysis (how many reassignments per join/leave?)
- Security analysis (what attacker gains vs. loses)
- Decision: GO (deterministic) or NO-GO (keep registry)

---

## Q12: Chunk Size Optimization (🟡 RECOVERABLE)

**Question**: What is the optimal chunk size for balancing distribution breadth vs coordination overhead?

**Why Asked**: State is split into fixed-size chunks for distribution. Smaller chunks → more distribution (harder to seize) but more coordination overhead. Larger chunks → simpler but less distribution benefit.

**Risk Level**: 🟡 RECOVERABLE - If chosen size proves suboptimal, can adjust later (re-chunk existing state).

### The Tradeoffs

| Chunk Size | Distribution | Coordination | Recovery |
|------------|--------------|--------------|----------|
| **1KB** | Excellent (500 chunks for 500KB) | Very high overhead | 500 network requests |
| **16KB** | Good (32 chunks for 500KB) | Moderate | 32 network requests |
| **64KB** (default) | Moderate (8 chunks for 500KB) | Low | 8 network requests |
| **256KB** | Limited (2 chunks for 500KB) | Minimal | 2 network requests |

### Key Factors

1. **Security scaling**: More chunks = more holders = harder to seize
2. **Recovery latency**: More chunks = more network requests (parallelizable)
3. **Network overhead**: Each chunk needs 2 replicas distributed
4. **Fairness complexity**: More chunks = more bookkeeping for 2x storage ratio

### Test Implementation

```rust
// Test 1: Recovery latency vs chunk size
async fn test_recovery_latency(chunk_size: usize) -> Duration {
    let state_size = 512 * 1024; // 512KB
    let num_chunks = state_size.div_ceil(chunk_size);
    
    let start = Instant::now();
    // Simulate parallel chunk requests
    let chunks = join_all(
        (0..num_chunks).map(|i| fetch_chunk_from_holder(i))
    ).await;
    start.elapsed()
}

// Test 2: Distribution uniformity at different sizes
fn test_distribution_uniformity(chunk_size: usize, num_bots: usize) {
    let state_size = 512 * 1024;
    let num_chunks = state_size.div_ceil(chunk_size);
    
    // Count how many chunks each bot holds
    let mut holder_counts: HashMap<ContractHash, usize> = HashMap::new();
    for chunk_idx in 0..num_chunks {
        let holders = compute_chunk_holders(&owner, chunk_idx, &bots, epoch);
        for holder in holders {
            *holder_counts.entry(holder).or_default() += 1;
        }
    }
    
    // Check distribution is uniform (no "hot" holders)
    let max_count = holder_counts.values().max().unwrap();
    let avg_count = holder_counts.values().sum::<usize>() / holder_counts.len();
    assert!(max_count <= avg_count * 2, "Distribution too skewed");
}
```

### Success Criteria

- [ ] Recovery latency < 5s for 1MB state (parallel fetching)
- [ ] Distribution spans > 50% of network for typical state sizes
- [ ] No "hot" holders (max 2x average chunk count)
- [ ] Coordination overhead < 10% of chunk data transferred

### Deliverable

`q12/RESULTS.md` with:
- Benchmark results for different chunk sizes
- Recommendation (likely 64KB based on analysis)
- Edge cases (very small states, very large states)
- Implementation guidance

---

## Q13: Fairness Verification (🟡 RECOVERABLE)

**Question**: How to verify a bot ACTUALLY stores the chunks it claims without revealing content?

**Why Asked**: The 2x fairness requirement means bots must store chunks for others. Bad actors could claim storage but not actually store (gaming the system). Need cryptographic verification.

**Risk Level**: 🟡 RECOVERABLE - If verification proves impractical, can rely on soft enforcement (reputation, eventual detection).

**Connection to WHY**: Fairness enables the persistence network to exist. Without verification, free-riders undermine the system.

### The Attack

```
Bad actor Bot-X registers, claims to store chunks
  ↓
Bot-X is selected as holder for Bot-A's chunk[3]
  ↓
Bot-X acknowledges receipt but doesn't actually store
  ↓
Bot-X saves storage space, breaks 2x fairness
  ↓
Bot-A crashes, tries to recover
  ↓
Bot-X can't return chunk[3] → Recovery may fail
```

### Challenge-Response Protocol

```rust
/// Challenge: Prove you have chunk[X] by returning hash(chunk || nonce)
struct ChunkChallenge {
    owner: ContractHash,       // Whose chunk
    chunk_index: u32,          // Which chunk
    nonce: [u8; 32],           // Random nonce (prevents replay)
}

struct ChunkResponse {
    proof: Hash,               // hash(chunk_data || nonce)
    signature: Signature,      // Signed by responder
}

/// Verify without revealing chunk content
fn verify_chunk_possession(
    challenge: &ChunkChallenge,
    response: &ChunkResponse,
    expected_chunk_hash: &Hash,  // Known from original distribution
) -> bool {
    // Responder must have actual chunk to compute correct hash
    // Cannot forge without possessing chunk data
    verify_hash_proof(challenge, response, expected_chunk_hash)
}
```

### Test Implementation

```rust
// Test 1: Honest holder passes challenge
async fn test_honest_holder() {
    let chunk = create_test_chunk();
    let holder = spawn_holder(&chunk).await;
    
    let challenge = ChunkChallenge::new(&chunk);
    let response = holder.respond_to_challenge(&challenge).await;
    
    assert!(verify_chunk_possession(&challenge, &response, &chunk.hash()));
}

// Test 2: Replay attack fails
async fn test_replay_fails() {
    let chunk = create_test_chunk();
    let holder = spawn_holder(&chunk).await;
    
    let challenge1 = ChunkChallenge::new(&chunk);
    let response1 = holder.respond_to_challenge(&challenge1).await;
    
    // Different nonce
    let challenge2 = ChunkChallenge::new(&chunk);
    
    // Old response should not verify for new challenge
    assert!(!verify_chunk_possession(&challenge2, &response1, &chunk.hash()));
}

// Test 3: Free-rider detection
async fn test_freerider_detection() {
    let chunk = create_test_chunk();
    let freerider = spawn_freerider(); // Claims to hold but doesn't store
    
    let challenge = ChunkChallenge::new(&chunk);
    let response = freerider.respond_to_challenge(&challenge).await;
    
    // Free-rider cannot produce valid response
    assert!(!verify_chunk_possession(&challenge, &response, &chunk.hash()));
}
```

### Enforcement Options

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| **Spot checks** | Random challenges before writes | Low overhead | Some free-riding escapes |
| **Reputation scoring** | Track challenge success rate | Gradual exclusion | Complex tracking |
| **Hard exclusion** | Fail 3 challenges = banned | Strong deterrent | May be too aggressive |
| **Soft deprioritization** | Bad actors less likely as holders | Graceful degradation | Doesn't fully prevent |

### Success Criteria

- [ ] Challenge-response in < 100ms
- [ ] No content leakage (holder learns nothing new)
- [ ] Replay resistant (nonce freshness)
- [ ] Detects free-riders with > 95% accuracy
- [ ] Low false positive rate (< 1% legitimate failures flagged)

### Deliverable

`q13/RESULTS.md` with:
- Challenge-response protocol specification
- Security analysis (what attacker can forge)
- Enforcement strategy recommendation
- Integration with registry/reputation

---

## Q14: Chunk Communication Protocol (🟡 RECOVERABLE)

**Question**: How do bots transmit chunks to holders?

**Why Asked**: Chunks must be distributed to 16+ holders per state update. The communication mechanism affects cost, latency, and complexity.

**Risk Level**: 🟡 RECOVERABLE - If Freenet contracts prove too expensive, can migrate to hybrid approach.

**Connection to WHY**: Chunks must REACH holders reliably. Without efficient distribution, persistence network has high overhead.

### Protocol Options

| Option | Mechanism | Pros | Cons |
|--------|-----------|------|------|
| **A. Freenet contracts** | Each bot has "chunk inbox" contract | Leverages Freenet primitives | Expensive (contract writes) |
| **B. Direct P2P** | Bots connect via Freenet network layer | Fast, efficient | NAT traversal, address discovery |
| **C. Hybrid** | Registry stores contact info, P2P for chunks | Best of both | Two mechanisms |

### Test Implementation

```rust
// Option A: Freenet contract-based distribution
async fn test_contract_distribution() {
    let bot_a = spawn_bot("A").await;
    let bot_b = spawn_bot("B").await;
    
    // Bot A distributes chunk to Bot B's storage contract
    let chunk = create_test_chunk();
    let start = Instant::now();
    let attestation = bot_a.distribute_chunk_via_contract(&bot_b, &chunk).await;
    let latency = start.elapsed();
    
    // Measure cost and latency
    assert!(latency < Duration::from_secs(5));
    assert!(attestation.is_valid());
}

// Option C: Hybrid with P2P
async fn test_hybrid_distribution() {
    let bot_a = spawn_bot("A").await;
    let bot_b = spawn_bot("B").await;
    
    // Bot B advertises P2P address in registry
    bot_b.advertise_p2p_address().await;
    
    // Bot A uses P2P for chunk, Freenet for attestation only
    let chunk = create_test_chunk();
    let start = Instant::now();
    let attestation = bot_a.distribute_chunk_hybrid(&bot_b, &chunk).await;
    let latency = start.elapsed();
    
    // Should be faster than pure contract approach
    assert!(latency < Duration::from_secs(2));
}
```

### Cost Analysis (To Validate)

```
Per state update (500KB state = 8 chunks, 2 replicas each = 16 distributions):

Option A (Freenet contracts):
  - 16 contract writes
  - Each write = Freenet PUT operation
  - Cost: ~16 × [Freenet write cost]
  - Latency: ~16 × [Freenet write latency] (parallelizable)

Option C (Hybrid):
  - 16 P2P transfers (fast, direct)
  - 16 attestation writes (small, can batch)
  - Cost: Lower than Option A
  - Latency: Lower than Option A
```

### Success Criteria

- [ ] Distribution completes in < 10s for 500KB state
- [ ] Cost is acceptable for typical update frequency (~10-100/month)
- [ ] Mechanism works across NAT (if using P2P)
- [ ] Attestations are verifiable

### Deliverable

`q14/RESULTS.md` with:
- Cost comparison (Option A vs C)
- Latency measurements
- Freenet API analysis (is message passing available?)
- Phase 0 recommendation (likely Option A for simplicity)
- Phase 1+ migration path (Option C if needed)

---

## Go/No-Go Criteria

### BLOCKING (Q7): Bot Discovery

| Result | Action |
|--------|--------|
| **GO** | Registry contract OR DHT discovery works reliably |
| **NO-GO** | Fall back to manual bootstrap list (operator configures known bots) |

**NO-GO Impact**: Acceptable for Phase 0. Operators manually configure peer bots. Automated discovery in Phase 1.

### RECOVERABLE (Q8, Q9, Q11, Q12, Q13, Q14): Proceed with Conservative Defaults

| Question | NO-GO Action |
|----------|--------------|
| Q8: Fake Bots | Rate limit registrations, accept some Sybil risk initially |
| Q9: Verification | Trust-on-first-verify, periodic re-verification, monitor for failures |
| Q11: Rendezvous Hashing | Fall back to registry-based assignment (encrypted holder records) |
| Q12: Chunk Size | Use 64KB default, adjust based on operational experience |
| Q13: Fairness Verification | Soft enforcement via reputation, manual escalation for bad actors |
| Q14: Communication Protocol | Use Freenet contracts (Option A), migrate to hybrid if too expensive |

---

## Execution Plan

### Day 1: Q7 (BLOCKING)
- Implement registry contract prototype
- Test basic discovery
- Test stale bot removal

### Day 2: Q7 + Q8
- Finalize Q7 (document results)
- Begin Q8: Implement PoW registration
- Test Sybil resistance

### Day 3: Q8 + Q9
- Finalize Q8 (document results)
- Begin Q9: Implement challenge-response
- Test replay resistance

### Day 4: Q9 + Integration
- Finalize Q9 (document results)
- Integration test: Full persistence flow
- Document edge cases

### Day 5: Documentation
- Complete all RESULTS.md files
- Update architecture documents
- Prepare for Phase 0 implementation

---

## Spike Validation Checklist

Before marking any spike COMPLETE, verify:

### Architecture Alignment
- [ ] Solution serves the WHY: "crashed bot recovers from adversarial peers"
- [ ] No trust map leakage to persistence peers
- [ ] Compatible with 1:1 bot-to-group model
- [ ] Works with Freenet's ComposableState (Q1)

### Security Constraints
- [ ] Adversarial peer model assumed throughout
- [ ] No cleartext Signal IDs exposed
- [ ] Chunk holder learns nothing about trust map
- [ ] Replay attacks prevented

### Technical Correctness
- [ ] Tested with SimNetwork (deterministic)
- [ ] Edge cases documented (network partition, stale peers)
- [ ] Performance acceptable (discovery <5s, verification <1s)

---

## Connection to Prior Spikes

| Prior Spike | Relevance to Spike Week 2 |
|-------------|---------------------------|
| Q1: Merge Conflicts | Trust state uses commutative deltas; persistence chunks don't merge |
| Q2: Contract Validation | Trust contract validates; persistence registry is simpler |
| Q3: Cluster Detection | Informs Q10 (federation overlap patterns); persistence is cross-bot |
| Q4: STARK Verification | Trust verification; persistence uses simpler crypto |
| Q5: Merkle Performance | Trust proofs; may inform Q9 chunk verification |
| Q6: Proof Storage | Store outcomes; persistence stores encrypted state |

**Note on Q10**: Cluster detection (Q3) may inform federation discovery efficacy analysis - understanding how clusters form helps predict overlap patterns.

**Note on Q11**: Rendezvous hashing is well-established (used in load balancers, CDNs). Spike validates it works for chunk assignment (churn stability, distribution uniformity).

---

## Next Steps After Spike

If all questions answered (Q7-Q9):

1. Implement persistence layer (Module: Agent-Freenet)
2. Implement chunk distribution (Module: Agent-Crypto)
3. Implement write-blocking (Module: Agent-Freenet)
4. Implement recovery protocol (Module: Agent-Signal)
5. Create documentation (`docs/PERSISTENCE.md`)

**Signal Bot**: Not re-spiked. Presage proven in Spike Week 1.

---

## Key Files Reference

### From Spike Week 1
- [SPIKE-WEEK-BRIEFING.md](SPIKE-WEEK-BRIEFING.md) - Q1-Q6 results
- [q1/RESULTS.md](q1/RESULTS.md) - Freenet merge semantics
- [q2/RESULTS.md](q2/RESULTS.md) - Contract validation

### Architecture Constraints
- [.beads/architecture-decisions.bead](../../.beads/architecture-decisions.bead) - Core decisions
- [.beads/security-constraints.bead](../../.beads/security-constraints.bead) - Security rules
- [.cursor/rules/freenet-integration.mdc](../../.cursor/rules/freenet-integration.mdc) - Freenet patterns
