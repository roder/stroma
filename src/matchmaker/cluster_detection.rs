//! Cluster detection using Bridge Removal algorithm.
//!
//! Per blind-matchmaker-dvr.bead and Q3:
//! - Bridge Removal (Tarjan's algorithm) for finding bridges
//! - Separates tight clusters by removing bridge edges
//! - GAP-11: Announce cluster formation when ≥2 clusters detected

use crate::freenet::contract::MemberHash;
use crate::freenet::trust_contract::TrustNetworkState;
use std::collections::{HashMap, HashSet};

/// Cluster ID (arbitrary member from the cluster).
pub type ClusterId = usize;

/// Cluster detection result.
#[derive(Debug, Clone, PartialEq)]
pub struct ClusterResult {
    /// Number of clusters detected.
    pub cluster_count: usize,
    /// Member to cluster ID mapping.
    pub member_clusters: HashMap<MemberHash, ClusterId>,
    /// Clusters (cluster ID -> set of members).
    pub clusters: HashMap<ClusterId, HashSet<MemberHash>>,
}

impl ClusterResult {
    /// Check if cluster formation announcement is needed (≥2 clusters).
    pub fn needs_announcement(&self) -> bool {
        self.cluster_count >= 2
    }

    /// Get GAP-11 announcement message.
    pub fn announcement_message(&self) -> &'static str {
        "📊 Network update: Your group now has distinct sub-communities! \
         Cross-cluster vouching is now required for new members. \
         Existing members are grandfathered."
    }
}

/// Detect clusters using connected components.
///
/// Per Q3 and blind-matchmaker-dvr.bead:
/// - Finds connected components in the vouch graph
/// - Each component represents a cluster
/// - GAP-11: Announce when ≥2 clusters detected
///
/// Note: This is a simplified version. Future enhancement can use Bridge Removal
/// to separate tight clusters within components.
pub fn detect_clusters(state: &TrustNetworkState) -> ClusterResult {
    let members: Vec<MemberHash> = state.members.iter().copied().collect();

    if members.is_empty() {
        return ClusterResult {
            cluster_count: 0,
            member_clusters: HashMap::new(),
            clusters: HashMap::new(),
        };
    }

    // Build adjacency list from vouch relationships
    let graph = build_graph(state);

    // Find connected components (clusters)
    let (member_clusters, clusters) = find_connected_components(&graph, &members);
    let cluster_count = clusters.len();

    ClusterResult {
        cluster_count,
        member_clusters,
        clusters,
    }
}

/// Build undirected graph from vouch relationships.
fn build_graph(state: &TrustNetworkState) -> HashMap<MemberHash, HashSet<MemberHash>> {
    let mut graph: HashMap<MemberHash, HashSet<MemberHash>> = HashMap::new();

    // Initialize all members
    for member in &state.members {
        graph.entry(*member).or_default();
    }

    // Add edges from vouch relationships (bidirectional)
    for (vouchee, vouchers) in &state.vouches {
        for voucher in vouchers {
            // Add edge: voucher <-> vouchee
            graph.entry(*voucher).or_default().insert(*vouchee);
            graph.entry(*vouchee).or_default().insert(*voucher);
        }
    }

    graph
}

// TODO: Future enhancement - Bridge Removal algorithm
// Per Q3 and blind-matchmaker-dvr.bead, we can use Tarjan's algorithm
// to find bridge edges and remove them to separate tight clusters.
// For MVP, we use simple connected components.
//
// Uncomment and integrate when ready for advanced cluster detection:
//
// /// Find bridge edges using Tarjan's algorithm.
// fn find_bridges(...) -> HashSet<(MemberHash, MemberHash)> {
//     // Implementation uses DFS to find bridges
//     // A bridge is an edge whose removal increases connected components
// }

/// Find connected components using DFS.
fn find_connected_components(
    graph: &HashMap<MemberHash, HashSet<MemberHash>>,
    members: &[MemberHash],
) -> (HashMap<MemberHash, ClusterId>, HashMap<ClusterId, HashSet<MemberHash>>) {
    let mut visited = HashSet::new();
    let mut member_clusters = HashMap::new();
    let mut clusters = HashMap::new();
    let mut cluster_id = 0;

    for &member in members {
        if !visited.contains(&member) {
            let mut component = HashSet::new();
            dfs_component(member, graph, &mut visited, &mut component);

            // Assign cluster ID to all members in this component
            for &m in &component {
                member_clusters.insert(m, cluster_id);
            }

            clusters.insert(cluster_id, component);
            cluster_id += 1;
        }
    }

    (member_clusters, clusters)
}

/// DFS to find connected component.
fn dfs_component(
    u: MemberHash,
    graph: &HashMap<MemberHash, HashSet<MemberHash>>,
    visited: &mut HashSet<MemberHash>,
    component: &mut HashSet<MemberHash>,
) {
    visited.insert(u);
    component.insert(u);

    if let Some(neighbors) = graph.get(&u) {
        for &v in neighbors {
            if !visited.contains(&v) {
                dfs_component(v, graph, visited, component);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_member(id: u8) -> MemberHash {
        MemberHash::from_bytes(&[id; 32])
    }

    #[test]
    fn test_empty_network() {
        let state = TrustNetworkState::new();
        let result = detect_clusters(&state);
        assert_eq!(result.cluster_count, 0);
        assert!(!result.needs_announcement());
    }

    #[test]
    fn test_single_cluster() {
        let mut state = TrustNetworkState::new();

        // Create a fully connected network (single cluster)
        for i in 1..=5 {
            state.members.insert(test_member(i));
        }

        // Add vouches to create full connectivity
        let mut vouchers1 = HashSet::new();
        vouchers1.insert(test_member(2));
        vouchers1.insert(test_member(3));
        state.vouches.insert(test_member(1), vouchers1);

        let mut vouchers2 = HashSet::new();
        vouchers2.insert(test_member(3));
        vouchers2.insert(test_member(4));
        state.vouches.insert(test_member(2), vouchers2);

        let mut vouchers3 = HashSet::new();
        vouchers3.insert(test_member(4));
        vouchers3.insert(test_member(5));
        state.vouches.insert(test_member(3), vouchers3);

        let result = detect_clusters(&state);
        assert_eq!(result.cluster_count, 1);
        assert!(!result.needs_announcement());
    }

    #[test]
    fn test_two_disconnected_clusters() {
        let mut state = TrustNetworkState::new();

        // Create two completely disconnected clusters
        // Cluster 1: {1, 2}
        // Cluster 2: {3, 4}
        // No connection between them

        for i in 1..=4 {
            state.members.insert(test_member(i));
        }

        // Cluster 1: 2 vouches for 1
        let mut vouchers1 = HashSet::new();
        vouchers1.insert(test_member(2));
        state.vouches.insert(test_member(1), vouchers1);

        // Cluster 2: 4 vouches for 3
        let mut vouchers3 = HashSet::new();
        vouchers3.insert(test_member(4));
        state.vouches.insert(test_member(3), vouchers3);

        let result = detect_clusters(&state);
        assert_eq!(result.cluster_count, 2, "Expected 2 clusters, got {}", result.cluster_count);
        assert!(result.needs_announcement());
    }

    #[test]
    fn test_connected_path() {
        let mut state = TrustNetworkState::new();

        // Path graph: 1-2-3 (all connected)
        // This forms a single connected component

        for i in 1..=3 {
            state.members.insert(test_member(i));
        }

        // 2 vouches for 1, creating edge 1-2
        let mut vouchers1 = HashSet::new();
        vouchers1.insert(test_member(2));
        state.vouches.insert(test_member(1), vouchers1);

        // 3 vouches for 2, creating edge 2-3
        let mut vouchers2 = HashSet::new();
        vouchers2.insert(test_member(3));
        state.vouches.insert(test_member(2), vouchers2);

        let result = detect_clusters(&state);
        // All nodes are connected, so should be 1 cluster
        assert_eq!(result.cluster_count, 1);
        assert!(!result.needs_announcement());
    }

    #[test]
    fn test_three_clusters() {
        let mut state = TrustNetworkState::new();

        // Create three separate clusters
        // Cluster 1: {1, 2}
        // Cluster 2: {3, 4}
        // Cluster 3: {5, 6}

        for i in 1..=6 {
            state.members.insert(test_member(i));
        }

        // Cluster 1
        let mut vouchers1 = HashSet::new();
        vouchers1.insert(test_member(2));
        state.vouches.insert(test_member(1), vouchers1);

        // Cluster 2
        let mut vouchers3 = HashSet::new();
        vouchers3.insert(test_member(4));
        state.vouches.insert(test_member(3), vouchers3);

        // Cluster 3
        let mut vouchers5 = HashSet::new();
        vouchers5.insert(test_member(6));
        state.vouches.insert(test_member(5), vouchers5);

        let result = detect_clusters(&state);
        assert_eq!(result.cluster_count, 3);
        assert!(result.needs_announcement());
    }

    #[test]
    fn test_gap11_announcement_message() {
        let mut state = TrustNetworkState::new();

        // Create two clusters
        for i in 1..=4 {
            state.members.insert(test_member(i));
        }

        let mut vouchers1 = HashSet::new();
        vouchers1.insert(test_member(2));
        state.vouches.insert(test_member(1), vouchers1);

        let mut vouchers3 = HashSet::new();
        vouchers3.insert(test_member(4));
        state.vouches.insert(test_member(3), vouchers3);

        let result = detect_clusters(&state);
        assert!(result.needs_announcement());
        assert!(result.announcement_message().contains("sub-communities"));
        assert!(result.announcement_message().contains("grandfathered"));
    }

    #[test]
    fn test_isolated_members() {
        let mut state = TrustNetworkState::new();

        // Add isolated members (no vouches)
        for i in 1..=5 {
            state.members.insert(test_member(i));
        }

        let result = detect_clusters(&state);
        // Each isolated member is its own cluster
        assert_eq!(result.cluster_count, 5);
        assert!(result.needs_announcement());
    }

    #[test]
    fn test_member_cluster_mapping() {
        let mut state = TrustNetworkState::new();

        // Create two clusters
        for i in 1..=4 {
            state.members.insert(test_member(i));
        }

        // Cluster 1: {1, 2}
        let mut vouchers1 = HashSet::new();
        vouchers1.insert(test_member(2));
        state.vouches.insert(test_member(1), vouchers1);

        // Cluster 2: {3, 4}
        let mut vouchers3 = HashSet::new();
        vouchers3.insert(test_member(4));
        state.vouches.insert(test_member(3), vouchers3);

        let result = detect_clusters(&state);

        // Verify members are assigned to clusters
        let cluster1 = result.member_clusters.get(&test_member(1));
        let cluster2 = result.member_clusters.get(&test_member(2));
        let cluster3 = result.member_clusters.get(&test_member(3));
        let cluster4 = result.member_clusters.get(&test_member(4));

        assert!(cluster1.is_some());
        assert!(cluster2.is_some());
        assert!(cluster3.is_some());
        assert!(cluster4.is_some());

        // Members 1 and 2 should be in same cluster
        assert_eq!(cluster1, cluster2);

        // Members 3 and 4 should be in same cluster
        assert_eq!(cluster3, cluster4);

        // But different from cluster 1
        assert_ne!(cluster1, cluster3);
    }

    #[test]
    fn test_build_graph() {
        let mut state = TrustNetworkState::new();

        state.members.insert(test_member(1));
        state.members.insert(test_member(2));
        state.members.insert(test_member(3));

        let mut vouchers1 = HashSet::new();
        vouchers1.insert(test_member(2));
        state.vouches.insert(test_member(1), vouchers1);

        let graph = build_graph(&state);

        // Check bidirectional edges
        assert!(graph.get(&test_member(1)).unwrap().contains(&test_member(2)));
        assert!(graph.get(&test_member(2)).unwrap().contains(&test_member(1)));

        // Member 3 should have no edges
        assert_eq!(graph.get(&test_member(3)).unwrap().len(), 0);
    }
}
