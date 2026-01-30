//! Cluster Detection Algorithms for Stroma
//!
//! This module implements various approaches to detecting tight clusters
//! in a vouch graph, specifically to distinguish clusters connected by bridges.

use std::collections::{HashMap, HashSet};

/// Represents a directed vouch graph
#[derive(Debug, Clone)]
pub struct VouchGraph {
    /// Adjacency list: member -> set of members they vouch for
    pub vouches: HashMap<String, HashSet<String>>,
    /// All members in the graph
    pub members: HashSet<String>,
}

impl VouchGraph {
    pub fn new() -> Self {
        Self {
            vouches: HashMap::new(),
            members: HashSet::new(),
        }
    }

    /// Add a member to the graph
    pub fn add_member(&mut self, member: &str) {
        self.members.insert(member.to_string());
        self.vouches.entry(member.to_string()).or_default();
    }

    /// Add a vouch: voucher vouches for vouchee
    pub fn add_vouch(&mut self, voucher: &str, vouchee: &str) {
        self.add_member(voucher);
        self.add_member(vouchee);
        self.vouches
            .entry(voucher.to_string())
            .or_default()
            .insert(vouchee.to_string());
    }

    /// Get undirected edges (for Union-Find and community detection)
    /// An undirected edge exists if either A vouches for B OR B vouches for A
    pub fn undirected_edges(&self) -> Vec<(String, String)> {
        let mut edges = HashSet::new();
        for (voucher, vouchees) in &self.vouches {
            for vouchee in vouchees {
                let mut pair = vec![voucher.clone(), vouchee.clone()];
                pair.sort();
                edges.insert((pair[0].clone(), pair[1].clone()));
            }
        }
        edges.into_iter().collect()
    }

    /// Get mutual vouches (bidirectional edges)
    /// A mutual vouch exists if A vouches for B AND B vouches for A
    pub fn mutual_edges(&self) -> Vec<(String, String)> {
        let mut edges = HashSet::new();
        for (voucher, vouchees) in &self.vouches {
            for vouchee in vouchees {
                // Check if the reverse vouch exists
                if let Some(reverse_vouchees) = self.vouches.get(vouchee) {
                    if reverse_vouchees.contains(voucher) {
                        let mut pair = vec![voucher.clone(), vouchee.clone()];
                        pair.sort();
                        edges.insert((pair[0].clone(), pair[1].clone()));
                    }
                }
            }
        }
        edges.into_iter().collect()
    }

    /// Count vouches received by a member
    pub fn vouch_count(&self, member: &str) -> usize {
        self.vouches
            .values()
            .filter(|vouchees| vouchees.contains(member))
            .count()
    }
}

// =============================================================================
// Algorithm 1: Standard Union-Find (Connected Components)
// =============================================================================

/// Union-Find data structure with path compression and union by rank
pub struct UnionFind {
    parent: HashMap<String, String>,
    rank: HashMap<String, usize>,
}

impl UnionFind {
    pub fn new(members: &HashSet<String>) -> Self {
        let parent: HashMap<String, String> =
            members.iter().map(|m| (m.clone(), m.clone())).collect();
        let rank: HashMap<String, usize> = members.iter().map(|m| (m.clone(), 0)).collect();
        Self { parent, rank }
    }

    /// Find with path compression
    pub fn find(&mut self, x: &str) -> String {
        let parent = self.parent.get(x).cloned().unwrap_or_else(|| x.to_string());
        if parent != x {
            let root = self.find(&parent);
            self.parent.insert(x.to_string(), root.clone());
            root
        } else {
            x.to_string()
        }
    }

    /// Union by rank
    pub fn union(&mut self, x: &str, y: &str) {
        let root_x = self.find(x);
        let root_y = self.find(y);

        if root_x != root_y {
            let rank_x = *self.rank.get(&root_x).unwrap_or(&0);
            let rank_y = *self.rank.get(&root_y).unwrap_or(&0);

            if rank_x < rank_y {
                self.parent.insert(root_x, root_y);
            } else if rank_x > rank_y {
                self.parent.insert(root_y, root_x);
            } else {
                self.parent.insert(root_y, root_x.clone());
                self.rank.insert(root_x, rank_x + 1);
            }
        }
    }

    /// Get all clusters
    pub fn clusters(&mut self) -> HashMap<String, Vec<String>> {
        let mut result: HashMap<String, Vec<String>> = HashMap::new();
        let members: Vec<String> = self.parent.keys().cloned().collect();
        for member in members {
            let root = self.find(&member);
            result.entry(root).or_default().push(member);
        }
        result
    }
}

/// Standard Union-Find clustering - finds connected components
pub fn cluster_union_find(graph: &VouchGraph) -> HashMap<String, Vec<String>> {
    let mut uf = UnionFind::new(&graph.members);

    // Union all connected members (using undirected edges)
    for (a, b) in graph.undirected_edges() {
        uf.union(&a, &b);
    }

    uf.clusters()
}

// =============================================================================
// Algorithm 2: Edge Density Analysis
// =============================================================================

/// Calculate edge density within a set of nodes
/// Density = actual_edges / possible_edges
/// For n nodes, possible_edges = n * (n - 1) / 2 (undirected)
fn edge_density(members: &[String], edges: &[(String, String)]) -> f64 {
    let n = members.len();
    if n < 2 {
        return 1.0; // Single node is maximally dense
    }

    let member_set: HashSet<&String> = members.iter().collect();

    let internal_edges = edges
        .iter()
        .filter(|(a, b)| member_set.contains(a) && member_set.contains(b))
        .count();

    let possible_edges = n * (n - 1) / 2;
    internal_edges as f64 / possible_edges as f64
}

/// Detect clusters using edge density threshold
/// A "tight" cluster has density >= threshold
pub fn cluster_by_density(
    graph: &VouchGraph,
    density_threshold: f64,
) -> HashMap<String, Vec<String>> {
    // Start with Union-Find clusters
    let initial_clusters = cluster_union_find(graph);

    // For each cluster, check if it should be split based on density
    let edges = graph.undirected_edges();
    let mut final_clusters: HashMap<String, Vec<String>> = HashMap::new();
    let mut cluster_id = 0;

    for (_root, members) in initial_clusters {
        let density = edge_density(&members, &edges);

        if density >= density_threshold {
            // Cluster is tight enough, keep as-is
            final_clusters.insert(format!("cluster_{}", cluster_id), members);
            cluster_id += 1;
        } else {
            // Cluster is sparse - try to find tight sub-clusters
            // Use a simple greedy approach: grow clusters from high-degree nodes
            let mut assigned: HashSet<String> = HashSet::new();

            for member in &members {
                if assigned.contains(member) {
                    continue;
                }

                // Start a new sub-cluster from this member
                let mut sub_cluster = vec![member.clone()];
                assigned.insert(member.clone());

                // Add neighbors that maintain high density
                loop {
                    let mut best_candidate: Option<String> = None;
                    let mut best_density = density_threshold;

                    for candidate in &members {
                        if assigned.contains(candidate) {
                            continue;
                        }

                        // Check if adding this candidate maintains density
                        let mut test_cluster = sub_cluster.clone();
                        test_cluster.push(candidate.clone());
                        let new_density = edge_density(&test_cluster, &edges);

                        if new_density >= best_density {
                            best_density = new_density;
                            best_candidate = Some(candidate.clone());
                        }
                    }

                    if let Some(candidate) = best_candidate {
                        sub_cluster.push(candidate.clone());
                        assigned.insert(candidate);
                    } else {
                        break;
                    }
                }

                if !sub_cluster.is_empty() {
                    final_clusters.insert(format!("cluster_{}", cluster_id), sub_cluster);
                    cluster_id += 1;
                }
            }

            // Handle any unassigned members as singletons
            for member in &members {
                if !assigned.contains(member) {
                    final_clusters.insert(format!("cluster_{}", cluster_id), vec![member.clone()]);
                    cluster_id += 1;
                }
            }
        }
    }

    final_clusters
}

// =============================================================================
// Algorithm 3: Mutual Vouch Clustering
// =============================================================================

/// Cluster based on mutual vouches only
/// Two members are in the same cluster if they mutually vouch for each other
/// (directly or transitively through other mutual vouchers)
pub fn cluster_by_mutual_vouches(graph: &VouchGraph) -> HashMap<String, Vec<String>> {
    let mut uf = UnionFind::new(&graph.members);

    // Only union members with mutual vouches
    for (a, b) in graph.mutual_edges() {
        uf.union(&a, &b);
    }

    uf.clusters()
}

// =============================================================================
// Algorithm 4: Bridge Detection (Articulation Point Analysis)
// =============================================================================

/// Detect bridges (edges whose removal disconnects the graph)
/// Then cluster by removing bridges
pub fn cluster_by_bridge_removal(graph: &VouchGraph) -> HashMap<String, Vec<String>> {
    let edges = graph.undirected_edges();

    // Build adjacency list for undirected graph
    let mut adj: HashMap<String, HashSet<String>> = HashMap::new();
    for member in &graph.members {
        adj.insert(member.clone(), HashSet::new());
    }
    for (a, b) in &edges {
        adj.get_mut(a).unwrap().insert(b.clone());
        adj.get_mut(b).unwrap().insert(a.clone());
    }

    // Find bridges using Tarjan's algorithm
    let bridges = find_bridges(&graph.members, &adj);

    // Remove bridges and find connected components
    let non_bridge_edges: Vec<(String, String)> = edges
        .into_iter()
        .filter(|(a, b)| {
            let mut pair = vec![a.clone(), b.clone()];
            pair.sort();
            !bridges.contains(&(pair[0].clone(), pair[1].clone()))
        })
        .collect();

    // Union-Find on non-bridge edges
    let mut uf = UnionFind::new(&graph.members);
    for (a, b) in non_bridge_edges {
        uf.union(&a, &b);
    }

    uf.clusters()
}

/// Find bridges using Tarjan's algorithm
fn find_bridges(
    members: &HashSet<String>,
    adj: &HashMap<String, HashSet<String>>,
) -> HashSet<(String, String)> {
    let mut disc: HashMap<String, usize> = HashMap::new();
    let mut low: HashMap<String, usize> = HashMap::new();
    let mut parent: HashMap<String, Option<String>> = HashMap::new();
    let mut bridges: HashSet<(String, String)> = HashSet::new();
    let mut time = 0;

    fn dfs(
        u: &str,
        adj: &HashMap<String, HashSet<String>>,
        disc: &mut HashMap<String, usize>,
        low: &mut HashMap<String, usize>,
        parent: &mut HashMap<String, Option<String>>,
        bridges: &mut HashSet<(String, String)>,
        time: &mut usize,
    ) {
        *time += 1;
        disc.insert(u.to_string(), *time);
        low.insert(u.to_string(), *time);

        if let Some(neighbors) = adj.get(u) {
            for v in neighbors {
                if !disc.contains_key(v) {
                    parent.insert(v.clone(), Some(u.to_string()));
                    dfs(v, adj, disc, low, parent, bridges, time);

                    let low_u = *low.get(u).unwrap();
                    let low_v = *low.get(v).unwrap();
                    low.insert(u.to_string(), low_u.min(low_v));

                    // If low[v] > disc[u], then (u, v) is a bridge
                    if low_v > *disc.get(u).unwrap() {
                        let mut pair = vec![u.to_string(), v.clone()];
                        pair.sort();
                        bridges.insert((pair[0].clone(), pair[1].clone()));
                    }
                } else if parent.get(u).map(|p| p.as_ref()) != Some(Some(v)) {
                    let low_u = *low.get(u).unwrap();
                    let disc_v = *disc.get(v).unwrap();
                    low.insert(u.to_string(), low_u.min(disc_v));
                }
            }
        }
    }

    for member in members {
        if !disc.contains_key(member) {
            parent.insert(member.clone(), None);
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

// =============================================================================
// Test Helper: Build the "Bridge Problem" Graph
// =============================================================================

/// Build the test graph from the Bridge Problem scenario:
/// Cluster A: Alice, Bob, Carol (all vouch each other)
/// Bridge: Charlie (vouched by Carol + Dave)
/// Cluster B: Dave, Eve, Frank (all vouch each other)
pub fn build_bridge_problem_graph() -> VouchGraph {
    let mut graph = VouchGraph::new();

    // Cluster A: Alice, Bob, Carol (tight cluster - all vouch each other)
    graph.add_vouch("Alice", "Bob");
    graph.add_vouch("Alice", "Carol");
    graph.add_vouch("Bob", "Alice");
    graph.add_vouch("Bob", "Carol");
    graph.add_vouch("Carol", "Alice");
    graph.add_vouch("Carol", "Bob");

    // Cluster B: Dave, Eve, Frank (tight cluster - all vouch each other)
    graph.add_vouch("Dave", "Eve");
    graph.add_vouch("Dave", "Frank");
    graph.add_vouch("Eve", "Dave");
    graph.add_vouch("Eve", "Frank");
    graph.add_vouch("Frank", "Dave");
    graph.add_vouch("Frank", "Eve");

    // Bridge: Charlie (connected to both clusters but not tight with either)
    // Carol vouches for Charlie, Dave vouches for Charlie
    graph.add_vouch("Carol", "Charlie");
    graph.add_vouch("Dave", "Charlie");
    // Charlie vouches back (making them bidirectional connections)
    graph.add_vouch("Charlie", "Carol");
    graph.add_vouch("Charlie", "Dave");

    graph
}

/// Build a fully connected graph (should return 1 cluster)
pub fn build_fully_connected_graph() -> VouchGraph {
    let mut graph = VouchGraph::new();
    let members = vec!["A", "B", "C", "D", "E"];

    for i in 0..members.len() {
        for j in 0..members.len() {
            if i != j {
                graph.add_vouch(members[i], members[j]);
            }
        }
    }

    graph
}

/// Build isolated nodes (should return N clusters)
pub fn build_isolated_graph() -> VouchGraph {
    let mut graph = VouchGraph::new();
    graph.add_member("Isolated1");
    graph.add_member("Isolated2");
    graph.add_member("Isolated3");
    graph
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_union_find_bridge_problem() {
        let graph = build_bridge_problem_graph();
        let clusters = cluster_union_find(&graph);

        // Standard Union-Find should find 1 cluster (all connected)
        assert_eq!(
            clusters.len(),
            1,
            "Union-Find should find 1 connected component"
        );
    }

    #[test]
    fn test_mutual_vouch_bridge_problem() {
        let graph = build_bridge_problem_graph();
        let clusters = cluster_by_mutual_vouches(&graph);

        // Mutual vouch clustering should separate based on mutual vouches
        // Alice-Bob-Carol have mutual vouches
        // Dave-Eve-Frank have mutual vouches
        // Charlie has mutual vouches with Carol and Dave
        println!("Mutual vouch clusters: {:?}", clusters);

        // With bidirectional Charlie connections, this will still be 1 cluster
        // because mutual vouches form a connected graph
    }

    #[test]
    fn test_density_bridge_problem() {
        let graph = build_bridge_problem_graph();
        let clusters = cluster_by_density(&graph, 0.8);

        println!("Density-based clusters (threshold 0.8): {:?}", clusters);
    }

    #[test]
    fn test_bridge_removal() {
        let graph = build_bridge_problem_graph();
        let clusters = cluster_by_bridge_removal(&graph);

        println!("Bridge removal clusters: {:?}", clusters);
    }
}
