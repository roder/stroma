//! Trust network state for Freenet contract.
//!
//! Per freenet-contract-design.bead:
//! - Set-based state with commutative merges
//! - StateDelta for eventual consistency
//! - CBOR serialization (NOT JSON)
//!
//! Per serialization-format.bead:
//! - Federation hooks with #[serde(default)]
//! - Backward compatible schema evolution

use crate::freenet::contract::MemberHash;
use crate::serialization::{from_cbor, to_cbor, SerializationError};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::time::Duration;

/// Trust network state (Freenet contract state).
///
/// Per freenet-contract-design.bead:
/// - Membership: set-based, commutative
/// - Trust graph: set-based, commutative
/// - Config: last-write-wins with timestamp
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrustNetworkState {
    /// Active members (set-based, commutative via union).
    pub members: BTreeSet<MemberHash>,

    /// Ejected members (can return, not permanent tombstone).
    pub ejected: BTreeSet<MemberHash>,

    /// Vouches: vouchee -> set of vouchers (set-based, commutative).
    pub vouches: HashMap<MemberHash, HashSet<MemberHash>>,

    /// Flags: flagged -> set of flaggers (set-based, commutative).
    pub flags: HashMap<MemberHash, HashSet<MemberHash>>,

    /// Group configuration.
    pub config: GroupConfig,

    /// Config timestamp for last-write-wins resolution.
    pub config_timestamp: u64,

    /// Schema version for evolution.
    pub schema_version: u64,

    /// Federation contract addresses (added in v2).
    #[serde(default)]
    pub federation_contracts: Vec<ContractHash>,

    /// GAP-11: Track if cluster formation announcement has been sent.
    /// Once ≥2 clusters detected, announcement sent once and this is set to true.
    #[serde(default)]
    pub gap11_announcement_sent: bool,

    /// Active proposals awaiting timeout expiration.
    /// Key: poll_id, Value: ActiveProposal
    #[serde(default)]
    pub active_proposals: HashMap<u64, ActiveProposal>,
}

/// Group configuration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroupConfig {
    /// Minimum vouches required for full standing.
    pub min_vouches: u32,

    /// Maximum flags before ejection.
    pub max_flags: u32,

    /// Whether new members can join.
    pub open_membership: bool,

    /// Operator member hashes (can modify config).
    #[serde(default)]
    pub operators: BTreeSet<MemberHash>,

    /// Default poll timeout in seconds (used when --timeout not specified).
    /// Must be between 3600 (1h) and 604800 (168h).
    #[serde(default = "default_poll_timeout_secs")]
    pub default_poll_timeout_secs: u64,

    /// Threshold for config change proposals (fraction 0.0-1.0).
    /// e.g., 0.70 = 70% of votes must be "approve" to pass.
    #[serde(default = "default_config_change_threshold")]
    pub config_change_threshold: f32,

    /// Minimum quorum (fraction 0.0-1.0).
    /// e.g., 0.50 = at least 50% of members must vote.
    #[serde(default = "default_min_quorum")]
    pub min_quorum: f32,
}

fn default_poll_timeout_secs() -> u64 {
    172800 // 48 hours
}

fn default_config_change_threshold() -> f32 {
    0.70 // 70%
}

fn default_min_quorum() -> f32 {
    0.50 // 50%
}

impl Default for GroupConfig {
    fn default() -> Self {
        Self {
            min_vouches: 2,
            max_flags: 3,
            open_membership: false,
            operators: BTreeSet::new(),
            default_poll_timeout_secs: default_poll_timeout_secs(),
            config_change_threshold: default_config_change_threshold(),
            min_quorum: default_min_quorum(),
        }
    }
}

impl GroupConfig {
    /// Get default poll timeout as Duration.
    pub fn default_poll_timeout(&self) -> Duration {
        Duration::from_secs(self.default_poll_timeout_secs)
    }
}

/// Contract address hash.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContractHash(pub [u8; 32]);

/// Active proposal awaiting expiration.
///
/// Stored in Freenet to persist proposal state across restarts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiveProposal {
    /// Poll ID from Signal.
    pub poll_id: u64,

    /// Proposal type (ConfigChange, Federation, Other).
    pub proposal_type: String,

    /// Proposal details (key=value for config, description for others).
    pub proposal_details: String,

    /// Poll creation timestamp.
    pub poll_timestamp: u64,

    /// Expiration timestamp (poll_timestamp + timeout).
    pub expires_at: u64,

    /// Timeout duration in seconds.
    pub timeout_secs: u64,

    /// Threshold for approval (e.g., 0.70 = 70%).
    pub threshold: f32,

    /// Minimum quorum (e.g., 0.50 = 50%).
    pub quorum: f32,

    /// Whether this proposal has been checked for expiration.
    pub checked: bool,

    /// Result after checking (if checked).
    pub result: Option<ProposalResult>,
}

/// Result of a proposal after timeout.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ProposalResult {
    Passed { approve_count: u32, reject_count: u32 },
    Failed { approve_count: u32, reject_count: u32 },
    QuorumNotMet { participation_rate: f32 },
}

/// State delta for commutative merge.
///
/// Per freenet-contract-design.bead:
/// "Delta updates MUST be commutative"
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StateDelta {
    /// Members added.
    pub members_added: Vec<MemberHash>,

    /// Members removed (moved to ejected).
    pub members_removed: Vec<MemberHash>,

    /// Vouches added: (voucher, vouchee).
    pub vouches_added: Vec<(MemberHash, MemberHash)>,

    /// Vouches removed: (voucher, vouchee).
    pub vouches_removed: Vec<(MemberHash, MemberHash)>,

    /// Flags added: (flagger, flagged).
    pub flags_added: Vec<(MemberHash, MemberHash)>,

    /// Flags removed: (flagger, flagged).
    pub flags_removed: Vec<(MemberHash, MemberHash)>,

    /// New config (if changed).
    #[serde(default)]
    pub config_update: Option<ConfigUpdate>,

    /// Proposals created: (poll_id, ActiveProposal).
    #[serde(default)]
    pub proposals_created: Vec<(u64, ActiveProposal)>,

    /// Proposals marked as checked: poll_id.
    #[serde(default)]
    pub proposals_checked: Vec<u64>,

    /// Proposals with results: (poll_id, ProposalResult).
    #[serde(default)]
    pub proposals_with_results: Vec<(u64, ProposalResult)>,
}

/// Configuration update with timestamp.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConfigUpdate {
    pub config: GroupConfig,
    pub timestamp: u64,
}

impl TrustNetworkState {
    /// Create new empty state.
    pub fn new() -> Self {
        Self {
            members: BTreeSet::new(),
            ejected: BTreeSet::new(),
            vouches: HashMap::new(),
            flags: HashMap::new(),
            config: GroupConfig::default(),
            config_timestamp: 0,
            schema_version: 1,
            federation_contracts: Vec::new(),
            gap11_announcement_sent: false,
            active_proposals: HashMap::new(),
        }
    }

    /// Serialize to CBOR bytes for Freenet storage.
    pub fn to_bytes(&self) -> Result<Vec<u8>, SerializationError> {
        to_cbor(self)
    }

    /// Deserialize from CBOR bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SerializationError> {
        from_cbor(bytes)
    }

    /// Apply a delta to the state.
    ///
    /// Per freenet-contract-design.bead:
    /// "Deltas must be commutative" - same result regardless of order.
    pub fn apply_delta(&mut self, delta: &StateDelta) {
        // Add members (set union)
        for member in &delta.members_added {
            self.members.insert(*member);
            self.ejected.remove(member);
        }

        // Remove members (move to ejected)
        for member in &delta.members_removed {
            self.members.remove(member);
            self.ejected.insert(*member);
        }

        // Add vouches (set union)
        for (voucher, vouchee) in &delta.vouches_added {
            self.vouches.entry(*vouchee).or_default().insert(*voucher);
        }

        // Remove vouches
        for (voucher, vouchee) in &delta.vouches_removed {
            if let Some(vouchers) = self.vouches.get_mut(vouchee) {
                vouchers.remove(voucher);
                if vouchers.is_empty() {
                    self.vouches.remove(vouchee);
                }
            }
        }

        // Add flags (set union)
        for (flagger, flagged) in &delta.flags_added {
            self.flags.entry(*flagged).or_default().insert(*flagger);
        }

        // Remove flags
        for (flagger, flagged) in &delta.flags_removed {
            if let Some(flaggers) = self.flags.get_mut(flagged) {
                flaggers.remove(flagger);
                if flaggers.is_empty() {
                    self.flags.remove(flagged);
                }
            }
        }

        // Config update (last-write-wins)
        if let Some(config_update) = &delta.config_update {
            if config_update.timestamp > self.config_timestamp {
                self.config = config_update.config.clone();
                self.config_timestamp = config_update.timestamp;
            }
        }

        // Add proposals
        for (poll_id, proposal) in &delta.proposals_created {
            self.active_proposals.insert(*poll_id, proposal.clone());
        }

        // Mark proposals as checked
        for poll_id in &delta.proposals_checked {
            if let Some(proposal) = self.active_proposals.get_mut(poll_id) {
                proposal.checked = true;
            }
        }

        // Update proposal results
        for (poll_id, result) in &delta.proposals_with_results {
            if let Some(proposal) = self.active_proposals.get_mut(poll_id) {
                proposal.result = Some(result.clone());
            }
        }
    }

    /// Merge two states (commutative).
    ///
    /// Per freenet-contract-design.bead:
    /// "Merging must be a commutative monoid"
    pub fn merge(&mut self, other: &TrustNetworkState) {
        // Merge members (set union)
        self.members.extend(other.members.iter().copied());

        // Merge ejected (set union)
        self.ejected.extend(other.ejected.iter().copied());

        // Merge vouches (set union per vouchee)
        for (vouchee, vouchers) in &other.vouches {
            self.vouches
                .entry(*vouchee)
                .or_default()
                .extend(vouchers.iter().copied());
        }

        // Merge flags (set union per flagged)
        for (flagged, flaggers) in &other.flags {
            self.flags
                .entry(*flagged)
                .or_default()
                .extend(flaggers.iter().copied());
        }

        // Config: last-write-wins
        if other.config_timestamp > self.config_timestamp {
            self.config = other.config.clone();
            self.config_timestamp = other.config_timestamp;
        }

        // Schema version: take max
        self.schema_version = self.schema_version.max(other.schema_version);

        // Federation contracts: merge as set
        let combined: HashSet<_> = self
            .federation_contracts
            .iter()
            .chain(other.federation_contracts.iter())
            .cloned()
            .collect();
        self.federation_contracts = combined.into_iter().collect();

        // GAP-11 announcement: logical OR (once sent in any replica, it's sent)
        self.gap11_announcement_sent =
            self.gap11_announcement_sent || other.gap11_announcement_sent;

        // Active proposals: merge by poll_id, prefer one with result if available
        for (poll_id, proposal) in &other.active_proposals {
            self.active_proposals
                .entry(*poll_id)
                .and_modify(|existing| {
                    // If other has a result and existing doesn't, use other's
                    if proposal.result.is_some() && existing.result.is_none() {
                        *existing = proposal.clone();
                    }
                    // If other is checked and existing isn't, use other's
                    else if proposal.checked && !existing.checked {
                        *existing = proposal.clone();
                    }
                })
                .or_insert_with(|| proposal.clone());
        }
    }

    /// Calculate standing for a member.
    ///
    /// Standing = Effective_Vouches - Regular_Flags
    /// Where:
    ///   Effective_Vouches = All_Vouchers - Voucher_Flaggers
    ///   Regular_Flags = All_Flaggers - Voucher_Flaggers
    ///
    /// This ensures voucher-flaggers are excluded from BOTH counts,
    /// preventing the 2-point swing when a voucher flags a member.
    ///
    /// Returns None if member is not active.
    pub fn calculate_standing(&self, member: &MemberHash) -> Option<i32> {
        if !self.members.contains(member) {
            return None;
        }

        let vouchers = self.vouches.get(member).cloned().unwrap_or_default();
        let flaggers = self.flags.get(member).cloned().unwrap_or_default();

        // Find voucher-flaggers (intersection of vouchers and flaggers)
        let voucher_flaggers: std::collections::HashSet<_> =
            vouchers.intersection(&flaggers).cloned().collect();

        // Calculate effective vouches and regular flags
        let all_vouchers = vouchers.len() as i32;
        let all_flaggers = flaggers.len() as i32;
        let voucher_flagger_count = voucher_flaggers.len() as i32;

        let effective_vouches = all_vouchers - voucher_flagger_count;
        let regular_flags = all_flaggers - voucher_flagger_count;

        Some(effective_vouches - regular_flags)
    }

    /// Check if member has good standing.
    pub fn has_good_standing(&self, member: &MemberHash) -> bool {
        self.calculate_standing(member)
            .map(|s| s >= self.config.min_vouches as i32)
            .unwrap_or(false)
    }

    /// Get vouchers for a member.
    pub fn vouchers_for(&self, member: &MemberHash) -> Vec<MemberHash> {
        self.vouches
            .get(member)
            .map(|v| v.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Get flaggers for a member.
    pub fn flaggers_for(&self, member: &MemberHash) -> Vec<MemberHash> {
        self.flags
            .get(member)
            .map(|f| f.iter().copied().collect())
            .unwrap_or_default()
    }
}

impl Default for TrustNetworkState {
    fn default() -> Self {
        Self::new()
    }
}

impl StateDelta {
    /// Create empty delta.
    pub fn new() -> Self {
        Self {
            members_added: Vec::new(),
            members_removed: Vec::new(),
            vouches_added: Vec::new(),
            vouches_removed: Vec::new(),
            flags_added: Vec::new(),
            flags_removed: Vec::new(),
            config_update: None,
            proposals_created: Vec::new(),
            proposals_checked: Vec::new(),
            proposals_with_results: Vec::new(),
        }
    }

    /// Serialize to CBOR bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, SerializationError> {
        to_cbor(self)
    }

    /// Deserialize from CBOR bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SerializationError> {
        from_cbor(bytes)
    }

    /// Add a member.
    pub fn add_member(mut self, member: MemberHash) -> Self {
        self.members_added.push(member);
        self
    }

    /// Remove a member.
    pub fn remove_member(mut self, member: MemberHash) -> Self {
        self.members_removed.push(member);
        self
    }

    /// Add a vouch.
    pub fn add_vouch(mut self, voucher: MemberHash, vouchee: MemberHash) -> Self {
        self.vouches_added.push((voucher, vouchee));
        self
    }

    /// Remove a vouch.
    pub fn remove_vouch(mut self, voucher: MemberHash, vouchee: MemberHash) -> Self {
        self.vouches_removed.push((voucher, vouchee));
        self
    }

    /// Add a flag.
    pub fn add_flag(mut self, flagger: MemberHash, flagged: MemberHash) -> Self {
        self.flags_added.push((flagger, flagged));
        self
    }

    /// Remove a flag.
    pub fn remove_flag(mut self, flagger: MemberHash, flagged: MemberHash) -> Self {
        self.flags_removed.push((flagger, flagged));
        self
    }
}

impl Default for StateDelta {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_member(id: u8) -> MemberHash {
        MemberHash::from_bytes(&[id; 32])
    }

    #[test]
    fn test_new_state() {
        let state = TrustNetworkState::new();
        assert!(state.members.is_empty());
        assert!(state.ejected.is_empty());
        assert!(state.vouches.is_empty());
        assert!(state.flags.is_empty());
        assert_eq!(state.schema_version, 1);
    }

    #[test]
    fn test_cbor_roundtrip() {
        let mut state = TrustNetworkState::new();
        let member = test_member(1);
        state.members.insert(member);

        let bytes = state.to_bytes().unwrap();
        let recovered = TrustNetworkState::from_bytes(&bytes).unwrap();

        assert_eq!(state, recovered);
    }

    #[test]
    fn test_delta_add_member() {
        let mut state = TrustNetworkState::new();
        let member = test_member(1);

        let delta = StateDelta::new().add_member(member);
        state.apply_delta(&delta);

        assert!(state.members.contains(&member));
    }

    #[test]
    fn test_delta_remove_member() {
        let mut state = TrustNetworkState::new();
        let member = test_member(1);

        state.members.insert(member);
        let delta = StateDelta::new().remove_member(member);
        state.apply_delta(&delta);

        assert!(!state.members.contains(&member));
        assert!(state.ejected.contains(&member));
    }

    #[test]
    fn test_delta_add_vouch() {
        let mut state = TrustNetworkState::new();
        let voucher = test_member(1);
        let vouchee = test_member(2);

        let delta = StateDelta::new().add_vouch(voucher, vouchee);
        state.apply_delta(&delta);

        assert!(state.vouches.get(&vouchee).unwrap().contains(&voucher));
    }

    #[test]
    fn test_delta_add_flag() {
        let mut state = TrustNetworkState::new();
        let flagger = test_member(1);
        let flagged = test_member(2);

        let delta = StateDelta::new().add_flag(flagger, flagged);
        state.apply_delta(&delta);

        assert!(state.flags.get(&flagged).unwrap().contains(&flagger));
    }

    #[test]
    fn test_standing_calculation() {
        let mut state = TrustNetworkState::new();
        let member = test_member(1);
        let voucher1 = test_member(2);
        let voucher2 = test_member(3);
        let flagger = test_member(4);

        state.members.insert(member);
        state
            .vouches
            .insert(member, [voucher1, voucher2].into_iter().collect());
        state.flags.insert(member, [flagger].into_iter().collect());

        // Standing = effective_vouches - regular_flags
        // effective_vouches = 2 (no voucher-flaggers)
        // regular_flags = 1 (no voucher-flaggers)
        // Standing = 2 - 1 = 1
        assert_eq!(state.calculate_standing(&member), Some(1));
    }

    #[test]
    fn test_standing_non_member() {
        let state = TrustNetworkState::new();
        let member = test_member(1);

        assert_eq!(state.calculate_standing(&member), None);
    }

    #[test]
    fn test_merge_is_commutative() {
        let mut state_a = TrustNetworkState::new();
        let mut state_b = TrustNetworkState::new();

        let member1 = test_member(1);
        let member2 = test_member(2);

        state_a.members.insert(member1);
        state_b.members.insert(member2);

        // Merge A + B
        let mut result_ab = state_a.clone();
        result_ab.merge(&state_b);

        // Merge B + A
        let mut result_ba = state_b.clone();
        result_ba.merge(&state_a);

        // Members should be identical
        assert_eq!(result_ab.members, result_ba.members);
    }

    #[test]
    fn test_config_last_write_wins() {
        let mut state = TrustNetworkState::new();
        state.config.min_vouches = 1;
        state.config_timestamp = 100;

        let delta = StateDelta {
            config_update: Some(ConfigUpdate {
                config: GroupConfig {
                    min_vouches: 5,
                    ..Default::default()
                },
                timestamp: 200,
            }),
            ..Default::default()
        };

        state.apply_delta(&delta);
        assert_eq!(state.config.min_vouches, 5);
        assert_eq!(state.config_timestamp, 200);
    }

    #[test]
    fn test_config_older_timestamp_ignored() {
        let mut state = TrustNetworkState::new();
        state.config.min_vouches = 3;
        state.config_timestamp = 200;

        let delta = StateDelta {
            config_update: Some(ConfigUpdate {
                config: GroupConfig {
                    min_vouches: 1,
                    ..Default::default()
                },
                timestamp: 100, // Older timestamp
            }),
            ..Default::default()
        };

        state.apply_delta(&delta);
        // Config should NOT change
        assert_eq!(state.config.min_vouches, 3);
        assert_eq!(state.config_timestamp, 200);
    }

    #[test]
    fn test_federation_contracts_default() {
        // Test backward compatibility with #[serde(default)]
        let state = TrustNetworkState::new();
        assert!(state.federation_contracts.is_empty());
    }

    #[test]
    fn test_delta_cbor_roundtrip() {
        let delta = StateDelta::new()
            .add_member(test_member(1))
            .add_vouch(test_member(2), test_member(1));

        let bytes = delta.to_bytes().unwrap();
        let recovered = StateDelta::from_bytes(&bytes).unwrap();

        assert_eq!(delta, recovered);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    fn arb_member_hash() -> impl Strategy<Value = MemberHash> {
        any::<[u8; 32]>().prop_map(|bytes| MemberHash::from_bytes(&bytes))
    }

    fn arb_delta() -> impl Strategy<Value = StateDelta> {
        (
            prop::collection::vec(arb_member_hash(), 0..5),
            prop::collection::vec(arb_member_hash(), 0..5),
            prop::collection::vec((arb_member_hash(), arb_member_hash()), 0..5),
            prop::collection::vec((arb_member_hash(), arb_member_hash()), 0..5),
        )
            .prop_map(
                |(members_added, members_removed, vouches_added, flags_added)| StateDelta {
                    members_added,
                    members_removed,
                    vouches_added,
                    vouches_removed: Vec::new(),
                    flags_added,
                    flags_removed: Vec::new(),
                    config_update: None,
                    proposals_created: Vec::new(),
                    proposals_checked: Vec::new(),
                    proposals_with_results: Vec::new(),
                },
            )
    }

    proptest! {
        /// Delta application must be commutative.
        ///
        /// Per freenet-contract-design.bead:
        /// "Deltas must be commutative" - same result regardless of order.
        #[test]
        fn test_delta_commutativity(delta1 in arb_delta(), delta2 in arb_delta()) {
            let initial = TrustNetworkState::new();

            // Order 1: delta1 then delta2
            let mut state1 = initial.clone();
            state1.apply_delta(&delta1);
            state1.apply_delta(&delta2);

            // Order 2: delta2 then delta1
            let mut state2 = initial.clone();
            state2.apply_delta(&delta2);
            state2.apply_delta(&delta1);

            // Members and ejected should be identical regardless of order
            prop_assert_eq!(state1.members, state2.members);
            prop_assert_eq!(state1.ejected, state2.ejected);
            prop_assert_eq!(state1.vouches, state2.vouches);
            prop_assert_eq!(state1.flags, state2.flags);
        }

        /// State merge must be commutative.
        #[test]
        fn test_merge_commutativity(
            members_a in prop::collection::btree_set(arb_member_hash(), 0..10),
            members_b in prop::collection::btree_set(arb_member_hash(), 0..10),
        ) {
            let mut state_a = TrustNetworkState::new();
            state_a.members = members_a;

            let mut state_b = TrustNetworkState::new();
            state_b.members = members_b;

            // Merge A + B
            let mut result_ab = state_a.clone();
            result_ab.merge(&state_b);

            // Merge B + A
            let mut result_ba = state_b.clone();
            result_ba.merge(&state_a);

            prop_assert_eq!(result_ab.members, result_ba.members);
        }

        /// CBOR serialization must be deterministic.
        #[test]
        fn test_cbor_determinism(members in prop::collection::btree_set(arb_member_hash(), 0..10)) {
            let mut state = TrustNetworkState::new();
            state.members = members;

            let bytes1 = state.to_bytes().unwrap();
            let bytes2 = state.to_bytes().unwrap();

            prop_assert_eq!(bytes1, bytes2);
        }

        /// Standing calculation must be consistent.
        #[test]
        fn test_standing_consistency(
            vouch_count in 0u32..10,
            flag_count in 0u32..10,
        ) {
            let mut state = TrustNetworkState::new();
            let member = MemberHash::from_bytes(&[1; 32]);
            state.members.insert(member);

            // Add vouchers (using range 10-19 to avoid overlap with flaggers)
            let mut vouchers = HashSet::new();
            for i in 0..vouch_count {
                vouchers.insert(MemberHash::from_bytes(&[10 + i as u8; 32]));
            }
            if !vouchers.is_empty() {
                state.vouches.insert(member, vouchers);
            }

            // Add flaggers (using range 100-109 to avoid overlap with vouchers)
            let mut flaggers = HashSet::new();
            for i in 0..flag_count {
                flaggers.insert(MemberHash::from_bytes(&[100 + i as u8; 32]));
            }
            if !flaggers.is_empty() {
                state.flags.insert(member, flaggers);
            }

            // With no overlap: effective_vouches = vouch_count, regular_flags = flag_count
            let standing = state.calculate_standing(&member).unwrap();
            let expected = vouch_count as i32 - flag_count as i32;

            prop_assert_eq!(standing, expected);
        }

        /// Property test: No 2-point swing when voucher flags
        ///
        /// This is the KEY REQUIREMENT: when someone who vouched for a member
        /// then flags that member, they should be excluded from BOTH counts,
        /// preventing a 2-point swing in standing.
        #[test]
        fn test_no_2point_swing_voucher_flags(
            base_vouchers in 0u32..5,
            base_flaggers in 0u32..5,
            voucher_flaggers in 1u32..3,
        ) {
            let mut state = TrustNetworkState::new();
            let member = MemberHash::from_bytes(&[1; 32]);
            state.members.insert(member);

            // Add some base vouchers (no overlap)
            let mut vouchers = HashSet::new();
            for i in 0..base_vouchers {
                vouchers.insert(MemberHash::from_bytes(&[10 + i as u8; 32]));
            }

            // Add some base flaggers (no overlap)
            let mut flaggers = HashSet::new();
            for i in 0..base_flaggers {
                flaggers.insert(MemberHash::from_bytes(&[100 + i as u8; 32]));
            }

            // Add voucher-flaggers (people who both vouched AND flagged)
            for i in 0..voucher_flaggers {
                let vf = MemberHash::from_bytes(&[200 + i as u8; 32]);
                vouchers.insert(vf);
                flaggers.insert(vf);
            }

            state.vouches.insert(member, vouchers.clone());
            state.flags.insert(member, flaggers.clone());

            // Calculate standing
            let standing = state.calculate_standing(&member).unwrap();

            // Expected: voucher-flaggers excluded from BOTH counts
            let expected = base_vouchers as i32 - base_flaggers as i32;

            prop_assert_eq!(
                standing,
                expected,
                "Standing should exclude voucher-flaggers from both counts"
            );
        }

        /// Property test: Vouch invalidation maintains correct standing
        ///
        /// When a voucher flags someone, their vouch should be invalidated.
        /// This tests that the standing calculation handles this correctly.
        #[test]
        fn test_vouch_invalidation_standing(
            vouchers in 2u32..10,
            flaggers in 0u32..5,
            invalidated in 1u32..3,
        ) {
            prop_assume!(invalidated <= vouchers.min(5));

            let mut state = TrustNetworkState::new();
            let member = MemberHash::from_bytes(&[1; 32]);
            state.members.insert(member);

            let mut voucher_set = HashSet::new();
            let mut flagger_set = HashSet::new();

            // Add vouchers
            for i in 0..vouchers {
                voucher_set.insert(MemberHash::from_bytes(&[10 + i as u8; 32]));
            }

            // Add non-voucher flaggers
            for i in 0..flaggers {
                flagger_set.insert(MemberHash::from_bytes(&[100 + i as u8; 32]));
            }

            // Invalidate some vouches by having those vouchers also flag
            let vouchers_vec: Vec<_> = voucher_set.iter().cloned().collect();
            for i in 0..invalidated {
                if i < vouchers_vec.len() as u32 {
                    flagger_set.insert(vouchers_vec[i as usize]);
                }
            }

            state.vouches.insert(member, voucher_set);
            state.flags.insert(member, flagger_set);

            let standing = state.calculate_standing(&member).unwrap();

            // Expected standing:
            // effective_vouches = vouchers - invalidated
            // regular_flags = flaggers (non-voucher flaggers only, since voucher-flaggers excluded)
            let expected = (vouchers - invalidated) as i32 - flaggers as i32;

            prop_assert_eq!(standing, expected);
        }

        /// Property test: Standing never decreases by more than 1 per flag
        ///
        /// A single flag should decrease standing by at most 1 (when the
        /// flagger is not a voucher). If the flagger is a voucher, standing
        /// decreases by 1 (vouch removed) not 2.
        #[test]
        fn test_single_flag_max_decrease(
            initial_vouchers in 2u32..10,
            is_voucher_flag in prop::bool::ANY,
        ) {
            let mut state_before = TrustNetworkState::new();
            let member = MemberHash::from_bytes(&[1; 32]);
            state_before.members.insert(member);

            let mut vouchers = HashSet::new();
            for i in 0..initial_vouchers {
                vouchers.insert(MemberHash::from_bytes(&[10 + i as u8; 32]));
            }
            state_before.vouches.insert(member, vouchers.clone());

            let standing_before = state_before.calculate_standing(&member).unwrap();

            // Add a single flag
            let mut state_after = state_before.clone();
            let flagger = if is_voucher_flag {
                // Use an existing voucher as the flagger
                *vouchers.iter().next().unwrap()
            } else {
                // Use a new person as the flagger
                MemberHash::from_bytes(&[100; 32])
            };

            state_after
                .flags
                .insert(member, [flagger].into_iter().collect());

            let standing_after = state_after.calculate_standing(&member).unwrap();
            let decrease = standing_before - standing_after;

            if is_voucher_flag {
                // Voucher flags: standing decreases by 1 (vouch excluded, flag excluded = net -1)
                prop_assert_eq!(decrease, 1);
            } else {
                // Non-voucher flags: standing decreases by 1 (just adds to regular_flags)
                prop_assert_eq!(decrease, 1);
            }
        }
    }
}
