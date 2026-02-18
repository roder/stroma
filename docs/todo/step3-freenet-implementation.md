# Real Freenet Integration Implementation Plan

## Problem Analysis

The current `EmbeddedKernel` implementation has three critical gaps:

1. **Mock-only execution**: Uses `HashMap` mocks instead of actual Freenet `Executor` APIs
2. **Disconnected state stream**: `subscribe()` returns `stream::empty()` instead of real events
3. **No production path**: `NodeConfig::build()` integration not wired

## Implementation Strategy

### Phase 1: Wire Production APIs (Priority: HIGH)

**Target File**: `src/freenet/embedded_kernel.rs`

**Changes Required**:

1. **Replace mock executor with real Executor**:
   - Update `EmbeddedKernel::new()` to use `Executor::new()`
   - Keep `Executor::new_mock_in_memory()` for tests
   - Add feature flag for test vs production mode

2. **Wire state stream to real events**:
   - Replace `stream::empty()` with actual `executor.events()` subscription
   - Implement proper backpressure handling for state updates

3. **Implement NodeConfig integration**:
   - Add `NodeConfig` dependency
   - Create `EmbeddedKernel::from_config()` constructor
   - Wire `build()` method for production initialization

### Phase 2: Integration Tests (Priority: HIGH)

**Target**: `src/freenet/embedded_kernel.rs`

**Tests Required**:

| Test | Purpose |
|------|---------|
| `test_production_executor` | Verify `Executor::new()` works |
| `test_state_stream_integration` | Verify real event subscription |
| `test_node_config_build` | Verify `NodeConfig::build()` path |
| `test_concurrent_operations` | Async safety with real executor |
| `test_error_handling` | Executor error propagation |

### Phase 3: Bot Integration (Priority: MEDIUM)

**Target File**: `src/signal/bot.rs`

**Changes Required**:

1. **Add state stream subscription**:
   ```rust
   // Replace TODO comment with:
   let mut state_stream = kernel.subscribe(&contract_hash).await?;
   while let Some(state_change) = state_stream.next().await {
       match state_change {
           StateChange::ContractDeployed(_) => {
               // Handle deployment
           }
           StateChange::MemberAdded(hash) => {
               // Handle new member
           }
           // ... other variants
       }
   }
   ```

2. **Add error recovery**:
   - Retry on transient failures
   - Graceful degradation for offline state

### Phase 4: UAT (Priority: MEDIUM)

**Testing Requirements**:

| Test | Purpose |
|------|---------|
| Real node connection | Verify network connectivity |
| Dark mode contract | Verify persistent storage |
| Concurrent deltas | Verify eventual consistency |
| Error recovery | Verify fault tolerance |

## Technical Details

### Executor Pattern (per freenet-integration.bead)

```rust
// Production path
use freenet::local_node::{NodeConfig, Executor};

let config = NodeConfig {
    should_connect: true,
    is_gateway: false,
    // ... other config
};
let node = config.build([client_proxy]).await?;

// Test path
let executor = Executor::new_mock_in_memory();
let kernel = EmbeddedKernel::with_executor(executor).await?;
```

### State Stream Subscription

```rust
async fn subscribe(&self, contract_hash: &ContractHash) -> Result<StateStream> {
    let mut events = self.executor.events(contract_hash)?;
    Ok(StateStream::new(events))
}
```

### Feature Flags

```toml
[features]
default = []
test-freenet = []  # Enable mock executor for tests
```

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Freenet API changes | High | Pin versions, add API change tests |
| NodeConfig breaking changes | High | Add integration tests for config path |
| Real node UAT failures | Medium | Fallback to mock for unstable nodes |
| State stream backpressure | Medium | Implement bounded channel |

## Success Criteria

1. ✅ `EmbeddedKernel` uses real `Executor` APIs (not mocks)
2. ✅ State stream delivers real Freenet events
3. ✅ `NodeConfig::build()` path tested and working
4. ✅ Bot integration with state stream
5. ✅ UAT passes with real Freenet node

## Dependencies

**Required Dependencies**:
- `freenet` = { path = "../freenet", features = ["embedded"] }
- `freenet-stdlib` = { path = "../freenet-stdlib" }

**New Dependencies**:
- `tracing` = "0.1" (for production logging)
- `anyhow` = "1.0" (for error propagation)

## Timeline Estimate

| Phase | Duration | Owner |
|-------|----------|-------|
| Phase 1: Production wiring | 4 hours | Agent |
| Phase 2: Integration tests | 3 hours | Agent |
| Phase 3: Bot integration | 2 hours | Agent |
| Phase 4: UAT | 4 hours | Manual |
| **Total** | **13 hours** | |

## Implementation Checklist

- [ ] Read current `src/freenet/embedded_kernel.rs` implementation
- [ ] Read `src/freenet/traits.rs` for FreenetClient trait
- [ ] Read `src/freenet/state_stream.rs` for StateStream implementation
- [ ] Read `src/signal/bot.rs` for current integration points
- [ ] Review `freenet` crate API documentation
- [ ] Create feature flag in `Cargo.toml`
- [ ] Implement production executor wiring
- [ ] Implement state stream integration
- [ ] Add NodeConfig integration
- [ ] Write integration tests
- [ ] Update bot.rs with state stream subscription
- [ ] Run full test suite
- [ ] Execute UAT with real Freenet node