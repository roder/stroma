---
name: Comprehensive Security CI/CD Workflow
overview: Create a comprehensive GitHub Actions workflow that integrates EmbarkStudios/cargo-deny-action for supply chain security, GitHub CodeQL for SAST, and additional security checks including clippy, rustfmt, binary size monitoring, and coverage requirements.
todos: []
isProject: false
---

# Comprehensive Security CI/CD Workflow Plan

## Overview

This plan creates a multi-layered security workflow for the Stroma Rust project that combines:

- **Supply Chain Security**: cargo-deny-action for dependency auditing
- **Static Analysis (SAST)**: GitHub CodeQL for vulnerability detection
- **Code Quality**: clippy, rustfmt, and additional linting
- **Binary Security**: Size monitoring and build verification
- **Test Coverage**: Coverage enforcement with reporting

## Files to Create

### 1. GitHub Actions Workflow

**File**: `.github/workflows/security.yml`

A comprehensive workflow that runs on:

- Push to main/master branches
- Pull requests
- Manual workflow dispatch
- Scheduled runs (weekly)

**Jobs Structure**:

- `cargo-deny`: Supply chain security checks
- `codeql-analysis`: SAST with CodeQL
- `static-analysis`: clippy, rustfmt, unsafe block detection
- `test-coverage`: nextest with coverage reporting
- `binary-size`: Monitor binary size for dependency bloat
- `security-summary`: Aggregate results and create summary

### 2. cargo-deny Configuration

**File**: `deny.toml`

Configuration for cargo-deny with:

- **Advisories**: Security vulnerability checks from RustSec
- **Bans**: Prohibited crates and version conflicts
- **Licenses**: License compliance checking
- **Sources**: Allowed dependency sources
- **Graph**: Dependency graph analysis

### 3. CodeQL Configuration

**File**: `.github/codeql/codeql-config.yml`

CodeQL analysis configuration for Rust:

- Query suites: `security-extended` for comprehensive analysis
- Build mode: `none` (analyzes codebase directly)
- Custom query paths if needed

## Workflow Components

### Job 1: cargo-deny (Supply Chain Security)

- Uses `EmbarkStudios/cargo-deny-action@v1`
- Checks:
  - Security advisories (RustSec database)
  - License compliance
  - Banned crates
  - Duplicate dependencies
  - Wildcard dependencies
- Fails on any violations
- Uploads results as artifact

### Job 2: CodeQL Analysis (SAST)

- Uses `github/codeql-action/init@v2` and `github/codeql-action/analyze@v2`
- Language: Rust
- Query suite: `security-extended` (includes security queries)
- Build mode: `none` (no build required for Rust)
- Uploads SARIF results for GitHub Security tab
- Runs on push and pull requests

### Job 3: Static Analysis

- **Clippy**: `cargo clippy -- -D warnings` (treat warnings as errors)
- **Rustfmt**: `cargo fmt --check` (format verification)
- **Unsafe Detection**: Custom script to detect and report `unsafe` blocks
- **Dependency Check**: Verify no prohibited dependencies added
- All checks must pass

### Job 4: Test Coverage

- Uses `cargo-nextest` for test execution
- Coverage tool: `cargo-llvm-cov` or `cargo-tarpaulin`
- Enforces 100% coverage requirement
- Generates coverage reports (LCOV format)
- Uploads coverage to Codecov or similar (optional)
- Fails if coverage drops below threshold

### Job 5: Binary Size Monitoring

- Builds release binary for `x86_64-unknown-linux-musl` target
- Measures binary size
- Compares against baseline (stored in workflow artifact or repo)
- Fails if size increase exceeds threshold (e.g., 10% or 1MB)
- Uploads binary size report

### Job 6: Security Summary

- Aggregates results from all security jobs
- Creates summary comment on PR (if applicable)
- Reports:
  - cargo-deny violations
  - CodeQL findings
  - Coverage percentage
  - Binary size change
  - Unsafe block count

## Configuration Details

### deny.toml Structure

```toml
[advisories]
# Check for security vulnerabilities
db-path = ""
db-urls = ["https://github.com/rustsec/advisory-db"]
vulnerability = "deny"
unmaintained = "warn"
unsound = "deny"
notice = "warn"
ignore = []

[bans]
# Prevent duplicate versions
multiple-versions = "deny"
# Prevent wildcard dependencies
wildcards = "deny"
# Ban specific crates
deny = []
skip = []

[licenses]
# License compliance
confidence-threshold = 0.9
allow = ["Apache-2.0", "MIT", "BSD-3-Clause", "ISC", "Zlib"]
deny = ["GPL-2.0", "GPL-3.0"]
copyleft = "deny"
clarify = {}

[sources]
# Allowed dependency sources
unknown-registry = "deny"
unknown-git = "deny"
path = "deny"
registry = ["crates-io"]

[graph]
# Dependency graph analysis
targets = ["x86_64-unknown-linux-musl"]
all-features = true
```

### CodeQL Configuration

- Query suite: `security-extended` for comprehensive security analysis
- Build mode: `none` (Rust doesn't require build for analysis)
- Custom queries: None initially, can be added later

## Workflow Features

### Matrix Strategy

- Run on multiple Rust versions (stable, beta for future-proofing)
- Multiple platforms if needed (ubuntu-latest primary)

### Caching

- Cache Rust toolchain
- Cache cargo registry
- Cache cargo-deny database
- Cache CodeQL database

### Artifacts

- cargo-deny reports
- CodeQL SARIF results
- Coverage reports
- Binary size reports
- Security summary

### Notifications

- PR comments with security findings
- Status checks that block merging
- Optional: Slack/Discord notifications for critical findings

## Security Checks Alignment

### Project Requirements (from tech-stack.mdc)

- ✅ cargo-deny: Mandatory dependency auditing
- ✅ cargo-crev: Can be added as separate job (optional)
- ✅ clippy: Code quality and safety
- ✅ rustfmt: Code formatting
- ✅ Binary size monitoring: Detect dependency bloat
- ✅ Coverage: 100% requirement enforcement

### Security Guardrails (from security-guardrails.mdc)

- ✅ No unsafe blocks without review (detection job)
- ✅ Supply chain security (cargo-deny)
- ✅ Dependency auditing (cargo-deny)
- ✅ Static analysis (CodeQL, clippy)

## Implementation Steps

1. Create `.github/workflows/` directory
2. Create `security.yml` workflow file
3. Create `deny.toml` configuration file
4. Create `.github/codeql/` directory and `codeql-config.yml`
5. Test workflow on a test branch
6. Verify all checks pass
7. Document workflow in README or separate docs

## Dependencies

### GitHub Actions

- `actions/checkout@v4`
- `actions-rs/toolchain@v1` (Rust toolchain management)
- `EmbarkStudios/cargo-deny-action@v1`
- `github/codeql-action@v2`
- `Swatinem/rust-cache@v2` (caching)

### Cargo Tools (installed in workflow)

- `cargo-deny` (via action)
- `cargo-nextest`
- `cargo-llvm-cov` or `cargo-tarpaulin`
- `cargo-clippy` (built-in)
- `rustfmt` (built-in)

## Workflow Triggers

- `push`: On push to main/master
- `pull_request`: On PR creation/update
- `workflow_dispatch`: Manual trigger
- `schedule`: Weekly security scan (cron: `0 0 * * 0`)

## Failure Conditions

Workflow fails if:

- cargo-deny finds security vulnerabilities
- cargo-deny finds license violations
- CodeQL finds high/critical severity issues
- Clippy finds warnings (with `-D warnings`)
- Code is not formatted (rustfmt check fails)
- Test coverage drops below 100%
- Binary size increases beyond threshold
- Unsafe blocks detected without justification

## Success Criteria

- All security checks pass
- No high/critical CodeQL findings
- 100% test coverage maintained
- Binary size within acceptable range
- No cargo-deny violations
- All code properly formatted