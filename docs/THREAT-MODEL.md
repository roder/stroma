# Threat Model & Problem Statement

**Last Updated**: 2026-02-01  
**Canonical Source**: `.beads/security-constraints.bead`

This document provides the high-level threat model for Stroma. For implementation details and immutable constraints, see the canonical beads.

## Problem Statement

### The Real Problems

1. **Trusting Strangers**: How do activists know members won't leak the group or infiltrate it?
2. **Trust Map Seizure**: What happens if an adversary (state-level, compromised operator, etc.) seizes the trust map?
3. **Verification vs. Anonymity Tension**: Verification requires exposure, but exposure creates vulnerability

### NOT the Problems

- ❌ "Voting on strangers" - This is not a Signal group problem
- ❌ "Bot learning why people trust each other" - The content of relationships is not the threat
- ❌ Focus on "what bot knows" rather than "what adversary could seize"

## Threat Model

### Primary Threat: Trust Map Seizure

**Adversary Goal**: Obtain the trust map to identify group members and their relationships

**Attack Vectors**:
1. **Server Compromise**: Adversary gains access to bot server
2. **Memory Dump**: Adversary captures running process memory
3. **Operator Compromise**: Operator is coerced or malicious
4. **Freenet Node Seizure**: Adversary seizes a Freenet node
5. **Signal Metadata Analysis**: Adversary analyzes Signal group metadata

### Three-Layer Defense

**Layer 1: No Centralized Storage**
- Trust map stored in decentralized Freenet network
- Distributed across peers - no single seizure point
- State can be reconstructed only with peer cooperation

**Layer 2: Cryptographic Privacy**
- All identities HMAC-hashed (Signal ACI-derived key, not separate pepper)
- Memory contains only hashes (zeroization of cleartext)
- ZK-proofs verify trust without revealing vouchers
- Memory dumps reveal nothing useful

**Layer 3: Metadata Isolation**
- All vetting in 1-on-1 PMs (no Signal group metadata leakage)
- Operator least-privilege (service runner only, can't export data)
- No logs of relationship content or reasons

**Result**: Even if adversary compromises bot or server, they only get:
- Hashes (not identities)
- Group size and topology (not relationship details)
- Vouch counts (not who vouched for whom in cleartext)

### Secondary Threats

1. **Sybil Attack**: Attacker creates many fake identities
   - **Defense**: 2-vouch requirement from members in DIFFERENT CLUSTERS (cross-cluster mandatory)
   - **Defense**: Same-cluster vouches count toward standing but don't satisfy cluster diversity for admission
   - **See**: `.beads/cross-cluster-requirement.bead`

2. **Infiltration**: Adversary gets admitted as member
   - **Defense**: Distributed vetting (no single gatekeeper)
   - **Defense**: Immediate ejection if flagged

3. **Vouch Bombing**: Attacker vouches then flags to manipulate
   - **Defense**: Vouch invalidation (flag invalidates vouch)

4. **Coordinated Infiltration**: Group of attackers vouch for each other
   - **Defense**: Cross-cluster requirement (prevents same-cluster rubber-stamping)
   - **Defense**: Distinct Validator Ratio (DVR) optimization
   - **How DVR helps**: Bot prioritizes creating Validators with non-overlapping voucher sets
   - **Result**: Compromising one voucher set doesn't cascade to multiple Validators
   - **See**: `.beads/blind-matchmaker-dvr.bead`, `.beads/mesh-health-metric.bead`

5. **Persistence Peer Attacks**: Adversary becomes chunk holder
   - **Attack**: Register fake bots to become chunk holders (DoS recovery)
   - **Attack**: Refuse to return chunks during recovery
   - **Attack**: Collude with other holders to reconstruct state
   - **Defense**: Need ALL chunks + ACI private key to reconstruct
   - **Defense**: AES-256-GCM encryption (even with all chunks, can't decrypt without ACI key)
   - **Defense**: Deterministic holder selection per-chunk (spreads chunks across many bots)
   - **Defense**: Challenge-response verification (prove chunk possession)
   - **Defense**: 3 copies per chunk (any 1 of 3 sufficient for that chunk)
   - **Result**: Even if adversary holds all chunks, they can't read trust map (need ACI key)
   - **Result**: Larger states = more chunks = more distribution = harder to seize
   - **Note**: Holder identities are computable (rendezvous hashing), but security comes from encryption, not obscurity
   - **See**: `.beads/persistence-model.bead`, `docs/PERSISTENCE.md`

6. **Freenet Data Loss**: Trust state falls off network
   - **Attack**: Wait for all subscribed peers to leave
   - **Attack**: Target bot's Freenet peers
   - **Defense**: Reciprocal Persistence Network (guaranteed minimum replicas)
   - **Defense**: Write-blocking prevents changes without backup
   - **Defense**: Recovery from encrypted fragments
   - **Result**: Trust state survives even if Freenet data falls off
   - **See**: `.beads/persistence-model.bead`

7. **Registry Availability Attacks**: Adversary disrupts persistence network
   - **Attack Vector 1 - State Bloat**: Register thousands of fake bots to bloat registry contract state
   - **Attack Vector 2 - Contract Computation**: Submit malformed/expensive queries that consume compute
   - **Attack Vector 3 - Read Amplification**: Query registry millions of times to exhaust Freenet node resources
   - **Attack Vector 4 - Shard-Targeted**: Focus DDoS on specific shard to deny service to subset of bots
   
   **Defense Layer 1: Freenet Native Protections**
   - Contracts hosted by multiple nodes (redundancy)
   - Nodes can rate-limit queries from specific sources
   - Contract state replicated across network
   - **Limitation**: Freenet protections are general-purpose, not Stroma-aware
   
   **Defense Layer 2: PoW Registration Cost**
   - ~30 seconds computation per registration (difficulty 18)
   - 1000 fake bots = ~8 hours of CPU time
   - **Limitation**: Wealthy adversary can still afford the cost
   
   **Defense Layer 3: Contract-Level Rate Limiting** (REQUIRED for Phase 1+)
   - Query rate limits per source identity
   - Computation budget per operation
   - Circuit breaker for expensive operations
   - **See**: `docs/PERSISTENCE.md` § Registry Availability Attacks
   
   **Defense Layer 4: Sharding Resilience**
   - Attack on one shard doesn't affect other shards
   - Bots in attacked shard can still operate (degraded, not blocked)
   - Gradual recovery as attack subsides
   
   **Accepted Risk**:
   - Registry is a known attack surface
   - Bot list is inherently discoverable (required for persistence)
   - Defense goal: degrade gracefully, recover quickly, not "prevent all DDoS"
   
   **Result**: Network continues operating under sustained attack, though with degraded performance
   
   **See**: `.beads/persistence-model.bead`, `docs/PERSISTENCE.md`

8. **Signal Client Storage Exposure**: Default Presage stores expose membership
   - **Attack**: Server seizure reveals message history, contacts, group metadata
   - **Attack**: Even encrypted, vetting conversations expose relationship context
   - **Defense**: `StromaStore` wrapper around encrypted `SqliteStore` (no-ops message persistence)
   - **Defense**: Persists protocol state, group config, profiles (encrypted with SQLCipher AES-256)
   - **Defense**: Never persist message content, history, or sticker packs (save_message is no-op)
   - **Defense**: Passphrase is 24-word BIP-39 recovery phrase (256 bits entropy)
   - **Result**: Server seizure yields only encrypted protocol state, no membership data
   - **See**: `.beads/security-constraints.bead` § 10, `docs/SECURITY-ANALYSIS-STORAGE.md`

9. **Bot Impersonation / Supply Chain Attack**
   - **Attack**: Adversary creates fake bot claiming to be Stroma
   - **Attack**: Adversary compromises bot binary via dependency tampering
   - **Attack**: Adversary distributes modified Stroma with backdoors
   - **Defense**: `cargo-deny` and `cargo-crev` for dependency auditing
   - **Defense**: Reproducible builds (static MUSL binary)
   - **Defense**: Operator verification procedures (to be documented)
   - **Accepted Risk**: Operators must verify they're running authentic Stroma
   - **See**: `.beads/technology-stack.bead`

10. **Signal Protocol Dependency**
    - **Attack**: Adversary exploits vulnerability in Signal protocol
    - **Attack**: Adversary intercepts device linking QR code during setup
    - **Attack**: libsignal-service-rs or Presage contains vulnerability
    - **Defense**: Inherit Signal's security model (well-audited)
    - **Defense**: Secure linking environment (operator responsibility)
    - **Defense**: Monitor Signal security advisories
    - **Accepted Risk**: Stroma's security is bounded by Signal's security
    - **See**: `.beads/signal-integration.bead`

11. **Operator Account Compromise**
    - **Attack**: Adversary compromises the Signal account the bot is linked to
    - **Attack**: Adversary steals operator's device to access linked bot
    - **Attack**: Adversary social-engineers operator into revealing credentials
    - **Defense**: Operator OPSEC (separate Signal account for bot)
    - **Defense**: Device linking can be revoked from primary device
    - **Defense**: Bot has no special privileges beyond group membership
    - **Accepted Risk**: Operator security practices are outside Stroma's control
    - **Note**: Compromised operator account ≠ compromised trust map (still encrypted in Freenet)

12. **Bot-Level Denial of Service**
    - **Attack**: Adversary floods bot with Signal messages to exhaust resources
    - **Attack**: Adversary triggers expensive operations (ZK proof generation)
    - **Attack**: Adversary creates many fake invitees to exhaust vetting capacity
    - **Defense**: Rate limiting on commands per user (to be implemented)
    - **Defense**: Proof generation bounded by legitimate operations
    - **Defense**: Invitation requires existing member (limits attack surface)
    - **Accepted Risk**: Determined adversary with member access can degrade service
    - **Note**: Distinct from registry DDoS (threat #7) - this targets individual bot

13. **Social Engineering / Long-Term Infiltration**
    - **Attack**: Adversary manipulates legitimate members into vouching for infiltrators
    - **Attack**: Adversary builds trust slowly over time, then acts maliciously
    - **Attack**: Adversary compromises existing trusted member
    - **Defense**: Cross-cluster requirement (requires deceiving multiple social contexts)
    - **Defense**: Immediate ejection on flag (quick response to detected threats)
    - **Defense**: DVR optimization (non-overlapping voucher sets limit cascade)
    - **Accepted Risk**: Social engineering is fundamentally a human problem
    - **Note**: Technology cannot fully prevent social attacks - members must remain vigilant
    - **See**: `.beads/cross-cluster-requirement.bead`

14. **Poll/Voting Manipulation**
    - **Attack**: Adversary times votes to prevent counter-votes (last-second voting)
    - **Attack**: Adversary creates many proposals to exhaust member attention
    - **Attack**: Adversary uses multiple accounts to vote multiple times (Sybil voting)
    - **Defense**: Poll timeout provides window for all members to vote
    - **Defense**: Quorum requirements ensure sufficient participation
    - **Defense**: One Signal account = one vote (Sybil requires multiple Signal accounts + vouches)
    - **Accepted Risk**: Sophisticated vote timing attacks possible within timeout window
    - **Note**: Stroma does NOT persist individual votes (privacy protection)
    - **See**: `.beads/voting-mechanism.bead`

15. **Traffic/Timing Analysis**
    - **Attack**: Adversary monitors network traffic to correlate bot activity with group events
    - **Attack**: Adversary monitors Freenet traffic patterns to identify active groups
    - **Attack**: Adversary correlates Signal message timing with Freenet state updates
    - **Defense**: Freenet's onion routing provides network-level protection
    - **Defense**: Trust state changes are infrequent (human timescale)
    - **Accepted Risk**: Traffic analysis partially mitigated but not fully prevented
    - **Note**: State-level adversary with global network view may correlate timing
    - **Scope Decision**: Deep traffic analysis resistance is out of scope for Phase 0

---

## Accepted Risks Summary

The following risks are explicitly accepted as outside Stroma's control or requiring disproportionate effort to mitigate:

| Risk | Rationale |
|------|-----------|
| Operator OPSEC failures | Operator security practices outside Stroma's control |
| Signal protocol vulnerabilities | Inherit Signal's security model (well-audited) |
| Social engineering attacks | Fundamentally a human problem, not solvable by technology |
| Sophisticated traffic analysis | Requires global network adversary; Freenet provides baseline protection |
| Supply chain attacks on operators | Operators must verify binary authenticity |

---

## Consistent Messaging Standards

### Required Terminology

**Use These Terms**:
- "Trust map seizure" (primary threat)
- "State-level adversary" (threat actor)
- "Three-layer defense" (protection model)
- "No single seizure point" (decentralization benefit)
- "Memory dumps reveal nothing" (cryptographic privacy benefit)
- "Metadata isolation" (Signal/operator protection)

### Prohibited Framings

**Never Use These**:
- "Bot doesn't know why people trust each other" (not the security concern)
- "Voting on strangers" (not a Signal problem - use "trusting strangers")
- Focus on bot's semantic knowledge (focus on adversary's ability to seize)

### Security Messaging Framework

**Always Frame Security Around**:
- What adversary gets if they compromise system
- Three independent defense layers
- Concrete attack scenarios and mitigations

