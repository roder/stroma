# Stroma User Guide

**For Group Members**

> **Pre-alpha**: This guide describes Stroma's designed behavior. The trust model, commands, and protocol logic are implemented and tested against mocks. Real Signal and Freenet integration is in progress. See [TODO.md](todo/TODO.md) for current status.

This guide explains how to use Stroma as a member of a trust network. You interact with Stroma through a bot in Signal -- the messaging app you already use.

## Key Concepts

Before diving in, here are the terms you'll need:

| Term | Meaning |
|------|---------|
| **Group** | Your Signal group — the chat where members communicate |
| **Network** | The web of trust relationships — who vouches for whom |
| **Vouch** | A personal endorsement — you stake your reputation on someone |
| **Flag** | The opposite of a vouch — indicates you no longer trust someone |
| **Cluster** | A peer circle within the network — people who know each other from the same social context |
| **Cross-cluster** | From *different* peer circles — required for admission and ongoing membership |
| **Bridge** | A member with 2 vouches from 2 clusters* — the minimum to be in the group |
| **Validator** | A member with 3+ vouches from 3+ clusters* — well-connected across the network |
| **Standing** | Your trust score: effective vouches minus regular flags (must stay positive) |

*****Cross-cluster requirement**: Vouches must come from as many distinct clusters as your vouch count (up to available clusters). Bridges need 2 clusters; Validators need 3+ clusters. When only 1 cluster exists, the requirement is suspended.

For detailed explanations, see [How It Works](HOW-IT-WORKS.md).

## Quick Start

1. **Get invited** by an existing member (counts as first vouch)
2. **An assessor reaches out** — the bot selects a member from a different cluster to contact you
3. **Get vouched** by the assessor (must be from different cluster than inviter)
4. **Join the group** automatically when you meet the admission threshold

**Note**: For small groups (3-5 members) that only have one cluster, the cross-cluster requirement is not yet enforced. Once the group grows and develops 2+ distinct clusters (typically 6+ members), cross-cluster vouching becomes mandatory.

## Daily Interaction

Everything happens through simple commands in 1-on-1 private messages with the bot:

### Inviting Someone New

```
You → Bot (1-on-1 PM): /invite @Peer "Great organizer from local community"

Bot → You:
"Your invitation for @Peer has been recorded as the first vouch.
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
"📈 Network Health: 🟢 Healthy (75%)

Well-connected members: 9 / 12 possible
Network Size: 47 members
Peer circles: 4 detected

Trust Distribution:
  2 connections:  ████████░░░░ 22 members (47%)
  3+ connections: ███████░░░░░ 25 members (53%)

💡 Your network has strong distributed trust!
Members with 3+ connections from different peer circles
are harder for bad actors to compromise — they'd need to
fool multiple independent groups of people.

To improve: Introduce people from different peer circles
to each other. 3 more well-connected members would make
your network even stronger."
```

## Bot Commands Reference

### Invitation & Vetting

#### `/invite @username [context]`
**Invites someone new to the group**

- Your invitation counts as their first vouch
- Bot starts vetting process immediately
- Bot selects a cross-cluster **assessor** to evaluate the invitee
- The assessor is NOT told who invited (Blind Matchmaker)
- You are NOT told which assessor was selected
- Context helps the assessor understand who they're evaluating
- **Bootstrap exception**: Small groups (3-5 members) with only 1 cluster are exempt until 2+ clusters exist

**Example:**
```
/invite @Matt "Local activist, met at community organizing event"
```

#### `/vouch @username`
**Vouches for an invitee or existing member**

- During vetting: Vouches for invitee after assessment
- After admission: Strengthens mesh by vouching for existing members
- Bot verifies you're a current member
- Admission requires meeting ALL GroupConfig thresholds (vouch count, cross-cluster, standing)

#### `/reject-intro @username`
**Declines an assessment request** (assessor only)

- Only usable by the member who received the assessment request from the bot
- Bot selects another cross-cluster member to assess the invitee
- No penalty for declining — participation is voluntary

**Example:**
```
/reject-intro @Matt
```

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
- Trust health (DVR percentage)
- Replication health (chunk status)
- Federation status
- Subcommand hints (`/mesh strength`, `/mesh replication`, `/mesh config`, `/mesh settings`)

#### `/mesh strength`
**Shows detailed network health metrics (trust)**

Returns:
- Distinct Validator Ratio (DVR) percentage
- Health status (🔴 Unhealthy / 🟡 Developing / 🟢 Healthy)
- Distinct Validators count vs maximum possible
- Vouch distribution histogram
- Improvement suggestions

#### `/mesh replication`
**Shows replication health (is data resilient?)**

Returns:
- Replication status (🟢 Replicated / 🟡 Partial / 🔴 At Risk / 🔵 Initializing)
- Last state change timestamp
- Fragments distributed (e.g., "3/3")
- Recovery confidence (Yes/No)
- Write permission status

Example response:
```
"💾 Replication Health: 🟢 Replicated

Last State Change: 3 hours ago (Alice joined)
State Size: 512KB (8 chunks)
Chunks Replicated: 8/8 (all 3/3 copies) ✅
State Version: 47

Recovery Confidence: ✅ Yes — all chunks available from multiple holders

💡 Your trust network is resilient. If this bot goes offline,
the state can be recovered from chunk holders."
```

#### `/mesh config`
**Shows current group configuration**

Returns all configurable parameters:
- Consensus thresholds
- Federation parameters
- Trust thresholds
- Config version and last updated

#### `/mesh settings`
**Shows all available configuration keys**

Returns comprehensive list of all configurable settings:
- Stroma trust settings (min_vouches, max_flags, thresholds, etc.)
- Signal group settings (name, description, disappearing messages, etc.)
- Valid value ranges for each key
- Current values
- Poll options (binary vs multi-option)

Example response:
```
⚙️ Available Configuration Keys

📋 Stroma Settings (/propose stroma <key> <value>):
  min_vouches: 2 (range: 1-10) - Minimum vouches for standing
  max_flags: 3 (range: 1-10) - Maximum flags before ejection
  open_membership: false (true/false) - Allow new members
  default_poll_timeout_secs: 172800 (3600-604800) - Default timeout
  config_change_threshold: 0.70 (0.50-1.00) - Vote threshold
  min_quorum: 0.50 (0.25-1.00) - Minimum participation

📡 Signal Settings (/propose signal <key> <value>):
  name: "Group Name" (1-32 chars) - Group display name
  description: "..." (0-480 chars) - Group description
  disappearing_messages: off (off, 1h, 1d, 7d, 14d, 30d, 90d) - Message timer
  announcements_only: false (true/false) - Admin-only messages

📊 Poll Options:
  Signal polls support up to 10 options.
  Binary: /propose signal <key> <value> (creates Approve/Reject poll)
  Multi:  /propose signal --key <key> --value <v1> --value <v2> ... (post-UAT)

💡 Example: /propose signal disappearing_messages 7d --timeout 48h
```

### Configuration

#### `/propose` - Unified Proposal System
**Proposes any group decision (config changes, federation)**

- Creates Signal Poll for group vote
- Requires `config_change_threshold` approval (e.g., 70%)
- Bot applies change automatically if approved
- All changes logged with timestamps

**Subcommands:**

**`/propose signal <setting> <value>`** - Signal group settings:
- `name` (1-32 chars) - Group display name
- `description` (0-480 chars) - Group description
- `disappearing_messages` (off, 1h, 1d, 7d, 14d, 30d, 90d) - Message retention timer
- `announcements_only` (true/false) - Only admins can send messages

**`/propose stroma <setting> <value>`** - Stroma trust settings:
- `min_vouches` (1-10) - Minimum vouches for full standing
- `max_flags` (1-10) - Maximum flags before ejection
- `open_membership` (true/false) - Whether new members can join
- `default_poll_timeout_secs` (3600-604800) - Default poll timeout in seconds
- `config_change_threshold` (0.50-1.00) - % of votes needed to pass proposals
- `min_quorum` (0.25-1.00) - % of members who must vote for quorum

**`/propose federate <group-id>`** - Federation:
- Proposes federation with another Stroma group (Phase 4+, not yet implemented)

**Examples:**
```
/propose signal disappearing_messages 7d --timeout 48h
/propose signal name "Privacy Advocates" --timeout 72h
/propose stroma min_vouches 3 --timeout 48h
```

**Tip:** Use `/mesh settings` to see all available configuration keys with their current values and valid ranges.

### Audit

#### `/audit operator`
**Shows operator action history**

Returns recent operator service operations (restarts, maintenance, etc.)

Confirms operator has no special privileges for membership or configuration.

## Understanding Trust Roles

### Invitees (Leaf Nodes)
**Status**: OUTSIDE Signal group, being vetted

- Have 1 vouch from the member who invited them
- Need 1 more vouch from a member in a DIFFERENT CLUSTER
- Same-cluster vouches don't count toward the cross-cluster minimum, unless there is only 1 cluster (small group)
- An **assessor** (selected by the bot) reaches out to them independently
- Cannot vouch, flag, or vote

**Transition**: When GroupConfig admission requirements met → automatically added to group as Bridge

### Bridges
**Status**: IN Signal group, minimum trust

- Have exactly 2 effective vouches
- Full member privileges (invite, vouch, flag, vote)
- At risk if voucher leaves OR flags (need immediate replacement)
- Bot may suggest building more connections

**Tip**: Build 3+ connections to become Validator for more resilience

### Validators
**Status**: IN Signal group, high trust

- Have 3+ effective vouches from 3+ clusters (when available)
- Same privileges as Bridges (no special powers)
- More resilient to voucher changes
- Both Bridges and Validators can be selected as **assessors** for invitees
- Bot prioritizes creating **distinct Validators** via mesh optimization (non-overlapping voucher sets)

**Why "distinct" matters**: The bot suggests vouchers that would create Validators whose voucher sets don't overlap with others. This maximizes network resilience — if one voucher is compromised, it only affects one Validator, not multiple.

**Note**: Validators don't have extra permissions - just higher resilience and better network health contribution

### Assessor (Transient Role)
**Status**: A member currently evaluating an invitee

- Selected by the Blind Matchmaker from a different cluster than the inviter
- Can be a Bridge or Validator — both are eligible
- Bot PMs them with the invitee's contact info and context
- They decide how to contact the invitee and what to reveal about themselves
- Can vouch (`/vouch @invitee`) or decline (`/reject-intro @invitee`)
- The inviter's identity is NOT revealed to the assessor

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
- ZK-proof circuits (STARK AIR definitions, hash-based commitments)
- Merkle Tree generation
- Summary-delta synchronization
- State stream monitoring

The bot abstracts all technical complexity. You just use simple Signal commands.

## Privacy & Security

### How Your Group is Protected

**The Real Threat**: What happens if a state-level adversary or compromised operator tries to seize the trust map to identify members?

**Three-Layer Defense** (designed):

1. **Decentralized Storage** (Freenet -- in progress)
   - Trust state will live across Freenet's peer-to-peer network, not on a single server
   - Adversary would need to compromise multiple peers to reconstruct the full map
   - Currently: trust state is in encrypted local databases while Freenet integration is completed

2. **Only Hashes, Not Identities** (implemented)
   - Your Signal ID is hashed immediately via HMAC-SHA256 with an operator-derived key (BIP-39 mnemonic, HKDF)
   - The original ID is zeroized from memory after hashing
   - Even if the server is compromised, the adversary gets hashes -- not identities

3. **Metadata Isolation** (implemented)
   - All vetting happens in 1-on-1 private messages (not group chat)
   - The bot stores no message history -- encrypted SQLite holds only protocol state
   - Operator cannot manually export or query the trust map
   - No logs of why people trust each other

**Result**: Even if an adversary compromises the bot server, they get:
- Encrypted databases with cryptographic hashes (not real identities)
- Connection patterns (not who you actually are)
- No message history, no vetting conversations, no relationship context

### What's Protected
- **Your Identity**: Never stored in cleartext -- hashed via HMAC-SHA256, originals zeroized
- **Trust Relationships**: Stored as hashes in encrypted databases; Freenet distribution planned
- **Vetting Conversations**: Deleted after admission (ephemeral, never persisted)
- **Message History**: Not stored -- the bot's SQLite store no-ops all message persistence

## Common Questions

### Can the operator see who I am?
No. The operator sees only hashed identifiers, not your real Signal ID. The hash is one-way (HMAC-SHA256) -- it cannot be reversed to recover the original identity.

### Can the operator add/remove people?
No. Only the bot can add/remove members, and only based on contract state (Freenet when live, local encrypted database during pre-alpha). The operator has no commands for membership management.

### What if the server is seized by police/government?
They would get encrypted SQLite databases containing cryptographic hashes (not identities) and connection patterns (not relationship details). Your real identity is never stored in cleartext. No message history is retained. Once Freenet integration is live, even the local databases become secondary to the distributed state.

### What if my voucher leaves the group?
You need an immediate replacement vouch to stay in. Bot monitors this automatically and will notify you.

### What if someone flags me?
Your standing is recalculated. If standing < 0 OR effective vouches < 2, you're ejected immediately.

### Can I see who vouched for me?
Yes, the `/status` command shows your vouchers by name. You already know this information (you participated in the vetting conversations), so revealing it to you doesn't leak new information. You cannot, however, see who vouched for *other* members — that would compromise their privacy.

### How do I build more trust?
Ask for strategic introductions via `/mesh` suggestions, or ask members to vouch for you (`/vouch @You`).

### What does "Replication Health: 🟢 Replicated" mean?
This means your trust network data is safely backed up across multiple peers. If the bot crashes, the data can be fully recovered. Check with `/mesh replication` for details.

### What if Replication Health shows 🔴 At Risk?
This means the trust data couldn't be fully replicated after the last change. The bot is blocked from making further changes until replication succeeds. This is automatic — the bot will keep retrying. If it persists, there may be network connectivity issues.

### Why isn't cross-cluster required in my small group?
Cross-cluster vouching is enforced once your group has 2+ distinct clusters (typically 6+ members). During bootstrap phase (3-5 members), everyone is in the same cluster, so cross-cluster isn't possible yet. As your group grows and develops separate "peer circles," the bot will start enforcing cross-cluster vouches to prevent infiltration.

### What happens when cross-cluster vouching activates?
When the bot detects that your group has developed 2+ distinct clusters (using Bridge Removal algorithm), it will announce the activation in the group chat:

> "📊 Network update: Your group now has distinct sub-communities! Cross-cluster vouching is now required for new members. Existing members are grandfathered."

**What this means:**
- **New invitees**: Must receive vouches from members in different clusters (as always intended)
- **Existing members**: Keep their current status (grandfathered in, no need to get new vouches)
- **Cluster detection**: Runs automatically on every membership change (fast, <1ms)
- **One-time announcement**: Bot only announces this once when ≥2 clusters first detected

This is called **GAP-11** in the implementation and ensures the cross-cluster requirement activates at the right time without disrupting existing members.

### What does "Network Health: 🟢 Healthy (75%)" mean?
The percentage shows your **Distinct Validator Ratio (DVR)** — what fraction of maximum possible distinct Validators your network has achieved. "Distinct" means Validators whose voucher sets don't overlap. Higher DVR = more independent verification = better resilience.

### What are the health levels?
- 🔴 **Unhealthy (0-33%)**: Trust is concentrated — bot actively suggests cross-cluster introductions
- 🟡 **Developing (33-66%)**: Growing toward optimal — bot suggests improvements opportunistically
- 🟢 **Healthy (66-100%)**: Strong distributed trust — maintenance mode

### Why does DVR matter more than "density"?
A network can have high connection density but low resilience if all connections are within one cluster. DVR measures what actually matters: how many members are verified by completely independent sets of vouchers. This directly measures resistance to coordinated infiltration attacks.

---

**See Also:**
- [How It Works](HOW-IT-WORKS.md) -- Plain-language protocol explanation
- [Trust Model](TRUST-MODEL.md) -- Mathematical details and edge cases
- [Threat Model](THREAT-MODEL.md) -- Security design, attack resistance, accepted risks
- [Federation](vision/FEDERATION.md) -- Future multi-group connections (Phase 4+)
