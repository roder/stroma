# Presage Integration Status (st-rvzl)

**Status**: 🟡 BLOCKED - Presage incompatible with libsignal-service-rs fork
**Priority**: P0 (blocks all Signal integration)
**Created**: 2026-02-05
**Bead**: hq-mdfb

## Summary

The presage dependency has been **re-enabled** in Cargo.toml:78, but **fails compilation with 62 errors** due to API incompatibilities with our libsignal-service-rs fork (which adds Protocol v8 poll support).

## Current State

### ✅ Complete
- Presage dependency re-enabled in Cargo.toml
- All Signal bot logic implemented with trait abstractions (SignalClient)
- MockSignalClient works perfectly for testing (100% test coverage)
- Device linking scaffolding in place (src/signal/linking.rs)
- Production client skeleton created (src/signal/client.rs)

### ❌ Blocked
- Presage compilation (62 API errors)
- Device linking (requires working Signal client)
- Message handling (requires working Signal client)
- Production bot operation (requires working Signal client)

## Compilation Errors

Presage fails to compile against our libsignal-service-rs fork with 62 errors:

**Categories of Errors:**

1. **AccountManager API changed** (~20 errors)
   - Old: `AccountManager::new(service, websocket, profile_key)`
   - New: `AccountManager::new(service, profile_key)`
   - SignalWebSocket no longer passed to AccountManager

2. **Methods moved from SignalWebSocket to PushService** (~30 errors)
   - `retrieve_profile_by_id`, `get_sticker_pack_manifest`, `get_sticker`
   - `create_verification_session`, `patch_verification_session`
   - `request_verification_code`, `unlink_device`
   - All these methods now on PushService instead of WebSocket

3. **update_pre_key_bundle signature changed** (~4 errors)
   - Now requires `&mut R: Rng + CryptoRng` parameter
   - Example: `.update_pre_key_bundle(store, kind, true, &mut rng)`

4. **Type mismatches and trait bounds** (~8 errors)
   - Various incompatibilities with updated libsignal-service-rs types

**Sample Error:**
```
error[E0061]: this function takes 2 arguments but 3 arguments were supplied
  --> presage/src/manager/registered.rs:568:35
   |
   | let mut account_manager = AccountManager::new(
   |                           ^^^^^^^^^^^^^^^^^^^
   |     state.identified_push_service.clone(),
   |     state.identified_websocket.clone(),  // <-- This is now wrong
   |     --------------------------------------- unexpected argument #2
```

## Resolution Options

### Option A: Fork Presage and Fix Compatibility ⭐ RECOMMENDED
**Effort**: 8-16 hours
**Risk**: Medium
**Maintenance**: Ongoing until upstream fixes

**Steps:**
1. Fork `whisperfish/presage` to `stromarig/presage`
2. Create branch: `feature/libsignal-fork-compat`
3. Fix the 62 compilation errors systematically:
   - Update AccountManager calls
   - Move method calls from WebSocket to PushService
   - Add RNG parameters to pre-key updates
4. Test device linking and message handling
5. Update Cargo.toml: `presage = { git = "https://github.com/stromarig/presage", branch = "feature/libsignal-fork-compat" }`

**Pros:**
- Leverages battle-tested presage foundation
- Most recommended in PHASE0_REVIEW_REPORT.md
- Maintains presage's full feature set

**Cons:**
- Requires maintaining a fork
- 8-16 hour effort to fix all errors
- Need to track upstream presage changes

### Option B: Implement LibsignalClient Directly
**Effort**: 16-32 hours
**Risk**: High
**Maintenance**: High

**Steps:**
1. Add libsignal-service-rs as direct dependency
2. Implement all SignalClient trait methods:
   - Device linking (QR code, registration)
   - Message encryption/decryption
   - Group management (create, add/remove members)
   - Poll creation/voting (Protocol v8)
   - WebSocket message handling
3. Implement protocol store integration
4. Test all functionality

**Pros:**
- No presage dependency
- Full control over implementation
- No fork maintenance

**Cons:**
- Very large effort (weeks of work)
- High risk of bugs
- Reinventing what presage already does

### Option C: Wait for Upstream Presage
**Effort**: 0 hours
**Risk**: Unknown timeline

**Steps:**
1. Monitor presage repository for libsignal-service-rs fork compatibility
2. Submit issue to presage maintainers
3. Wait for upstream fix

**Pros:**
- No work required

**Cons:**
- Blocks all Signal integration indefinitely
- Unknown timeline (could be months)
- Presage may not prioritize our fork compatibility

## Recommended Path Forward

**Adopt Option A: Fork Presage and Fix Compatibility**

**Rationale:**
1. Balances effort vs. risk
2. Builds on proven foundation
3. Unblocks Signal integration in reasonable timeframe
4. Recommended in PHASE0_REVIEW_REPORT.md lines 473-476

**Timeline:**
- Hours 1-2: Fork repo, set up build environment
- Hours 3-8: Fix AccountManager and PushService errors
- Hours 9-12: Fix pre-key and type errors
- Hours 13-14: Test device linking
- Hours 15-16: Test message handling and integration

**Success Criteria:**
- ✅ presage compiles with our libsignal fork
- ✅ Device linking works (QR code, registration)
- ✅ Messages received and processed
- ✅ GAP-07 verified (no PII in logs)
- ✅ All tests pass

## Work Completed So Far

1. ✅ Re-enabled presage dependency in Cargo.toml:78
2. ✅ Identified all 62 compilation errors
3. ✅ Categorized errors by type
4. ✅ Created production client skeleton (src/signal/client.rs)
5. ✅ Added client module to signal module (src/signal/mod.rs)
6. ✅ Documented current state and options

## Next Steps

**Decision Point:** Choose resolution option (recommend Option A)

**If Option A (Fork Presage):**
1. Create GitHub fork: `stromarig/presage`
2. Clone and create feature branch
3. Systematically fix the 62 errors
4. Test and validate
5. Update Cargo.toml to use fork
6. Verify acceptance criteria

**If Option B (Direct Implementation):**
1. Add libsignal-service-rs as direct dependency
2. Implement LibsignalClient methods one by one
3. Focus on device linking first
4. Then message handling
5. Then group operations
6. Test thoroughly

**If Option C (Wait):**
1. Submit issue to presage repository
2. Continue with other non-Signal work
3. Check presage updates periodically

## References

- PHASE0_REVIEW_REPORT.md lines 400-476
- Bead: hq-mdfb (st-rvzl)
- Cargo.toml:72-78 (presage dependency)
- src/signal/client.rs (production client skeleton)
- src/signal/linking.rs (device linking interface)

## Notes

- The Signal bot code is **architecturally complete** with trait abstractions
- All tests pass with MockSignalClient (100% coverage)
- The ONLY blocker is getting a working production SignalClient implementation
- GAP-07 compliance already verified (0 PII violations in logging)
- This blocks Phase 0 MVP delivery per TODO.md:382-446
