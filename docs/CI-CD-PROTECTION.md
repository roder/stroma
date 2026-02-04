# CI/CD Green Branch Protection

## Overview

This document describes the **green branch protection policy** that ensures the `main` branch CI/CD status is **always passing** (✅). This is a non-negotiable requirement enforced through multiple layers: local hooks, remote monitoring, and human authorization gates for infrastructure changes.

## The Core Policy

**ABSOLUTE REQUIREMENT**: Main branch CI/CD status MUST be ✅ passing at all times.

### Why Main Must Stay Green

**Technical reasons:**
- **Enables continuous integration** - Any developer can safely branch from or merge to main
- **Maintains bisectability** - `git bisect` works reliably to find regressions
- **Prevents merge conflict accumulation** - Red main blocks all work, forcing conflicts
- **Allows safe reverts** - Clear "last known good" state to revert to

**Team reasons:**
- **Confidence** - Team can trust main is always deployable
- **Clear quality signal** - No ambiguity about what "passing" means
- **Respects time** - Doesn't waste downstream developers' time on broken code
- **Prevents normalization** - Red main normalizes failure (broken windows theory)

**Process reasons:**
- **Quality gates matter** - Gates are meaningless if main can violate them
- **Forces discipline** - Must fix code, not weaken CI thresholds
- **Accountability** - Clear ownership of failures

## Pre-Merge Checklist

Before any code reaches main, you MUST complete these steps:

### 1. Local Quality Gates

Run all quality checks locally before pushing:

```bash
# Quick security check (2-3 minutes)
cargo fmt --check              # Format verification
cargo clippy --all-targets --all-features -- -D warnings  # Lint
cargo deny check               # Supply chain security

# Full security check (5-10 minutes)
cargo llvm-cov nextest --all-features  # 100% coverage required
cargo build --release --target x86_64-unknown-linux-musl  # Binary size check
```

**All checks must pass ✅ before pushing to PR.**

### 2. Remote CI Status Check

Before merging your PR to main:

```bash
# Watch PR checks in real-time
gh pr checks --watch

# Before merge approval, verify main is currently green
gh api repos/$(gh repo view --json nameWithOwner -q .nameWithOwner)/commits/main/status --jq '.state'
# Must return "success"
```

**Only merge when:**
- ✅ All local checks pass
- ✅ All PR CI checks pass
- ✅ Main branch is currently green
- ✅ No blocking dependencies remain

### 3. Merge with Confidence

If all checks pass, merge via:
```bash
gh pr merge --squash  # or --merge or --rebase, per project preference
```

## CI Failure Response Protocol

**If main branch CI fails**, follow this escalation procedure:

### Immediate Triage (Within 5 Minutes)

1. **File P0 bug immediately:**
   ```bash
   bd create \
     --title="CI BROKEN: <failure-description>" \
     --type=bug \
     --priority=0 \
     --description="CI is failing on main branch.

   Workflow: <workflow-name>
   Run: <run-url>
   Commit: <breaking-sha>

   DO NOT MERGE MORE PRS UNTIL FIXED."
   ```

2. **Notify project owner:**
   ```bash
   gt mail send mayor/ -s "🚨 CI BROKEN on main"
   ```

3. **Identify breaking commit:**
   ```bash
   # Find recent commits
   git log origin/main --oneline -10

   # Check which commit broke CI
   gh api repos/$(gh repo view --json nameWithOwner -q .nameWithOwner)/actions/runs \
     --jq '.workflow_runs[] | select(.head_branch=="main" and .conclusion=="failure") | {sha: .head_sha, message: .head_commit.message, url: .html_url}'
   ```

### Immediate Action (Within 15 Minutes)

**DO NOT push more code to main until fixed.**

Choose one of these actions:

#### Option A: Fix Forward (Preferred)

1. Create hotfix branch from main:
   ```bash
   git checkout -b hotfix/ci-failure origin/main
   ```

2. Make minimal fix to restore CI

3. Test locally (all checks must pass)

4. Push and merge immediately:
   ```bash
   git push -u origin hotfix/ci-failure
   gh pr create --title "Fix CI failure: <description>" --base main
   gh pr merge --squash  # After CI passes
   ```

#### Option B: Revert Breaking Commit

1. Identify exact breaking commit (see Immediate Triage step 3)

2. Create revert:
   ```bash
   git checkout main
   git pull
   git revert <breaking-sha>
   ```

3. Push revert (emergency bypass if hook blocks):
   ```bash
   git push origin main --no-verify
   ```

4. Verify CI recovers:
   ```bash
   gh run watch
   ```

5. File issue to properly fix reverted code:
   ```bash
   bd create \
     --title="Properly fix reverted commit: <original-title>" \
     --type=bug \
     --priority=1
   ```

### Root Cause Analysis (Within 24 Hours)

After main is green again:

1. **Document what broke and why**
   - Update P0 bug with RCA
   - Identify gap in local or CI checks
   - Determine if policy needs strengthening

2. **Add regression test**
   - Ensure this failure mode is caught in future
   - Add to local pre-push checks if applicable

3. **Update processes**
   - If policy gap: Update `.beads/ci-cd-protection.bead`
   - If tooling gap: File issue for better automation
   - If human error: Add to AGENTS.md checklist

## CI/CD Infrastructure Changes

### Human Authorization Requirement

**ABSOLUTE RULE**: Changes to CI/CD infrastructure REQUIRE human approval.

This is not about agent autonomy—it's about **infrastructure stability**. CI/CD is the codebase's immune system. Agents can fix bugs in code, but changing the immune system requires human oversight.

### Protected Files

**Cannot modify without explicit human authorization:**

```
.github/workflows/*.yml          # All workflow definitions
.github/actions/*                # Custom actions
.github/codeql/codeql-config.yml # CodeQL configuration
deny.toml                        # Dependency policy
.git/hooks/*                     # Git hooks (logic changes, not beads shims)
```

**What makes a file "protected":**
- Defines or configures CI/CD jobs
- Gates merges or deployments
- Enforces security/quality thresholds
- Automates infrastructure decisions

### When Authorization is Required

**Requires human approval:**
- ✅ Modifying workflow job definitions
- ✅ Changing workflow triggers (`on:` sections)
- ✅ Adding/removing security checks
- ✅ Updating coverage thresholds
- ✅ Changing binary size limits
- ✅ Modifying CodeQL query suites
- ✅ Updating cargo-deny policy rules
- ✅ Changing branch protection rules
- ✅ Modifying git hook logic

**Does NOT require approval (agents can change freely):**
- ❌ Source code that causes CI failures (fix the code!)
- ❌ Test code to achieve coverage requirements
- ❌ Documentation in `docs/` folder
- ❌ Dependency updates (if allowed by deny.toml)
- ❌ AGENTS.md or other agent guidance
- ❌ Issue/PR templates
- ❌ GitHub labels or milestones

### Authorization Process

**Step-by-step workflow:**

1. **Identify need** for CI/CD change

2. **File issue** with detailed proposal:
   ```bash
   bd create \
     --title="CI/CD: <clear-description-of-change>" \
     --type=bug \
     --priority=1 \
     --description="
   **Problem**: <what's wrong with current CI/CD>
   **Proposed Change**: <what to modify>
   **Rationale**: <why this is needed>
   **Risk**: <what could break>
   **Testing**: <how to verify>
   **Alternatives Considered**: <other options>
   "
   ```

3. **Request human review:**
   ```bash
   gt mail send /Users/matt/gt/stromarig/crew/matt \
     -s "CI/CD Change Request: <issue-id>" \
     -m "See <issue-id> for details. Requesting approval to modify <file-path>."
   ```

4. **WAIT for approval**
   - Human will respond via mail or issue comment
   - May request clarifications or modifications
   - May deny if change is too risky or unnecessary
   - Typical response time: <24 hours

5. **After approval**:
   - Make changes in feature branch
   - Test thoroughly (trigger CI on branch)
   - Submit PR with reference to approval:
     ```bash
     gh pr create \
       --title "CI/CD: <change-description>" \
       --body "Implements <issue-id>

     HUMAN_APPROVED: <approval-message-id>

     **Changes:**
     - <change-1>
     - <change-2>

     **Testing:**
     - <test-result-1>
     - <test-result-2>
     "
     ```

6. **Human approves PR** before merge

### Why Human Approval is Required

**Rationale:**

1. **Conflict of interest** - Agents incentivized to weaken gates when they fail
2. **Cascading failures** - Bad CI change can break all future work
3. **Security implications** - CI has access to secrets, deploy keys, release channels
4. **Expertise required** - CI/CD debugging requires understanding of GitHub Actions internals
5. **Accountability** - Humans must take responsibility for infrastructure decisions

**This is NOT about distrust**—it's about appropriate authority boundaries. Agents are trusted to write production code, but infrastructure requires different expertise and risk tolerance.

## Common Scenarios

### Scenario 1: Local checks pass, CI fails

**Root cause**: Environment difference (dependencies, caching, timing)

**Action**:
1. Pull latest changes:
   ```bash
   git pull --rebase origin/main
   ```

2. Clear local state:
   ```bash
   cargo clean
   rm -rf target/
   ```

3. Re-run checks:
   ```bash
   cargo nextest run --all-features
   cargo llvm-cov nextest --all-features
   ```

4. If still passing locally, investigate CI logs for differences:
   - Check for flaky tests (timing, randomness)
   - Check for environment-specific failures (disk space, network)
   - Compare dependency versions (Cargo.lock)

### Scenario 2: Flaky test fails on CI

**Root cause**: Non-deterministic test (timing, randomness, parallelism)

**What NOT to do:**
- ❌ Disable the test
- ❌ Increase timeout arbitrarily
- ❌ Add retry-until-pass in CI config
- ❌ Mark as "known flake" and ignore

**Correct action:**
1. **Fix the test to be deterministic**
   - Use fixed seeds for randomness
   - Mock time-dependent behavior
   - Use proper synchronization for async tests

2. **If fix is complex, file issue:**
   ```bash
   bd create \
     --title="Fix flaky test: <test-name>" \
     --type=bug \
     --priority=1 \
     --description="Test <test-name> is flaky on CI.

   Failure mode: <describe>
   Frequency: <percentage>
   Root cause hypothesis: <theory>
   "
   ```

3. **Temporarily disable ONLY if blocking all work:**
   ```rust
   #[test]
   #[ignore] // TODO(issue-id): Flaky test, fix in progress
   fn flaky_test() { ... }
   ```
   - File P1 bug to re-enable
   - Must be fixed within 1 week

### Scenario 3: Coverage drops below 100%

**Root cause**: New code added without tests

**What NOT to do:**
- ❌ Lower coverage threshold in CI
- ❌ Exclude files from coverage
- ❌ Add empty tests that don't exercise code

**Correct action:**
1. **Identify uncovered code:**
   ```bash
   cargo llvm-cov nextest --all-features --html
   open target/llvm-cov/html/index.html
   ```

2. **Write tests for gaps:**
   - Test all code paths (happy path, error paths, edge cases)
   - Test all branches (if/else, match arms)
   - Test all error conditions

3. **Verify 100% restored:**
   ```bash
   cargo llvm-cov nextest --all-features
   # Should show 100% coverage
   ```

### Scenario 4: Binary size exceeds limit

**Root cause**: New dependency or feature bloat

**What NOT to do:**
- ❌ Increase binary size limit in CI
- ❌ Ignore the warning

**Correct action:**
1. **Investigate bloat:**
   ```bash
   cargo bloat --release --crates
   ```

2. **Identify culprit:**
   - New dependency added recently?
   - New feature with large code footprint?
   - Accidentally included debug symbols?

3. **Reduce size:**
   - Remove unnecessary dependencies
   - Use feature flags to make large features optional
   - Optimize large data structures

4. **If legitimate growth, file issue for human review:**
   ```bash
   bd create \
     --title="Binary size increase: <reason>" \
     --type=bug \
     --priority=2 \
     --description="Binary size increased by <amount> due to <reason>.

   Justification: <why this is necessary>
   Alternatives considered: <other approaches>
   "
   ```

### Scenario 5: Need urgent hotfix, but main is red

**Root cause**: Someone else broke main, blocking your work

**Action**:
1. **File P0 if not already filed** (see CI Failure Response Protocol)

2. **Create hotfix branch from last-known-good commit:**
   ```bash
   # Find last green commit
   LAST_GOOD=$(gh api repos/$(gh repo view --json nameWithOwner -q .nameWithOwner)/actions/runs \
     --jq '.workflow_runs[] | select(.head_branch=="main" and .conclusion=="success") | .head_sha' \
     | head -1)

   # Branch from it
   git checkout -b hotfix/urgent-fix $LAST_GOOD
   ```

3. **Make fix, test locally:**
   ```bash
   # All local checks must pass
   cargo fmt --check
   cargo clippy --all-targets --all-features -- -D warnings
   cargo nextest run --all-features
   ```

4. **Submit PR targeting last-known-good, not main:**
   ```bash
   git push -u origin hotfix/urgent-fix
   gh pr create --base main --title "HOTFIX: <description>"
   ```

5. **After main fixed, rebase and merge normally**

### Scenario 6: GitHub Actions down or API unavailable

**Root cause**: External dependency failure (GitHub infrastructure)

**Action**:
1. **Verify outage:**
   - Check https://www.githubstatus.com/
   - Check internal network connectivity

2. **If confirmed GitHub outage:**
   - Work can continue locally
   - PRs can be prepared but not merged
   - Wait for GitHub recovery

3. **If local network issue:**
   - Fix network connectivity
   - Re-run checks

4. **Emergency bypass** (ONLY if critical hotfix needed during outage):
   ```bash
   # Local checks must still pass
   cargo nextest run --all-features  # etc.

   # Push with hook bypass (CI check will fail anyway)
   git push --no-verify
   ```
   - File issue to verify CI post-recovery
   - This should be EXTREMELY rare

## Emergency Procedures

### When to Use `--no-verify`

**ONLY in these scenarios:**
- ✅ Reverting a commit that broke CI (need to push revert immediately)
- ✅ Hotfix for critical security vulnerability (must bypass temporarily broken CI)
- ✅ CI infrastructure is completely broken (need to push fix to .github/workflows)
- ✅ GitHub Actions outage (confirmed via githubstatus.com)

**NEVER use `--no-verify` to:**
- ❌ Skip failing tests ("I'll fix them later")
- ❌ Bypass coverage requirements ("It's just a small change")
- ❌ Work around flaky tests ("It passes sometimes")
- ❌ Save time ("I'm in a hurry")

### Recovering from Force-Push to Main

**If someone force-pushed to main and broke history:**

1. **DO NOT panic-revert** (will make it worse)

2. **Assess damage:**
   ```bash
   git reflog origin/main  # See what was lost
   git log --oneline --graph --all  # Visualize history
   ```

3. **Coordinate with human immediately:**
   ```bash
   gt mail send human -s "🚨 FORCE PUSH TO MAIN" -m "Force push detected on main.

   Lost commits: <list>
   Current HEAD: <sha>

   Awaiting direction for recovery."
   ```

4. **If safe to recover (human approves):**
   ```bash
   git push origin <good-sha>:main --force-with-lease
   ```

5. **Document incident:**
   - What was force-pushed
   - Why it happened
   - How to prevent recurrence

## Enforcement Mechanisms

### Local: Pre-Push Hook

**Location**: `.git/hooks/pre-push`

**Behavior**:
- If pushing to `main`: Check last commit CI status via GitHub API
- If CI red: Block push with error message + instructions
- If CI green or pushing to branch: Allow
- Skip check with `--no-verify` (emergency escape hatch)

**Future implementation** (requires human approval):
- Currently, this is planned but not yet implemented
- See issue hq-a3nl5 for details

### Remote: CI Monitor Workflow

**Location**: `.github/workflows/ci-monitor.yml`

**Triggers**: On workflow_run completion (ci.yml, security.yml) for main branch

**Behavior**:
1. Check if workflow failed on main branch
2. If failed:
   - Create P0 bug (auto-filed)
   - Send notification
   - Alert project owner

**Future implementation** (requires human approval):
- Currently, this is planned but not yet implemented
- See issue hq-pzcn7 for details

### Remote: Protected Files Verification

**Location**: `.github/workflows/security.yml` (new job)

**Behavior**:
1. Check git diff for protected file patterns
2. If CI/CD files modified:
   - Check PR body for `HUMAN_APPROVED: <message-id>` tag
   - Fail if tag missing
   - Pass if tag present

**Future implementation** (requires human approval):
- Currently, this is planned but not yet implemented
- See issue hq-77o64 for details

## Related Documentation

**Policy documents:**
- `.beads/ci-cd-protection.bead` - Canonical policy source
- `.beads/security-constraints.bead` - Security policy (similar structure)
- `.beads/git-standards.bead` - Commit standards

**Agent instructions:**
- `AGENTS.md` - Agent requirements (includes CI/CD protection section)
- `.cursor/rules/ci-cd-protection.mdc` - Cursor AI context injection

**Workflow documentation:**
- `docs/SECURITY-CI-CD.md` - Security CI/CD workflow details
- `.github/workflows/ci.yml` - Main CI workflow (5 jobs)
- `.github/workflows/security.yml` - Security workflow (7 jobs)

## Revision History

**2026-02-04**: Initial version - Comprehensive CI/CD green branch protection policy
- Establishes main must always be green requirement
- Documents human authorization gate for infrastructure changes
- Defines CI failure response protocol
- Adds common scenario troubleshooting
- Documents emergency procedures

**Future revisions**: This document should be updated when:
- Enforcement mechanisms are implemented (pre-push hook, CI monitor, protected files check)
- New failure modes are discovered
- Authorization process proves too strict or too lenient
- Team grows and different approval model needed
