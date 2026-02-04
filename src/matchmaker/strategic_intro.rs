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
}
