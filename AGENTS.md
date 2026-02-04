# Agent Instructions

This project uses **bd** (beads) for issue tracking. Run `bd onboard` to get started.

## Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --status in_progress  # Claim work
bd close <id>         # Complete work
bd sync               # Sync with git
```

## Security CI/CD Requirements

**CRITICAL**: All PRs to `main` are automatically blocked if they violate security constraints.

**Before committing code, you MUST run:**

```bash
# Quick security check (2-3 minutes)
cargo fmt --check              # Format verification
cargo clippy --all-targets --all-features -- -D warnings  # Lint
cargo deny check               # Supply chain security

# Full security check (5-10 minutes)
cargo llvm-cov nextest --all-features  # 100% coverage required
cargo build --release --target x86_64-unknown-linux-musl  # Binary size check
```

**Security Constraints (Auto-Reject)**:
- ❌ No cleartext Signal IDs (use `mask_identity()` + zeroize)
- ❌ No `presage-store-sqlite` (use `StromaProtocolStore`)
- ❌ No grace periods in ejection logic
- ❌ Unsafe blocks MUST have `// SAFETY:` comments
- ❌ Test coverage MUST be 100%

**See**: `docs/SECURITY-CI-CD.md` for complete security requirements and fix patterns.

## CI/CD Green Branch Protection

**ABSOLUTE REQUIREMENT**: Main branch CI/CD status MUST be ✅ passing at all times.

### Before Pushing to Main:

1. **Verify local quality gates pass:**
   ```bash
   cargo fmt --check
   cargo clippy --all-targets --all-features -- -D warnings
   cargo nextest run --all-features
   cargo llvm-cov nextest --all-features  # 100% coverage required
   cargo deny check
   ```

2. **Check remote CI status before merge:**
   ```bash
   # If pushing to PR:
   gh pr checks --watch

   # Before merging to main, verify main is currently green:
   gh api repos/$(gh repo view --json nameWithOwner -q .nameWithOwner)/commits/main/status --jq '.state'
   # Must return "success" before merging
   ```

3. **If CI fails on main:**
   - **IMMEDIATE**: File P0 bug: `bd create --title="CI BROKEN: <description>" --type=bug --priority=0`
   - Notify mayor: `gt mail send mayor/ -s "🚨 CI BROKEN"`
   - **DO NOT push more code** until main is green
   - Fix or revert the breaking commit immediately

### CI/CD Infrastructure Changes (Human Authorization Required)

**ABSOLUTE RULE**: Changes to CI/CD infrastructure REQUIRE human authorization.

**Protected Files** (Cannot modify without approval):
- `.github/workflows/*.yml` (All workflow files)
- `.github/actions/*` (Custom actions)
- `.github/codeql/codeql-config.yml` (CodeQL config)
- `deny.toml` (Dependency policy)
- Git hooks (if modifying hook logic, not just beads updates)

**Process for CI/CD Changes**:
1. Identify CI/CD bug or improvement need
2. File issue: `bd create --title="CI/CD: <description>" --type=bug --priority=1`
3. Document proposed change in issue description
4. Request human review: `gt mail send crew-approvals -s "CI/CD Change Request: <issue-id>"`
5. **WAIT for human approval** before modifying workflow files
6. After approval: Make changes, test in branch, submit PR
7. Human must approve PR before merge

**Note**: `crew-approvals` is a mail group that routes to all crew members (`*/crew/*`)

**What qualifies as CI/CD infrastructure**:
- GitHub Actions workflow definitions
- Job configurations, dependencies, triggers
- Security check configurations (CodeQL, cargo-deny)
- Coverage thresholds or enforcement logic
- Binary size baselines or limits
- Any automation that gates merges

**What does NOT require authorization** (can be changed by agents):
- Source code that causes CI failures (fix the code, not the CI)
- Test code to achieve coverage
- Documentation in docs/ folder
- Dependency updates (as long as deny.toml allows)

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - **Security checks MANDATORY** (see above)
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd sync
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds

## Git Commit Standards

**When creating commits as an AI agent**, you MUST follow these standards:

### Co-Authorship Attribution (MANDATORY)
**CRITICAL**: All commits authored by Claude MUST include Co-authored-by trailer:

```bash
git commit -m "$(cat <<'EOF'
Commit message title

Detailed description of changes...

Co-authored-by: Claude <noreply@anthropic.com>
EOF
)"
```

**Format Requirements**:
- Co-authored-by line at the END of commit message
- Blank line before Co-authored-by
- Exact format: `Co-authored-by: Claude <noreply@anthropic.com>`
- Use HEREDOC for multi-line commit messages

**Example**:
```bash
git commit -m "$(cat <<'EOF'
Add HMAC identity masking with zeroization

- Implement HMAC-SHA256 with group-secret pepper
- Add immediate zeroization of sensitive buffers
- Add unit tests with fixed test pepper

Co-authored-by: Claude <noreply@anthropic.com>
EOF
)"
```

**See**: `.cursor/rules/git-standards.mdc` for complete standards.

## Additional Resources

- **Security Requirements**: `docs/SECURITY-CI-CD.md` - Complete security workflow and fix patterns
- **Security Constraints**: `.beads/security-constraints.bead` - Immutable security policy
- **Technology Stack**: `.beads/technology-stack.bead` - Technology decisions and patterns
- **Issue Tracking**: Run `bd onboard` for beads workflow introduction

