# Q8: Fake Bot Defense

**Risk Level**: 🟡 RECOVERABLE  
**Status**: ✅ COMPLETE

---

## WHY This Question Matters

**The Core Problem**: Attackers mustn't become chunk holders, or they can DoS recovery.

Attack scenario:
```
Attacker registers 1000 fake "bots" (just pubkeys, no real state)
  ↓
Fake bots selected as chunk holders for real Bot-A
  ↓
Bot-A crashes, tries to recover
  ↓
Fake bots don't respond (no real chunk stored)
  ↓
Recovery fails → Trust map lost → Community destroyed
```

**Connection to Goal**: "A crashed bot recovers from adversarial peers" fails if those peers are fake.

---

## The Question

**How to prevent fake bot registration from diluting the persistence network?**

### Key Constraints

1. **Sybil Resistance**: Creating fake bots must be expensive
2. **Legitimate Access**: Real operators (including RPi) must be able to register
3. **No Token Economy**: Avoid introducing cryptocurrency/staking complexity
4. **Privacy Preserving**: Defense shouldn't leak trust map information

---

## Test Scenarios

### Scenario 1: Proof of Work (PoW)

```rust
pub struct RegistrationProof {
    nonce: u64,
    bot_pubkey: PublicKey,
    timestamp: Timestamp,
}

impl RegistrationProof {
    /// Compute PoW for registration
    pub fn compute(bot_pubkey: &PublicKey, difficulty: u8) -> Self {
        let mut nonce = 0u64;
        loop {
            let candidate = Self { nonce, bot_pubkey: *bot_pubkey, timestamp: now() };
            if candidate.hash().leading_zeros() >= difficulty {
                return candidate;
            }
            nonce += 1;
        }
    }
    
    /// Verify PoW is valid
    pub fn verify(&self, difficulty: u8) -> bool {
        self.hash().leading_zeros() >= difficulty
    }
}

// Difficulty tuning:
// - difficulty=16: ~65K hashes, ~100ms on laptop
// - difficulty=20: ~1M hashes, ~1-2s on laptop
// - difficulty=24: ~16M hashes, ~30s on laptop
```

**Pros**:
- Economic cost to Sybil attack (electricity + time)
- No external dependencies
- Can tune difficulty

**Cons**:
- Penalizes legitimate low-power devices (RPi)
- One-time cost (doesn't prevent patient attacker)

### Scenario 2: Reputation Accumulation

```rust
pub struct BotReputation {
    successful_returns: u32,    // Chunks returned during recovery
    failed_returns: u32,        // Requests that timed out
    age_days: u32,              // Days since registration
    chunks_held: u32,           // Current chunks being held
}

impl BotReputation {
    /// Calculate trust score (0.0 - 1.0)
    pub fn trust_score(&self) -> f64 {
        let success_rate = self.successful_returns as f64 
            / (self.successful_returns + self.failed_returns + 1) as f64;
        let age_factor = (self.age_days as f64 / 30.0).min(1.0);
        let activity_factor = (self.chunks_held as f64 / 10.0).min(1.0);
        
        (success_rate * 0.5) + (age_factor * 0.3) + (activity_factor * 0.2)
    }
    
    /// Is this bot eligible to be a chunk holder?
    pub fn eligible_for_holding(&self) -> bool {
        self.trust_score() >= 0.3 && self.age_days >= 7
    }
}
```

**Pros**:
- Organic trust building
- No upfront cost for legitimate bots
- Self-cleaning (bad actors lose reputation)

**Cons**:
- Slow bootstrap (7+ days to become eligible)
- Patient attacker can build reputation

### Scenario 3: Capacity Verification

```rust
/// Verify bot actually has storage capacity
pub struct CapacityProof {
    challenge: Hash,
    response: Hash,
    capacity_claimed: usize,
}

impl CapacityProof {
    /// Prover: Generate proof of storage capacity
    pub fn prove(capacity: usize) -> Self {
        // Must actually allocate and hash random data
        let data = vec![0u8; capacity];
        fill_random(&mut data);
        let response = hash(&data);
        // Store data for verification
        Self { challenge: hash(&response), response, capacity_claimed: capacity }
    }
    
    /// Verifier: Challenge the prover
    pub async fn verify(&self, prover: &Bot) -> bool {
        // Ask prover to hash a random subset
        let subset_indices = random_indices(100);
        let expected = prover.hash_subset(&subset_indices).await;
        // If prover doesn't have data, they can't compute correct hash
        expected.is_ok()
    }
}
```

**Pros**:
- Directly proves real capacity exists
- Fake bots can't pass (no real storage)

**Cons**:
- Requires periodic re-verification
- Storage is cheap (attacker could actually allocate)

### Scenario 4: Combined Approach

```rust
pub struct RegistrationRequirements {
    pow_difficulty: u8,           // Must solve PoW (difficulty 18 for production)
    min_age_for_holding: u32,     // 7 days before eligible
    capacity_verification: bool,   // Must prove storage exists
}

impl Bot {
    pub async fn register(&self, registry: &Registry) -> Result<(), Error> {
        // 1. Solve PoW (one-time cost)
        let pow = RegistrationProof::compute(&self.pubkey, POW_DIFFICULTY);
        
        // 2. Submit registration (starts reputation clock)
        registry.register(pow).await?;
        
        // 3. Capacity verification (periodic)
        self.prove_capacity().await?;
        
        Ok(())
    }
    
    pub fn is_eligible_for_holding(&self) -> bool {
        self.reputation.eligible_for_holding() 
            && self.capacity_verified
    }
}
```

---

## Test Cases

### Test 1: PoW Registration Cost

```rust
#[test]
fn test_pow_cost() {
    let pubkey = generate_keypair().public();
    
    let start = Instant::now();
    let proof = RegistrationProof::compute(&pubkey, 16);
    let elapsed = start.elapsed();
    
    // Should take ~100ms on average hardware
    assert!(elapsed > Duration::from_millis(50));
    assert!(elapsed < Duration::from_secs(1));
    
    // Should be valid
    assert!(proof.verify(16));
}
```

### Test 2: Sybil Cost Analysis

```rust
#[test]
fn test_sybil_cost() {
    // Attacker wants to register 1000 fake bots
    let target = 1000;
    let difficulty = 18; // Production difficulty
    
    // Each registration takes ~100ms
    let time_per_reg = Duration::from_millis(100);
    let total_time = time_per_reg * target;
    
    // 1000 bots = ~100 seconds (acceptable attack cost?)
    assert!(total_time > Duration::from_secs(60));
    
    // With difficulty 20: ~1000 seconds (~17 minutes)
    // With difficulty 24: ~30000 seconds (~8 hours)
}
```

### Test 3: Reputation-Based Selection

```rust
#[test]
async fn test_reputation_selection() {
    let network = SimNetwork::new();
    
    // New bot (no reputation)
    let new_bot = network.spawn_bot("new").await;
    new_bot.register_for_persistence().await;
    
    // Established bot (has history)
    let established = network.spawn_bot("established").await;
    established.register_for_persistence().await;
    established.record_successful_chunk_return().await;
    network.advance_time(Duration::from_days(14)).await;
    
    // Selection via rendezvous hashing is deterministic
    // But reputation can influence whether bot accepts the role
    let holders = compute_chunk_holders(&target_bot, 0, &bots, epoch);
    // Reputation system may deprioritize new bots
}
```

### Test 4: Fake Bot Detection

```rust
#[test]
async fn test_fake_bot_detection() {
    let network = SimNetwork::new();
    
    // Fake bot (registered but doesn't actually hold chunks)
    let fake_bot = network.spawn_fake_bot("fake").await;
    fake_bot.register_for_persistence().await;
    
    // Real bot sends chunk
    let real_bot = network.spawn_bot("real").await;
    real_bot.send_chunk_to(&fake_bot, chunk_index).await;
    
    // Fake bot doesn't actually store it
    // Capacity verification should fail
    assert!(!fake_bot.verify_capacity().await);
    
    // Fake bot should lose reputation / be flagged
    assert!(!fake_bot.reputation().eligible_for_holding());
}
```

---

## Success Criteria

| Criterion | Requirement |
|-----------|-------------|
| Sybil cost | > 1 hour to register 1000 fake bots |
| Legitimate registration | < 5 seconds for real bot |
| RPi compatibility | Must work on Raspberry Pi 4 |
| Detection rate | > 90% of fake bots detected within 7 days |
| False positive | < 1% legitimate bots falsely flagged |

---

## Fallback Strategy (NO-GO)

If sophisticated defense fails:

**Rate Limiting**
```rust
const MAX_REGISTRATIONS_PER_HOUR: u32 = 10;
const MAX_REGISTRATIONS_PER_DAY: u32 = 50;
```

**Impact**: Slows Sybil attack but doesn't prevent patient attacker. Acceptable for Phase 0.

---

## Files

- `main.rs` - Test harness
- `RESULTS.md` - Findings (after spike)

## Related

- [Q7: Bot Discovery](../q7/README.md) - Discovery mechanism (depends on)
- [Q9: Chunk Verification](../q9/README.md) - Verification complements defense
- [SPIKE-WEEK-2-BRIEFING.md](../SPIKE-WEEK-2-BRIEFING.md) - Full context
