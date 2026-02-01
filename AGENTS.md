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

**See**: `SECURITY-CI-CD.md` for complete security requirements and fix patterns.

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

- **Security Requirements**: `SECURITY-CI-CD.md` - Complete security workflow and fix patterns
- **Security Constraints**: `.beads/security-constraints.bead` - Immutable security policy
- **Technology Stack**: `.beads/technology-stack.bead` - Technology decisions and patterns
- **Issue Tracking**: Run `bd onboard` for beads workflow introduction

