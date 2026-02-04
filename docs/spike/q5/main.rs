//! Q5 Spike: Merkle Tree Performance Benchmark
//!
//! Benchmarks Merkle tree operations at various member counts to determine
//! if on-demand generation is viable.

#![allow(dead_code)]
mod merkle;

use merkle::*;
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

/// Number of iterations for each benchmark
const ITERATIONS: usize = 100;

/// Member counts to test
const MEMBER_COUNTS: &[usize] = &[10, 100, 500, 1000, 2000, 5000];

fn main() {
    println!("{}", "=".repeat(70));
    println!("Q5 SPIKE: Merkle Tree Performance Benchmark");
    println!("{}", "=".repeat(70));
    println!();
    println!("Testing on-demand Merkle tree generation from BTreeSet");
    println!("Iterations per benchmark: {}", ITERATIONS);
    println!();

    // Results storage
    let mut results: Vec<BenchmarkResult> = Vec::new();

    for &count in MEMBER_COUNTS {
        println!("{}", "-".repeat(70));
        println!("BENCHMARK: {} members", count);
        println!("{}", "-".repeat(70));

        // Create test data
        let mut members: BTreeSet<Hash> = BTreeSet::new();
        for i in 0..count {
            members.insert(test_hash(i as u64));
        }

        // Benchmark 1: Full tree construction
        let tree_times = benchmark(ITERATIONS, || MerkleTree::from_btreeset(&members));
        println!(
            "  Tree construction:   {:>10.3}ms (avg)",
            tree_times.avg.as_secs_f64() * 1000.0
        );

        // Benchmark 2: Root-only calculation
        let root_times = benchmark(ITERATIONS, || calculate_root(&members));
        println!(
            "  Root calculation:    {:>10.3}ms (avg)",
            root_times.avg.as_secs_f64() * 1000.0
        );

        // Benchmark 3: Proof generation (requires tree)
        let tree = MerkleTree::from_btreeset(&members).unwrap();
        let test_leaf = test_hash(count as u64 / 2); // Middle element
        let proof_gen_times = benchmark(ITERATIONS, || tree.generate_proof(&test_leaf));
        println!(
            "  Proof generation:    {:>10.3}ms (avg)",
            proof_gen_times.avg.as_secs_f64() * 1000.0
        );

        // Benchmark 4: Proof verification
        let proof = tree.generate_proof(&test_leaf).unwrap();
        let root = *tree.root();
        let verify_times = benchmark(ITERATIONS, || verify_proof(&proof, &root));
        println!(
            "  Proof verification:  {:>10.3}ms (avg)",
            verify_times.avg.as_secs_f64() * 1000.0
        );

        // Store results
        results.push(BenchmarkResult {
            members: count,
            tree_construction: tree_times.avg,
            root_calculation: root_times.avg,
            proof_generation: proof_gen_times.avg,
            proof_verification: verify_times.avg,
        });

        println!();
    }

    // Print summary table
    println!("{}", "=".repeat(70));
    println!("SUMMARY: Performance Results");
    println!("{}", "=".repeat(70));
    println!();
    println!("| Members | Tree (ms) | Root (ms) | Proof Gen (ms) | Verify (ms) |");
    println!("|---------|-----------|-----------|----------------|-------------|");

    for r in &results {
        println!(
            "| {:>7} | {:>9.3} | {:>9.3} | {:>14.3} | {:>11.3} |",
            r.members,
            r.tree_construction.as_secs_f64() * 1000.0,
            r.root_calculation.as_secs_f64() * 1000.0,
            r.proof_generation.as_secs_f64() * 1000.0,
            r.proof_verification.as_secs_f64() * 1000.0
        );
    }
    println!();

    // Decision analysis
    println!("{}", "=".repeat(70));
    println!("DECISION ANALYSIS");
    println!("{}", "=".repeat(70));
    println!();

    // Find the 1000 member result
    let target_result = results.iter().find(|r| r.members == 1000);

    if let Some(r) = target_result {
        let root_ms = r.root_calculation.as_secs_f64() * 1000.0;
        let tree_ms = r.tree_construction.as_secs_f64() * 1000.0;

        println!("Decision Criteria (at 1000 members):");
        println!("  Root calculation: {:.3}ms", root_ms);
        println!("  Full tree build:  {:.3}ms", tree_ms);
        println!();

        if root_ms < 100.0 {
            println!("DECISION: GO - Generate on demand");
            println!();
            println!(
                "Root calculation at 1000 members: {:.3}ms < 100ms threshold",
                root_ms
            );
            println!();
            println!("Recommendation:");
            println!("  - Calculate Merkle root on-demand for each verification");
            println!("  - Build full tree only when proof generation needed");
            println!("  - No caching infrastructure required");
        } else if root_ms < 500.0 {
            println!("DECISION: PARTIAL - Cache in bot, invalidate on change");
            println!();
            println!(
                "Root calculation at 1000 members: {:.3}ms (between 100-500ms)",
                root_ms
            );
            println!();
            println!("Recommendation:");
            println!("  - Cache Merkle tree in bot memory");
            println!("  - Invalidate on membership changes");
            println!("  - Re-generate tree after invalidation");
        } else {
            println!("DECISION: NO-GO - Requires optimization or contract caching");
            println!();
            println!(
                "Root calculation at 1000 members: {:.3}ms > 500ms threshold",
                root_ms
            );
            println!();
            println!("Recommendation:");
            println!("  - Investigate faster hash implementation (SIMD, etc.)");
            println!("  - Consider incremental Merkle tree updates");
            println!("  - May need to store tree in Freenet contract");
        }
    } else {
        println!("ERROR: Could not find 1000 member benchmark result");
    }

    // Additional insights
    println!();
    println!("{}", "-".repeat(70));
    println!("Additional Insights:");
    println!("{}", "-".repeat(70));
    println!();

    // Calculate scaling factor
    if let (Some(r100), Some(r1000)) = (
        results.iter().find(|r| r.members == 100),
        results.iter().find(|r| r.members == 1000),
    ) {
        let scale_factor =
            r1000.root_calculation.as_secs_f64() / r100.root_calculation.as_secs_f64();
        println!("Scaling (100 → 1000 members): {:.1}x", scale_factor);
        println!("Expected complexity: O(n log n) for tree building");
    }

    // Memory estimate
    println!();
    println!("Memory estimate for full tree:");
    for r in &results {
        // Each node: 32 bytes hash + ~16 bytes overhead
        // Number of nodes: approximately 2n - 1 for n leaves
        let nodes = 2 * r.members - 1;
        let bytes = nodes * 48; // Approximate
        println!("  {} members: ~{} KB", r.members, bytes / 1024);
    }
}

struct BenchmarkTimes {
    avg: Duration,
    _min: Duration,
    _max: Duration,
}

fn benchmark<T, F: FnMut() -> T>(iterations: usize, mut f: F) -> BenchmarkTimes {
    let mut times: Vec<Duration> = Vec::with_capacity(iterations);

    // Warmup
    for _ in 0..5 {
        let _ = f();
    }

    // Actual benchmark
    for _ in 0..iterations {
        let start = Instant::now();
        let _ = f();
        times.push(start.elapsed());
    }

    times.sort();

    let sum: Duration = times.iter().sum();
    let avg = sum / iterations as u32;
    let min = times[0];
    let max = times[iterations - 1];

    BenchmarkTimes {
        avg,
        _min: min,
        _max: max,
    }
}

struct BenchmarkResult {
    members: usize,
    tree_construction: Duration,
    root_calculation: Duration,
    proof_generation: Duration,
    proof_verification: Duration,
}
