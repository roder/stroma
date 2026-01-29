# Stroma User Guide

**For Group Members**

This guide explains how to use Stroma as a member of a trust network. You interact with Stroma through a bot in Signal - the messaging app you already use.

## Quick Start

1. **Get invited** by an existing member (counts as first vouch)
2. **Chat with a member from a different cluster** (10-15 min introduction)
3. **Get vouched** by that member (must be from different cluster than inviter)
4. **Join the group** automatically when you reach 2 cross-cluster vouches

**Note**: For small groups (3-5 members) that only have one cluster, the cross-cluster requirement is not yet enforced. Once the group grows and develops 2+ distinct clusters (typically 6+ members), cross-cluster vouching becomes mandatory.

## Daily Interaction

Everything happens through simple commands in 1-on-1 private messages with the bot:

### Inviting Someone New

```
You → Bot (1-on-1 PM): /invite @Friend "Great organizer from local community"

Bot → You:
"Your invitation for @Friend has been recorded as the first vouch.
I'm now reaching out to a member from a different cluster for the 
cross-cluster vouch. Same-cluster vouches don't count — we need 
independent perspectives for security. I'll keep you updated."

(Note: In small groups with only one cluster, the bot will find any available member for the second vouch. Cross-cluster enforcement activates once the group develops 2+ distinct clusters.)
```

### Checking Your Status

```
You → Bot: /status

Bot → You:
"📊 Your Trust Status
Role: Bridge
All vouches: 2 (Alice, Bob)
All flags: 0
Voucher-flaggers: 0
Effective vouches: 2 ✅
Regular flags: 0
Standing: +2 (positive)

💡 Tip: You're at minimum effective vouch threshold. Building more 
connections will increase your resilience. If a voucher 
leaves the group OR flags you, you'll need immediate replacement vouches."
```

### Viewing Network Health

```
You → Bot: /mesh strength

Bot → You:
"📈 Mesh Health: 100/100 ✅

Network Balance: 🟢 OPTIMAL
Mesh Density: 38% (target range: 30-60%)

Vouch Distribution:
  2 vouches (Bridges):    ████████░░░░ 22 members (47%)
  3-5 vouches:            █████░░░░░░░ 15 members (32%)
  6-10 vouches:           ███░░░░░░░░░  8 members (17%)
  11+ vouches (Validators): ██░░░░░░░░░░  2 members (4%)

Total: 47 members
Actual vouches: 820
Max possible: 2,162 (full mesh)

💡 Your network is in the optimal balance range!
This provides strong resilience without over-connection.
A 100% mesh would actually reduce network health by
creating excessive interdependence."
```

## Bot Commands Reference

### Invitation & Vetting

#### `/invite @username [context]`
**Invites someone new to the group**

- Your invitation counts as their first vouch
- Bot starts vetting process immediately
- Bot selects second member from a DIFFERENT CLUSTER for introduction
- Same-cluster vouches don't count (cross-cluster required)
- **Bootstrap exception**: Small groups (3-5 members) with only 1 cluster are exempt until 2+ clusters exist
- Context helps the second member understand who they're meeting

**Example:**
```
/invite @Matt "Local activist, met at community organizing event"
```

#### `/vouch @username`
**Vouches for an invitee or existing member**

- During vetting: Vouches for invitee after introduction
- After admission: Strengthens mesh by vouching for existing members
- Bot verifies you're a current member

**Example:**
```
/vouch @Matt
```

### Trust Management

#### `/flag @username [reason]`
**Flags a member (opposite of vouch)**

- Only members can flag (not invitees)
- Reason is required (for your records)
- If you flagged someone you vouched for, your vouch is invalidated
- Bot recalculates their standing immediately
- Standing < 0 → automatic ejection

**Example:**
```
/flag @Bob "Posted spam links repeatedly"
```

**Vouch Invalidation Note:**
If you previously vouched for someone and then flag them:
- Your vouch is invalidated (you can't both trust and distrust)
- Their effective vouch count decreases by 1
- May trigger ejection if they drop below 2 effective vouches

### Status Queries

#### `/status`
**Shows your own trust standing**

Returns:
- Role (Invitee, Bridge, or Validator)
- All vouches count
- All flags count
- Voucher-flaggers count (if any)
- Effective vouches (after invalidation)
- Regular flags (excluding voucher-flaggers)
- Standing score

#### `/status @username`
**Shows another member's standing**

Same info as `/status` but for specified member (privacy settings may apply)

#### `/mesh`
**Shows network overview**

Returns:
- Total members
- Mesh health score
- Federation status
- Your position in network
- Network health summary

#### `/mesh strength`
**Shows detailed mesh health with histogram**

Returns:
- Mesh Health Score (0-100)
- Network Balance status (🔴🟡🟢)
- Raw mesh density percentage
- Vouch distribution histogram
- Interpretation guidance

#### `/mesh config`
**Shows current group configuration**

Returns all configurable parameters:
- Consensus thresholds
- Federation parameters
- Trust thresholds
- Config version and last updated

### Configuration

#### `/propose` - Unified Proposal System
**Proposes any group decision (config changes, federation)**

- Creates Signal Poll for group vote (anonymous voting)
- Requires `config_change_threshold` approval (e.g., 70%)
- Bot applies change automatically if approved
- All changes logged with timestamps

**Subcommands:**

**`/propose config <setting> <value>`** - Signal group settings:
- `name` - Group name
- `description` - Group description
- `disappearing_messages` - Message retention (e.g., 24h)

**`/propose stroma <setting> <value>`** - Stroma trust settings:
- `config_change_threshold` (0.5-1.0) - Approval threshold for all proposals
- `default_poll_timeout` (duration) - Default poll timeout if not specified
- `min_intersection_density` (0.0-1.0) - Federation overlap threshold
- `validator_percentile` (1-100) - Validator threshold percentile
- `min_vouch_threshold` (≥2) - Minimum effective vouches to stay in group

**`/propose federate <group-id>`** - Federation:
- Proposes federation with another Stroma group

**Example:**
```
/propose stroma min_intersection_density 0.15
```

### Audit

#### `/audit operator`
**Shows operator action history**

Returns recent operator service operations (restarts, maintenance, etc.)

Confirms operator has no special privileges for membership or configuration.

## Understanding Trust Roles

### Invitees (Leaf Nodes)
**Status**: OUTSIDE Signal group, being vetted

- Have 1 vouch from the member who invited them
- Need 1 more vouch from a different member
- Receive 1-on-1 PMs from bot during vetting
- Can chat with validators during introduction
- Cannot vouch, flag, or vote

**Transition**: When 2 vouches confirmed → automatically added to group as Bridge

### Bridges
**Status**: IN Signal group, minimum trust

- Have exactly 2 effective vouches
- Full member privileges (invite, vouch, flag, vote)
- At risk if voucher leaves OR flags (need immediate replacement)
- Bot may suggest building more connections

**Tip**: Build 3+ connections to become Validator for more resilience

### Validators
**Status**: IN Signal group, high trust

- Have 3+ effective vouches
- Same privileges as Bridges (no special powers)
- More resilient to voucher changes
- Preferred by Blind Matchmaker for strategic introductions
- Bot uses them for mesh optimization

**Note**: Validators don't have extra permissions - just higher resilience

## Understanding Trust Standing

### The Formula

```
All_Vouchers = Set of members who vouched for you
All_Flaggers = Set of members who flagged you
Voucher_Flaggers = All_Vouchers ∩ All_Flaggers (contradictory)

Effective_Vouches = |All_Vouchers| - |Voucher_Flaggers|
Regular_Flags = |All_Flaggers| - |Voucher_Flaggers|
Standing = Effective_Vouches - Regular_Flags
```

### What This Means

**Effective Vouches**: Real vouches that count (excludes voucher-flaggers)  
**Regular Flags**: Flags from people who didn't vouch for you  
**Voucher-Flaggers**: People who both vouched AND flagged (contradictory - invalidates their vouch)

### Ejection Triggers (Automatic & Immediate)

**Trigger 1**: `Standing < 0` (too many flags relative to vouches)  
**Trigger 2**: `Effective_Vouches < 2` (dropped below minimum threshold)

Either trigger → immediate ejection, no warnings, no grace period

### Example Scenarios

**Scenario 1: Flagged by Non-Voucher (Stay in Group)**
- Vouches: 2 (Alice, Bob)
- Flags: 1 (Carol - never vouched for you)
- Voucher-flaggers: 0
- Effective vouches: 2 ✅
- Regular flags: 1
- Standing: +1 ✅
- **Result**: Stay in group

**Scenario 2: Flagged by Voucher (May Trigger Ejection)**
- Vouches: 2 (Alice, Bob)
- Flags: 1 (Alice - who vouched for you)
- Voucher-flaggers: 1 (Alice)
- Effective vouches: 1 ❌ (Alice's vouch invalidated)
- Regular flags: 0
- Standing: +1
- **Result**: EJECTED (Trigger 2: effective vouches < 2)

**Scenario 3: Both Vouchers Flag You (Definite Ejection)**
- Vouches: 2 (Alice, Bob)
- Flags: 2 (Alice, Bob)
- Voucher-flaggers: 2 (both)
- Effective vouches: 0 ❌
- Regular flags: 0
- Standing: 0
- **Result**: EJECTED (Trigger 2: effective vouches < 2)

**Scenario 4: Many Flags (Ejected via Standing)**
- Vouches: 3 (Alice, Bob, Carol)
- Flags: 5 (Dave, Eve, Frank, Grace, Hank)
- Voucher-flaggers: 0
- Effective vouches: 3 ✅
- Regular flags: 5
- Standing: -2 ❌
- **Result**: EJECTED (Trigger 1: standing < 0)

### Re-Entry After Ejection

If ejected, you can re-enter by:
1. Ask current members to invite you (`/invite @You`)
2. Complete vetting process again
3. Get 2 new effective vouches
4. Automatic admission when threshold met

**No cooldown period** - you can re-enter immediately

## What You See vs What You Don't See

### You See (Transparent)
- Your trust standing and role
- Network health metrics
- Configuration settings
- Bot's conversational responses
- Strategic introduction suggestions

### You Don't See (Hidden Complexity)
- Freenet contracts and ComposableState
- HMAC hashing and zeroization
- STARK proof generation/verification
- Merkle Tree generation
- Summary-delta synchronization
- State stream monitoring

The bot abstracts all technical complexity. You just use simple Signal commands.

## Privacy & Security

### How Your Group is Protected

**The Real Threat**: What happens if a state-level adversary or compromised operator tries to seize the trust map to identify members?

**Three-Layer Defense**:

1. **No Single Place to Seize**
   - Trust map stored across distributed Freenet network (not one server)
   - Adversary would need to seize multiple peers to reconstruct

2. **Only Hashes, Not Identities**
   - Your Signal ID is hashed immediately (can't be reversed)
   - Bot memory contains only hashes, never cleartext
   - Even if server compromised, adversary only gets hashes

3. **No Signal Metadata**
   - All vetting in 1-on-1 private messages (not group chat)
   - Operator can't manually export or query trust map
   - No logs of why people trust each other

**Result**: Even if adversary compromises the bot or server, they only get:
- Hashes (not your real identity)
- Group size and connection patterns (not who you actually are)
- Vouch counts (not relationship details)

### What's Protected
- **Your Identity**: Never stored in cleartext, always hashed
- **Trust Relationships**: Distributed across network, can't be seized from one place
- **Vetting Conversations**: Deleted after admission (ephemeral)
- **Group Metadata**: All operations in 1-on-1 PMs (not group chat)

## Common Questions

### Can the operator see who I am?
No. The operator sees only hashed identifiers, not your real Signal ID. Even if coerced, they can't reveal your identity.

### Can the operator add/remove people?
No. Only the bot can add/remove people, and only based on Freenet contract state. Operator can't manually override this.

### What if the server is seized by police/government?
They would only get hashes (not identities) and connection patterns (not relationship details). Your real identity remains protected because it's never stored in cleartext.

### What if my voucher leaves the group?
You need an immediate replacement vouch to stay in. Bot monitors this automatically and will notify you.

### What if someone flags me?
Your standing is recalculated. If standing < 0 OR effective vouches < 2, you're ejected immediately.

### Can I see who vouched for me?
Yes, the `/status` command shows your vouchers (as hashes, not full identities for privacy).

### How do I build more trust?
Ask for strategic introductions via `/mesh` suggestions, or ask members to vouch for you (`/vouch @You`).

### Why isn't cross-cluster required in my small group?
Cross-cluster vouching is enforced once your group has 2+ distinct clusters (typically 6+ members). During bootstrap phase (3-5 members), everyone is in the same cluster, so cross-cluster isn't possible yet. As your group grows and develops separate "friend circles," the bot will start enforcing cross-cluster vouches to prevent infiltration.

### What does "Mesh Health 100/100" mean?
Your network is in the optimal balance range (30-60% density). This is the goal - not 100% mesh density!

### Why is 100% mesh density bad?
If everyone vouches for everyone, the network becomes over-connected. Members lose individual trust signals and the network can't grow. 30-60% density provides better balance.

---

**See Also:**
- [Trust Model Deep Dive](TRUST-MODEL.md) - Mathematical details and edge cases
- [Federation Overview](FEDERATION.md) - Future multi-group connections (Phase 4+)
