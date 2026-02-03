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
}

impl Default for GroupConfig {
    fn default() -> Self {
        Self {
            min_vouches: 2,
            max_flags: 3,
            open_membership: false,
            operators: BTreeSet::new(),
        }
    }
}

/// Contract address hash.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContractHash(pub [u8; 32]);

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
    }

    /// Calculate standing for a member.
    ///
    /// Standing = vouches - (flags * weight)
    /// Returns None if member is not active.
    pub fn calculate_standing(&self, member: &MemberHash) -> Option<i32> {
        if !self.members.contains(member) {
            return None;
        }

        let vouch_count = self
            .vouches
            .get(member)
            .map(|v| v.len() as i32)
            .unwrap_or(0);

        let flag_count = self.flags.get(member).map(|f| f.len() as i32).unwrap_or(0);

        // Flags have 2x weight in standing calculation
        Some(vouch_count - (flag_count * 2))
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

        // Standing = 2 vouches - (1 flag * 2) = 0
        assert_eq!(state.calculate_standing(&member), Some(0));
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

            // Add vouchers
            let mut vouchers = HashSet::new();
            for i in 0..vouch_count {
                vouchers.insert(MemberHash::from_bytes(&[10 + i as u8; 32]));
            }
            if !vouchers.is_empty() {
                state.vouches.insert(member, vouchers);
            }

            // Add flaggers
            let mut flaggers = HashSet::new();
            for i in 0..flag_count {
                flaggers.insert(MemberHash::from_bytes(&[100 + i as u8; 32]));
            }
            if !flaggers.is_empty() {
                state.flags.insert(member, flaggers);
            }

            let standing = state.calculate_standing(&member).unwrap();
            let expected = vouch_count as i32 - (flag_count as i32 * 2);

            prop_assert_eq!(standing, expected);
        }
    }
}
