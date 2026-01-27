# Stroma

**Secure group messaging where trust is earned, not granted.**

The goal of this project is to help build federations of human networks based on trust and anonymity. 

## The Problem

You want a private group chat where everyone can trust each other. But how do you know who to let in?

Traditional solutions create new problems:
- **Invite links**: Anyone with the link can join - no vetting
- **Admin gatekeepers**: One person controls who gets in - creates hierarchy
- **Voting on strangers**: How do you vote on someone you don't know?
- **Large groups become cliques**: Trust clusters form, newcomers stay on the periphery

## How Stroma Solves This

Stroma is a **smart bot** (no AI) that manages your Signal group using a simple principle: **You can only join if two people already in the group vouch for you, and those two people must be from different parts of the network.**

### What This Means:
- **No strangers**: Every member is personally vouched for by at least 2 people already in the group
- **No gatekeepers**: No single person controls entry - trust is distributed
- **No cliques**: The bot ensures vouches come from different parts of your network
- **No hierarchy**: Trust emerges from relationships, not authority

### How It Works (Simple):

1. **Someone invites you**: A member sends `/invite @YourName "Context about you"` to the bot in a private message
   - Their invitation counts as your first vouch
   - Bot immediately starts the vetting process

2. **You get vetted**: The bot introduces you to a second member from a different part of the network
   - Bot creates a 3-person Signal chat (you, the member, and the bot)
   - You have a brief conversation (10-15 minutes)
   - Bot doesn't participate - just facilitates the introduction

3. **Second vouch**: After your conversation, the member vouches for you
   - They send `/vouch @YourName` to the bot
   - Bot verifies the second vouch with cryptographic proof

4. **You're admitted**: The bot adds you to the Signal group
   - You're now a Bridge (2 effective vouches)
   - Your trust standing is positive
   - Bot welcomes you and deletes all vetting session data

5. **You stay connected**: Keep building relationships in the group
   - If a voucher leaves the group → you need a new vouch immediately
   - If a voucher flags you → their vouch is invalidated, you need a replacement
   - Build 3+ connections to become a Validator (more resilient)

### The Magic:

The bot acts like a **"Blind Matchmaker"** - it sees the pattern of connections but doesn't know your personal relationships. It suggests introductions to strengthen the group's trust network without knowing why people trust each other.

**For non-technical users**: It just feels like a helpful bot that manages your Signal group. You use simple commands like `/invite @friend` or `/status` in private messages with the bot. The bot handles everything else automatically - vetting newcomers, monitoring trust, and keeping the group secure. You don't need to understand the technical details.

**For privacy advocates**: All identities are cryptographically hashed (HMAC-SHA256 with group-secret pepper), trust is verified with zero-knowledge proofs (STARKs - no trusted setup, post-quantum secure), state is stored in decentralized Freenet network with eventual consistency (ComposableState, summary-delta sync), and the social graph is never exposed.

**For developers**: Built on [freenet-core](https://github.com/freenet/freenet-core) (Rust-native Wasm contracts with ComposableState trait). Uses set-based membership (BTreeSet) with on-demand Merkle Tree generation for ZK-proof verification. State synchronizes via summary-delta protocol with CRDT-like merge semantics (no consensus algorithms). Trust verified via STARKs (winterfell library). See [freenet-contract-design.mdc](.cursor/rules/freenet-contract-design.mdc) for patterns.

## Why "Stroma"?

In biology, stroma is the supportive tissue that holds organs together. In your group, Stroma is the underlying trust network that holds the community together - invisible but essential.

---

## User Experience

### For Group Members

#### Daily Interaction
You interact with Stroma through a bot in Signal - the messaging app you already use. Everything happens through simple commands in private messages:

**Inviting someone new:**
```
You → Bot (1-on-1 PM): /invite @Friend "Great organizer from local community"

Bot → You:
"Your invitation for @Friend has been recorded as the first vouch.
I'm now reaching out to a member from a different part of the network 
for the second vouch. I'll keep you updated."
```

**Checking your status:**
```
You → Bot: /status

Bot → You:
"📊 Your Trust Status
Role: Bridge
Effective vouches: 2 ✅ (Alice, Bob)
Standing: +2 (positive)

💡 Tip: You're at minimum threshold. Building more connections 
will increase your resilience if a voucher leaves or flags you."
```

**Viewing network health:**
```
You → Bot: /mesh strength

Bot → You:
"📈 Network Strength: 38% Mesh Density

Vouch Distribution:
  2 vouches (Bridges):    ████████░░░░ 22 members (47%)
  3-5 vouches:            █████░░░░░░░ 15 members (32%)
  ...
"
```

#### What You See
- **Natural Language**: Bot speaks conversationally, not with technical jargon
- **Privacy**: All interactions in 1-on-1 PMs (never in group chat)
- **Transparency**: You can see your trust standing, network health, and configuration
- **Simplicity**: Complex cryptography and Freenet state management completely hidden

#### What You Don't See
- Freenet contracts, ComposableState, or summary-delta synchronization
- HMAC hashing, zeroization, or Merkle Tree generation
- STARK proof generation or verification
- State stream monitoring or merge conflicts

The bot handles all technical complexity automatically. You just use simple Signal commands.

### For Operators

#### Responsibilities
- **Bootstrap** (one-time): Manually add 3 seed members, initialize Freenet contract, start bot
- **Maintenance** (ongoing): Run bot service (systemd), monitor logs, restart on crashes
- **NO Special Privileges**: Operator is just another member (cannot override group decisions)

#### What Operators Can't Do
- Manually add/remove members (bot does this automatically based on Freenet state)
- Change configuration without group vote
- Override ejections or bypass trust protocol
- See cleartext Signal IDs (only hashed identifiers)

#### Deployment
```bash
# Start freenet-core node
systemctl start freenet-core

# Start Stroma bot
systemctl start stroma-bot

# Monitor (read-only)
journalctl -u stroma-bot -f
```

Operator is a **service runner**, not a **privileged admin**. The group governs itself through Signal Polls.

---

## Technical Overview

Stroma is a scalable trust network that leverages Signal for its user interface and Freenet for its decentralized, anonymous back-end. The core innovation is **Recursive Zero-Knowledge (ZK) Vouching**, which allows the network to scale by 10²-10³ without revealing the social graph.

**Fundamental Principle**: Trust is an **emergent property** of the mesh, not a centralized database.

**Core Invariant**: The Stroma Signal group contains ONLY fully vetted members. Every member has ≥2 vouches from independent Members at all times.

**Design Philosophy**: Mutual arising, fluid selfhood, and non-hierarchical organization where trust emerges from collective interaction.

## MVP Scope & Federation as North Star

### What's In MVP
**Objective**: Fully functional single-group trust network

✅ **Included in MVP:**
- Bootstrap seed group (3 members)
- Invitation & vetting flow (no token exchange)
- Vouching & flagging (any Member can vouch)
- Ejection enforcement (two independent triggers)
- Internal mesh optimization (Blind Matchmaker)
- Configuration management (Signal Polls)
- Bot commands: `/invite`, `/vouch`, `/flag`, `/status`, `/mesh`, `/mesh strength`, `/mesh config`, `/propose-config`, `/audit operator`
- Mesh density metrics and histogram
- Operator least privilege
- Full security model (HMAC masking, STARKs, zeroization)

❌ **NOT in MVP (Phase 4+ - Future):**
- Federation between groups
- Shadow Beacon broadcast
- PSI-CA bot-to-bot protocol
- Cross-mesh vouching
- Federated Merkle Trees

### Federation as North Star
**Critical**: Even though federation is NOT in the MVP, it is the **ultimate objective** and **north star** for all design decisions.

**Why This Matters:**
- All architecture decisions optimize for future federation
- Contract schema includes federation hooks (unused in MVP)
- Identity hashing is re-computable for PSI-CA
- Bot module structure includes `federation/` (disabled in MVP)
- Node type definitions support cross-mesh scenarios

**Goal**: Connect as many people as possible anonymously via trust through federated groups.

## Core Innovation

### Recursive Zero-Knowledge Vouching

Traditional trust networks require revealing the social graph to scale. Stroma solves this by:

- **Zero-Knowledge Proofs**: Verify trust without revealing who vouched (STARKs - no trusted setup)
- **Recursive Proofs**: Batch updates for constant-time verification regardless of network size
- **Private Set Intersection (PSI)**: Calculate overlap between groups without revealing member identities
- **Set-Based Membership**: Store member sets (BTreeSet), generate Merkle Trees on-demand for ZK-proofs
- **Mergeable State**: CRDT-like structures enable eventual consistency without consensus algorithms
- **Vouch Invalidation**: Voucher-flaggers invalidate their own vouches (logical consistency)

### Scaling Model

- **Bootstrap (3 users)**: Manual seed group creation, all vouch for each other (initial triangle)
- **Local Phase (3-50 users)**: Single Stroma group, strategic matching for MST within group
- **Cluster Phase (50-200 users)**: Internal clusters form, Blind Matchmaker optimizes cross-cluster connections
- **Federation Phase (200+ users)**: Multiple Stroma groups, Blind Rendezvous for anonymous federation
- **Mass Scale (2000+ users)**: Multi-bot consensus, Sybil detection, recursive proofs, sharded storage
- **Scaling Factor**: 10²-10³ (100x to 1000x) without revealing social graph

### Key Terminology

- **Trust Standing**: `Standing = Effective_Vouches - Regular_Flags` (must remain ≥ 0)
- **Effective Vouches**: Total vouches minus voucher-flaggers (vouchers who also flagged)
- **Vouch Invalidation**: If a voucher flags a member, that vouch is invalidated
- **Voucher-Flaggers**: Members who both vouched for AND flagged someone (contradictory, invalidates vouch)
- **Social Anchor**: Top-N validators used for emergent bot discovery
- **Shadow Beacon**: Encrypted bot discovery advertisement on Freenet
- **PSI-CA**: Private Set Intersection Cardinality for anonymous overlap detection
- **ComposableState**: Freenet trait for mergeable state with summary-delta synchronization
- **On-Demand Merkle Trees**: Generated from member sets for ZK-proofs (not stored in contract)
- **Fluid Selfhood**: Dynamic, relational identity concept

## Architecture

### Three-Layer Design

#### 1. User Interface Layer: Signal
- **Human-facing interface** for trust operations (familiar, E2E encrypted)
- **Bot commands**: 10 commands for invitation, vouching, flagging, status, configuration
- **All operations in 1-on-1 PMs** (never group chat) - prevents metadata leakage
- **Conversational responses**: Natural language, non-technical (abstracts complexity)
- **Signal Polls**: Structured voting for configuration and federation decisions
- **Privacy**: Bot never stores Signal IDs in cleartext (HMAC-hashed immediately)

#### 2. Trust Logic Layer: Rust Bot
- **Four Roles**:
  - **Protocol Gatekeeper**: Enforces 2-effective-vouch requirement for admission
  - **Blind Matchmaker**: Suggests strategic introductions across internal clusters for MST optimization
  - **Diplomat**: Discovers and proposes federation with other groups (Phase 4+)
  - **Health Monitor**: Continuous trust standing checks via Freenet state stream

- **Core Functions**:
  - Identity masking (HMAC-SHA256 with group-secret pepper, immediate zeroization)
  - Trust verification (STARK proof validation via winterfell)
  - Vouch invalidation (voucher-flaggers invalidate their own vouches)
  - Ejection protocol (immediate when Standing < 0 OR Effective_Vouches < 2)
  - Mesh optimization (graph analysis, cluster detection, strategic suggestions)
  - Configuration management (Signal Polls, automatic updates)
  
- **Design Principles**:
  - **Ephemeral Memory**: Raw Signal IDs wiped immediately, vetting data deleted after admission
  - **Stateless**: All persistent state comes from Freenet (bot can restart without data loss)
  - **Event-Driven**: Monitors Freenet state stream, reacts to changes in real-time
  - **User Abstraction**: Completely hides Freenet/crypto complexity from users
  - **Automatic Execution**: Executes all Freenet contract-approved actions without operator intervention

#### 3. State Layer: Freenet (Dark)
- **Decentralized Storage**: No central server, state exists across peer-to-peer network
- **Eventual Consistency**: Summary-delta synchronization (no consensus algorithms like Paxos/Raft)
- **ComposableState Contracts**: Wasm code defines how to merge conflicting states deterministically
- **Set-Based Membership**: BTreeSet for members (naturally mergeable via set union)
- **Vouch Graph**: HashMap<MemberHash, BTreeSet<MemberHash>> (mergeable via map union)
- **On-Demand Merkle Trees**: Generated from member sets for ZK-proof verification (not stored in contract)
- **State Stream**: Real-time monitoring (not polling) - bot reacts to changes immediately
- **Anonymous Routing**: Dark mode (no IP exposure)
- **Emergent Discovery**: Bots find each other via Social Anchor Hashing (Phase 4+)

**Key Innovation**: Freenet's summary-delta sync enables efficient eventual consistency without requiring all nodes to agree simultaneously.

## Core Modules

### A. The Kernel (Identity Masking)
- **Purpose**: Never store Signal IDs in cleartext
- **Method**: HMAC-based hashing with group-secret pepper (not deterministic hashing)
- **Security**: Zeroize buffers immediately after hashing (ring + zeroize crates)
- **Ephemeral State**: Vetting session data deleted after admission threshold met
- **Blinded Identifiers**: Public ID for bot, private ID for ZK-math
- **Result**: Memory dump contains only hashed identifiers

### B. The Shadow Beacon (Emergent Discovery)
- **Purpose**: Bots find each other without admin coordination
- **Method**: Social Anchor Hashing (hash of top-N validators, dynamic threshold)
- **Social Frequency**: Discovery based on shared social roots, not pre-shared keys
- **Discovery**: Bloom Filters + PSI-CA for anonymous overlap detection
- **Threshold**: Federation proposed if `|A ∩ B| / |A ∪ B| > 10%` (intersection density)
- **Commutative Encryption**: Double-blinding for PSI handshake

### C. The Gatekeeper (Signal Admin Bot)
- **Purpose**: Strictly enforce 2-effective-vouch requirement for group admission
- **Admission Protocol**:
  1. Member sends `/invite @username [context]` - invitation counts as first vouch
  2. Bot selects second Member via Blind Matchmaker (prefers Validators for optimization)
  3. Bot facilitates vetting interview with selected Member from different cluster
  4. Second Member vouches via `/vouch @username`
  5. Admission ONLY after 2 effective vouches confirmed in Freenet contract
- **Vouches Source**: ONLY from Members already IN the Stroma Signal group
- **Who Can Vouch**: ANY Member (Bridges and Validators), not restricted to Validators only
- **Waiting Room**: State of being OUTSIDE Signal group during vetting (not a separate chat)
- **Trust Standing**: `Standing = Effective_Vouches - Regular_Flags` (must remain ≥ 0)
- **Vouch Invalidation**: If a voucher flags a member, that vouch is invalidated
- **Ejection Triggers**:
  - `Standing < 0` (too many regular flags) → Immediate removal
  - `Effective_Vouches < 2` (voucher left OR voucher flagged) → Immediate removal
- **No Grace Periods**: No warnings, no re-verification windows, instant ejection
- **UX**: All operations in 1-on-1 PMs, bot responds conversationally
- **Bot Commands**: `/invite`, `/vouch`, `/flag`, `/status`, `/mesh`, `/mesh strength`, `/mesh config`, `/propose-config`, `/audit operator`

### D. The Diplomat (External Federation)
- **Purpose**: Coordinate federation between SEPARATE Stroma groups
- **Scope**: EXTERNAL - different Signal groups with different bots
- **Method**: Blind Rendezvous with PSI-CA, BidirectionalMin threshold evaluation
- **Consensus**: Signal Poll vote in each group (configurable approval threshold)
- **Independent Thresholds**: Each group evaluates with own minimum intersection density
- **Mutual Consent**: Both groups must approve for federation
- **Bridge Maintenance**: Proactively suggest cross-mesh connections when bridge density drops
- **Cross-Mesh Vouching**: After federation, members from Mesh-B can vouch for Mesh-A newcomers
- **Reciprocal Buffer**: Groups act as mutual buffers for each other

## Trust Model

### Vouching System
- **Requirement**: Two distinct Members must vouch for new member
- **Who Can Vouch**: ANY Member in the Stroma Signal group (Bridges and Validators)
- **Blind Matchmaker**: Bot suggests Validators for optimal intersectional diversity, but any Member can vouch
- **Vouches Source**: ONLY from Members already IN the Stroma Signal group
- **First Vouch**: Invitation itself counts as first vouch (no token exchange)
- **Verification**: `zk-Proof(Voucher_A IN Tree AND Voucher_B IN Tree AND Voucher_A != Voucher_B AND Invitee NOT_IN Tree)`
- **Privacy**: Vouchers remain anonymous (ZK-proof)
- **Admission**: Invitee added to Signal group ONLY after Freenet confirms 2 vouches

### Trust Standing & Ejection
- **Calculation**: `Standing = Effective_Vouches - Regular_Flags`
- **Vouch Invalidation**: If a voucher flags a member, that vouch is invalidated (logical inconsistency)
- **Ejection Triggers** (Two Independent Conditions):
  1. **Trigger 1**: `Standing < 0` (too many regular flags relative to effective vouches)
  2. **Trigger 2**: `Effective_Vouches < min_vouch_threshold` (default: 2, includes voucher-flagger invalidation)
- **No Grace Periods**: Immediate ejection, no warnings, no re-verification windows
- **Heartbeat Monitor**: Checks every 60 minutes, automatic ejection if either trigger met
- **Continuous Evaluation**: Trust monitored in real-time via Freenet state stream
- **Member Responsibility**: Cultivate multiple vouches for resilience
- **Re-Entry Path**: Secure 2 new vouches from Members IN the group, go through admission again
- **No Cooldown**: Can re-enter immediately after securing new vouches
- **No Public Shaming**: Bot uses hashes, not names in notifications

#### **Standing Math with Vouch Invalidation**

**Key Principle**: If a voucher flags you, their vouch is invalidated (they can't both trust and distrust you).

**Calculation**:
```
All_Vouchers = Set of members who vouched for you
All_Flaggers = Set of members who flagged you
Voucher_Flaggers = All_Vouchers ∩ All_Flaggers (contradictory)

Effective_Vouches = |All_Vouchers| - |Voucher_Flaggers|
Regular_Flags = |All_Flaggers| - |Voucher_Flaggers|
Standing = Effective_Vouches - Regular_Flags
```

**Examples**:

| All Vouches | All Flags | Voucher-Flaggers | Effective Vouches | Regular Flags | Standing | Trigger 1 | Trigger 2 | Result |
|-------------|-----------|------------------|-------------------|---------------|----------|-----------|-----------|---------|
| 2 (A,B) | 0 () | 0 | 2 | 0 | +2 | ✅ (≥0) | ✅ (2≥2) | **Stays** |
| 2 (A,B) | 1 (C) | 0 | 2 | 1 | +1 | ✅ (≥0) | ✅ (2≥2) | **Stays** (flagged by non-voucher) |
| 2 (A,B) | 1 (A) | 1 (A) | **1** | 0 | +1 | ✅ (≥0) | ❌ (1<2) | **EJECTED** (voucher flagged = invalidated vouch) |
| 3 (A,B,C) | 1 (A) | 1 (A) | **2** | 0 | +2 | ✅ (≥0) | ✅ (2≥2) | **Stays** (2 effective vouches remain) |
| 2 (A,B) | 2 (A,B) | 2 (A,B) | **0** | 0 | 0 | ✅ (≥0) | ❌ (0<2) | **EJECTED** (both vouchers flagged) |
| 2 (A,B) | 3 (A,C,D) | 1 (A) | **1** | 2 | -1 | ❌ (<0) | ❌ (1<2) | **EJECTED** (both triggers) |
| 3 (A,B,C) | 5 (D,E,F,G,H) | 0 | 3 | 5 | -2 | ❌ (<0) | ✅ (3≥2) | **EJECTED** (Trigger 1) |

**Key Insights**:
- Voucher-flaggers invalidate their own vouches (logical consistency)
- You need 2 effective vouches (after invalidation) to stay in group
- Standing is calculated from effective vouches and regular flags (excluding voucher-flaggers from both)
- The two triggers remain independent but now work with effective vouches

### Network Topology & Node Types

**Critical Distinction:**
- **Invitees (Leaf Nodes)**: OUTSIDE Signal group (1 vouch, being vetted)
- **Bridges**: IN Signal group (exactly 2 vouches, minimum requirement)
- **Validators**: IN Signal group (3+ vouches, high-trust members)

**Node Classification:**
- **Invitees (Leaf Nodes)**: 1 vouch, being vetted in "waiting room" (outside group)
- **Bridges**: 2 effective vouches (minimum threshold) - at risk if voucher leaves OR flags, but in group
- **Validators**: 3+ effective vouches - more resilient, used for Blind Matchmaker optimization

**Effective Vouches**: Total vouches minus voucher-flaggers (members who both vouched AND flagged you). See Trust Standing section for details.

**No Special Privileges:**
- Validators have no extra permissions
- ANY Member can vouch (Bridges and Validators)
- Validators are preferred by Blind Matchmaker for strategic introductions

**Threshold Configuration:**
- **Default**: Fixed numbers (min_vouch_threshold=2, validator_threshold=3)
- **Configurable**: Percentage-based (e.g., top 20% are Validators)
- **Dynamic**: Scales with group size when percentage-based

**Internal Clusters:**
- Sub-communities within SINGLE Stroma group based on social affinities
- Example: Artist cluster, Engineer cluster within 50-person group
- Bot identifies clusters for strategic cross-cluster introductions
- Blind Matchmaker suggests connections across clusters for intersectional diversity

**Minimum Spanning Tree (MST):**
- Goal: Full intersectional mesh with least new interactions
- Efficiency: N(Bridges_at_minimum) + I(Disconnected_Islands) = Total introductions needed
- Example: 20-person group with 5 Bridges + 2 islands = 6 strategic introductions

## Network Strength: Mesh Density

### What is Mesh Density?
Mesh density measures how fully connected the trust network is, expressed as a percentage.

**Calculation:**
```
Mesh Density % = (Actual Vouches / Max Possible Vouches) × 100

Where:
- Actual Vouches = Current number of vouches in the network
- Max Possible Vouches = n × (n - 1) for n members (full mesh)
```

**Example:**
- 47 members, 213 actual vouches
- Max possible: 47 × 46 = 2,162 vouches
- Density: 213 / 2,162 × 100 = **9.8%**

### Histogram Visualization

The `/mesh strength` command shows vouch distribution:

```
📈 Network Strength: 38% Mesh Density

Vouch Distribution:
  2 vouches (Bridges):    ████████░░░░ 22 members (47%)
  3-5 vouches:            █████░░░░░░░ 15 members (32%)
  6-10 vouches:           ███░░░░░░░░░  8 members (17%)
  11+ vouches (Validators): ██░░░░░░░░░░  2 members (4%)

Total: 47 members
Actual vouches: 213
Max possible: 552 (full mesh)
Density: 38%
```

### Interpretation
- **10-30%**: Minimal connections, efficient but fragile
- **30-60%**: Balanced - good resilience without over-connection (recommended)
- **60-90%**: Very resilient, but may indicate small or tightly-knit group
- **100%**: Full mesh (everyone trusts everyone) - rare, only in very small groups

## Federation Model

### Discovery Process
1. **Multiple Discovery URIs**: Bot generates URIs using percentile-based validator thresholds (not fixed N)
2. **Bloom Filter Broadcast**: Bot publishes encrypted summaries at multiple URIs
3. **Discovery Match**: Bots scan multiple URIs to find shared validator overlap
4. **PSI-CA Handshake**: Bots calculate exact overlap without revealing identities
5. **Threshold Exchange**: Bots exchange group sizes and threshold requirements
6. **BidirectionalMin Evaluation**: Both groups check if overlap meets their own threshold
7. **Federation Proposal**: If both thresholds satisfied, propose to respective groups

### Federation Decision
- **Human Control**: Members vote via Signal Poll on federation proposal
- **Consensus**: Uses `config_change_threshold` (e.g., 70% approval)
- **Independent Evaluation**: Each group evaluates based on their own threshold
- **Mutual Consent**: Both groups must approve for federation to proceed
- **Contract**: Bot signs federation contract on Freenet after both groups approve
- **Bridge Maintenance**: Bot proactively suggests connections if bridge density drops

### Cross-Mesh Vouching
- **Shadow-Vouch**: Member from Mesh-B can vouch for invitee to Mesh-A
- **Reciprocal Buffer**: Groups act as mutual buffers for each other
- **Fluid Movement**: Member's trust identity precedes them across meshes
- **Expedited Vetting**: Existing members of federated groups may have faster vetting

**Note**: Cross-mesh vouching is Phase 4+ functionality (out of MVP scope)

## Technical Stack

### Core Technologies
- **Language**: Rust 1.93+ (musl 1.2.5 with improved DNS resolver)
- **Static Binary**: x86_64-unknown-linux-musl
- **State Storage**: [freenet-core](https://github.com/freenet/freenet-core) (Rust-native, Wasm contracts, v0.1.107+)
- **Contract Framework**: [freenet-scaffold](https://github.com/freenet/freenet-scaffold) (ComposableState, summary-delta sync)
- **ZK-Proofs**: STARKs via winterfell (no trusted setup, post-quantum secure)
- **Identity Masking**: HMAC-SHA256 via ring crate
- **Memory Hygiene**: zeroize crate (immediate buffer purging)
- **Signal Integration**: libsignal-service-rs
- **Async Runtime**: tokio
- **Supply Chain Security**: cargo-deny, cargo-crev

### Node Architecture
- Each bot runs its own freenet-core node (no shared nodes)
- Bot operator provides Signal credentials
- Bot can recover state after going offline

### Contract Architecture
- **State Structures**: Set-based (BTreeSet, HashMap) for natural mergeability
- **Synchronization**: Summary-delta sync via ComposableState trait
- **Merkle Trees**: Generated on-demand from member sets (not stored in contract)
- **ZK-Proofs**: Validate state transitions (client-side or contract-side, TBD in Spike Week)
- **Merge Semantics**: CRDT-like eventual consistency (no consensus algorithms)

### Performance Targets
- **Scalability**: 10²-10³ (100x to 1000x) without revealing social graph
- **Latency**: Seconds to hours acceptable (security > speed)
- **Upper Bound**: Signal group limits (~1000 members per group)
- **Proof Size**: Target < 100KB per STARK proof
- **Proof Generation**: Target < 10 seconds

## Group Configuration

All group settings stored in Freenet contract, changeable only via group vote:

### GroupConfig Schema
```rust
pub struct GroupConfig {
    // Consensus thresholds
    config_change_threshold: f32,      // e.g., 0.70 (70%) - used for ALL decisions
    ejection_appeal_threshold: f32,    // e.g., 0.60 (60%)
    
    // Federation parameters
    min_intersection_density: f32,     // e.g., 0.10-0.30 (per-group, configurable)
    validator_percentile: u32,         // e.g., 20 (top 20%)
    
    // Trust parameters
    min_vouch_threshold: usize,        // Default: 2 (minimum vouches to stay in group)
    
    // Metadata
    config_version: u64,
    last_updated: Timestamp,
}
```

### Configuration Management
- **Single Consensus Threshold**: `config_change_threshold` used for all group decisions (configuration changes, federation proposals)
- **Voting Mechanism**: Signal Polls with structured options (✅ Approve / ❌ Reject / ⏸️ Abstain)
- **Automatic Application**: Bot updates Freenet config when vote exceeds threshold
- **Audit Trail**: All config changes logged with timestamps

### Bot Commands
- **Invitation & Vetting**:
  - `/invite @username [context]` - Invitation counts as first vouch
  - `/vouch @username` - Vouch for invitee or existing Member
  - `/flag @username [reason]` - Flag Member (reason required)

- **Status Queries**:
  - `/status` - View your own trust standing
  - `/status @username` - View another Member's standing
  - `/mesh` - Network overview (size, density, federation status)
  - `/mesh strength` - Mesh density histogram with vouch distribution
  - `/mesh config` - View current group configuration

- **Configuration**:
  - `/propose-config key=value [reason]` - Propose configuration change (triggers Signal Poll)

- **Audit**:
  - `/audit operator` - View operator action history (service operations only)

## Operator Role

The operator runs the bot as a **service** and performs ONE-TIME manual bootstrap. After bootstrap, operator has NO special privileges.

### Bootstrap Phase (One-Time)
**Operator Responsibilities:**
- Manually add 3 seed members to initial Signal group
- Initialize Freenet contract with mutual vouches
- Start bot service

### Post-Bootstrap (Ongoing)
**Operator Responsibilities:**
- Run and maintain bot service (systemd daemon or similar)
- Monitor logs for errors and system health
- Ensure bot stays online and connected to Freenet/Signal
- **No manual operations**: Operator CANNOT execute trust or membership commands

**Bot Automatic Execution:**
- Bot monitors Freenet state stream in real-time via event loop
- Bot automatically executes ALL actions approved by Freenet contract
- Bot automatically sends Signal Polls for proposals
- Bot automatically adds/removes members based on contract state
- Bot automatically facilitates vetting interviews
- Bot automatically suggests strategic introductions (Blind Matchmaker)
- Bot automatically initiates federation when both groups approve

**Operator CANNOT:**
- Override group decisions or bypass contract state
- Manually execute trust operations or membership changes
- Add members to Signal group (except 3-member bootstrap)
- Access cleartext Signal IDs or modify configuration
- Change GroupConfig without group vote via Signal Poll

## Security Model

### Anonymity-First
- **No Cleartext Storage**: All identifiers must be hashed
- **Zero-Knowledge**: Trust verified without revealing social graph
- **Private Set Intersection**: Overlap calculated without revealing identities
- **Metadata Leakage Prevention**: All operations in 1-on-1 PMs

### Trust Verification
- **ZK-Proofs Required**: All trust operations use zero-knowledge proofs
- **Freenet as Source of Truth**: Signal group state is derived, not authoritative
- **Immediate Enforcement**: No grace periods, instant ejection
- **Continuous Monitoring**: State stream monitored in real-time

### Attack Resistance
- **No Pre-Shared Keys**: Discovery is emergent, not coordinated
- **Sybil Resistance**: Dynamic validator threshold based on group size (percentile-based)
- **Social Anchor Security**: Federation dissolves if shared members leave
- **Minimal Attack Surface**: Static MUSL binary, seccomp sandbox
- **Operator Least Privilege**: Operator cannot override group consensus or modify config (see Operator Role section)
- **Independent Thresholds**: Each group controls their own federation criteria (BidirectionalMin)
- **Asymmetric Safety**: Small groups can require higher overlap to avoid absorption
- **No Vouch Restrictions**: ANY Member can vouch (not restricted to Validators)

## Development

### Prerequisites
- **Rust 1.93+** (required for musl 1.2.5 with improved DNS resolver)
- Signal account for bot (phone number required)
- freenet-core node (https://github.com/freenet/freenet-core)
- MUSL toolchain for static binary

**Why Rust 1.93+:**
- Bundled musl 1.2.5 with major DNS resolver improvements
- More reliable networking for Signal and freenet-core
- Better handling of large DNS records and recursive name servers
- See: [Rust 1.93 Release](https://www.infoworld.com/article/4120988/rust-1-93-updates-bundled-musl-library-to-boost-networking.html)

### Setup
```bash
# Ensure Rust 1.93+ is installed
rustup update stable
rustc --version  # Should show 1.93+

# Add MUSL target
rustup target add x86_64-unknown-linux-musl

# Install freenet-core
git clone https://github.com/freenet/freenet-core.git
cd freenet-core
cargo install --path crates/core

# Clone Stroma
git clone https://github.com/roder/stroma.git
cd stroma
```

### Build
```bash
# Development build
cargo build

# Production build (static MUSL binary)
cargo build --release --target x86_64-unknown-linux-musl
```

### Testing
```bash
# Run all tests
cargo nextest run

# Run with coverage
cargo llvm-cov nextest
```

### Security Audits
```bash
# Check dependencies for vulnerabilities
cargo deny check

# Verify crate authenticity
cargo crev verify
```

### Run Bot
```bash
# Start freenet-core node
freenet &

# Run Stroma bot
./target/release/stroma --config config.toml
```

## Implementation Roadmap

### MVP Scope
**Objective**: Single group with full trust mechanics (no federation in MVP)

**Critical**: All design decisions optimize for future federation (Phase 4+), but MVP implements single-group functionality only.

### Spike Week (Week 0 - Validation Phase)
**Before full implementation, validate core technologies:**

1. **freenet-core Integration & ComposableState** (2 days)
   - Install and run freenet-core node locally
   - Install freenet-scaffold and implement ComposableState trait
   - Test set-based membership (BTreeSet) with merge semantics
   - Test on-demand Merkle Tree generation (benchmark 10-1000 members)
   - Deploy contract with ComposableState to freenet-core
   - Test state stream monitoring (real-time updates)
   - **Answer 5 critical questions** about contract design (see Outstanding Questions below)

2. **Signal Bot Registration** (1 day)
   - Register bot account with Signal (phone number)
   - Test group management (create group, add/remove members)
   - Test 1-on-1 PM handling and command parsing
   - Validate: Can we automate admission/ejection?

3. **STARK Proof Generation** (2 days)
   - Generate sample STARK proof with winterfell
   - Proof circuit: "2 vouches from different Members verified"
   - Measure proof size (target: < 100KB) and generation time (target: < 10 seconds)
   - Validate: Are proofs practical for our use case?

**Deliverable**: Go/No-Go decision report with technology validation

#### **Outstanding Questions (MUST Resolve in Spike Week)**

**Critical**: These questions fundamentally affect contract architecture and MUST be answered before Phase 0:

1. **STARK Verification in Wasm**: Can we verify STARK proofs in contract verify() without performance issues?
   - Test: Compile winterfell to Wasm, measure verification time
   - Target: < 100ms per proof
   - Decision: Client-side vs contract-side verification

2. **Proof Storage Strategy**: Should we store STARK proofs in contract state or just outcomes?
   - Options: Temporary storage, permanent storage, no storage
   - Impact: Storage costs, audit trail, trustlessness

3. **Merkle Tree Performance**: How expensive is on-demand Merkle Tree generation from BTreeSet?
   - Test: Benchmark with 10, 100, 500, 1000 members
   - Target: < 100ms for 1000 members
   - Decision: On-demand generation vs caching Merkle root

4. **Conflict Resolution**: How does Freenet handle conflicts when two nodes submit incompatible updates?
   - Test: Create divergent states, attempt merge
   - Document: Freenet's conflict resolution behavior
   - Impact: May need vector clocks or causal ordering

5. **Custom Validation**: Can we enforce complex invariants (e.g., "every member >= 2 vouches") in verify()?
   - Test: Implement complex validation in verify() method
   - Document: Contract API limitations
   - Decision: Contract-enforced vs bot-enforced invariants

### Phase 0: Foundation (Weeks 1-2)
**Focus**: Core infrastructure with federation-ready design

- **Kernel**: HMAC identity masking with immediate zeroization
- **Freenet Integration**: freenet-core node management, state stream monitoring
- **Signal Integration**: Bot authentication, group management, command parsing
- **Crypto Layer**: STARK circuits for vouch verification (winterfell)
- **Contract Schema**: ComposableState-based (set-based membership, vouch graph, flags)
- **Merkle Tree**: On-demand generation from BTreeSet for ZK-proofs

**Key Decisions from Spike Week**:
- Set-based membership (not Merkle Tree as primary storage)
- ComposableState trait for mergeable state
- Summary-delta synchronization for eventual consistency
- Answers to 5 outstanding questions about contract design

**Deliverable**: Working foundation modules with validated contract design

### Phase 1: Bootstrap & Core Trust (Weeks 3-4)
**Focus**: Seed group, vetting, admission, ejection

- **Bootstrap Module**: 3-member seed group with initial triangle vouching
- **Trust Operations**: Invitation (first vouch), second vouch selection, admission, ejection
- **Basic Commands**: `/invite`, `/vouch`, `/flag`, `/status`
- **Health Monitoring**: Continuous state stream, automatic ejection (two triggers)

**Deliverable**: Working single-group bot with core trust mechanics

### Phase 2: Internal Mesh Optimization (Weeks 5-6)
**Focus**: Graph analysis, strategic introductions, MST

- **Blind Matchmaker**: Graph topology analysis, cluster identification, strategic suggestions
- **Advanced Commands**: `/mesh`, `/mesh strength`, `/mesh config`, `/propose-config`
- **Configuration Management**: Signal Polls for voting, automatic config updates
- **Operator Audit**: `/audit operator` command

**Deliverable**: Optimized single-group bot with full command set

### Phase 3: Federation Preparation (Week 7)
**Focus**: Validate federation infrastructure (compute locally, no broadcast)

- **Shadow Beacon**: Social Anchor hashing, validator percentile calculation, discovery URI generation (not broadcast)
- **PSI-CA**: Bloom filter generation, commutative encryption, intersection density calculation (test locally)
- **Contract Validation**: Test federation hooks (unused but present in schema)
- **Documentation**: Federation design document, Phase 4+ roadmap

**Deliverable**: MVP ready for production + validated federation path

### Phase 4+: Federation (Future - Out of MVP Scope)
**North Star for all design decisions, but not implemented in MVP**

- **Emergent Discovery**: Shadow Beacon broadcast, multiple discovery URIs
- **PSI-CA Protocol**: Anonymous overlap calculation, BidirectionalMin evaluation
- **Federation Voting**: Signal Polls, mutual consent (both groups vote)
- **Cross-Mesh Vouching**: Federated Merkle Trees, shadow vouches, expedited vetting

**Note**: Federation is the ultimate objective - all architecture decisions optimize for it.

## Design Philosophy

### Trust as Emergent Property
- Trust **mutually arises** across the network through social relationships
- No central authority controls access or membership
- The network's "Self" persists even as individual members come and go
- Trust is relational, not hierarchical

### Mutual Arising
- Groups discover each other through **emergent discovery**, not pre-coordination
- Bots find each other because they share a "social frequency," not because admins exchanged passwords
- Federation occurs organically when groups share trusted members
- The network scales as a "single, coherent organism rather than fragmented silos"

### Fluid Identity
- Presence in the group is **temporary permission** arising from current trust balance
- Members can be immediately ejected when trust threshold is violated
- No grace periods - trust is continuously evaluated
- Identity is fluid and relational, not fixed

## License

[To be determined]

## Contributing

[To be determined]

---

**Note**: This project is in early development. The architecture and implementation details are subject to change as the system evolves.
