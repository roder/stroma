# Codecov Setup

**Status**: ⏳ Pending - Requires CODECOV_TOKEN secret

---

## Overview

Codecov integration is configured in the workflows but requires a repository secret to function.

**Current Coverage Target**: 87% (working toward 100%)

---

## Workflows with Codecov Upload

### 1. CI Workflow (`ci.yml`)

Uses reusable coverage workflow that uploads to Codecov:
- Runs on every push to main and PR
- Generates coverage report with `cargo-llvm-cov`
- Uploads to Codecov (optional, won't fail CI if token missing)

### 2. Refinery Quality Gates Workflow (`pr-quality-gates.yml`)

Uploads coverage specifically for PR validation:
- Runs only on pull requests to main
- Flags reports with `refinery-quality-gates`
- Requires `CODECOV_TOKEN` to upload

---

## Setup Instructions

### Step 1: Create Codecov Account

1. Visit https://codecov.io/
2. Sign in with GitHub
3. Add the Stroma repository

### Step 2: Get Upload Token

1. Navigate to repository settings in Codecov
2. Copy the **Repository Upload Token**

### Step 3: Add GitHub Secret

1. Navigate to: Repository Settings → Secrets and variables → Actions
2. Click "New repository secret"
3. Name: `CODECOV_TOKEN`
4. Value: (paste the upload token from Step 2)
5. Click "Add secret"

### Step 4: Verify Integration

Create a test PR and check:

```bash
# Create test branch
git checkout -b test-codecov-upload
echo "// test" >> src/lib.rs
git add . && git commit -m "test: codecov upload"
git push origin test-codecov-upload

# Create PR
gh pr create --title "Test: Codecov Upload" \
  --body "Testing Codecov integration"

# Watch workflow
gh pr checks --watch

# Expected: Coverage uploaded to Codecov, comment appears on PR
```

Clean up:
```bash
gh pr close --delete-branch
```

---

## Current Configuration

### Coverage Thresholds

**Current**: 87% (does not fail builds if below)
**Goal**: 100% (aspirational, not enforced)

**Rationale**: Stroma is in active development. Enforcing 100% coverage would block feature development. We track coverage trends via Codecov and encourage improvement.

### Coverage Ignores

From `reusable-coverage.yml`:
```yaml
--ignore-filename-regex '(docs/spike/.*\.rs|src/main\.rs)$'
```

**Ignored files**:
- `docs/spike/**/*.rs` - Spike code (exploratory, not production)
- `src/main.rs` - CLI entry point (hard to test in isolation)

### Upload Configuration

**From `pr-quality-gates.yml`**:
```yaml
- name: Upload coverage to Codecov
  if: success()  # Only upload if all checks passed
  uses: codecov/codecov-action@v4
  with:
    files: ./coverage.lcov
    fail_ci_if_error: false  # Don't fail CI if upload fails
    flags: refinery-quality-gates
    token: ${{ secrets.CODECOV_TOKEN }}
```

**Key settings**:
- `fail_ci_if_error: false` - Upload is optional, won't block merges
- `if: success()` - Only uploads if Refinery checks passed
- `flags: refinery-quality-gates` - Tags PR coverage separately

---

## Monitoring Coverage

### View Coverage Reports

**Codecov Dashboard**: https://codecov.io/gh/{owner}/stroma

**Features**:
- Line-by-line coverage visualization
- Coverage trends over time
- PR coverage diffs (shows if PR increases/decreases coverage)
- File-level coverage breakdown

### Local Coverage Reports

**Generate HTML report**:
```bash
cargo llvm-cov nextest --all-features --html
open target/llvm-cov/html/index.html
```

**Check coverage percentage**:
```bash
cargo llvm-cov nextest --all-features --summary-only
```

---

## Troubleshooting

### "Codecov upload failed"

**Cause**: CODECOV_TOKEN not set or invalid

**Solution**:
1. Verify secret exists: Repository Settings → Secrets → Actions
2. Regenerate token in Codecov if needed
3. Update GitHub secret

### "Coverage report not appearing on PR"

**Possible causes**:
1. Codecov GitHub App not installed → Install from Codecov dashboard
2. Token permissions incorrect → Use repository upload token (not global token)
3. First upload takes time → Wait 2-3 minutes, check Codecov dashboard

### "Coverage percentage seems wrong"

**Check**:
1. Are spike files being included? (should be ignored)
2. Is `src/main.rs` counted? (should be ignored)
3. Run locally to compare: `cargo llvm-cov nextest --all-features`

---

## Coverage Goals

### Short-term (Current State)

- ✅ Track coverage in CI
- ✅ Generate reports for PRs
- ✅ Warn when coverage drops below 87%
- ⏳ Set up Codecov integration (pending TOKEN)

### Medium-term (Next 3 months)

- 🎯 Achieve 90% coverage
- 🎯 Add coverage requirements to critical modules (crypto, trust logic)
- 🎯 Set up coverage status checks (require coverage doesn't drop)

### Long-term (Future)

- 🎯 Achieve 100% coverage on production code
- 🎯 Enforce 100% coverage on new code (via Codecov configuration)
- 🎯 Property test coverage metrics (track proptest execution)

---

## Related Documentation

- **Refinery Quality Gates**: `scripts/REFINERY-QUALITY-GATES.md`
- **Testing Standards**: `.beads/testing-standards.bead`
- **CI Configuration**: `.github/workflows/ci.yml`
- **Branch Protection**: `docs/BRANCH-PROTECTION-SETUP.md`
