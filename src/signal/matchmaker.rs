//! Blind Matchmaker: Cross-Cluster Validator Selection
//!
//! Selects validators for vetting interviews optimizing for:
//! 1. Cross-cluster diversity (security requirement)
//! 2. DVR optimization (distinct validators, non-overlapping voucher sets)
//! 3. Network health (spread trust connections)
//!
//! See: README.md § Blind Matchmaker, docs/ALGORITHMS.md

use crate::freenet::contract::MemberHash;
use crate::freenet::trust_contract::TrustNetworkState;
use std::collections::HashSet;

/// Blind Matchmaker for validator selection
pub struct BlindMatchmaker;

impl BlindMatchmaker {
    /// Select a validator for vetting interview
    ///
    /// Requirements:
    /// 1. Must be an active member
    /// 2. Must NOT be the inviter
    /// 3. Should be from a different cluster than inviter (cross-cluster)
    /// 4. Optimize for DVR (distinct validators with non-overlapping voucher sets)
    ///
    /// For Phase 0 MVP:
    /// - Simple selection: any member not in inviter's voucher set
    /// - TODO Phase 1: Full cluster detection and MST-based matching
    pub fn select_validator(
        state: &TrustNetworkState,
        inviter: &MemberHash,
    ) -> Option<MemberHash> {
        // Get inviter's vouchers (their cluster/peer circle)
        let inviter_vouchers = state
            .vouches
            .get(inviter)
            .cloned()
            .unwrap_or_else(HashSet::new);

        // Find candidates: active members who are NOT in inviter's voucher set
        // This approximates "different cluster" for Phase 0
        let candidates: Vec<_> = state
            .members
            .iter()
            .filter(|member| {
                *member != inviter && // Not the inviter
                !inviter_vouchers.contains(member) // Different cluster (approximation)
            })
            .collect();

        // If no cross-cluster candidates, fall back to any member except inviter
        // (Bootstrap exception: single cluster scenario)
        if candidates.is_empty() {
            return state
                .members
                .iter()
                .find(|member| *member != inviter)
                .copied();
        }

        // Select first candidate (Phase 0 simple selection)
        // TODO Phase 1: Use DVR optimization (MST, non-overlapping voucher sets)
        candidates.first().copied().copied()
    }

    /// Check if two members are in different clusters
    ///
    /// Phase 0 approximation: members are in different clusters if they
    /// don't vouch for each other and don't share most of their vouchers.
    ///
    /// TODO Phase 1: Full cluster detection via Bridge Removal algorithm
    pub fn are_cross_cluster(
        state: &TrustNetworkState,
        member1: &MemberHash,
        member2: &MemberHash,
    ) -> bool {
        // Get voucher sets
        let vouchers1 = state
            .vouches
            .get(member1)
            .cloned()
            .unwrap_or_else(HashSet::new);
        let vouchers2 = state
            .vouches
            .get(member2)
            .cloned()
            .unwrap_or_else(HashSet::new);

        // Check if they vouch for each other (same cluster indicator)
        if vouchers1.contains(member2) || vouchers2.contains(member1) {
            return false;
        }

        // Check voucher set overlap
        let overlap: HashSet<_> = vouchers1.intersection(&vouchers2).collect();

        // If ALL vouchers overlap AND both have 2+ vouchers, likely same cluster
        // Single shared voucher in small network is acceptable (bootstrap scenario)
        let min_size = vouchers1.len().min(vouchers2.len());
        if min_size >= 2 && overlap.len() == min_size {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_member_hash(id: u8) -> MemberHash {
        MemberHash::from_bytes(&[id; 32])
    }

    fn create_test_state() -> TrustNetworkState {
        let mut state = TrustNetworkState::new();

        // Add members
        let alice = test_member_hash(1);
        let bob = test_member_hash(2);
        let carol = test_member_hash(3);

        state.members.insert(alice);
        state.members.insert(bob);
        state.members.insert(carol);

        // Alice vouched by Bob
        let mut alice_vouchers = HashSet::new();
        alice_vouchers.insert(bob);
        state.vouches.insert(alice, alice_vouchers);

        // Bob vouched by Alice and Carol (different cluster)
        let mut bob_vouchers = HashSet::new();
        bob_vouchers.insert(alice);
        bob_vouchers.insert(carol);
        state.vouches.insert(bob, bob_vouchers);

        // Carol vouched by Bob
        let mut carol_vouchers = HashSet::new();
        carol_vouchers.insert(bob);
        state.vouches.insert(carol, carol_vouchers);

        state
    }

    #[test]
    fn test_select_validator_simple() {
        let state = create_test_state();
        let alice = test_member_hash(1);

        let validator = BlindMatchmaker::select_validator(&state, &alice);

        assert!(validator.is_some());
        let validator = validator.unwrap();

        // Should not be Alice
        assert_ne!(validator, alice);

        // Should be Carol (not in Alice's voucher set)
        assert_eq!(validator, test_member_hash(3));
    }

    #[test]
    fn test_select_validator_no_cross_cluster() {
        let mut state = TrustNetworkState::new();

        let alice = test_member_hash(1);
        let bob = test_member_hash(2);

        state.members.insert(alice);
        state.members.insert(bob);

        // Everyone vouches for each other (single cluster)
        let mut alice_vouchers = HashSet::new();
        alice_vouchers.insert(bob);
        state.vouches.insert(alice, alice_vouchers);

        let mut bob_vouchers = HashSet::new();
        bob_vouchers.insert(alice);
        state.vouches.insert(bob, bob_vouchers);

        // Should still select Bob (bootstrap exception)
        let validator = BlindMatchmaker::select_validator(&state, &alice);
        assert_eq!(validator, Some(bob));
    }

    #[test]
    fn test_select_validator_empty_network() {
        let mut state = TrustNetworkState::new();
        let alice = test_member_hash(1);
        state.members.insert(alice);

        // No other members to select
        let validator = BlindMatchmaker::select_validator(&state, &alice);
        assert!(validator.is_none());
    }

    #[test]
    fn test_are_cross_cluster_no_overlap() {
        let state = create_test_state();
        let alice = test_member_hash(1);
        let carol = test_member_hash(3);

        // Alice and Carol don't vouch for each other and have different vouchers
        assert!(BlindMatchmaker::are_cross_cluster(&state, &alice, &carol));
    }

    #[test]
    fn test_are_cross_cluster_mutual_vouch() {
        let mut state = TrustNetworkState::new();
        let alice = test_member_hash(1);
        let bob = test_member_hash(2);

        let mut alice_vouchers = HashSet::new();
        alice_vouchers.insert(bob);
        state.vouches.insert(alice, alice_vouchers);

        let mut bob_vouchers = HashSet::new();
        bob_vouchers.insert(alice);
        state.vouches.insert(bob, bob_vouchers);

        // They vouch for each other - same cluster
        assert!(!BlindMatchmaker::are_cross_cluster(&state, &alice, &bob));
    }
}
