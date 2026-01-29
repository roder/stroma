# Stroma

**Signal groups where trust is verified by many, exposed to none.**

## Mission

Build federations of human networks connected by trust while preserving individual anonymity.

## Goals

- Create a Signal Messenger bot that maintains groups with fully-vetted members
- Use minimal real-world vouches to create resilient mesh networks of trust
- Bridge disparate groups that have overlapping trusted members for novel connections across trust networks 

## The Problem

Activists are subject to repression and repression prevents coordination. How can an activist know the members of a Signal group can be trusted?  

On the other hand, anonymity is security, but how can the group know they can trust you?

This creates a fundamental tension: **verification requires exposure, but exposure creates vulnerability.**

Traditional solutions create new problems:
- **Invite links**: Anyone with the link can join - no vetting, no trust verification
- **Admin gatekeepers**: One person controls who gets in - single point of failure, creates hierarchy
- **Trusting strangers**: Members join without vetting - how do you know they won't leak the group or infiltrate it?
- **Large groups become cliques**: Friend circles form, newcomers isolated on the periphery

## How Stroma Solves This

Stroma resolves the tension between verification and anonymity through **distributed trust verification with cryptographic privacy**. 

**The core principle**: You can only join if two members from **different parts of the network** vouch for you, while ensuring member identities are protected even if an adversary seizes the server.

### What This Means:
- **No strangers**: Every member is personally vouched for by at least 2 people already in the group
- **No gatekeepers**: No single person controls entry - trust is distributed across the network
- **No cliques**: Vouches must come from different friend circles (not your buddy vouching for your other buddy)
- **No hierarchy**: Trust emerges from relationships, not authority or admin power
- **No identity exposure**: Even if the bot server is compromised, the adversary only gets cryptographic hashes — not real identities

### How It Works (Simple):

1. **Someone invites you**: A member sends `/invite @YourName "Context about you"` to the bot (private message only)
   - Their invitation counts as your first vouch
   - Bot hashes your Signal ID immediately (never stores cleartext)
   - Vetting process begins

2. **You get vetted**: The bot suggests you chat with a second member from a different part of the network
   - Bot facilitates introduction (suggests a well-connected member from a different friend circle)
   - You have a brief conversation to establish trust
   - Bot doesn't participate - just makes strategic matchmaking suggestion

3. **Second vouch**: After your conversation, the member vouches for you
   - They send `/vouch @YourName` to the bot (private message)
   - Bot verifies: (a) voucher is a member, (b) voucher is from a **different friend circle** than the inviter
   - Same friend circle vouches are rejected — diversity is mandatory to prevent coordinated infiltration
   - **Bootstrap exception**: For small groups (3-5 members) where everyone knows each other, diversity requirement is suspended

4. **You're admitted**: The bot adds you to the Signal group
   - You're now a Bridge (2 effective vouches from members in different friend circles)
   - Your trust standing is positive (Standing = Effective_Vouches - Regular_Flags >= 0)
   - Bot welcomes you and immediately deletes all vetting session data (ephemeral)

5. **You stay connected**: Keep building relationships in the group
   - If a voucher leaves → you need a replacement vouch immediately (automatic ejection otherwise)
   - If a voucher flags you → their vouch is invalidated, you need a replacement (vouch invalidation)
   - Build 3+ connections to become a Validator (more resilient, helps with network optimization)

### The Magic: Trust Map Protection

The bot acts as a **"Blind Matchmaker"** - it optimizes the trust mesh using graph algorithms while maintaining complete anonymity. The critical innovation: **the trust map never exists in any form that could be seized or exposed.**

**For non-technical users**: It feels like a helpful bot managing your Signal group. You use simple commands like `/invite @friend` or `/status` in private messages. The bot handles everything automatically - vetting newcomers, monitoring trust standing, suggesting strategic introductions, and keeping the group secure. All technical complexity is hidden. You don't need to understand cryptography any more than you need to understand TCP/IP to use the internet securely. But you can - this project is fully open-source.
 
**For privacy advocates & security auditors**: **Trust map protection** via three independent defense layers:
1. **No centralized storage**: Trust map in decentralized Freenet network (distributed across peers, no single seizure point)
2. **Cryptographic privacy**: All identities hashed (HMAC-SHA256 with group-secret pepper), trust verified with ZK-proofs (STARKs), memory zeroized, minimal protocol-only store (~100KB encrypted file, NO message history)
3. **Metadata isolation**: All vetting in 1-on-1 PMs (no Signal group metadata), bot operator least-privilege (service runner only), vetting conversations ephemeral (never persisted to disk)
Together: **Even if adversary seizes the bot server, they get: small encrypted file (~100KB protocol state), hashes (not identities), topology (not relationship context), NO vetting conversations, NO message history.**

**For developers & contributors**: Built on embedded [freenet-core](https://github.com/freenet/freenet-core) kernel (in-process, not external service). Contracts use ComposableState trait for mergeable state with CRDT-like semantics. Set-based membership (BTreeSet) with on-demand Merkle Tree generation for ZK-proof verification (not stored). Internal matchmaking uses Minimum Spanning Tree algorithm (Union-Find, betweenness centrality) achieving optimal mesh with maximum anonymity. External federation via Private Set Intersection with Cardinality (PSI-CA) and Social Anchor Hashing (emergent discovery, no pre-shared keys). Trust verified via STARKs (winterfell). See [ALGORITHMS.md](docs/ALGORITHMS.md) for MST implementation, [freenet-contract-design.mdc](.cursor/rules/freenet-contract-design.mdc) for patterns.

## Why "Stroma"?

In biology, stroma is the supportive tissue that holds organs together. In your group, Stroma is the underlying trust network that holds the community together - invisible but essential.

---

## Documentation Guide

Stroma serves three audiences. Choose your path:

### 👥 For End Users (Group Members)
**You want to use Stroma in your Signal group.**

- **[How It Works](docs/HOW-IT-WORKS.md)** - **Start here:** Plain-language explanation of the trust protocol
- **[User Guide](docs/USER-GUIDE.md)** - Bot commands, daily workflows, trust management
- **[Trust Model Explained](docs/TRUST-MODEL.md)** - How vouching, flagging, and standing work
- **Quick Start**: Install Signal → Get invited by a member → Chat with validator → Join group

### 🔧 For Operators (Bot Runners)
**You want to run a Stroma bot for your community.**

- **[Operator Guide](docs/OPERATOR-GUIDE.md)** - Installation, configuration, maintenance
- **[Spike Week Briefing](docs/SPIKE-WEEK-BRIEFING.md)** - Technology validation checklist
- **Prerequisites**: Linux server, Signal account, Rust 1.93+, freenet-core

### 💻 For Developers (Contributors)
**You want to understand or contribute to Stroma.**

- **[Developer Guide](docs/DEVELOPER-GUIDE.md)** - Architecture, technical stack, contract design
- **[Algorithms](docs/ALGORITHMS.md)** - **MST matchmaking, PSI-CA federation, complexity analysis**
- **[Trust Model Technical](docs/TRUST-MODEL.md)** - Vouch invalidation, ejection triggers, mesh health
- **[Federation Roadmap](docs/FEDERATION.md)** - Phase 4+ vision and design
- **[TODO Checklist](docs/TODO.md)** - Implementation roadmap and task tracking

---

## Quick Overview

Stroma provides three layers that work together seamlessly:

### 1. Signal Bot Interface
Members interact via simple commands: `/invite`, `/vouch`, `/flag`, `/propose`, `/status`, `/mesh`

**What members see**: Natural language responses, trust standing, network health, anonymous voting  
**What's hidden**: All crypto, Freenet state, Merkle trees, ZK-proofs, individual votes

→ **[Complete User Guide](docs/USER-GUIDE.md)** - Commands, workflows, examples

### 2. Trust Logic (Rust Bot)
- **Architecture**: One bot per group (1:1 relationship)
- **Implementation**: Presage (high-level Rust API wrapping libsignal-service-rs)
- **Protocol Gatekeeper**: Enforces 2-vouch requirement with ZK-proofs
- **Blind Matchmaker**: Suggests strategic introductions across different friend circles  
- **Health Monitor**: Continuous trust standing checks via Freenet state stream
- **Consensus Enforcer**: Executes only contract-approved actions (no autonomous decisions)
- **Diplomat**: Discovers and proposes federation (Phase 4+)

→ **[Trust Model Deep Dive](docs/TRUST-MODEL.md)** - Vouching, flagging, ejection math

### 3. Decentralized State (Embedded Freenet Kernel)
- **Embedded kernel**: Runs in-process with bot (single binary, no external service)
- **No central server**: State exists across peer-to-peer network
- **Eventual consistency**: Summary-delta synchronization (no consensus algorithms)
- **Set-based membership**: BTreeSet with on-demand Merkle Tree generation
- **Anonymous routing**: Dark mode (no IP exposure)

→ **[Developer Guide](docs/DEVELOPER-GUIDE.md)** - Architecture, contract design, tech stack

---

## Core Concepts

### Trust Model
- **Requirement**: 2 vouches from members in **different friend circles** to join (diversity mandatory to prevent coordinated infiltration)
- **Bootstrap Exception**: Small groups (3-5 members) where everyone knows each other; diversity enforced once multiple friend circles exist
- **Standing**: `Effective_Vouches - Regular_Flags` (must stay ≥ 0)
- **Vouch Invalidation**: If voucher flags you, their vouch is invalidated
- **Ejection**: Immediate when Standing < 0 OR Effective_Vouches < 2
- **Re-entry**: Get 2 new vouches, no cooldown

→ **[Full Trust Model](docs/TRUST-MODEL.md)** with examples and edge cases

### Network Health: Distinct Validator Ratio (DVR)
Network health is measured by **Distinct Validator Ratio** — the fraction of maximum possible Validators with non-overlapping voucher sets. This graph-theory-grounded metric directly measures resilience against coordinated attacks.

**Formula**: `DVR = Distinct_Validators / (N / 4)` where N = network size

**Three-tier health status (thirds)**:
- 🔴 **Unhealthy** (0-33%): Trust concentrated — actively suggest improvements
- 🟡 **Developing** (33-66%): Growing toward optimal
- 🟢 **Healthy** (66-100%): **THE GOAL** - strong distributed trust

### Federation (Phase 4+ - Future)
- **Emergent discovery**: Bots find each other via shared validators
- **Human control**: Both groups vote to approve
- **Private overlap**: PSI-CA reveals only count, not identities
- **Cross-mesh vouching**: Members vouch across federated groups
- **Shadow Handover**: Bot identity rotation with cryptographic succession

→ **[Federation Roadmap](docs/FEDERATION.md)** - North star design

## Project Status

### MVP Scope (Phase 0-3)
✅ Single-group trust network with full security model  
✅ All bot commands and mesh optimization  
❌ Federation (Phase 4+ - designed but not implemented)

### Federation as North Star
Even though federation isn't in MVP, **every design decision optimizes for it**:
- Contract schema federation-ready
- Identity hashing re-computable
- Module structure includes `federation/` (disabled)

**Goal**: Connect as many people as possible anonymously via trust

→ **[Implementation Roadmap](docs/TODO.md)** - Phased development plan

## Technical Architecture

### Core Innovation: Recursive ZK-Vouching
- **Embedded Freenet Kernel**: In-process (single binary, no external service)
- **Zero-Knowledge Proofs**: Verify trust without revealing who vouched (STARKs)
- **Set-Based State**: BTreeSet membership with on-demand Merkle Trees
- **Mergeable Contracts**: CRDT-like eventual consistency (no consensus algorithms)
- **Vouch Invalidation**: Voucher-flaggers cancel their own vouches (logical consistency)

### Three Layers (Single Binary)
1. **Signal** - Human interface (bot commands, 1-on-1 PMs, conversational)
2. **Rust Bot** - Trust logic (gatekeeper, matchmaker, health monitor, diplomat)
3. **Embedded Freenet** - Decentralized state (in-process kernel, ComposableState, anonymous routing)

→ **[Technical Deep Dive](docs/DEVELOPER-GUIDE.md)** - Architecture, modules, contracts

---

_For detailed specifications on Trust Model, Mesh Health, Federation, Technical Stack, Configuration, Security, and Implementation Phases, see the documentation links above._

---

## Technical Stack

| Component | Technology | Why |
|-----------|------------|-----|
| **Language** | Rust 1.93+ | musl 1.2.5, improved DNS, static binaries |
| **Embedded Kernel** | freenet-stdlib v0.1.30+ | In-process, decentralized, anonymous |
| **Contracts** | freenet-scaffold v0.2+ | ComposableState, summary-delta sync |
| **ZK-Proofs** | STARKs (winterfell) | No trusted setup, post-quantum |
| **Identity** | HMAC-SHA256 (ring) | Group-scoped hashing |
| **Signal (high-level)** | Presage | High-level Rust API, group management, polls |
| **Signal (low-level)** | libsignal-service-rs (FORK) | Protocol v8 poll support via our fork |
| **Voting** | Native Signal Polls | Anonymous voting (protocol v8) |
| **CLI** | clap 4+ | Operator commands |

→ **[Full Technical Stack](docs/DEVELOPER-GUIDE.md)** - Architecture, contracts, performance targets

## Security Principles

- **Anonymity-First**: All identifiers hashed (HMAC), immediate zeroization
- **Zero-Knowledge**: Trust verified via STARKs without revealing social graph
- **Freenet as Truth**: Signal state derived from decentralized contract
- **Operator Least Privilege**: Service runner only, no override powers
- **Immediate Ejection**: No grace periods when trust threshold violated

→ **[Security Model](docs/DEVELOPER-GUIDE.md#security)** - Threat model, attack resistance

## Getting Started

### For Operators

**Container (Recommended - Easiest):**
```bash
docker run -d -v stroma-data:/data ghcr.io/roder/stroma:latest
```

**Static Binary (Maximum Security):**
```bash
wget https://github.com/roder/stroma/releases/download/v1.0.0/stroma
gpg --verify stroma.asc && chmod +x stroma && ./stroma run
```

Both methods use the **same secure static binary** (container just wraps it for ease).

→ **[Operator Guide](docs/OPERATOR-GUIDE.md)** - Complete installation, bootstrap, maintenance

### For Developers

```bash
# Clone and build (includes embedded Freenet kernel)
git clone https://github.com/roder/stroma.git
cd stroma
cargo build --release --target x86_64-unknown-linux-musl

# Binary includes everything - no external freenet-core needed
./target/x86_64-unknown-linux-musl/release/stroma --help
```

→ **[Developer Guide](docs/DEVELOPER-GUIDE.md)** - Architecture, testing, contributing  
→ **[Spike Week](docs/SPIKE-WEEK-BRIEFING.md)** - Technology validation checklist

## Implementation Roadmap

### Current Phase: Protocol v8 Poll Support (Priority)
**Agent-Signal implements protocol v8 poll support in forked libsignal-service-rs**

**Why Critical:**
- Native Signal Polls provide anonymous voting (reactions expose voters)
- Required for `/propose` system (Phase 3)
- Blocks proposal system implementation

**Timeline**: 1-2 weeks

**Next Phase: Spike Week (Technology Validation)**

**Key Validations:**
- Freenet Dark with ComposableState trait
- Presage group management and poll support
- STARK proofs (size, performance)
- Friend circle detection for diversity requirement (bootstrap exception)
- Answer 6 critical architecture questions

**Deliverable**: Go/No-Go decision report

→ **[Spike Week Briefing](docs/SPIKE-WEEK-BRIEFING.md)** - Day-by-day test plans and questions

### Development Phases
- **Phase -1** (Weeks 1-2): Protocol v8 Polls (Agent-Signal priority task)
- **Phase 0** (Weeks 1-2): Spike Week + Foundation (Kernel, Freenet, Signal, Crypto, Contract)
- **Phase 1** (Weeks 3-4): Bootstrap & Core Trust (Vetting, admission, ejection)
- **Phase 2** (Weeks 5-6): Proposals & Mesh (Anonymous voting, graph analysis)
- **Phase 3** (Week 7): Federation Prep (Validate design, don't broadcast)
- **Phase 4+** (Future): Federation (Emergent discovery, cross-mesh vouching)

→ **[Complete TODO Checklist](docs/TODO.md)** - 390+ implementation tasks

## Design Philosophy

### Trust as Emergent Property
Trust **mutually arises** across the network through relationships, not central authority.

### Fluid Identity
Membership is **temporary permission** from current trust balance. Ejection is immediate when threshold violated - no grace periods.

### Mutual Arising
Groups discover each other via **emergent discovery** (shared validators), not admin coordination. The network scales as a coherent organism.

→ **[Philosophy & Principles](docs/FEDERATION.md)** - Deep dive on design values

---

## Contributing

This project uses:
- **Gastown coordination** - Claude agents with specialized roles
- **Beads** - Immutable design constraints (see `.beads/`)
- **Rules** - Development standards (see `.cursor/rules/`)

**All commits by Claude agents must include:**
```
Co-authored-by: Claude <noreply@anthropic.com>
```

→ **[AGENTS.md](AGENTS.md)** - Agent coordination model  
→ **[TODO.md](docs/TODO.md)** - Current tasks and progress

## License

[To be determined]

---

**Status**: Architectural foundation complete. Gastown landing zone ready. Next: Agent-Signal implements protocol v8 poll support.

**Last Updated**: 2026-01-28
