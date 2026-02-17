# Stroma

> **Pre-alpha**: Architecture validated through two spike weeks (14 questions resolved). Mock-layer logic complete (~31,600 lines, 502+ tests). Real Signal and Freenet integration in progress. Not yet functional against live networks.

**Signal groups where trust is verified by many, exposed to none.**

## The Problem

Activists are subject to repression and repression prevents coordination. How can an activist know the members of a Signal group can be trusted? And how can the group know they can trust you?

**Verification requires exposure, but exposure creates vulnerability.**

Traditional solutions create new problems:
- **Invite links**: Anyone with the link can join -- no vetting, no trust
- **Admin gatekeepers**: One person controls who gets in -- single point of failure, creates hierarchy
- **Trusting strangers**: Members join without vetting -- no way to detect infiltration
- **Large groups become cliques**: Peer circles form, newcomers isolated at the periphery

## How Stroma Solves This

**The core principle**: You can only join if two members from **different parts of the network** vouch for you, while ensuring member identities are protected even if an adversary seizes the server.

- **No strangers**: Every member is personally vouched for by at least 2 people in the group
- **No gatekeepers**: No single person controls entry -- trust is distributed across relationships
- **No cliques**: Vouches must come from different peer circles (cross-cluster requirement)
- **No hierarchy**: Trust emerges from relationships, not authority
- **No identity exposure**: Even if the bot server is seized, the adversary gets cryptographic hashes -- not identities

### How It Works

1. **Someone invites you**: A member sends `/invite @YourName` to the bot (private message). Their invitation counts as your first vouch. The bot hashes your Signal ID immediately and never stores the original.

2. **You get vetted**: The bot selects a member from a different part of the network to assess you. This assessor reaches out to you independently -- the bot only facilitates the introduction.

3. **Second vouch**: After meeting you, the assessor vouches for you. The bot verifies the voucher is from a different peer circle than the inviter. Same-circle vouches are rejected.

4. **You're admitted**: The bot adds you to the Signal group. You're a Bridge (2 vouches from different clusters). All vetting session data is deleted immediately.

5. **Trust is continuous**: If a voucher leaves or flags you and your effective vouches drop below 2, you're removed immediately. No grace periods. But you can always re-enter by getting 2 new cross-cluster vouches -- no permanent bans.

### Trust Map Protection

The bot knows the trust graph, but that graph **never exists in a form that could be seized**:

1. **Decentralized storage**: Trust state lives in Freenet's peer-to-peer network -- no single server to raid
2. **Cryptographic privacy**: All identities masked via HMAC-SHA256 with operator-derived keys (BIP-39 mnemonic, HKDF-SHA256 key derivation). Memory zeroized after hashing. Encrypted SQLite databases with no message history stored.
3. **Metadata isolation**: All vetting happens in 1-on-1 PMs. Bot operator has no special privileges. Vetting conversations are ephemeral.

**If an adversary seizes the bot server**: they get encrypted databases, cryptographic hashes (not identities), and no message history.

## Why "Stroma"?

In biology, stroma is the supportive tissue that holds organs together -- invisible but essential. In your group, Stroma is the trust network that holds the community together.

---

## Current Status

| Layer | Status | Detail |
|-------|--------|--------|
| **Trust model** | Complete | Standing, vouching, flagging, ejection, cross-cluster enforcement, DVR health, persistence |
| **Signal integration** | In progress | Polls UAT-validated on live Signal. Group CRUD, message routing in progress. |
| **Freenet integration** | Mocked | Architecture validated. In-memory mock; embedded kernel blocked on upstream visibility fix. |
| **ZK-proofs** | Scaffolded | Winterfell AIR circuits defined. Phase 0 uses simplified hash-based commitments. Full STARK proving planned. |
| **Federation** | Designed | Social anchor computation works locally. Discovery, PSI-CA, cross-mesh vouching are Phase 4+. |
| **Persistence** | Complete (mock) | Reciprocal network architecture validated. Chunking, encryption, registry, rendezvous hashing all implemented against mocks. |

**Next milestone**: UAT -- bot links to real Signal account, creates group, runs full admission flow against live networks.

See [TODO.md](docs/todo/TODO.md) for the complete implementation checklist.

---

## Documentation

### For End Users
- **[How It Works](docs/HOW-IT-WORKS.md)** -- Plain-language explanation of the trust protocol
- **[User Guide](docs/USER-GUIDE.md)** -- Bot commands, daily workflows, trust management
- **[Trust Model](docs/TRUST-MODEL.md)** -- Vouching, flagging, standing, ejection

### For Operators
- **[Operator Guide](docs/OPERATOR-GUIDE.md)** -- Installation, configuration, maintenance

### For Security Researchers & Auditors
- **[Threat Model](docs/THREAT-MODEL.md)** -- Primary threat (trust map seizure), three-layer defense, secondary threats, accepted risks
- **[Security CI/CD](docs/SECURITY-CI-CD.md)** -- Automated security checks: supply chain (cargo-deny), static analysis (CodeQL), coverage enforcement, unsafe block detection
- **[Threat Model Audit](docs/THREAT-MODEL-AUDIT.md)** -- Audit findings and verification results
- **[Trust Model](docs/TRUST-MODEL.md)** -- Standing formula, ejection triggers, vouch invalidation math
- **[Vouch Invalidation Logic](docs/VOUCH-INVALIDATION-LOGIC.md)** -- Why a single member's action can never cause a 2-point swing
- **[Algorithms](docs/ALGORITHMS.md)** -- DVR calculation, cluster detection (Tarjan's), complexity analysis
- **Architectural constraints**: `.beads/security-constraints.bead`, `.beads/philosophical-foundations.bead`

### For Developers
- **[Developer Guide](docs/DEVELOPER-GUIDE.md)** -- Architecture, modules, contracts
- **[Algorithms](docs/ALGORITHMS.md)** -- DVR matchmaking, cluster detection, complexity analysis

### Vision (Long-Horizon)
- **[Extensible Capability Interface](docs/vision/EXTENSIBLE-CONTRACT-INTERFACES.md)** -- Platform design: trust-gated capabilities
- **[Federation](docs/vision/FEDERATION.md)** -- North star design for cross-group trust (Phase 4-5)
- **[Trust Topology Platform](docs/vision/TRUST-TOPOLOGY-PLATFORM.md)** -- Trust shaped by natural patterns (Phase 6+)
- **[MutualAI Convergence](docs/vision/MUTUALAI-CONVERGENCE.md)** -- Collective intelligence integration (Phase 7+)

---

## Architecture

### Three Layers, Single Binary

```
Signal           Rust Bot              Freenet
(user interface) (trust logic)         (decentralized state)
                                       
/invite -------> Gatekeeper ---------> Contract state
/vouch --------> Blind Matchmaker      Set-based membership
/flag ---------> Health Monitor        Mergeable deltas
/propose ------> Consensus Enforcer    Anonymous routing
/mesh ---------> DVR Calculator        Persistence network
```

1. **Signal** -- Members interact via simple commands in private messages. All crypto is hidden.
2. **Rust Bot** -- Enforces trust thresholds, selects assessors, monitors standing, executes consensus. One bot per group.
3. **Freenet** -- Decentralized state with eventual consistency. No central server. Anonymous routing (dark mode).

### Core Concepts

**Trust standing**: `Effective_Vouches - Regular_Flags` (must stay >= 0). If a voucher flags you, their vouch is invalidated -- a single member's action can never cause a 2-point swing.

**Mesh health (DVR)**: Distinct Validator Ratio measures independently-verified members. `DVR = Distinct_Validators / floor(N/4)`. Three tiers: red (0-33%), yellow (33-66%), green (66-100%).

**Cluster detection**: Bridge Removal algorithm (Tarjan's) identifies peer circles. Cross-cluster vouching is enforced to prevent coordinated infiltration. Bootstrap exception for groups under 4 members.

**Governance**: All group decisions via `/propose` and Signal Polls. Bot executes only contract-approved actions. Operator has zero override power.

### Technical Stack

| Component | Technology | Notes |
|-----------|------------|-------|
| Language | Rust 1.93+ | Static MUSL binary, edition 2021 |
| Signal (high-level) | Presage (fork) | Group management, polls (protocol v8) |
| Signal (low-level) | libsignal-service-rs (fork) | Protocol v8 poll support |
| State | freenet / freenet-stdlib | ComposableState, summary-delta sync |
| Identity masking | HMAC-SHA256 (ring) | BIP-39 mnemonic, HKDF-SHA256 key derivation |
| ZK circuits | winterfell | AIR defined; full STARK proving planned |
| Serialization | CBOR (ciborium) | Not JSON -- compact binary format |
| Store | SQLite + SQLCipher | Encrypted protocol state, no message history |

---

## Security Principles

- **Anonymity-first**: All identifiers hashed (HMAC), immediate zeroization of sensitive buffers
- **No message history**: Store wraps presage SQLite with message persistence disabled
- **Operator least-privilege**: Service runner only -- cannot add/remove members, change settings, or override consensus
- **Immediate ejection**: No grace periods when trust threshold violated
- **Decentralized truth**: Signal state derived from Freenet contract, not the other way around

---

## Getting Started

### For Developers

```bash
git clone https://github.com/roder/stroma.git
cd stroma
cargo build --release
cargo nextest run --all-features
```

> **Note**: No releases or container images exist yet. The project is pre-alpha.

### For Operators

Operator deployment is not yet available. See [Operator Guide](docs/OPERATOR-GUIDE.md) for the planned deployment model.

---

## The Road Ahead

Stroma's development follows three horizons:

**Now** (Phases 0-3): [TODO](docs/todo/TODO.md) -- Wire the validated mock layer to real Signal and Freenet networks. Reach UAT. 

**Next** (Phases 4-5): [Federation](docs/vision/FEDERATION.md) -- connect groups through shared members. Emergent discovery via Social Anchor Hashing. Trust that spans communities.

**Future** (Phase 6+): [Trust Topology Platform](docs/vision/TRUST-TOPOLOGY-PLATFORM.md) -- groups choose how trust organizes itself, guided by natural patterns. The protocol becomes a laboratory for collective intelligence.

---

## Contributing

This project uses:
- **Gastown coordination** -- Claude agents with specialized roles
- **Beads** -- Immutable architectural constraints (see `.beads/`)

All commits by Claude agents must include:
```
Co-authored-by: Claude <noreply@anthropic.com>
```

See [AGENTS.md](AGENTS.md) for the agent coordination model and [TODO.md](docs/todo/TODO.md) for current tasks.

---

## License

**AGPL-3.0-or-later**

**Why AGPL?**

1. **Legal requirement**: Stroma depends on AGPL-3.0 libraries (libsignal-service-rs, presage). Copyleft propagates.

2. **Security alignment**: Every operator runs a service for their group. AGPL ensures group members can inspect the bot's source code to verify it isn't leaking identities. Enforced transparency is a security feature.

3. **Philosophical alignment**: Stroma distributes power laterally. AGPL prevents proprietary forks that could capture the trust infrastructure. Network effects strengthen the commons, not a vendor.

**What this means**: If you run a Stroma bot, you must provide source code to your group members on request. If you fork Stroma, your fork must also be AGPL-3.0-or-later.

See [LICENSE](LICENSE) for full text.

---

**Last Updated**: 2026-02-17
