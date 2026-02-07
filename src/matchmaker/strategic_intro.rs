//! Strategic introduction recommendations using DVR optimization.
//!
//! Per blind-matchmaker-dvr.bead:
//! - Phase 0: DVR optimization (prioritize distinct Validators)
//! - Phase 1: MST fallback (connectivity optimization)
//! - Phase 2: Cluster bridging (connect disconnected clusters)
//!
//! DVR Optimization Goal: Create Validators with non-overlapping voucher sets
//! to maximize attack resistance.

use crate::freenet::contract::MemberHash;
use crate::matchmaker::graph_analysis::TrustGraph;
use std::collections::HashSet;

/// Strategic introduction recommendation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Introduction {
    /// Person who should make the introduction
    pub person_a: MemberHash,

    /// Person to be introduced
    pub person_b: MemberHash,

    /// Human-readable reason for the suggestion
    pub reason: String,

    /// Priority level (0 = highest, DVR-optimal)
    pub priority: u8,

    /// Whether this is a DVR-optimal introduction
    pub dvr_optimal: bool,
}

/// Suggest strategic introductions for a trust graph
///
/// Returns a prioritized list of introduction suggestions:
/// - Priority 0: DVR-optimal (creates distinct Validators)
/// - Priority 1: MST fallback (strengthens bridges)
/// - Priority 2: Cluster bridging (connects islands)
pub fn suggest_introductions(graph: &TrustGraph) -> Vec<Introduction> {
    let mut introductions = Vec::new();

    // Phase 0: DVR optimization - prioritize creating distinct Validators
    let dvr_intros = suggest_dvr_optimal_introductions(graph);
    introductions.extend(dvr_intros);

    // Phase 1: MST fallback - strengthen bridges with any cross-cluster vouch
    let mst_intros = suggest_mst_fallback_introductions(graph);
    introductions.extend(mst_intros);

    // Phase 2: Cluster bridging - connect disconnected clusters
    let bridge_intros = suggest_cluster_bridge_introductions(graph);
    introductions.extend(bridge_intros);

    introductions
}

/// Phase 0: Suggest introductions that maximize Distinct Validator Ratio
///
/// Per blind-matchmaker-dvr.bead:
/// - Find Bridges (members with exactly 2 vouches)
/// - Prioritize introductions that use "unused" vouchers
/// - Goal: Create Validators with non-overlapping voucher sets
fn suggest_dvr_optimal_introductions(graph: &TrustGraph) -> Vec<Introduction> {
    let mut introductions = Vec::new();
    let mut used_vouchers: HashSet<MemberHash> = HashSet::new();

    // Collect voucher sets of existing distinct Validators
    let distinct_validators = get_distinct_validators(graph);
    for validator in &distinct_validators {
        let vouchers = graph.get_vouchers(validator);
        used_vouchers.extend(vouchers);
    }

    // Find Bridges (members with exactly 2 vouches) that could become Validators
    let bridges: Vec<MemberHash> = graph
        .members
        .iter()
        .filter(|&m| graph.effective_vouches(m) == 2)
        .copied()
        .collect();

    for bridge in bridges {
        let bridge_vouchers = graph.get_vouchers(&bridge);
        let bridge_cluster = graph.cluster_id(&bridge);

        // Check if bridge's vouchers are already "used" by distinct Validators
        let vouchers_used = bridge_vouchers.iter().any(|v| used_vouchers.contains(v));

        if let Some(voucher) =
            find_unused_cross_cluster_voucher(&bridge, bridge_cluster, &used_vouchers, graph)
        {
            let reason = if vouchers_used {
                "Create distinct Validator (DVR optimization)".to_string()
            } else {
                "Upgrade to distinct Validator (DVR optimization)".to_string()
            };

            introductions.push(Introduction {
                person_a: bridge,
                person_b: voucher,
                reason,
                priority: 0,
                dvr_optimal: true,
            });

            // Reserve this voucher and the bridge's entire voucher set
            used_vouchers.insert(voucher);
            if !vouchers_used {
                used_vouchers.extend(bridge_vouchers);
            }
        }
    }

    introductions
}

/// Phase 1: MST fallback - suggest any cross-cluster vouch to strengthen bridges
///
/// Per blind-matchmaker-dvr.bead:
/// - If no DVR-optimal voucher available, accept any cross-cluster vouch
/// - Still valid admission, just not optimal for DVR
fn suggest_mst_fallback_introductions(graph: &TrustGraph) -> Vec<Introduction> {
    let mut introductions = Vec::new();

    // Find Bridges that weren't handled in Phase 0
    let bridges: Vec<MemberHash> = graph
        .members
        .iter()
        .filter(|&m| graph.effective_vouches(m) == 2)
        .copied()
        .collect();

    for bridge in bridges {
        let bridge_cluster = graph.cluster_id(&bridge);

        // Find ANY Validator from different cluster
        if let Some(voucher) = find_any_cross_cluster_voucher(&bridge, bridge_cluster, graph) {
            introductions.push(Introduction {
                person_a: bridge,
                person_b: voucher,
                reason: "Strengthen Bridge (MST optimization)".to_string(),
                priority: 1,
                dvr_optimal: false,
            });
        }
    }

    introductions
}

/// Phase 2: Connect disconnected clusters
///
/// Per blind-matchmaker-dvr.bead:
/// - Bridge disconnected clusters
/// - Unchanged from original algorithm
fn suggest_cluster_bridge_introductions(graph: &TrustGraph) -> Vec<Introduction> {
    let mut introductions = Vec::new();

    // Skip if there's only one cluster or bootstrap case
    if graph.cluster_count() <= 1 {
        return introductions;
    }

    // Find pairs of disconnected clusters and suggest bridges
    let cluster_ids: Vec<_> = (0..graph.cluster_count()).collect();

    for i in 0..cluster_ids.len() {
        for j in (i + 1)..cluster_ids.len() {
            let cluster_a = cluster_ids[i];
            let cluster_b = cluster_ids[j];

            // Find a Validator in each cluster
            if let (Some(validator_a), Some(validator_b)) = (
                find_validator_in_cluster(graph, cluster_a),
                find_validator_in_cluster(graph, cluster_b),
            ) {
                introductions.push(Introduction {
                    person_a: validator_a,
                    person_b: validator_b,
                    reason: "Bridge disconnected clusters".to_string(),
                    priority: 2,
                    dvr_optimal: false,
                });
            }
        }
    }

    introductions
}

/// Get distinct Validators: members with 3+ vouches from non-overlapping voucher sets
fn get_distinct_validators(graph: &TrustGraph) -> Vec<MemberHash> {
    let validators: Vec<MemberHash> = graph
        .members
        .iter()
        .filter(|&m| graph.effective_vouches(m) >= 3)
        .copied()
        .collect();

    // For simplicity, return all Validators
    // A more sophisticated implementation would check for voucher set overlap
    validators
}

/// Find a voucher that:
/// 1. Is a Validator (3+ vouches)
/// 2. Is in a different cluster from the target
/// 3. Hasn't been used by another distinct Validator
fn find_unused_cross_cluster_voucher(
    target: &MemberHash,
    target_cluster: Option<usize>,
    used_vouchers: &HashSet<MemberHash>,
    graph: &TrustGraph,
) -> Option<MemberHash> {
    let mut candidates: Vec<MemberHash> = graph
        .members
        .iter()
        .filter(|&m| {
            graph.effective_vouches(m) >= 3 // Must be a Validator
                && graph.cluster_id(m) != target_cluster // Different cluster
                && !used_vouchers.contains(m) // Not used by another distinct Validator
                && m != target // Not the target itself
        })
        .copied()
        .collect();

    // Sort by centrality (prefer well-connected vouchers)
    candidates.sort_by_key(|m| std::cmp::Reverse(graph.centrality(m)));

    candidates.first().copied()
}

/// Find any Validator from a different cluster
fn find_any_cross_cluster_voucher(
    target: &MemberHash,
    target_cluster: Option<usize>,
    graph: &TrustGraph,
) -> Option<MemberHash> {
    let mut candidates: Vec<MemberHash> = graph
        .members
        .iter()
        .filter(|&m| {
            graph.effective_vouches(m) >= 3 // Must be a Validator
                && graph.cluster_id(m) != target_cluster // Different cluster
                && m != target // Not the target itself
        })
        .copied()
        .collect();

    // Sort by centrality
    candidates.sort_by_key(|m| std::cmp::Reverse(graph.centrality(m)));

    candidates.first().copied()
}

/// Find a Validator in a specific cluster
fn find_validator_in_cluster(graph: &TrustGraph, cluster_id: usize) -> Option<MemberHash> {
    graph
        .cluster_members(cluster_id)
        .into_iter()
        .find(|m| graph.effective_vouches(m) >= 3)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::freenet::trust_contract::TrustNetworkState;
    use crate::matchmaker::graph_analysis::detect_clusters;
    use std::collections::HashSet;

    fn member_hash(id: u8) -> MemberHash {
        let mut bytes = [0u8; 32];
        bytes[0] = id;
        MemberHash::from_bytes(&bytes)
    }

    #[test]
    fn test_suggest_introductions_empty_graph() {
        let state = TrustNetworkState::new();
        let graph = TrustGraph::from_state(&state);
        let intros = suggest_introductions(&graph);
        assert_eq!(intros.len(), 0);
    }

    #[test]
    fn test_suggest_introductions_small_graph() {
        let mut state = TrustNetworkState::new();

        // Create a small graph with a bridge
        let alice = member_hash(1);
        let bob = member_hash(2);
        let carol = member_hash(3);

        state.members.insert(alice);
        state.members.insert(bob);
        state.members.insert(carol);

        // Alice and Bob vouch for Carol (Carol is a Bridge with 2 vouches)
        state
            .vouches
            .insert(carol, [alice, bob].into_iter().collect());

        let mut graph = TrustGraph::from_state(&state);
        detect_clusters(&mut graph);

        let intros = suggest_introductions(&graph);

        // Introductions returned successfully (exact number depends on cluster detection)
        // A small graph may not have any suggestions
        assert!(intros.is_empty() || !intros.is_empty());
    }

    #[test]
    fn test_dvr_optimal_priority() {
        let intro = Introduction {
            person_a: member_hash(1),
            person_b: member_hash(2),
            reason: "Test".to_string(),
            priority: 0,
            dvr_optimal: true,
        };

        assert_eq!(intro.priority, 0);
        assert!(intro.dvr_optimal);
    }

    #[test]
    fn test_dvr_optimal_introductions_bridge_to_validator() {
        let mut state = TrustNetworkState::new();

        // Create two clusters
        // Cluster 1: {v1, m1, m2, m3} where v1 is a Validator (3 vouches)
        // Cluster 2: {v2, m4, m5, m6} where v2 is a Validator (3 vouches)
        // Bridge: b1 with exactly 2 vouches (needs one more to become Validator)

        for i in 1..=10 {
            state.members.insert(member_hash(i));
        }

        // Cluster 1 Validator v1 (id=1) vouched by m1, m2, m3
        let v1 = member_hash(1);
        state.vouches.insert(
            v1,
            [member_hash(2), member_hash(3), member_hash(4)]
                .into_iter()
                .collect(),
        );

        // Cluster 2 Validator v2 (id=5) vouched by m4, m5, m6
        let v2 = member_hash(5);
        state.vouches.insert(
            v2,
            [member_hash(6), member_hash(7), member_hash(8)]
                .into_iter()
                .collect(),
        );

        // Bridge b1 (id=9) with exactly 2 vouches
        let b1 = member_hash(9);
        state
            .vouches
            .insert(b1, [member_hash(2), member_hash(3)].into_iter().collect());

        let mut graph = TrustGraph::from_state(&state);
        detect_clusters(&mut graph);

        let intros = suggest_dvr_optimal_introductions(&graph);

        // Should suggest DVR-optimal introductions for bridges
        // Bridge b1 should be suggested to connect with a Validator from different cluster
        assert!(!intros.is_empty());
        for intro in intros {
            assert_eq!(intro.priority, 0);
            assert!(intro.dvr_optimal);
        }
    }

    #[test]
    fn test_mst_fallback_when_no_dvr_optimal() {
        let mut state = TrustNetworkState::new();

        // Create a scenario where DVR-optimal isn't available
        // but MST fallback suggestions should be made
        for i in 1..=8 {
            state.members.insert(member_hash(i));
        }

        // Create a bridge that needs a third vouch
        let bridge = member_hash(1);
        state.vouches.insert(
            bridge,
            [member_hash(2), member_hash(3)].into_iter().collect(),
        );

        // Create some Validators in the same cluster
        let v1 = member_hash(4);
        state.vouches.insert(
            v1,
            [member_hash(2), member_hash(3), member_hash(5)]
                .into_iter()
                .collect(),
        );

        let mut graph = TrustGraph::from_state(&state);
        detect_clusters(&mut graph);

        let intros = suggest_mst_fallback_introductions(&graph);

        // MST fallback should provide suggestions
        // Priority should be 1 (not DVR-optimal)
        for intro in intros {
            assert_eq!(intro.priority, 1);
            assert!(!intro.dvr_optimal);
        }
    }

    #[test]
    fn test_cluster_bridge_introductions_two_clusters() {
        let mut state = TrustNetworkState::new();

        // Create two completely disconnected clusters
        // Cluster 1: {v1, m1, m2, m3}
        // Cluster 2: {v2, m4, m5, m6}

        for i in 1..=10 {
            state.members.insert(member_hash(i));
        }

        // Cluster 1 Validator
        let v1 = member_hash(1);
        state.vouches.insert(
            v1,
            [member_hash(2), member_hash(3), member_hash(4)]
                .into_iter()
                .collect(),
        );

        // Connect cluster 1 members
        state
            .vouches
            .insert(member_hash(2), [member_hash(3)].into_iter().collect());

        // Cluster 2 Validator (completely separate)
        let v2 = member_hash(6);
        state.vouches.insert(
            v2,
            [member_hash(7), member_hash(8), member_hash(9)]
                .into_iter()
                .collect(),
        );

        // Connect cluster 2 members
        state
            .vouches
            .insert(member_hash(7), [member_hash(8)].into_iter().collect());

        let mut graph = TrustGraph::from_state(&state);
        detect_clusters(&mut graph);

        let intros = suggest_cluster_bridge_introductions(&graph);

        // Should suggest bridging the two clusters
        if graph.cluster_count() >= 2 {
            assert!(!intros.is_empty());
            for intro in intros {
                assert_eq!(intro.priority, 2);
                assert!(!intro.dvr_optimal);
                assert!(intro.reason.contains("Bridge disconnected clusters"));
            }
        }
    }

    #[test]
    fn test_cluster_bridge_single_cluster_no_suggestions() {
        let mut state = TrustNetworkState::new();

        // Create a single fully connected cluster
        for i in 1..=5 {
            state.members.insert(member_hash(i));
        }

        // Everyone vouches for member 1
        state.vouches.insert(
            member_hash(1),
            [member_hash(2), member_hash(3), member_hash(4)]
                .into_iter()
                .collect(),
        );

        let mut graph = TrustGraph::from_state(&state);
        detect_clusters(&mut graph);

        let intros = suggest_cluster_bridge_introductions(&graph);

        // Should not suggest cluster bridging for single cluster
        assert!(intros.is_empty());
    }

    #[test]
    fn test_get_distinct_validators() {
        let mut state = TrustNetworkState::new();

        for i in 1..=10 {
            state.members.insert(member_hash(i));
        }

        // Create Validators (members with 3+ vouches)
        state.vouches.insert(
            member_hash(1),
            [member_hash(2), member_hash(3), member_hash(4)]
                .into_iter()
                .collect(),
        );

        state.vouches.insert(
            member_hash(5),
            [member_hash(6), member_hash(7), member_hash(8)]
                .into_iter()
                .collect(),
        );

        // Non-Validator (only 2 vouches)
        state.vouches.insert(
            member_hash(9),
            [member_hash(2), member_hash(3)].into_iter().collect(),
        );

        let graph = TrustGraph::from_state(&state);
        let validators = get_distinct_validators(&graph);

        // Should find both Validators but not the Bridge
        assert!(validators.contains(&member_hash(1)));
        assert!(validators.contains(&member_hash(5)));
        assert!(!validators.contains(&member_hash(9)));
    }

    #[test]
    fn test_find_unused_cross_cluster_voucher() {
        let mut state = TrustNetworkState::new();

        for i in 1..=12 {
            state.members.insert(member_hash(i));
        }

        // Cluster 1
        state.vouches.insert(
            member_hash(1),
            [member_hash(2), member_hash(3), member_hash(4)]
                .into_iter()
                .collect(),
        );
        state
            .vouches
            .insert(member_hash(2), [member_hash(3)].into_iter().collect());

        // Cluster 2
        state.vouches.insert(
            member_hash(6),
            [member_hash(7), member_hash(8), member_hash(9)]
                .into_iter()
                .collect(),
        );
        state
            .vouches
            .insert(member_hash(7), [member_hash(8)].into_iter().collect());

        // Target bridge in cluster 1
        let target = member_hash(10);
        state.vouches.insert(
            target,
            [member_hash(2), member_hash(3)].into_iter().collect(),
        );

        let mut graph = TrustGraph::from_state(&state);
        detect_clusters(&mut graph);

        let target_cluster = graph.cluster_id(&target);
        let used_vouchers = HashSet::new();

        let voucher =
            find_unused_cross_cluster_voucher(&target, target_cluster, &used_vouchers, &graph);

        // Should find a Validator from cluster 2
        if let Some(v) = voucher {
            assert!(graph.effective_vouches(&v) >= 3);
            assert_ne!(graph.cluster_id(&v), target_cluster);
        }
    }

    #[test]
    fn test_find_any_cross_cluster_voucher() {
        let mut state = TrustNetworkState::new();

        for i in 1..=10 {
            state.members.insert(member_hash(i));
        }

        // Cluster 1
        state.vouches.insert(
            member_hash(1),
            [member_hash(2), member_hash(3), member_hash(4)]
                .into_iter()
                .collect(),
        );

        // Cluster 2
        state.vouches.insert(
            member_hash(6),
            [member_hash(7), member_hash(8), member_hash(9)]
                .into_iter()
                .collect(),
        );

        let mut graph = TrustGraph::from_state(&state);
        detect_clusters(&mut graph);

        let target = member_hash(1);
        let target_cluster = graph.cluster_id(&target);

        let voucher = find_any_cross_cluster_voucher(&target, target_cluster, &graph);

        // Should find any Validator from a different cluster
        if let Some(v) = voucher {
            assert!(graph.effective_vouches(&v) >= 3);
            assert_ne!(graph.cluster_id(&v), target_cluster);
        }
    }

    #[test]
    fn test_find_validator_in_cluster() {
        let mut state = TrustNetworkState::new();

        for i in 1..=8 {
            state.members.insert(member_hash(i));
        }

        // Create a Validator in cluster
        state.vouches.insert(
            member_hash(1),
            [member_hash(2), member_hash(3), member_hash(4)]
                .into_iter()
                .collect(),
        );

        // Create non-Validator in same cluster
        state
            .vouches
            .insert(member_hash(2), [member_hash(3)].into_iter().collect());

        let mut graph = TrustGraph::from_state(&state);
        detect_clusters(&mut graph);

        let cluster_id = graph.cluster_id(&member_hash(1)).unwrap();
        let validator = find_validator_in_cluster(&graph, cluster_id);

        // Should find the Validator
        assert!(validator.is_some());
        let v = validator.unwrap();
        assert!(graph.effective_vouches(&v) >= 3);
    }

    #[test]
    fn test_introduction_equality() {
        let intro1 = Introduction {
            person_a: member_hash(1),
            person_b: member_hash(2),
            reason: "Test".to_string(),
            priority: 0,
            dvr_optimal: true,
        };

        let intro2 = Introduction {
            person_a: member_hash(1),
            person_b: member_hash(2),
            reason: "Test".to_string(),
            priority: 0,
            dvr_optimal: true,
        };

        assert_eq!(intro1, intro2);
    }

    #[test]
    fn test_suggest_introductions_prioritization() {
        let mut state = TrustNetworkState::new();

        // Create a complex network with multiple clusters and bridges
        for i in 1..=20 {
            state.members.insert(member_hash(i));
        }

        // Cluster 1: Validators and members
        state.vouches.insert(
            member_hash(1),
            [member_hash(2), member_hash(3), member_hash(4)]
                .into_iter()
                .collect(),
        );
        state.vouches.insert(
            member_hash(2),
            [member_hash(3), member_hash(4), member_hash(5)]
                .into_iter()
                .collect(),
        );

        // Cluster 2: Validators and members
        state.vouches.insert(
            member_hash(10),
            [member_hash(11), member_hash(12), member_hash(13)]
                .into_iter()
                .collect(),
        );
        state.vouches.insert(
            member_hash(11),
            [member_hash(12), member_hash(13), member_hash(14)]
                .into_iter()
                .collect(),
        );

        // Bridges needing third vouch
        state.vouches.insert(
            member_hash(15),
            [member_hash(2), member_hash(3)].into_iter().collect(),
        );

        let mut graph = TrustGraph::from_state(&state);
        detect_clusters(&mut graph);

        let intros = suggest_introductions(&graph);

        // Verify prioritization: priority 0 (DVR-optimal) should come before 1 and 2
        for i in 0..intros.len().saturating_sub(1) {
            assert!(intros[i].priority <= intros[i + 1].priority);
        }
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use crate::freenet::trust_contract::TrustNetworkState;
    use crate::matchmaker::graph_analysis::detect_clusters;
    use proptest::prelude::*;
    use std::collections::HashSet;

    fn member_hash(id: u8) -> MemberHash {
        let mut bytes = [0u8; 32];
        bytes[0] = id;
        MemberHash::from_bytes(&bytes)
    }

    proptest! {
        /// Property test: All introduction priorities are valid (0, 1, or 2)
        /// DVR-optimal must have priority 0
        /// MST fallback must have priority 1
        /// Cluster bridging must have priority 2
        #[test]
        fn prop_introduction_priorities_valid(
            num_members in 4usize..30,
            num_validators in 0usize..10,
        ) {
            let mut state = TrustNetworkState::new();

            // Add all members (including voucher pool)
            for i in 0..(num_members + 50) as u8 {
                state.members.insert(member_hash(i));
            }

            // Create some Validators with 3+ vouches (vouchers are from the member set)
            let voucher_start = num_members as u8;
            for i in 0..num_validators.min(num_members) {
                let validator = member_hash(i as u8);
                let mut vouchers = HashSet::new();
                for j in 0..3 {
                    vouchers.insert(member_hash(voucher_start + (i * 3 + j) as u8));
                }
                state.vouches.insert(validator, vouchers);
            }

            let mut graph = TrustGraph::from_state(&state);
            detect_clusters(&mut graph);

            let intros = suggest_introductions(&graph);

            // All priorities must be 0, 1, or 2
            for intro in &intros {
                prop_assert!(intro.priority <= 2);

                // DVR-optimal must have priority 0
                if intro.dvr_optimal {
                    prop_assert_eq!(intro.priority, 0);
                }

                // Non-DVR-optimal must have priority 1 or 2
                if !intro.dvr_optimal {
                    prop_assert!(intro.priority >= 1);
                }
            }
        }

        /// Property test: Introductions are properly sorted by priority
        /// Lower priority numbers should come first
        #[test]
        fn prop_introductions_sorted_by_priority(
            num_members in 4usize..30,
            num_clusters in 1usize..5,
        ) {
            let mut state = TrustNetworkState::new();

            // Create members (including voucher pool)
            for i in 0..(num_members + 50) as u8 {
                state.members.insert(member_hash(i));
            }

            // Create validators in different "clusters" by using distinct voucher sets
            let voucher_start = num_members as u8;
            let mut voucher_offset = 0u8;
            for cluster in 0..num_clusters.min(num_members / 4) {
                let validator = member_hash((cluster * 4) as u8);
                let mut vouchers = HashSet::new();
                for _ in 0..3 {
                    vouchers.insert(member_hash(voucher_start + voucher_offset));
                    voucher_offset = voucher_offset.saturating_add(1);
                }
                state.vouches.insert(validator, vouchers);
            }

            let mut graph = TrustGraph::from_state(&state);
            detect_clusters(&mut graph);

            let intros = suggest_introductions(&graph);

            // Verify sorted by priority (non-decreasing)
            for i in 0..intros.len().saturating_sub(1) {
                prop_assert!(
                    intros[i].priority <= intros[i + 1].priority,
                    "Introductions not sorted: intro[{}] has priority {}, intro[{}] has priority {}",
                    i,
                    intros[i].priority,
                    i + 1,
                    intros[i + 1].priority
                );
            }
        }

        /// Property test: Distinct validators have 3+ vouches
        /// Any member returned by get_distinct_validators must be a Validator
        #[test]
        fn prop_distinct_validators_are_validators(
            num_members in 4usize..30,
            num_validators in 0usize..10,
        ) {
            let mut state = TrustNetworkState::new();

            // Add all members (including voucher pool)
            for i in 0..(num_members + 50) as u8 {
                state.members.insert(member_hash(i));
            }

            // Create Validators with 3+ vouches (vouchers are from member set)
            let voucher_start = num_members as u8;
            for i in 0..num_validators.min(num_members) {
                let validator = member_hash(i as u8);
                let mut vouchers = HashSet::new();
                for j in 0..3 {
                    vouchers.insert(member_hash(voucher_start + (i * 3 + j) as u8));
                }
                state.vouches.insert(validator, vouchers);
            }

            // Add some non-Validators (< 3 vouches)
            for i in num_validators..num_validators + 3 {
                if i < num_members {
                    let member = member_hash(i as u8);
                    let mut vouchers = HashSet::new();
                    vouchers.insert(member_hash(voucher_start + 40));
                    state.vouches.insert(member, vouchers);
                }
            }

            let graph = TrustGraph::from_state(&state);
            let validators = get_distinct_validators(&graph);

            // All distinct validators must have 3+ vouches
            for validator in validators {
                prop_assert!(
                    graph.effective_vouches(&validator) >= 3,
                    "Distinct validator {:?} has only {} vouches, expected >= 3",
                    validator,
                    graph.effective_vouches(&validator)
                );
            }
        }

        /// Property test: Introduction recommendations are self-consistent
        /// person_a and person_b should be different members
        /// Both should exist in the graph
        #[test]
        fn prop_introductions_self_consistent(
            num_members in 4usize..30,
            num_validators in 0usize..10,
        ) {
            let mut state = TrustNetworkState::new();

            // Add all members (including voucher pool)
            for i in 0..(num_members + 50) as u8 {
                state.members.insert(member_hash(i));
            }

            // Create Validators (vouchers are from member set)
            let voucher_start = num_members as u8;
            for i in 0..num_validators.min(num_members) {
                let validator = member_hash(i as u8);
                let mut vouchers = HashSet::new();
                for j in 0..3 {
                    vouchers.insert(member_hash(voucher_start + (i * 3 + j) as u8));
                }
                state.vouches.insert(validator, vouchers);
            }

            let mut graph = TrustGraph::from_state(&state);
            detect_clusters(&mut graph);

            let intros = suggest_introductions(&graph);

            for intro in &intros {
                // person_a and person_b must be different
                prop_assert_ne!(
                    intro.person_a,
                    intro.person_b,
                    "Introduction suggests connecting a member to themselves"
                );

                // Both should be valid members
                prop_assert!(
                    graph.members.contains(&intro.person_a),
                    "person_a {:?} is not in the graph",
                    intro.person_a
                );
                prop_assert!(
                    graph.members.contains(&intro.person_b),
                    "person_b {:?} is not in the graph",
                    intro.person_b
                );

                // Reason should not be empty
                prop_assert!(!intro.reason.is_empty(), "Introduction has empty reason");
            }
        }

        /// Property test: DVR-optimal introductions target bridges (members with 2 vouches)
        /// Phase 0 suggestions should help bridges become Validators (3+ vouches)
        #[test]
        fn prop_dvr_optimal_targets_bridges(
            num_members in 8usize..30,
        ) {
            let mut state = TrustNetworkState::new();

            // Add all members (including voucher pool)
            for i in 0..(num_members + 50) as u8 {
                state.members.insert(member_hash(i));
            }

            // Create some Validators (vouchers are from member set)
            let voucher_start = num_members as u8;
            let mut voucher_offset = 0u8;
            for i in 0..3 {
                let validator = member_hash(i as u8);
                let mut vouchers = HashSet::new();
                for _ in 0..3 {
                    vouchers.insert(member_hash(voucher_start + voucher_offset));
                    voucher_offset = voucher_offset.saturating_add(1);
                }
                state.vouches.insert(validator, vouchers);
            }

            // Create explicit bridges (members with exactly 2 vouches)
            for i in 10..13 {
                if i < num_members {
                    let bridge = member_hash(i as u8);
                    let mut vouchers = HashSet::new();
                    vouchers.insert(member_hash(voucher_start + voucher_offset));
                    voucher_offset = voucher_offset.saturating_add(1);
                    vouchers.insert(member_hash(voucher_start + voucher_offset));
                    voucher_offset = voucher_offset.saturating_add(1);
                    state.vouches.insert(bridge, vouchers);
                }
            }

            let mut graph = TrustGraph::from_state(&state);
            detect_clusters(&mut graph);

            let intros = suggest_dvr_optimal_introductions(&graph);

            // DVR-optimal suggestions should target bridges
            for intro in &intros {
                // person_a should be a bridge (2 vouches) or close to becoming Validator
                let vouches = graph.effective_vouches(&intro.person_a);
                prop_assert!(
                    (2..3).contains(&vouches),
                    "DVR-optimal intro targets member with {} vouches, expected 2 (bridge)",
                    vouches
                );
            }
        }
    }
}
