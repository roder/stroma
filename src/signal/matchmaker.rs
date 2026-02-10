//! Blind Matchmaker: Cross-Cluster Assessor Selection
//!
//! Selects assessors for vetting interviews optimizing for:
//! 1. Cross-cluster diversity (security requirement)
//! 2. DVR optimization (distinct validators, non-overlapping voucher sets)
//! 3. Network health (spread trust connections)
//!
//! See: README.md § Blind Matchmaker, docs/ALGORITHMS.md

use crate::freenet::contract::MemberHash;
use crate::freenet::trust_contract::TrustNetworkState;
use crate::matchmaker::graph_analysis::{detect_clusters, TrustGraph};
use std::collections::HashSet;

/// Blind Matchmaker for assessor selection
pub struct BlindMatchmaker;

impl BlindMatchmaker {
    /// Select an assessor for vetting interview
    ///
    /// Requirements:
    /// 1. Must be an active member
    /// 2. Must NOT be the inviter
    /// 3. Must NOT be in the exclusion list
    /// 4. Should be from a different cluster than inviter (cross-cluster)
    /// 5. Optimize for DVR (distinct validators with non-overlapping voucher sets)
    ///
    /// Selection priority:
    /// 1. Cross-cluster member (DVR-optimal) - not in exclusion list
    /// 2. Any cross-cluster member (MST fallback)
    /// 3. Bridge-level (different cluster)
    /// 4. Bootstrap exception (single cluster scenario)
    ///
    /// Candidates are either:
    /// - Validators: 3+ vouches
    /// - Bridges: 2 vouches
    ///
    /// Sorted by centrality for optimal trust network health.
    pub fn select_validator(
        state: &TrustNetworkState,
        inviter: &MemberHash,
        excluded: &HashSet<MemberHash>,
    ) -> Option<MemberHash> {
        // Build trust graph and detect clusters
        let mut graph = TrustGraph::from_state(state);
        detect_clusters(&mut graph);

        // Get inviter's cluster
        let inviter_cluster = graph.cluster_id(inviter);

        // Filter candidates: validators (3+ vouches) OR bridges (2 vouches)
        let candidates: Vec<_> = state
            .members
            .iter()
            .filter(|member| {
                *member != inviter // Not the inviter
                    && graph.effective_vouches(member) >= 2 // At least 2 vouches (bridge or validator)
            })
            .copied()
            .collect();

        // Categorize candidates by priority
        let (dvr_optimal, cross_cluster, same_cluster): (Vec<_>, Vec<_>, Vec<_>) =
            candidates.iter().fold(
                (Vec::new(), Vec::new(), Vec::new()),
                |(mut dvr, mut cross, mut same), &member| {
                    let member_cluster = graph.cluster_id(&member);

                    // Check if cross-cluster
                    let is_cross_cluster = match (inviter_cluster, member_cluster) {
                        (Some(ic), Some(mc)) => ic != mc,
                        _ => false,
                    };

                    if is_cross_cluster && !excluded.contains(&member) {
                        // DVR-optimal: cross-cluster AND not excluded
                        dvr.push(member);
                    } else if is_cross_cluster {
                        // Cross-cluster but in exclusion list (MST fallback)
                        cross.push(member);
                    } else {
                        // Same cluster (bridge-level or bootstrap)
                        same.push(member);
                    }

                    (dvr, cross, same)
                },
            );

        // Select from each priority tier, sorted by centrality
        for tier in [dvr_optimal, cross_cluster, same_cluster] {
            if !tier.is_empty() {
                let mut sorted_tier = tier;
                sorted_tier.sort_by_key(|member| std::cmp::Reverse(graph.centrality(member)));
                return Some(sorted_tier[0]);
            }
        }

        // Bootstrap exception: if no candidates found, fall back to any member except inviter
        // This handles very small networks
        state
            .members
            .iter()
            .find(|member| *member != inviter)
            .copied()
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
        let excluded = HashSet::new();

        let assessor = BlindMatchmaker::select_validator(&state, &alice, &excluded);

        assert!(assessor.is_some());
        let assessor = assessor.unwrap();

        // Should not be Alice
        assert_ne!(assessor, alice);

        // Should be Bob (2 vouches = bridge level, Carol only has 1 vouch)
        // New behavior: Candidates must have >= 2 vouches (bridge or validator level)
        assert_eq!(assessor, test_member_hash(2));
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

        let excluded = HashSet::new();

        // Should still select Bob (bootstrap exception)
        let assessor = BlindMatchmaker::select_validator(&state, &alice, &excluded);
        assert_eq!(assessor, Some(bob));
    }

    #[test]
    fn test_select_validator_empty_network() {
        let mut state = TrustNetworkState::new();
        let alice = test_member_hash(1);
        state.members.insert(alice);

        let excluded = HashSet::new();

        // No other members to select
        let assessor = BlindMatchmaker::select_validator(&state, &alice, &excluded);
        assert!(assessor.is_none());
    }

}
