//! Benchmarks for cluster detection using connected components
//!
//! Performance requirement: < 1ms
//!
//! Per blind-matchmaker-dvr.bead and Q3:
//! - Finds connected components in vouch graph
//! - Each component represents a cluster
//! - GAP-11: Announce when ≥2 clusters detected

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::{BTreeSet, HashMap, HashSet};
use stroma::freenet::contract::MemberHash;
use stroma::freenet::trust_contract::{GroupConfig, TrustNetworkState};
use stroma::matchmaker::cluster_detection::detect_clusters;

fn test_member(id: u8) -> MemberHash {
    let mut bytes = [0u8; 32];
    bytes[0] = id;
    MemberHash::from_bytes(&bytes)
}

/// Create a network with a single fully connected cluster
fn create_single_cluster(size: usize) -> TrustNetworkState {
    let members: BTreeSet<_> = (0..size as u8).map(test_member).collect();
    let mut vouches = HashMap::new();

    // Full mesh: everyone vouches for everyone
    for i in 0..size {
        let member = test_member(i as u8);
        let mut voucher_set = HashSet::new();

        for j in 0..size {
            if i != j {
                voucher_set.insert(test_member(j as u8));
            }
        }

        vouches.insert(member, voucher_set);
    }

    TrustNetworkState {
        members,
        ejected: BTreeSet::new(),
        vouches,
        flags: HashMap::new(),
        config: GroupConfig::default(),
        config_timestamp: 0,
        schema_version: 1,
        federation_contracts: vec![],
        gap11_announcement_sent: false,
        active_proposals: HashMap::new(),
        audit_log: vec![],
    }
}

/// Create a network with multiple disconnected clusters
fn create_multi_cluster(clusters: usize, cluster_size: usize) -> TrustNetworkState {
    let total_size = clusters * cluster_size;
    let members: BTreeSet<_> = (0..total_size as u8).map(test_member).collect();
    let mut vouches = HashMap::new();

    // Create separate clusters
    for cluster_id in 0..clusters {
        let start = cluster_id * cluster_size;
        let end = start + cluster_size;

        for i in start..end {
            let member = test_member(i as u8);
            let mut voucher_set = HashSet::new();

            // Vouch only within cluster
            for j in start..end {
                if i != j {
                    voucher_set.insert(test_member(j as u8));
                }
            }

            vouches.insert(member, voucher_set);
        }
    }

    TrustNetworkState {
        members,
        ejected: BTreeSet::new(),
        vouches,
        flags: HashMap::new(),
        config: GroupConfig::default(),
        config_timestamp: 0,
        schema_version: 1,
        federation_contracts: vec![],
        gap11_announcement_sent: false,
        active_proposals: HashMap::new(),
        audit_log: vec![],
    }
}

/// Create a network with sparse connectivity (star topology per cluster)
fn create_sparse_clusters(clusters: usize, cluster_size: usize) -> TrustNetworkState {
    let total_size = clusters * cluster_size;
    let members: BTreeSet<_> = (0..total_size as u8).map(test_member).collect();
    let mut vouches = HashMap::new();

    for cluster_id in 0..clusters {
        let start = cluster_id * cluster_size;
        let hub = test_member(start as u8);

        // Hub vouches for all spokes
        let mut hub_vouchers = HashSet::new();
        for i in 1..cluster_size {
            hub_vouchers.insert(test_member((start + i) as u8));
        }
        if !hub_vouchers.is_empty() {
            vouches.insert(hub, hub_vouchers);
        }

        // Spokes vouch for hub
        for i in 1..cluster_size {
            let spoke = test_member((start + i) as u8);
            let mut spoke_vouchers = HashSet::new();
            spoke_vouchers.insert(hub);
            vouches.insert(spoke, spoke_vouchers);
        }
    }

    TrustNetworkState {
        members,
        ejected: BTreeSet::new(),
        vouches,
        flags: HashMap::new(),
        config: GroupConfig::default(),
        config_timestamp: 0,
        schema_version: 1,
        federation_contracts: vec![],
        gap11_announcement_sent: false,
        active_proposals: HashMap::new(),
        audit_log: vec![],
    }
}

fn benchmark_single_cluster_small(c: &mut Criterion) {
    let state = create_single_cluster(10);

    c.bench_function("cluster_detection_single_10", |b| {
        b.iter(|| detect_clusters(black_box(&state)));
    });
}

fn benchmark_single_cluster_large(c: &mut Criterion) {
    let state = create_single_cluster(100);

    c.bench_function("cluster_detection_single_100", |b| {
        b.iter(|| detect_clusters(black_box(&state)));
    });
}

fn benchmark_multi_cluster_small(c: &mut Criterion) {
    let state = create_multi_cluster(3, 10);

    c.bench_function("cluster_detection_3x10", |b| {
        b.iter(|| detect_clusters(black_box(&state)));
    });
}

fn benchmark_multi_cluster_large(c: &mut Criterion) {
    let state = create_multi_cluster(5, 20);

    c.bench_function("cluster_detection_5x20", |b| {
        b.iter(|| detect_clusters(black_box(&state)));
    });
}

fn benchmark_sparse_clusters(c: &mut Criterion) {
    let state = create_sparse_clusters(4, 15);

    c.bench_function("cluster_detection_sparse_4x15", |b| {
        b.iter(|| detect_clusters(black_box(&state)));
    });
}

fn benchmark_cluster_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("cluster_scaling");

    for size in [10, 25, 50, 75, 100].iter() {
        let state = create_single_cluster(*size);
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| detect_clusters(black_box(&state)));
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_single_cluster_small,
    benchmark_single_cluster_large,
    benchmark_multi_cluster_small,
    benchmark_multi_cluster_large,
    benchmark_sparse_clusters,
    benchmark_cluster_scaling
);
criterion_main!(benches);
