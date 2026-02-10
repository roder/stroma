//! Benchmarks for Blind Matchmaker assessor selection
//!
//! Performance requirement: < 200ms
//!
//! Per blind-matchmaker-dvr.bead:
//! - Cross-cluster assessor selection
//! - DVR optimization (distinct validators, non-overlapping voucher sets)

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::{BTreeSet, HashMap, HashSet};
use stroma::freenet::contract::MemberHash;
use stroma::freenet::trust_contract::{GroupConfig, TrustNetworkState};
use stroma::signal::matchmaker::BlindMatchmaker;

fn test_member(id: u8) -> MemberHash {
    let mut bytes = [0u8; 32];
    bytes[0] = id;
    MemberHash::from_bytes(&bytes)
}

/// Create a network with multiple clusters
fn create_clustered_network(cluster_count: usize, cluster_size: usize) -> TrustNetworkState {
    let total_size = cluster_count * cluster_size;
    let members: BTreeSet<_> = (0..total_size as u8).map(test_member).collect();
    let mut vouches = HashMap::new();

    // Create clusters with internal vouching
    for cluster_id in 0..cluster_count {
        let start = cluster_id * cluster_size;
        let end = start + cluster_size;

        for i in start..end {
            let member = test_member(i as u8);
            let mut voucher_set = HashSet::new();

            // Vouch for others in the same cluster
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

/// Create a single well-connected cluster
fn create_single_cluster_network(size: usize) -> TrustNetworkState {
    let members: BTreeSet<_> = (0..size as u8).map(test_member).collect();
    let mut vouches = HashMap::new();

    for i in 0..size {
        let member = test_member(i as u8);
        let mut voucher_set = HashSet::new();

        // Each member vouches for ~half the network
        for j in 0..size {
            if i != j && j % 2 == 0 {
                voucher_set.insert(test_member(j as u8));
            }
        }

        if !voucher_set.is_empty() {
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

fn benchmark_matchmaker_small_network(c: &mut Criterion) {
    let state = create_clustered_network(3, 5);
    let inviter = test_member(0);
    let excluded = HashSet::new();

    c.bench_function("matchmaker_select_3x5", |b| {
        b.iter(|| {
            BlindMatchmaker::select_validator(
                black_box(&state),
                black_box(&inviter),
                black_box(&excluded),
            )
        });
    });
}

fn benchmark_matchmaker_medium_network(c: &mut Criterion) {
    let state = create_clustered_network(5, 10);
    let inviter = test_member(0);
    let excluded = HashSet::new();

    c.bench_function("matchmaker_select_5x10", |b| {
        b.iter(|| {
            BlindMatchmaker::select_validator(
                black_box(&state),
                black_box(&inviter),
                black_box(&excluded),
            )
        });
    });
}

fn benchmark_matchmaker_large_network(c: &mut Criterion) {
    let state = create_clustered_network(10, 10);
    let inviter = test_member(0);
    let excluded = HashSet::new();

    c.bench_function("matchmaker_select_10x10", |b| {
        b.iter(|| {
            BlindMatchmaker::select_validator(
                black_box(&state),
                black_box(&inviter),
                black_box(&excluded),
            )
        });
    });
}

fn benchmark_matchmaker_single_cluster(c: &mut Criterion) {
    let state = create_single_cluster_network(50);
    let inviter = test_member(0);
    let excluded = HashSet::new();

    c.bench_function("matchmaker_select_single_cluster_50", |b| {
        b.iter(|| {
            BlindMatchmaker::select_validator(
                black_box(&state),
                black_box(&inviter),
                black_box(&excluded),
            )
        });
    });
}

fn benchmark_matchmaker_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("matchmaker_scaling");

    for size in [10, 25, 50, 75, 100].iter() {
        let state = create_single_cluster_network(*size);
        let inviter = test_member(0);
        let excluded = HashSet::new();
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                BlindMatchmaker::select_validator(
                    black_box(&state),
                    black_box(&inviter),
                    black_box(&excluded),
                )
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_matchmaker_small_network,
    benchmark_matchmaker_medium_network,
    benchmark_matchmaker_large_network,
    benchmark_matchmaker_single_cluster,
    benchmark_matchmaker_scaling
);
criterion_main!(benches);
