# Git Hooks for CI/CD Protection

This directory contains git hooks that enforce CI/CD green branch protection.

## Available Hooks

### `pre-push`

**Purpose**: Prevents pushing to main when CI is failing

**What it does**:
1. Runs `bd hooks run pre-push` for beads JSONL sync (if bd is available)
2. Checks GitHub Actions CI status on main branch
3. Blocks push if main CI is red (failure or error state)
4. Shows clear error message with fix instructions
5. Provides emergency bypass via `--no-verify`

**Part of**: CI/CD Green Branch Protection (hq-a3nl5, hq-diq31)

## Installation

### Automatic Installation (Recommended)

```bash
# From the repository root
./scripts/hooks/install.sh
```

### Manual Installation

```bash
# Copy pre-push hook
cp scripts/hooks/pre-push .git/hooks/pre-push
chmod +x .git/hooks/pre-push

# Verify installation
.git/hooks/pre-push --help 2>&1 | head -10
```

### Installation for All Stromarig Repos

To install in mayor, refinery, and polecats:

```bash
# Mayor
cp scripts/hooks/pre-push .git/hooks/pre-push
chmod +x .git/hooks/pre-push

# Refinery
cp scripts/hooks/pre-push ../refinery/rig/.git/hooks/pre-push
chmod +x ../refinery/rig/.git/hooks/pre-push

# Polecats (each one)
for polecat in ../polecats/*/stromarig; do
    if [ -d "$polecat/.git" ]; then
        cp scripts/hooks/pre-push "$polecat/.git/hooks/pre-push"
        chmod +x "$polecat/.git/hooks/pre-push"
    fi
done
```

## Requirements

The pre-push hook requires:
- **gh CLI**: For checking CI status
  ```bash
  brew install gh
  gh auth login
  ```
- **bd CLI** (optional): For beads sync
  ```bash
  brew install beads
  ```

## Testing

### Test Without Pushing

```bash
# Simulate pre-push (won't actually push)
echo "refs/heads/main $(git rev-parse HEAD) refs/heads/main 0000000000000000000000000000000000000000" | \
  .git/hooks/pre-push origin github.com:roder/stroma.git
```

### Test with Green CI

```bash
# Should pass with ✅ message
git push origin main --dry-run
```

### Test Emergency Bypass

```bash
# Should skip all hooks
git push origin main --no-verify --dry-run
```

## Behavior

### When CI is Passing (success)

```
🔍 Checking CI status on main branch...
   Repository: roder/stroma
✅ CI is passing on main
```

Push proceeds normally.

### When CI is Running (pending)

```
🔍 Checking CI status on main branch...
   Repository: roder/stroma
⏳ CI is currently running on main
   Allowing push (check will complete asynchronously)
```

Push proceeds (CI will complete after push).

### When CI is Failing (failure/error)

```
🔍 Checking CI status on main branch...
   Repository: roder/stroma

❌ ERROR: CI is currently FAILING on main branch
   CI State: failure

Cannot push to main while CI is broken.

📋 To fix:
  1. File P0 bug: bd create --title='CI BROKEN: ...' --type=bug --priority=0
  2. Check failure: gh run list --branch=main --limit=5
  3. Fix or revert the breaking commit
  4. Verify CI passes before pushing

🚨 Emergency bypass: git push --no-verify
   (Use only for reverting the breaking commit or critical hotfixes)
```

Push is blocked with exit code 1.

### When gh CLI is Missing/Unauthenticated

```
⚠️  Warning: gh CLI not found, skipping CI check
   Install with: brew install gh
```

Push proceeds with warning (graceful degradation).

## Emergency Procedures

### When to Use `--no-verify`

**ONLY use in these scenarios:**
- ✅ Reverting a commit that broke CI
- ✅ Critical security hotfix
- ✅ CI infrastructure is broken (pushing fix to workflows)
- ✅ Confirmed false positive

**NEVER use to:**
- ❌ Skip failing tests
- ❌ Bypass coverage requirements
- ❌ Work around flaky tests
- ❌ Save time

### Bypassing the Hook

```bash
# Emergency bypass
git push --no-verify

# Always document why
git commit --amend -m "$(git log -1 --pretty=%B)

Emergency bypass: [reason]"
```

## Troubleshooting

### Hook Not Running

```bash
# Check if executable
ls -l .git/hooks/pre-push

# Make executable if needed
chmod +x .git/hooks/pre-push

# Test directly
.git/hooks/pre-push origin test-url < /dev/null
```

### "gh CLI not found"

```bash
# Install gh
brew install gh

# Authenticate
gh auth login

# Verify
gh auth status
```

### "Could not determine repository"

```bash
# Check git remote
git remote -v

# Verify gh can see repo
gh repo view
```

### False Positive (CI actually passing)

```bash
# Check CI status manually
gh run list --branch=main --limit=5

# If actually passing, may be API lag
# Wait 30 seconds and try again

# Or check on GitHub web UI
gh repo view --web
```

## Maintenance

### Updating the Hook

When the hook template is updated:

```bash
# Re-install from template
cp scripts/hooks/pre-push .git/hooks/pre-push

# Verify changes
diff scripts/hooks/pre-push .git/hooks/pre-push
```

### Disabling the Hook Temporarily

```bash
# Rename to disable
mv .git/hooks/pre-push .git/hooks/pre-push.disabled

# Re-enable
mv .git/hooks/pre-push.disabled .git/hooks/pre-push
```

## Integration with Beads

The hook calls `bd hooks run pre-push` first, which handles:
- Syncing issues.jsonl before push
- Checking for unstaged changes in beads files
- Validating beads database integrity

Then the CI status check runs after beads checks pass.

## See Also

- **CI/CD Protection Policy**: `docs/CI-CD-PROTECTION.md`
- **Patrol Scripts**: `scripts/patrol/README.md`
- **AGENTS.md**: Pre-push requirements section
