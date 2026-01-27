# Stroma

**Secure group messaging where trust is earned, not granted.**

## Mission

Build federations of human networks connected by trust while preserving individual anonymity.

## Goals

- Create a Signal Messenger bot that maintains groups with fully-vetted members
- Use minimal real-world vouches to create resilient mesh networks of trust
- Bridge disparate groups that have overlapping trusted members for novel connections across trust networks 

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

**For non-technical users**: It just feels like a helpful bot that manages your Signal group. You use simple commands like `/invite @friend` or `/status` in private messages with the bot. The bot handles everything else automatically - vetting newcomers, monitoring trust, and keeping the group secure. You don't need to understand the technical details to use it, any more than you need to understand the technical details of the internet for it to be secure and valuable. You can if you want though, this project is fully open-source.
 
**For privacy advocates**: All identities are cryptographically hashed (HMAC-SHA256 with group-secret pepper), trust is verified with zero-knowledge proofs (STARKs - no trusted setup, post-quantum secure), state is stored in decentralized Freenet network with eventual consistency (ComposableState, summary-delta sync), and the social graph is never exposed.

**For developers**: Built on [freenet-core](https://github.com/freenet/freenet-core) (Rust-native Wasm contracts with ComposableState trait). Uses set-based membership (BTreeSet) with on-demand Merkle Tree generation for ZK-proof verification. State synchronizes via summary-delta protocol with CRDT-like merge semantics (no consensus algorithms). Trust verified via STARKs (winterfell library). See [freenet-contract-design.mdc](.cursor/rules/freenet-contract-design.mdc) for patterns.

## Why "Stroma"?

In biology, stroma is the supportive tissue that holds organs together. In your group, Stroma is the underlying trust network that holds the community together - invisible but essential.

---

## Documentation Guide

Stroma serves three audiences. Choose your path:

### 👥 For End Users (Group Members)
**You want to use Stroma in your Signal group.**

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
- **[Trust Model Technical](docs/TRUST-MODEL.md)** - Vouch invalidation, ejection triggers, mesh health
- **[Federation Roadmap](docs/FEDERATION.md)** - Phase 4+ vision and design
- **[TODO Checklist](docs/TODO.md)** - Implementation roadmap and task tracking

---

## Quick Overview

Stroma provides three layers that work together seamlessly:

### 1. Signal Bot Interface
Members interact via simple commands: `/invite`, `/vouch`, `/flag`, `/status`, `/mesh`

**What members see**: Natural language responses, trust standing, network health  
**What's hidden**: All crypto, Freenet state, Merkle trees, ZK-proofs

→ **[Complete User Guide](docs/USER-GUIDE.md)** - Commands, workflows, examples

### 2. Trust Logic (Rust Bot)
- **Protocol Gatekeeper**: Enforces 2-vouch requirement with ZK-proofs
- **Blind Matchmaker**: Suggests strategic introductions across clusters  
- **Health Monitor**: Continuous trust standing checks via Freenet state stream
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
- **Requirement**: 2 vouches from independent Members to join
- **Standing**: `Effective_Vouches - Regular_Flags` (must stay ≥ 0)
- **Vouch Invalidation**: If voucher flags you, their vouch is invalidated
- **Ejection**: Immediate when Standing < 0 OR Effective_Vouches < 2
- **Re-entry**: Get 2 new vouches, no cooldown

→ **[Full Trust Model](docs/TRUST-MODEL.md)** with examples and edge cases

### Mesh Health Score
Rather than raw density %, we show **Mesh Health** (0-100) that peaks at optimal 30-60% density:
- 🔴 **Fragile** (0-10%): Minimal connections
- 🟡 **Building** (10-30%): Developing
- 🟢 **Optimal** (30-60%): **THE GOAL** - balanced resilience
- 🟡 **Dense** (60-90%): Over-connected  
- 🔴 **Saturated** (90-100%): Excessive interdependence

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
| **Interface** | Signal (libsignal-service-rs) | Familiar UX, E2E encrypted |
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

### Current Phase: Spike Week (Week 0)
**Validate core technologies before committing to implementation:**

**Key Validations:**
- freenet-core with ComposableState trait
- Signal bot automation (add/remove members)  
- STARK proofs (size, performance)
- Answer 5 critical architecture questions

**Deliverable**: Go/No-Go decision report

→ **[Spike Week Briefing](docs/SPIKE-WEEK-BRIEFING.md)** - Day-by-day test plans and questions

### Development Phases
- **Phase 0** (Weeks 1-2): Foundation (Kernel, Freenet, Signal, Crypto, Contract)
- **Phase 1** (Weeks 3-4): Bootstrap & Core Trust (Vetting, admission, ejection)
- **Phase 2** (Weeks 5-6): Mesh Optimization (Blind Matchmaker, graph analysis)
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
Co-authored-by: Claude <claude@anthropic.com>
```

→ **[AGENTS.md](AGENTS.md)** - Agent coordination model  
→ **[TODO.md](docs/TODO.md)** - Current tasks and progress

## License

[To be determined]

---

**Status**: Early development (Spike Week). Architecture validated through comprehensive documentation. Ready for technology validation phase.

**Last Updated**: 2026-01-27
