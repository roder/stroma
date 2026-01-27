# Threat Model & Problem Statement Audit

**Date**: 2026-01-27  
**Purpose**: Ensure all documentation accurately reflects the real security concerns and threat model

## Problem Statement (Corrected)

### The Real Problems

1. **Trusting Strangers**: How do activists know members won't leak the group or infiltrate it?
2. **Trust Map Seizure**: What happens if an adversary (state-level, compromised operator, etc.) seizes the trust map?
3. **Verification vs. Anonymity Tension**: Verification requires exposure, but exposure creates vulnerability

### NOT the Problems

- ❌ "Voting on strangers" - This is not a Signal group problem
- ❌ "Bot learning why people trust each other" - The content of relationships is not the threat
- ❌ Focus on "what bot knows" rather than "what adversary could seize"

## Threat Model (Corrected Focus)

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
- All identities HMAC-hashed (group-secret pepper)
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
   - **Defense**: 2-vouch requirement from independent members
   - **Defense**: Cross-cluster vouching requirement

2. **Infiltration**: Adversary gets admitted as member
   - **Defense**: Distributed vetting (no single gatekeeper)
   - **Defense**: Immediate ejection if flagged

3. **Vouch Bombing**: Attacker vouches then flags to manipulate
   - **Defense**: Vouch invalidation (flag invalidates vouch)

## Files Requiring Updates

### Priority 1: Core Security Documents

- [ ] `.beads/security-constraints.bead` - Add explicit threat model section
- [ ] `.cursor/rules/security-guardrails.mdc` - Update threat model references
- [ ] `docs/DEVELOPER-GUIDE.md` - Expand threat model section
- [ ] `docs/ALGORITHMS.md` - Update privacy guarantees section

### Priority 2: User-Facing Documentation

- [ ] `docs/USER-GUIDE.md` - Update security explanations
- [ ] `docs/OPERATOR-GUIDE.md` - Add threat model for operators
- [ ] `docs/TRUST-MODEL.md` - Align with seizure threat

### Priority 3: Architecture Documentation

- [ ] `.cursor/rules/architecture-objectives.mdc` - Update security goals
- [ ] `.beads/architecture-decisions.bead` - Update threat model section
- [ ] `docs/FEDERATION.md` - Federation threat model

### Priority 4: Rules & Standards

- [ ] `.cursor/rules/vetting-protocols.mdc` - Update privacy rationale
- [ ] `.cursor/rules/freenet-integration.mdc` - Emphasize decentralization defense
- [ ] `.cursor/rules/signal-integration.mdc` - Emphasize metadata isolation
- [ ] `.cursor/rules/cluster-terminology.mdc` - Clarify privacy boundaries

## Key Messaging Updates

### Old Framing (Incorrect Focus)
- "Bot doesn't know why people trust each other"
- Focus on bot's semantic knowledge
- Emphasis on relationship content privacy

### New Framing (Correct Focus)
- "Trust map protected from seizure"
- Focus on adversary's ability to compromise system
- Emphasis on three-layer defense against state-level threats

### Consistent Language

**Use These Terms**:
- "Trust map seizure" (primary threat)
- "State-level adversary" (threat actor)
- "Three-layer defense" (protection model)
- "No single seizure point" (decentralization benefit)
- "Memory dumps reveal nothing" (cryptographic privacy benefit)
- "Metadata isolation" (Signal/operator protection)

**Avoid These Framings**:
- "Bot doesn't know why" (not the security concern)
- "Voting on strangers" (not a Signal problem)
- Focus on bot's knowledge vs. adversary's access

## Implementation Checklist

### Phase 1: Security Core
- [ ] Update `.beads/security-constraints.bead` with explicit threat model
- [ ] Update `.cursor/rules/security-guardrails.mdc` with three-layer defense
- [ ] Verify all security rules reference trust map protection

### Phase 2: Developer Documentation
- [ ] Expand `docs/DEVELOPER-GUIDE.md` threat model section
- [ ] Update `docs/ALGORITHMS.md` privacy guarantees
- [ ] Add trust map seizure scenarios to security tests

### Phase 3: User Documentation
- [ ] Update `docs/USER-GUIDE.md` security explanations
- [ ] Update `docs/OPERATOR-GUIDE.md` threat model
- [ ] Simplify for non-technical users

### Phase 4: Architecture Alignment
- [ ] Update all architecture documents with consistent threat model
- [ ] Ensure beads reflect trust map protection focus
- [ ] Update federation roadmap with threat model

### Phase 5: Validation
- [ ] Grep all files for old framing patterns
- [ ] Verify consistent language across all documents
- [ ] Test security messaging with target audiences

---

**Status**: Audit in progress  
**Next Step**: Systematically update files in priority order  
**Completion Criteria**: All documents consistently emphasize trust map protection via three-layer defense
