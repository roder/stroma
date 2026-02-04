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
}
