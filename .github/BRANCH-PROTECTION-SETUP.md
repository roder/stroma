# Branch Protection Setup for Main

**Status**: 📋 Documentation - Ready for Implementation

This document describes the branch protection rules to enable on the `main` branch to enforce Layer 2 defense in depth with GitHub Actions.

---

## Overview

With Layer 2 workflows in place, branch protection ensures:
- ✅ All PRs pass quality gates before merge
- ✅ Main branch stays green at all times
- ✅ Linear history (no merge commits)
- ✅ Human review for protected file changes

---

## Branch Protection Configuration

### GitHub UI Settings

Navigate to: **Settings → Branches → Branch protection rules → Add rule**

**Branch name pattern**: `main`

### Required Settings

#### 1. Require a pull request before merging
- ✅ **Require approvals**: 0 (rely on automated checks)
  - _Note: Increase to 1 if you want human review on all PRs_
- ✅ **Dismiss stale pull request approvals when new commits are pushed**
- ✅ **Require review from Code Owners**: Off (unless you have CODEOWNERS file)

#### 2. Require status checks to pass before merging
- ✅ **Require branches to be up to date before merging**

**Required status checks** (these must pass before merge):

From `pr-quality-gates.yml`:
- `Layer 2 - Refinery Validation`

From `ci.yml`:
- `Format & Lint`
- `Test Suite`
- `Code Coverage`
- `Dependencies`
- `CI Success`

From `security.yml`:
- `Static Analysis (CodeQL)`
- `Security Code Quality Checks`
- `Binary Size Monitor`
- `Security Constraints (Bead Compliance)`
- `Supply Chain Security`
- `Protected Files Verification`
- `Security Summary`

**Total: 14 required checks**

#### 3. Require conversation resolution before merging
- ✅ **Enabled** - All PR comments must be resolved

#### 4. Require linear history
- ✅ **Enabled** - Enforces `--ff-only` merges, no merge commits

#### 5. Do not allow bypassing the above settings
- ✅ **Do not allow bypassing the above settings** - Even admins must pass checks
  - _Exception: Can be disabled temporarily for emergency hotfixes_

#### 6. Allow force pushes
- ❌ **Disabled** - No force pushes to main

#### 7. Allow deletions
- ❌ **Disabled** - Cannot delete main branch

---

## Verification

After enabling branch protection, verify it's working:

### Test 1: Try to push directly to main
```bash
git checkout main
echo "test" >> README.md
git add README.md
git commit -m "test: direct push"
git push origin main
```

**Expected**: ❌ Rejected - "Cannot push to protected branch"

### Test 2: Create PR without passing checks
```bash
git checkout -b test-branch
echo "bad_format(){}" >> src/lib.rs
git add . && git commit -m "bad format"
git push origin test-branch
gh pr create --title "Test PR" --body "Testing branch protection"
```

**Expected**: 
- PR created ✅
- Checks run and fail ❌
- Merge button disabled 🚫

### Test 3: Create PR with passing checks
```bash
git checkout -b test-branch-good
# Make valid changes
cargo fmt
git add . && git commit -m "feat: valid change"
git push origin test-branch-good
gh pr create --title "Test PR" --body "Testing branch protection"
```

**Expected**:
- PR created ✅
- All 14 checks pass ✅
- Merge button enabled ✅

---

## Emergency Procedures

### If Main Branch CI Breaks

**IMMEDIATE Response**:
1. File P0 bug: `bd create --title="CI BROKEN: <description>" --type=bug --priority=0`
2. Notify crew: `gt mail send crew/ -s "🚨 CI BROKEN ON MAIN"`
3. **DO NOT merge more PRs** until main is green
4. Options:
   - **Option A**: Revert the breaking commit
   - **Option B**: Fix forward (only if fix is trivial)

**Temporary bypass** (if absolutely necessary):
1. Get human authorization
2. Admin can temporarily disable "Do not allow bypassing" setting
3. Push hotfix
4. **Immediately re-enable** bypass protection
5. File post-mortem issue

### If GitHub Actions is Down

**Fallback to manual validation**:
1. Checkout branch locally
2. Run refinery validation script: `scripts/refinery-validate.sh`
3. If passes, manually merge (requires admin override)
4. Document in commit message: `Manual merge due to GitHub Actions outage`

---

## Workflow Integration

### For Polecats (Feature Development)

1. Develop on feature branch
2. Pre-push hook (Layer 1) validates locally
3. Push to remote
4. Create PR
5. Layer 2 workflows run automatically
6. Wait for all 14 checks to pass ✅
7. Merge button becomes available
8. Refinery or human merges

### For Refinery (Merge Queue Processing)

1. Receive MERGE_READY notification
2. **Optional**: Run `bd formula stroma-refinery-validate` locally for confidence
3. GitHub Actions already validated via PR checks
4. Merge with `git merge --ff-only` (enforced by branch protection)
5. Send MERGED notification
6. Close MR bead

### For Protected File Changes

If PR modifies protected files (`.github/workflows/`, `deny.toml`, etc.):

1. `Protected Files Verification` check runs
2. Checks PR body for `HUMAN_APPROVED: <message-id>`
3. If missing → Check fails → Merge blocked
4. To approve:
   ```bash
   # Request approval
   gt mail send  mayor -s "CI/CD Change Request: <pr-number>"
   
   # After human review, add to PR description:
   # HUMAN_APPROVED: <mail-message-id>
   
   # Workflow re-runs and passes
   ```

---

## Monitoring

### Check Branch Protection Status
```bash
gh api repos/{owner}/{repo}/branches/main/protection | jq '.required_status_checks'
```

### View Required Checks
```bash
gh api repos/{owner}/{repo}/branches/main/protection/required_status_checks | jq '.contexts'
```

### Check CI Status Before Merge
```bash
# From pre-push hook or refinery validation:
gh api repos/{owner}/{repo}/commits/main/status --jq '.state'
# Must return "success"
```

---

## Troubleshooting

### "Required status check is not passing"

**Cause**: One or more of the 14 required checks failed.

**Solution**:
1. View PR checks: `gh pr checks`
2. Identify failing check
3. View logs: Click "Details" link in PR UI
4. Fix the issue in the PR branch
5. Push fix: Checks re-run automatically

### "Branch is out of date"

**Cause**: Main branch has moved since PR was created.

**Solution**:
```bash
git checkout <pr-branch>
git merge origin/main
# Resolve conflicts if any
git push
# Checks re-run on updated branch
```

### "Merge button disabled even though checks pass"

**Possible causes**:
1. Unresolved PR comments → Resolve all conversations
2. Branch out of date → Merge main into PR branch
3. Insufficient approvals → Get required reviewers to approve
4. Protected files without approval → Add `HUMAN_APPROVED:` to PR body

---

## Related Documentation

- **Layer 1 Defense**: `scripts/README.md` - Pre-push hooks
- **Layer 2 Defense**: `scripts/REFINERY-QUALITY-GATES.md` - PR quality gates
- **CI/CD Protection**: `docs/CI-CD-PROTECTION.md` - Protected file authorization
- **Security Constraints**: `.beads/security-constraints.bead` - The 8 Absolutes

---

## Summary

**Defense in Depth:**
```
Developer (Local)
      ↓
Layer 1: Pre-push hook validates
      ↓
GitHub (Remote PR)
      ↓
Layer 2: 14 required status checks
      ↓
Branch Protection: Blocks merge if checks fail
      ↓
Main Branch (Protected, Always Green) ✅
```

**Key Principle**: Main branch must ALWAYS be green. Branch protection + Layer 2 workflows enforce this automatically.
