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

See [How Stroma Works](HOW-IT-WORKS.md) for the full protocol and [Trust Topology Platform](TRUST-TOPOLOGY-PLATFORM.md) for the topology vision.

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

**The interface is minimal:**
- A Freenet contract hash for the PoI ledger (stored in Stroma's `GroupConfig`)
- Signal group membership (AI agents are vouched members)
- The `/propose` command (universal governance mechanism)

Activating MutualAI for a Stroma group is itself a proposal:

```
/propose stroma poi_contract <freenet-contract-hash>
```

The group votes. If approved, the Stroma bot begins writing Proof of Impact claims to that contract on behalf of trusted members. If not, the group remains a pure trust network.

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

### 3. AI Agents as Vouched Community Members

AI agents in the converged system are not special. They are members -- vouched in through the same mechanism as humans. A human `/invite`s their AI agent. An assessor from a different cluster evaluates it. If vouched, the AI gets a `MemberHash`, a trust standing, and can be flagged and ejected like any other member.

This creates a new kind of AI alignment: **alignment through social accountability.** The community decides whether an AI agent is trustworthy using the same mechanism it uses for humans. An AI that proposes harmful actions gets flagged. An AI that consistently produces useful work builds standing. The trust model doesn't care whether you're carbon or silicon -- it cares whether the community trusts you.

Signal group configurations could include:
- **Human-majority groups**: A community organizing group where 1-2 AI agents participate, proposing logistics, surfacing needs, recording impact. Humans lead, AI assists.
- **AI-majority groups**: A coordination group where multiple AI agents (each vouched by different human sponsors from different clusters) deliberate on resource matching and logistics. Humans can observe and intervene.
- **Mixed groups**: Federation planning groups where human delegates work alongside AI agents that RAG each group's PoI ledger to find collaboration opportunities.

In all cases, the interaction protocol is identical: Signal messages, `/propose` for decisions, Signal Polls for votes.

### 4. Trust Topology Shapes Collective Action

Different trust topologies produce different collective action dynamics:

| Topology | Natural Pattern | Collective Action Use |
|----------|----------------|-----------------------|
| **Phyllotaxis** | Golden spiral | Organic community growth. Anti-clique property ensures no faction dominates decisions. Maximum diversity of perspective. |
| **Mycelial** | Fungal network | Material mutual aid -- food, housing, energy distribution. Optimizes for resource flow, not structural resilience. Reroutes around failures. |
| **Stigmergy** | Ant colony trails | Emergent coordination. No algorithmic introductions. Trust patterns emerge from accumulated Proof of Impact trails. Minimal structure, maximum self-organization. |

The topology doesn't just shape trust -- it shapes **how the group thinks collectively** and **how resources flow through the network.**

A group organizing food distribution chooses mycelial topology. The health metric is flow capacity: can resources reach every member regardless of individual failures? Strategic introductions connect members who would create new flow paths, not just new structural bridges. The message: "The network suggests you connect with someone in a part of the web that's currently thin on connections to your area."

The topology platform stops being abstract and becomes **materially useful** -- deterministic trust patterns (Stroma) combined with non-deterministic collective intelligence (MutualAI), creating self-organizing ways of producing real-world impact, in non-hierarchical, voluntary ways.

### 5. Proof of Impact Strengthens Vouches

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
| **PoI Freenet contract** | Append-only `BTreeSet<SignedClaim>` contract on Freenet -- the immutable ledger |
| **`/record-impact` command** | Stroma bot command for writing PoI claims through the trust boundary |
| **`poi_contract` config** | `GroupConfig` field linking a Stroma group to its MutualAI PoI ledger |
| **Trust-weighted PoI** | Impact claims weighted by the author's Stroma trust standing |
| **Evidence-based vouching** | `/vouch` grounded in verifiable PoI history |
| **Mycelial topology** | Trust topology optimized for resource flow (mutual aid distribution) |

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

## The Experiment

The system is the hypothesis. The running of it is the experiment. What emerges is the discovery.

Nobody can predict what a trust network that thinks, remembers, and grows in natural patterns will actually produce when real humans and real AI agents start using it for real mutual aid. The phyllotaxis topology might produce social dynamics nobody anticipated. The mycelial flow patterns might reveal resource distribution strategies that no economist has modeled. The self-training loop might develop community intelligence that surprises its own creators.

The system is not designed to produce a specific outcome. It's designed to create the **conditions for emergence** -- trust, privacy, accountability, natural growth patterns, immutable memory, AI augmentation -- and then observe what arises.

The system demonstrates its own philosophy by existing. And it gets better at demonstrating it every time the loop completes.

---

**See Also:**
- [How Stroma Works](HOW-IT-WORKS.md) -- The trust protocol
- [Trust Topology Platform](TRUST-TOPOLOGY-PLATFORM.md) -- Natural patterns for trust organization
- [Federation](FEDERATION.md) -- Connecting groups through shared trust
- [Threat Model](THREAT-MODEL.md) -- Security design and attack resistance

---

*Last Updated: 2026-02-14*
