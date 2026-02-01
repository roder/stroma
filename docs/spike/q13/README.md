# Q13: Fairness Verification

**Risk Level**: 🟡 RECOVERABLE
**Status**: COMPLETE

---

## WHY This Question Matters

**The Core Problem**: The 2x fairness requirement means bots must store chunks for others. Bad actors could claim storage but not actually store (gaming the system), breaking the persistence network.

**The Attack**:
```
Bad actor Bot-X registers, claims to store chunks
  ↓
Bot-X selected as holder for Bot-A's chunk[3]
  ↓
Bot-X acknowledges receipt but doesn't actually store
  ↓
Bot-X saves storage space, breaks 2x fairness
  ↓
Bot-A crashes, tries to recover
  ↓
Bot-X can't return chunk[3] → Recovery may fail
```

**Connection to Goal**: "Fairness enables the persistence network to exist." Without verification, free-riders undermine the entire system.

---

## The Question

**How to verify a bot ACTUALLY stores the chunks it claims without revealing content?**

### Key Constraints

1. **Cryptographic proof**: Holder must prove possession without revealing chunk
2. **Replay resistance**: Old proofs can't be reused
3. **Content privacy**: Verification doesn't leak chunk data
4. **Performance**: Verification must be fast (< 100ms)
5. **Accuracy**: Detect free-riders with > 95% accuracy, < 1% false positives

---

## Challenge-Response Protocol

### Protocol Overview

**Challenge**: Owner asks holder to prove possession by hashing a random sample.

```rust
struct ChunkChallenge {
    owner: ContractHash,       // Whose chunk
    chunk_index: u32,          // Which chunk
    nonce: [u8; 32],           // Random (prevents replay)
    timestamp: u64,            // Freshness check
    offset: usize,             // Sample location
    length: usize,             // Sample size (256 bytes)
}
```

**Response**: Holder computes proof.

```rust
struct ChunkResponse {
    proof: Hash,               // SHA-256(nonce || chunk_sample)
    responder: ContractHash,   // Who responded
}
```

**Verification**: Owner checks proof.

```rust
fn verify(challenge: &ChunkChallenge, response: &ChunkResponse) -> bool {
    // Recompute expected proof
    let sample = chunk_data[challenge.offset..challenge.offset + 256];
    let expected = SHA-256(challenge.nonce || sample);

    // Check freshness and proof
    challenge.is_fresh() && response.proof == expected
}
```

---

## Test Scenarios

### Test 1: Honest Holder Passes Challenge

**Setup**: Legitimate bot stores chunk, receives challenge.

```rust
let holder = HonestHolder::new();
holder.store_chunk(owner, chunk_index, chunk_data);

let challenge = ChunkChallenge::new(owner, chunk_index, chunk_size);
let response = holder.respond_to_challenge(&challenge)?;

assert!(response.verify(&challenge, &chunk_data));
```

**Expected**: Holder produces valid proof, verification passes.

### Test 2: Replay Attack Fails

**Setup**: Attacker tries to reuse old response for new challenge.

```rust
let challenge1 = ChunkChallenge::new(...);
let response1 = holder.respond_to_challenge(&challenge1)?;

// New challenge with different nonce
let challenge2 = ChunkChallenge::new(...);

// Old response should NOT verify for new challenge
assert!(!response1.verify(&challenge2, &chunk_data));
```

**Expected**: Nonce mismatch causes verification failure.

### Test 3: Free-Rider Detection

**Setup**: Malicious bot claims to store but doesn't.

```rust
let freerider = FreeRider::new();
// Does NOT store the chunk

let challenge = ChunkChallenge::new(...);
let response = freerider.respond_to_challenge(&challenge);

assert!(response.is_none()); // Can't respond without chunk
```

**Expected**: Free-rider cannot produce valid response.

### Test 4: Challenge Latency

**Setup**: Benchmark verification time.

```rust
for _ in 0..100 {
    let start = Instant::now();
    let response = holder.respond_to_challenge(&challenge)?;
    let verified = response.verify(&challenge, &chunk_data);
    let latency = start.elapsed();

    assert!(latency < Duration::from_millis(100));
}
```

**Expected**: Average latency < 100ms, ideally < 10ms.

### Test 5: Content Privacy

**Setup**: Analyze what holder learns from challenge.

**Holder already knows**:
- They are storing a chunk (they received it)
- Which chunk index
- The owner identity

**Challenge reveals**:
- Nonce (random, no information)
- Offset (256 bytes of 64KB = 0.4% of structure)
- Length (256 bytes)

**Holder does NOT learn**:
- Full chunk content (only 256 bytes sampled)
- Other chunks' content
- Decryption key (chunks are encrypted)
- Trust map data (encrypted within chunks)

**Expected**: < 1% chunk structure leakage.

### Test 6: False Positive Rate

**Setup**: Challenge 100 legitimate holders.

```rust
let mut successes = 0;
let mut failures = 0;

for holder in legitimate_holders {
    let challenge = ChunkChallenge::new(...);
    let response = holder.respond_to_challenge(&challenge)?;

    if response.verify(&challenge, &chunk_data) {
        successes += 1;
    } else {
        failures += 1; // False positive
    }
}

let false_positive_rate = failures as f64 / 100.0;
assert!(false_positive_rate < 0.01); // < 1%
```

**Expected**: < 1% legitimate holders incorrectly flagged.

---

## Enforcement Strategies

### Strategy 1: Spot Checks

**Description**: Challenge random holders before each write.

**Implementation**:
```rust
// Before distributing new chunks, verify existing holders
let sample_size = (holders.len() as f64 * 0.01).max(1.0) as usize; // 1% sample
let sample = holders.choose_multiple(&mut rng, sample_size);

for holder in sample {
    let challenge = ChunkChallenge::new(...);
    if !verify_holder(holder, challenge).await {
        mark_suspicious(holder);
    }
}
```

**Pros**:
- Low overhead (~1% of holders checked per write)
- Probabilistic deterrent

**Cons**:
- Some free-riding escapes detection
- Requires consistent checking

**Recommendation**: **Phase 0**

### Strategy 2: Reputation Scoring

**Description**: Track challenge success rate per bot.

**Implementation**:
```rust
struct BotReputation {
    challenges_sent: u32,
    challenges_passed: u32,
    success_rate: f64,
}

// Deprioritize bots with low success rate
fn select_holders(candidates: &[Bot], reputation: &HashMap<BotId, Reputation>) -> Vec<Bot> {
    candidates
        .iter()
        .filter(|bot| reputation[bot.id].success_rate > 0.95)
        .take(REPLICA_COUNT)
        .collect()
}
```

**Pros**:
- Gradual exclusion based on pattern
- Less aggressive than hard bans
- Tolerates occasional failures

**Cons**:
- Requires persistent reputation storage
- Complex tracking

**Recommendation**: **Phase 1+**

### Strategy 3: Hard Exclusion

**Description**: Ban bots after N failed challenges.

**Implementation**:
```rust
const MAX_FAILURES: u32 = 3;

if bot.challenge_failures >= MAX_FAILURES {
    blacklist.insert(bot.id);
    // Bot permanently excluded from holder selection
}
```

**Pros**:
- Strong deterrent
- Simple rule

**Cons**:
- May be too aggressive for network issues
- No appeal mechanism
- Could exclude legitimate bots with connectivity problems

**Recommendation**: **Phase 1+ with appeal process**

### Strategy 4: Soft Deprioritization

**Description**: Failed challenges reduce likelihood of being selected.

**Implementation**:
```rust
fn holder_score(bot: &Bot, reputation: &Reputation) -> f64 {
    let base_score = rendezvous_hash(owner, chunk_idx, bot.id);
    let reputation_multiplier = reputation.success_rate;

    base_score * reputation_multiplier
}
```

**Pros**:
- Graceful degradation
- No hard bans
- Self-correcting (bot can improve reputation)

**Cons**:
- Doesn't fully prevent free-riding
- Still requires reputation tracking

**Recommendation**: **Phase 1+ alongside reputation scoring**

---

## Success Criteria

| Criterion | Requirement | Result |
|-----------|-------------|--------|
| Challenge-response latency | < 100ms | ✅ < 1ms |
| Content leakage | Minimal | ✅ < 1% (256 of 65536 bytes) |
| Replay resistance | Nonce freshness | ✅ Enforced |
| Free-rider detection | > 95% accuracy | ✅ 100% accuracy |
| False positive rate | < 1% | ✅ 0% |

---

## Fallback Strategy (NO-GO)

If verification proves impractical:

**Soft Enforcement via Reputation**
- Track holder success rate over time
- Gradually exclude bad actors
- Manual escalation for persistent offenders

**Impact**: Acceptable for Phase 0. Some free-riding tolerated initially.

---

## Files

- `main.rs` - Test harness with challenge-response protocol
- `RESULTS.md` - Findings and enforcement recommendations

## Related

- [SPIKE-WEEK-2-BRIEFING.md](../SPIKE-WEEK-2-BRIEFING.md) - Full context
- [Q9: Chunk Verification](../q9/RESULTS.md) - Similar challenge-response pattern
- [Q12: Chunk Size](../q12/RESULTS.md) - Sample size considerations
