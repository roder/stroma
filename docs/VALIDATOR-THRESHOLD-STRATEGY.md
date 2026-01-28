# Validator Threshold Strategy: Phased Approach

**Date**: 2026-01-27  
**Status**: Strategic Decision Document  
**Alignment**: Matches project phased approach (Phases 0-4)

## Executive Summary

Validator thresholds will evolve from **fixed (MVP)** → **configurable safeguards (Phase 2)** → **percentage-based scaling (Phase 4+)** to match group maturity and adoption patterns.

**Current Focus**: Small groups (3-30 members) with fixed thresholds.  
**Future Expansion**: Medium/large groups via configurable and percentage-based approaches.

---

## Current Decision: MVP Phase (Phases 0-3)

### Fixed Validator Threshold

**Configuration**:
- **Bridge**: Exactly 2 effective vouches (membership minimum)
- **Validator**: 3+ effective vouches (fixed number)
- **Invitees**: 1 vouch (being vetted)

**Rationale**:
- Simplest implementation (lowest complexity)
- Most transparent to members (easy to understand)
- Adequate for target MVP audience (small affinity groups)
- Lowest governance overhead (no config debates)
- Hardest to game (can't manipulate via group size changes)

**Target Groups**:
- 3-30 members
- Tight-knit affinity groups
- Technical capacity for deployment
- No expectation of rapid scaling

**Mathematical Properties**:
- 3-person seed group: 0% Validators (just Bridges)
- 10-person group: 30% Validators on average
- 20-person group: 20% Validators on average

**Status**: ✅ Implement in MVP

---

## Phase 2 Decision: Medium Groups (Weeks 5-6)

### Introduction of Configurable min_vouch_threshold

**When to Implement**:
- After MVP validates core vetting flow (Phase 1 complete)
- Once operators report consistent group stability
- When first groups approach 30-50 members

**Scope**:
- Allow `/propose stroma min_vouch_threshold` votes
- Configurable range: 2-4 (must be ≥2)
- Requires `config_change_threshold` consensus

**Why Only min_vouch_threshold, NOT validator_percentile**:
- Lower risk: directly tied to ejection logic (well-understood)
- Use case: groups can choose 2 (easier admission) vs 3+ (higher barrier)
- Governance precedent: easier to add validator threshold later
- Prevents gaming: validator count remains stable

**No Percentage-Based Validators Yet**:
- Validator threshold remains fixed (3+)
- Percentile configuration deferred to Phase 4+

**Safeguards**:
- `min_vouch_threshold` changes require full `config_change_threshold` consensus
- Cannot retroactively eject members who fell below new threshold
- New threshold applies to future admissions only

**Status**: 📋 Design (not MVP priority)

**Implementation Trigger**: "Operator feedback indicates stable medium-group operations (50+ members)"

---

## Phase 4+ Decision: Large Groups & Federation (Q2 2026+)

### Addition of Percentage-Based Validator Threshold

**When to Implement**:
- After federation mechanics validated (Phase 4 foundational)
- When groups report validator scaling issues
- When multi-mesh deployments exceed 200 members

**Scope**:
- Add `validator_percentile` to GroupConfig (1-100 range)
- Calculated as: `max(3, group_size * validator_percentile / 100)`
- Example: 20% of 100 members = 20 validators
- Example: 20% of 20 members = 3 validators (floor)

**Why This Matters at Scale**:
- **100-person group, fixed 3+ validators**: Only 3% reach validator status (inefficient)
- **100-person group, 20% validator**: 20 validators available for MST optimization (better)
- **500-person group**: Fixed 3+ = 0.6% vs 20% = 100 validators (massive difference)

**Safeguards**:
- `validator_percentile` changes require **elevated consensus** (85%+ threshold)
- Changes limited to **once per quarter** (prevent gaming)
- Cannot retroactively demote validators (only affects new admissions)
- Minimum always >= 3 (never drop below fixed MVP threshold)

**Federation Context**:
- Percentage-based scaling critical for cross-mesh vouching
- Blind Matchmaker needs sufficient validator diversity
- PSI-CA federation requires stable validator identification

**Status**: 📋 Design (Phase 4+ target)

**Implementation Trigger**: "Multiple federated groups request percentage-based validator scaling"

---

## Decision Matrix

| Aspect | MVP (Now) | Phase 2 | Phase 4+ |
|--------|-----------|---------|---------|
| **Target Group Size** | 3-30 | 30-200 | 200+ |
| **Bridge Threshold** | Fixed (2) | Configurable | Configurable |
| **Validator Threshold** | Fixed (3+) | Fixed (3+) | Percentage-based |
| **Configuration Method** | N/A | Signal Poll | Signal Poll |
| **Governance Overhead** | None | Low | Medium |
| **Attack Surface** | Minimal | Low | Medium |
| **Transparency** | High | High | Medium |

---

## Implementation Roadmap

### Phase 0-3: MVP (Current)
- [ ] Implement fixed Bridge = 2, Validator = 3+
- [ ] Document fixed threshold strategy
- [ ] Gather operator feedback on medium-group performance

### Phase 2 Gate
- [ ] Monitor: Do groups stabilize at 30-50 members?
- [ ] Decision: Is `min_vouch_threshold` configurability needed?
- [ ] If YES: Implement configurable min_vouch_threshold
- [ ] If NO: Continue with fixed thresholds

### Phase 4 Gate
- [ ] Monitor: Do 200+ member groups request percentage-based thresholds?
- [ ] Monitor: Does federation reveal validator scaling issues?
- [ ] Decision: Is percentage-based validator threshold needed?
- [ ] If YES: Implement `validator_percentile` with safeguards
- [ ] If NO: Continue with configurable min_vouch_threshold only

---

## Why This Phased Approach

### Principle: Ship the Simplest Thing That Works

1. **MVP is simple**: Fixed thresholds require zero governance
2. **Phase 2 adds flexibility**: But only for the most-needed parameter
3. **Phase 4 optimizes**: Percentage-based scaling only when necessary

### Avoid Premature Complexity

- Not implementing percentage-based validators in MVP (no current need)
- Configurable min_vouch_threshold deferred until medium-group feedback
- Graduated rollout means we learn from each phase before next

### Governance Cost Management

- MVP: Zero governance decisions about thresholds
- Phase 2: One new config parameter (low overhead)
- Phase 4: Only activated when federation demand exists

---

## Success Criteria for Each Phase

### MVP Success
- ✅ Small groups (3-30) operate stably with fixed thresholds
- ✅ Validators organically emerge (no forced thresholds)
- ✅ MST algorithm works with available validators
- ✅ Blind Matchmaker suggests good pairings

### Phase 2 Success (if needed)
- ✅ Medium groups (30-200) can configure min_vouch_threshold
- ✅ No gaming or unintended consequences
- ✅ Operator feedback indicates satisfaction
- ✅ Configuration changes rare (< 1 per year per group)

### Phase 4 Success (if needed)
- ✅ Large groups (200+) benefit from percentage-based validators
- ✅ Federation works smoothly with percentage-based identification
- ✅ MST optimization significantly better than fixed thresholds
- ✅ No evidence of gaming via validator_percentile manipulation

---

## Safeguards Against Gaming

### MVP (Fixed Thresholds)
- Cannot game validator count (fixed at 3+)
- Cannot game via config changes (no configurability)
- Gaming vector: **Sybil attacks** (addressed by MST, 2-vouch requirement)

### Phase 2 (Configurable min_vouch_threshold)
- Can change min_vouch_threshold, but:
  - Requires group-wide consensus (`config_change_threshold`)
  - Cannot retroactively eject existing members
  - Only affects future admissions
- Gaming vector: **Lower minimum to flood with bad actors**
- Defense: **Consensus requirement + vetting still requires 2 independent vouches**

### Phase 4 (Percentage-Based validator_percentile)
- Can change validator_percentile, but:
  - Requires elevated consensus (85%+)
  - Limited to once per quarter
  - Cannot retroactively change existing validators
  - Minimum always >= 3 (MVP floor preserved)
- Gaming vector: **Lower percentile to reduce validator population**
- Defense: **Elevated threshold + quarterly limit + MST diversity requirement**

---

## Open Questions for Future Phases

### Before Phase 2 Implementation
- [ ] Do small groups naturally reach 30-50 members?
- [ ] What percentage become Validators at current fixed threshold?
- [ ] Do operators request min_vouch_threshold changes?
- [ ] Are there observed downsides to fixed thresholds?

### Before Phase 4 Implementation
- [ ] How many groups exceed 200 members?
- [ ] Do federated groups report scaling issues?
- [ ] Is fixed 3+ validator threshold limiting MST optimization?
- [ ] Would percentage-based validator selection improve bridge density?

---

## Documentation Anchors

**Related Documents**:
- `docs/TRUST-MODEL.md` - Current fixed threshold definitions
- `docs/ALGORITHMS.md` - MST algorithm assuming validator availability
- `.cursor/rules/architecture-objectives.mdc` - Phased implementation strategy

**Key Code References**:
- `graph-analysis.mdc` (lines 21-42) - Threshold calculation logic
- `docs/TRUST-MODEL.md` (lines 372-403) - GroupConfig structure

---

**Status**: Decision finalized for MVP; Phase 2 and Phase 4 gates to be reviewed as project matures.  
**Last Updated**: 2026-01-27  
**Owner**: Project Architecture
