# Q9: Chunk Verification

**Risk Level**: 🟡 RECOVERABLE  
**Status**: ✅ COMPLETE

---

## WHY This Question Matters

**The Core Problem**: Must PROVE chunks exist before trusting recovery.

Without verification:
- Registry says "Bot X holds my chunk[3]"
- But Bot X may have deleted it (disk space, malice, crash)
- Bot A tries to recover, discovers some chunks are gone
- Recovery fails → Trust map lost → Community destroyed

**Connection to Goal**: "A crashed bot recovers from adversarial peers" requires certainty that chunks actually exist at recovery time.

---

## The Question

**How to verify a holder ACTUALLY has a chunk without revealing content?**

### Key Constraints

1. **Content Privacy**: Holder learns nothing about chunk contents
2. **Freshness**: Can't replay old proofs for deleted chunks
3. **Efficiency**: Verification should be < 1KB exchange
4. **No Trust**: Assume holder is adversarial

---

## Security Model

### What Holder CANNOT Do

| Action | Prevented By |
|--------|-------------|
| Learn chunk content | Challenges don't reveal structure |
| Forge proof without chunk | Challenge requires actual bytes |
| Replay old proofs | Nonce/timestamp freshness |
| Selectively delete parts | Random offset challenges |

### What Owner Knows

| Information | Source |
|-------------|--------|
| Chunk exists | Challenge-response succeeded |
| Chunk is intact | Hash matches commitment |
| Holder is online | Response received |

---

## Test Scenarios

### Scenario 1: Challenge-Response with Nonce

```rust
pub struct VerificationChallenge {
    nonce: [u8; 32],        // Random, prevents replay
    offset: u32,            // Where to read in chunk
    length: u32,            // How many bytes (256 bytes for production)
    timestamp: Timestamp,   // Freshness (reject if too old)
}

pub struct VerificationResponse {
    hash: Hash,             // H(nonce || chunk[offset..offset+length])
    challenge: VerificationChallenge,  // Echo back challenge
}

impl ChunkHolder {
    pub fn respond(&self, challenge: &VerificationChallenge) -> VerificationResponse {
        let chunk = self.get_chunk();
        let slice = &chunk[challenge.offset..challenge.offset + challenge.length];
        let hash = hash_with_nonce(&challenge.nonce, slice);
        VerificationResponse { hash, challenge: *challenge }
    }
}

impl ChunkOwner {
    pub fn verify(&self, response: &VerificationResponse) -> bool {
        // Check freshness
        if response.challenge.timestamp < now() - Duration::from_hours(1) {
            return false;
        }
        
        // Compute expected hash (owner has the chunk too)
        let expected = self.compute_expected_hash(&response.challenge);
        expected == response.hash
    }
}
```

**Pros**:
- Simple, well-understood
- Nonce prevents replay
- Small exchange (~100 bytes)

**Cons**:
- Holder learns offset exists (minor leak)
- Requires owner to have chunk (for verification)

### Scenario 2: Merkle Proof of Chunk

```rust
pub struct MerkleVerification {
    chunk_merkle_root: Hash,  // Published when chunk distributed
    leaf_index: u32,          // Which leaf to prove
}

pub struct MerkleProof {
    leaf: Hash,
    siblings: Vec<Hash>,
    index: u32,
}

impl ChunkHolder {
    /// Build Merkle tree over 1KB sub-chunks
    pub fn merkle_proof(&self, leaf_index: u32) -> MerkleProof {
        let tree = self.build_merkle_tree();
        tree.proof(leaf_index)
    }
}

impl ChunkOwner {
    pub fn verify_merkle(&self, proof: &MerkleProof) -> bool {
        proof.verify_against_root(&self.chunk_merkle_root)
    }
}
```

**Pros**:
- Standard cryptographic technique
- Proves specific portions exist

**Cons**:
- Requires pre-computed Merkle root at distribution time
- Larger proof size (~512 bytes for 64KB chunk)

### Scenario 3: Periodic Attestation with Commitment

```rust
pub struct Attestation {
    holder_pubkey: PublicKey,
    chunk_hash: Hash,        // H(chunk)
    chunk_index: u32,        // Which chunk
    version: u64,            // State version
    timestamp: Timestamp,
    signature: Signature,    // Holder signs attestation
}

impl ChunkHolder {
    /// Publish attestation periodically
    pub fn attest(&self, chunk_index: u32) -> Attestation {
        let chunk = self.get_chunk(chunk_index);
        Attestation {
            holder_pubkey: self.pubkey(),
            chunk_hash: hash(&chunk),
            chunk_index,
            version: self.state_version(),
            timestamp: now(),
            signature: self.sign(&(chunk_hash, chunk_index, version, timestamp)),
        }
    }
}
```

**Pros**:
- Asynchronous (holder publishes, owner reads)
- Low overhead for owner

**Cons**:
- Holder could sign without actually having chunk
- Requires owner to know chunk hash

### Scenario 4: Interactive Proof of Retrievability (PoR)

```rust
/// Based on Shacham-Waters PoR scheme (simplified)
pub struct PorChallenge {
    indices: Vec<u32>,      // Which blocks to combine
    coefficients: Vec<u64>, // Linear combination coefficients
}

pub struct PorResponse {
    sigma: Hash,            // Combined authenticator
    mu: Vec<u8>,            // Combined block data
}

impl ChunkHolder {
    pub fn respond_por(&self, challenge: &PorChallenge) -> PorResponse {
        // Compute linear combination of challenged blocks
        let mut mu = vec![0u8; BLOCK_SIZE];
        let mut sigma = Hash::zero();
        
        for (i, coef) in challenge.indices.iter().zip(&challenge.coefficients) {
            let block = self.get_block(*i);
            let auth = self.get_authenticator(*i);
            
            // mu = Σ coef_i * block_i
            add_scaled(&mut mu, &block, *coef);
            // sigma = Π auth_i^coef_i
            sigma = sigma.add(&auth.mul(*coef));
        }
        
        PorResponse { sigma, mu }
    }
}
```

**Pros**:
- Academically proven secure
- Single challenge can verify entire chunk

**Cons**:
- Complex implementation
- Requires authenticators stored with chunk

---

## Test Cases

### Test 1: Basic Challenge-Response

```rust
#[test]
async fn test_challenge_response_success() {
    let chunk = random_chunk(64 * 1024); // 64KB
    let holder = spawn_holder(&chunk).await;
    
    // Create challenge
    let challenge = VerificationChallenge {
        nonce: random_nonce(),
        offset: 12345,
        length: 256,
        timestamp: now(),
    };
    
    // Holder responds
    let response = holder.respond(&challenge).await;
    
    // Owner verifies (owner also has chunk)
    let owner = ChunkOwner::new(&chunk);
    assert!(owner.verify(&response));
}
```

### Test 2: Replay Resistance

```rust
#[test]
async fn test_replay_resistance() {
    let chunk = random_chunk(64 * 1024);
    let holder = spawn_holder(&chunk).await;
    
    // Get valid response
    let challenge1 = VerificationChallenge::new();
    let response1 = holder.respond(&challenge1).await;
    
    // Response is valid
    let owner = ChunkOwner::new(&chunk);
    assert!(owner.verify(&response1));
    
    // Different nonce requires different response
    let challenge2 = VerificationChallenge::new(); // Different nonce
    // Replaying old response should fail
    let replayed = VerificationResponse {
        hash: response1.hash, // Old hash
        challenge: challenge2, // New challenge
    };
    assert!(!owner.verify(&replayed));
}
```

### Test 3: Deleted Chunk Detection

```rust
#[test]
async fn test_deleted_detection() {
    let chunk = random_chunk(64 * 1024);
    let holder = spawn_holder(&chunk).await;
    let owner = ChunkOwner::new(&chunk);
    
    // Initially valid
    let challenge = VerificationChallenge::new();
    let response = holder.respond(&challenge).await;
    assert!(owner.verify(&response));
    
    // Holder deletes chunk
    holder.delete_chunk().await;
    
    // Subsequent challenge fails (holder can't respond correctly)
    let challenge2 = VerificationChallenge::new();
    let response2 = holder.respond(&challenge2).await;
    assert!(!owner.verify(&response2)); // Wrong hash (random guess)
}
```

### Test 4: Content Privacy

```rust
#[test]
fn test_no_content_leak() {
    let chunk = random_chunk(64 * 1024);
    
    // Challenge reveals only:
    // - An offset exists (offset: 12345)
    // - A length is requested (length: 256, 0.4% of 64KB chunk)
    // - A nonce (random, reveals nothing)
    
    let challenge = VerificationChallenge {
        nonce: random_nonce(),
        offset: 12345,
        length: 256,
        timestamp: now(),
    };
    
    // Holder learns:
    // - That bytes 12345-12409 were requested
    // - NOT what those bytes mean
    // - NOT the rest of the chunk
    // - This is acceptable information leak
    
    // Response reveals only:
    // - H(nonce || bytes) - hash, not content
    // - Holder cannot reverse hash to get bytes
}
```

### Test 5: Efficiency

```rust
#[test]
fn test_verification_size() {
    let challenge = VerificationChallenge::new();
    let challenge_bytes = serialize(&challenge);
    
    // Challenge should be small
    assert!(challenge_bytes.len() < 100); // nonce(32) + offset(4) + length(4) + timestamp(8) = 48
    
    let response = VerificationResponse::new();
    let response_bytes = serialize(&response);
    
    // Response should be small
    assert!(response_bytes.len() < 150); // hash(32) + challenge(48) = 80
    
    // Total < 1KB
    assert!(challenge_bytes.len() + response_bytes.len() < 1024);
}
```

---

## Success Criteria

| Criterion | Requirement |
|-----------|-------------|
| Exchange size | < 1KB total (challenge + response) |
| Verification time | < 100ms |
| Replay resistance | Different nonce = different valid response |
| Content privacy | Holder learns only offset, not content meaning |
| False negative | 0% (real chunk always verifies) |
| False positive | < 0.001% (fake response passes) |

---

## Recommended Approach

Based on analysis:

**Primary**: Challenge-Response with Nonce (Scenario 1)
- Simple, proven, efficient
- Acceptable information leak (offset only)
- Easy to implement

**Enhancement**: Periodic Merkle Commitment (Scenario 2)
- Publish Merkle root when chunk distributed
- Allows multiple verification methods
- Defense in depth

---

## Fallback Strategy (NO-GO)

If cryptographic verification too complex:

**Trust-on-First-Verify**
```rust
// First verification establishes baseline
// Subsequent verifications compared to baseline
// Periodic re-verification (every 24h)
// Flag holders who fail > 2 times
```

**Impact**: Some risk of undetected deletion. Acceptable for Phase 0 with monitoring.

---

## Files

- `main.rs` - Test harness
- `RESULTS.md` - Findings (after spike)

## Related

- [Q7: Bot Discovery](../q7/README.md) - Discovers holders to verify
- [Q8: Fake Bot Defense](../q8/README.md) - Verification complements defense
- [SPIKE-WEEK-2-BRIEFING.md](../SPIKE-WEEK-2-BRIEFING.md) - Full context
