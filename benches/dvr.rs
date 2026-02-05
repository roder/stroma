//! Benchmarks for DVR (Distinct Validator Ratio) calculation
//!
//! Performance requirement: < 1ms
//!
//! Per mesh-health-metric.bead:
//! - DVR = Distinct_Validators / floor(N/4)
//! - Greedy algorithm selecting validators with non-overlapping voucher sets

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::collections::{BTreeSet, HashMap, HashSet};
use stroma::freenet::contract::MemberHash;
use stroma::freenet::trust_contract::{TrustNetworkState, GroupConfig};
use stroma::matchmaker::dvr::calculate_dvr;

fn test_member(id: u8) -> MemberHash {
    let mut bytes = [0u8; 32];
    bytes[0] = id;
    MemberHash::from_bytes(&bytes)
}

/// Create a test network with specified size and vouch patterns
fn create_test_network(size: usize, avg_vouches: usize) -> TrustNetworkState {
    let members: BTreeSet<_> = (0..size as u8).map(test_member).collect();
    let mut vouches = HashMap::new();

    // Create vouch patterns: each member gets vouches from `avg_vouches` others
    for i in 0..size {
        let member = test_member(i as u8);
        let mut voucher_set = HashSet::new();

        for j in 1..=avg_vouches.min(size - 1) {
            let voucher = test_member(((i + j) % size) as u8);
            if voucher != member {
                voucher_set.insert(voucher);
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
    }
}

fn benchmark_dvr_small_network(c: &mut Criterion) {
    let state = create_test_network(10, 3);

    c.bench_function("dvr_calculation_10_members", |b| {
        b.iter(|| calculate_dvr(black_box(&state)));
    });
}

fn benchmark_dvr_medium_network(c: &mut Criterion) {
    let state = create_test_network(50, 5);

    c.bench_function("dvr_calculation_50_members", |b| {
        b.iter(|| calculate_dvr(black_box(&state)));
    });
}

fn benchmark_dvr_large_network(c: &mut Criterion) {
    let state = create_test_network(100, 8);

    c.bench_function("dvr_calculation_100_members", |b| {
        b.iter(|| calculate_dvr(black_box(&state)));
    });
}

fn benchmark_dvr_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("dvr_scaling");

    for size in [10, 25, 50, 75, 100].iter() {
        let state = create_test_network(*size, 5);
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| calculate_dvr(black_box(&state)));
        });
    }

    group.finish();
}

fn benchmark_dvr_high_connectivity(c: &mut Criterion) {
    // Network with high vouch counts to stress voucher set intersections
    let state = create_test_network(50, 15);

    c.bench_function("dvr_calculation_high_connectivity", |b| {
        b.iter(|| calculate_dvr(black_box(&state)));
    });
}

criterion_group!(
    benches,
    benchmark_dvr_small_network,
    benchmark_dvr_medium_network,
    benchmark_dvr_large_network,
    benchmark_dvr_scaling,
    benchmark_dvr_high_connectivity
);
criterion_main!(benches);
