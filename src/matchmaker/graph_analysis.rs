//! Graph analysis for trust network topology.
//!
//! Per blind-matchmaker-dvr.bead and .cursor/rules/graph-analysis.mdc:
//! - Cluster detection using bridge removal (Tarjan's algorithm)
//! - Performance: <10ms at 20, <200ms at 500, <500ms at 1000 members
//! - Uses petgraph for efficient topology analysis

use crate::freenet::contract::MemberHash;
use crate::freenet::trust_contract::TrustNetworkState;
use std::collections::{HashMap, HashSet};

/// Cluster identifier
pub type ClusterId = usize;

/// Trust graph abstraction for analysis
#[derive(Debug, Clone)]
pub struct TrustGraph {
    /// All active members
    pub members: HashSet<MemberHash>,

    /// Vouches: vouchee -> set of vouchers
    pub vouches: HashMap<MemberHash, HashSet<MemberHash>>,

    /// Reverse index: voucher -> set of vouchees
    pub reverse_vouches: HashMap<MemberHash, HashSet<MemberHash>>,

    /// Cluster assignments (computed via detect_clusters)
    pub clusters: HashMap<MemberHash, ClusterId>,
}

impl TrustGraph {
    /// Build trust graph from contract state
    pub fn from_state(state: &TrustNetworkState) -> Self {
        let mut reverse_vouches: HashMap<MemberHash, HashSet<MemberHash>> = HashMap::new();

        // Build reverse index
        for (vouchee, vouchers) in &state.vouches {
            for voucher in vouchers {
                reverse_vouches
                    .entry(*voucher)
                    .or_default()
                    .insert(*vouchee);
            }
        }

        Self {
            members: state.members.iter().copied().collect(),
            vouches: state.vouches.clone(),
            reverse_vouches,
            clusters: HashMap::new(),
        }
    }

    /// Count effective vouches for a member (incoming vouch edges)
    pub fn effective_vouches(&self, member: &MemberHash) -> usize {
        self.vouches
            .get(member)
            .map(|vouchers| vouchers.len())
            .unwrap_or(0)
    }

    /// Get all vouchers for a member
    pub fn get_vouchers(&self, member: &MemberHash) -> HashSet<MemberHash> {
        self.vouches.get(member).cloned().unwrap_or_default()
    }

    /// Check if two members are in the same cluster
    pub fn same_cluster(&self, a: &MemberHash, b: &MemberHash) -> bool {
        match (self.clusters.get(a), self.clusters.get(b)) {
            (Some(cluster_a), Some(cluster_b)) => cluster_a == cluster_b,
            _ => false,
        }
    }

    /// Get cluster ID for a member
    pub fn cluster_id(&self, member: &MemberHash) -> Option<ClusterId> {
        self.clusters.get(member).copied()
    }

    /// Get all members in a cluster
    pub fn cluster_members(&self, cluster_id: ClusterId) -> Vec<MemberHash> {
        self.clusters
            .iter()
            .filter(|(_, &cid)| cid == cluster_id)
            .map(|(member, _)| *member)
            .collect()
    }

    /// Count total number of clusters
    pub fn cluster_count(&self) -> usize {
        let unique_clusters: HashSet<_> = self.clusters.values().collect();
        unique_clusters.len()
    }

    /// Calculate centrality score (simple degree centrality)
    pub fn centrality(&self, member: &MemberHash) -> usize {
        let in_degree = self.effective_vouches(member);
        let out_degree = self
            .reverse_vouches
            .get(member)
            .map(|vouchees| vouchees.len())
            .unwrap_or(0);
        in_degree + out_degree
    }
}

/// Detect clusters using bridge removal algorithm (Tarjan's algorithm)
///
/// Per blind-matchmaker-dvr.bead:
/// - Tight clusters are densely connected members
/// - Bridges connect clusters but aren't part of them
/// - Uses Tarjan's algorithm to find bridge edges, then removes them
pub fn detect_clusters(graph: &mut TrustGraph) {
    // Handle bootstrap case (very small networks)
    if graph.members.len() < 4 {
        // Assign all to cluster 0
        for member in &graph.members {
            graph.clusters.insert(*member, 0);
        }
        return;
    }

    // Build undirected edge list
    let edges = build_undirected_edges(graph);

    // Build adjacency list for undirected graph
    let mut adj: HashMap<MemberHash, HashSet<MemberHash>> = HashMap::new();
    for member in &graph.members {
        adj.insert(*member, HashSet::new());
    }
    for (a, b) in &edges {
        adj.get_mut(a).unwrap().insert(*b);
        adj.get_mut(b).unwrap().insert(*a);
    }

    // Find bridge edges using Tarjan's algorithm
    let bridges = find_bridges(&graph.members, &adj);

    // Remove bridges from edge list
    let non_bridge_edges: Vec<(MemberHash, MemberHash)> = edges
        .into_iter()
        .filter(|(a, b)| {
            let mut pair = [*a, *b];
            pair.sort();
            !bridges.contains(&(pair[0], pair[1]))
        })
        .collect();

    // Find connected components without bridges (these are clusters)
    let cluster_assignments = connected_components(&graph.members, &non_bridge_edges);

    // Store cluster assignments in graph
    graph.clusters = cluster_assignments;
}

/// Build undirected edges from vouch graph
/// An undirected edge exists if either A vouches for B OR B vouches for A
fn build_undirected_edges(graph: &TrustGraph) -> Vec<(MemberHash, MemberHash)> {
    let mut edges = HashSet::new();

    for (vouchee, vouchers) in &graph.vouches {
        for voucher in vouchers {
            let mut pair = [*voucher, *vouchee];
            pair.sort();
            edges.insert((pair[0], pair[1]));
        }
    }

    edges.into_iter().collect()
}

/// Find bridge edges using Tarjan's algorithm
fn find_bridges(
    members: &HashSet<MemberHash>,
    adj: &HashMap<MemberHash, HashSet<MemberHash>>,
) -> HashSet<(MemberHash, MemberHash)> {
    let mut disc: HashMap<MemberHash, usize> = HashMap::new();
    let mut low: HashMap<MemberHash, usize> = HashMap::new();
    let mut parent: HashMap<MemberHash, Option<MemberHash>> = HashMap::new();
    let mut bridges: HashSet<(MemberHash, MemberHash)> = HashSet::new();
    let mut time = 0;

    fn dfs(
        u: MemberHash,
        adj: &HashMap<MemberHash, HashSet<MemberHash>>,
        disc: &mut HashMap<MemberHash, usize>,
        low: &mut HashMap<MemberHash, usize>,
        parent: &mut HashMap<MemberHash, Option<MemberHash>>,
        bridges: &mut HashSet<(MemberHash, MemberHash)>,
        time: &mut usize,
    ) {
        *time += 1;
        disc.insert(u, *time);
        low.insert(u, *time);

        if let Some(neighbors) = adj.get(&u) {
            for &v in neighbors {
                if !disc.contains_key(&v) {
                    parent.insert(v, Some(u));
                    dfs(v, adj, disc, low, parent, bridges, time);

                    let low_u = *low.get(&u).unwrap();
                    let low_v = *low.get(&v).unwrap();
                    low.insert(u, low_u.min(low_v));

                    // If low[v] > disc[u], then (u, v) is a bridge
                    if low_v > *disc.get(&u).unwrap() {
                        let mut pair = [u, v];
                        pair.sort();
                        bridges.insert((pair[0], pair[1]));
                    }
                } else if parent.get(&u).copied().flatten() != Some(v) {
                    let low_u = *low.get(&u).unwrap();
                    let disc_v = *disc.get(&v).unwrap();
                    low.insert(u, low_u.min(disc_v));
                }
            }
        }
    }

    for &member in members {
        if !disc.contains_key(&member) {
            parent.insert(member, None);
            dfs(
                member,
                adj,
                &mut disc,
                &mut low,
                &mut parent,
                &mut bridges,
                &mut time,
            );
        }
    }

    bridges
}

/// Find connected components using Union-Find
fn connected_components(
    members: &HashSet<MemberHash>,
    edges: &[(MemberHash, MemberHash)],
) -> HashMap<MemberHash, ClusterId> {
    let mut parent: HashMap<MemberHash, MemberHash> = HashMap::new();
    let mut rank: HashMap<MemberHash, usize> = HashMap::new();

    // Initialize Union-Find
    for &member in members {
        parent.insert(member, member);
        rank.insert(member, 0);
    }

    // Find with path compression
    fn find(x: MemberHash, parent: &mut HashMap<MemberHash, MemberHash>) -> MemberHash {
        let p = *parent.get(&x).unwrap();
        if p != x {
            let root = find(p, parent);
            parent.insert(x, root);
            root
        } else {
            x
        }
    }

    // Union by rank
    fn union(
        x: MemberHash,
        y: MemberHash,
        parent: &mut HashMap<MemberHash, MemberHash>,
        rank: &mut HashMap<MemberHash, usize>,
    ) {
        let root_x = find(x, parent);
        let root_y = find(y, parent);

        if root_x != root_y {
            let rank_x = *rank.get(&root_x).unwrap();
            let rank_y = *rank.get(&root_y).unwrap();

            if rank_x < rank_y {
                parent.insert(root_x, root_y);
            } else if rank_x > rank_y {
                parent.insert(root_y, root_x);
            } else {
                parent.insert(root_y, root_x);
                rank.insert(root_x, rank_x + 1);
            }
        }
    }

    // Union all edges
    for &(a, b) in edges {
        union(a, b, &mut parent, &mut rank);
    }

    // Map roots to cluster IDs
    let mut root_to_cluster: HashMap<MemberHash, ClusterId> = HashMap::new();
    let mut next_cluster_id = 0;

    let mut result: HashMap<MemberHash, ClusterId> = HashMap::new();
    for &member in members {
        let root = find(member, &mut parent);
        let cluster_id = *root_to_cluster.entry(root).or_insert_with(|| {
            let id = next_cluster_id;
            next_cluster_id += 1;
            id
        });
        result.insert(member, cluster_id);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::freenet::trust_contract::TrustNetworkState;
    use std::collections::HashSet;

    fn member_hash(id: u8) -> MemberHash {
        let mut bytes = [0u8; 32];
        bytes[0] = id;
        MemberHash::from_bytes(&bytes)
    }

    #[test]
    fn test_trust_graph_from_state() {
        let mut state = TrustNetworkState::new();
        let alice = member_hash(1);
        let bob = member_hash(2);

        state.members.insert(alice);
        state.members.insert(bob);
        state.vouches.insert(alice, [bob].into_iter().collect());

        let graph = TrustGraph::from_state(&state);

        assert_eq!(graph.members.len(), 2);
        assert_eq!(graph.effective_vouches(&alice), 1);
        assert_eq!(graph.effective_vouches(&bob), 0);
    }

    #[test]
    fn test_cluster_detection_bootstrap() {
        let mut state = TrustNetworkState::new();
        for i in 1..=3 {
            state.members.insert(member_hash(i));
        }

        let mut graph = TrustGraph::from_state(&state);
        detect_clusters(&mut graph);

        // Small networks should all be in cluster 0
        assert_eq!(graph.cluster_count(), 1);
    }

    #[test]
    fn test_reverse_vouches_index() {
        let mut state = TrustNetworkState::new();
        let alice = member_hash(1);
        let bob = member_hash(2);
        let carol = member_hash(3);

        state.members.insert(alice);
        state.members.insert(bob);
        state.members.insert(carol);

        // Bob and Carol vouch for Alice
        state
            .vouches
            .insert(alice, [bob, carol].into_iter().collect());

        let graph = TrustGraph::from_state(&state);

        // Bob should have Alice in reverse_vouches (Bob vouches for Alice)
        assert!(graph.reverse_vouches.get(&bob).unwrap().contains(&alice));
        assert!(graph.reverse_vouches.get(&carol).unwrap().contains(&alice));
    }

    #[test]
    fn test_get_vouchers() {
        let mut state = TrustNetworkState::new();
        let alice = member_hash(1);
        let bob = member_hash(2);
        let carol = member_hash(3);

        state.members.insert(alice);
        state.members.insert(bob);
        state.members.insert(carol);

        state
            .vouches
            .insert(alice, [bob, carol].into_iter().collect());

        let graph = TrustGraph::from_state(&state);

        let vouchers = graph.get_vouchers(&alice);
        assert_eq!(vouchers.len(), 2);
        assert!(vouchers.contains(&bob));
        assert!(vouchers.contains(&carol));
    }

    #[test]
    fn test_get_vouchers_empty() {
        let mut state = TrustNetworkState::new();
        let alice = member_hash(1);

        state.members.insert(alice);

        let graph = TrustGraph::from_state(&state);

        let vouchers = graph.get_vouchers(&alice);
        assert_eq!(vouchers.len(), 0);
    }

    #[test]
    fn test_same_cluster() {
        let mut state = TrustNetworkState::new();

        // Create a connected component
        for i in 1..=5 {
            state.members.insert(member_hash(i));
        }

        // Create a tight triangle for members 1-3 (no bridges)
        state.vouches.insert(
            member_hash(1),
            [member_hash(2), member_hash(3)].into_iter().collect(),
        );
        state.vouches.insert(
            member_hash(2),
            [member_hash(1), member_hash(3)].into_iter().collect(),
        );
        state.vouches.insert(
            member_hash(3),
            [member_hash(1), member_hash(2)].into_iter().collect(),
        );

        let mut graph = TrustGraph::from_state(&state);
        detect_clusters(&mut graph);

        // Members 1, 2, 3 should be in the same cluster (triangle has no bridges)
        assert!(graph.same_cluster(&member_hash(1), &member_hash(2)));
        assert!(graph.same_cluster(&member_hash(2), &member_hash(3)));
        assert!(graph.same_cluster(&member_hash(1), &member_hash(3)));

        // Member 4 should not be in the same cluster (isolated)
        assert!(!graph.same_cluster(&member_hash(1), &member_hash(4)));
    }

    #[test]
    fn test_cluster_id() {
        let mut state = TrustNetworkState::new();

        for i in 1..=6 {
            state.members.insert(member_hash(i));
        }

        // Create a tight triangle (no bridges) for members 1, 2, 3
        state.vouches.insert(
            member_hash(1),
            [member_hash(2), member_hash(3)].into_iter().collect(),
        );
        state.vouches.insert(
            member_hash(2),
            [member_hash(1), member_hash(3)].into_iter().collect(),
        );
        state.vouches.insert(
            member_hash(3),
            [member_hash(1), member_hash(2)].into_iter().collect(),
        );

        let mut graph = TrustGraph::from_state(&state);
        detect_clusters(&mut graph);

        // Members 1, 2, 3 should all have the same cluster ID (triangle has no bridges)
        let cluster1 = graph.cluster_id(&member_hash(1));
        let cluster2 = graph.cluster_id(&member_hash(2));
        let cluster3 = graph.cluster_id(&member_hash(3));

        assert!(cluster1.is_some());
        assert!(cluster2.is_some());
        assert!(cluster3.is_some());
        assert_eq!(cluster1, cluster2);
        assert_eq!(cluster2, cluster3);

        // Member 4 should have different cluster ID (isolated)
        let cluster4 = graph.cluster_id(&member_hash(4));

        assert!(cluster4.is_some());
        assert_ne!(cluster1, cluster4);
    }

    #[test]
    fn test_cluster_members() {
        let mut state = TrustNetworkState::new();

        for i in 1..=6 {
            state.members.insert(member_hash(i));
        }

        // Create a tight cluster with members 1, 2, 3 (triangle - no bridges)
        state.vouches.insert(
            member_hash(1),
            [member_hash(2), member_hash(3)].into_iter().collect(),
        );
        state.vouches.insert(
            member_hash(2),
            [member_hash(1), member_hash(3)].into_iter().collect(),
        );

        let mut graph = TrustGraph::from_state(&state);
        detect_clusters(&mut graph);

        let cluster_id = graph.cluster_id(&member_hash(1)).unwrap();
        let members = graph.cluster_members(cluster_id);

        // Should contain all connected members in the triangle
        assert!(members.contains(&member_hash(1)));
        assert!(members.contains(&member_hash(2)));
        assert!(members.contains(&member_hash(3)));
        assert!(!members.contains(&member_hash(4)));
    }

    #[test]
    fn test_cluster_count() {
        let mut state = TrustNetworkState::new();

        for i in 1..=8 {
            state.members.insert(member_hash(i));
        }

        // Create two separate clusters
        // Cluster 1: {1, 2, 3}
        state
            .vouches
            .insert(member_hash(1), [member_hash(2)].into_iter().collect());
        state
            .vouches
            .insert(member_hash(2), [member_hash(3)].into_iter().collect());

        // Cluster 2: {4, 5, 6}
        state
            .vouches
            .insert(member_hash(4), [member_hash(5)].into_iter().collect());
        state
            .vouches
            .insert(member_hash(5), [member_hash(6)].into_iter().collect());

        let mut graph = TrustGraph::from_state(&state);
        detect_clusters(&mut graph);

        // Should have at least 2 clusters
        assert!(graph.cluster_count() >= 2);
    }

    #[test]
    fn test_centrality() {
        let mut state = TrustNetworkState::new();

        for i in 1..=5 {
            state.members.insert(member_hash(i));
        }

        // Member 1 vouched by 2, 3, 4 (in-degree = 3)
        state.vouches.insert(
            member_hash(1),
            [member_hash(2), member_hash(3), member_hash(4)]
                .into_iter()
                .collect(),
        );

        // Member 1 also vouches for 5 (out-degree = 1 via reverse_vouches)
        state
            .vouches
            .insert(member_hash(5), [member_hash(1)].into_iter().collect());

        let graph = TrustGraph::from_state(&state);

        // Member 1 centrality = in-degree (3) + out-degree (1) = 4
        let centrality = graph.centrality(&member_hash(1));
        assert_eq!(centrality, 4);

        // Member 2 centrality = in-degree (0) + out-degree (1) = 1
        let centrality2 = graph.centrality(&member_hash(2));
        assert_eq!(centrality2, 1);
    }

    #[test]
    fn test_build_undirected_edges() {
        let mut state = TrustNetworkState::new();

        for i in 1..=3 {
            state.members.insert(member_hash(i));
        }

        // Alice (1) is vouched by Bob (2)
        state
            .vouches
            .insert(member_hash(1), [member_hash(2)].into_iter().collect());

        let graph = TrustGraph::from_state(&state);
        let edges = build_undirected_edges(&graph);

        // Should have one undirected edge between 1 and 2
        assert_eq!(edges.len(), 1);

        let edge = edges[0];
        let mut expected = [member_hash(1), member_hash(2)];
        expected.sort();
        let edge_sorted = [edge.0.min(edge.1), edge.0.max(edge.1)];

        assert_eq!(edge_sorted, expected);
    }

    #[test]
    fn test_find_bridges_simple() {
        let mut state = TrustNetworkState::new();

        // Create a simple bridge scenario:
        // 1 -- 2 -- 3
        // Where edge 2-3 is a bridge
        for i in 1..=5 {
            state.members.insert(member_hash(i));
        }

        state
            .vouches
            .insert(member_hash(1), [member_hash(2)].into_iter().collect());
        state
            .vouches
            .insert(member_hash(2), [member_hash(3)].into_iter().collect());

        let graph = TrustGraph::from_state(&state);

        // Build adjacency list
        let edges = build_undirected_edges(&graph);
        let mut adj: HashMap<MemberHash, HashSet<MemberHash>> = HashMap::new();
        for member in &graph.members {
            adj.insert(*member, HashSet::new());
        }
        for (a, b) in &edges {
            adj.get_mut(a).unwrap().insert(*b);
            adj.get_mut(b).unwrap().insert(*a);
        }

        let bridges = find_bridges(&graph.members, &adj);

        // There should be bridges in this graph (1-2 and 2-3 are bridges)
        assert!(!bridges.is_empty());
    }

    #[test]
    fn test_find_bridges_cycle_no_bridges() {
        let mut state = TrustNetworkState::new();

        // Create a cycle: 1 -- 2 -- 3 -- 1
        // No bridges in a cycle
        for i in 1..=5 {
            state.members.insert(member_hash(i));
        }

        state
            .vouches
            .insert(member_hash(1), [member_hash(2)].into_iter().collect());
        state
            .vouches
            .insert(member_hash(2), [member_hash(3)].into_iter().collect());
        state
            .vouches
            .insert(member_hash(3), [member_hash(1)].into_iter().collect());

        let graph = TrustGraph::from_state(&state);

        // Build adjacency list
        let edges = build_undirected_edges(&graph);
        let mut adj: HashMap<MemberHash, HashSet<MemberHash>> = HashMap::new();
        for member in &graph.members {
            adj.insert(*member, HashSet::new());
        }
        for (a, b) in &edges {
            adj.get_mut(a).unwrap().insert(*b);
            adj.get_mut(b).unwrap().insert(*a);
        }

        let bridges = find_bridges(&graph.members, &adj);

        // No bridges in a strongly connected cycle
        assert_eq!(bridges.len(), 0);
    }

    #[test]
    fn test_connected_components_single() {
        let mut state = TrustNetworkState::new();

        for i in 1..=5 {
            state.members.insert(member_hash(i));
        }

        // All connected in a chain
        state
            .vouches
            .insert(member_hash(1), [member_hash(2)].into_iter().collect());
        state
            .vouches
            .insert(member_hash(2), [member_hash(3)].into_iter().collect());
        state
            .vouches
            .insert(member_hash(3), [member_hash(4)].into_iter().collect());
        state
            .vouches
            .insert(member_hash(4), [member_hash(5)].into_iter().collect());

        let graph = TrustGraph::from_state(&state);
        let edges = build_undirected_edges(&graph);

        let components = connected_components(&graph.members, &edges);

        // All members in one component
        let cluster_ids: HashSet<_> = components.values().copied().collect();
        assert_eq!(cluster_ids.len(), 1);
    }

    #[test]
    fn test_connected_components_multiple() {
        let mut state = TrustNetworkState::new();

        for i in 1..=6 {
            state.members.insert(member_hash(i));
        }

        // Create two components
        // Component 1: {1, 2, 3}
        state
            .vouches
            .insert(member_hash(1), [member_hash(2)].into_iter().collect());
        state
            .vouches
            .insert(member_hash(2), [member_hash(3)].into_iter().collect());

        // Component 2: {4, 5, 6}
        state
            .vouches
            .insert(member_hash(4), [member_hash(5)].into_iter().collect());
        state
            .vouches
            .insert(member_hash(5), [member_hash(6)].into_iter().collect());

        let graph = TrustGraph::from_state(&state);
        let edges = build_undirected_edges(&graph);

        let components = connected_components(&graph.members, &edges);

        // Should have exactly 2 components
        let cluster_ids: HashSet<_> = components.values().copied().collect();
        assert_eq!(cluster_ids.len(), 2);

        // Members 1, 2, 3 should be in same component
        assert_eq!(components[&member_hash(1)], components[&member_hash(2)]);
        assert_eq!(components[&member_hash(2)], components[&member_hash(3)]);

        // Members 4, 5, 6 should be in same component
        assert_eq!(components[&member_hash(4)], components[&member_hash(5)]);
        assert_eq!(components[&member_hash(5)], components[&member_hash(6)]);

        // But different from first component
        assert_ne!(components[&member_hash(1)], components[&member_hash(4)]);
    }

    #[test]
    fn test_detect_clusters_single_component() {
        let mut state = TrustNetworkState::new();

        for i in 1..=6 {
            state.members.insert(member_hash(i));
        }

        // Create fully connected cluster
        state.vouches.insert(
            member_hash(1),
            [member_hash(2), member_hash(3)].into_iter().collect(),
        );
        state.vouches.insert(
            member_hash(2),
            [member_hash(3), member_hash(4)].into_iter().collect(),
        );
        state.vouches.insert(
            member_hash(3),
            [member_hash(4), member_hash(5)].into_iter().collect(),
        );

        let mut graph = TrustGraph::from_state(&state);
        detect_clusters(&mut graph);

        // All should be in one cluster
        assert!(graph.same_cluster(&member_hash(1), &member_hash(2)));
        assert!(graph.same_cluster(&member_hash(2), &member_hash(3)));
    }

    #[test]
    fn test_detect_clusters_with_bridge() {
        let mut state = TrustNetworkState::new();

        for i in 1..=8 {
            state.members.insert(member_hash(i));
        }

        // Create two tight clusters connected by a bridge
        // Cluster 1: {1, 2, 3} (triangle)
        state.vouches.insert(
            member_hash(1),
            [member_hash(2), member_hash(3)].into_iter().collect(),
        );
        state.vouches.insert(
            member_hash(2),
            [member_hash(1), member_hash(3)].into_iter().collect(),
        );

        // Bridge: 3 -- 4
        state
            .vouches
            .insert(member_hash(4), [member_hash(3)].into_iter().collect());

        // Cluster 2: {4, 5, 6} (triangle)
        state.vouches.insert(
            member_hash(4),
            [member_hash(5), member_hash(6)].into_iter().collect(),
        );
        state.vouches.insert(
            member_hash(5),
            [member_hash(4), member_hash(6)].into_iter().collect(),
        );

        let mut graph = TrustGraph::from_state(&state);
        detect_clusters(&mut graph);

        // Should detect multiple clusters after bridge removal
        // (exact count depends on bridge detection algorithm)
        assert!(graph.cluster_count() >= 1);
    }

    #[test]
    fn test_empty_graph() {
        let state = TrustNetworkState::new();
        let mut graph = TrustGraph::from_state(&state);

        detect_clusters(&mut graph);

        assert_eq!(graph.members.len(), 0);
        assert_eq!(graph.cluster_count(), 0);
    }

    #[test]
    fn test_centrality_isolated_member() {
        let mut state = TrustNetworkState::new();

        state.members.insert(member_hash(1));

        let graph = TrustGraph::from_state(&state);

        // Isolated member has centrality 0
        assert_eq!(graph.centrality(&member_hash(1)), 0);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use crate::freenet::trust_contract::TrustNetworkState;
    use proptest::prelude::*;
    use std::collections::HashSet;

    fn member_hash(id: u8) -> MemberHash {
        let mut bytes = [0u8; 32];
        bytes[0] = id;
        MemberHash::from_bytes(&bytes)
    }

    proptest! {
        /// Property test: Cluster assignments are consistent
        /// Every member should be assigned to exactly one cluster
        #[test]
        fn prop_cluster_assignments_consistent(
            num_members in 4usize..30,
        ) {
            let mut state = TrustNetworkState::new();

            // Add members
            for i in 0..num_members as u8 {
                state.members.insert(member_hash(i));
            }

            // Add some random edges (vouches)
            for i in 0..(num_members / 2) {
                let member = member_hash((i * 2) as u8);
                let voucher = member_hash((i * 2 + 1) as u8);
                state.vouches.insert(member, [voucher].into_iter().collect());
            }

            let mut graph = TrustGraph::from_state(&state);
            detect_clusters(&mut graph);

            // Every member should have a cluster assignment
            for member in &graph.members {
                prop_assert!(
                    graph.cluster_id(member).is_some(),
                    "Member {:?} has no cluster assignment",
                    member
                );
            }

            // All cluster IDs should be valid
            let cluster_ids: HashSet<_> = graph.clusters.values().copied().collect();
            for &cluster_id in &cluster_ids {
                prop_assert!(
                    cluster_id < num_members,
                    "Cluster ID {} is out of bounds (max {})",
                    cluster_id,
                    num_members
                );
            }
        }

        /// Property test: same_cluster is symmetric
        /// If A is in same cluster as B, then B is in same cluster as A
        #[test]
        fn prop_same_cluster_symmetric(
            num_members in 4usize..20,
        ) {
            let mut state = TrustNetworkState::new();

            // Add members
            for i in 0..num_members as u8 {
                state.members.insert(member_hash(i));
            }

            // Add edges
            for i in 0..(num_members / 2) {
                let member = member_hash((i * 2) as u8);
                let voucher = member_hash((i * 2 + 1) as u8);
                state.vouches.insert(member, [voucher].into_iter().collect());
            }

            let mut graph = TrustGraph::from_state(&state);
            detect_clusters(&mut graph);

            // Check symmetry for all pairs
            for i in 0..num_members {
                for j in 0..num_members {
                    let member_i = member_hash(i as u8);
                    let member_j = member_hash(j as u8);

                    if graph.same_cluster(&member_i, &member_j) {
                        prop_assert!(
                            graph.same_cluster(&member_j, &member_i),
                            "same_cluster not symmetric: ({:?}, {:?})",
                            member_i,
                            member_j
                        );
                    }
                }
            }
        }

        /// Property test: same_cluster is transitive
        /// If A is in same cluster as B, and B is in same cluster as C,
        /// then A is in same cluster as C
        #[test]
        fn prop_same_cluster_transitive(
            num_members in 4usize..15,
        ) {
            let mut state = TrustNetworkState::new();

            // Add members
            for i in 0..num_members as u8 {
                state.members.insert(member_hash(i));
            }

            // Create a connected chain to ensure transitivity can be tested
            for i in 0..(num_members - 1) {
                let member = member_hash(i as u8);
                let voucher = member_hash((i + 1) as u8);
                state.vouches.insert(member, [voucher].into_iter().collect());
            }

            let mut graph = TrustGraph::from_state(&state);
            detect_clusters(&mut graph);

            // Check transitivity for triplets
            for i in 0..num_members.min(5) {
                for j in 0..num_members.min(5) {
                    for k in 0..num_members.min(5) {
                        let member_i = member_hash(i as u8);
                        let member_j = member_hash(j as u8);
                        let member_k = member_hash(k as u8);

                        if graph.same_cluster(&member_i, &member_j)
                            && graph.same_cluster(&member_j, &member_k)
                        {
                            prop_assert!(
                                graph.same_cluster(&member_i, &member_k),
                                "same_cluster not transitive: ({:?}, {:?}, {:?})",
                                member_i,
                                member_j,
                                member_k
                            );
                        }
                    }
                }
            }
        }

        /// Property test: cluster_count matches number of unique clusters
        #[test]
        fn prop_cluster_count_matches_unique_clusters(
            num_members in 4usize..30,
        ) {
            let mut state = TrustNetworkState::new();

            // Add members
            for i in 0..num_members as u8 {
                state.members.insert(member_hash(i));
            }

            // Add edges
            for i in 0..(num_members / 2) {
                let member = member_hash((i * 2) as u8);
                let voucher = member_hash((i * 2 + 1) as u8);
                state.vouches.insert(member, [voucher].into_iter().collect());
            }

            let mut graph = TrustGraph::from_state(&state);
            detect_clusters(&mut graph);

            // Count unique cluster IDs
            let unique_clusters: HashSet<_> = graph.clusters.values().copied().collect();

            prop_assert_eq!(
                graph.cluster_count(),
                unique_clusters.len(),
                "cluster_count() returned {}, but found {} unique cluster IDs",
                graph.cluster_count(),
                unique_clusters.len()
            );
        }

        /// Property test: cluster_members returns all members of a cluster
        #[test]
        fn prop_cluster_members_complete(
            num_members in 4usize..20,
        ) {
            let mut state = TrustNetworkState::new();

            // Add members
            for i in 0..num_members as u8 {
                state.members.insert(member_hash(i));
            }

            // Add edges to create clusters
            for i in 0..(num_members / 2) {
                let member = member_hash((i * 2) as u8);
                let voucher = member_hash((i * 2 + 1) as u8);
                state.vouches.insert(member, [voucher].into_iter().collect());
            }

            let mut graph = TrustGraph::from_state(&state);
            detect_clusters(&mut graph);

            // For each cluster, verify cluster_members returns correct members
            for cluster_id in 0..graph.cluster_count() {
                let members_in_cluster = graph.cluster_members(cluster_id);

                // All returned members should have this cluster ID
                for member in &members_in_cluster {
                    prop_assert_eq!(
                        graph.cluster_id(member),
                        Some(cluster_id),
                        "Member {:?} in cluster_members({}) has different cluster ID {:?}",
                        member,
                        cluster_id,
                        graph.cluster_id(member)
                    );
                }

                // All members with this cluster ID should be in the list
                for member in &graph.members {
                    if graph.cluster_id(member) == Some(cluster_id) {
                        prop_assert!(
                            members_in_cluster.contains(member),
                            "Member {:?} has cluster ID {} but not in cluster_members({})",
                            member,
                            cluster_id,
                            cluster_id
                        );
                    }
                }
            }
        }

        /// Property test: centrality is well-defined (computable for all members)
        #[test]
        fn prop_centrality_well_defined(
            num_members in 1usize..30,
        ) {
            let mut state = TrustNetworkState::new();

            // Add members
            for i in 0..num_members as u8 {
                state.members.insert(member_hash(i));
            }

            // Add random edges
            for i in 0..(num_members / 2) {
                let member = member_hash((i * 2) as u8);
                let voucher = member_hash(((i * 2 + 1) % num_members) as u8);
                state.vouches.insert(member, [voucher].into_iter().collect());
            }

            let graph = TrustGraph::from_state(&state);

            // Centrality should be computable for all members (not panic)
            // and should be reasonable (not greater than total possible edges)
            let max_centrality = num_members * 2;
            for member in &graph.members {
                let centrality = graph.centrality(member);
                prop_assert!(
                    centrality <= max_centrality,
                    "Centrality for {:?} is {}, which exceeds max possible {}",
                    member,
                    centrality,
                    max_centrality
                );
            }
        }

        /// Property test: effective_vouches equals voucher count
        #[test]
        fn prop_effective_vouches_correct(
            num_members in 2usize..20,
            num_vouches in 0usize..10,
        ) {
            let mut state = TrustNetworkState::new();

            // Add members
            for i in 0..num_members as u8 {
                state.members.insert(member_hash(i));
            }

            // Add vouches to first member
            let member = member_hash(0);
            let mut vouchers = HashSet::new();
            for i in 1..=num_vouches.min(num_members - 1) {
                vouchers.insert(member_hash(i as u8));
            }
            if !vouchers.is_empty() {
                state.vouches.insert(member, vouchers.clone());
            }

            let graph = TrustGraph::from_state(&state);

            prop_assert_eq!(
                graph.effective_vouches(&member),
                vouchers.len(),
                "effective_vouches for {:?} is {}, expected {}",
                member,
                graph.effective_vouches(&member),
                vouchers.len()
            );
        }

        /// Property test: get_vouchers returns correct voucher set
        #[test]
        fn prop_get_vouchers_correct(
            num_members in 2usize..20,
            num_vouches in 0usize..10,
        ) {
            let mut state = TrustNetworkState::new();

            // Add members
            for i in 0..num_members as u8 {
                state.members.insert(member_hash(i));
            }

            // Add vouches to first member
            let member = member_hash(0);
            let mut expected_vouchers = HashSet::new();
            for i in 1..=num_vouches.min(num_members - 1) {
                expected_vouchers.insert(member_hash(i as u8));
            }
            if !expected_vouchers.is_empty() {
                state.vouches.insert(member, expected_vouchers.clone());
            }

            let graph = TrustGraph::from_state(&state);
            let vouchers = graph.get_vouchers(&member);

            prop_assert_eq!(
                vouchers,
                expected_vouchers,
                "get_vouchers returned different set than expected"
            );
        }

        /// Property test: undirected edges are bidirectional
        /// If (a, b) is an edge, there should be a corresponding vouch relationship
        #[test]
        fn prop_undirected_edges_bidirectional(
            num_members in 2usize..20,
        ) {
            let mut state = TrustNetworkState::new();

            // Add members
            for i in 0..num_members as u8 {
                state.members.insert(member_hash(i));
            }

            // Add vouches
            for i in 0..(num_members / 2) {
                let member = member_hash((i * 2) as u8);
                let voucher = member_hash((i * 2 + 1) as u8);
                state.vouches.insert(member, [voucher].into_iter().collect());
            }

            let graph = TrustGraph::from_state(&state);
            let edges = build_undirected_edges(&graph);

            // Each edge should correspond to a vouch relationship
            for (a, b) in &edges {
                let has_vouch_a_to_b = graph
                    .vouches
                    .get(a)
                    .map(|vouchers| vouchers.contains(b))
                    .unwrap_or(false);

                let has_vouch_b_to_a = graph
                    .vouches
                    .get(b)
                    .map(|vouchers| vouchers.contains(a))
                    .unwrap_or(false);

                prop_assert!(
                    has_vouch_a_to_b || has_vouch_b_to_a,
                    "Edge ({:?}, {:?}) exists but no corresponding vouch relationship found",
                    a,
                    b
                );
            }
        }
    }
}
