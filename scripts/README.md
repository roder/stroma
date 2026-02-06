# Gastown CI/CD Protection Scripts

Generic, repository-agnostic scripts for CI/CD green branch protection across all gastown rigs.

## Philosophy

These scripts live at the **gastown workspace level** (not in individual rigs) because:
- ✅ CI/CD protection is a gastown workflow concern, not application-specific
- ✅ Works with any GitHub repository automatically
- ✅ Single source of truth (update once, applies everywhere)
- ✅ No gastown-specific code polluting application repositories

## Directory Structure

```
$HOME/gt/scripts/
├── hooks/
│   └── pre-push              # Pre-push hook (Layer 1 quality gates + CI check)
├── patrol/
│   └── ci-failures-sync.sh   # Mayor patrol: sync GitHub issues to beads
├── refinery-validate.sh      # Refinery validation script (Layer 2 quality gates)
├── install-hooks.sh          # Install hooks for all repos (bash)
├── install-hooks.fish        # Install hooks for all repos (fish shell)
└── README.md                 # This file
```

## Defense in Depth Architecture

**Layer 1**: Pre-push hook validates quality before ANY push
**Layer 2**: Refinery validates quality before merge to main

This two-layer approach ensures main stays green while allowing iteration speed.

## Components

### 1. Pre-Push Hook (`hooks/pre-push`) - Layer 1

**Purpose**: Enforces quality gates on ALL pushes and blocks pushes to main when CI is failing

**Features**:
- **Quality gates for ALL pushes** (Rust projects - Layer 1 defense):
  - `cargo fmt --check` - formatting verification
  - `cargo clippy --all-targets --all-features -- -D warnings` - lint checks
  - `cargo nextest run --all-features` - test suite (or `cargo test` fallback)
- Auto-detects repository via `gh repo view`
- Works with any GitHub repository
- Checks CI status before push to main
- Graceful degradation if tools unavailable
- **WIP escape hatches**:
  - `git push --no-verify` (skip all checks)
  - `export GASTOWN_SKIP_QUALITY_GATES=1` (skip quality gates, keep CI check)

**Defense in Depth**:
- Layer 1: Pre-push quality gates (this hook)
- Layer 2: Refinery validates before merge to main

**Installation**:
```bash
# Install for all gastown repos
scripts/install-hooks.sh

# Install for specific repo
scripts/install-hooks.sh /path/to/repo
```

### 2. Refinery Validation (`refinery-validate.sh`) - Layer 2

**Purpose**: Final quality gate validation before merging to main

**Features**:
- Verifies branch is up-to-date with main
- Runs same quality gates as Layer 1:
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo nextest run --all-features`
  - `cargo llvm-cov nextest --all-features` (coverage check)
  - `cargo deny check` (supply chain security)
- Safety net for code pushed with `--no-verify`
- Catches merge conflicts resolved in refinery
- Clear pass/fail output with remediation instructions

**Usage**:
```bash
# Checkout branch to validate
git checkout merge-queue/feature-xyz

# Run validation
$HOME/gt/scripts/refinery-validate.sh

# If passes, safe to merge
git checkout main
git merge merge-queue/feature-xyz
git push
```

**See**: `docs/REFINERY-QUALITY-GATES.md` for full documentation

### 3. CI Failures Sync (`patrol/ci-failures-sync.sh`)

**Purpose**: Syncs GitHub CI failure issues to beads database

**Features**:
- Auto-discovers all GitHub repos in gastown
- Polls for issues labeled `ci-failure,auto-alert`
- Creates P0 beads issues locally
- Sends notifications to crew-approvals
- Works with multiple repositories simultaneously

**Setup**:
```bash
# Add to crontab (runs every 5 minutes)
crontab -e
# Add:
*/5 * * * * $HOME/gt/scripts/patrol/ci-failures-sync.sh >> $HOME/gt/logs/patrol-ci-sync.log 2>&1
```

## Usage

### Install Hooks

```bash
# Install for all repos in gastown (bash)
cd $HOME/gt
./scripts/install-hooks.sh

# Or if you use fish shell:
fish ./scripts/install-hooks.fish
```

This copies the centralized hook to each repo's `.git/hooks/pre-push`.

**Available scripts**:
- `install-hooks.sh` - Bash version
- `install-hooks.fish` - Fish shell version

**Why copy instead of symlink?**
- Works on all developer machines (no hardcoded paths)
- Works in CI/CD environments (GitHub Actions, etc.)
- No dependency on gastown workspace structure
- Portable across different users and systems

### Test Hook

```bash
# From any gastown repo with GitHub remote
cd $HOME/gt/stromarig/mayor/rig
git push --dry-run  # Hook will check CI status
```

### Run Patrol Manually

```bash
# Test the patrol script
$HOME/gt/scripts/patrol/ci-failures-sync.sh
```

## Repository-Specific Files

Individual repositories only need:
- `.github/workflows/ci-monitor.yml` - GitHub Actions workflow (repository-specific)

Everything else is centralized at gastown level.

## How It Works

### Pre-Push Hook Flow
```
Developer: git push origin main
           ↓
Pre-Push Hook (gastown/scripts/hooks/pre-push)
           ↓
1. Run bd hooks (beads sync)
2. Detect current repository (gh repo view)
3. Check CI status (gh api)
4. Block if CI red, allow if green
```

### CI Monitoring Flow
```
CI Fails on main
        ↓
GitHub Actions (repo/.github/workflows/ci-monitor.yml)
        ↓
Creates GitHub issue (label: ci-failure, auto-alert)
        ↓
Mayor Patrol (gastown/scripts/patrol/ci-failures-sync.sh)
        ↓
1. Scans all gastown repos
2. Finds ci-failure issues
3. Creates P0 beads issue
4. Sends notification
5. Removes auto-alert label
```

## Requirements

- **gh CLI**: `brew install gh && gh auth login`
- **bd CLI**: `brew install beads`
- **jq**: `brew install jq` (for patrol script)

## Maintenance

### Update Hooks for All Repos

Hooks are copied (not symlinked), so after editing the centralized hook, re-run the installer:
```bash
# 1. Edit centralized hook
vim $HOME/gt/scripts/hooks/pre-push

# 2. Re-install to all repos (overwrites existing)
$HOME/gt/scripts/install-hooks.sh

# Or fish:
fish $HOME/gt/scripts/install-hooks.fish
```

### Add New Repository

New repos get hooks automatically on next install run:
```bash
$HOME/gt/scripts/install-hooks.sh
```

Or install manually:
```bash
# Copy hook to new repo
cp $HOME/gt/scripts/hooks/pre-push <repo>/.git/hooks/pre-push
chmod +x <repo>/.git/hooks/pre-push
```

## Troubleshooting

### Hook Not Running

```bash
# Check if hook exists and is executable
ls -l <repo>/.git/hooks/pre-push
# Should show: -rwxr-xr-x and file should exist

# Re-install if missing or not executable
$HOME/gt/scripts/install-hooks.sh <repo>
```

### Patrol Not Finding Repos

```bash
# Check GASTOWN_ROOT
echo $GASTOWN_ROOT  # Should be $HOME/gt

# Manually test repo discovery
find $HOME/gt -name ".git" -type d -maxdepth 6
```

### gh CLI Issues

```bash
# Verify gh is authenticated
gh auth status

# Re-authenticate if needed
gh auth login
```

## See Also

- **CI/CD Protection Policy**: Individual repo `docs/CI-CD-PROTECTION.md`
- **AGENTS.md**: Pre-push requirements section
- **Issue Tracking**: hq-diq31 (CI/CD Green Branch Protection epic)
