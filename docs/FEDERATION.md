# Federation: The North Star

**Phase 4+ Vision & Design Philosophy**

## Ultimate Objective

**Connect as many people as possible anonymously via trust through federated groups.**

This is the **north star** for ALL architectural decisions. Even though federation is NOT implemented in the MVP (Phase 0-3), it guides EVERY design choice.

## Why Federation Matters

### The Scaling Challenge

**Single Group Limitations:**
- Signal group limit: ~1000 members per group
- Trust degrades in very large groups (becomes impersonal)
- Single point of failure (one bot, one group)

**Federation Solution:**
- Unlimited scaling via multiple interconnected groups
- Each group remains human-scale (50-200 members optimal)
- Trust spans across groups through shared members
- Resilience through redundancy

### The Vision

```
Group-A (100 members) ←--→ Group-B (150 members)
      ↑                           ↓
      ↑                           ↓
      ↑----→ Group-C (80 members) ←----
      
Total Network: 330 members across 3 federated groups
Scaling Factor: 3x via federation (can continue to 10²-10³)
```

**Key Properties:**
- Each group maintains own trust standards
- Members can vouch across groups (if federated)
- No central authority coordinates federation
- Bots discover each other emergently

## Design Philosophy

### Mutual Arising

**Concept**: Groups discover each other through **emergent patterns**, not admin coordination.

**Traditional Approach** (WRONG):
- Admins exchange URLs/keys
- Manual federation setup
- Pre-coordinated trust relationships

**Stroma Approach** (CORRECT):
- Bots discover each other via "social frequency"
- Shared validators create natural resonance
- No pre-shared keys or admin handshakes
- Federation arises mutually when groups overlap

**Metaphor**: Like people naturally forming friendships - you don't need a central matchmaker if you share common connections.

### Emergent Discovery

**How Bots Find Each Other (No Admin Coordination):**

1. **Social Anchor Hashing**
   - Hash top-N validators (percentile-based, e.g., top 20%)
   - Groups with shared validators → similar social anchors
   - Multiple anchors at different percentiles (increases discovery chances)

2. **Shadow Beacon Broadcast**
   - Publish encrypted Bloom filter at discovery URIs
   - URIs derived from social anchors
   - No IP addresses, no identity correlation

3. **Discovery Match**
   - Bots scan URIs looking for compatible groups
   - PSI-CA reveals overlap count (not identities)
   - Both groups evaluate independently

**Result**: Bots find each other because they share a "social frequency" (overlapping validators).

### Blind Rendezvous

**Problem**: Calculating overlap reveals member identities.

**Solution**: Private Set Intersection Cardinality (PSI-CA)

**Protocol**:
```
Bot-A: Has member set A (hashed)
Bot-B: Has member set B (hashed)

Goal: Calculate |A ∩ B| without revealing which members overlap

Method:
1. Both bots generate Bloom filters
2. Commutative encryption (double-blinding)
3. Exchange encrypted filters
4. Calculate intersection COUNT only
5. No member identities revealed
```

**Privacy Guarantee**: Bots learn ONLY the count of overlap, not which specific members overlap.

### BidirectionalMin (Asymmetric Thresholds)

**Problem**: Small group federating with large group risks absorption.

**Solution**: Each group sets their own threshold independently.

**Example**:
```
Group-A (Small - 20 members):
- min_intersection_density: 0.30 (30%)
- Required overlap: 6 members

Group-B (Large - 100 members):
- min_intersection_density: 0.10 (10%)
- Required overlap: 10 members

Actual Overlap: 8 members

Evaluation:
- Group-A: 8 > 6 ✅ (satisfied)
- Group-B: 8 < 10 ❌ (not satisfied)

Result: Federation REJECTED (both must approve)
```

**Benefit**: Small groups protect themselves from large group dominance.

### Human Control (No Automatic Federation)

**Problem**: Bot could federate without group consent.

**Solution**: Signal Poll for every federation proposal.

**Process**:
1. Bot detects overlap meets threshold
2. Bot proposes federation to group
3. Members vote via Signal Poll
4. Requires `min_quorum` participation AND `config_change_threshold` approval
5. Both groups must approve
6. Bot signs federation contract on Freenet

**Human Override**: Members can reject even if technically viable.

### Fluid Identity Across Meshes

**Concept**: Your trust identity precedes you across federated groups.

**Example**:
- You're a member of Group-A (4 vouches)
- Group-A federates with Group-B
- You want to join Group-B
- Member from Group-A invites you to Group-B
- Member from Group-B vouches (second vouch)
- Faster admission (already trusted in sister group)

**Key**: Trust is portable across federated groups.

## Federation Discovery Protocol (Phase 4)

### Step 1: Social Anchor Calculation

```rust
fn calculate_social_anchor(
    members: &[Member],
    validator_percentile: u32,
) -> SocialAnchor {
    // Sort members by effective vouch count
    let mut sorted = members.clone();
    sorted.sort_by_key(|m| m.effective_vouch_count());
    
    // Take top percentile (e.g., top 20%)
    let threshold_idx = (members.len() * validator_percentile / 100).max(3);
    let top_validators = &sorted[threshold_idx..];
    
    // Hash top validators to create social anchor
    let mut hasher = Sha256::new();
    for validator in top_validators {
        hasher.update(validator.hash.as_bytes());
    }
    
    SocialAnchor::from(hasher.finalize())
}
```

**Why Fibonacci Buckets (not percentiles):**
- **Fixed counts** → groups of different sizes produce MATCHING hashes at same bucket
- Groups with shared top-3 validators hash to the SAME URI-3
- Larger groups publish at more buckets → more discovery chances
- Smaller groups still discoverable at lower buckets

### Step 2: Discovery URI Generation

```rust
// Fibonacci buckets (up to Signal's 1000-member limit)
const FIBONACCI_BUCKETS: &[usize] = &[
    3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377, 610, 987
];

fn generate_discovery_uris(validators: &[Hash]) -> Vec<(usize, Uri)> {
    let mut sorted = validators.to_vec();
    sorted.sort();  // Deterministic ordering by hash
    
    FIBONACCI_BUCKETS.iter()
        .filter(|&&bucket| sorted.len() >= bucket)
        .map(|&bucket| {
            let top_validators = &sorted[..bucket];
            let anchor = compute_social_anchor(top_validators);
            (bucket, format!("freenet://stroma/discovery/{}", anchor))
        })
        .collect()
}
```

**Multiple URIs**: Groups publish at ALL Fibonacci buckets they can fill (increases discovery chances)

### Step 3: Bloom Filter Broadcast

```rust
async fn broadcast_presence(
    anchor: &SocialAnchor,
    members: &[Hash],
) -> Result<(), Error> {
    // Create Bloom filter from member hashes
    let bloom = BloomFilter::new(members);
    
    // Encrypt for anonymous publication
    let encrypted = encrypt_bloom_filter(&bloom)?;
    
    // Publish at all discovery URIs
    for uri in generate_discovery_uris(anchor) {
        freenet.publish(&uri, &encrypted).await?;
    }
    
    Ok(())
}
```

### Step 4: PSI-CA Handshake

```rust
async fn calculate_overlap(
    our_bloom: &BloomFilter,
    their_encrypted_bloom: &[u8],
) -> Result<OverlapInfo, Error> {
    // PSI-CA reveals ONLY count, not identities
    let overlap_count = psi_ca::calculate_intersection_count(
        our_bloom,
        their_encrypted_bloom,
    ).await?;
    
    Ok(OverlapInfo {
        overlap_count,
        our_size: our_bloom.len(),
        their_size_estimate: estimate_bloom_size(their_encrypted_bloom),
    })
}
```

### Step 5: Threshold Evaluation

```rust
fn evaluate_federation_threshold(
    overlap: &OverlapInfo,
    our_threshold: f32,
) -> bool {
    // Calculate intersection density from OUR perspective
    let union_size = overlap.our_size + overlap.their_size_estimate - overlap.overlap_count;
    let density = overlap.overlap_count as f32 / union_size as f32;
    
    // Check if it meets OUR threshold (BidirectionalMin)
    density >= our_threshold
}
```

### Step 6: Human Vote

```rust
async fn propose_federation(
    other_group: GroupInfo,
    overlap: OverlapInfo,
) -> Result<bool, Error> {
    // Create Signal Poll
    let poll = signal.create_poll(Poll {
        question: format!("Federate with {}?", other_group.name),
        details: format!(
            "Overlap: {} members ({}% of our group)\n\
             Their size: ~{} members",
            overlap.overlap_count,
            overlap.overlap_count * 100 / overlap.our_size,
            overlap.their_size_estimate,
        ),
        options: vec!["✅ Approve", "❌ Reject", "⏸️ Abstain"],
    }).await?;
    
    // Wait for result
    let result = poll.wait_for_result().await?;
    let total_members = group.member_count();
    let participation = result.total_votes as f32 / total_members as f32;
    let approval = result.approve_count as f32 / result.total_votes as f32;
    
    // Require both quorum AND threshold
    let quorum_met = participation >= config.min_quorum;
    let threshold_met = approval >= config.config_change_threshold;
    
    Ok(quorum_met && threshold_met)
}
```

### Step 7: Federation Contract Signing

```rust
async fn establish_federation(
    our_group: &GroupContract,
    their_group: &GroupContract,
) -> Result<FederationContract, Error> {
    // Both groups voted to approve
    let federation = FederationContract {
        group_a: our_group.hash(),
        group_b: their_group.hash(),
        established: Timestamp::now(),
        shared_validators: calculate_bridge_members().await?,
    };
    
    // Both bots sign
    let signed = federation.sign(our_group.key())?;
    freenet.publish_contract(signed).await?;
    
    Ok(federation)
}
```

## Cross-Mesh Vouching (Phase 4)

### Concept: Shadow Vouch

After federation, members from Group-B can vouch for invitees to Group-A.

**Example Flow:**
1. Alice (Group-A) invites Dan to Group-A
2. Dan is already a member of Group-B (federated) with 4 vouches
3. Bot detects Dan's presence in Group-B via shared contract
4. Bob (Group-A validator) vouches for Dan (second vouch)
5. Dan admitted to Group-A (now in BOTH groups)

**Benefits:**
- Faster vetting (already trusted in sister group)
- Groups act as mutual buffers
- Trust identity is portable
- Expedited admission for federated members

### Federated Merkle Trees

```rust
pub struct FederatedTrustNetworkState {
    // Local state (our group)
    local_members: BTreeSet<MemberHash>,
    local_vouches: HashMap<MemberHash, BTreeSet<MemberHash>>,
    
    // Federated state (sister groups)
    federation_contracts: Vec<ContractHash>,
    federated_members: HashMap<GroupHash, BTreeSet<MemberHash>>,
    
    // Bridge members (in multiple groups)
    bridge_members: HashSet<MemberHash>,
}
```

### Trust Validation Across Groups

```rust
async fn verify_cross_mesh_vouch(
    invitee: Hash,
    voucher_from_group_b: Hash,
    group_a: &TrustNetworkState,
    group_b: &TrustNetworkState,
) -> Result<bool, Error> {
    // Verify voucher is in Group-B
    let in_group_b = group_b.members.contains(&voucher_from_group_b);
    
    // Verify invitee is NOT in Group-A
    let not_in_group_a = !group_a.members.contains(&invitee);
    
    // Verify groups are federated
    let groups_federated = group_a.federation_contracts
        .contains(&group_b.contract_hash());
    
    Ok(in_group_b && not_in_group_a && groups_federated)
}
```

## Scaling Beyond Federation

### Multi-Hop Federation (Phase 5+)

```
Group-A ←--→ Group-B ←--→ Group-C

Group-A and Group-C are NOT directly federated,
but can discover each other via Group-B's social anchor.
```

**Process**:
- Group-A discovers Group-C via Group-B's beacon
- Both evaluate intersection density
- Both groups vote on federation
- Direct federation established (bypassing Group-B)

**Benefit**: Organic network growth without central coordinator

### Recursive Proofs (Phase 5+)

**Problem**: With many groups, ZK-proofs become expensive

**Solution**: Batch proofs for constant-time verification

```
Proof-1: Member X vouched in Group-A
Proof-2: Member X vouched in Group-B
Proof-3: Member X vouched in Group-C

Recursive Proof: Prove all 3 proofs are valid in single verification
```

**Benefit**: O(1) verification regardless of federation size

### Sybil Detection (Phase 5+)

**Problem**: At scale, coordinated Sybil attacks become possible

**Solution**: Multi-bot consensus on suspicious patterns

**Detection Signals**:
- Sudden spike in flags across multiple groups
- Identical vouch patterns (machine-like)
- Rapid membership growth (unnatural)
- Bridge member behavior anomalies

**Response**:
- Bot suggests enhanced vetting
- Groups vote on heightened security mode
- Temporary increase in min_vouch_threshold

## Design Principles

### 1. Emergent, Not Coordinated

**Wrong**: Admins exchange keys to federate groups  
**Right**: Bots find each other via shared members

**Implementation**: Social Anchor Hashing discovers natural overlap

### 2. Privacy-Preserving

**Wrong**: Share member lists to calculate overlap  
**Right**: PSI-CA reveals only count

**Implementation**: Bloom filters + commutative encryption

### 3. Asymmetric Safety

**Wrong**: Same threshold for all groups  
**Right**: Each group sets own threshold

**Implementation**: BidirectionalMin (both must approve with own criteria)

### 4. Human Control

**Wrong**: Bot automatically federates when overlap detected  
**Right**: Members vote on every federation

**Implementation**: Signal Polls for federation proposals

### 5. Fluid & Portable

**Wrong**: Trust is group-specific and non-transferable  
**Right**: Trust identity precedes members across meshes

**Implementation**: Cross-mesh vouching, shadow vouches

## Federation-Ready Design (MVP Phase 0-3)

Even though federation isn't implemented, the architecture is **ready**:

### Contract Schema Includes Federation Hooks

```rust
#[composable]
pub struct TrustNetworkState {
    // ... MVP fields ...
    
    // Federation hooks (PRESENT but EMPTY in MVP)
    #[cfg(feature = "federation")]
    federation_contracts: Vec<ContractHash>,  // Empty: []
    
    #[cfg(feature = "federation")]
    validator_anchors: BloomFilter,           // Computed but not broadcast
}
```

**Why Include Now**:
- Validates schema works for federation
- Ensures merge semantics support federated state
- No breaking changes needed in Phase 4
- Can test PSI-CA locally in Phase 3

### Module Structure Includes federation/

```rust
// src/federation/mod.rs
#[cfg(feature = "federation")] // DISABLED in MVP
pub mod shadow_beacon;

#[cfg(feature = "federation")]
pub mod psi_ca;

#[cfg(feature = "federation")]
pub mod diplomat;
```

**Why Include Now**:
- Validates architecture is modular
- Can develop/test federation logic separately
- Feature flag enables smooth transition
- No refactoring needed later

### Identity Hashing is Re-Computable

```rust
// Bot-scoped HMAC using ACI-derived key (each bot has unique Signal identity)
let key_a = derive_identity_masking_key(&bot_a_aci_identity);
let key_b = derive_identity_masking_key(&bot_b_aci_identity);

let hash_in_group_a = hmac::sign(&key_a, signal_id);
let hash_in_group_b = hmac::sign(&key_b, signal_id);

// Same person, different hashes in different groups (different bot ACI identities)
assert_ne!(hash_in_group_a, hash_in_group_b);
```

**Why This Matters**:
- Enables PSI-CA (privacy-preserving overlap calculation)
- Prevents cross-group identity correlation
- Same person appears as different hashes in different groups (different bot ACI keys)
- But overlap can still be detected via PSI protocol
- All cryptographic keys derived from Signal ACI identity (no separate key management)

## Migration Path: MVP → Federation

### Phase 3: Validate Design (Week 7)

**Objectives** (Compute Locally, Don't Broadcast):
1. Calculate Social Anchor hashing
2. Generate discovery URIs (don't publish)
3. Generate Bloom filters (don't broadcast)
4. Test PSI-CA with mock data (simulate two groups)
5. Validate BidirectionalMin logic
6. Confirm contract schema supports federation

**Deliverable**: Proof that federation design is viable

### Phase 4: Implement Federation

**Activate** (Enable Feature Flag):
```bash
cargo build --features federation
```

**Implement**:
1. Shadow Beacon broadcast (publish at discovery URIs)
2. Discovery URI monitoring (scan for other bots)
3. PSI-CA handshake (calculate real overlap)
4. Federation proposal (Signal Poll)
5. Contract signing (both groups)
6. Cross-mesh vouching (shadow vouches)
7. Shadow Handover Protocol (bot identity rotation)

**No Breaking Changes**: Just enable existing code

### Bot Identity Rotation: Shadow Handover (Phase 4+)

**Problem**: Signal phone numbers can be banned, compromised, or operators may wish to rotate for security reasons.

**Solution**: Shadow Handover Protocol for cryptographic succession.

**Concept**: Bot's Signal identity (phone number) is ephemeral; cryptographic identity (keypair) persists. When rotation is needed:

1. Bot-Old generates new keypair for Bot-New
2. Bot-Old creates Succession Document (signed)
3. Bot-Old deploys to Freenet contract
4. Bot-New proves possession of new_bot_privkey
5. Freenet contract validates; trust graph unchanged
6. Bot-New announces to Signal group

**Benefits**:
- Cryptographic proof of succession (not operator assertion)
- Trust context preserved (members' vouches unchanged)
- Freenet contract authorizes transition (decentralized)
- Aligns with fluid identity philosophy (identity is relational, not fixed)

**Operator CLI (Phase 4+)**:
```bash
stroma rotate \
  --config /etc/stroma/config.toml \
  --new-phone "+0987654321" \
  --reason "Signal ban recovery"
```

**MVP Workaround**: Operator manually handles Signal bans by re-registering with backup phone number.

See `.beads/federation-roadmap.bead` for full protocol specification.

## Federation Use Cases

### Use Case 1: Local Community Expansion

```
Group-A: Local activists (50 members)
Group-B: Labor organizers (60 members)

Overlap: 12 shared members (active in both communities)
Intersection Density: 12 / (50 + 60 - 12) = 12.2%

Both groups approve federation:
- Cross-community organizing now possible
- Trust spans both networks
- Total reach: 110 members
```

### Use Case 2: Geographic Bridge

```
Group-A: NYC organizers (80 members)
Group-B: Boston organizers (70 members)

Overlap: 8 members (live/work in both cities)
Intersection Density: 8 / (80 + 70 - 8) = 5.6%

Group-A threshold: 5% ✅
Group-B threshold: 10% ❌

Result: Federation rejected (Group-B threshold not met)
```

### Use Case 3: Thematic Connection

```
Group-A: Climate activists (100 members)
Group-B: Housing rights advocates (90 members)
Group-C: Food justice organizers (85 members)

A ↔ B overlap: 15 members (climate ∩ housing)
B ↔ C overlap: 18 members (housing ∩ food)
A ↔ C overlap: 12 members (climate ∩ food)

All three groups federate:
- Total network: 275 members
- Trust spans three issue areas
- Cross-movement organizing enabled
```

## Technical Challenges (Phase 4+)

### Challenge 1: State Synchronization

**Problem**: Multiple Freenet contracts (one per group) need coordination

**Solution**:
- Each group maintains own contract (independent)
- Federation contract tracks relationships (lightweight)
- Cross-references via contract hashes
- No shared state (just references)

### Challenge 2: Vouch Verification Across Groups

**Problem**: How to verify voucher from Group-B is valid in Group-A?

**Solution**:
```rust
// Group-A contract can reference Group-B contract
async fn verify_cross_mesh_voucher(
    voucher: Hash,
    group_b_contract: ContractHash,
) -> Result<bool, Error> {
    // Fetch Group-B state from Freenet
    let group_b_state = freenet.fetch_contract(group_b_contract).await?;
    
    // Verify voucher is active member of Group-B
    group_b_state.members.contains(&voucher)
}
```

### Challenge 3: Federation Dissolution

**Problem**: What if shared members leave?

**Solution**: Dynamic federation maintenance
- Monitor bridge member count
- If overlap drops below threshold → propose dissolution
- Groups vote on whether to maintain federation
- Graceful un-federation (no data loss)

### Challenge 4: Proof Size with Many Groups

**Problem**: Proving membership across 10+ groups creates large proofs

**Solution**: Recursive proofs (Phase 5+)
- Batch proofs for efficiency
- Constant verification time regardless of federation size
- STARK composition (proof-of-proofs)

## Success Criteria for Federation

When Phase 4 is complete, federation should:

- [ ] Bots discover each other without admin coordination
- [ ] PSI-CA reveals only overlap count (no identities)
- [ ] BidirectionalMin protects small groups from absorption
- [ ] Signal Poll voting for federation approval
- [ ] Both groups must approve before federation
- [ ] Cross-mesh vouching works after federation
- [ ] No social graph exposed during discovery
- [ ] Federation dissolves gracefully if bridge members leave
- [ ] Performance: < 10 seconds for PSI-CA handshake
- [ ] Trust map protection: Even if one group's bot is seized, can't enumerate other group's members (PSI-CA defense)

## Philosophy: Why Federation is the North Star

### Beyond Technology

Stroma isn't just about building a secure messaging bot. It's about:

**Mutual Arising**: Trust emerges from authentic relationships, not central authority

**Fluid Identity**: Your identity is relational - it arises from your connections, not a fixed profile

**Emergent Organization**: Groups self-organize without hierarchical coordination

**Scalable Privacy**: The network can grow to millions while protecting members from trust map seizure

### The Vision

Imagine a world where:
- Trust networks span continents
- No central authority validates identity
- Member identities protected even if infrastructure seized
- Groups self-organize around shared values
- Trust emerges from authentic human relationships
- Activists can coordinate without fear of trust map compromise

**That's why we build federation-ready from day one.**

Even though MVP is single-group, we design for the world we want to create - not just the prototype we're building.

---

## See Also

- [Architecture Decisions Bead](../.beads/architecture-decisions.bead) - Immutable design constraints
- [Federation Roadmap Bead](../.beads/federation-roadmap.bead) - Immutable north star
- [Developer Guide](DEVELOPER-GUIDE.md) - Technical implementation
- [Trust Model](TRUST-MODEL.md) - Trust mechanics

---

**Last Updated**: 2026-01-27
