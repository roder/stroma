# Admission Flow Gap Analysis

**Date:** 2026-02-10
**Author:** Polecat obsidian (automated)
**Reference:** st-pxwa, st-xsg epic
**Plan:** `docs/todo/privacy-first_admission_flow_90400c89.plan.md`

---

## 1. Executive Summary

The admission vetting flow is **substantially complete** with the core architecture fully implemented. All four st-xsg critical deliverables have been closed (st-6p3, st-7q0, st-mpf, st-t88). The primary remaining gaps are in documentation/terminology consistency and minor code refinements.

| Metric | Value |
|--------|-------|
| **Overall Completion** | ~85% |
| **Plan TODOs Complete** | 6/9 (67%) |
| **st-xsg Dependencies** | 4/4 closed (100%) |
| **Security Constraints Met** | 7/8 (88%) |
| **Critical Gaps** | 1 (assessor PM message missing `/reject-intro` instruction) |
| **Documentation Gaps** | 2 (plan TODO items 8 and 9 partially done) |

---

## 2. Component-by-Component Analysis

### 2.1 BlindMatchmaker (`src/signal/matchmaker.rs`)

**Status: 🔄 PARTIAL**

| Feature | Status | Notes |
|---------|--------|-------|
| Real cluster detection (TrustGraph + detect_clusters) | ✅ COMPLETE | Uses `TrustGraph::from_state()` and `detect_clusters()` |
| DVR-optimal selection | 🔄 PARTIAL | Categorizes DVR-optimal vs cross-cluster vs same-cluster tiers, but does not check voucher set overlap for true DVR optimization |
| Exclusion list for re-selection | ✅ COMPLETE | Accepts `excluded: &HashSet<MemberHash>` parameter |
| Bridge + Validator candidates | ✅ COMPLETE | Filters `effective_vouches >= 2` (bridges and validators) |
| Centrality-based sorting | ✅ COMPLETE | Sorts each tier by `graph.centrality()` descending |
| Bootstrap exception (single cluster) | ✅ COMPLETE | Falls back to any member except inviter |
| Returns `Option<MemberHash>` | ✅ COMPLETE | Returns None when no candidates available |
| `select_validator_with_exclusions()` eliminated | ✅ COMPLETE | Only `select_validator()` exists (st-t88 closed) |

**Gap Detail:** The current DVR categorization puts all non-excluded cross-cluster members in the "dvr_optimal" tier without actually checking voucher set overlap against existing distinct validators. Per the plan, Phase 0 should select members "whose voucher sets don't overlap with existing distinct validators." The current implementation uses cluster membership as the primary discriminator, which is a reasonable approximation but not the full DVR algorithm described in `blind-matchmaker-dvr.bead`.

**Severity:** Low. The current implementation provides correct cross-cluster selection. Full DVR optimization is a Phase 0 enhancement that improves network resilience but doesn't affect functional correctness.

---

### 2.2 handle_invite() (`src/signal/bot.rs:239-399`)

**Status: ✅ COMPLETE**

| Feature | Status | Notes |
|---------|--------|-------|
| Freenet verification (sender is member) | ✅ COMPLETE | Queries contract state, hashes sender, checks membership |
| Previous flags query (GAP-10) | ✅ COMPLETE | Checks `state.ejected` and `state.flags` |
| Record first vouch in Freenet | ✅ COMPLETE | Creates `StateDelta` with `add_vouch()` |
| Ephemeral VettingSession creation | ✅ COMPLETE | With invitee, inviter, context, flags |
| BlindMatchmaker selection | ✅ COMPLETE | Calls `select_validator()` with real Freenet state |
| MemberResolver resolution | ✅ COMPLETE | `get_service_id(&assessor)` |
| Assign assessor to session | ✅ COMPLETE | `assign_assessor()` |
| PM to assessor (assessment request) | ✅ COMPLETE | Uses `msg_assessment_request()` |
| PM to inviter (confirmation) | ✅ COMPLETE | Uses `msg_inviter_confirmation()` |
| Bootstrap phase handling | ✅ COMPLETE | Separate `handle_invite_bootstrap()` path |
| No-candidates fallback | ✅ COMPLETE | Bootstrap-style message when no assessor found |

**Minor Issue:** The excluded set is initialized as empty (`HashSet::new()`) on each invite rather than reading from an existing session. This is correct for initial invitations but means previously excluded candidates from a prior `/reject-intro` cycle wouldn't carry over if the same invitee is re-invited after session deletion. This is by design (sessions are ephemeral).

---

### 2.3 handle_vouch() (`src/signal/bot.rs:441-719`)

**Status: ✅ COMPLETE**

| Feature | Status | Notes |
|---------|--------|-------|
| Hash sender to MemberHash | ✅ COMPLETE | `identity::mask_identity()` + `From<MaskedIdentity>` |
| Query Freenet state | ✅ COMPLETE | Full state query and deserialization |
| Verify sender is active member | ✅ COMPLETE | `state.members.contains()` |
| VettingSession lookup | ✅ COMPLETE | `get_session(username)` |
| Prevent self-vouch (inviter can't vouch twice) | ✅ COMPLETE | Checks `voucher_hash == inviter_hash` |
| Cross-cluster requirement | ✅ COMPLETE | Uses `detect_clusters()`, checks cluster IDs |
| Record vouch in Freenet | ✅ COMPLETE | `StateDelta` with `add_vouch()` |
| Check ALL GroupConfig requirements | ✅ COMPLETE | effective_vouches >= threshold AND standing >= 0 |
| STARK proof generation | ✅ COMPLETE | `verify_admission_proof()` |
| Add to Signal group | ✅ COMPLETE | `group_manager.add_member()` |
| Add to Freenet as active member | ✅ COMPLETE | `StateDelta` with `add_member()` |
| Announce admission (hash only) | ✅ COMPLETE | Uses member hash hex |
| GAP-11 cluster formation check | ✅ COMPLETE | `check_and_announce_cluster_formation()` |
| Delete ephemeral session | ✅ COMPLETE | `vetting_sessions.admit()` |
| Notify voucher | ✅ COMPLETE | Detailed response with vouch count |
| Notify inviter | ✅ COMPLETE | Admission notification |
| Below-threshold handling | ✅ COMPLETE | Reports remaining vouches needed |

---

### 2.4 handle_reject_intro() (`src/signal/bot.rs:724-930`)

**Status: ✅ COMPLETE**

| Feature | Status | Notes |
|---------|--------|-------|
| Verify session exists | ✅ COMPLETE | Error message if no session |
| Verify sender is assigned assessor | ✅ COMPLETE | Compares `assessor_id` to sender |
| Add to excluded candidates | ✅ COMPLETE | `session.excluded_candidates.insert()` |
| Reset to PendingMatch | ✅ COMPLETE | Clears assessor, resets status |
| Acknowledgment PM to declining assessor | ✅ COMPLETE | Confirmation message |
| Re-run BlindMatchmaker with exclusion list | ✅ COMPLETE | Queries Freenet, calls `select_validator()` |
| Assign new assessor | ✅ COMPLETE | Full resolution + assignment flow |
| PM to new assessor | ✅ COMPLETE | Uses `msg_assessment_request()` |
| Stalled state (no candidates) | ✅ COMPLETE | Sets `VettingStatus::Stalled`, notifies inviter |
| Notify inviter of re-matching | ✅ COMPLETE | Success and failure paths |

---

### 2.5 VettingSession / VettingSessionManager (`src/signal/vetting.rs`)

**Status: ✅ COMPLETE**

| Feature | Status | Notes |
|---------|--------|-------|
| Ephemeral (RAM only, never persisted) | ✅ COMPLETE | In-memory HashMap |
| VettingSession struct fields | ✅ COMPLETE | invitee_id, invitee_username, inviter, inviter_id, context, assessor, assessor_id, status, has_previous_flags, previous_flag_count, excluded_candidates |
| `excluded_candidates: HashSet<MemberHash>` | ✅ COMPLETE | Per plan requirement |
| VettingStatus enum | ✅ COMPLETE | PendingMatch, AwaitingVouch, Stalled, Admitted, Rejected |
| Create/get/assign/admit/reject methods | ✅ COMPLETE | Full lifecycle management |
| `assessor_declined()` helper | ✅ COMPLETE | Adds to exclusions, resets status |
| Duplicate session prevention | ✅ COMPLETE | Returns error if session exists |
| Session deletion on admit/reject | ✅ COMPLETE | `sessions.remove()` |

**Note:** The plan mentions a `Declined` state that transitions back to `PendingMatch`. The implementation handles this as a reset to `PendingMatch` directly (without a separate `Declined` variant), which is functionally equivalent and simpler.

---

### 2.6 PM Command Parsing (`src/signal/pm.rs`)

**Status: ✅ COMPLETE**

| Feature | Status | Notes |
|---------|--------|-------|
| `/reject-intro @username` command | ✅ COMPLETE | `RejectIntro { username }` variant in Command enum |
| Parsing with username extraction | ✅ COMPLETE | `parts[1]` extraction |
| Missing username -> Unknown | ✅ COMPLETE | Falls through to `Command::Unknown` |
| Routing to StromaBot handler | ✅ COMPLETE | Handled in `StromaBot::handle_message()`, not `handle_pm_command()` |

---

### 2.7 MemberResolver (`src/signal/member_resolver.rs`)

**Status: ✅ COMPLETE**

| Feature | Status | Notes |
|---------|--------|-------|
| Transient in-memory mapping | ✅ COMPLETE | HashMap, never persisted |
| Bidirectional MemberHash <-> ServiceId | ✅ COMPLETE | Both hash_to_id and id_to_hash |
| HMAC with bot ACI-derived key | ✅ COMPLETE | Uses `mask_identity()` with pepper |
| Rebuild from roster | ✅ COMPLETE | `rebuild_from_roster()` method |
| Zeroize on clear/drop | ✅ COMPLETE | `ServiceId.0.zeroize()` in `clear()` and `Drop` |
| Add/remove member | ✅ COMPLETE | Both methods implemented |

---

### 2.8 Privacy-Safe Message Templates (`src/signal/vetting.rs:226-382`)

**Status: 🔄 PARTIAL**

| Template | Status | Notes |
|----------|--------|-------|
| Assessment request (to assessor) | 🔄 PARTIAL | Present but missing explicit `/reject-intro` instruction |
| Inviter confirmation | ✅ COMPLETE | Privacy-safe, no assessor identity |
| Rejection acknowledgment | ✅ COMPLETE | `msg_rejection_ack()` |
| Vouch recorded (below threshold) | ✅ COMPLETE | `msg_vouch_recorded()` |
| No candidates (stalled) | ✅ COMPLETE | `msg_no_candidates()` |
| Admission success | ✅ COMPLETE | `msg_admission_success()` |

**Gap Detail:** The `msg_assessment_request()` function says "If you wish to vouch for them, use: `/vouch {username}`. If you prefer not to vouch, no action is needed." Per the plan, it should explicitly mention `/reject-intro @invitee` as the decline option: "Use `/vouch @[invitee]` to confirm, or `/reject-intro @[invitee]` to pass." The current message implies passive decline rather than active re-selection.

**Severity:** Medium. The assessor can still use `/reject-intro` (the command exists and works), but they won't know about it from the assessment request PM. This means assessors who want to decline must either already know the command or simply take no action, leaving the session in `AwaitingVouch` indefinitely rather than triggering re-selection.

---

### 2.9 Freenet State Stream Integration (`src/signal/bot.rs:89-125`)

**Status: ✅ COMPLETE**

| Feature | Status | Notes |
|---------|--------|-------|
| Real-time subscription | ✅ COMPLETE | `freenet.subscribe()` in `run()` |
| `tokio::select!` for dual monitoring | ✅ COMPLETE | Signal messages + Freenet stream |
| ProposalExpired handling | ✅ COMPLETE | Full poll termination + outcome flow |
| MemberVetted handling | ✅ COMPLETE | Add to group + announce |
| MemberRevoked handling | ✅ COMPLETE | Remove from group + announce |

---

### 2.10 handle_status() (`src/signal/pm.rs:574-717`)

**Status: ✅ COMPLETE**

| Feature | Status | Notes |
|---------|--------|-------|
| Real Freenet query | ✅ COMPLETE | Queries contract state |
| Trust metrics calculation | ✅ COMPLETE | effective_vouches, regular_flags, standing |
| Role determination | ✅ COMPLETE | Invitee/Bridge/Validator |
| Voucher hash display (first 8 chars) | ✅ COMPLETE | Privacy-safe partial hashes |
| Third-party query rejection (GAP-04) | ✅ COMPLETE | Rejects `/status @username` |

---

## 3. Plan TODO Status

| # | TODO ID | Description | Status | Evidence |
|---|---------|-------------|--------|----------|
| 1 | `enhance-blind-matchmaker` | Enhance BlindMatchmaker with real cluster detection, DVR-optimal selection, exclusion list | 🔄 PARTIAL | Real cluster detection and exclusion list: ✅. Full DVR voucher-set optimization: not implemented (uses cluster-tier approximation) |
| 2 | `implement-invite-flow` | Complete handle_invite() with Freenet verification, flags, resolution, PM, session update | ✅ COMPLETE | `bot.rs:239-399` - full implementation |
| 3 | `reject-intro-command` | Add /reject-intro command with parsing, handler, re-selection | ✅ COMPLETE | `pm.rs:172-179` (parsing), `bot.rs:724-930` (handler) |
| 4 | `implement-vouch-flow` | Complete handle_vouch() with cross-cluster, Freenet recording, STARK proof, admission | ✅ COMPLETE | `bot.rs:441-719` - full implementation |
| 5 | `vetting-messages` | Create privacy-safe PM message templates | 🔄 PARTIAL | 6/6 templates exist. Assessment request missing `/reject-intro` instruction |
| 6 | `vetting-session-exclusions` | Extend VettingSession with excluded_candidates and Stalled status | ✅ COMPLETE | `vetting.rs:51` (excluded_candidates), `vetting.rs:63` (Stalled) |
| 7 | `member-resolver` | Create MemberResolver for transient MemberHash <-> ServiceId mapping | ✅ COMPLETE | `member_resolver.rs` - full implementation with zeroize |
| 8 | `update-terminology-assessor` | Update terminology to use 'assessor' throughout beads and docs | 🔄 PARTIAL | Beads updated (terminology.bead, vetting-protocols.bead, blind-matchmaker-dvr.bead, signal-integration.bead, user-roles-ux.bead). Docs updated (HOW-IT-WORKS.md, USER-GUIDE.md, TRUST-MODEL.md, ALGORITHMS.md). Cursor rules status unknown |
| 9 | `update-docs-reject-intro` | Add /reject-intro to beads and docs | 🔄 PARTIAL | Beads: signal-integration.bead ✅, user-roles-ux.bead ✅, vetting-protocols.bead ✅. Docs: USER-GUIDE.md ✅, HOW-IT-WORKS.md ✅. Cursor rules status unknown |

---

## 4. st-xsg Dependency Status

| Bead | Title | Status | Close Reason |
|------|-------|--------|--------------|
| st-6p3 | Fix handle_reject_intro() - BROKEN | ✅ CLOSED | Resolved in PR #101 |
| st-7q0 | Integrate Freenet state stream in run() | ✅ CLOSED | Already completed in commit 0b158d7. tokio::select! monitoring both Signal and Freenet |
| st-t88 | Eliminate select_validator_with_exclusions() | ✅ CLOSED | Merged via PR #100. Only `select_validator()` remains |
| st-mpf | Fix handle_status() stub | ✅ CLOSED | Replaced hardcoded placeholder with real Freenet query |

**Note:** All four dependencies are closed. The code reviewed in this analysis reflects the post-merge state on `main` which includes all these fixes.

---

## 5. Security Constraint Compliance

| # | Constraint | Status | Evidence |
|---|-----------|--------|----------|
| 1 | No cleartext Signal IDs stored or logged | ✅ MET | MemberResolver uses HMAC, zeroizes on clear/drop. `member_resolver.rs:164-171` |
| 2 | VettingSession ephemeral (RAM only) | ✅ MET | In-memory HashMap, deleted on admit/reject. `vetting.rs:77-80` |
| 3 | Bot never creates secondary Signal groups | ✅ MET | All communication via `send_message()` (1-on-1 PM) or `send_group_message()` (single group) |
| 4 | Validator selection uses real cluster detection | ✅ MET | `TrustGraph::from_state()` + `detect_clusters()` in `matchmaker.rs:45-46` |
| 5 | Cross-cluster requirement enforced (with bootstrap exception) | ✅ MET | `handle_vouch()` checks cluster IDs (bot.rs:498-518); bootstrap exception when `cluster_count < 2` |
| 6 | STARK proof generated and verified before admission | ✅ MET | `verify_admission_proof()` called in handle_vouch (bot.rs:597-606) |
| 7 | No grace periods | ✅ MET | Immediate ejection in `check_ejection()` and `handle_flag()` |
| 8 | Inviter identity never revealed to assessor | 🔄 PARTIAL | `msg_assessment_request()` does NOT reveal inviter - says "You've been selected to assess a candidate." However, the phrase "Someone has invited" from the plan spec is not used. Functionally compliant. |
| 9 | Assessor identity never revealed to inviter | ✅ MET | `msg_inviter_confirmation()` says "An assessor has been selected" - no identity |

---

## 6. Recommendations (Priority-Ordered)

### P1: Fix Assessment Request Message (Medium Impact)

**File:** `src/signal/vetting.rs:271-278`

Update `msg_assessment_request()` to include `/reject-intro` as an explicit option:

Current:
```
"If you wish to vouch for them, use:\n/vouch {username}\n\nIf you prefer not to vouch, no action is needed."
```

Should be:
```
"When ready: /vouch @{username} to confirm, or /reject-intro @{username} to pass."
```

This is important because without the `/reject-intro` instruction, assessors who decline will simply do nothing, leaving sessions stuck in `AwaitingVouch` rather than triggering re-selection to find a willing assessor.

### P2: Cursor Rules Verification (Low Impact)

Verify that cursor rules (`.cursor/rules/*.mdc`) have been updated with assessor terminology and `/reject-intro` command. The beads and docs are updated, but the cursor rules status is unverified from this analysis.

### P3: Full DVR Voucher-Set Optimization (Low Impact, Future)

The current matchmaker uses cluster-tier categorization as a DVR approximation. For true Phase 0 DVR optimization per `blind-matchmaker-dvr.bead`, the algorithm should check voucher set overlap against existing distinct validators. This is an optimization enhancement, not a functional gap - the current implementation correctly selects cross-cluster assessors.

### P4: Session Timeout Handling (Future Enhancement)

The plan mentions timeout tracking in VettingSession, but no timeout mechanism exists. Sessions in `AwaitingVouch` state will remain indefinitely if the assessor neither vouches nor uses `/reject-intro`. Consider adding a timeout that auto-transitions sessions to `PendingMatch` for re-selection.

### P5: Close Plan TODO Items

Mark plan TODO items 1 (enhance-blind-matchmaker), 5 (vetting-messages), 8 (update-terminology-assessor), and 9 (update-docs-reject-intro) as complete or partially complete in the plan file. Items 2, 3, 4, 6, 7 are fully implemented.

---

## 7. Test Coverage Summary

| Component | Unit Tests | Integration Tests | Notes |
|-----------|-----------|-------------------|-------|
| BlindMatchmaker | 3 tests | - | Simple, no-cross-cluster, empty network |
| handle_invite() | 2 tests | - | Basic invite, invite with flags |
| handle_vouch() | - | - | Tested through bot integration tests |
| handle_reject_intro() | 3 tests | - | Success, not-assessor, no-session |
| VettingSession | 5 tests | - | Create, duplicate, assign, admit, flags |
| MemberResolver | 8 tests | - | Add, determinism, remove, rebuild, bidirectional, pepper isolation |
| Message templates | 3 tests | - | Inviter confirmation (no flags, 1 flag, multiple flags) |
| Bot (general) | 4 tests | - | Creation, text message, admission, ejection |
| STARK proofs | 4 tests | - | Valid, insufficient, negative standing, voucher-flaggers |
| GAP-11 | 4 tests | - | Cluster formation, single cluster, integration, no-repeat |

---

*Generated by polecat obsidian for st-pxwa gap analysis task.*
