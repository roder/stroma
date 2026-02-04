# Refinery Quality Gates (Layer 2 Defense)

**Role**: Final validation before merging to main branch

**Status**: 🚧 Documentation Complete, Automation Pending

---

## Defense in Depth Architecture

```
Polecat (feature branch)
        ↓
   Layer 1: Pre-push hook validates
        ↓
   GitHub (feature branch pushed)
        ↓
   Submit to Merge Queue
        ↓
   Layer 2: Refinery validates ← YOU ARE HERE
        ↓
   Merge to Main (only if Layer 2 passes)
```

## Layer 2 Responsibilities

The refinery is the **final gatekeeper** before code reaches main. It MUST validate:

### 1. Quality Gates (Same as Layer 1)

```bash
# Run full quality suite
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
cargo llvm-cov nextest --all-features  # 100% coverage required
cargo deny check  # Supply chain security
```

### 2. Additional Refinery Checks

- **Merge conflict resolution**: If refinery resolves conflicts, re-run quality gates
- **Branch freshness**: Ensure feature branch is up-to-date with main
- **CI status**: Verify GitHub Actions CI passes
- **Security constraints**: Check for violations (see AGENTS.md)

### 3. Why Layer 2 is Essential

Even with Layer 1 (pre-push hooks), Layer 2 catches:

- ✅ Code pushed with `--no-verify` (WIP that wasn't cleaned up)
- ✅ Code pushed with `GASTOWN_SKIP_QUALITY_GATES=1`
- ✅ Merge conflicts resolved in refinery (new code introduced)
- ✅ Bitrot (dependencies changed since feature branch creation)
- ✅ Agent non-compliance (skipped Layer 1 checks)

**Without Layer 2**: Main branch protection relies solely on agent discipline
**With Layer 2**: Enforced validation, main stays green

---

## Implementation Options

### Option A: GitHub Actions (PR Checks) ✅ Recommended

**Approach**: Use GitHub Actions to run quality gates on all PRs

**Workflow**: `.github/workflows/pr-quality-gates.yml`

```yaml
name: PR Quality Gates (Layer 2)

on:
  pull_request:
    branches: [main]

jobs:
  quality-gates:
    name: Layer 2 - Refinery Validation
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1

      - name: Install nextest
        uses: taiki-e/install-action@nextest

      - name: Format check
        run: cargo fmt --check

      - name: Clippy
        run: cargo clippy --all-targets --all-features -- -D warnings

      - name: Tests
        run: cargo nextest run --all-features

      - name: Coverage
        run: cargo llvm-cov nextest --all-features --lcov --output-path coverage.lcov

      - name: Check coverage threshold
        run: |
          # Parse coverage and fail if < 100%
          # (Implementation depends on llvm-cov output format)

      - name: Supply chain security
        run: cargo deny check
```

**Benefits**:
- Automated, runs on every PR
- Visible in PR checks UI
- Blocks merge if fails (via branch protection rules)
- No manual refinery work required

**Setup**:
1. Create workflow file
2. Enable branch protection on main:
   - Require PR reviews
   - Require status checks: "Layer 2 - Refinery Validation"
   - Require branches to be up to date

### Option B: Refinery Patrol Script

**Approach**: Automated script that refinery runs before merging

**Script**: `scripts/refinery-validate.sh`

```bash
#!/bin/bash
# Refinery Layer 2 Validation
# Run this before merging any branch to main

set -e

BRANCH="${1:-$(git branch --show-current)}"

echo "🔍 [Refinery Layer 2] Validating branch: $BRANCH"

# 1. Ensure branch is up-to-date with main
git fetch origin main
if ! git merge-base --is-ancestor origin/main HEAD; then
    echo "❌ Branch is not up-to-date with main"
    echo "   Run: git merge origin/main"
    exit 1
fi

# 2. Run quality gates
echo "→ Running cargo fmt --check"
cargo fmt --check

echo "→ Running cargo clippy"
cargo clippy --all-targets --all-features -- -D warnings

echo "→ Running tests"
cargo nextest run --all-features

echo "→ Running coverage check"
cargo llvm-cov nextest --all-features --summary-only

echo "→ Running supply chain check"
cargo deny check

echo ""
echo "✅ Layer 2 validation passed!"
echo "   Safe to merge to main"
```

**Usage**:
```bash
# Checkout merge queue branch
git checkout merge-queue/feature-xyz

# Run validation
/Users/matt/gt/scripts/refinery-validate.sh

# If passes, merge
git checkout main
git merge merge-queue/feature-xyz
git push
```

**Benefits**:
- Full control over validation
- Can add custom checks
- Works without GitHub Actions

**Drawbacks**:
- Manual process
- Requires refinery discipline

### Option C: Hybrid Approach ✅ Best Practice

**Combine A + B**:
- GitHub Actions runs automatically (primary defense)
- Refinery script for final check before merge (backup)

```bash
# Even if GitHub Actions passed, refinery re-validates
/Users/matt/gt/scripts/refinery-validate.sh

# Double-check before merge
git merge merge-queue/feature-xyz
```

---

## Current Status

- ✅ **Layer 1**: Pre-push hooks enforcing quality gates on ALL pushes
- ⏳ **Layer 2**: Documentation complete, awaiting implementation choice

## Next Steps

**For full protection, implement Layer 2**:

1. **Immediate** (Option B): Create `scripts/refinery-validate.sh`
   - Refinery can start using immediately
   - Manual but effective

2. **Long-term** (Option A): Create GitHub Actions workflow
   - Automated PR checks
   - Configure branch protection rules
   - No manual work required

3. **Best** (Option C): Implement both
   - GitHub Actions for automation
   - Refinery script for final verification

## Recommendation

**Start with Option B** (refinery script) today for immediate protection.
**Add Option A** (GitHub Actions) for long-term automation.

This gives you:
- Immediate Layer 2 protection (manual but reliable)
- Path to full automation (GitHub Actions)
- Defense in depth (both layers active)

---

## Testing Layer 2

### Test 1: Catch formatting issues
```bash
# On feature branch, introduce formatting issue
echo "fn bad_format(){}" >> src/lib.rs
git add . && git commit -m "bad format"

# Push with --no-verify (bypass Layer 1)
git push --no-verify

# Layer 2 should catch it
/Users/matt/gt/scripts/refinery-validate.sh
# Expected: ❌ cargo fmt --check fails
```

### Test 2: Catch test failures
```bash
# Break a test
# Push with --no-verify
# Layer 2 should catch
```

### Test 3: Normal flow
```bash
# Clean code, both layers pass
# Merge should succeed
```

---

## See Also

- **Layer 1 Implementation**: `/Users/matt/gt/scripts/hooks/pre-push`
- **CI/CD Protection**: `docs/CI-CD-PROTECTION.md`
- **Agent Requirements**: `AGENTS.md`
- **Security Constraints**: `.beads/security-constraints.bead`
