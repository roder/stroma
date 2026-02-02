# Pre-Gastown Audit: Human Review Checklist

**Date**: 2026-02-01 
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
- [X] **Terminology consistency**: Do all beads use the same terms for the same concepts?
- [X] **No contradictions**: Do any beads conflict with each other?
- [X] **Completeness**: Are all architectural decisions documented?
- [X] **Priority clarity**: When beads conflict, is there clear precedence?
- [X] **Spike Week alignment**: Do beads reflect outstanding questions from SPIKE-WEEK-BRIEFING.md?

**Key Beads to Audit**:
- [X] `architecture-decisions.bead` - Core decisions
- [X] `architectural-decisions-open.bead` - Open architectural questions (NEW)
- [X] `bot-deployment-model.bead` - 1:1 bot-to-group
- [X] `contract-encryption.bead` - Encryption key derivation (NEW)
- [X] `cross-cluster-requirement.bead` - Diversity requirement (uses "cluster" not "peer circles")
- [X] `discovery-protocols.bead` - Bot discovery protocols (NEW)
- [X] `governance-model.bead` - Bot execute-only
- [X] `mesh-health-metric.bead` - DVR metric
- [X] `blind-matchmaker-dvr.bead` - Algorithm enhancement
- [X] `persistence-model.bead` - Reciprocal Persistence Network (NEW)
- [X] `philosophical-foundations.bead` - Core principles
- [X] `poll-implementation-gastown.bead` - Signal Polls
- [X] `proposal-system.bead` - Consensus mechanism
- [X] `security-constraints.bead` - Security model
- [X] `serialization-format.bead` - CBOR serialization (NEW)
- [X] `technology-stack.bead` - Tech decisions
- [X] `terminology.bead` - Canonical definitions
- [X] `voting-mechanism.bead` - Signal Polls voting

### 2. Rules (Cursor Agent Guidance)
**Location**: `.cursor/rules/*.mdc`

**What to Check**:
- [X] **Alignment with beads**: Do rules reflect bead decisions?
- [X] **Terminology consistency**: Same terms as beads and docs?
- [X] **No contradictions**: Do rules conflict with each other or beads?
- [X] **Implementation clarity**: Are guardrails clear and enforceable?
- [X] **Security constraints**: Are all "NEVER" rules absolute?

**Key Rules to Audit**:
- [X] `architecture-objectives.mdc` - High-level goals
- [X] `cluster-terminology.mdc` - Internal vs external (uses "cluster" terminology)
- [X] `core-standards.mdc` - Naming, conventions
- [X] `cryptography-zk.mdc` - ZK-proof patterns
- [X] `freenet-contract-design.mdc` - Contract schema
- [X] `freenet-integration.mdc` - Freenet patterns
- [X] `gastown-workflow.mdc` - Agent coordination
- [X] `git-standards.mdc` - Commit conventions
- [X] `graph-analysis.mdc` - DVR-optimized algorithm
- [X] `operator-cli.mdc` - CLI output
- [X] `philosophical-foundations.mdc` - Core principles
- [X] `rust-async.mdc` - Async patterns
- [X] `rust-standards.mdc` - Rust conventions
- [X] `security-guardrails.mdc` - Security constraints
- [X] `signal-integration.mdc` - Signal patterns
- [X] `tech-stack.mdc` - Technology standards
- [X] `testing-standards.mdc` - Test requirements
- [X] `user-roles-ux.mdc` - User experience
- [X] `vetting-protocols.mdc` - Admission logic

### 3. Documentation (User & Developer Facing)
**Location**: `docs/*.md`, `README.md`

**What to Check**:
- [X] **Terminology consistency**: "Peer circles" vs "clusters" throughout
- [X] **Alignment with architecture**: Do docs reflect bead decisions?
- [X] **No contradictions**: Do user docs conflict with technical docs?
- [x] **Completeness**: Are all features documented?
- [X] **Clarity**: Can non-technical users understand HOW-IT-WORKS.md?

**Key Docs to Audit**:
- [X] `README.md` - Project overview (recently updated)
- [X] `docs/HOW-IT-WORKS.md` - Non-technical explanation
- [X] `docs/USER-GUIDE.md` - Bot commands
- [X] `docs/TRUST-MODEL.md` - Trust logic
- [X] `docs/DEVELOPER-GUIDE.md` - Technical implementation
- [X] `docs/ALGORITHMS.md` - Graph algorithms
- [X] `docs/PERSISTENCE.md` - State durability & recovery (NEW)
- [X] `docs/THREAT-MODEL-AUDIT.md` - Security analysis
- [X] `docs/spike/SPIKE-WEEK-BRIEFING.md` - Spike Week 1 validation
- [X] `docs/spike/SPIKE-WEEK-2-BRIEFING.md` - Spike Week 2 persistence validation (NEW)
- [X] `docs/FEDERATION.md` - Future federation
- [X] `docs/OPERATOR-GUIDE.md` - Running the bot

---

## Architectural Consistency Checks

### Check 1: Cross-Cluster Requirement
**Question**: Is the scaling cross-cluster requirement consistently documented?

**Where to Check**:
- [X] `.beads/cross-cluster-requirement.bead` - Defines scaling requirement
- [X] `.cursor/rules/vetting-protocols.mdc` - Admission logic
- [X] `docs/TRUST-MODEL.md` - Trust model explanation
- [X] `docs/HOW-IT-WORKS.md` - User-facing explanation
- [X] `docs/USER-GUIDE.md` - Bot behavior

**Expected Consistency**:
- Bridges: 2 vouches from 2 clusters (or "peer circles" in user docs)
- Validators: min(vouch_count, available_clusters) clusters
- Bootstrap exception: 1 cluster suspends requirement

### Check 2: DVR Metric
**Question**: Is the Distinct Validator Ratio consistently defined?

**Where to Check**:
- [X] `.beads/mesh-health-metric.bead` - Metric definition
- [X] `.beads/blind-matchmaker-dvr.bead` - Algorithm
- [X] `.cursor/rules/freenet-contract-design.mdc` - Contract calculation
- [X] `.cursor/rules/user-roles-ux.mdc` - UX display
- [X] `docs/ALGORITHMS.md` - Mathematical definition
- [X] `docs/TRUST-MODEL.md` - Health metrics
- [X] `docs/USER-GUIDE.md` - User explanation
- [X] `README.md` - Overview

**Expected Consistency**:
- Formula: `DVR = Distinct_Validators / (N / 4)`
- Three tiers: 🔴 0-33% / 🟡 33-66% / 🟢 66-100%
- Definition: Non-overlapping voucher sets

### Check 3: Vouch Invalidation
**Question**: Is vouch invalidation logic consistently defined?

**Where to Check**:
- [X] `.beads/security-constraints.bead` - No 2-point swings
- [X] `.cursor/rules/security-guardrails.mdc` - Enforcement
- [X] `.cursor/rules/freenet-contract-design.mdc` - Calculation
- [X] `docs/TRUST-MODEL.md` - Trust standing
- [X] `docs/VOUCH-INVALIDATION-LOGIC.md` - Detailed examples
- [X] `docs/USER-GUIDE.md` - User explanation

**Expected Consistency**:
- Voucher-flaggers excluded from BOTH counts
- No single member can cause 2-point swing
- Standing = Effective_Vouches - Regular_Flags

### Check 4: Bot Governance Model
**Question**: Is bot execute-only role consistently enforced?

**Where to Check**:
- [X] `.beads/governance-model.bead` - Bot role definition
- [X] `.cursor/rules/security-guardrails.mdc` - Operator constraints
- [X] `.cursor/rules/architecture-objectives.mdc` - Core invariant
- [X] `docs/OPERATOR-GUIDE.md` - Operator instructions
- [X] `docs/DEVELOPER-GUIDE.md` - Bot architecture

**Expected Consistency**:
- Bot is Signal admin (technical) but execute-only (no decisions)
- All decisions via group consensus (Signal Polls)
- Operator is service runner only (no privileges)

### Check 5: Signal Polls vs Reactions
**Question**: Is Signal Polls for voting consistently required?

**Where to Check**:
- [X] `.beads/voting-mechanism.bead` - Poll rationale
- [X] `.beads/poll-implementation-gastown.bead` - Implementation
- [X] `.beads/proposal-system.bead` - Proposal workflow
- [X] `.cursor/rules/security-guardrails.mdc` - Voting constraints
- [X] `docs/USER-GUIDE.md` - User commands

**Expected Consistency**:
- Never use reactions (binary only, hard to tally)
- Always use Signal Polls (structured voting, multiple choice)
- Protocol v8 required (libsignal-service-rs fork)

---

## Security Constraint Audit

### Three-Layer Defense
**Question**: Is trust map seizure protection consistently explained?

**Where to Check**:
- [X] `.beads/security-constraints.bead` - Three layers
- [X] `.cursor/rules/security-guardrails.mdc` - Enforcement
- [X] `docs/THREAT-MODEL-AUDIT.md` - Threat model
- [X] `docs/DEVELOPER-GUIDE.md` - Implementation
- [X] `README.md` - Overview

**Expected Layers**:
1. No centralized storage (Freenet distributed)
2. Cryptographic privacy (HMAC hashing, zeroization)
3. Metadata isolation (1-on-1 PMs, operator least-privilege)

### Server Seizure Protection
**Question**: Is ephemeral storage consistently enforced?

**Where to Check**:
- [X] `.beads/security-constraints.bead` - Bot storage
- [X] `.beads/technology-stack.bead` - Presage store
- [X] `.cursor/rules/security-guardrails.mdc` - Storage violations
- [X] `.cursor/rules/tech-stack.mdc` - Store requirements
- [X] `docs/DEVELOPER-GUIDE.md` - Implementation

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
- [X] `docs/spike/SPIKE-WEEK-2-BRIEFING.md` - 8 persistence questions
- [X] `docs/PERSISTENCE.md` - Reciprocal Persistence Network design
- [X] `.beads/persistence-model.bead` - Persistence constraints

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
*Document any "cluster" vs "peer circles" inconsistencies here*

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
- [X] All checklists above marked complete
- [X] Findings section filled with specific issues
- [X] Recommended fixes section has concrete action items
- [X] Go/No-Go decision made with confidence
- [X] If GO: Handoff document prepared for Gastown agents

**Gastown handoff ready when**:
- [X] No terminology confusion
- [X] No architectural contradictions
- [X] Security model airtight
- [x] Spike Week aligned
- [X] Agent can follow guidance literally without ambiguity
