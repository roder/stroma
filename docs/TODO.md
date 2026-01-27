# Stroma Implementation Checklist

## 🚀 Immediate Actions

### Git & Workspace Setup
- [X] Complete git initialization
  - [X Stage all existing files: `git add .`
  - [X] Create initial commit: `git commit -m "Initial commit: Gastown workspace with UX specification"`
  - [X] Optionally set up remote: `git remote add origin <url>`

### Constraint Beads (Immutable)
- [X] Create `.beads/security-constraints.bead`
  - [X] Anonymity-first design rules
  - [X] No cleartext Signal IDs
  - [X] Immediate ejection protocol
  - [X] HMAC hashing requirements
  - [X] Zeroization requirements
  - [X] Vouch permissions (ANY Member can vouch)
  
- [X] Create `.beads/architecture-decisions.bead`
  - [X] freenet-core as state storage
  - [X] STARKs (winterfell) for ZK-proofs
  - [X] Each bot runs own node
  - [X] Performance targets
  - [X] Threat model
  
- [X] Create `.beads/federation-roadmap.bead`
  - [X] MVP = single group (no federation)
  - [X] Federation as north star
  - [X] Phase 4+ federation features
  - [X] Design principles

### Agent Structure
- [ ] Define agent boundaries
  - [ ] Agent-Signal: Signal protocol integration
  - [ ] Agent-Freenet: freenet-core node & state
  - [ ] Agent-Crypto: STARKs + HMAC + zeroization
  - [ ] Witness-Agent: Security audit (continuous)
  
- [ ] Create Mayor briefing document
  - [ ] Technology stack decisions
  - [ ] Security constraints
  - [ ] Implementation roadmap
  - [ ] Agent coordination strategy

## 🔬 Spike Week (Week 0 - Validation Phase)

**Objective**: Validate core technologies before committing to architecture

### Day 1-2: freenet-core Integration & Contract Design
- [ ] Install freenet-core from https://github.com/freenet/freenet-core
  - [ ] `git clone https://github.com/freenet/freenet-core.git`
  - [ ] `cd freenet-core`
  - [ ] `git submodule update --init --recursive`
  - [ ] `cargo install --path crates/core`
  
- [ ] Run freenet-core node locally
  - [ ] Start node: `freenet &`
  - [ ] Verify node is running
  
- [ ] Test ComposableState trait (CRITICAL)
  - [ ] Install freenet-scaffold: Add to test Cargo.toml
  - [ ] Implement simple ComposableState (e.g., counter)
  - [ ] Test merge semantics (create two states, merge them)
  - [ ] Verify merge is commutative (order-independent)
  
- [ ] Test set-based membership (Stroma-specific)
  - [ ] Implement MemberSet with BTreeSet (active + removed tombstones)
  - [ ] Test adding members to set
  - [ ] Test removing members (tombstone pattern)
  - [ ] Test merging two divergent member sets
  - [ ] Verify tombstones prevent re-addition
  
- [ ] Test on-demand Merkle Tree generation
  - [ ] Generate Merkle Tree from BTreeSet<MemberHash>
  - [ ] Benchmark with 10, 100, 500, 1000 members
  - [ ] Measure tree generation time (target: < 100ms)
  - [ ] Test Merkle proof generation for ZK-proof verification
  
- [ ] Test state stream monitoring
  - [ ] Deploy contract to freenet-core
  - [ ] Subscribe to state changes (real-time, not polling)
  - [ ] Submit state update from one node
  - [ ] Verify other node receives update via stream
  
- [ ] **Answer Outstanding Questions** (CRITICAL)
  - [ ] **Q1**: Can we verify STARK proofs in contract verify()? (Wasm performance test)
  - [ ] **Q2**: Should we store proofs or just outcomes? (Storage strategy decision)
  - [ ] **Q3**: How expensive is on-demand Merkle Tree generation? (Performance benchmark)
  - [ ] **Q4**: How does Freenet handle merge conflicts? (Create conflicting updates, observe)
  - [ ] **Q5**: Can we add custom validation beyond ComposableState? (Review contract API)
  
- [ ] Document findings
  - [ ] Merkle Tree approach: Store set, generate tree on-demand
  - [ ] ZK-proof strategy: Client-side vs contract-side verification
  - [ ] Merge semantics: CRDT-like patterns for Stroma
  - [ ] Performance: Benchmarks and bottlenecks
  - [ ] Limitations: What we can't do with Freenet contracts

### Day 3: Signal Bot Registration
- [ ] Register bot account with Signal
  - [ ] Obtain phone number for bot
  - [ ] Complete Signal registration process
  - [ ] Test authentication
  
- [ ] Test group management
  - [ ] Create Signal group via bot
  - [ ] Add test member to group
  - [ ] Remove test member from group
  - [ ] Verify admin capabilities
  
- [ ] Test 1-on-1 PM handling
  - [ ] Send PM to bot
  - [ ] Receive PM from bot
  - [ ] Test command parsing
  
- [ ] Document findings
  - [ ] Can we automate admission/ejection?
  - [ ] What are Signal's rate limits?
  - [ ] Risk of bot bans?

### Day 4-5: STARK Proof Generation
- [ ] Set up winterfell library
  - [ ] Add dependency to test project
  - [ ] Review winterfell documentation
  
- [ ] Create sample STARK circuit
  - [ ] Design circuit: "2 vouches from different Members verified"
  - [ ] Implement proof generation
  - [ ] Implement proof verification
  
- [ ] Measure performance
  - [ ] Proof size (target: < 100KB)
  - [ ] Proof generation time (target: < 10 seconds)
  - [ ] Verification time
  
- [ ] Document findings
  - [ ] Are proofs practical for our use case?
  - [ ] STARKs vs PLONK comparison
  - [ ] Performance bottlenecks

### Spike Week Deliverable
- [ ] Create Go/No-Go decision report
  - [ ] freenet-core validation results
  - [ ] Signal bot validation results
  - [ ] STARK proof validation results
  - [ ] Recommendation: Proceed or adjust architecture
  - [ ] Identified risks and mitigations

## 📦 Phase 0: Foundation (Weeks 1-2)

**Objective**: Core infrastructure with federation-ready design

### Module Structure
- [ ] Create `src/kernel/` directory
  - [ ] `src/kernel/mod.rs`
  - [ ] `src/kernel/hmac.rs` - HMAC-based hashing
  - [ ] `src/kernel/zeroize_helpers.rs` - Immediate buffer purging
  
- [ ] Create `src/freenet/` directory
  - [ ] `src/freenet/mod.rs`
  - [ ] `src/freenet/node.rs` - freenet-core node management
  - [ ] `src/freenet/contract.rs` - Wasm contract deployment
  - [ ] `src/freenet/state_stream.rs` - Real-time state monitoring
  
- [ ] Create `src/signal/` directory
  - [ ] `src/signal/mod.rs`
  - [ ] `src/signal/bot.rs` - Bot authentication & commands
  - [ ] `src/signal/group.rs` - Group management (add/remove)
  - [ ] `src/signal/pm.rs` - 1-on-1 PM handling
  
- [ ] Create `src/crypto/` directory
  - [ ] `src/crypto/mod.rs`
  - [ ] `src/crypto/stark_circuit.rs` - STARK circuit for vouching
  - [ ] `src/crypto/proof_generation.rs` - Generate proofs
  - [ ] `src/crypto/proof_verification.rs` - Verify proofs
  
- [ ] Create `src/gatekeeper/` directory
  - [ ] `src/gatekeeper/mod.rs`
  - [ ] `src/gatekeeper/admission.rs` - Vetting & admission logic
  - [ ] `src/gatekeeper/ejection.rs` - Immediate ejection
  - [ ] `src/gatekeeper/health_monitor.rs` - Continuous standing checks
  
- [ ] Create `src/matchmaker/` directory
  - [ ] `src/matchmaker/mod.rs`
  - [ ] `src/matchmaker/graph_analysis.rs` - Topology analysis
  - [ ] `src/matchmaker/cluster_detection.rs` - Identify internal clusters
  - [ ] `src/matchmaker/strategic_intro.rs` - MST optimization
  
- [ ] Create `src/config/` directory
  - [ ] `src/config/mod.rs`
  - [ ] `src/config/group_config.rs` - GroupConfig struct
  
- [ ] Create `src/federation/` directory (disabled in MVP)
  - [ ] `src/federation/mod.rs`
  - [ ] `src/federation/shadow_beacon.rs` - Social Anchor Hashing (unused)
  - [ ] `src/federation/psi_ca.rs` - PSI-CA (unused)
  - [ ] `src/federation/diplomat.rs` - Federation proposals (unused)

### Cargo Configuration
- [ ] Update `Cargo.toml`
  - [ ] Add ring (HMAC)
  - [ ] Add zeroize (memory hygiene)
  - [ ] Add winterfell (STARKs)
  - [ ] Add freenet-core dependency
  - [ ] Add libsignal-service-rs
  - [ ] Add tokio (async runtime)
  - [ ] Add serde (serialization)
  - [ ] Add tracing (logging)
  
- [ ] Create `.cargo/config.toml`
  - [ ] Configure MUSL target: `x86_64-unknown-linux-musl`
  - [ ] Add linker configuration
  - [ ] Add rustflags for static linking
  
- [ ] Create `cargo-deny.toml`
  - [ ] Configure advisories (deny vulnerabilities)
  - [ ] Configure licenses (allow list)
  - [ ] Configure bans (deny multiple versions)
  - [ ] Configure sources (deny unknown registries)

### Phase 0 Beads Issues
- [ ] Create Bead-01: HMAC identity masking with zeroization
  - [ ] `bd create --title "Implement HMAC identity masking"`
  - [ ] Specify: HMAC-SHA256 with group-secret pepper
  - [ ] Specify: Zeroize buffers immediately
  - [ ] Specify: Unit tests with fixed test pepper
  
- [ ] Create Bead-02: freenet-core node and state monitoring
  - [ ] `bd create --title "Integrate freenet-core node"`
  - [ ] Specify: Node lifecycle management
  - [ ] Specify: Wasm contract deployment (stub)
  - [ ] Specify: State stream monitoring (real-time)
  
- [ ] Create Bead-03: Signal bot authentication and commands
  - [ ] `bd create --title "Implement Signal bot"`
  - [ ] Specify: Bot registration
  - [ ] Specify: Group management
  - [ ] Specify: 1-on-1 PM handling
  - [ ] Specify: Command parsing
  
- [ ] Create Bead-04: STARK circuits for vouch verification
  - [ ] `bd create --title "Implement STARK circuits"`
  - [ ] Specify: Circuit design
  - [ ] Specify: Proof generation
  - [ ] Specify: Proof verification
  
- [ ] Create Bead-05: Contract schema with federation hooks
  - [ ] `bd create --title "Design Freenet contract schema"`
  - [ ] Specify: TrustNetworkState struct
  - [ ] Specify: Federation hooks (unused in MVP)
  - [ ] Specify: GroupConfig struct

### Phase 0 Success Criteria
- [ ] freenet-core node runs successfully
- [ ] STARK proof generated (< 100KB, < 10 seconds)
- [ ] Signal bot can manage group (add/remove members)
- [ ] HMAC masking works with immediate zeroization
- [ ] Contract schema supports federation hooks (present but unused)

## 🌱 Phase 1: Bootstrap & Core Trust (Weeks 3-4)

**Objective**: Seed group, vetting, admission, ejection

### Bootstrap Module
- [ ] Implement seed group bootstrap
  - [ ] Manually add 3 seed members to Signal group
  - [ ] Create initial triangle vouching (all vouch for each other)
  - [ ] Initialize Freenet contract with 3 members
  - [ ] Each seed member has 2 vouches
  
- [ ] Verify bootstrap
  - [ ] Confirm all 3 members in Freenet state
  - [ ] Confirm all 3 members in Signal group
  - [ ] Confirm vouch counts are correct

### Trust Operations
- [ ] Implement invitation flow
  - [ ] Member sends `/invite @username [context]`
  - [ ] Bot records invitation as first vouch
  - [ ] Bot selects second Member via Blind Matchmaker
  - [ ] Bot sends PMs to invitee and selected Member
  
- [ ] Implement vetting interview
  - [ ] Bot creates 3-person chat (invitee, Member, bot)
  - [ ] Bot facilitates introduction
  - [ ] Member vouches via `/vouch @username`
  - [ ] Bot records second vouch in Freenet
  
- [ ] Implement admission
  - [ ] Bot verifies 2 vouches from different Members
  - [ ] Bot generates ZK-proof
  - [ ] Bot stores proof in Freenet contract
  - [ ] Bot adds invitee to Signal group (now a Bridge)
  - [ ] Bot announces admission
  - [ ] Bot deletes vetting session data
  
- [ ] Implement flagging
  - [ ] Member sends `/flag @username [reason]`
  - [ ] Bot records flag in Freenet
  - [ ] Bot recalculates: `Standing = Vouches - Flags`
  - [ ] Bot checks ejection triggers

### Ejection Protocol
- [ ] Implement ejection triggers
  - [ ] Trigger 1: `Standing < 0` (too many flags)
  - [ ] Trigger 2: `Vouches < min_vouch_threshold` (voucher left)
  
- [ ] Implement immediate ejection
  - [ ] Bot removes member from Signal group
  - [ ] Bot sends PM to ejected member
  - [ ] Bot announces to group (uses hash, not name)
  - [ ] No grace period
  
- [ ] Implement health monitoring
  - [ ] Run heartbeat every 60 minutes
  - [ ] Check all members' standing
  - [ ] Trigger ejection if thresholds violated

### Basic Commands
- [ ] Implement `/invite @username [context]`
  - [ ] Parse command
  - [ ] Validate inviter is Member
  - [ ] Record as first vouch
  - [ ] Start vetting process
  
- [ ] Implement `/vouch @username`
  - [ ] Parse command
  - [ ] Validate voucher is Member
  - [ ] Record vouch in Freenet
  - [ ] Check if admission threshold met
  
- [ ] Implement `/flag @username [reason]`
  - [ ] Parse command
  - [ ] Validate flagger is Member
  - [ ] Validate reason is provided
  - [ ] Record flag in Freenet
  - [ ] Check ejection triggers
  
- [ ] Implement `/status`
  - [ ] Show user's own trust standing
  - [ ] Show vouch count
  - [ ] Show flag count
  - [ ] Show role (Bridge/Validator)

### Phase 1 Success Criteria
- [ ] 3-member seed group bootstrapped successfully
- [ ] New member admitted after 2 vouches (ZK-proof verified)
- [ ] Member ejected when `Standing < 0`
- [ ] Member ejected when `Effective_Vouches < 2` (includes voucher-flagger invalidation)
- [ ] Vouch invalidation works correctly (voucher who flags = vouch invalidated)
- [ ] All vetting in 1-on-1 PMs (no group chat exposure)
- [ ] No cleartext Signal IDs stored anywhere

## 🎯 Phase 2: Internal Mesh Optimization (Weeks 5-6)

**Objective**: Graph analysis, strategic introductions, MST

### Blind Matchmaker
- [ ] Implement graph topology analysis
  - [ ] Build trust graph from Freenet state
  - [ ] Identify Bridges (2 vouches)
  - [ ] Identify Validators (3+ vouches)
  - [ ] Calculate vouch distribution
  
- [ ] Implement cluster identification
  - [ ] Detect internal clusters (sub-communities)
  - [ ] Find disconnected islands
  - [ ] Calculate cluster sizes
  
- [ ] Implement strategic introduction suggestions
  - [ ] Priority 1: Connect Bridges to Validators (different clusters)
  - [ ] Priority 2: Connect Validators across islands
  - [ ] Generate introduction recommendations
  
- [ ] Implement MST optimization
  - [ ] Calculate minimum new interactions needed
  - [ ] Suggest strategic introductions to Members
  - [ ] Track introduction acceptance rate

### Advanced Commands
- [ ] Implement `/mesh` (network overview)
  - [ ] Show total member count
  - [ ] Show mesh density percentage
  - [ ] Show federation status (if any)
  - [ ] Show user's position in network
  
- [ ] Implement `/mesh strength` (histogram)
  - [ ] Calculate mesh density: `(Actual Vouches / Max Possible) × 100`
  - [ ] Generate histogram of vouch distribution
  - [ ] Show Bridges count (2 vouches)
  - [ ] Show Validators count (3+ vouches)
  - [ ] Show ASCII visualization
  
- [ ] Implement `/mesh config` (configuration view)
  - [ ] Show `config_change_threshold`
  - [ ] Show `ejection_appeal_threshold`
  - [ ] Show `min_intersection_density`
  - [ ] Show `validator_percentile`
  - [ ] Show `min_vouch_threshold`
  
- [ ] Implement `/propose-config key=value [reason]`
  - [ ] Parse command
  - [ ] Validate key is valid config parameter
  - [ ] Validate value is valid for parameter type
  - [ ] Create Signal Poll for voting
  - [ ] Track votes (✅ Approve / ❌ Reject / ⏸️ Abstain)

### Configuration Management
- [ ] Implement Signal Poll voting
  - [ ] Create poll with 3 options
  - [ ] Monitor poll responses
  - [ ] Calculate approval percentage
  - [ ] Auto-close after voting period (e.g., 48 hours)
  
- [ ] Implement automatic config updates
  - [ ] Check if approval > `config_change_threshold`
  - [ ] Update Freenet contract if approved
  - [ ] Announce result to group
  - [ ] Log change in audit trail

### Operator Audit
- [ ] Implement `/audit operator` command
  - [ ] Show operator action history (last 30 days)
  - [ ] Show action types (ServiceStart, ServiceRestart)
  - [ ] Show timestamps
  - [ ] Note: No manual operations logged (bot is automatic)

### Phase 2 Success Criteria
- [ ] Graph topology correctly identifies Bridges and Validators
- [ ] Strategic introductions suggested for MST
- [ ] Mesh density histogram displayed correctly
- [ ] Configuration changes via Signal Poll (70% threshold)
- [ ] Operator audit trail queryable
- [ ] Bot proactively suggests mesh optimization

## 🔧 Phase 3: Federation Preparation (Week 7)

**Objective**: Validate federation infrastructure (locally, no broadcast)

### Shadow Beacon (Compute Locally)
- [ ] Implement Social Anchor hashing
  - [ ] Calculate top-N validators (percentile-based)
  - [ ] Generate discovery URI from validator hashes
  - [ ] Store locally (DO NOT broadcast in MVP)
  
- [ ] Implement validator percentile calculation
  - [ ] Sort members by vouch count
  - [ ] Calculate percentile threshold
  - [ ] Identify top validators
  
- [ ] Implement discovery URI generation
  - [ ] Hash social anchor
  - [ ] Generate multiple URIs (10%, 20%, 30%, 50%)
  - [ ] Store for future Phase 4 use

### PSI-CA (Test Locally)
- [ ] Implement Bloom filter generation
  - [ ] Create Bloom filter from member hashes
  - [ ] Optimize filter size/false positive rate
  - [ ] Serialize filter
  
- [ ] Implement commutative encryption
  - [ ] Encrypt Bloom filter (double-blinding)
  - [ ] Test encryption is commutative
  - [ ] Prepare for anonymous handshake
  
- [ ] Implement intersection density calculation
  - [ ] Calculate overlap: `|A ∩ B|`
  - [ ] Calculate union: `|A ∪ B|`
  - [ ] Calculate density: `|A ∩ B| / |A ∪ B|`
  - [ ] Test with mock data (simulate two groups)

### Contract Schema Validation
- [ ] Test federation hooks (present but unused)
  - [ ] Verify `federation_contracts` field exists
  - [ ] Verify `validator_anchors` field exists
  - [ ] Confirm they're empty in MVP
  
- [ ] Verify identity hashes are re-computable
  - [ ] Test HMAC hashing with different peppers
  - [ ] Confirm PSI-CA can work with hashes
  - [ ] Validate privacy preservation

### Documentation
- [ ] Create federation design document
  - [ ] Emergent discovery protocol
  - [ ] PSI-CA handshake protocol
  - [ ] BidirectionalMin threshold evaluation
  - [ ] Cross-mesh vouching protocol
  
- [ ] Create Phase 4+ roadmap
  - [ ] Shadow Beacon broadcast
  - [ ] Bot-to-bot discovery
  - [ ] Federation voting
  - [ ] Cross-mesh vouching implementation

### Phase 3 Success Criteria
- [ ] Social Anchor hash computed correctly
- [ ] PSI-CA overlap calculated locally (test with mock data)
- [ ] Federation hooks in contract validated
- [ ] Documentation complete for Phase 4
- [ ] MVP ready for production deployment

## 🚢 Launch Phase 0 Convoy

- [ ] Brief Mayor agent
  - [ ] Provide technology stack decisions
  - [ ] Provide security constraints (read Beads)
  - [ ] Provide implementation roadmap
  - [ ] Provide agent coordination strategy
  
- [ ] Launch convoy with parallel agents
  ```bash
  gt convoy start \
    --phase "Phase 0: Foundation" \
    --beads "Bead-01,Bead-02,Bead-03,Bead-04,Bead-05" \
    --agents "Agent-Signal,Agent-Freenet,Agent-Crypto" \
    --witness "Witness-Agent"
  ```
  
- [ ] Monitor convoy progress
  - [ ] Check agent status
  - [ ] Review Witness Agent security audits
  - [ ] Verify no cleartext IDs in code
  - [ ] Verify zeroization implemented correctly

## 📊 Overall Success Metrics

### Security
- [ ] No cleartext Signal IDs stored anywhere
- [ ] All sensitive buffers zeroized immediately
- [ ] ZK-proofs used for all trust operations
- [ ] Memory dump contains only hashed identifiers
- [ ] cargo-deny and cargo-crev checks pass

### Functionality
- [ ] Seed group bootstrapped (3 members)
- [ ] Invitation & vetting flow working
- [ ] Admission requires 2 vouches from different Members
- [ ] Ejection immediate (both triggers working)
- [ ] All bot commands functional
- [ ] Mesh density calculated correctly

### Architecture
- [ ] Static MUSL binary produced
- [ ] freenet-core node runs successfully
- [ ] Signal bot authenticates and manages group
- [ ] STARK proofs < 100KB, generation < 10 seconds
- [ ] Federation infrastructure present (disabled in MVP)
- [ ] Freenet contract uses ComposableState (mergeable state)
- [ ] Set-based membership with on-demand Merkle Tree generation

### Documentation
- [ ] README.md accurate and complete
- [ ] All .cursor/rules/ updated
- [ ] Beads created for all phases
- [ ] API documentation complete
- [ ] Federation roadmap documented

## 🚨 Outstanding Questions (MUST RESOLVE)

**Critical**: These questions MUST be answered during Spike Week (Day 1-2). They fundamentally affect contract architecture.

### Q1: STARK Proof Verification Performance in Wasm
**Question**: Can we verify STARK proofs in contract `verify()` method without performance issues?

**Why This Matters**:
- Determines client-side vs contract-side verification strategy
- Affects trustlessness (contract verification is more trustless)
- Impacts Wasm bundle size and execution time

**Test Plan**:
- [ ] Attempt to compile winterfell to Wasm (may not be possible/practical)
- [ ] If possible, measure verification time in Wasm context
- [ ] Target: < 100ms per proof verification
- [ ] If too slow or not possible, use client-side verification (Approach 1)

**Decision Criteria**:
- ✅ If verification < 100ms: Use Approach 2 (contract-side verification)
- ❌ If verification > 100ms OR can't compile to Wasm: Use Approach 1 (client-side)

### Q2: Proof Storage Strategy
**Question**: Should we store STARK proofs in contract state, or just store outcomes?

**Options**:
- **A**: Store proofs temporarily (verified once in verify(), then removed in apply_delta)
- **B**: Store proofs permanently (complete audit trail)
- **C**: Don't store proofs at all (bot verifies client-side, contract trusts outcome)

**Why This Matters**:
- Storage costs (STARKs can be large)
- Audit trail (can we verify historical vouches?)
- Trustlessness (contract verification vs bot verification)

**Recommendation**:
- MVP: Use Option C (simplest, smallest contract state)
- Phase 4: Evaluate Options A/B for federated trust verification

**Decision**:
- [ ] Decide in Spike Week based on Q1 answer
- [ ] If Q1 = contract-side verification, consider Option A
- [ ] If Q1 = client-side verification, use Option C

### Q3: On-Demand Merkle Tree Performance
**Question**: How expensive is generating Merkle Tree from BTreeSet on every ZK-proof verification?

**Why This Matters**:
- Determines if we cache Merkle root or regenerate on demand
- Affects bot performance (proof generation speed)
- May require contract state changes if caching needed

**Test Plan**:
- [ ] Benchmark Merkle Tree generation from BTreeSet
- [ ] Test with 10, 100, 500, 1000 members
- [ ] Measure generation time on modern CPU
- [ ] Target: < 100ms for 1000 members

**Decision Criteria**:
- ✅ If generation < 100ms for 1000 members: Generate on demand (no caching)
- ⚠️ If generation 100-500ms: Cache Merkle root, invalidate on member changes
- ❌ If generation > 500ms: Need optimized Merkle Tree implementation

### Q4: Freenet Conflict Resolution Semantics
**Question**: How does Freenet handle conflicts when two nodes submit incompatible updates?

**Example Conflict**:
```
Node A submits: Add member X with vouches (A, B)
Node B submits: Remove member A (X's voucher is being removed)

These updates conflict - which wins?
```

**Why This Matters**:
- Determines if we need causal ordering or vector clocks
- Affects ejection timing (can ejection and admission race?)
- May require additional conflict resolution logic

**Test Plan**:
- [ ] Create two separate freenet-core nodes
- [ ] Submit conflicting state updates from each node
- [ ] Observe which update wins (last-write? first-write? merge both?)
- [ ] Document Freenet's conflict resolution behavior

**Decision**:
- [ ] If Freenet handles conflicts well: Use default behavior
- [ ] If conflicts cause issues: Add vector clocks or causal ordering

### Q5: Custom Validation Beyond ComposableState
**Question**: Does freenet-core support custom validation logic beyond the `verify()` method?

**Use Case**: Complex invariants like:
- "Every member must have ≥2 vouches from different Members"
- "Standing = Vouches - Flags must be ≥ 0"
- "Config changes require version increment"

**Why This Matters**:
- Determines if contract can enforce all trust invariants
- May need bot-side validation if contract can't express complex logic
- Affects trustlessness (contract enforcement is more trustless)

**Test Plan**:
- [ ] Review freenet-core contract API documentation
- [ ] Check if there's a separate validation hook beyond verify()
- [ ] Test complex validation logic in verify() method
- [ ] Determine if verify() is sufficient for our invariants

**Decision**:
- [ ] If verify() is sufficient: Enforce all invariants in contract
- [ ] If verify() is limited: Use hybrid (basic invariants in contract, complex in bot)

---

## 📋 Outstanding Questions Status Tracking

| Question | Status | Decision | Date Resolved |
|----------|--------|----------|---------------|
| Q1: STARK verification in Wasm | ⏳ Pending | TBD | - |
| Q2: Proof storage strategy | ⏳ Pending | TBD | - |
| Q3: Merkle Tree performance | ⏳ Pending | TBD | - |
| Q4: Conflict resolution | ⏳ Pending | TBD | - |
| Q5: Custom validation | ⏳ Pending | TBD | - |

**Update this table as questions are resolved during Spike Week!**