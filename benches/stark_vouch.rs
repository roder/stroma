//! Benchmarks for STARK vouch verification
//!
//! Performance requirements:
//! - Proof generation: < 10 seconds
//! - Proof size: < 100KB
//! - Verification: < 100ms (target)

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::collections::BTreeSet;
use stroma::stark::{prove_vouch_claim, verify_vouch_proof, VouchClaim};

fn test_member(id: u8) -> stroma::stark::types::MemberHash {
    let mut bytes = [0u8; 32];
    bytes[0] = id;
    stroma::stark::types::MemberHash(bytes)
}

fn benchmark_proof_generation(c: &mut Criterion) {
    let member = test_member(1);
    let vouchers: BTreeSet<_> = (2..10).map(test_member).collect();
    let flaggers: BTreeSet<_> = (10..15).map(test_member).collect();

    c.bench_function("prove_vouch_claim", |b| {
        b.iter(|| {
            let claim = VouchClaim::new(
                black_box(member),
                black_box(vouchers.clone()),
                black_box(flaggers.clone()),
            );
            prove_vouch_claim(black_box(&claim))
        });
    });
}

fn benchmark_verification(c: &mut Criterion) {
    let member = test_member(1);
    let vouchers: BTreeSet<_> = (2..10).map(test_member).collect();
    let flaggers: BTreeSet<_> = (10..15).map(test_member).collect();
    let claim = VouchClaim::new(member, vouchers, flaggers);
    let proof = prove_vouch_claim(&claim).expect("proof generation failed");

    c.bench_function("verify_vouch_proof", |b| {
        b.iter(|| verify_vouch_proof(black_box(&proof)));
    });
}

criterion_group!(benches, benchmark_proof_generation, benchmark_verification);
criterion_main!(benches);
