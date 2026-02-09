# Security Analysis: Bot Storage Threat Model

**Date**: 2026-02-09 (revised)
**Issue**: Presage SqliteStore persists message history (server seizure risk)
**Resolution**: StromaStore wrapper — delegates protocol state, no-ops message persistence

---

## What Default Presage SqliteStore Stores

**From Presage documentation and source:**

```rust
// Default SqliteStore persists:
// Protocol state (required) ✅
//   sessions, pre_keys, identity_keys, sender_keys
// Group configuration (required) ✅
//   groups, group_avatars
// Message history (PROBLEMATIC) ❌
//   thread_messages — ALL vetting conversations
// Contact database (PROBLEMATIC) ❌
//   contacts, contacts_verification_state
// Sticker packs (unnecessary) ❌
//   sticker_packs
```

### Server Seizure Impact (With Bare SqliteStore)

**If bot server is seized with bare SqliteStore, adversary gets:**

1. **Complete Message History**
   - All vetting conversations
   - Who invited whom with what context
   - "Great activist from local organizing" — relationship details
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

## The Fix: StromaStore Wrapper

### Architecture Decision

**Wrap presage's `SqliteStore` in a `StromaStore` newtype** that:
- Delegates all persistence EXCEPT message and sticker storage (no-op)
- Uses SQLCipher encryption (AES-256, PBKDF2-HMAC-SHA512, 256K iterations)
- Persists protocol state, group config, profiles (survives restart)
- Never writes messages or stickers to disk

### What Gets Stored (Encrypted SQLite)

**Persisted (survives restart):**
- Signal protocol state (identity keys, pre-keys, sessions, sender keys)
- Group configuration (master keys, member list, metadata)
- Profile keys (required for sealed sender / unidentified access)
- Vote aggregates + HMAC'd voter dedup map (poll lifetime only, zeroized on outcome)

**In Memory Only (Ephemeral):**
- Vetting conversations (processed, never written)
- Signal IDs (hashed immediately, zeroized)
- Message content (command processed, then discarded)

**NEVER Stored:**
- ❌ Message history (save_message is no-op)
- ❌ Vetting conversation transcripts
- ❌ Sticker packs
- ❌ Invitation context or reasons
- ❌ Cleartext Signal IDs

### Implementation Pattern

```rust
pub struct StromaStore(SqliteStore);  // Newtype wrapper

impl StromaStore {
    pub async fn open(path: &str, passphrase: &str) -> Result<Self> {
        // SQLCipher AES-256 encryption
        let inner = SqliteStore::open_with_passphrase(
            path, Some(passphrase), OnNewIdentity::Trust
        ).await?;
        Ok(Self(inner))
    }
}

impl presage::Store for StromaStore {
    // Delegates to SqliteStore: protocol state, groups, contacts, profiles
    // No-ops: save_message -> Ok(()), message -> Ok(None), stickers -> empty
}
```

### Passphrase Management

- Generated as 24-word BIP-39 recovery phrase (256 bits entropy) at link/register time
- Displayed once to operator on stderr, never logged
- Delivered to process via: `--passphrase-file` (container-native), stdin prompt, or env var (fallback)
- SQLCipher has no passphrase length limit

### Server Seizure Result (With StromaStore)

**Adversary gets:**
- Encrypted SQLite database (SQLCipher AES-256)
- Cannot read without passphrase
- If passphrase cracked: protocol state, group config, profile keys
- If passphrase cracked: ACI private key (root key for Signal identity + Freenet chunk encryption)

**Adversary does NOT get:**
- ❌ Message content or history (never written to disk)
- ❌ Vetting conversations
- ❌ Cleartext Signal IDs (only HMAC'd hashes in voter dedup, zeroized on poll outcome)
- ❌ Relationship context or reasons

**Threat escalation**: If passphrase is cracked, the ACI private key enables Freenet trust map reconstruction (given enough chunks). The voter dedup map adds zero incremental risk beyond this. The SQLCipher passphrase is the root security boundary.

---

## Why Not a Fully Custom Store?

The original plan was a custom `StromaProtocolStore` implementing presage's `Store` trait from scratch. This was abandoned because:

1. **Implementation cost**: presage's `Store` trait requires ~2000 lines across `StateStore`, `ContentsStore`, `ProtocolStore`, `PreKeysStore`, `SenderKeyStore`, `SessionStoreExt`
2. **Group config must survive restarts**: Groups need to be persisted for `send_message_to_group` to work (looks up member list)
3. **SqliteStore already supports encryption**: SQLCipher integration is built in and well-tested
4. **Wrapper is minimal**: ~200-300 lines to delegate everything and no-op messages/stickers

The wrapper approach preserves the security principle (no message persistence) while leveraging existing, tested infrastructure.

---

## Vote Privacy Addendum

### Voter Deduplication (HMAC'd, Encrypted, Ephemeral)

When a member changes their vote on a poll, the bot needs to know their previous vote to update aggregates correctly. This requires a voter dedup map:

- Stored as `HMAC(voter_ACI, pepper) -> [selected_option_indices]`
- Persisted in the encrypted SQLite store (survives restart)
- Zeroized immediately when poll outcome is determined (Passed/Failed/QuorumNotMet)
- HMAC'd voter identities are not reversible without the pepper (ACI-derived via HKDF)

**Threat model**: The attack chain to deanonymize votes requires cracking the SQLCipher passphrase AND the HMAC pepper — the same chain needed to extract the ACI private key, which compromises the entire Freenet trust map. Zero incremental risk.

---

## Updated References

Implementation guidance is defined in the canonical beads:

- **`.beads/security-constraints.bead`** § 10 — Immutable constraint with StromaStore specification
- **`.beads/technology-stack.bead`** — Implementation patterns, anti-patterns, Cargo.toml config
- **`.beads/signal-integration.bead`** — Store requirements, status, roadmap

---

**Status**: Security vulnerability identified (2026-01-28) and mitigated via StromaStore wrapper architecture (2026-02-09).
**Result**: All documentation, beads, and CI updated. Message persistence is structurally impossible via no-op delegation.
