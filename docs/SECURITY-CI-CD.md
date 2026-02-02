# Security CI/CD Workflow

## Overview

This document describes the comprehensive security pipeline that **blocks all PRs to `main`** that violate security constraints defined in `.beads/security-constraints.bead`.

## Automated Security Checks

Every PR must pass **all** of the following checks before merging:

### 1. Supply Chain Security (cargo-deny)

**Tool**: `cargo-deny`
**Configuration**: `deny.toml`
**Enforces**:
- ✅ No security vulnerabilities (RustSec database)
- ✅ No unmaintained dependencies
- ✅ License compliance (Apache-2.0, MIT, BSD only)
- ✅ No banned crates (e.g., `presage-store-sqlite`)
- ✅ No duplicate dependency versions
- ✅ No wildcard dependencies

**Fails if**: Any advisory, license violation, or banned crate detected

### 2. Static Analysis (CodeQL)

**Tool**: GitHub CodeQL
**Configuration**: `.github/codeql/codeql-config.yml`
**Enforces**:
- ✅ SAST (Static Application Security Testing)
- ✅ Security-extended query suite
- ✅ Vulnerability detection
- ✅ Code quality issues

**Fails if**: High or critical severity findings

### 3. Code Quality (Clippy & Rustfmt)

**Tools**: `cargo clippy`, `cargo fmt`
**Enforces**:
- ✅ All clippy warnings treated as errors (`-D warnings`)
- ✅ Code properly formatted
- ✅ No style violations

**Fails if**: Any clippy warning or format deviation

### 4. Test Coverage (100% Required)

**Tool**: `cargo-llvm-cov` with `cargo-nextest`
**Enforces**:
- ✅ 100% line coverage **MANDATORY**
- ✅ All tests must pass
- ✅ Coverage reports uploaded to Codecov

**Fails if**: Coverage drops below 100%

### 5. Binary Size Monitoring

**Target**: `x86_64-unknown-linux-musl`
**Enforces**:
- ✅ Binary size tracked
- ✅ Increase limited to 10% or 1MB

**Fails if**: Binary size exceeds threshold (dependency bloat)

### 6. Security Constraints Compliance

**Source**: `.beads/security-constraints.bead`
**Enforces**:
- ✅ No cleartext Signal IDs stored
- ✅ No `presage-store-sqlite` usage (must use `StromaProtocolStore`)
- ✅ Zeroization on sensitive data
- ✅ No grace periods in ejection logic
- ✅ Unsafe blocks must have `// SAFETY:` comments

**Fails if**: Any security constraint violation detected

### 7. Unsafe Block Detection

**Enforces**:
- ✅ All `unsafe` blocks must be documented with `// SAFETY:` comments
- ✅ Unsafe block count reported

**Fails if**: Unsafe blocks without safety justification

## Running Checks Locally

Before submitting a PR, run these commands:

### Quick Check (Fast)
```bash
# Format check
cargo fmt --check

# Clippy
cargo clippy --all-targets --all-features -- -D warnings

# Supply chain security
cargo deny check
```

### Full Check (Comprehensive)
```bash
# All of the above, plus:

# Tests with coverage
cargo install cargo-llvm-cov cargo-nextest
cargo llvm-cov nextest --all-features

# Build release binary
cargo build --release --target x86_64-unknown-linux-musl

# Security constraint checks
grep -r "presage-store-sqlite" Cargo.toml src/
grep -r "unsafe" --include="*.rs" src/ | grep -v "// SAFETY:"
```

## Security Constraints Reference

### Critical Violations (Auto-Reject)

1. **Cleartext Signal IDs** - NEVER store Signal IDs in cleartext
   - Use `mask_identity()` with HMAC-SHA256
   - Zeroize immediately after hashing

2. **Message Persistence** - NEVER persist message history
   - No `presage-store-sqlite`
   - Custom `StromaProtocolStore` required

3. **Grace Periods** - NEVER add ejection delays
   - Ejection is immediate (no warnings)

4. **Unsafe Blocks** - MUST document with `// SAFETY:` comments

5. **Test Coverage** - 100% coverage MANDATORY

### How to Fix Violations

#### Cleartext Signal ID violation
```rust
// ❌ WRONG
let signal_id = user.signal_id.clone();
self.db.store(signal_id)?;

// ✅ CORRECT
let hash = mask_identity(&user.signal_id, &self.aci_identity);
user.signal_id.zeroize();
self.db.store(hash)?;
```

#### Unsafe block without documentation
```rust
// ❌ WRONG
unsafe {
    ptr::write(dest, value);
}

// ✅ CORRECT
// SAFETY: `dest` is valid for writes and properly aligned.
// `value` is initialized and `T` has no drop glue to leak.
unsafe {
    ptr::write(dest, value);
}
```

#### Test coverage below 100%
```rust
// Add tests for all code paths
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_branches() {
        // Test happy path
        // Test error paths
        // Test edge cases
    }
}
```

## Workflow Triggers

The security pipeline runs on:
- **Push to main/master** - Full security scan
- **Pull requests** - Block merge if any check fails
- **Manual dispatch** - On-demand security audit
- **Weekly schedule** - Sunday midnight UTC

## Artifacts & Reports

All runs generate:
- `cargo-deny-results` - Supply chain audit report
- `coverage-report` - LCOV coverage data
- `binary-size-report` - Size metrics
- `unsafe-blocks-report` - Unsafe usage analysis

## Status Checks (Required)

PRs cannot merge until ALL status checks pass:
- ✅ cargo-deny
- ✅ codeql-analysis
- ✅ static-analysis (clippy + rustfmt)
- ✅ test-coverage (100%)
- ✅ binary-size
- ✅ security-constraints

## Questions?

- See `.beads/security-constraints.bead` for full security policy
- See `.beads/technology-stack.bead` for technology decisions
- See GitHub Actions workflow: `.github/workflows/security.yml`
