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

## Core Standards, Philosophy, and Security Requiresments

**MUST READ**:

* `.beads/core-standards.bead`
* `.beads/philosophical-foundations.bead`
* `.beads/rust-standards.bead`
* `.beads/git-standards.bead`
* `.beads/security-requirements.bead`
* `.beads/documentation-workflow.bead`
* `.beads/rust-async.bead`
* `.beads/testing-standards.bead`
* `.beads/terminology.bead`


## CI/CD Green Branch Protection

**CRITICAL**: All PRs to `main` are automatically blocked.

**ABSOLUTE REQUIREMENT**: Main branch CI/CD status MUST be ✅ passing at all times.

### Before Pushing Code:

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

**Format Requirements**:
- Blank line before Co-authored-by
- Use HEREDOC for multi-line commit messages

**Example**:
```bash
git commit -m "$(cat <<'EOF'
Add HMAC identity masking with zeroization

- Implement HMAC-SHA256 with group-secret pepper
- Add immediate zeroization of sensitive buffers
- Add unit tests with fixed test pepper

EOF
)"
```
**See**: `.beads/git-standards.bead` for complete git standards.

## Additional Resources

- **Security Requirements**: `docs/SECURITY-CI-CD.md` - Complete security workflow and fix patterns
- **Technology Stack**: `.beads/technology-stack.bead` - Technology decisions and patterns
- **Architecture Decisions**: `.beads/*.bead` has specific information and context about decision making, objectives, architecture, goals, and constraints. 
- **Issue Tracking**: Run `bd onboard` for beads workflow introduction

