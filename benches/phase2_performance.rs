//! Phase 2 performance benchmarks
//!
//! Performance targets:
//! - DVR calculation: < 1ms
//! - Cluster detection: < 1ms
//! - Blind Matchmaker: < 200ms
//! - /mesh commands: < 100ms

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::HashSet;
use stroma::freenet::contract::MemberHash;
use stroma::freenet::trust_contract::TrustNetworkState;
use stroma::matchmaker::cluster_detection::detect_clusters;
use stroma::matchmaker::dvr::calculate_dvr;
use stroma::matchmaker::graph_analysis::TrustGraph;
use stroma::matchmaker::strategic_intro::suggest_introductions;

/// Helper to create test member hash
fn test_member(id: u8) -> MemberHash {
    MemberHash::from_bytes(&[id; 32])
}

/// Create a test network with specified size and vouch density
fn create_test_network(size: usize, avg_vouches: usize) -> TrustNetworkState {
    let mut state = TrustNetworkState::new();

    // Add members
    for i in 0..size {
        state.members.insert(test_member(i as u8));
    }

    // Create vouch relationships (ring topology with additional random vouches)
    for i in 0..size {
        let vouchee = test_member(i as u8);
        let mut vouchers = HashSet::new();

        // Ring vouches (each member vouched by neighbors)
        for j in 1..=avg_vouches.min(size - 1) {
            let voucher = test_member(((i + j) % size) as u8);
            vouchers.insert(voucher);
        }

        if !vouchers.is_empty() {
            state.vouches.insert(vouchee, vouchers);
        }
    }

    state
}

/// Create a network with multiple clusters
fn create_clustered_network(cluster_count: usize, cluster_size: usize) -> TrustNetworkState {
    let mut state = TrustNetworkState::new();
    let total_size = cluster_count * cluster_size;

    // Add all members
    for i in 0..total_size {
        state.members.insert(test_member(i as u8));
    }

    // Create vouches within each cluster (fully connected)
    for cluster_id in 0..cluster_count {
        let cluster_start = cluster_id * cluster_size;
        let cluster_end = cluster_start + cluster_size;

        for i in cluster_start..cluster_end {
            let vouchee = test_member(i as u8);
            let mut vouchers = HashSet::new();

            // Vouch from all other members in the cluster
            for j in cluster_start..cluster_end {
                if i != j {
                    vouchers.insert(test_member(j as u8));
                }
            }

            if vouchers.len() >= 3 {
                // Limit to 3 vouches to create validators
                let vouchers_vec: Vec<_> = vouchers.into_iter().take(3).collect();
                state
                    .vouches
                    .insert(vouchee, vouchers_vec.into_iter().collect());
            }
        }
    }

    // Add a few cross-cluster bridges
    if cluster_count > 1 {
        for cluster_id in 0..(cluster_count - 1) {
            let bridge_a = test_member((cluster_id * cluster_size) as u8);
            let bridge_b = test_member(((cluster_id + 1) * cluster_size) as u8);

            // Add mutual vouch between clusters
            if let Some(vouchers) = state.vouches.get_mut(&bridge_a) {
                vouchers.insert(bridge_b);
            }
            if let Some(vouchers) = state.vouches.get_mut(&bridge_b) {
                vouchers.insert(bridge_a);
            }
        }
    }

    state
}

/// Benchmark DVR calculation
fn benchmark_dvr_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("dvr_calculation");

    // Small network (20 members)
    let small_network = create_test_network(20, 3);
    group.bench_function("small_20_members", |b| {
        b.iter(|| calculate_dvr(black_box(&small_network)));
    });

    // Medium network (100 members)
    let medium_network = create_test_network(100, 4);
    group.bench_function("medium_100_members", |b| {
        b.iter(|| calculate_dvr(black_box(&medium_network)));
    });

    // Large network (500 members)
    let large_network = create_test_network(500, 5);
    group.bench_function("large_500_members", |b| {
        b.iter(|| calculate_dvr(black_box(&large_network)));
    });

    // Very large network (1000 members) - stress test
    let xlarge_network = create_test_network(1000, 6);
    group.bench_function("xlarge_1000_members", |b| {
        b.iter(|| calculate_dvr(black_box(&xlarge_network)));
    });

    group.finish();
}

/// Benchmark cluster detection
fn benchmark_cluster_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("cluster_detection");

    // Single cluster
    let single_cluster = create_clustered_network(1, 20);
    group.bench_function("single_cluster_20_members", |b| {
        b.iter(|| detect_clusters(black_box(&single_cluster)));
    });

    // Two clusters
    let two_clusters = create_clustered_network(2, 15);
    group.bench_function("two_clusters_30_members", |b| {
        b.iter(|| detect_clusters(black_box(&two_clusters)));
    });

    // Multiple clusters
    let multi_clusters = create_clustered_network(5, 20);
    group.bench_function("five_clusters_100_members", |b| {
        b.iter(|| detect_clusters(black_box(&multi_clusters)));
    });

    // Large network with clusters
    let large_clusters = create_clustered_network(10, 50);
    group.bench_function("ten_clusters_500_members", |b| {
        b.iter(|| detect_clusters(black_box(&large_clusters)));
    });

    group.finish();
}

/// Benchmark Blind Matchmaker (strategic introductions)
fn benchmark_blind_matchmaker(c: &mut Criterion) {
    let mut group = c.benchmark_group("blind_matchmaker");
    group.sample_size(20); // Reduce sample size for longer benchmarks

    // Small clustered network
    let small_network = create_clustered_network(2, 10);
    let small_graph = TrustGraph::from_state(&small_network);
    group.bench_function("small_2_clusters_20_members", |b| {
        b.iter(|| suggest_introductions(black_box(&small_graph)));
    });

    // Medium network with multiple clusters
    let medium_network = create_clustered_network(3, 20);
    let medium_graph = TrustGraph::from_state(&medium_network);
    group.bench_function("medium_3_clusters_60_members", |b| {
        b.iter(|| suggest_introductions(black_box(&medium_graph)));
    });

    // Large network
    let large_network = create_clustered_network(5, 40);
    let large_graph = TrustGraph::from_state(&large_network);
    group.bench_function("large_5_clusters_200_members", |b| {
        b.iter(|| suggest_introductions(black_box(&large_graph)));
    });

    // Very large network (stress test for <200ms target)
    let xlarge_network = create_clustered_network(10, 50);
    let xlarge_graph = TrustGraph::from_state(&xlarge_network);
    group.bench_function("xlarge_10_clusters_500_members", |b| {
        b.iter(|| suggest_introductions(black_box(&xlarge_graph)));
    });

    group.finish();
}

/// Benchmark TrustGraph construction
fn benchmark_graph_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_construction");

    for size in [20, 100, 500, 1000].iter() {
        let network = create_test_network(*size, 4);
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| TrustGraph::from_state(black_box(&network)));
        });
    }

    group.finish();
}

/// Combined benchmark: Full DVR + Cluster Detection pipeline
fn benchmark_combined_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("combined_analysis");

    // Simulate full mesh analysis workflow
    let network = create_clustered_network(5, 40);

    group.bench_function("full_pipeline_200_members", |b| {
        b.iter(|| {
            let dvr_result = calculate_dvr(black_box(&network));
            let cluster_result = detect_clusters(black_box(&network));
            let graph = TrustGraph::from_state(black_box(&network));
            let introductions = suggest_introductions(black_box(&graph));

            (dvr_result, cluster_result, introductions)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_dvr_calculation,
    benchmark_cluster_detection,
    benchmark_blind_matchmaker,
    benchmark_graph_construction,
    benchmark_combined_analysis
);

criterion_main!(benches);
