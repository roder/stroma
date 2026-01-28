# UX Clarifications Changelog

**Date**: 2026-01-26

## Summary

This document summarizes all rule updates made to clarify user roles, UX flows, and terminology based on critical ambiguities identified in the architecture.

## New Rule Created

### `user-roles-ux.mdc`
**Location**: `.cursor/rules/user-roles-ux.mdc`

**Contents**:
- Complete role definitions (Operator, Members, Invitees, Bridges, Validators, Bot, Seed Group, Federation)
- Detailed user flows (Invitation, Vetting, Vouching, Flagging, Configuration, Federation, Status Queries)
- Comprehensive bot command reference with examples
- Network strength metrics (mesh density calculation and histogram display)
- Bot voice and personality guidelines
- State transition diagrams
- Privacy and security considerations

**Key Clarifications**:
- Invitees (Leaf Nodes) are OUTSIDE the Signal group (1 vouch)
- Bridges are IN the Signal group with 2 vouches (minimum requirement)
- Validators are IN the Signal group with 3+ vouches (no special privileges, used for optimization)
- ANY Member can vouch (not restricted to Validators)
- Invitation itself counts as first vouch (no token exchange)
- Federation uses `config_change_threshold` (no separate threshold)

## Existing Rules Updated

### 1. `vetting-protocols.mdc`

**Critical Fixes**:
- ✅ Fixed "Leaf Node" terminology throughout (now correctly refers to people OUTSIDE group)
- ✅ Clarified "Bridges" = IN group with 2 vouches
- ✅ Clarified "Validators" = IN group with 3+ vouches
- ✅ Removed token exchange system (invitation counts as first vouch)
- ✅ Updated admission flow to reflect simplified invitation process
- ✅ Changed "validators" to "Members" for vouch requirements
- ✅ Added clarification that ANY Member can vouch

**Specific Changes**:
- Line 13-22: Added critical terminology definitions
- Line 54-65: Simplified invitation flow (no token exchange)
- Line 68-76: Updated vetting interview to start immediately
- Line 79-102: Renamed function parameters from `newcomer` to `invitee`
- Line 166-194: Fixed node type analysis (removed incorrect "leaves" category)
- Line 199-245: Updated priority algorithm (Bridges instead of Leaves)
- Line 253: Changed bot response from "Leaf Node (2 vouches)" to "Bridge (2 vouches)"
- Line 327-349: Added comprehensive key architectural principles

### 2. `architecture-objectives.mdc`

**Critical Fixes**:
- ✅ Fixed Core Invariant (changed "validators" to "Members")
- ✅ Updated Vouching System requirements (ANY Member can vouch)
- ✅ Clarified Trust Standing with two ejection triggers
- ✅ Fixed Network Topology & Node Types definitions
- ✅ Removed redundant `federation_approval_threshold` from GroupConfig
- ✅ Updated Federation Decision voting mechanism
- ✅ Clarified Waiting Room as state, not separate chat

**Specific Changes**:
- Line 21: Changed "independent validators" to "independent Members"
- Line 117-124: Updated Gatekeeper section with correct terminology
- Line 127-132: Updated Diplomat to use `config_change_threshold`
- Line 136-146: Complete rewrite of Vouching System section
- Line 142-147: Added two ejection triggers and no cooldown
- Line 148-159: Complete rewrite of Network Topology & Node Types
- Line 168-174: Updated Federation Decision voting
- Line 176-179: Updated Cross-Mesh Vouching terminology
- Line 186-201: Removed `federation_approval_threshold`, added `min_vouch_threshold`
- Line 203-213: Updated Configuration Discovery & Modification
- Line 299-302: Changed "Waiting Room Experience" to "Vetting Experience"

### 3. `signal-integration.mdc`

**Critical Fixes**:
- ✅ Added complete vouch permissions section
- ✅ Clarified Blind Matchmaker role (suggests, doesn't restrict)
- ✅ Updated Gatekeeper Pattern with correct node types
- ✅ Simplified admission flow (no token exchange)
- ✅ Updated member commands with all options
- ✅ Enhanced Signal Poll examples
- ✅ Updated federation notification format

**Specific Changes**:
- Line 14: Added note about ANY Member can vouch
- Line 19-25: Complete rewrite of Gatekeeper Pattern
- Line 26-30: Simplified admission flow
- Line 81-95: Complete rewrite of member commands section
- Line 96-101: Updated voting interface details
- Line 109-135: Enhanced Signal Poll examples
- Line 164-177: Updated federation proposal format
- Line 180-221: Added comprehensive "Vouch Permissions" section

### 4. `security-guardrails.mdc`

**Critical Fixes**:
- ✅ Added constraint: NEVER restrict vouching to only Validators
- ✅ Clarified waiting room as state, not separate chat
- ✅ Updated vouch source constraint

**Specific Changes**:
- Line 101-104: Added new security constraints

### 5. `cluster-terminology.mdc`

**Critical Fixes**:
- ✅ Changed "Leaves" to "Bridges" in priority algorithm
- ✅ Added node type definitions to internal cluster section
- ✅ Clarified vouch permissions
- ✅ Updated function names and parameters

**Specific Changes**:
- Line 22-32: Updated Bot Role section with correct terminology
- Line 85-88: Renamed function parameters
- Line 110-115: Added vouch permissions to design principles

## Key Terminology Changes

### Before → After

| Before | After | Reason |
|--------|-------|--------|
| "Leaf Node (2 vouches)" | "Bridge (2 vouches)" | Leaf Nodes are OUTSIDE group (1 vouch) |
| "Validators must vouch" | "Members can vouch" | ANY Member can vouch, not restricted |
| Token exchange system | Invitation = first vouch | Simplified UX, no token needed |
| "federation_approval_threshold" | Uses "config_change_threshold" | Single consensus threshold for all decisions |
| "Waiting Room" (separate chat) | State of being outside group | No separate space, just not in group yet |
| "newcomer" | "invitee" | Clearer distinction from "member" |

## Node Type Definitions (Clarified)

### Invitees (Leaf Nodes)
- **Location**: OUTSIDE Signal group
- **Status**: Being vetted (1 vouch)
- **Can Vouch**: NO
- **Can Flag**: NO
- **In Waiting Room**: YES (state, not separate chat)

### Bridges
- **Location**: IN Signal group
- **Status**: Fully vetted (2 vouches)
- **Can Vouch**: YES
- **Can Flag**: YES
- **Role**: Minimum requirement for membership

### Validators
- **Location**: IN Signal group
- **Status**: High-trust (3+ vouches)
- **Can Vouch**: YES
- **Can Flag**: YES
- **Role**: Used for Blind Matchmaker optimization (no special privileges)

## User Flow Changes

### Old Invitation Flow
1. Member sends `/invite` → Bot generates token
2. Invitee receives token from inviter
3. Invitee PMs token to bot
4. Bot starts vetting process

### New Invitation Flow
1. Member sends `/invite @username [context]` → Invitation counts as first vouch
2. Bot immediately starts vetting process
3. Bot selects second Member via Blind Matchmaker
4. Bot facilitates introduction
5. Second Member vouches → Admission

**Improvement**: Simpler UX, no token exchange, immediate process start

## Configuration Changes

### Old GroupConfig
```rust
pub struct GroupConfig {
    federation_approval_threshold: f32,    // Removed (2026-01)
    config_change_threshold: f32,          // Used for all decisions
    ejection_appeal_threshold: f32,        // Removed (2026-01) - appeals via re-invite
    min_intersection_density_self: f32,    // Renamed (2026-01)
    validator_percentile: u32,
}
```

### New GroupConfig
```rust
pub struct GroupConfig {
    group_name: String,                    // Added (2026-01)
    config_change_threshold: f32,          // Used for ALL decisions
    default_poll_timeout: Duration,        // Added (2026-01)
    ejection_appeal_threshold: f32,        // Removed (2026-01) - appeals via re-invite
    min_intersection_density: f32,         // Renamed (no "self")
    validator_percentile: u32,
    min_vouch_threshold: usize,            // New - configurable minimum
    config_version: u64,
    last_updated: Timestamp,
}
```

**Changes**:
- Removed redundant `federation_approval_threshold`
- Federation now uses `config_change_threshold`
- Renamed `min_intersection_density_self` → `min_intersection_density`
- Added `min_vouch_threshold` (default: 2)

## Bot Command Reference (New)

### Complete Command Set

| Command | Who Can Use | Effect |
|---------|-------------|--------|
| `/invite @user [context]` | Any Member | Invitation = first vouch, starts vetting |
| `/vouch @user` | Any Member | Vouch for invitee or existing member |
| `/flag @user [reason]` | Members only | Flag member (reason required) |
| `/status` | Any Member | View own trust standing |
| `/status @user` | Any Member | View another member's standing |
| `/mesh` | Any Member | Network overview |
| `/mesh strength` | Any Member | Mesh density histogram |
| `/mesh config` | Any Member | View group configuration |
| `/propose-config key=value` | Any Member | Propose configuration change |
| `/audit operator` | Any Member | View operator action history |

## Network Strength Metric (New)

### Mesh Density Calculation

```rust
fn calculate_mesh_density(members: &[Hash], vouches: &[(Hash, Hash)]) -> f32 {
    let n = members.len();
    let max_possible_vouches = n * (n - 1);  // Full mesh
    let actual_vouches = vouches.len();
    
    (actual_vouches as f32 / max_possible_vouches as f32) * 100.0
}
```

### Histogram Display

```
📈 Network Strength: 38% Mesh Density

Vouch Distribution:
  2 vouches (Bridges):    ████████░░░░ 22 members (47%)
  3-5 vouches:            █████░░░░░░░ 15 members (32%)
  6-10 vouches:           ███░░░░░░░░░  8 members (17%)
  11+ vouches (Validators): ██░░░░░░░░░░  2 members (4%)

Total: 47 members
Actual vouches: 213
Max possible: 552 (full mesh)
Density: 38%
```

## Ejection Triggers (Clarified)

### Two Independent Triggers

1. **Trust Standing**: `Standing = Vouches - Flags < 0`
   - Too many flags relative to vouches
   - Immediate ejection when negative

2. **Vouch Count**: `Vouches < min_vouch_threshold` (default: 2)
   - Voucher(s) left the group
   - Immediate ejection when below threshold

**No Grace Periods**: Both triggers result in immediate removal

**Re-Entry**: Can re-enter immediately by securing 2 new vouches (no cooldown)

## Federation Decision (Clarified)

### Old System
- Separate `federation_approval_threshold` (e.g., 70%)
- Different threshold from config changes

### New System
- Uses single `config_change_threshold` for ALL group decisions
- Simplifies governance model
- Federation is "just another configuration decision"

## Privacy & Security

### Unchanged (Critical Constraints)
- ❌ NEVER store Signal IDs in cleartext
- ❌ NEVER bypass ZK-proof verification
- ❌ NEVER add grace periods for ejection
- ❌ NEVER require admin coordination for federation
- ❌ NEVER make Signal group the source of truth
- ❌ NEVER expose social graph structure

### Added Constraints
- ❌ NEVER restrict vouching to only Validators
- ❌ NEVER create "waiting room" as separate chat

## Impact on Implementation

### Bot Behavior Changes
1. **No token generation** - Invitation directly recorded as first vouch
2. **Immediate vetting start** - No waiting for token exchange
3. **Member selection** - Bot can suggest ANY Member (prefers Validators for optimization)
4. **Single consensus threshold** - All votes use `config_change_threshold`

### Freenet Contract Changes
1. **Remove** `federation_approval_threshold` field
2. **Add** `min_vouch_threshold` field (configurable)
3. **Update** voting logic to use single threshold

### Signal Integration Changes
1. **Simplified command flow** - No token exchange
2. **Updated bot messages** - Correct terminology (Bridge, not Leaf)
3. **Enhanced commands** - More query options (`/mesh strength`, `/audit operator`)
4. **Signal Polls** - All voting via structured polls (not reactions)

## Testing Implications

### New Test Cases Needed
1. Test that ANY Member can vouch (not just Validators)
2. Test that invitation counts as first vouch (no token)
3. Test both ejection triggers independently
4. Test mesh density calculation and histogram generation
5. Test federation voting with `config_change_threshold`
6. Test re-entry with no cooldown

### Updated Test Cases
1. Update vetting flow tests (remove token exchange)
2. Update node type tests (correct Leaf/Bridge/Validator definitions)
3. Update vouch permission tests (allow all Members)
4. Update federation threshold tests (use config_change_threshold)

## Documentation Updates Required

### README.md
- Update user flow diagrams
- Update command reference
- Add mesh density explanation

### Architecture Diagrams
- Fix node type labels (Leaf = outside, Bridge = 2 vouches)
- Update vetting flow (no token exchange)
- Add mesh density visualization

### API Documentation
- Update bot command specifications
- Update GroupConfig schema
- Add mesh density endpoint

## Migration Path (Future Implementation)

### For Existing Groups
1. **Configuration Migration**: Remove `federation_approval_threshold`, use `config_change_threshold`
2. **No Breaking Changes**: Existing members unaffected
3. **Bot Update**: Deploy new bot with updated terminology and flows
4. **Freenet Update**: Migrate contract schema

### For New Groups
1. Start with new GroupConfig schema
2. Use simplified invitation flow from beginning
3. Train users with correct terminology (Bridge, not Leaf)

## Conclusion

These updates resolve all critical ambiguities identified in the UX review:

✅ Leaf Node terminology fixed (OUTSIDE group = 1 vouch)  
✅ Bridge terminology clarified (IN group = 2 vouches)  
✅ Validator privileges clarified (no special permissions)  
✅ Vouch permissions clarified (ANY Member can vouch)  
✅ Invitation flow simplified (no token exchange)  
✅ Federation threshold unified (uses config_change_threshold)  
✅ Mesh density metric defined (histogram with calculation)  
✅ Waiting Room clarified (state, not separate chat)  

All rules are now consistent and unambiguous. Implementation can proceed with clear specifications.
