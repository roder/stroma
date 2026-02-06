# Presage Integration - Critical Blocker Update

**Date**: 2026-02-05
**Agent**: stromarig/polecats/quartz
**Status**: 🔴 BLOCKED - Infrastructure Issue

## Root Cause Identified

The presage incompatibility is caused by a **missing branch on GitHub**.

### The Problem

Two branches exist in the libsignal-service-rs fork (roder/libsignal-service-rs):

1. **`feature/protocol-v8-polls-fixed`** (commit 532a8d6) ✅ ON GITHUB
   - OLD API: methods in `push_service` module
   - AccountManager::new takes 2 params (service, profile_key)
   - Presage does NOT work with this

2. **`feature/protocol-v8-polls-rebased`** (commit 498c03d) ❌ LOCAL ONLY
   - NEW API: methods in `websocket` module
   - AccountManager::new takes 3 params (service, websocket, profile_key)
   - Presage DOES work with this

### Why This Matters

- Presage was written for the NEW API (post-websocket-migration)
- Our Cargo.toml uses the OLD branch (only one on GitHub)
- The NEW branch exists locally but was never pushed to GitHub
- Result: 62 compilation errors

### Evidence

```bash
# Check GitHub branches (only old branch exists)
$ git ls-remote https://github.com/roder/libsignal-service-rs
532a8d6...  refs/heads/feature/protocol-v8-polls-fixed  # OLD API

# Check local branches (new branch exists but not pushed)
$ cd /tmp/libsignal-service-rs && git branch -a
* feature/protocol-v8-polls-rebased  # NEW API (local only!)
  feature/protocol-v8-polls-fixed     # OLD API

# Key commit that migrated methods to websocket
$ git log --oneline | grep -i migrate
a673ad9 Migrate PushService methods to websocket (#366)
# ^ This commit is in rebased branch but NOT in fixed branch
```

### Attempted Fixes

I tried to fix presage for the OLD API by:
1. ✅ Removing websocket parameter from AccountManager::new
2. ✅ Adding RNG parameter to update_pre_key_bundle
3. ❌ But presage imports from `websocket::account` which doesn't exist in OLD API

The imports are fundamentally incompatible:
```rust
// Presage expects (NEW API):
use libsignal_service::websocket::account::*;
use libsignal_service::websocket::registration::*;

// But OLD branch has:
use libsignal_service::push_service::account::*;
use libsignal_service::push_service::registration::*;
```

Fixing all the imports would require rewriting significant portions of presage.

## Required Action

**Someone with push access to `roder/libsignal-service-rs` needs to:**

```bash
cd /path/to/local/libsignal-service-rs-fork
git checkout feature/protocol-v8-polls-rebased
git push origin feature/protocol-v8-polls-rebased
```

Then update Stroma's Cargo.toml:
```toml
[patch."https://github.com/whisperfish/libsignal-service-rs"]
libsignal-service = {
    git = "https://github.com/roder/libsignal-service-rs",
    branch = "feature/protocol-v8-polls-rebased"  # Use rebased branch!
}
```

## Alternative Solutions

If push access is not available:

### Option A: Fork libsignal-service-rs to stromarig
1. Fork `roder/libsignal-service-rs` to `stromarig/libsignal-service-rs`
2. Push the rebased branch to stromarig fork
3. Update Cargo.toml to use stromarig fork

### Option B: Rewrite presage imports (NOT RECOMMENDED)
- Requires changing ~50+ import statements
- Requires testing all presage functionality
- Creates maintenance burden
- Estimated effort: 8-12 hours

## Recommendation

**Adopt Option A: Fork libsignal-service-rs to stromarig**

Rationale:
- Gives us full control over the dependency
- Allows pushing the rebased branch immediately
- Aligns with forking presage (already planned)
- No rewriting required - just infrastructure changes

## Next Steps

**Waiting for decision from witness/mayor:**
1. Who has access to push to roder/libsignal-service-rs?
2. If no one, should we fork to stromarig/libsignal-service-rs?
3. Once branch is available on GitHub, presage should compile successfully

## Files Changed

- `/tmp/presage` - Fixed presage for OLD API (partial, incomplete)
- `Cargo.toml` - Temporarily using local presage path for testing

## Commits

Presage branch: `feature/stroma-libsignal-compat` (commit da5b50ba9)
- Fixed AccountManager::new calls (2 params)
- Added RNG to update_pre_key_bundle
- But still blocked by import incompatibilities

---

**Current Status**: Blocked waiting for infrastructure decision
**Estimated Time to Unblock**: 30 minutes (if branch can be pushed)
**Alternative Time**: 4-8 hours (if we fork and set up infrastructure)
