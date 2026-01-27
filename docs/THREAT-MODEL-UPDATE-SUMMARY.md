# Threat Model & Problem Statement Update - Complete

**Date**: 2026-01-27  
**Status**: ✅ COMPLETE

## Overview

Systematically updated all documentation, rules, and beads to align with the correct threat model and problem statement for Stroma.

## Problem Statement Corrections

### What Was Fixed

**Old (Incorrect)**:
- "Voting on strangers" - Not a Signal group problem

**New (Correct)**:
- "Trusting strangers" - How do activists know members won't leak or infiltrate the group?
- Explicit tension: "Verification requires exposure, but exposure creates vulnerability"

### Key Addition

Added clear statement of the fundamental security challenge:
> **This creates a fundamental tension: verification requires exposure, but exposure creates vulnerability.**

## Threat Model Alignment

### Primary Threat (Now Consistently Stated)

**Trust map seizure by state-level adversary or compromised operator**

**Adversary Goal**: Obtain trust map to identify group members and their relationships

### Three-Layer Defense (Now Applied Everywhere)

All documentation now consistently frames security around three independent defense layers:

1. **No Centralized Storage**
   - Trust map distributed across Freenet network
   - No single seizure point
   - Adversary needs multiple peer seizures

2. **Cryptographic Privacy**
   - All identities HMAC-hashed (group-secret pepper)
   - Immediate zeroization of cleartext
   - Memory dumps contain only hashes

3. **Metadata Isolation**
   - All vetting in 1-on-1 PMs (not group chat)
   - Operator least-privilege (service runner only)
   - No logs of relationship content

### Adversary Scenario (Now Explicit)

Every security section now states:

> Even if adversary compromises bot or server, they only get:
> - Hashes (not identities)
> - Group size and topology (not relationship details)
> - Vouch counts (not who vouched for whom in cleartext)

## Files Updated

### Priority 1: Core Security Documents (6 files)
- `README.md` - Trust map protection messaging
- `.beads/security-constraints.bead` - Explicit threat model section
- `.cursor/rules/security-guardrails.mdc` - Leads with threat model
- `docs/DEVELOPER-GUIDE.md` - Expanded threat model with attack vectors
- `docs/ALGORITHMS.md` - Privacy guarantees reframed
- `docs/THREAT-MODEL-AUDIT.md` - Audit tracking document (new)

### Priority 2: User-Facing Documentation (3 files)
- `docs/USER-GUIDE.md` - "How Your Group is Protected" section
- `docs/OPERATOR-GUIDE.md` - "Operator Threat Model" section
- `docs/TRUST-MODEL.md` - ZK-proof purpose clarified

### Priority 3: Architecture Documentation (3 files)
- `.beads/architecture-decisions.bead` - Three-layer defense as architectural core
- `.cursor/rules/architecture-objectives.mdc` - Security goals updated
- `docs/FEDERATION.md` - Federation vision aligned

### Priority 4: Rules & Standards (4 files)
- `.cursor/rules/vetting-protocols.mdc` - Trust map protection emphasis
- `.cursor/rules/freenet-integration.mdc` - Decentralization defense
- `.cursor/rules/signal-integration.mdc` - Metadata isolation
- `.cursor/rules/cluster-terminology.mdc` - Privacy boundaries clarified

**Total**: 17 files updated

## Validation Results

### Terminology Consistency

**Prohibited Framings** (searched across all files):
- "Bot doesn't know why people trust each other" - 0 occurrences (except audit doc examples)
- "Voting on strangers" - 0 occurrences (except audit doc examples)

**Required Terminology** (searched across all files):
- "Three-layer defense" / "Trust map protection" / "Trust map seizure" - **42 occurrences across 16 files**

### Git History

**7 commits** documenting the systematic update:
1. `a17b832` - Priority 1: Core security documents
2. `469f97d` - Priority 2: User-facing documentation
3. `1ebf80b` - Priority 3: Architecture documentation
4. `5348898` - Priority 4: Rules & standards
5. `a7176b0` - Remove historical framing from audit
6. `e9d4602` - Refocus README on trust map seizure
7. `dc00073` - Mark audit complete with validation

## Key Messaging Changes

### Before (Incorrect Focus)

- Emphasis on "bot's semantic knowledge"
- Focus on "bot doesn't know why people trust each other"
- Abstract privacy concerns without concrete threat

### After (Correct Focus)

- Emphasis on "what adversary gets if they seize trust map"
- Focus on three-layer defense preventing useful seizure
- Concrete threat: state-level adversary or compromised operator
- Explicit outcomes: "only hashes and topology, not identities"

## Impact on Development

### For AI Agents

All documentation now consistently provides:
- Clear threat model to design against
- Three-layer defense framework to implement
- Explicit adversary scenarios to test
- Consistent terminology for security reasoning

### For Human Developers

Clear understanding of:
- Primary threat: trust map seizure
- Why each layer of defense is necessary
- What happens if any single layer is compromised
- How the three layers work together

### For Security Auditors

Explicit framing of:
- Adversary goals and capabilities
- Defense mechanisms and their interactions
- What adversary gets vs what they don't get
- Concrete attack scenarios and mitigations

## Documentation Standards Established

### Required Terminology

**Always Use**:
- "Trust map seizure" (primary threat)
- "State-level adversary" (threat actor)
- "Three-layer defense" (protection model)
- "No single seizure point" (decentralization)
- "Memory dumps reveal nothing" (cryptographic privacy)
- "Metadata isolation" (Signal/operator protection)

### Prohibited Framings

**Never Use**:
- "Bot doesn't know why" (not the security concern)
- "Voting on strangers" (not a Signal problem)
- Focus on bot's knowledge vs adversary's access

### Security Messaging Framework

**Always Frame Security Around**:
- What adversary gets if they compromise system
- Three independent defense layers
- Concrete attack scenarios and mitigations

## Completion Criteria Met

✅ All documents consistently emphasize trust map seizure protection  
✅ Three-layer defense framework applied everywhere  
✅ Problem statement corrected (trusting vs voting)  
✅ Explicit adversary scenarios in all security sections  
✅ Consistent terminology across 16 files  
✅ 42 references to core security framework  
✅ 0 prohibited framings found  

---

**Outcome**: The Stroma project now has complete alignment between its stated problem (activists needing trust verification without identity exposure) and its threat model (defending against trust map seizure by state-level adversaries). All documentation, rules, and beads consistently frame security around the three-layer defense architecture.
