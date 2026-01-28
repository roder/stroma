# Security Analysis: Bot Storage Threat Model

**Date**: 2026-01-28  
**Issue**: Presage SqliteStore persists message history (server seizure risk)  
**Resolution**: Custom minimal ProtocolStore requirement

---

## The Critical Finding

**User Question (2026-01-28):**
> "I do have concerns about using Presage, since it has SQLite, which if the bot's server was seized by an adversary, what records would be at risk? All conversations, and thus all trust records?"

**Answer**: YES - this is a critical security vulnerability that wasn't adequately addressed by our existing rules.

---

## What Default Presage SqliteStore Stores

**From Presage documentation and source:**

```rust
// Default SqliteStore persists:
pub struct SqliteStore {
    // Protocol state (required)
    sessions: Table<Session>,
    pre_keys: Table<PreKey>,
    identity_keys: Table<IdentityKey>,
    
    // Message history (PROBLEMATIC)
    messages: Table<Message>,          // ❌ All vetting conversations
    
    // Contact database (PROBLEMATIC)
    contacts: Table<Contact>,          // ❌ Links to Signal IDs
    
    // Group metadata (PROBLEMATIC)
    groups: Table<Group>,              // ❌ Group structure
}
```

### Server Seizure Impact (With Default Store)

**If bot server is seized, adversary gets:**

1. **Complete Message History**
   - All vetting conversations
   - Who invited whom with what context
   - "Great activist from local organizing" → relationship details
   - Timestamps and patterns

2. **Contact Database**
   - Signal IDs (or easily reversible links)
   - Names, phone numbers
   - Profile information

3. **Group Metadata**
   - Group structure
   - Member lists
   - Conversation patterns

**This completely undermines Stroma's anonymity-first architecture.**

---

## Why Existing Rules Didn't Catch This

### What Our Rules Covered

**✅ Identity Layer** (Adequate):
- "Never store Signal IDs in cleartext"
- "HMAC hash immediately"
- "Zeroize buffers"

**✅ Memory Layer** (Adequate):
- Immediate zeroization
- Memory dumps contain only hashes

**✅ Application Layer** (Adequate):
- "Ephemeral state: Relationship data deleted after vetting"
- Freenet is source of truth

### What Our Rules MISSED

**❌ Signal Client Persistence Layer** (Gap):
- Didn't specify what bot's Signal client stores
- Didn't address message history persistence
- Assumed "ephemeral vetting" meant no persistence anywhere
- Focused on "don't store Signal IDs" not "don't store messages"

**The Gap:**

Our rules said:
```
"Ephemeral State: Relationship data deleted after vetting threshold met"
```

**But didn't explicitly say:**
```
"Bot's Signal client must NOT persist message history"
"Implement custom Store that stores ONLY protocol state"
```

### Why This Happened

**Assumption Error:**
- We focused on "what Stroma application stores" (Freenet contract)
- We didn't think about "what Signal library stores" (Presage's SqliteStore)
- We saw "ephemeral vetting" as application-level concern
- We didn't extend it to the persistence layer of dependencies

**This is a valuable lesson**: Security constraints must address ALL persistence layers, not just application state.

---

## The Fix: Minimal Custom Store

### Architecture Decision

**Use Presage Manager API but with custom StromaProtocolStore**

### What Gets Stored

**On Disk (Encrypted, ~100KB):**
- Signal session keys (for encryption continuity)
- Pre-keys (for new conversations)
- Identity keys (bot's Signal identity)

**In Memory Only (Ephemeral):**
- Vetting conversations (processed, never written)
- Signal IDs (hashed immediately, zeroized)
- Message content (command processed, then discarded)

**NEVER Stored:**
- ❌ Message history
- ❌ Vetting conversation transcripts
- ❌ Contact database
- ❌ Invitation context or reasons
- ❌ Signal IDs (even encrypted)

### Implementation Pattern

```rust
pub struct StromaProtocolStore {
    // Protocol state for encryption
    sessions: HashMap<ServiceId, Session>,
    pre_keys: HashMap<u32, PreKey>,
    identity_keys: IdentityKeyPair,
    
    // Minimal encrypted file
    state_file: EncryptedProtocolState,  // ~100KB
    
    // Message handling (ephemeral)
    current_session_messages: Vec<EphemeralMessage>,  // In-memory, cleared after processing
}

impl presage::Store for StromaProtocolStore {
    // Implement protocol requirements
    async fn load_session(&self, id: &ServiceId) -> Result<Session> {
        Ok(self.sessions.get(id).cloned())
    }
    
    async fn save_session(&mut self, id: ServiceId, session: Session) -> Result<()> {
        self.sessions.insert(id, session);
        self.persist_protocol_state().await  // Small file update
    }
    
    // DO NOT implement message persistence
    // presage::Store trait may have optional message methods
    // Leave them unimplemented or return empty
}
```

### Server Seizure Result (With Minimal Store)

**Adversary gets:**
- ~100KB encrypted file
- Passphrase required to decrypt
- Contains only protocol state (sessions, keys)

**Adversary does NOT get:**
- Message content
- Vetting conversations
- Relationship context
- Signal IDs
- Contact information

**This aligns with Stroma's threat model.**

---

## Updated Security Constraints

### Added to `.beads/security-constraints.bead` Section 10:

**Bot Storage Constraints**:
- ❌ NEVER persist message history
- ❌ NEVER use default SqliteStore
- ❌ NEVER store vetting conversations
- ✅ Implement custom minimal ProtocolStore
- ✅ Store ONLY Signal protocol state
- ✅ Encrypt protocol state file

### Added to `.cursor/rules/security-guardrails.mdc`:

**Bot Storage Constraints (BLOCK - Server Seizure Threat)**:
- Same constraints as above
- Explicit blocking patterns
- Enforcement patterns
- Server seizure threat model

### Added to `.beads/technology-stack.bead`:

**Custom Store Requirement**:
- Implementation guidance
- Code examples
- Why necessary

### Added to `docs/DEVELOPER-GUIDE.md`:

**Bot Storage (CRITICAL - Server Seizure Protection)**:
- Full implementation pattern
- Explanation of gap in previous rules
- Testing requirements

### Updated `.beads/poll-implementation-gastown.bead`:

**Storage Security Requirement**:
- Agent-Signal must use custom store
- DO NOT use presage-store-sqlite
- Testing checklist

---

## Why This Is Critical

**Network Topology IS Social Structure** (User's insight Q4):

The trust map reveals:
- Who knows whom
- How they're connected
- Relationship strengths (vouch counts)
- Community structure (clusters)

**Message history adds:**
- WHY people trust each other ("Great activist...")
- Relationship context
- Conversation patterns
- Identity hints

**Together**: Complete deanonymization of the network.

**With minimal store**: Adversary only gets topology (hashes + counts), not identities or context.

---

## Lesson Learned

**Security rules must be comprehensive across ALL layers:**

| Layer | Constraint |
|-------|-----------|
| Application | Don't store Signal IDs in app code ✅ |
| Memory | Zeroize sensitive buffers ✅ |
| Freenet | Trust map in decentralized storage ✅ |
| **Signal Client** | **Don't persist message history** ← WAS MISSING |

**The gap**: We focused on application-level constraints but didn't drill into dependency persistence.

**The fix**: Explicitly constrain ALL persistence layers, including third-party libraries.

---

## Action Items (Completed)

- [x] Add Section 10 to security-constraints.bead (Bot Storage)
- [x] Update security-guardrails.mdc (Storage violations)
- [x] Update technology-stack.bead (Custom store requirement)
- [x] Update poll-implementation-gastown.bead (Agent-Signal guidance)
- [x] Update DEVELOPER-GUIDE.md (Storage security section + gap explanation)
- [x] Update README.md (Security auditor description)
- [x] Create this analysis document

---

## Gastown Agent Guidance

**Agent-Signal MUST:**
1. Use Presage Manager API (high-level operations)
2. Implement custom StromaProtocolStore (minimal persistence)
3. DO NOT use presage-store-sqlite dependency
4. Store ONLY protocol state (~100KB encrypted file)
5. Never persist message content or history
6. Test: Verify no messages in storage after vetting

**This is now explicitly required in:**
- `.beads/security-constraints.bead` (immutable constraint)
- `.beads/technology-stack.bead` (implementation guidance)
- `.beads/poll-implementation-gastown.bead` (Agent-Signal task)

---

**Status**: Security vulnerability identified and mitigated via architectural constraints.  
**Result**: All documentation, rules, and beads updated to prevent implementation of vulnerable pattern.
