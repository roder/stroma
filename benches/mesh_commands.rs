//! Benchmarks for /mesh command parsing and processing
//!
//! Performance requirement: < 100ms per command
//!
//! Note: Current implementation uses stub handlers that return hardcoded strings.
//! These benchmarks focus on command parsing and demonstrate the structure for
//! future async handler benchmarks.

use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use stroma::signal::pm::{parse_command, Command};

fn benchmark_parse_mesh_commands(c: &mut Criterion) {
    c.bench_function("parse_mesh_overview", |b| {
        b.iter(|| parse_command(black_box("/mesh")));
    });

    c.bench_function("parse_mesh_strength", |b| {
        b.iter(|| parse_command(black_box("/mesh strength")));
    });

    c.bench_function("parse_mesh_replication", |b| {
        b.iter(|| parse_command(black_box("/mesh replication")));
    });

    c.bench_function("parse_mesh_config", |b| {
        b.iter(|| parse_command(black_box("/mesh config")));
    });
}

fn benchmark_parse_various_commands(c: &mut Criterion) {
    let commands = vec![
        "/status",
        "/status @alice",
        "/vouch @bob",
        "/flag @charlie spam",
        "/invite @dave knows crypto",
        "/propose config min_vouches 3",
        "/audit operator",
        "/create-group My Trust Network",
    ];

    for cmd_text in commands {
        let bench_name = format!("parse_command_{}", cmd_text.replace(' ', "_"));
        c.bench_function(&bench_name, |b| {
            b.iter(|| parse_command(black_box(cmd_text)));
        });
    }
}

fn benchmark_command_validation(c: &mut Criterion) {
    c.bench_function("validate_mesh_command", |b| {
        b.iter(|| {
            let cmd = parse_command(black_box("/mesh strength"));
            match cmd {
                Command::Mesh { subcommand } => subcommand == Some("strength".to_string()),
                _ => false,
            }
        });
    });
}

fn benchmark_complex_command_parse(c: &mut Criterion) {
    c.bench_function("parse_complex_propose", |b| {
        b.iter(|| {
            parse_command(black_box(
                "/propose config --key min_vouches --value 4 --reason security",
            ))
        });
    });
}

// Note: Async handler benchmarks to be added when implementation is complete
// Example structure for future benchmarks:
//
// use tokio::runtime::Runtime;
//
// fn benchmark_mesh_overview_handler(c: &mut Criterion) {
//     let rt = Runtime::new().unwrap();
//     let state = create_test_network(50, 5);
//
//     c.bench_function("mesh_overview_handler", |b| {
//         b.to_async(&rt).iter(|| async {
//             // Benchmark handle_mesh_overview with real Freenet queries
//         });
//     });
// }

criterion_group!(
    benches,
    benchmark_parse_mesh_commands,
    benchmark_parse_various_commands,
    benchmark_command_validation,
    benchmark_complex_command_parse
);
criterion_main!(benches);
