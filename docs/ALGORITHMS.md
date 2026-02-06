# Stroma Algorithms: Trust Network Optimization

**For Technical Audience: Implementers, Auditors, and Algorithm Designers**

This document provides the mathematical foundations and algorithmic details for Stroma's trust network optimization. It covers both **internal matchmaking** (Blind Matchmaker) and **external federation** (Blind Rendezvous).

**Terminology**: See `.beads/terminology.bead` for canonical definitions of all terms used in this document.

## Table of Contents

1. [Core Principles](#core-principles)
2. [Graph Theory Foundations](#graph-theory-foundations)
3. [Internal Matchmaking: Minimum Spanning Tree Algorithm](#internal-matchmaking-minimum-spanning-tree-algorithm)
4. [External Federation: Private Set Intersection Algorithm](#external-federation-private-set-intersection-algorithm)
5. [Complexity Analysis](#complexity-analysis)
6. [Privacy Guarantees](#privacy-guarantees)
7. [Implementation Pseudocode](#implementation-pseudocode)
8. [Worked Examples](#worked-examples)

---

## Core Principles

### Optimization Goals

Stroma's algorithms balance **three competing objectives**:

1. **Maximum Anonymity**: Minimize bot's knowledge of social graph structure
2. **Minimum Interactions**: Achieve full mesh with fewest new relationships
3. **Maximum Resilience**: Every member connected via multiple independent paths

### Key Insight: Minimum Spanning Tree of Trust

A **Minimum Spanning Tree (MST)** provides the **optimal balance** between these objectives:

- **Anonymity**: Bot knows only vouch counts, not relationship content
- **Efficiency**: Minimal new introductions needed (N-1 edges for N nodes)
- **Resilience**: Every member reachable from every other member

### Trust Model Constraints

All algorithms must respect these invariants:

```
INVARIANT 1: Every member MUST have >= MIN_VOUCH_THRESHOLD vouches (default: 2)
INVARIANT 2: Vouches MUST come from members in DIFFERENT CLUSTERS (hard requirement)
INVARIANT 3: Bot MUST NOT know relationship content (only topology)
```

**INVARIANT 2 Enforcement** (Coordinated Infiltration Prevention):
- Same-cluster vouches do NOT count toward admission threshold
- Bot MUST verify `Cluster(Voucher_A) != Cluster(Voucher_B)` before admission
- Rejection message: "Second vouch must come from a different cluster than the inviter"
- **Bootstrap Exception**: First 3-5 members exempt (only one cluster exists)
- **Transition**: Cross-cluster enforced once cluster detection identifies 2+ distinct clusters

**Why This Is Required**: Without cross-cluster enforcement, a compromised cluster can self-amplify by rubber-stamping confederates. Cross-cluster forces verification from independent social contexts, making coordinated infiltration require deceiving multiple unrelated groups — which doesn't scale for attackers.

**See**: `.beads/cross-cluster-requirement.bead` for full threat model

---

## Graph Theory Foundations

### Trust Graph Definition

```
G = (V, E)

where:
  V = {m₁, m₂, ..., mₙ} = Set of members (HMAC-hashed identifiers)
  E = {(mᵢ, mⱼ) | mⱼ vouches for mᵢ} = Directed edges (vouch relationships)
```

**Properties:**
- **Directed**: Edge (j, i) means "j vouches for i"
- **Weighted**: Each edge has weight = 1 (uniform trust)
- **Sparse**: Most members have 2-5 vouches (not fully connected)

### Node Classification

Based on **in-degree** (number of incoming vouch edges):

```
NodeType(m) = 
  | Bridge     if in_degree(m) = MIN_VOUCH_THRESHOLD (default: 2)
  | Validator  if in_degree(m) >= MIN_VOUCH_THRESHOLD + 1 (default: 3+)
```

**Semantic Meaning:**
- **Bridge**: Minimum-membership member (2 vouches, vulnerable to single voucher departure)
- **Validator**: High-trust member (3+ vouches, resilient, good for verification)

**Note**: "Invitees" (sometimes called Leaf Nodes) are OUTSIDE the Signal group with only 1 vouch (being vetted). They are NOT represented in the internal trust graph since they're not yet members. The graph analysis operates only on admitted members (Bridges and Validators).

### Cluster Detection

**Definition**: A cluster is a **tight community** — members who are densely connected to each other but connected to other clusters only through bridge members.

**Problem with Union-Find**: Standard Union-Find finds connected components, but fails to distinguish tight clusters connected by bridges. It sees one large cluster when there are actually multiple tight communities connected by bridge members. (Validated in Spike Week Q3.)

**Algorithm**: Bridge Removal (Tarjan's Algorithm)

Bridge Removal identifies **articulation edges** (edges whose removal would disconnect the graph) and removes them to find tight components.

```
function FIND_CLUSTERS_BRIDGE_REMOVAL(G):
    // Step 1: Find all articulation edges (bridges)
    bridges = TARJAN_BRIDGES(G)
    
    // Step 2: Create subgraph without bridges
    G' = G.remove_edges(bridges)
    
    // Step 3: Find connected components in subgraph
    clusters = CONNECTED_COMPONENTS(G')
    
    // Step 4: Bridge members form their own "cluster"
    // (They can vouch but don't form tight cluster with either side)
    bridge_members = members_incident_to(bridges)
    
    return (clusters, bridge_members)

function TARJAN_BRIDGES(G):
    // Tarjan's algorithm for finding bridges
    // An edge (u, v) is a bridge if removing it disconnects the graph
    
    time = 0
    disc = {}      // Discovery time
    low = {}       // Lowest reachable discovery time
    bridges = []
    
    function dfs(u, parent):
        nonlocal time
        disc[u] = low[u] = time
        time += 1
        
        for v in neighbors(u):
            if v not in disc:
                dfs(v, u)
                low[u] = min(low[u], low[v])
                
                // If lowest reachable from v is beyond u, edge is a bridge
                if low[v] > disc[u]:
                    bridges.append((u, v))
            elif v != parent:
                low[u] = min(low[u], disc[v])
    
    for node in V:
        if node not in disc:
            dfs(node, None)
    
    return bridges
```

**Example (from Q3 Spike):**
```
Input Graph:
  Cluster A (tight):  Alice ←→ Bob ←→ Carol (all vouch each other)
  Bridge:             Charlie (vouched by Carol + Dave, vouches back)
  Cluster B (tight):  Dave ←→ Eve ←→ Frank (all vouch each other)

Bridge Removal Result:
  Cluster A: [Alice, Bob, Carol]
  Cluster B: [Dave, Eve, Frank]
  Bridge Members: [Charlie]  // Can vouch but doesn't form tight cluster
```

**Why Bridge Removal Works:**
- Automatically detects bridges without threshold tuning
- Correct semantics: bridges are members who connect otherwise-disconnected communities
- Predictable: same input always produces same output
- Well-understood: Tarjan's algorithm is O(V+E), widely used

**Complexity**: O(V + E) where V = members, E = vouch edges

**Cross-Cluster Enforcement Activation (GAP-11):**
When cluster detection identifies ≥2 distinct clusters (typically when group reaches 6+ members), the bot automatically activates cross-cluster vouching requirements:
- **Announcement**: "📊 Network update: Your group now has distinct sub-communities! Cross-cluster vouching is now required for new members. Existing members are grandfathered."
- **New invitees**: Must receive vouches from members in different clusters
- **Existing members**: Grandfathered (no action needed)
- **Detection frequency**: Runs on every membership change (fast, <1ms)
- **One-time only**: Announcement sent once when threshold first crossed

**See**: `docs/spike/q3/RESULTS.md` for validation results, `docs/USER-GUIDE.md` for user-facing behavior

### Centrality Measures

**Betweenness Centrality**: Measures how often a node lies on shortest paths between other nodes.

```
Betweenness(v) = Σ_{s≠v≠t} (σ_st(v) / σ_st)

where:
  σ_st = total number of shortest paths from s to t
  σ_st(v) = number of those paths passing through v
```

**Why it matters**: High-betweenness nodes are critical bridges between clusters.

---

## Two Distinct Blind Matchmaker Functions

The Blind Matchmaker serves two architecturally separate functions. Both use the DVR-optimized selection algorithm described in this section, but differ in purpose, trigger, and implementation module:

| Function | Purpose | Module | Trigger | Participants |
|----------|---------|--------|---------|--------------|
| **Admission Vetting** | Select cross-cluster assessor to evaluate an invitee | `signal/matchmaker.rs` | `/invite` | 1 invitee (leaf node) + 1 assessor (existing member) |
| **Mesh Optimization** | Suggest strategic introductions between existing members | `matchmaker/strategic_intro.rs` | `/mesh` suggestions, proactive bot behavior | 2 existing members |

**Admission Vetting**: After `/invite`, the bot adds the invitee as a leaf node in the trust graph, then selects an assessor from a different cluster. The bot PMs the assessor with the invitee's contact info. The assessor independently contacts the invitee. The assessor can vouch (`/vouch`) or decline (`/reject-intro`), which triggers re-selection with an exclusion list.

**Mesh Optimization**: The algorithm below describes this function — operating on existing members IN the Signal group to suggest cross-cluster introductions that improve DVR.

---

## Internal Matchmaking: Minimum Spanning Tree Algorithm

### Problem Statement (Mesh Optimization)

**Given:**
- Trust graph G = (V, E) with N members (all IN the Signal group)
- K clusters (disconnected components within the group)
- B vulnerable Bridges (in_degree = MIN_VOUCH_THRESHOLD, only 2 vouches)

**Goal:**
- Connect all clusters into single component
- Strengthen vulnerable Bridges to Validators (in_degree >= MIN_VOUCH_THRESHOLD + 1, 3+ vouches)
- Minimize total new introductions needed

**Constraint:**
- Bot knows only topology (vouch counts), not relationship content

### Algorithm: DVR-Optimized Blind Matchmaker (Hybrid)

The algorithm has three phases:
- **Phase 0**: DVR Optimization — prioritize distinct Validators
- **Phase 1**: MST Fallback — strengthen remaining Bridges
- **Phase 2**: Connect Clusters — bridge disconnected components

**See**: `.beads/blind-matchmaker-dvr.bead` for full rationale

```
Algorithm: BUILD_DVR_OPTIMIZED_TRUST_TREE(G)

Input: Trust graph G = (V, E)
Output: List of strategic introduction pairs (DVR-optimal where possible)

1. Classify nodes (members IN the Signal group):
   // Note: "Invitees" (1 vouch, OUTSIDE group) are not in this graph
   bridges = {v ∈ V | in_degree(v) = MIN_VOUCH_THRESHOLD}        // 2 vouches (minimum, vulnerable)
   validators = {v ∈ V | in_degree(v) >= MIN_VOUCH_THRESHOLD + 1} // 3+ vouches (resilient)

2. Detect clusters:
   clusters = FIND_CLUSTERS(G)

3. Initialize:
   introductions = []
   used_vouchers = {}  // Track vouchers used by distinct Validators
   
   // Collect voucher sets of existing distinct Validators
   for each distinct_validator in GET_DISTINCT_VALIDATORS(G):
       used_vouchers = used_vouchers ∪ vouchers_of(distinct_validator)

4. PHASE 0: DVR Optimization (Priority 0 — NEW)
   // Prioritize introductions that create DISTINCT Validators
   for each bridge in bridges:
       bridge_vouchers = vouchers_of(bridge)
       bridge_cluster = find_cluster(bridge, clusters)
       
       // Check if bridge's vouchers are already "used" by other distinct Validators
       vouchers_overlap = bridge_vouchers ∩ used_vouchers
       
       // Find voucher that is:
       // (a) In different cluster from bridge
       // (b) NOT already used by another distinct Validator
       candidate = argmax_{v ∈ validators} (
           centrality(v)
           WHERE find_cluster(v) ≠ bridge_cluster
           AND v ∉ used_vouchers
       )
       
       if candidate exists:
           introductions.append({
               person_a: bridge,
               person_b: candidate,
               reason: "Create distinct Validator (DVR optimization)",
               priority: 0,
               dvr_optimal: true
           })
           
           // Reserve this voucher and bridge's voucher set
           used_vouchers = used_vouchers ∪ bridge_vouchers ∪ {candidate}
           
           // Mark bridge as handled (don't process in Phase 1)
           bridges.remove(bridge)

5. PHASE 1: MST Fallback (Priority 1)
   // For bridges not handled in Phase 0, use any cross-cluster Validator
   for each bridge in bridges:  // Remaining bridges only
       bridge_cluster = find_cluster(bridge, clusters)
       
       // Find ANY validator from DIFFERENT cluster (no used_vouchers constraint)
       target_validator = argmax_{v ∈ validators} (
           centrality(v) 
           WHERE find_cluster(v) ≠ bridge_cluster
       )
       
       if target_validator exists:
           introductions.append({
               person_a: bridge,
               person_b: target_validator,
               reason: "Strengthen Bridge via cross-cluster vouch (MST fallback)",
               priority: 1,
               dvr_optimal: false
           })

5. PHASE 2: Bridge Disconnected Clusters (Priority 2)
   if |clusters| > 1:
       // Sort clusters by size (largest first)
       sorted_clusters = sort(clusters, key=|c| |c|, reverse=True)
       
       // Connect clusters in sequence
       for i from 0 to |sorted_clusters| - 2:
           cluster_a = sorted_clusters[i]
           cluster_b = sorted_clusters[i + 1]
           
           // Select highest-centrality validators from each cluster
           validator_a = argmax_{v ∈ cluster_a ∩ validators} centrality(v)
           validator_b = argmax_{v ∈ cluster_b ∩ validators} centrality(v)
           
           introductions.append({
               person_a: validator_a,
               person_b: validator_b,
               reason: "Bridge disconnected clusters",
               priority: 2
           })

6. Sort by priority and return:
   return sort(introductions, key=priority)
```

### Why This Works: Mathematical Proof

**Theorem**: The greedy MST algorithm achieves full mesh connectivity with minimal introductions.

**Proof Sketch:**

1. **Phase 1 Correctness** (Bridge Strengthening):
   - Each Bridge (2 vouches) needs exactly 1 additional vouch to become more resilient
   - Cross-cluster vouching maximizes intersectional diversity
   - Total introductions needed: B (number of Bridges with only 2 vouches)

2. **Phase 2 Correctness** (Cluster Linking):
   - K disconnected clusters require exactly K-1 connections to connect
   - Connecting sequentially (largest to smallest) minimizes disruption
   - Total introductions needed: K-1 (number of clusters minus 1)

3. **Optimality**:
   - Total introductions: B + (K-1)
   - This is minimal because:
     * Each vulnerable Bridge MUST get one more vouch (no way to reduce B)
     * K clusters MUST be connected via K-1 edges (MST property)
   - QED

**Complexity**: O(N log N + E) where N = |V|, E = |edges|

### Anonymity Preservation

**What Bot Knows:**
- Node classification (Bridge vs Validator) based on vouch count
- Cluster membership (which nodes are connected)
- Centrality scores (who is structurally important)

**What Bot Does NOT Know:**
- Why members vouch for each other (relationship content)
- Personal details or identities (only HMAC hashes)
- Who should be paired for social compatibility

**Privacy Guarantee**: Bot has **topological knowledge** but **zero semantic knowledge**.

### Privacy Model for Admission Vetting

When used for assessor selection (admission vetting), additional privacy constraints apply:

- **Inviter identity hidden from assessor**: Bot tells assessor "someone invited @invitee" but NOT who
- **Assessor identity hidden from inviter**: Bot tells inviter "reaching out to a cross-cluster member" but NOT who
- **Bot never contacts invitee**: The assessor independently decides how to approach the invitee
- **Assessor controls identity exposure**: The assessor decides what to reveal about themselves to the invitee
- **Bot belongs to ONE Signal group**: No secondary chats or 3-person groups are created
- **Exclusion list is ephemeral**: Members who declined via `/reject-intro` are tracked in RAM-only VettingSession

---

## External Federation: Private Set Intersection Algorithm

### Problem Statement

**Given:**
- Two Stroma groups: Group A (size N_A), Group B (size N_B)
- No shared knowledge (different bots, different contracts)
- Both groups use HMAC hashing with **different ACI-derived keys** (each bot has unique Signal identity)

**Goal:**
- Calculate overlap |A ∩ B| without revealing member identities
- Propose federation if overlap meets both groups' thresholds
- Maintain complete anonymity (zero knowledge of graph structure)

**Constraint:**
- Different bots → same person has different hashes (privacy requirement)

### Challenge: Different Hash Spaces

**Problem**: Alice in Group A has hash H_A(alice) = HMAC(alice, key_A)
            Alice in Group B has hash H_B(alice) = HMAC(alice, key_B)
            Since key_A ≠ key_B (different ACI identities), H_A(alice) ≠ H_B(alice)

**Solution**: Use **Social Anchor Hashing** + **Private Set Intersection with Cardinality (PSI-CA)**

### Social Anchor Hashing

**Concept**: Derive group identifier from top-N validators (stable, high-entropy members).

```
Algorithm: COMPUTE_SOCIAL_ANCHOR(group)

Input: Group with members and vouch counts
Output: Social anchor hash (group identifier)

1. Sort members by vouch count (descending)
2. Select top N validators (configurable, e.g., N = 10 or top 20%)
3. Normalize validator set:
   validators_sorted = sort(top_validators, key=hash)
4. Compute group anchor:
   anchor = HASH(validators_sorted[0] || validators_sorted[1] || ... || validators_sorted[N-1])
5. Return anchor
```

**Properties:**
- **Stable**: Top validators rarely change
- **Unique**: Different groups → different validator sets → different anchors
- **Discoverable**: Groups with shared validators will have related anchors

### Fibonacci Bucket Discovery

**Problem**: Different-sized groups need to discover each other

**Solution**: Generate discovery URIs at fixed Fibonacci bucket sizes

```
Algorithm: GENERATE_DISCOVERY_URIS(group)

Input: Group with members
Output: List of (bucket_size, anchor, uri)

// Fibonacci buckets (up to Signal's 1000-member limit)
FIBONACCI_BUCKETS = [3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377, 610, 987]
uris = []

for bucket_size in FIBONACCI_BUCKETS:
    if validator_count >= bucket_size:
        // Sort validators by hash for deterministic ordering
        top_validators = sorted_validators[:bucket_size]
        anchor = COMPUTE_SOCIAL_ANCHOR(top_validators)
        uri = "freenet://discovery/" + anchor
        uris.append((bucket_size, anchor, uri))

return uris
```

**Why Fibonacci Buckets**:
- **Natural scaling** - Growth pattern (3→5→8→13...) reflects organic group expansion
- **Better granularity at small sizes** - More buckets in 3-89 range where most groups live
- **Fixed counts** (not percentiles) → groups of different sizes produce MATCHING hashes
- **Discovery requires overlap** - Must share actual validators, not just similar percentages

**Strategy**: Bot publishes Bloom Filters at ALL buckets it can fill, scans ALL bucket URIs.

### Private Set Intersection with Cardinality (PSI-CA)

**Goal**: Calculate |A ∩ B| without revealing A or B

**Protocol**: Commutative Encryption (Double-Blinding)

```
Algorithm: PSI_CA_HANDSHAKE(group_a, group_b)

Phase 1: Group A Setup
  1. Bot A generates ephemeral key pair (sk_a, pk_a)
  2. For each member m in A:
       // CRITICAL: Cleartext access is ONLY for PSI-CA ephemeral encryption
       // This is the ONE exception where cleartext is accessed - immediately zeroized
       // In ALL other code paths, Signal IDs are hashed immediately upon receipt
       encrypted_m = E(pk_a, cleartext_signal_id(m))  // Encrypt immediately
  3. Send {encrypted_m} to Bot B
  4. Zeroize cleartext_signal_id IMMEDIATELY after encryption (MANDATORY)

Phase 2: Group B Double-Blind
  5. Bot B generates ephemeral key pair (sk_b, pk_b)
  6. For each encrypted_a in received set:
       double_blind_a = E(pk_b, encrypted_a)  // Encrypt already-encrypted data
  7. For each member n in B:
       // CRITICAL: Same exception - cleartext for PSI-CA only, immediately zeroized
       encrypted_n = E(pk_b, cleartext_signal_id(n))  // Encrypt immediately
       double_blind_b = E(pk_a, encrypted_n)  // Use A's public key
  8. Send {double_blind_a} and {double_blind_b} to Bot A
  9. Zeroize cleartext_signal_id IMMEDIATELY (MANDATORY)

Phase 3: Group A Intersection Calculation
  10. Bot A completes double-blinding on its own set:
       for each member m in A:
           double_blind_m = E(pk_b, E(pk_a, m))  // Now both keys applied
  11. Calculate intersection:
       overlap = |{double_blind_m} ∩ {double_blind_a}|
  12. Return overlap COUNT only (not identities)

Phase 4: Ephemeral Key Destruction
  13. Both bots zeroize ephemeral keys immediately
  14. No persistent state retained
```

**Security Properties:**
- **Commutative Encryption**: E(pk_a, E(pk_b, m)) = E(pk_b, E(pk_a, m))
- **Double-Blinding**: Neither bot can decrypt the other's members alone
- **Cardinality Only**: Only intersection count is revealed, not identities
- **Ephemeral Keys**: Keys destroyed after handshake (no replay attacks)

### BidirectionalMin Federation Threshold

**Concept**: Both groups must independently satisfy their own thresholds.

```
Algorithm: EVALUATE_FEDERATION_VIABILITY(group_a, group_b, overlap)

Input: 
  - group_a with size N_a and threshold T_a
  - group_b with size N_b and threshold T_b
  - overlap count |A ∩ B|

Output: (a_accepts, b_accepts, recommend_federation)

1. Calculate intersection densities:
   density_a = overlap / N_a
   density_b = overlap / N_b

2. Check thresholds:
   a_accepts = (density_a >= T_a)
   b_accepts = (density_b >= T_b)

3. Recommend federation only if BOTH accept:
   recommend = (a_accepts AND b_accepts)

4. Return (a_accepts, b_accepts, recommend)
```

**Example:**
```
Group A: 100 members, T_a = 10%
Group B: 20 members, T_b = 30%
Overlap: 15 members

density_a = 15/100 = 15% >= 10% ✅ (A accepts)
density_b = 15/20 = 75% >= 30% ✅ (B accepts)

Result: Recommend federation
```

**Asymmetry Handling**: Smaller groups naturally require higher absolute overlap.

### Federation Decision Protocol

```
Algorithm: PROPOSE_FEDERATION(group_a, group_b, overlap)

Input: Two groups and calculated overlap
Output: Federation contract (if both groups vote yes)

1. Bot A sends proposal to Group A via Signal Poll:
   "🔗 Federation Proposal
    Group B (size: 20) shares 15 members with us.
    Intersection density: 15% (our threshold: 10%)
    
    Vote: ✅ Approve Federation | ❌ Reject | ⏸️ Abstain"

2. Bot B sends similar proposal to Group B

3. Wait for votes (require min_quorum participation AND config_change_threshold approval)

4. If BOTH groups approve:
   - Bots sign federation contract on Freenet
   - Share Merkle Tree roots (for ZK-proof verification)
   - Enable cross-mesh vouching
   - Monitor bridge density (proactive maintenance)

5. If EITHER group rejects:
   - Discard ephemeral keys
   - No federation established
   - Bots may retry later if overlap changes
```

---

## Complexity Analysis

### Internal Matchmaking (Blind Matchmaker)

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| Graph construction | O(N + E) | N = members, E = vouch edges |
| Node classification | O(N) | Single pass over vertices |
| Cluster detection (Bridge Removal) | O(N + E) | Tarjan's algorithm |
| Centrality calculation | O(N × E) | Betweenness centrality (Brandes) |
| Bridge strengthening | O(B × V) | B = Bridges (2 vouches), V = Validators |
| Cluster linking | O(K²) | K = clusters (usually K << N) |
| **Total** | **O(N × E)** | Dominated by centrality calculation |

**Space Complexity**: O(N + E) for graph storage

**Practical Performance** (empirical targets):
- 20 members: < 10ms
- 100 members: < 50ms
- 500 members: < 200ms
- 1000 members: < 500ms

### External Federation (Blind Rendezvous)

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| Social Anchor computation | O(N log N) | Sorting validators |
| Bloom Filter generation | O(N × k) | k = hash functions (typically 3-5) |
| PSI-CA encryption (per group) | O(N × T_enc) | T_enc = encryption time |
| PSI-CA intersection | O(N_a × N_b) | Worst case (optimized with hashing) |
| Threshold evaluation | O(1) | Simple arithmetic |
| **Total per handshake** | **O(N_a × N_b)** | Dominated by PSI-CA |

**Optimization**: Use Bloom Filters to reduce PSI-CA candidates (O(N × k) instead of O(N²))

**Practical Performance** (empirical targets):
- 100 members each: < 2 seconds
- 500 members each: < 10 seconds
- 1000 members each: < 30 seconds

---

## Privacy Guarantees

### Internal Matchmaking Privacy Model

**Threat Model**: State-level adversary seizes bot server or compromises operator

**Three-Layer Defense Against Trust Map Seizure:**

| Layer | Defense | Result if Compromised |
|-------|---------|---------------------|
| **No Centralized Storage** | Freenet distributed state | Adversary needs multiple peer seizures |
| **Cryptographic Privacy** | HMAC-hashed identifiers | Memory dumps contain only hashes |
| **Metadata Isolation** | 1-on-1 PMs only | No Signal group metadata to analyze |

**Information Available to Adversary (if bot compromised):**

| What Adversary Gets | Privacy Impact | Why It's Safe |
|-------------------|----------------|---------------|
| Hashed identifiers | LOW - can't reverse HMAC | Group-secret pepper required to correlate |
| Vouch counts | LOW - aggregate only | No cleartext identities |
| Cluster membership (hashes) | MEDIUM - topology visible | But can't identify who they are |
| Introduction suggestions | LOW - just matching logic | No personal relationship content |

**Guaranteed Protections**:
- ✅ Adversary cannot reverse hashes to identities (HMAC with secret pepper)
- ✅ Adversary cannot correlate identities across groups (different peppers = different hashes)
- ✅ Adversary cannot learn relationship content (only topology visible)
- ✅ Adversary needs multiple peer seizures to reconstruct state (Freenet distribution)
- ✅ Memory dumps reveal only hashes, not cleartext (immediate zeroization)

### External Federation Privacy Model

**Threat Model**: Malicious group or state adversary attempts to enumerate other group's members

**Federation-Specific Attack Vectors:**

| Attack Vector | Threat | Defense |
|--------------|--------|---------|
| **Fake Group Enumeration** | Create fake group to extract member list | PSI-CA double-blinding (neither side can decrypt alone) |
| **Timing Analysis** | Infer group size from response latency | Add random delays (constant-time operations) |
| **Sybil Attack** | Flood with fake members to boost overlap | Require ZK-proof of existing vouches before federation |
| **Replay Attack** | Reuse captured PSI-CA messages | Ephemeral keys destroyed after handshake |
| **Cross-Group Tracking** | Correlate same person across groups | Different Signal ACI keys = different hashes |
| **Bloom Filter Analysis** | Deduce members from filter patterns | Multi-threshold publishing (adds noise) |

**Three-Layer Defense (Applied to Federation):**

1. **No Centralized Registry**: Shadow Beacon uses emergent discovery (no admin coordination, no seizure target)
2. **Cryptographic Privacy**: PSI-CA double-blinding (neither group can decrypt other's members alone)
3. **Metadata Isolation**: Only overlap COUNT revealed, not identities

**Guaranteed Protections**:
- ✅ Neither bot can decrypt the other's member list without cooperation
- ✅ Only intersection COUNT revealed, never identities
- ✅ Ephemeral keys prevent replay or correlation attacks
- ✅ Different hash spaces prevent cross-group tracking (same person = different hash)
- ✅ If adversary seizes one group's bot, they can't enumerate the other group's members

**Formal Security**: PSI-CA is secure under the Decisional Diffie-Hellman (DDH) assumption (computational hardness)

---

## Implementation Pseudocode

### Internal: Strategic Introduction Selector

```rust
use petgraph::Graph;
use std::collections::{HashMap, HashSet};

pub struct StrategicMatcher {
    graph: Graph<MemberHash, ()>,
    thresholds: Thresholds,
}

impl StrategicMatcher {
    /// Main entry point: Generate strategic introduction pairs
    pub fn generate_introductions(&self) -> Vec<IntroductionPair> {
        let mut pairs = Vec::new();
        
        // Step 1: Classify nodes (members IN the group)
        // Note: Invitees (1 vouch, OUTSIDE group) are not in this graph
        let vulnerable_bridges = self.find_vulnerable_bridges();  // 2 vouches, need strengthening
        let validators = self.find_validators();  // 3+ vouches
        let clusters = self.detect_clusters();
        
        // Step 2: Calculate centrality for all validators
        let centrality = self.compute_betweenness_centrality(&validators);
        
        // Step 3: PHASE 1 - Strengthen vulnerable Bridges (only 2 vouches)
        for bridge in vulnerable_bridges {
            let bridge_cluster = self.find_cluster_id(bridge, &clusters);
            
            // Find best validator from different cluster
            let target = validators.iter()
                .filter(|v| self.find_cluster_id(**v, &clusters) != bridge_cluster)
                .max_by_key(|v| centrality[v])
                .cloned();
            
            if let Some(validator) = target {
                pairs.push(IntroductionPair {
                    person_a: bridge,
                    person_b: validator,
                    reason: "Cross-cluster vouch to strengthen Bridge (2→3+ vouches)".to_string(),
                    priority: 1,
                });
            }
        }
        
        // Step 4: PHASE 2 - Link clusters
        if clusters.len() > 1 {
            pairs.extend(self.link_clusters(&clusters, &validators, &centrality));
        }
        
        // Step 5: Sort by priority
        pairs.sort_by_key(|p| p.priority);
        pairs
    }
    
    /// Find Bridges with only MIN_VOUCH_THRESHOLD vouches (vulnerable to single departure)
    fn find_vulnerable_bridges(&self) -> Vec<MemberHash> {
        self.graph.node_indices()
            .filter(|&n| {
                self.graph.edges_directed(n, petgraph::Incoming).count() 
                    == self.thresholds.min_vouch  // Exactly 2 vouches
            })
            .map(|n| self.graph[n])
            .collect()
    }
    
    fn find_validators(&self) -> Vec<MemberHash> {
        self.graph.node_indices()
            .filter(|&n| {
                self.graph.edges_directed(n, petgraph::Incoming).count() 
                    > self.thresholds.min_vouch + 1
            })
            .map(|n| self.graph[n])
            .collect()
    }
    
    fn detect_clusters(&self) -> Vec<HashSet<MemberHash>> {
        // Bridge Removal (Tarjan's algorithm) - NOT Union-Find
        // Union-Find fails to distinguish tight clusters connected by bridges (Q3 Spike)
        
        // Step 1: Find articulation edges (bridges) using Tarjan's algorithm
        let bridges = self.find_bridges();
        
        // Step 2: Create subgraph without bridge edges
        let mut subgraph = self.graph.clone();
        for (u, v) in &bridges {
            if let Some(edge) = subgraph.find_edge(*u, *v) {
                subgraph.remove_edge(edge);
            }
        }
        
        // Step 3: Find connected components in subgraph (these are tight clusters)
        let mut visited = HashSet::new();
        let mut clusters = Vec::new();
        
        for node in subgraph.node_indices() {
            if !visited.contains(&node) {
                let mut cluster = HashSet::new();
                let mut stack = vec![node];
                
                while let Some(current) = stack.pop() {
                    if visited.insert(current) {
                        cluster.insert(subgraph[current]);
                        for neighbor in subgraph.neighbors(current) {
                            if !visited.contains(&neighbor) {
                                stack.push(neighbor);
                            }
                        }
                    }
                }
                clusters.push(cluster);
            }
        }
        
        clusters
    }
    
    fn find_bridges(&self) -> Vec<(NodeIndex, NodeIndex)> {
        // Tarjan's algorithm for finding bridge edges
        let mut time = 0;
        let mut disc: HashMap<NodeIndex, usize> = HashMap::new();
        let mut low: HashMap<NodeIndex, usize> = HashMap::new();
        let mut bridges = Vec::new();
        
        fn dfs(
            graph: &Graph<MemberHash, ()>,
            u: NodeIndex,
            parent: Option<NodeIndex>,
            time: &mut usize,
            disc: &mut HashMap<NodeIndex, usize>,
            low: &mut HashMap<NodeIndex, usize>,
            bridges: &mut Vec<(NodeIndex, NodeIndex)>,
        ) {
            disc.insert(u, *time);
            low.insert(u, *time);
            *time += 1;
            
            for v in graph.neighbors(u) {
                if !disc.contains_key(&v) {
                    dfs(graph, v, Some(u), time, disc, low, bridges);
                    low.insert(u, low[&u].min(low[&v]));
                    
                    // If lowest reachable from v is beyond u, edge is a bridge
                    if low[&v] > disc[&u] {
                        bridges.push((u, v));
                    }
                } else if parent != Some(v) {
                    low.insert(u, low[&u].min(disc[&v]));
                }
            }
        }
        
        for node in self.graph.node_indices() {
            if !disc.contains_key(&node) {
                dfs(&self.graph, node, None, &mut time, &mut disc, &mut low, &mut bridges);
            }
        }
        
        bridges
    }
    
    fn compute_betweenness_centrality(
        &self,
        validators: &[MemberHash]
    ) -> HashMap<MemberHash, f64> {
        // Brandes' algorithm (simplified)
        let mut centrality = HashMap::new();
        
        for validator in validators {
            // BFS from each node to compute shortest paths
            // Accumulate centrality scores
            // (Full implementation in src/matchmaker/graph_analysis.rs)
            centrality.insert(*validator, 0.0);
        }
        
        centrality
    }
    
    fn bridge_clusters(
        &self,
        clusters: &[HashSet<MemberHash>],
        validators: &[MemberHash],
        centrality: &HashMap<MemberHash, f64>
    ) -> Vec<IntroductionPair> {
        let mut bridges = Vec::new();
        
        // Sort clusters by size (descending)
        let mut sorted_clusters = clusters.to_vec();
        sorted_clusters.sort_by_key(|c| std::cmp::Reverse(c.len()));
        
        // Connect adjacent clusters
        for i in 0..sorted_clusters.len() - 1 {
            let cluster_a = &sorted_clusters[i];
            let cluster_b = &sorted_clusters[i + 1];
            
            // Find best validator from each cluster
            let validator_a = validators.iter()
                .filter(|v| cluster_a.contains(v))
                .max_by_key(|v| centrality[v])
                .cloned();
            
            let validator_b = validators.iter()
                .filter(|v| cluster_b.contains(v))
                .max_by_key(|v| centrality[v])
                .cloned();
            
            if let (Some(a), Some(b)) = (validator_a, validator_b) {
                bridges.push(IntroductionPair {
                    person_a: a,
                    person_b: b,
                    reason: format!("Bridge cluster {} to cluster {}", i, i + 1),
                    priority: 2,
                });
            }
        }
        
        bridges
    }
    
}
```

### External: PSI-CA Protocol Implementation

```rust
use ring::{agreement, rand};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(ZeroizeOnDrop)]
struct EphemeralKey {
    private_key: agreement::EphemeralPrivateKey,
    public_key: Vec<u8>,
}

pub struct PSIProtocol {
    group_size: usize,
    threshold: f32,
}

impl PSIProtocol {
    /// Phase 1: Encrypt own members with ephemeral key
    /// 
    /// CRITICAL: This is the ONE exception where bulk cleartext access Signal IDs are accessed.
    /// This is ONLY for PSI-CA federation discovery. In ALL other code paths,
    /// Signal IDs are HMAC-hashed immediately upon receipt and never stored.
    /// See: security-constraints.bead § 1 (Anonymity-First Design)
    pub fn phase1_encrypt_members(
        &self,
        members: &[String],  // Cleartext Signal IDs (PSI-CA exception - immediately zeroized)
    ) -> (EphemeralKey, Vec<Vec<u8>>) {
        let rng = rand::SystemRandom::new();
        
        // Generate ephemeral key pair
        let private_key = agreement::EphemeralPrivateKey::generate(
            &agreement::X25519, &rng
        ).unwrap();
        let public_key = private_key.compute_public_key().unwrap();
        
        // Encrypt each member
        let mut encrypted = Vec::new();
        for member in members {
            let ciphertext = self.encrypt(&public_key, member.as_bytes());
            encrypted.push(ciphertext);
            
            // CRITICAL: Zeroize cleartext IMMEDIATELY after encryption (MANDATORY)
            // Use ZeroizeOnDrop derive macro for member strings in actual implementation
        }
        
        (EphemeralKey { private_key, public_key: public_key.as_ref().to_vec() }, encrypted)
    }
    
    /// Phase 2: Double-blind encryption (receive other group's encrypted set)
    pub fn phase2_double_blind(
        &self,
        our_key: &EphemeralKey,
        their_public_key: &[u8],
        their_encrypted: &[Vec<u8>],
        our_members: &[String],
    ) -> (Vec<Vec<u8>>, Vec<Vec<u8>>) {
        // Re-encrypt their set with our key (double-blind)
        let their_double_blind: Vec<_> = their_encrypted.iter()
            .map(|ct| self.encrypt(&our_key.public_key, ct))
            .collect();
        
        // Encrypt our set with their key, then our key (double-blind)
        let our_double_blind: Vec<_> = our_members.iter()
            .map(|m| {
                let ct1 = self.encrypt(their_public_key, m.as_bytes());
                self.encrypt(&our_key.public_key, &ct1)
            })
            .collect();
        
        (their_double_blind, our_double_blind)
    }
    
    /// Phase 3: Calculate intersection cardinality
    pub fn phase3_calculate_overlap(
        &self,
        our_double_blind: &[Vec<u8>],
        their_double_blind: &[Vec<u8>],
    ) -> usize {
        // Convert to sets for efficient intersection
        let our_set: HashSet<_> = our_double_blind.iter().collect();
        let their_set: HashSet<_> = their_double_blind.iter().collect();
        
        // Intersection count (NOT identities)
        our_set.intersection(&their_set).count()
    }
    
    /// Evaluate if federation should be proposed
    pub fn evaluate_federation(
        &self,
        overlap: usize,
        other_group_size: usize,
    ) -> bool {
        let our_density = overlap as f32 / self.group_size as f32;
        our_density >= self.threshold
    }
    
    // Helper: Encrypt with public key (simplified - use proper ECIES in production)
    fn encrypt(&self, public_key: &[u8], plaintext: &[u8]) -> Vec<u8> {
        // Real implementation: Use ECIES (Elliptic Curve Integrated Encryption Scheme)
        // For now: placeholder (to be implemented with `ecies` crate)
        vec![]
    }
}

// Usage example
async fn discover_and_federate() {
    let psi = PSIProtocol {
        group_size: 100,
        threshold: 0.10,  // 10% intersection density
    };
    
    // Our members (cleartext - ephemeral)
    let our_members = load_members_from_freenet().await;
    
    // Phase 1: Encrypt with our ephemeral key
    let (our_key, our_encrypted) = psi.phase1_encrypt_members(&our_members);
    
    // Send our_encrypted to other group via Freenet
    send_to_other_group(&our_encrypted).await;
    
    // Receive their encrypted set
    let (their_public_key, their_encrypted) = receive_from_other_group().await;
    
    // Phase 2: Double-blind encryption
    let (their_double_blind, our_double_blind) = psi.phase2_double_blind(
        &our_key,
        &their_public_key,
        &their_encrypted,
        &our_members,
    );
    
    // Phase 3: Calculate overlap
    let overlap = psi.phase3_calculate_overlap(&our_double_blind, &their_double_blind);
    
    // Evaluate federation
    if psi.evaluate_federation(overlap, their_encrypted.len()) {
        propose_federation_to_group(overlap, their_encrypted.len()).await;
    }
    
    // CRITICAL: Zeroize ephemeral keys
    drop(our_key);  // ZeroizeOnDrop ensures cleanup
}
```

**Implementation Status**: ✅ **COMPLETED** - PSI-CA protocol is fully implemented in `src/crypto/psi_ca.rs`

The implementation includes:
- **Commutative encryption** using bytewise addition (mock implementation - production should use ECIES/DH)
- **Three-phase protocol** (encrypt, double-blind, calculate overlap)
- **Ephemeral key zeroization** for security
- **Federation threshold evaluation** with configurable density requirements
- **Comprehensive test suite** with mock data (no real broadcasts)

Test coverage:
- Ephemeral key generation and uniqueness
- Federation threshold validation
- Commutative property verification
- Full protocol execution with partial overlap (5 of 10 members)
- Edge cases: no overlap, complete overlap

See `cargo test crypto::psi_ca` for test execution.

---

## Worked Examples

### Example 1: Internal Matchmaking (20-Person Group)

#### Initial State

```
Group: 20 members (all IN the Signal group)
Clusters: 3 (Artist, Engineer, Activist)
Vulnerable Bridges: 5 (only 2 vouches - need strengthening)

Note: "Invitees" (1 vouch) are OUTSIDE the group and not shown here.

Cluster 1 (Artist): 8 members
  - Alice (2 vouches) ← Bridge (vulnerable)
  - Bob (4 vouches) ← Validator
  - Carol (3 vouches) ← Validator
  - David (2 vouches) ← Bridge (vulnerable)
  - Eve (5 vouches) ← Validator
  - Frank (3 vouches) ← Validator
  - Grace (2 vouches) ← Bridge (vulnerable)
  - Henry (4 vouches) ← Validator

Cluster 2 (Engineer): 7 members
  - Ivy (3 vouches) ← Validator
  - Jack (2 vouches) ← Bridge (vulnerable)
  - Kim (4 vouches) ← Validator
  - Leo (3 vouches) ← Validator
  - Mia (5 vouches) ← Validator
  - Nina (3 vouches) ← Validator
  - Oscar (2 vouches) ← Bridge (vulnerable)

Cluster 3 (Activist): 5 members
  - Paul (3 vouches) ← Validator
  - Quinn (4 vouches) ← Validator
  - Rita (3 vouches) ← Validator
  - Sam (2 vouches) ← Bridge (vulnerable, disconnected - needs cross-cluster connection)
  - Tina (3 vouches) ← Validator
```

#### Algorithm Execution

**Step 1**: Classify nodes (members IN the group)
- Bridges (2 vouches, vulnerable): Alice, David, Grace, Jack, Oscar, Sam (6 total)
- Validators (3+ vouches): Bob, Carol, Eve, Frank, Henry, Ivy, Kim, Leo, Mia, Nina, Paul, Quinn, Rita, Tina (14 total)

**Step 2**: Detect clusters
- Cluster 1 (Artist): 8 members
- Cluster 2 (Engineer): 7 members
- Cluster 3 (Activist): 5 members (NOTE: Sam is disconnected - 0 vouches from other clusters)

**Step 3**: Calculate centrality (simplified)
- Eve: 0.82 (high betweenness - central in Artist cluster)
- Mia: 0.79 (high betweenness - central in Engineer cluster)
- Quinn: 0.71 (high betweenness - central in Activist cluster)

**Step 4**: PHASE 1 - Strengthen Vulnerable Bridges (2 vouches → 3+ vouches)

| Bridge (vulnerable) | Current Cluster | Target Validator | Target Cluster | Priority |
|---------------------|----------------|------------------|----------------|----------|
| Alice | Artist | Mia | Engineer | 1 |
| David | Artist | Quinn | Activist | 1 |
| Grace | Artist | Mia | Engineer | 1 |
| Jack | Engineer | Eve | Artist | 1 |
| Oscar | Engineer | Quinn | Activist | 1 |
| Sam | Activist | Eve | Artist | 1 |

**Step 5**: PHASE 2 - Link Clusters

Since all clusters are already connected via existing cross-cluster vouches, no additional bridges needed.

BUT: Wait - checking graph connectivity reveals Sam is NOT connected to other clusters!

Additional bridge needed:
| Validator A | Cluster A | Validator B | Cluster B | Priority |
|-------------|-----------|-------------|-----------|----------|
| Quinn | Activist | Eve | Artist | 2 |

**Step 6**: Final Introduction List

```
Total introductions needed: 7

Priority 1 (Strengthen Vulnerable Bridges):
  1. Alice ↔ Mia (Artist Bridge → Engineer Validator)
  2. David ↔ Quinn (Artist Bridge → Activist Validator)
  3. Grace ↔ Mia (Artist Bridge → Engineer Validator)
  4. Jack ↔ Eve (Engineer Bridge → Artist Validator)
  5. Oscar ↔ Quinn (Engineer Bridge → Activist Validator)
  6. Sam ↔ Eve (Activist Bridge → Artist Validator)

Priority 2 (Link Clusters):
  7. Quinn ↔ Eve (Activist Validator → Artist Validator)
```

#### Result After Implementation

```
All 20 members now have >= 3 vouches
All 3 clusters connected
Fully intersectional mesh achieved with only 6 new introductions
```

**Efficiency**: 6 introductions for 20-person group = 30% interaction rate (minimal)

---

### Example 2: External Federation (Two Groups)

#### Initial State

```
Group A (Seattle):
  - Size: 100 members
  - Threshold: 10% (need >= 10 shared members)
  - Top validators: Alice, Bob, Carol, David, Eve

Group B (Portland):
  - Size: 20 members
  - Threshold: 30% (need >= 6 shared members)
  - Top validators: Carol, Frank, Grace, Henry
```

#### Algorithm Execution

**Step 1**: Social Anchor Computation

Group A (top 20%):
```
validators = [Alice, Bob, Carol, David, Eve, Frank, Grace, ...]
sorted = sort([hash(Alice), hash(Bob), ...])
anchor_a = SHA256(sorted[0] || sorted[1] || ... || sorted[19])
uri_a = "freenet://discovery/" + anchor_a
```

Group B (top 20%):
```
validators = [Carol, Frank, Grace, Henry]
sorted = sort([hash(Carol), hash(Frank), hash(Grace), hash(Henry)])
anchor_b = SHA256(sorted[0] || sorted[1] || sorted[2] || sorted[3])
uri_b = "freenet://discovery/" + anchor_b
```

**Step 2**: Discovery

- Bot A publishes Bloom Filter at uri_a
- Bot B publishes Bloom Filter at uri_b
- Bots scan discovery URIs and find potential match (shared validators: Carol, Frank, Grace)

**Step 3**: PSI-CA Handshake

Group A encrypts all 100 members:
```
encrypted_a = [E(pk_a, alice_signal_id), E(pk_a, bob_signal_id), ...]
```

Group B receives and double-blinds:
```
double_blind_a = [E(pk_b, encrypted_a[0]), E(pk_b, encrypted_a[1]), ...]
encrypted_b = [E(pk_b, carol_signal_id), E(pk_b, frank_signal_id), ...]
double_blind_b = [E(pk_a, encrypted_b[0]), E(pk_a, encrypted_b[1]), ...]
```

Group A completes double-blinding and calculates intersection:
```
our_double_blind = [E(pk_b, E(pk_a, alice)), ...]
overlap = |our_double_blind ∩ double_blind_a| = 15 members
```

**Step 4**: Threshold Evaluation

Group A:
```
density_a = 15 / 100 = 15% >= 10% ✅ (A accepts)
```

Group B:
```
density_b = 15 / 20 = 75% >= 30% ✅ (B accepts)
```

**Step 5**: Federation Proposal

Bot A sends Signal Poll to Group A:
```
🔗 Federation Proposal

Group: Portland Stroma (20 members)
Shared members: 15 (15% of our group)
Our threshold: 10%

This federation would allow cross-mesh vouching and 
increase our collective trust network size to 105 members.

Vote: ✅ Approve | ❌ Reject | ⏸️ Abstain
```

Bot B sends similar poll to Group B.

**Step 6**: Vote Results

- Group A: 82 approve, 12 reject, 6 abstain → 82% approval, 100% participation (quorum + threshold met) ✅
- Group B: 17 approve, 2 reject, 1 abstain → 85% approval, 100% participation (quorum + threshold met) ✅

**Step 7**: Federation Established

- Both bots sign federation contract on Freenet
- Merkle Tree roots shared (for ZK-proof verification)
- Cross-mesh vouching enabled:
  * Members of Group A can vouch for invitees to Group B
  * Members of Group B can vouch for invitees to Group A

#### Result

```
Federated Network:
  - Total members: 105 (100 + 20 - 15 duplicate)
  - Cross-mesh vouching enabled
  - Complete privacy preserved (no graph structure revealed)
```

---

## Trade-offs and Design Decisions

### Internal Matchmaking

**Trade-off**: Bot knowledge vs optimization efficiency

| Approach | Bot Knowledge | Efficiency | Privacy |
|----------|--------------|------------|---------|
| Random pairing | None (blind) | Poor (many introductions) | Maximum |
| Cluster-aware | Topology only | Good (MST optimal) | High |
| Full graph | All relationships | Optimal | Low ❌ |

**Stroma's Choice**: Cluster-aware (middle ground)
- Bot knows topology (vouch counts, clusters)
- Bot doesn't know relationship content
- Achieves MST optimality with high privacy

### External Federation

**Trade-off**: Discovery speed vs anonymity

| Approach | Discovery Speed | Anonymity | Complexity |
|----------|----------------|-----------|------------|
| Pre-shared keys | Instant | None ❌ | Low |
| Centralized registry | Fast | Low ❌ | Low |
| Social Anchor (Stroma) | Medium | High ✅ | Medium |
| Fully random | Very slow | Maximum | High |

**Stroma's Choice**: Social Anchor Hashing
- Emergent discovery (no admin coordination)
- High anonymity (PSI-CA reveals only count)
- Reasonable speed (multi-threshold optimization)

---

## Future Optimizations

### Internal Matchmaking

1. **Adaptive Thresholds**: Dynamic validator threshold based on group growth
2. **Proactive Pairing**: Suggest introductions before Bridges become vulnerable (single voucher departure)
3. **Quality Metrics**: Use vouch success rate to refine centrality scores
4. **Parallel Processing**: Calculate centrality in background (async)

### External Federation

1. **Bloom Filter Optimization**: Reduce false positive rate for faster discovery
2. **Incremental PSI**: Update overlap calculation as members join/leave
3. **Multi-Hop Federation**: A ↔ B ↔ C (transitive federation chains)
4. **Reputation Leakage**: Allow limited cross-mesh reputation sharing

---

## Security Considerations

### Attack Vectors

#### Internal

1. **Malicious Bot**: Bot suggests bad pairings to isolate members
   - **Mitigation**: Bot behavior auditable (introduction history logged)
   - **Mitigation**: Members can reject introductions

2. **Sybil Attack**: Attacker creates many fake accounts
   - **Mitigation**: 2-vouch requirement from different clusters (INVARIANT 2)
   - **Mitigation**: Cross-cluster vouching is enforced, not optional

3. **Coordinated Infiltration**: Bad actors rubber-stamp confederates
   - **Attack**: Alice joins legitimately, then vouches for confederate Bob
   - **Attack**: Alice's peer Carol (same cluster) vouches for Bob
   - **Without defense**: Bob admitted with 2 same-cluster vouches → infiltration cluster forms
   - **Mitigation**: Cross-cluster vouching REQUIRED (INVARIANT 2)
   - **Mitigation**: Same-cluster vouches do NOT count toward admission
   - **Result**: Bob needs vouch from someone in a DIFFERENT cluster — independent verification

#### External

1. **Fake Group**: Attacker creates fake group to enumerate members
   - **Mitigation**: PSI-CA reveals only count, not identities
   - **Mitigation**: Ephemeral keys prevent replay attacks

2. **Traffic Analysis**: Adversary monitors Freenet traffic
   - **Mitigation**: Freenet Dark mode (anonymous routing)
   - **Mitigation**: Bloom Filters add plausible deniability

### Formal Verification

Future work: Formal verification of privacy guarantees using tools like:
- **ProVerif**: Protocol security verification
- **Tamarin Prover**: Security protocol analysis
- **F***: Functional correctness proofs

---

## Network Health Metrics: Distinct Validator Ratio (DVR)

### Problem Statement

**Given:**
- A trust network with N members
- Some members are Validators (3+ vouches from 3+ clusters)
- Need to measure network resilience against coordinated attacks

**Goal:**
- Define a metric that captures "independent verification depth"
- Provide actionable feedback for network improvement
- Ground the metric in graph theory (not arbitrary percentages)

### Distinct Validator Ratio (DVR)

**Definition**: The fraction of maximum possible Validators with non-overlapping voucher sets.

**Formula:**
```
DVR = Distinct_Validators / Max_Possible_Distinct_Validators

Where:
  Distinct_Validators = |{V : V is a Validator with voucher set disjoint from all other selected Validators}|
  Max_Possible = floor(N / 4)
```

**Why N/4?** Each distinct Validator requires approximately 4 members:
- 1 member: The Validator themselves
- 3 members: Unique vouchers (from 3 different clusters)
- Total: ~4 members per distinct Validator

### Distinct Validator Selection Algorithm

```
Algorithm: COUNT_DISTINCT_VALIDATORS(graph)

Input: Trust graph with members and vouch relationships
Output: Count of Validators with non-overlapping voucher sets

1. Identify all Validators (members with >= 3 vouches from >= 3 clusters)
2. Sort Validators by vouch count (descending)
3. Initialize:
   distinct = []
   used_vouchers = {}

4. For each validator V in sorted order:
   voucher_set = get_vouchers(V)
   
   if voucher_set ∩ used_vouchers = ∅:
       distinct.append(V)
       used_vouchers = used_vouchers ∪ voucher_set

5. Return |distinct|
```

**Complexity**: O(V log V + V × E) where V = Validators, E = vouch edges

### Three-Tier Health Classification

| DVR Range | Status | Color | Bot Behavior |
|-----------|--------|-------|--------------|
| 0% - 33% | Unhealthy | 🔴 | Actively suggest cross-cluster introductions |
| 33% - 66% | Developing | 🟡 | Suggest improvements opportunistically |
| 66% - 100% | Healthy | 🟢 | Maintenance mode |

**Why Thirds?**
- Cognitively simple (three states)
- Equal ranges (no arbitrary "optimal zone")
- Each state has clear action implications

**Activation Note (GAP-11):**
DVR calculation becomes meaningful once ≥2 clusters are detected. In bootstrap phase (single cluster), DVR is not displayed since cross-cluster requirements aren't yet enforced. Once Bridge Removal algorithm detects cluster formation, DVR tracking begins and the bot announces cross-cluster activation.

### Example Calculation

**20-member network with 4 clusters:**

```
Members: M1-M20
Validators: V1 (4 vouches), V2 (3 vouches), V3 (4 vouches), V4 (3 vouches)

V1 vouched by: {M5, M8, M12, M15}  (4 vouchers from 4 clusters)
V2 vouched by: {M6, M9, M13}       (3 vouchers from 3 clusters)
V3 vouched by: {M5, M10, M14, M16} (4 vouchers, but shares M5 with V1)
V4 vouched by: {M7, M11, M17}      (3 vouchers from 3 clusters)

Selection (greedy by vouch count):
1. V1: Add to distinct, used_vouchers = {M5, M8, M12, M15}
2. V3: {M5, M10, M14, M16} ∩ {M5, M8, M12, M15} = {M5} ≠ ∅ → SKIP
3. V2: {M6, M9, M13} ∩ {M5, M8, M12, M15} = ∅ → ADD
   used_vouchers = {M5, M6, M8, M9, M12, M13, M15}
4. V4: {M7, M11, M17} ∩ {...} = ∅ → ADD

Distinct_Validators = 3 (V1, V2, V4)
Max_Possible = 20 / 4 = 5

DVR = 3 / 5 = 60% → 🟡 Developing
```

### DVR vs Density Comparison

| Metric | Measures | Limitation |
|--------|----------|------------|
| Density (edges/max_edges) | Raw connectivity | Structure-blind — high density can mask clustered vulnerabilities |
| DVR | Independent verification depth | Directly tied to security model |

**Key Insight**: A network with 50% density could have 0% DVR if all Validators share vouchers. DVR captures what density misses: **redundancy of independent verification**.

### Security Properties

**High DVR implies:**
1. **No shared voucher vulnerabilities**: Compromising one voucher set doesn't cascade
2. **Distributed trust**: Multiple independent clusters have verified different members
3. **Infiltration resistance**: Attackers can't create "hub" Validators through shared vouches

**See**: `.beads/mesh-health-metric.bead` for full architectural decision

---

## References

### Graph Theory
- Cormen et al., "Introduction to Algorithms" (MST algorithms)
- Brandes, "A Faster Algorithm for Betweenness Centrality" (2001)
- Tarjan, "A Note on Finding the Bridges of a Graph" (Bridge Detection)

### Cryptography
- Pinkas et al., "Scalable Private Set Intersection" (PSI-CA protocols)
- Meadows, "A More Efficient Cryptographic Matchmaking Protocol" (1986)
- Boneh & Shoup, "A Graduate Course in Applied Cryptography" (DDH assumption)

### Privacy
- Dwork, "Differential Privacy" (privacy quantification)
- Narayanan & Shmatikov, "De-anonymization of Social Networks" (graph privacy)

---

**Document Status**: Living document, updated as algorithms are refined

**Last Updated**: 2026-02-01

**Next Review**: After Spike Week (Q1-Q5 answered)
