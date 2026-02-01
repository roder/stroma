# Pre-Gastown Audit: Human Review Checklist

**Date**: 2026-01-31  
**Purpose**: Final human audit before turning Stroma over to Gastown agents for implementation  
**Status**: Planning phase

## Why This Audit Matters

Gastown agents will follow beads, rules, and documentation **literally**. Any inconsistencies, ambiguities, or conflicting guidance will:
- Cause agents to ask for clarification (slowing progress)
- Lead to incorrect implementations (requiring rework)
- Create confusion about priorities (what takes precedence?)

**Goal**: Ensure all architectural guidance is consistent, unambiguous, and ready for literal interpretation by agents.

---

## Audit Scope

### 1. Beads (Architectural Constraints)
**Location**: `.beads/*.bead`

**What to Check**:
- [ ] **Terminology consistency**: Do all beads use the same terms for the same concepts?
- [ ] **No contradictions**: Do any beads conflict with each other?
- [ ] **Completeness**: Are all architectural decisions documented?
- [ ] **Priority clarity**: When beads conflict, is there clear precedence?
- [ ] **Spike Week alignment**: Do beads reflect outstanding questions from SPIKE-WEEK-BRIEFING.md?

**Key Beads to Audit**:
- [ ] `architecture-decisions.bead` - Core decisions
- [ ] `architectural-decisions-open.bead` - Open architectural questions (NEW)
- [ ] `bot-deployment-model.bead` - 1:1 bot-to-group
- [ ] `contract-encryption.bead` - Encryption key derivation (NEW)
- [ ] `cross-cluster-requirement.bead` - Diversity requirement (uses "cluster" not "friend circles")
- [ ] `discovery-protocols.bead` - Bot discovery protocols (NEW)
- [ ] `governance-model.bead` - Bot execute-only
- [ ] `mesh-health-metric.bead` - DVR metric
- [ ] `blind-matchmaker-dvr.bead` - Algorithm enhancement
- [ ] `persistence-model.bead` - Reciprocal Persistence Network (NEW)
- [ ] `philosophical-foundations.bead` - Core principles
- [ ] `poll-implementation-gastown.bead` - Signal Polls
- [ ] `proposal-system.bead` - Consensus mechanism
- [ ] `security-constraints.bead` - Security model
- [ ] `serialization-format.bead` - CBOR serialization (NEW)
- [ ] `technology-stack.bead` - Tech decisions
- [ ] `terminology.bead` - Canonical definitions
- [ ] `voting-mechanism.bead` - Anonymous voting

### 2. Rules (Cursor Agent Guidance)
**Location**: `.cursor/rules/*.mdc`

**What to Check**:
- [ ] **Alignment with beads**: Do rules reflect bead decisions?
- [ ] **Terminology consistency**: Same terms as beads and docs?
- [ ] **No contradictions**: Do rules conflict with each other or beads?
- [ ] **Implementation clarity**: Are guardrails clear and enforceable?
- [ ] **Security constraints**: Are all "NEVER" rules absolute?

**Key Rules to Audit**:
- [ ] `architecture-objectives.mdc` - High-level goals
- [ ] `cluster-terminology.mdc` - Internal vs external (uses "cluster" terminology)
- [ ] `core-standards.mdc` - Naming, conventions
- [ ] `cryptography-zk.mdc` - ZK-proof patterns
- [ ] `freenet-contract-design.mdc` - Contract schema
- [ ] `freenet-integration.mdc` - Freenet patterns
- [ ] `gastown-workflow.mdc` - Agent coordination
- [ ] `git-standards.mdc` - Commit conventions
- [ ] `graph-analysis.mdc` - DVR-optimized algorithm
- [ ] `operator-cli.mdc` - CLI output
- [ ] `philosophical-foundations.mdc` - Core principles
- [ ] `rust-async.mdc` - Async patterns
- [ ] `rust-standards.mdc` - Rust conventions
- [ ] `security-guardrails.mdc` - Security constraints
- [ ] `signal-integration.mdc` - Signal patterns
- [ ] `tech-stack.mdc` - Technology standards
- [ ] `testing-standards.mdc` - Test requirements
- [ ] `user-roles-ux.mdc` - User experience
- [ ] `vetting-protocols.mdc` - Admission logic

### 3. Documentation (User & Developer Facing)
**Location**: `docs/*.md`, `README.md`

**What to Check**:
- [ ] **Terminology consistency**: "Friend circles" vs "clusters" throughout
- [ ] **Alignment with architecture**: Do docs reflect bead decisions?
- [ ] **No contradictions**: Do user docs conflict with technical docs?
- [ ] **Completeness**: Are all features documented?
- [ ] **Clarity**: Can non-technical users understand HOW-IT-WORKS.md?

**Key Docs to Audit**:
- [ ] `README.md` - Project overview (recently updated)
- [ ] `docs/HOW-IT-WORKS.md` - Non-technical explanation
- [ ] `docs/USER-GUIDE.md` - Bot commands
- [ ] `docs/TRUST-MODEL.md` - Trust logic
- [ ] `docs/DEVELOPER-GUIDE.md` - Technical implementation
- [ ] `docs/ALGORITHMS.md` - Graph algorithms
- [ ] `docs/PERSISTENCE.md` - State durability & recovery (NEW)
- [ ] `docs/THREAT-MODEL-AUDIT.md` - Security analysis
- [ ] `docs/spike/SPIKE-WEEK-BRIEFING.md` - Spike Week 1 validation
- [ ] `docs/spike/SPIKE-WEEK-2-BRIEFING.md` - Spike Week 2 persistence validation (NEW)
- [ ] `docs/FEDERATION.md` - Future federation
- [ ] `docs/OPERATOR-GUIDE.md` - Running the bot

---

## Critical Terminology Audit

**Problem Identified**: "Cluster" vs "Friend Circles"

### Current State
| Location | Term Used | Status |
|----------|-----------|--------|
| `README.md` | "friend circles" | ✅ Fixed (commit e8239f2) |
| `docs/HOW-IT-WORKS.md` | ? | ❓ Unknown |
| `docs/USER-GUIDE.md` | ? | ❓ Unknown |
| `docs/TRUST-MODEL.md` | ? | ❓ Unknown |
| `.beads/cross-cluster-requirement.bead` | "cluster" | ⚠️ Technical term |
| `.beads/terminology.bead` | "cluster" | ⚠️ Technical term |
| `.cursor/rules/cluster-terminology.mdc` | "cluster" | ⚠️ Technical term |
| `.cursor/rules/vetting-protocols.mdc` | "cross-cluster" | ⚠️ Technical term |

### Decision Needed
**Question**: Should we:
1. **Option A**: Keep "cluster" in technical docs/beads, use "friend circles" only in user-facing docs?
2. **Option B**: Replace "cluster" everywhere with "friend circles"?
3. **Option C**: Define "cluster" clearly early in all docs, then use it consistently?

**Recommendation**: **Option A** - Technical precision in beads/rules, user-friendly language in docs.

**Rationale**:
- Agents need precise terminology ("cluster" is well-defined in graph theory)
- Users need intuitive terminology ("friend circles" is self-explanatory)
- Bridge documents (like DEVELOPER-GUIDE.md) should explain both

---

## Architectural Consistency Checks

### Check 1: Cross-Cluster Requirement
**Question**: Is the scaling cross-cluster requirement consistently documented?

**Where to Check**:
- [ ] `.beads/cross-cluster-requirement.bead` - Defines scaling requirement
- [ ] `.cursor/rules/vetting-protocols.mdc` - Admission logic
- [ ] `docs/TRUST-MODEL.md` - Trust model explanation
- [ ] `docs/HOW-IT-WORKS.md` - User-facing explanation
- [ ] `docs/USER-GUIDE.md` - Bot behavior

**Expected Consistency**:
- Bridges: 2 vouches from 2 clusters (or "friend circles" in user docs)
- Validators: min(vouch_count, available_clusters) clusters
- Bootstrap exception: 1 cluster suspends requirement

### Check 2: DVR Metric
**Question**: Is the Distinct Validator Ratio consistently defined?

**Where to Check**:
- [ ] `.beads/mesh-health-metric.bead` - Metric definition
- [ ] `.beads/blind-matchmaker-dvr.bead` - Algorithm
- [ ] `.cursor/rules/freenet-contract-design.mdc` - Contract calculation
- [ ] `.cursor/rules/user-roles-ux.mdc` - UX display
- [ ] `docs/ALGORITHMS.md` - Mathematical definition
- [ ] `docs/TRUST-MODEL.md` - Health metrics
- [ ] `docs/USER-GUIDE.md` - User explanation
- [ ] `README.md` - Overview

**Expected Consistency**:
- Formula: `DVR = Distinct_Validators / (N / 4)`
- Three tiers: 🔴 0-33% / 🟡 33-66% / 🟢 66-100%
- Definition: Non-overlapping voucher sets

### Check 3: Vouch Invalidation
**Question**: Is vouch invalidation logic consistently defined?

**Where to Check**:
- [ ] `.beads/security-constraints.bead` - No 2-point swings
- [ ] `.cursor/rules/security-guardrails.mdc` - Enforcement
- [ ] `.cursor/rules/freenet-contract-design.mdc` - Calculation
- [ ] `docs/TRUST-MODEL.md` - Trust standing
- [ ] `docs/VOUCH-INVALIDATION-LOGIC.md` - Detailed examples
- [ ] `docs/USER-GUIDE.md` - User explanation

**Expected Consistency**:
- Voucher-flaggers excluded from BOTH counts
- No single member can cause 2-point swing
- Standing = Effective_Vouches - Regular_Flags

### Check 4: Bot Governance Model
**Question**: Is bot execute-only role consistently enforced?

**Where to Check**:
- [ ] `.beads/governance-model.bead` - Bot role definition
- [ ] `.cursor/rules/security-guardrails.mdc` - Operator constraints
- [ ] `.cursor/rules/architecture-objectives.mdc` - Core invariant
- [ ] `docs/OPERATOR-GUIDE.md` - Operator instructions
- [ ] `docs/DEVELOPER-GUIDE.md` - Bot architecture

**Expected Consistency**:
- Bot is Signal admin (technical) but execute-only (no decisions)
- All decisions via group consensus (Signal Polls)
- Operator is service runner only (no privileges)

### Check 5: Signal Polls vs Reactions
**Question**: Is anonymous voting via Signal Polls consistently required?

**Where to Check**:
- [ ] `.beads/voting-mechanism.bead` - Poll rationale
- [ ] `.beads/poll-implementation-gastown.bead` - Implementation
- [ ] `.beads/proposal-system.bead` - Proposal workflow
- [ ] `.cursor/rules/security-guardrails.mdc` - Voting constraints
- [ ] `docs/USER-GUIDE.md` - User commands

**Expected Consistency**:
- Never use reactions (expose voter identity)
- Always use Signal Polls (anonymous)
- Protocol v8 required (libsignal-service-rs fork)

---

## Security Constraint Audit

### Three-Layer Defense
**Question**: Is trust map seizure protection consistently explained?

**Where to Check**:
- [ ] `.beads/security-constraints.bead` - Three layers
- [ ] `.cursor/rules/security-guardrails.mdc` - Enforcement
- [ ] `docs/THREAT-MODEL-AUDIT.md` - Threat model
- [ ] `docs/DEVELOPER-GUIDE.md` - Implementation
- [ ] `README.md` - Overview

**Expected Layers**:
1. No centralized storage (Freenet distributed)
2. Cryptographic privacy (HMAC hashing, zeroization)
3. Metadata isolation (1-on-1 PMs, operator least-privilege)

### Server Seizure Protection
**Question**: Is ephemeral storage consistently enforced?

**Where to Check**:
- [ ] `.beads/security-constraints.bead` - Bot storage
- [ ] `.beads/technology-stack.bead` - Presage store
- [ ] `.cursor/rules/security-guardrails.mdc` - Storage violations
- [ ] `.cursor/rules/tech-stack.mdc` - Store requirements
- [ ] `docs/DEVELOPER-GUIDE.md` - Implementation

**Expected Enforcement**:
- Never use SqliteStore (stores all messages)
- Custom StromaProtocolStore (protocol state only)
- Vetting conversations ephemeral (never persisted)
- Memory contains only hashes (zeroization)

---

## Spike Week Alignment

### Spike Week 1 (Q1-Q6) — ✅ COMPLETE
**Status**: All questions answered, ready for Phase 0 implementation.

**Results**:
- ✅ Q1: Freenet merge conflicts — GO (commutative deltas)
- ✅ Q2: Contract validation — GO (trustless model viable)
- ✅ Q3: Cluster detection — GO (Bridge Removal algorithm)
- ✅ Q4: STARK verification — PARTIAL (bot-side for Phase 0)
- ✅ Q5: Merkle Tree performance — GO (0.09ms @ 1000 members)
- ✅ Q6: Proof storage — Store outcomes only

**Verification**:
- [x] `docs/spike/SPIKE-WEEK-BRIEFING.md` - 6 questions answered
- [x] Beads updated with Q3 findings (Bridge Removal)
- [x] Rules reflect Q1-Q6 decisions

### Spike Week 2 (Q7-Q14) — ✅ COMPLETE
**Status**: Persistence network validation pending. Must complete before persistence implementation.

**Where to Check**:
- [ ] `docs/spike/SPIKE-WEEK-2-BRIEFING.md` - 8 persistence questions
- [ ] `docs/PERSISTENCE.md` - Reciprocal Persistence Network design
- [ ] `.beads/persistence-model.bead` - Persistence constraints

**Q7 Dependencies** (BLOCKING):
- Bot discovery mechanism for persistence network
- Chunk distribution and recovery
- Write-blocking states (DEGRADED, ACTIVE, etc.)

**Expected Alignment**:
- Beads note Spike Week 2 dependencies
- Persistence implementation blocked until Q7-Q14 validated
- Fallback strategies documented for each question

---

## Audit Process

### Phase 1: Terminology Sweep (1-2 hours)
1. **Search all files** for "cluster" and "cross-cluster"
2. **Categorize** by location (user-facing vs technical)
3. **Decide** on Option A, B, or C (see "Decision Needed" above)
4. **Update** inconsistent files

### Phase 2: Architectural Consistency (2-3 hours)
1. **Review each bead** against checklist above
2. **Cross-reference** with rules and docs
3. **Flag contradictions** for resolution
4. **Document decisions** in this file

### Phase 3: Security Constraint Verification (1-2 hours)
1. **Review all "NEVER" rules** in security-guardrails.mdc
2. **Verify enforcement** in contract design and bot architecture
3. **Check** that docs reflect security model
4. **Test** threat model against architecture

### Phase 4: Spike Week Alignment (1 hour)
1. **Review Q1-Q6** in SPIKE-WEEK-BRIEFING.md
2. **Verify** beads note dependencies
3. **Check** fallback strategies documented
4. **Confirm** no architectural decisions bypass Spike Week

### Phase 5: Final Review (1 hour)
1. **Read through** all beads in sequence
2. **Imagine** you're a Gastown agent — is everything clear?
3. **Flag** any ambiguities or missing context
4. **Update** this checklist with findings

---

## Go/No-Go Decision Criteria

### GO if:
- ✅ All terminology consistent within each audience (user vs technical)
- ✅ No contradictions between beads, rules, and docs
- ✅ All architectural decisions documented and aligned
- ✅ Security constraints consistently enforced
- ✅ Spike Week dependencies noted and fallbacks provided
- ✅ Agent handoff feels confident (no major ambiguities)

### NO-GO if:
- ❌ Major terminology inconsistencies confuse guidance
- ❌ Contradictions between beads or rules
- ❌ Missing architectural decisions or unclear priorities
- ❌ Security model has gaps or conflicting enforcement
- ❌ Spike Week questions not reflected in beads
- ❌ Agent handoff feels risky (too many unknowns)

---

## Audit Findings (To Be Filled)

### Terminology Issues
*Document any "cluster" vs "friend circles" inconsistencies here*

### Architectural Contradictions
*Document any conflicts between beads, rules, or docs here*

### Security Gaps
*Document any missing security constraints or enforcement gaps here*

### Ambiguities for Agents
*Document anything that would confuse a Gastown agent here*

### Recommended Fixes
*List specific files and changes needed before agent handoff*

---

## Next Steps

1. **Schedule audit session** (estimate: 6-9 hours total)
2. **Complete phases 1-5** systematically
3. **Document findings** in this file
4. **Make necessary fixes** to beads, rules, and docs
5. **Final review** with fresh eyes
6. **Go/No-Go decision** based on criteria above
7. **If GO**: Hand off to Gastown agents
8. **If NO-GO**: Address critical issues and re-audit

---

## Success Criteria

**Audit is complete when**:
- [ ] All checklists above marked complete
- [ ] Findings section filled with specific issues
- [ ] Recommended fixes section has concrete action items
- [ ] Go/No-Go decision made with confidence
- [ ] If GO: Handoff document prepared for Gastown agents

**Gastown handoff ready when**:
- [ ] No terminology confusion
- [ ] No architectural contradictions
- [ ] Security model airtight
- [ ] Spike Week aligned
- [ ] Agent can follow guidance literally without ambiguity
