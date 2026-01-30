//! Q3 Spike: Cluster Detection Test Runner
//!
//! Tests whether various cluster detection algorithms can distinguish
//! tight clusters connected by bridges.

mod cluster;

use cluster::*;

fn main() {
    println!("{}", "=".repeat(70));
    println!("Q3 SPIKE: Cluster Detection - The Bridge Problem");
    println!("{}", "=".repeat(70));
    println!();

    // Build the test graph
    let graph = build_bridge_problem_graph();

    println!("TEST GRAPH: The Bridge Problem");
    println!("{}", "-".repeat(50));
    println!("Cluster A (tight): Alice, Bob, Carol (all vouch each other)");
    println!("Bridge:            Charlie (vouched by Carol + Dave, vouches back)");
    println!("Cluster B (tight): Dave, Eve, Frank (all vouch each other)");
    println!();
    println!("EXPECTED: 2 clusters (A and B are distinct social contexts)");
    println!("         Charlie is a bridge, not part of either tight cluster");
    println!();

    // Show graph statistics
    println!("Graph Statistics:");
    println!("  Members: {:?}", graph.members);
    println!("  Undirected edges: {}", graph.undirected_edges().len());
    println!("  Mutual edges: {}", graph.mutual_edges().len());
    println!();

    // Test Algorithm 1: Standard Union-Find
    println!("{}", "-".repeat(70));
    println!("ALGORITHM 1: Standard Union-Find (Connected Components)");
    println!("{}", "-".repeat(70));
    let clusters = cluster_union_find(&graph);
    print_clusters(&clusters);
    let result1 = clusters.len();
    println!(
        "RESULT: {} cluster(s) - {}",
        result1,
        if result1 == 1 {
            "EXPECTED (Union-Find sees all as connected)"
        } else {
            "UNEXPECTED"
        }
    );
    println!();

    // Test Algorithm 2: Mutual Vouch Clustering
    println!("{}", "-".repeat(70));
    println!("ALGORITHM 2: Mutual Vouch Clustering");
    println!("{}", "-".repeat(70));
    let clusters = cluster_by_mutual_vouches(&graph);
    print_clusters(&clusters);
    let result2 = clusters.len();
    println!(
        "RESULT: {} cluster(s) - {}",
        result2,
        if result2 >= 2 {
            "SUCCESS: Distinguishes tight clusters!"
        } else {
            "Still sees as 1 cluster (mutual vouches connected)"
        }
    );
    println!();

    // Test Algorithm 3: Edge Density Analysis (various thresholds)
    println!("{}", "-".repeat(70));
    println!("ALGORITHM 3: Edge Density Analysis");
    println!("{}", "-".repeat(70));

    for threshold in [0.5, 0.6, 0.7, 0.8, 0.9] {
        println!("\nDensity threshold: {:.1}", threshold);
        let clusters = cluster_by_density(&graph, threshold);
        print_clusters(&clusters);
        let result = clusters.len();
        println!(
            "  -> {} cluster(s){}",
            result,
            if result >= 2 {
                " - Separates clusters!"
            } else {
                ""
            }
        );
    }
    println!();

    // Test Algorithm 4: Bridge Removal
    println!("{}", "-".repeat(70));
    println!("ALGORITHM 4: Bridge Detection & Removal");
    println!("{}", "-".repeat(70));
    let clusters = cluster_by_bridge_removal(&graph);
    print_clusters(&clusters);
    let result4 = clusters.len();
    println!(
        "RESULT: {} cluster(s) - {}",
        result4,
        if result4 >= 2 {
            "SUCCESS: Bridge removal separates clusters!"
        } else {
            "Did not separate (no bridges detected)"
        }
    );
    println!();

    // Additional test: Fully connected graph
    println!("{}", "=".repeat(70));
    println!("CONTROL TEST: Fully Connected Graph (5 members)");
    println!("{}", "=".repeat(70));
    let fully_connected = build_fully_connected_graph();
    let fc_clusters = cluster_union_find(&fully_connected);
    print_clusters(&fc_clusters);
    println!(
        "EXPECTED: 1 cluster - {}",
        if fc_clusters.len() == 1 {
            "PASS"
        } else {
            "FAIL"
        }
    );
    println!();

    // Additional test: Isolated nodes
    println!("{}", "=".repeat(70));
    println!("CONTROL TEST: Isolated Nodes (3 members, no vouches)");
    println!("{}", "=".repeat(70));
    let isolated = build_isolated_graph();
    let iso_clusters = cluster_union_find(&isolated);
    print_clusters(&iso_clusters);
    println!(
        "EXPECTED: 3 clusters - {}",
        if iso_clusters.len() == 3 {
            "PASS"
        } else {
            "FAIL"
        }
    );
    println!();

    // Summary
    println!("{}", "=".repeat(70));
    println!("SUMMARY: Bridge Problem Results");
    println!("{}", "=".repeat(70));
    println!();
    println!("| Algorithm                  | Clusters | Status      |");
    println!("|----------------------------|----------|-------------|");
    println!(
        "| Union-Find (baseline)      | {}        | {}  |",
        result1,
        if result1 == 1 {
            "Expected  "
        } else {
            "Unexpected"
        }
    );
    println!(
        "| Mutual Vouch               | {}        | {}      |",
        result2,
        if result2 >= 2 { "GO   " } else { "NO-GO" }
    );
    println!(
        "| Bridge Removal             | {}        | {}      |",
        result4,
        if result4 >= 2 { "GO   " } else { "NO-GO" }
    );
    println!();

    // Determine GO/NO-GO
    let any_success = result2 >= 2 || result4 >= 2;

    if any_success {
        println!("DECISION: GO");
        println!();
        println!("At least one algorithm successfully distinguishes tight clusters.");
        if result2 >= 2 {
            println!("  - Mutual Vouch Clustering: RECOMMENDED");
            println!("    Simple, effective, uses existing vouch data");
        }
        if result4 >= 2 {
            println!("  - Bridge Removal: Alternative approach available");
        }
    } else {
        println!("DECISION: NO-GO - Use Fallback");
        println!();
        println!("No algorithm successfully distinguishes tight clusters.");
        println!(
            "FALLBACK: Use proxy rule 'Vouchers must not have vouched for each other directly'"
        );
        println!();
        println!("Proxy Rule Benefits:");
        println!("  - Simpler implementation (no cluster algorithm needed)");
        println!("  - Blocks obvious same-cluster vouching");
        println!("  - Trade-off: May reject some valid edge cases");
    }
}

fn print_clusters(clusters: &std::collections::HashMap<String, Vec<String>>) {
    for (id, members) in clusters {
        let mut sorted_members = members.clone();
        sorted_members.sort();
        println!("  {}: {:?}", id, sorted_members);
    }
}
