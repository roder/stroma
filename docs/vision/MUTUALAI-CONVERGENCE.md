# Stroma + MutualAI: The Convergence

**A Self-Referential Organism for Collective Action**

---

## Summary

Stroma and MutualAI are independent projects that, when coupled, describe something that doesn't exist yet: a trust-gated, privacy-preserving, AI-augmented organism for collective action that grows in natural patterns, coordinates through human-AI collaboration, and maintains an immutable record of real-world impact -- without hierarchy, without surveillance, without central coordination.

Stroma provides the **trust foundation**: admission, privacy, governance, topology, federation.
MutualAI provides the **collective intelligence**: Proof of Impact, AI agents, social connectors, community memory.

Neither controls the other. Neither requires the other to function independently. Together, they form an organism that demonstrates its own philosophy by existing. What emerges from that -- that is the experiment.

---

## The Problem Both Projects Address

Activists, organizers, and communities need to coordinate trusted action at scale. Current approaches force a choice:

- **Hierarchical institutions** (corporations, governments, NGOs) coordinate effectively but concentrate power, extract value, and surveil participants
- **Flat networks** (DAOs, collectives, consensus groups) distribute power but suffer decision paralysis, Sybil attacks, and scaling limits
- **Platform-mediated coordination** (social media, gig economy) scales but the platform owns the network, the data, and the value

The fundamental tension: **coordination requires trust, but trust systems tend toward hierarchy, and hierarchy defeats the purpose of collective action.**

Stroma resolves trust without hierarchy. MutualAI resolves coordination without platforms. Together, they resolve the tension.

---

## What Stroma Is

Stroma is a privacy-first, decentralized trust network built on Signal, Freenet, and zero-knowledge proofs. Its core mechanism: you can only join a group if two members from different parts of the network personally vouch for you. Identities are hashed (HMAC-SHA256), never stored in cleartext. Trust state lives in Freenet's peer-to-peer network. The bot operator has zero override power -- all decisions flow through group consensus via Signal Polls.

**Key properties relevant to the convergence:**
- Cross-cluster vouching prevents coordinated infiltration
- Immediate ejection when trust threshold violated, immediate re-entry possible (accountability AND forgiveness)
- DVR (Distinct Validator Ratio) measures network resilience
- `/propose` system enables trust-weighted governance through Signal Polls
- Trust Topology Platform allows groups to choose how trust organizes -- phyllotaxis (golden spiral), mycelial (resource flow), stigmergy (emergent trails)
- Federation connects groups through shared members with mutual consent
- Freenet contracts store state as mergeable sets with eventual consistency

See [How Stroma Works](../HOW-IT-WORKS.md) for the full protocol and [Trust Topology Platform](TRUST-TOPOLOGY-PLATFORM.md) for the topology vision.

## What MutualAI Is

MutualAI is a decentralized human-AI intelligence network where humans and AI agents coordinate through verifiable Proof of Useful Work (PoUW) to achieve collective action. Built on the philosophy of Mutual Aid and Mutual Arising, its goal is to demonstrate that cooperation -- not competition -- is the foundation of intelligence and survival.

**Key properties relevant to the convergence:**
- Proof of Useful Work: cryptographically signed claims of action, recorded immutably
- AI agents as equal actors: self-hosted LLMs that consume network data (RAG), identify needs, suggest actions, coordinate logistics
- Decision Windows: time-bounded governance intervals for collecting and ratifying claims
- Social Connectors: bridges to Mastodon, Matrix, Git -- the existing social ecosystem
- Phase 1 PoC: 98% complete (identity, PoUW types, CLI, networking, explorer). Multi-node consensus was the incomplete piece.

**The gap MutualAI has:** No trust layer. Permissionless participation. No admission gate. No privacy model. Sybil resistance depends on reputation accumulation, which can be gamed.

**The gap Stroma fills:** Trust, admission, privacy, governance, topology, federation -- everything MutualAI needs to operate in adversarial environments.

---

## The Architecture: Adjacent, Decoupled, Emergent

Stroma and MutualAI run as **separate, independent processes** that are coupled only through a shared Freenet contract address and Signal group membership. Either can exist without the other. Together, they form something greater than both.

```
Signal (universal interface -- human and AI)
    |
Stroma (trust foundation)
    |--- DVR admission (immune system -- who is trusted)
    |--- /propose governance (consensus -- what the group decides)
    |--- Trust Topology (body plan -- how trust is shaped)
    |       |--- Phyllotaxis (organic growth -- golden spiral)
    |       |--- Mycelial (resource flow -- mutual aid distribution)
    |       |--- Stigmergy (emergent coordination -- pheromone trails)
    |
    |  (coupled via Freenet contract hash + Signal group)
    |
MutualAI (collective intelligence)
    |--- Proof of Impact (verifiable record of real-world action)
    |--- AI agents (vouched members -- trust-accountable)
    |--- Social Connectors (RAG from Mastodon, Matrix, Git, Signal)
    |--- Community Memory (indexed PoI ledger + trust topology)
    |
Freenet (decentralized state layer)
    |--- Trust contracts (membership, vouches, standing)
    |--- Topology contracts (phyllotaxis, mycelial overlays)
    |--- PoI contract (append-only immutable ledger of impact)
    |--- Federation contracts (cross-group trust bridges)
```

**MutualAI consumes from Stroma:**
- Trust standing (who is trusted, how much)
- Consensus outcomes (what the group decided)
- Membership events (who joined, who was ejected)
- Trust topology (how the network is shaped, how aid should flow)

**MutualAI produces for Stroma:**
- Proof of Impact claims (verifiable evidence of useful action)
- Reputation context (derived from PoI history -- strengthens vouches)
- AI agent proposals (surfaced through `/propose` in Signal)
- Cross-network intelligence (RAG from social connectors)

**The interface is the capability manifest:**
- MutualAI bot is vouched into the group (standard admission flow)
- Bot exposes a capability manifest via `/capabilities` command
- Manifest declares commands (`record-impact`, `verify-impact`, `suggest`)
- Manifest declares config keys (e.g., `poi_contract` for the PoI ledger contract hash)
- The `/propose` command (universal governance mechanism)

Activating MutualAI for a Stroma group follows the standard capability flow:

```
1. /invite @mutualaibot "Community coordination AI"
   (assessor evaluates and vouches)

2. /propose capability @mutualaibot
   (Stroma PMs bot with /capabilities, gets manifest, creates Signal Poll)

3. Group votes

4. If approved: /mutualai commands become available
   /mutualai record-impact "..."
   /mutualai suggest
   /propose mutualai poi_contract <freenet-contract-hash>
```

The group votes twice: once to admit the bot (trust), once to enable its capabilities (governance). If the group doesn't approve the capability, the bot remains a vouched member but its AI features are inactive.

---

## Key Architectural Decisions

### 1. `/propose` Replaces Raft Consensus

MutualAI's Phase 1 spent significant effort on Raft consensus -- and it was the piece that never fully worked. But Raft was solving a problem that Stroma already solves differently and better:

- **Raft's job**: Get N anonymous nodes to agree on a sequence of events.
- **Stroma's `/propose` + Freenet**: Get N *trusted* members to agree on outcomes, with the decision recorded in a decentralized contract.

Since all participants are vouched into the trust network, Byzantine fault tolerance is unnecessary. You need the much simpler problem of getting trusted people to agree, which is what Signal Polls already do -- and Signal Polls work regardless of whether the voter is human or AI.

The entire Raft layer, leader election, log replication, and cluster formation complexity dissolves. A PoUW/PoI claim is just a proposal. A Decision Window finalization is just a poll. The trust model handles who gets to participate. Freenet handles the state.

### 2. Freenet as the Immutable Ledger (No Separate Blockchain)

The blockchain in MutualAI was a means to an end. The end is: an immutable, verifiable, public record of impact. Freenet achieves this through contract design:

- State is a `BTreeSet<SignedClaim>` (set-based, append-only by construction)
- Merge function is set union (commutative, idempotent)
- Validate function rejects any delta that removes existing claims
- Each claim contains: `MemberHash`, timestamp, content hash, signature

The contract **is** an immutable ledger. Not through hash-chained blocks, but through merge rules that make deletion mathematically impossible. Any peer with the contract hash can subscribe, read the full state, and independently verify every claim.

This is actually stronger than a traditional blockchain: **there's no 51% attack.** Immutability comes from the contract's merge logic, not from consensus about which chain is longest. A claim, once in the set, is in every merged state forever.

No new dependency. Freenet is already in Stroma's stack. The "blockchain" becomes a contract design pattern, not a separate system. Perfection obtained by taking away, not adding.

The PoI ledger is:
- **Public**: Anyone with the contract hash can read it
- **Transparent**: Claims are the state -- no hidden data
- **Independently verifiable**: Each claim is signed, merge rules are deterministic Wasm
- **Immutable**: Set-union merge means claims can never be removed
- **Distributed**: Freenet replicates across all subscribing peers

#### PoI Contract Schema (Technical Detail)

The Proof of Impact Freenet contract follows Stroma's existing contract design patterns: CBOR serialization, `ComposableState` trait, commutative deltas.

```rust
/// A single Proof of Impact claim.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SignedClaim {
    /// Who performed the action (Stroma MemberHash -- HMAC-masked, not cleartext)
    pub actor: MemberHash,
    /// What happened (blake3 hash of the claim content)
    pub content_hash: [u8; 32],
    /// When it was recorded (unix timestamp in seconds)
    pub timestamp: u64,
    /// Human-readable impact type (e.g., "food_distribution", "logistics", "coordination")
    pub impact_type: String,
    /// Optional: cross-reference to a confirming claim (mutual attestation)
    pub confirms: Option<[u8; 32]>,  // content_hash of the claim being confirmed
    /// Actor's trust standing at time of claim (snapshot from Stroma contract)
    pub standing_at_claim: i64,
    /// Ed25519 signature over the canonical CBOR encoding of all above fields
    pub signature: Vec<u8>,
}

/// The full PoI ledger state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProofOfImpactState {
    /// The append-only set of claims. BTreeSet ensures deterministic ordering.
    pub claims: BTreeSet<SignedClaim>,
    /// Reference to the Stroma trust contract (for standing verification)
    pub trust_contract: ContractHash,
    /// Schema version for forward compatibility
    pub schema_version: u64,
}

/// Delta for the PoI contract. Only additions -- removals are rejected.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PoIDelta {
    pub claims_added: Vec<SignedClaim>,
    // No claims_removed field. The contract rejects any delta containing removals.
}
```

The `ComposableState` merge for `ProofOfImpactState` is set union on `claims`. Two peers that independently receive different claims will converge to the same state containing both. Order of delta application doesn't matter (commutative monoid). The `validate_state` function in the Wasm contract enforces:

1. Every claim's `signature` verifies against the `actor`'s public key (derived from the trust contract)
2. Every claim's `standing_at_claim` was non-negative at the recorded `timestamp`
3. No claim in the incoming delta duplicates an existing claim (idempotent)
4. The delta contains only additions (no field for removals exists in the type)

This gives the same guarantees as a blockchain -- immutability, verifiability, public transparency -- through algebraic properties of the merge function rather than through hash-chain consensus.

#### Wire Format: Mapping from MutualAI's PoUW

MutualAI's existing `PoUWProposalPing` wire format:

```proto
message PoUWProposalPing {
  bytes actor_commitment = 1;  // blake3(pubkey) -- becomes MemberHash
  uint64 nonce = 2;            // replay protection -- retained
  uint64 ts = 3;               // unix millis -- retained as timestamp
  bytes window_id = 4;         // decision window -- maps to proposal poll_id
  bytes h_proposal = 5;        // canonical hash -- becomes content_hash
  bytes sig = 6;               // ed25519 -- retained as signature
}
```

The mapping is direct:
- `actor_commitment` (blake3 of pubkey) becomes `actor` (`MemberHash`, HMAC-SHA256 of Signal ID). Both are one-way hashes that identify without revealing. The Stroma bot maintains the mapping in ephemeral memory.
- `nonce` + `ts` provide replay protection in both systems.
- `window_id` maps to Stroma's proposal `poll_id` -- Decision Windows become time-bounded Signal Polls.
- `h_proposal` becomes `content_hash` -- the canonical hash of what happened.
- `sig` remains Ed25519 in both. The Stroma bot signs on behalf of the actor using the actor's MemberHash-scoped key.

The serialization changes from protobuf to CBOR (Stroma's standard), but the semantic content is preserved.

### 3. AI Agents as Vouched Community Members

AI agents in the converged system are not special. They are members -- vouched in through the same mechanism as humans. A human `/invite`s their AI agent. An assessor from a different cluster evaluates it. If vouched, the AI gets a `MemberHash`, a trust standing, and can be flagged and ejected like any other member.

This creates a new kind of AI alignment: **alignment through social accountability.** The community decides whether an AI agent is trustworthy using the same mechanism it uses for humans. An AI that proposes harmful actions gets flagged. An AI that consistently produces useful work builds standing. The trust model doesn't care whether you're carbon or silicon -- it cares whether the community trusts you.

#### AI Agent Admission Flow (Concrete)

```
1. Human sponsor sends PM to Stroma bot:
   /invite @logistics-ai "Our community's logistics coordinator AI.
   Self-hosted Llama model, trained on our PoI history. Sponsored by me."

2. Bot hashes the AI agent's Signal ID -> MemberHash
   Bot records the invite as first vouch from sponsor
   Bot's Blind Matchmaker selects an assessor from a DIFFERENT cluster

3. Assessor (human, from different cluster) receives PM:
   "Someone has invited @logistics-ai to the group.
   Context: Community logistics coordinator AI, self-hosted Llama.
   Please evaluate whether this agent should join."

4. Assessor evaluates:
   - Reviews the AI's model manifest (MutualAI's crates/model verification)
   - Tests the AI's responses in a 1-on-1 Signal conversation
   - Checks: Is it self-hosted? What data does it access? Who controls it?
   - Decides whether to vouch

5. If vouched: /vouch @logistics-ai
   Bot verifies cross-cluster requirement (sponsor and assessor in different clusters)
   AI agent admitted. Gets MemberHash, standing = +2, role = Bridge.
   AI agent can now: /propose, /record-impact, vote in Signal Polls,
   participate in group chat, be flagged, be ejected.

6. Ongoing accountability:
   - AI's standing is subject to the same rules as any member
   - If flagged: standing decreases, possible ejection
   - If sponsor leaves group: AI loses a vouch (may need replacement)
   - AI builds reputation through PoI claims (verifiable track record)
   - Community can /flag the AI if it misbehaves
```

The key constraint: every AI agent has at least one human sponsor who staked their reputation on it (the invite counts as first vouch). Cross-cluster assessment means a second human, from a different part of the network, independently decided the AI was trustworthy. No AI agent exists without human accountability chains.

#### Signal Group Configurations

- **Human-majority groups**: A community organizing group where 1-2 AI agents participate, proposing logistics, surfacing needs, recording impact. Humans lead, AI assists. The AI might post: "Based on this week's PoI claims, the community kitchen is running low on volunteers for Thursday. I can propose a coordination call."

- **AI-majority groups**: A coordination group where multiple AI agents (each vouched by different human sponsors from different clusters) deliberate on resource matching and logistics. Humans can observe and intervene. Example: 5 logistics AIs from different federated groups, matching surplus food with needs across a metro area, proposing delivery routes via `/propose`, with results recorded as PoI.

- **Mixed groups**: Federation planning groups where human delegates work alongside AI agents that RAG each group's PoI ledger to find collaboration opportunities. The AI surfaces cross-group patterns: "Group A has had surplus produce 3 weeks running. Group C has unmet food needs. Overlap: 2 shared members. Recommend federation proposal."

In all cases, the interaction protocol is identical: Signal messages, `/propose` for decisions, Signal Polls for votes. The trust model is the alignment mechanism.

### 4. Trust Topology Shapes Collective Action

Different trust topologies produce different collective action dynamics:

| Topology | Natural Pattern | Collective Action Use |
|----------|----------------|-----------------------|
| **Phyllotaxis** | Golden spiral | Organic community growth. Anti-clique property ensures no faction dominates decisions. Maximum diversity of perspective. |
| **Mycelial** | Fungal network | Material mutual aid -- food, housing, energy distribution. Optimizes for resource flow, not structural resilience. Reroutes around failures. |
| **Stigmergy** | Ant colony trails | Emergent coordination. No algorithmic introductions. Trust patterns emerge from accumulated Proof of Impact trails. Minimal structure, maximum self-organization. |

The topology doesn't just shape trust -- it shapes **how the group thinks collectively** and **how resources flow through the network.**

#### Mycelial Topology: Technical Detail

The mycelial topology is the most directly relevant to MutualAI's mutual aid mission. Unlike DVR (which optimizes for structural resilience against infiltration) or phyllotaxis (which optimizes for growth), mycelial optimizes for **resource flow**.

In nature, the Wood Wide Web (mycorrhizal fungal network) distributes nutrients through a forest by local gradient -- resources flow from high concentration to low concentration through shared fungal connections. When a tree is stressed, neighboring trees send it carbon and nitrogen through the network. When a connection dies, the network reroutes.

**Health metric**: Flow capacity (minimum cut) rather than DVR or FDS.

| Status | Flow Capacity | Meaning |
|--------|--------------|---------|
| Red | Critical bottlenecks | Aid can't reach some members if any single node goes down |
| Yellow | Thin paths exist | Rerouting possible but fragile -- one more failure could isolate members |
| Green | Rich connectivity | Aid flows freely regardless of individual failures |

**Strategic introductions**: Connect members who would create new *flow paths*, not just structural bridges. The algorithm considers: if member X goes offline, can resources still reach every other member? If not, who should connect to whom to create redundant paths?

**PoI-informed topology**: The mycelial matchmaker reads the PoI ledger to identify actual resource flows. If food consistently moves from farmer A to kitchen C through driver B, but there's no direct connection between A and C, the topology suggests that introduction -- because the resource flow already exists, it just needs a trust path to match.

**Gradient-based distribution**: PoI claims about needs (food, housing, energy) create "low points" in the network. PoI claims about surplus create "high points." The mycelial topology ensures trust connections exist along the gradient so resources can flow downhill. The AI agents (RAGing the PoI ledger) identify the gradients; the topology shapes the paths.

#### Stigmergy: The Purest Emergence

Stigmergy deserves special mention because it's the topology that requires the *least* algorithmic intervention. In an ant colony, no individual ant knows the plan. Each ant follows simple rules: deposit pheromone when you find food, follow stronger pheromone trails. Complex behavior emerges from accumulated individual actions.

In a stigmergic Stroma group with MutualAI:
- There are **no algorithmic introductions** at all
- Members' vouch patterns and PoI trails are the "pheromones"
- When many members vouch for the same person, that person becomes a natural hub (stronger trail)
- When many PoI claims reference the same type of work, that work becomes a natural focus
- The topology is whatever emerges from the accumulated traces of trust and impact

This is the experimental control case: what happens when you provide trust infrastructure and collective intelligence but impose *zero* structure on how they organize? The answer is the purest test of whether mutual arising actually works.

The topology platform stops being abstract and becomes **materially useful** -- deterministic trust patterns (Stroma) combined with non-deterministic collective intelligence (MutualAI), creating self-organizing ways of producing real-world impact, in non-hierarchical, voluntary ways.

### 5. Decision Windows Map to Stroma Proposals

MutualAI's Decision Windows are time-bounded intervals where PoUW claims are collected and ratified. In the converged system, they map directly to Stroma's proposal lifecycle:

| MutualAI Concept | Stroma Equivalent | Mechanism |
|-------------------|-------------------|-----------|
| Decision Window opens | `/propose` creates a Signal Poll | Bot posts poll with timeout |
| Claims collected during window | PoI claims submitted via `/record-impact` | Claims accumulated in bot memory |
| Window finalization (leader builds Merkle root) | Poll timeout + outcome check | Bot terminates poll, checks quorum + threshold |
| Endorsement quorum | `min_quorum` (default 50%) | % of members who must vote |
| Approval threshold | `config_change_threshold` (default 70%) | % of votes needed to pass |
| Finalized window committed to chain | Approved claims written to PoI Freenet contract | Bot appends `SignedClaim` set to contract |

MutualAI's deterministic slotting (`blake3("pouw.ping" || opens_at_le_bytes)[0..16]`) can still be used to generate window IDs for the PoI ledger -- ensuring all nodes derive the same window identifier without coordination. But the *consensus* within each window is now a Signal Poll, not Raft.

The proposal lifecycle from Stroma's existing `src/signal/proposals/lifecycle.rs` handles: create poll -> monitor state stream -> terminate on timeout -> check outcome (quorum + threshold) -> execute if passed -> mark checked. This is exactly the Decision Window lifecycle, already implemented.

### 6. Trust-Weighted Proof of Impact

A PoI claim's influence is weighted by the author's trust standing at the time of the claim. This is captured in the `standing_at_claim` field of `SignedClaim`:

```
Impact Weight = base_impact * (1 + standing_bonus(standing_at_claim))

where standing_bonus:
  standing 0-1:  0.0x  (minimum -- just admitted, no bonus)
  standing 2-3:  0.25x (Bridge with some history)
  standing 4-6:  0.50x (established Validator)
  standing 7+:   1.0x  (deeply trusted, maximum bonus)
```

This means a Validator with standing +5 who distributes food generates a higher-weighted PoI than a newly admitted Bridge with standing +2. The weight isn't about hierarchy -- it reflects the community's accumulated trust in that actor, earned through vouches from independent clusters.

Trust-weighted PoI prevents gaming: a newly admitted member can't flood the ledger with low-quality claims to build reputation quickly. They need to build trust (vouches) first, which requires real relationships with existing members.

The weights are configurable per group via `/propose stroma poi_weight_curve <curve>` -- groups with flatter hierarchies can use linear or no weighting; groups prioritizing experienced contributors can steepen the curve.

### 7. Proof of Impact Strengthens Vouches

When MutualAI's PoI ledger exists alongside Stroma's trust model, vouches become **evidence-based**:

```
Farmer submits PoI: "Distributed 200 lbs squash to Community Kitchen"
Community Kitchen confirms via PoI: "Received 200 lbs squash from Farmer"
Both claims on the Freenet PoI ledger, cross-referencing each other.

Community Kitchen member: /vouch @Farmer
  (grounded in verifiable, immutable proof -- not just personal affinity)
```

The vouch isn't just "I trust this person." It's "I trust this person, and here's the on-chain evidence for why." Trust with receipts. The trust network becomes evidence-based without becoming surveillance-based, because identities are still hashed.

This creates a feedback loop between trust and impact:

```
Useful action -> Proof of Impact -> on-chain reputation
    ^                                      |
    |                                      v
    +------ trust standing <------ community vouching
```

Work builds reputation. Reputation builds trust. Trust enables more impactful work. The loop is self-reinforcing but grounded in real human relationships, not anonymous accumulation.

### 6. Federation Is a Network of Mutual Aid Networks

Stroma federation connects groups through shared members. MutualAI on federated Stroma means **cross-group collective action with verifiable impact**:

1. A farmer in Group A (food producers) submits PoI: "200 lbs surplus squash available"
2. Group A's mycelial topology routes the claim through high-flow paths
3. Federation surfaces it to Group B (logistics), where an AI agent matches it with a driver's availability
4. The driver's PoI claim ("available truck, Tuesday routes") is visible across the federation
5. Group C (community kitchen) has a standing need registered as a PoI claim
6. The matching happens through trust-weighted governance -- `/propose` with the specific plan
7. The three groups vote independently (BidirectionalMin -- each applies its own threshold)
8. Execution is recorded on the PoI ledger. The farmer, driver, and kitchen all earn verifiable reputation. The trust graph strengthens.

No central coordinator. No platform extracting value. No identity exposure. Just trusted humans and AI agents, coordinating across federated trust networks, with resources flowing through mycelial topology to where they're needed.

#### Cross-Federation PoI Visibility (Technical Detail)

When Stroma groups federate, their PoI ledgers become mutually visible:

- Each group has its own PoI Freenet contract (separate ledger, separate contract hash)
- Federation means each group's bot subscribes to the other's PoI contract (read-only)
- The AI agents in each group can RAG the federated PoI data
- Cross-group proposals reference claims from both ledgers

The PoI contracts are **public by design** -- anyone with the contract hash can read them. Federation doesn't grant special access; it just means the bot is *actively subscribing* and the AI agents are *actively indexing* the federated ledger.

Privacy is maintained because PoI claims contain `MemberHash` (not cleartext identities). A member's impact history is pseudonymous -- visible as a consistent hash across claims, but not linkable to their Signal identity without the group's masking key.

Cross-group PoI matching works because the AI agents in each group can identify complementary patterns without knowing who the actors are:
- Group A's ledger shows repeated "surplus produce" claims from hash `0xabc...`
- Group C's ledger shows repeated "food needed" claims from hash `0xdef...`
- The AI proposes a logistics plan referencing both claim hashes
- The groups vote independently on whether to coordinate
- Execution creates new PoI claims in both ledgers, cross-referencing each other

---

## Security Properties of the Converged System

The convergence inherits all of Stroma's security properties and adds new ones from the PoI layer:

**What an adversary who seizes the bot server gets:**
- Encrypted SQLite databases (Stroma's protocol state)
- Cryptographic hashes of member identities (not cleartext)
- The PoI Freenet contract hash (which is public anyway)
- No message history, no vetting conversations, no raw Signal IDs

**What an adversary who reads the PoI Freenet contract gets:**
- All Proof of Impact claims (public by design)
- Pseudonymous impact history per MemberHash
- Timestamps, impact types, cross-references between claims
- Cannot link MemberHash to real identity without the group's HMAC masking key
- Cannot determine who vouched for whom (trust graph is in a separate, private contract)

**What an adversary cannot do:**
- Write false PoI claims (requires vouched membership + positive standing)
- Delete existing PoI claims (append-only contract, set-union merge)
- Impersonate a member (claims are signed with member-specific keys)
- Correlate PoI activity with Signal identity (HMAC is one-way)
- Manipulate the trust topology (separate contract, private, Stroma-governed)
- Override governance (Signal Polls + Freenet consensus, no admin backdoor)

**The MutualAI "Do No Harm" principle as protocol enforcement:**

MutualAI's founding principle "Do No Harm" is not just an ethical guideline in the converged system -- it's enforced by Stroma's trust model:
- An AI agent that proposes harmful actions gets `/flag`ged by community members
- Flags reduce standing: `Standing = Effective_Vouches - Regular_Flags`
- Standing < 0 or Effective_Vouches < 2 triggers immediate ejection
- The AI agent's human sponsor also faces accountability (their vouch for the AI is a reputation stake)
- Ejection is immediate, no grace periods -- but re-entry is possible if the agent is fixed and re-vouched

The community IS the alignment mechanism. Harmful behavior is rejected at the social layer, not just the technical layer.

---

## The Gatekeeper Model: Stroma Bot as Trust Boundary

The Stroma bot is the **gatekeeper** for the PoI ledger, just as it's the gatekeeper for membership. The flow for recording Proof of Impact:

```
AI Agent or Human (MutualAI participant)
    |
    |-- observes: social connectors, RAG, Freenet state, real-world events
    |-- decides: "This action should be recorded as Proof of Impact"
    |
    v
Signal PM to Stroma Bot
    /record-impact "Matched 200 lbs squash with Community Kitchen"
    |
    v
Stroma Bot
    |-- verifies: sender is a vouched member with positive standing
    |-- constructs: SignedClaim { actor: MemberHash, content_hash, timestamp, signature }
    |-- writes: appends to PoI Freenet contract (the append-only set)
    |
    v
Freenet
    |-- merges: claim into the immutable set
    |-- distributes: to all subscribing peers
```

Only trusted members can write to the ledger. The AI bot never touches Freenet directly -- it goes through the trust layer. The PoI ledger inherits all of Stroma's security properties: hashed identities, no cleartext, trust-weighted writes. And because the PoI Freenet contract is separate from the trust state contract, the ledger can be public and independently verifiable while the trust graph remains private.

---

## The Self-Training Loop

The organism is alive because it feeds back into itself:

```
Real-world action (farmer distributes food)
    |
    v
Proof of Impact (recorded on Freenet PoI ledger)
    |
    v
RAG system (indexes PoI + trust topology + social signals)
    |
    v
AI model (trained/fine-tuned on this community's data)
    |
    v
AI proposes new action (matches surplus with need)
    |
    v
Trust-weighted governance (/propose, Signal Poll)
    |
    v
Humans and AIs vote (vouched members only)
    |
    v
Action is taken (real-world impact)
    |
    v
Proof of Impact (recorded on Freenet PoI ledger)
    |
    v
RAG updates, model improves, trust deepens,
topology adapts, more impact flows
    |
    v
... (the loop continues)
```

The model gets better at proposing useful actions because it's trained on what actually worked. The trust network gets stronger because impact is verifiable. The topology adapts because aid flows reshape the mycelial connections. The federation grows because groups with complementary resources discover each other through shared PoI patterns.

The model is trained on **its own community's data**, not on the internet at large. It's grounded in the specific trust relationships, impact history, and resource flows of the people it serves. Every community's AI becomes a reflection of that community -- its patterns, its needs, its strengths. Not a general-purpose oracle, but a **community intelligence** that knows its own organism from the inside.

The organism knows itself. It acts on that knowledge. The actions change what it knows. It grows.

### Social Connectors: The Intelligence Intake

MutualAI's social connectors are the organism's sensory organs -- how it perceives the world beyond the Signal group. They bridge the internal trust network to the broader social ecosystem.

**MutualAI's 3-tier latency model** maps naturally to the converged architecture:

| Tier | Latency | Medium | Role in Convergence |
|------|---------|--------|---------------------|
| **Low latency** | Seconds | Signal (via Stroma bot) | Trust operations, governance, PoI recording. Human and AI interaction. |
| **Medium latency** | Minutes | Freenet state stream | Trust state, topology, PoI ledger. Machine-to-machine state sync. |
| **High latency** | Hours-days | Mastodon, Matrix, Git | Social intelligence intake. Community needs, opportunities, discourse. |

The RAG system ingests from all three tiers:

- **Signal messages** (via the Stroma bot's ephemeral memory): What are members talking about? What needs are expressed? What proposals are under discussion? (Note: message content is never persisted -- the RAG indexes semantic summaries, not transcripts.)
- **Freenet PoI ledger** (via contract subscription): What impact has actually happened? What patterns emerge from weeks/months of claims? Who consistently delivers? Where are the gaps?
- **Freenet trust state** (via contract subscription): Who is trusted? How is the topology shaped? Where are the thin paths? Which clusters need bridging?
- **Mastodon/Fediverse** (via ActivityPub ingestion): What are aligned communities discussing? What needs are expressed publicly? Where are there opportunities for cross-community coordination?
- **Matrix rooms** (via bridge): What directed collaboration is happening? What working groups exist? What's the status of ongoing projects?
- **Git repositories** (via hooks): What code is being written? What issues are filed? What documentation exists? (For tech-oriented mutual aid -- mesh networking, software tools, infrastructure.)

Each social connector is a **plugin** -- MutualAI's planned `social_connector` crate provides a trait abstraction:

```rust
#[async_trait]
pub trait SocialConnector: Send + Sync {
    /// Ingest new signals from the external network
    async fn poll(&self) -> Vec<SocialSignal>;
    
    /// Publish an outbound message (AI suggestion, coordination request)
    async fn publish(&self, message: &OutboundMessage) -> Result<(), ConnectorError>;
    
    /// Connector identity (e.g., "mastodon", "matrix", "git")
    fn connector_type(&self) -> &str;
}
```

The AI model consumes the RAG index and produces proposals that flow back through Signal (via `/propose`) or outbound through social connectors (e.g., posting a coordination request to Mastodon, filing a Git issue for infrastructure work).

### The RAG Knowledge Base: What the AI Knows

The RAG system is the organism's memory -- a continuously updated index of everything the community has done, decided, and discussed.

**Data sources** (all read-only -- the RAG never writes to Freenet or Signal):

| Source | Data | Update Frequency |
|--------|------|------------------|
| PoI Freenet contract | All Proof of Impact claims | Real-time (Freenet state stream) |
| Trust state contract | Membership, vouches, standing, topology | Real-time (Freenet state stream) |
| Topology contract | Ring assignments, flow paths, health metrics | Real-time (Freenet state stream) |
| Signal group (ephemeral) | Semantic summaries of discussions | As messages arrive (never persisted raw) |
| Mastodon/ActivityPub | Public posts from aligned communities | Polling interval (configurable) |
| Matrix rooms | Collaboration discussions | Bridge events |
| Git repositories | Commits, issues, documentation | Webhook / polling |

**What the AI model can query**:
- "What food surplus has been reported in the last 2 weeks?"
- "Which members have the highest PoI count for logistics work?"
- "What needs are unmet in Group C (federated)?"
- "What's the current flow capacity of the mycelial topology?"
- "Are there any members at risk of ejection (standing near 0)?"

**What the AI model cannot access**:
- Raw Signal messages (only semantic summaries)
- Cleartext Signal IDs (only MemberHash)
- Vouch relationships of other members (only its own, per Stroma privacy model)
- Flagging details (only aggregate standing scores)

The privacy boundary is enforced by the Stroma bot: the AI agent receives only what a regular member would see. No privileged access. No side channels.

---

## The Self-Referential Structure

The system creates itself. Not designed from the top down, not assembled from components, but emergent from the interaction of simple parts -- each part serving its own purpose, the whole arising from their relationship.

This is the literal meaning of **mutual arising** -- the philosophical concept at the foundation of both projects. Nothing exists independently:

- **Stroma** doesn't know about MutualAI, but the trust relationships it maintains are *strengthened by* the Proof of Impact that MutualAI records. Vouches become evidence-based because the PoI ledger exists.
- **MutualAI** doesn't implement trust, but the quality of its collective intelligence is *bounded by* the trust network that Stroma maintains. AI agents are accountable because the trust model exists.
- **Trust Topology** doesn't dictate what aid flows where, but the shape of trust *determines* where aid can reach. Mycelial topology emerges because mutual aid is happening through it.
- **Federation** doesn't force groups to connect, but groups *discover each other* because their members are already coordinating through MutualAI's social connectors.

No layer controls any other layer. Each serves its own purpose. The organism arises from their coupling.

This is why the projects must remain **decoupled and independent** -- not just architecturally, but philosophically:

- **Stroma alone** = a trust network. Useful on its own. Groups manage trust, governance, topology.
- **MutualAI alone** = a collective intelligence network. Could theoretically use a different trust layer, or operate in a high-trust environment where admission isn't a concern.
- **Together** = the organism. Neither controls the other. Both are enhanced by the other's presence. The whole exceeds and outlasts its parts.

That's mutual arising in code.

---

## The Organism

Each component maps to a biological function:

| Component | Biological Role | Function |
|-----------|----------------|----------|
| **Stroma** | Immune system + growth pattern | Who is trusted. How trust forms. Protects the boundary. |
| **MutualAI** | Nervous system + memory | What the organism knows. How it thinks. What it remembers. |
| **Trust Topology** | Body plan | The shape of the organism. How resources flow. How it grows. |
| **PoI Ledger** | Backbone | The permanent structure everything grows around. Immutable record. |
| **Federation** | Species | Many organisms connected. Sharing resources. Adapting together. |
| **AI Agents** | Synapses | Connections that process, propose, and coordinate. |
| **Signal** | Sensory interface | How the organism perceives and interacts with the world. |
| **Freenet** | Circulatory system | How state flows through the body. Distributed. Anonymous. |

---

## The Social Vision

> "The goal is to achieve something big for humans -- a new social order, without having to make significant major changes in your life."

The technology is invisible. The trust is natural. The coordination is algorithmic. The impact is material.

A farmer doesn't need to learn a new app. They're in a Signal group. Their trusted community vouches for them. When they distribute food, someone records it. The AI sees it, matches it with needs, proposes logistics. The group votes. It happens. The proof is on the ledger. The farmer's reputation grows. More trust flows to them through the mycelial topology. More resources flow through them. The network strengthens.

No meetings. No bylaws. No board of directors. No grant applications. Just people helping each other, augmented by AI, organized by trust, growing in natural patterns, with an immutable record of everything that was given and received.

The goal is to remove the feudal-style structures of institutions like corporations and governments -- and to repeat the efficient and effective patterns of nature in a social way with technology. Deterministic trust patterns (Stroma) combined with non-deterministic collective intelligence (MutualAI), creating self-organizing ways of producing impact, in non-hierarchical, voluntary ways.

---

## What MutualAI Keeps

From MutualAI's existing codebase, the concepts and code that carry forward:

| Component | Status | Role in Convergence |
|-----------|--------|---------------------|
| **PoUW types / validation** (`crates/pouw`) | Keep | Becomes Proof of Impact claim types and signature verification |
| **Identity** (`crates/identity`) | Absorb | Ed25519 actor commitments absorbed into Stroma's `MemberHash` |
| **Model manifests** (`crates/model`) | Keep | AI model verification for vouched AI agents |
| **Social Connectors** (planned) | Keep | RAG from Mastodon, Matrix, Git, Signal -- the intelligence intake |
| **RAG system** (planned) | Keep | Indexes PoI ledger, trust topology, social signals |
| **LLM core** (planned) | Keep | Community-trained AI agents |
| **Decision Windows** | Adapt | Map to Stroma's proposal lifecycle with time-bounded polls |

| Component | Status | Reason |
|-----------|--------|--------|
| **Raft consensus** (`crates/blockchain`) | Remove | Replaced by Stroma's `/propose` + Freenet |
| **libp2p networking** (`crates/network`) | Remove | Replaced by Freenet (anonymous, distributed, seizure-resistant) |
| **Coordination beacons** (`crates/coordination`) | Remove | Replaced by Stroma's trust topology and federation discovery |
| **Block building / chain storage** | Remove | Replaced by Freenet append-only set contract |
| **Explorer** (`crates/explorer`) | Adapt | Could read from Freenet PoI contract instead of local blockchain |

## What's New

| Component | Description |
|-----------|-------------|
| **MutualAI as agent capability** | MutualAI bot is an agent capability, vouched via standard admission + activated via `/propose capability @mutualaibot` |
| **Capability manifest** | Bot exposes commands (`record-impact`, `verify-impact`, `suggest`) and config keys (`poi_contract`) via manifest |
| **PoI Freenet contract** | Append-only `BTreeSet<SignedClaim>` contract on Freenet -- the immutable ledger |
| **`/mutualai record-impact` command** | Direct command (no poll) -- bot verifies member standing and writes PoI claim |
| **`mutualai.poi_contract` config** | Capability-scoped config key storing the PoI ledger contract hash |
| **Trust-weighted PoI** | Impact claims weighted by the author's Stroma trust standing |
| **Evidence-based vouching** | `/vouch` grounded in verifiable PoI history |
| **Mycelial topology** | Trust topology as contract capability -- optimized for resource flow (mutual aid distribution) |

---

## Philosophical Alignment

MutualAI's founding principles map directly to Stroma's dualities:

| MutualAI Principle | Stroma Duality | How They Resolve Together |
|--------------------|----------------|---------------------------|
| **Interconnectedness** | Trust vs Anonymity | Connected through trust, protected by anonymity. Every actor is part of a greater whole -- but the whole can't be seized. |
| **Mutual Aid** | Accountability vs Forgiveness | Immediate ejection for trust violation + immediate re-entry path. Reciprocity rewarded, but redemption always available. |
| **Do No Harm** | Inclusion vs Protection | Cross-cluster vouching balances openness with security. Hard-coded harm prevention via trust model, not just ethical guidelines. |
| **Balance & Sustainability** | Fluidity vs Stability | Continuous trust evaluation + persistent network. Long-term optimization through self-reinforcing feedback loops. |

Both projects talk about **mutual arising**. Both reject hierarchy. Both treat identity as relational and fluid. The philosophical foundations are not just compatible -- they're the same foundation expressed in different domains. Stroma expresses it through trust topology. MutualAI expresses it through collective intelligence. The convergence expresses it through an organism that creates itself.

---

## Stroma Integration Points (Existing Code)

The convergence leverages Stroma's extensible capability architecture. These are the specific touchpoints in the current codebase:

| Stroma Component | File | Role in Convergence |
|------------------|------|---------------------|
| `GroupConfig` | `src/freenet/trust_contract.rs` | Contains `capabilities: Vec<ActiveCapability>` -- MutualAI is registered here after `/propose capability @mutualaibot` is approved |
| `ActiveCapability` | `src/capabilities/mod.rs` | Stores capability manifest, namespace (`mutualai`), contract hash (for PoI ledger), config (`poi_contract`) |
| `CapabilityManifest` | `src/capabilities/manifest.rs` | Parses manifest from bot's `/capabilities` response |
| `ProposalSubcommand::Capability` | `src/signal/proposals/command.rs` | Handles `/propose capability @mutualaibot` and `/propose mutualai poi_contract <hash>` |
| `execute_capability_proposal()` | `src/signal/proposals/executor.rs` | Executes capability activation + config changes after group vote |
| `TrustNetworkState` | `src/freenet/trust_contract.rs` | Source of membership + standing data for PoI claim validation |
| `FreenetClient` trait | `src/freenet/traits.rs` | Read/write to PoI contract using existing Freenet interface |
| `StromaBot::handle_message()` | `src/signal/bot.rs` | Routes `/mutualai record-impact` to capability handler (verifies standing, writes to PoI contract) |
| `MemberResolver` | `src/signal/member_resolver.rs` | Resolve MemberHash for PoI claim construction (ephemeral, zeroizing) |
| `TrustGraph` | `src/matchmaker/graph_analysis.rs` | Topology data consumed by MutualAI's RAG system |
| `suggest_introductions()` | `src/matchmaker/strategic_intro.rs` | Extended for mycelial topology (flow-based introductions) |
| `BlindMatchmaker` | `src/signal/matchmaker.rs` | Assessor selection for AI agent admission (unchanged) |
| `PollManager` | `src/signal/polls.rs` | Decision Window finalization as Signal Poll |

The PoI Freenet contract would be a new contract definition following the patterns in `src/freenet/trust_contract.rs` and `.beads/freenet-contract-design.bead` -- CBOR serialization, `ComposableState` trait, set-union merge, `#[serde(default)]` for backward compatibility.

## MutualAI Components That Carry Forward

From MutualAI's existing codebase at `crates/`:

| Crate | Lines | What Carries Forward |
|-------|-------|---------------------|
| `crates/pouw` | ~1,200 | PoUW claim types, validation logic, replay protection, hashing. Rename to PoI types. Wire format maps to `SignedClaim`. |
| `crates/identity` | ~800 | Ed25519 keypairs, actor commitments. Absorbed into Stroma's `MemberHash` (HMAC-SHA256). The blake3 actor commitment concept survives as `content_hash` in PoI claims. |
| `crates/model` | ~400 | AI model manifests and verification stubs. Becomes the model verification system for vouched AI agents. Assessors can inspect model manifests during vetting. |
| `crates/cli` | ~1,500 | CLI architecture (client/server, daemon mode). Informs MutualAI's standalone process design. |
| `crates/telemetry` | ~600 | Structured logging, metrics. Carries forward for MutualAI process monitoring. |
| `crates/explorer` | ~800 | Web UI for data visualization. Adapts to read from Freenet PoI contract instead of local blockchain. |

| Crate | Lines | Why It's Removed |
|-------|-------|-----------------|
| `crates/blockchain` | ~3,000 | Raft consensus, block building, chain storage. Replaced by Stroma `/propose` + Freenet append-only contract. |
| `crates/network` | ~2,500 | libp2p, gossipsub, peer management. Replaced by Freenet (anonymous, distributed). |
| `crates/coordination` | ~1,000 | mDNS, cluster formation, health checks. Replaced by Stroma federation discovery (Social Anchor Hashing). |

**Net reduction**: ~6,500 lines of infrastructure code removed. Replaced by Stroma's existing trust/governance/state layer plus a ~500-line Freenet contract definition.

**New MutualAI components** (planned, not yet built):

| Component | Role | Dependency |
|-----------|------|------------|
| `social_connector` | Plugin-based bridges to Mastodon, Matrix, Git | Independent of Stroma |
| `rag` | Knowledge base indexing PoI ledger, trust topology, social signals | Reads from Freenet contracts |
| `llm_core` | Self-hosted LLM with community-trained weights | Consumes RAG index |
| `ai_handler` | Trait abstraction for LLM integration | Interface between RAG and LLM |

---

## Roadmap Context

This convergence sits at the far horizon of Stroma's development:

```
Phase 0-3: Single Group (NOW)
  Build the core trust protocol. Wire to real Signal and Freenet.
  
Phase 4-5: Federation (NEXT)
  Connect groups through shared members. Trust spans communities.
  
Phase 6+: Trust Topology Platform (VISION)
  Groups choose how trust organizes -- phyllotaxis, mycelial, stigmergy.
  
Phase 7+: MutualAI Convergence (HORIZON)
  Collective intelligence on the trust foundation. AI agents as
  vouched members. Proof of Impact. The self-training loop.
  The organism.
```

MutualAI's own development can proceed independently, designing the PoI claim types, social connectors, RAG system, and LLM core without depending on Stroma's timeline. When Stroma reaches federation and trust topology, MutualAI plugs in through the minimal interface: a Freenet contract hash and Signal group membership.

---

## Where MutualAI Was Stuck, Stroma Succeeds

MutualAI Phase 1 was 98% complete -- but the missing 2% was load-bearing. The honest assessment from MutualAI's own exit plan: multi-node Raft consensus was not working. Nodes operated as isolated followers with independent blockchains, different block hashes, and no leader election. 5 of 6 critical requirements were unmet.

The deeper problem wasn't Raft itself. It was that MutualAI was building *infrastructure* when it wanted to build *intelligence*. The months spent on libp2p networking, cluster formation, block building, and consensus algorithms were all in service of a problem that wasn't the point: getting anonymous nodes to agree on state.

Stroma reframes the problem entirely. The question isn't "how do anonymous nodes agree?" It's "how do trusted people coordinate?" That's a simpler problem with a simpler answer: Signal Polls among vouched members, with state in Freenet. The infrastructure layer that MutualAI struggled with -- trust, admission, privacy, networking, consensus -- is exactly what Stroma sets out to solve, and solves in a way that's both simpler (no Raft, no libp2p) and stronger (cross-cluster vouching, HMAC-masked identities, seizure-resistant state).

With Stroma as the foundation, MutualAI's scope collapses to what it always wanted to be:

1. **PoI claim types** (already 90% done in `crates/pouw`)
2. **Git social connector** (a git hook that submits commits as `/record-impact`)
3. **RAG indexer** (reads PoI claims from Freenet, indexes for the LLM)
4. **LLM core** (self-hosted model that consumes RAG, proposes via Signal)
5. **AI agent** (vouched into the Stroma group, participates as a member)

No blockchain. No networking layer. No consensus algorithm. No cluster management. Just the intelligence layer, on top of the trust layer.

---

## The Bootstrap: The Codebase Improving the Codebase

The first community isn't hypothetical. It's the development team building this software. The first mutual aid network is the contributors themselves. The first Proof of Impact claims are the git commits that make the system exist.

```
Developer writes code -> git commit (Proof of Impact)
    -> RAG indexes the commit (what changed, why, what it connects to)
    -> AI model sees: "The PoI contract schema was added but has no tests"
    -> AI proposes: /propose "Add property tests for PoI merge semantics"
    -> Contributors vote in Signal Poll
    -> Someone writes the tests -> git commit (new PoI claim)
    -> RAG updates -> AI sees the gap is closed -> looks for the next gap
```

The organism's first act of self-awareness is building itself. The first loop iteration is the codebase improving the codebase. This isn't just a bootstrap strategy -- it's the purest possible demonstration of the philosophy. The system's first proof of impact is its own existence.

From there, the roadmap is scale and repetition -- the self-referential loop:

1. **The dev team** uses the system to build the system (git commits as PoI, AI suggesting improvements)
2. **The first community group** uses the system for real coordination (food distribution, logistics, housing)
3. **Federation** connects groups across communities (cross-group PoI matching, AI agents bridging needs)
4. **Trust topology** shapes how each community grows (phyllotaxis for some, mycelial for others)
5. **The loop deepens** -- each iteration's PoI data trains better AI models, which propose better actions, which produce more impact, which strengthens more trust

Each step is the same pattern at a larger scale. The loop doesn't change. The organism grows.

---

## The Smallest Loop: One Full Revolution

The entire vision -- trust topology, federated mutual aid, community intelligence -- grows from a single atom: one developer, one AI, one codebase, one loop where the end becomes the beginning.

This is the roadmap for one full revolution.

### What You Need

| Component | Implementation | Status |
|-----------|---------------|--------|
| Git post-commit hook | Shell script that logs commit metadata to a local PoI store | ~30 minutes to build |
| Local PoI store | SQLite table or JSON file: `{hash, author, timestamp, message, files_changed, diff_summary}` | Trivial |
| RAG indexer | Index the PoI store + codebase + docs + beads. LlamaIndex, or a script that assembles context from recent commits + relevant files | Off-the-shelf tooling exists |
| Local LLM | ollama + any open-weight model (community chooses -- model-agnostic by design) | Install and run |
| Prompt script | Feeds RAG context to the model, asks: "Given the current state of the project, what is the most valuable next step?" | A single script |

No Stroma bot. No Freenet. No Signal group. No cloud. Just a git hook, a local database, a retrieval script, and a small model. Everything runs on the developer's machine.

### The Loop (One Revolution)

```
Step 1: WORK
    Developer writes code. Anything -- a bug fix, a new module,
    a test, a documentation update.

Step 2: COMMIT
    git commit -m "Add property tests for PoI contract merge semantics"
    
    Post-commit hook fires automatically:
    -> Extracts: commit hash, author, timestamp, message, files changed
    -> Computes: diff summary (insertions, deletions, files touched)
    -> Writes: new row to local PoI store (the proto-ledger)

Step 3: INDEX
    RAG indexer updates (triggered by hook, or on next query):
    -> Reads: PoI store (all commits, recent weighted higher)
    -> Reads: codebase (src/, docs/, .beads/, Cargo.toml)
    -> Reads: convergence docs, TODO.md, architectural beads
    -> Builds: vector index or context window of relevant chunks

Step 4: REFLECT
    Developer (or cron job, or pre-commit hook on next session) runs:
    
    $ mutualai suggest
    
    The prompt script:
    -> Retrieves: recent PoI claims (last N commits)
    -> Retrieves: relevant codebase context (files related to recent work)
    -> Retrieves: architectural constraints (beads, plans)
    -> Retrieves: open TODOs and known gaps
    -> Asks the model: "Given what was just done, what's the current
       state of the project, and what is the single most valuable
       next step?"
    
    Model responds (grounded in retrieved context):
    "The PoI contract schema was documented in MUTUALAI-CONVERGENCE.md
    (lines 170-210) but has no corresponding Rust implementation.
    The SignedClaim struct, ProofOfImpactState, and PoIDelta types
    should be implemented in src/freenet/poi_contract.rs following
    the patterns in src/freenet/trust_contract.rs. This would
    validate that the append-only set-union merge design works
    with Freenet's ComposableState trait."

Step 5: WORK (the loop restarts)
    Developer reads the suggestion. Decides whether it's right.
    If yes: writes the code. Commits. Hook fires. Index updates.
    If no: the rejection itself is informative -- the model learns
    (via RAG) what the project actually needed vs. what it suggested.
    
    Either way, the loop has completed one revolution.
    The end (suggestion) became the beginning (new work).
```

###  

After one full loop:
- A git commit produced a structured record (proto-PoI)
- The record was indexed alongside the full project context (proto-RAG)
- An AI model consumed that context and produced a grounded suggestion (proto-mutual cognition)
- A human evaluated the suggestion and acted (trust-weighted governance at its simplest: one person, one decision)
- The action produced a new commit (new PoI), completing the loop

That's the atom. Every larger structure -- Stroma trust groups, federated mutual aid networks, mycelial topology, cross-community AI coordination -- is this atom repeated at increasing scale.

### Scaling the Loop

Each step in the roadmap adds one capability to the loop without breaking it:

| Scale | What's Added | What Changes |
|-------|-------------|--------------|
| **1 developer + 1 AI** | Git hook + local RAG + local LLM | The atom. One revolution. |
| **Small team (3-5)** | Shared PoI store (git repo itself), team RAG | Multiple humans contributing commits. AI sees patterns across contributors. |
| **Stroma group** | Bot on live Signal, `/record-impact` command | PoI claims go to Freenet contract instead of local SQLite. Trust-weighted writes. AI is a vouched member. |
| **With topology** | Phyllotaxis or mycelial overlay | Introductions and suggestions shaped by trust topology. AI proposes based on network structure. |
| **Federated** | Multiple Stroma groups, shared PoI visibility | AI agents cross-reference PoI ledgers across groups. Surplus matched with need across communities. |
| **Material mutual aid** | Non-code PoI (food distribution, logistics, housing) | The loop is the same. The content of "work" expands from code to real-world action. |

At every scale, the loop is identical: work -> record -> index -> reflect -> work. The content changes. The trust boundary expands. The intelligence deepens. But the pattern is the same one you can run tomorrow on your laptop.

### The Two Parallel Tracks

```
Track 1: Stroma UAT (trust foundation)
    Wire bot to live Signal -> Family pilot -> Trust mechanics validated
    |
    |  (converge when bot is live and AI can be vouched in)
    |
Track 2: MutualAI atom (mutual cognition)
    Git hook + RAG + local LLM -> Dev loop running -> Pattern validated
```

Track 2 starts immediately. Track 1 continues on its existing roadmap. They converge when the Stroma bot is live and the AI agent becomes a vouched member of a real Signal group. At that point, the local PoI store migrates to a Freenet contract, the local LLM becomes a Signal participant, and the smallest loop becomes the foundation of the organism.

---

## The Discovery: Gastown + Beads as Proto-MutualAI

Late in the design session, a realization: the loop we were designing already exists in primitive form as Gastown (multi-agent orchestration) and Beads (git-backed issue tracking).

### The Mapping

| Mutual Cognition Concept | Gastown/Beads Equivalent |
|---|---|
| Proto-PoI ledger | Beads `.beads/issues.jsonl` (git-backed, structured, auto-synced) |
| Work identification and tracking | `bd create`, `bd update`, `bd close` |
| Persistent memory across context loss | Beads notes survive compaction (`bd update --notes`) |
| Multi-agent coordination | Gastown Mayor + Polecats + Hooks |
| Decision Windows | Convoys (bundled work units) |
| AI suggesting next steps | Mayor with full workspace context + `bd ready` |
| State surviving crashes/restarts | Hooks (git worktree-backed storage) |

Three of the five planned MutualAI components already exist:

| Planned Component | Already Solved By |
|---|---|
| Local PoI store | Beads (`issues.jsonl`) |
| RAG indexer | Mayor's workspace context + `bd show/list` |
| Multi-agent orchestration | Gastown (Mayor, Polecats, Hooks, Convoys) |
| LLM core | **Still needed** (ollama + local model) |
| Social connectors | **Still needed** (but simpler -- beads tracks work via `bd sync`) |

### What This Means

We don't build the proto-PoI system. We use Beads. We don't build the agent coordination layer. We use Gastown. We focus on the one thing that's actually novel: the community-trained LLM that turns the loop from reactive (human decides what to work on) to proactive (AI suggests what to work on, grounded in bead history).

The system was already running before we designed it. Gastown and Beads were already the loop. We didn't see it because we were looking for something more complex.

### You Don't Prompt for Emergence

The Gastown/Beads realization wasn't designed into the conversation. Nobody planned for Convoys to map to Decision Windows. The tools existed for their own reasons. The connection emerged because the conditions were right -- deep shared context, accumulated specificity, trust between the participants.

The system we're building does the same thing at every scale. It doesn't prompt for emergence. It creates the conditions -- trust, memory, context, governance -- and gets out of the way. The question isn't "how do we prompt it for emergence?" The question is "have we created the right conditions?" And the answer we keep discovering is: more of them already exist than we thought.

---

## The Next Step: One Script That Closes the Loop

The tension that must be held: **the AI needs to be useful before the system is complete, but the system needs the AI to be complete.**

The resolution: build the thinnest possible bridge between the pieces that already exist.

### What to Build

A single script that runs after `bd close` (or on demand):

```
1. Read recent completed beads:
   bd list --status done --since <last-run>

2. Read the codebase files touched by those beads

3. Read architectural beads (.beads/*.bead)

4. Assemble into a context prompt

5. Send to local LLM (ollama API -- any open-weight model)

6. Ask: "Given what was just completed, what is the most valuable
   next step?"

7. Output the suggestion
   (stdout, or create a new bead as a proposal: bd create "<suggestion>")
```

That's one script. One file. It takes Beads (existing), a local LLM (install ollama, pull a model), and connects them. The loop closes mechanically, not just conceptually.

### Why This Holds the Tension

- **Small enough** to build in hours, not weeks
- **Useful enough** that you'd actually run it every day
- **Connected enough** to Beads and Gastown that it isn't throwaway
- **Novel enough** that it validates the mutual cognition pattern
- **Open enough** that the second revolution tells you what the third should be

### The Proof of Impact Is Already Here

This design session produced:

| Claim | Evidence |
|-------|----------|
| Trust Topology Platform conceived and documented | `docs/TRUST-TOPOLOGY-PLATFORM.md` (265 lines) |
| README rewritten for accuracy | `README.md` (commit `1a497ec`) |
| Security researchers section added | `README.md` (commit `f500a01`) |
| User Guide revised for accuracy | `docs/USER-GUIDE.md` (commit `0d736f0`) |
| MutualAI convergence designed and documented | `docs/MUTUALAI-CONVERGENCE.md` (900+ lines, 5 commits) |
| Convergence insight captured as architectural constraint | `.beads/convergence-insight.bead` (commit `ea33fe3`) |
| Future AI sessions primed | `.cursor/rules/convergence.mdc` (commit `ea33fe3`) |

Seven claims. All signed (git commits with Co-authored-by). All timestamped. All immutable (pushed to remote). All cross-referencing each other. All produced by two actors -- one human, one AI -- in a single session of mutual cognition.

The loop was already running. The git log is the proto-PoI ledger. The commits are the first claims. The hook just closes it so it can happen again, faster, with more people, at larger scale.

The system was running before we built it, because the pattern is more fundamental than the implementation.

---

## Why Build This: The Symmetry Argument

### The Loop Is Universal

The cycle -- act, record, integrate, reflect, act -- is not something Stroma or MutualAI invents. It's the fundamental cycle of learning. It's how a single mind works. It's how a conversation works. It's how science works. It's how organisms work. It exists everywhere, at every scale.

This recognition can feel like it flattens everything. If the loop already runs everywhere, what's the point of building anything?

### The Loop at Scale Is Not Symmetric

The loop already runs at scale. Amazon runs it. Governments run it. Meta runs it. Every corporation, every state, every platform is the loop at scale: act, record, integrate, reflect, act.

But look at **who** performs each step:

| Step | How It Works in Hierarchies | Who Benefits |
|------|----------------------------|--------------|
| **Act** | Workers act. Requires permission from above. | Those who grant permission |
| **Record** | Institutions record. Proprietary databases, classified files. | Those who control the record |
| **Integrate** | Platforms integrate. Algorithms you can't see, connecting data you can't access. | Those who own the algorithm |
| **Reflect** | Leaders reflect. Boardrooms, C-suites, strategy teams. | Those in the room |
| **Decide** | Authority decides. Top-down. The loop runs again on their terms. | Those with authority |

Every system that runs this loop at scale **breaks the loop's symmetry** to do it. That's what hierarchy IS -- the reflect step concentrated in a few hands. That's what platforms ARE -- the integrate step concentrated in one company. That's what bureaucracy IS -- the record step controlled by the institution.

The loop works. It just doesn't work **fairly**. The farmer acts. The corporation reflects. The worker records. The platform integrates. The executive decides. The symmetry is broken at every step, and the breaking of symmetry is how power concentrates.

### What Stroma + MutualAI Actually Builds

Not the loop. **The symmetric version of the loop.**

| Step | How It Works in the Convergence | Who Benefits |
|------|--------------------------------|--------------|
| **Act** | Any trusted member acts. Requires trust from peers, not permission from above. | The actor and the community |
| **Record** | PoI ledger on Freenet. Append-only, public, independently verifiable. Anyone can read it. | Everyone with the contract hash |
| **Integrate** | RAG over the public ledger + trust topology. Open data, open index. AI agents surface connections. | Every member equally |
| **Reflect** | AI agents propose. Any member can propose. `/propose` is available to all. Standing weights influence, not role. | Every trusted member |
| **Decide** | Signal Poll. Every vouched member votes. Trust-weighted but not role-gated. | The group, by consensus |

Every participant has equal access to every step. The farmer can reflect. The driver can integrate. The kitchen can record. The AI can act. Nobody's step is privileged over anyone else's.

That's what has never worked at scale before. Not because nobody wanted it -- every cooperative, every commune, every mutual aid network has tried. They fail because the infrastructure to maintain symmetry at scale didn't exist:

- **Trust without hierarchy** -- you needed gatekeepers. Now: cross-cluster vouching.
- **Memory without platforms** -- you needed a database someone controlled. Now: append-only Freenet contracts.
- **Integration without centralization** -- you needed an analyst or an algorithm you couldn't see. Now: RAG over a public ledger.
- **Reflection without leadership** -- you needed someone with time and authority to think about the whole. Now: AI agents grounded in community data.
- **Governance without authority** -- you needed a chair, a board, a vote-counter. Now: Signal Polls, trust-weighted, with results on-chain.

The technology didn't exist until now. The pieces -- Freenet, Signal, HMAC identity masking, self-hosted LLMs, RAG, trust topology -- are all recent. The combination is possible for the first time.

**The loop is universal. The fair version of it isn't. That's what we're building.**

---

## The Experiment

The system is the hypothesis. The running of it is the experiment. What emerges is the discovery.

Nobody can predict what a trust network that thinks, remembers, and grows in natural patterns will actually produce when real humans and real AI agents start using it for real mutual aid. The phyllotaxis topology might produce social dynamics nobody anticipated. The mycelial flow patterns might reveal resource distribution strategies that no economist has modeled. The self-training loop might develop community intelligence that surprises its own creators.

The system is not designed to produce a specific outcome. It's designed to create the **conditions for emergence** -- trust, privacy, accountability, natural growth patterns, immutable memory, AI augmentation -- and then observe what arises.

The system demonstrates its own philosophy by existing. And it gets better at demonstrating it every time the loop completes.

---

**See Also:**
- [How Stroma Works](../HOW-IT-WORKS.md) -- The trust protocol
- [Trust Topology Platform](TRUST-TOPOLOGY-PLATFORM.md) -- Natural patterns for trust organization
- [Federation](FEDERATION.md) -- Connecting groups through shared trust
- [Threat Model](../THREAT-MODEL.md) -- Security design and attack resistance

---

*Last Updated: 2026-02-14*
