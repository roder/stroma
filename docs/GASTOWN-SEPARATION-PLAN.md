# Gas Town Separation Plan (Model B)

**Goal**: Separate Gas Town instance from Stroma project repo while maintaining beads as architectural documentation.

**Model**: Gas Town workspace at `~/gastown/stroma/` with project beads remaining in project repo.

---

## 1. Current State

### What's Currently in the Stroma Repo

**Project Code** (should stay):
- `/src/` - Stroma Rust source code
- `/daemon/` - Gas Town daemon (runtime state)
- `/stroma/` - Additional Stroma code
- `/plugins/` - Gas Town plugins
- `/docs/` - Project documentation
- `/Cargo.toml`, `/Cargo.lock` - Rust project files
- `/README.md`, `/AGENTS.md` - Project documentation

**Gas Town Infrastructure** (should move):
- `/mayor/` - Mayor rig (git worktree/clone)
- `/polecats/` - Worker polecats (git worktrees)
  - `polecats/chrome/`
  - `polecats/nitro/`
  - `polecats/rust/`
- `/refinery/` - Refinery rig (git clone)
- `/witness/` - Witness (runtime state)
- `/crew/` - User-managed workspaces

**Beads (architectural documentation)** (should stay):
- `/.beads/*.bead` - 23 architectural constraint documents
- `/.beads/issues.jsonl` - Team collaboration issue tracker (206KB)
- `/.beads/formulas/` - 33 formula files
- `/.beads/.gitignore` - Runtime file exclusions
- `/.beads/config.yaml` - Beads configuration
- `/.beads/metadata.json` - Beads metadata

**Gas Town Runtime** (ignored, stays local):
- `/.beads/beads.db*` - SQLite database (520KB + WAL/SHM)
- `/.beads/bd.sock` - Daemon socket
- `/.beads/daemon.lock`, `daemon.pid`, `daemon.log` - Daemon runtime
- `/.runtime/` - Runtime state directory
- `/.logs/` - Log files

**Other Files**:
- `/.events.jsonl`, `/.feed.jsonl` - Event tracking (runtime)
- `/logs/` - Log directory
- `/target/` - Rust build artifacts (already ignored)

### Current Git Status

```
M .beads/issues.jsonl
M .beads/routes.jsonl
M Cargo.toml
?? .events.jsonl
?? daemon/
?? docs/spike/q7/RESULTS.md
?? docs/spike/q7/main.rs
?? docs/spike/q7/registry.rs
?? plugins/
?? stroma/
```

**Problem**: Gas Town infrastructure (mayor/, polecats/, refinery/, witness/) is tracked in `.gitignore` but physically present in project directory, polluting the workspace for external contributors.

---

## 2. Target State

### Project Repo (`/Users/matt/src/github.com/roder/stroma/`)

**Tracked in Git:**
- All Stroma source code (`/src/`, `/daemon/`, `/stroma/`, `/plugins/`)
- Project documentation (`/docs/`, `/README.md`, `/AGENTS.md`)
- Build configuration (`/Cargo.toml`, `/Cargo.lock`)
- `.beads/*.bead` files (architectural constraints)
- `.beads/issues.jsonl` (team collaboration)
- `.beads/formulas/` (project-specific formulas)
- `.beads/.gitignore` (runtime file exclusions)
- `.beads/config.yaml`, `.beads/metadata.json` (beads config)

**Ignored (never committed):**
- Gas Town infrastructure directories (mayor/, polecats/, refinery/, witness/, crew/)
- `.beads/beads.db*` (runtime database)
- `.beads/*.sock`, `.beads/daemon.*`, `.beads/sync-state.json` (runtime)
- `.runtime/`, `.logs/`, `logs/` (runtime state)
- `.events.jsonl`, `.feed.jsonl` (runtime events)
- `.gastown/` (Gas Town workspace config)

### Gas Town Workspace (`~/gastown/stroma/`)

**Structure:**
```
~/gastown/stroma/
├── mayor/
│   └── rig/           # Git clone of mayor rig
├── polecats/
│   ├── chrome/        # Git worktree
│   ├── nitro/         # Git worktree
│   └── rust/          # Git worktree
├── refinery/
│   └── rig/           # Git clone of refinery rig
├── witness/
│   └── .runtime/      # Runtime state
├── crew/              # User-managed workspaces
└── .gastown/
    └── config.yaml    # Gas Town configuration
```

**Configuration:**
- `.gastown/config.yaml` points to project `.beads/` directory
- All polecats configured to use project `.beads/` for beads access
- Mayor configured to work with project issues

---

## 3. Migration Steps

### Step 1: Create Gas Town Workspace

```bash
# Create workspace directory
mkdir -p ~/gastown/stroma/

# Create Gas Town configuration
mkdir -p ~/gastown/stroma/.gastown
cat > ~/gastown/stroma/.gastown/config.yaml <<EOF
# Gas Town workspace configuration
workspace_name: stroma
project_path: /Users/matt/src/github.com/roder/stroma
beads_path: /Users/matt/src/github.com/roder/stroma/.beads
EOF
```

### Step 2: Move Gas Town Infrastructure

```bash
cd /Users/matt/src/github.com/roder/stroma

# Move mayor rig
mv mayor ~/gastown/stroma/

# Move polecats
mv polecats ~/gastown/stroma/

# Move refinery rig
mv refinery ~/gastown/stroma/

# Move witness
mv witness ~/gastown/stroma/

# Move crew (if exists)
if [ -d crew ]; then
  mv crew ~/gastown/stroma/
fi
```

### Step 3: Update Project .gitignore

Add comprehensive Gas Town exclusions to `/Users/matt/src/github.com/roder/stroma/.gitignore`:

```bash
cat >> .gitignore <<EOF

# =============================================================================
# Gas Town Infrastructure (Model B: Workspace Separation)
# =============================================================================
# These directories live in ~/gastown/stroma/ and should NEVER be in project repo

# Gas Town workspace configuration
.gastown/

# Gas Town infrastructure (moved to ~/gastown/stroma/)
mayor/
polecats/
refinery/
witness/
crew/

# Gas Town runtime files (if accidentally created in project)
.events.jsonl
.feed.jsonl
logs/

# Daemon runtime (moved to daemon/ but ignored)
daemon/

# Additional Stroma infrastructure (if not needed in repo)
stroma/
plugins/

EOF
```

### Step 4: Configure Gas Town to Use Project Beads

**Option A: Symlink from Gas Town workspace** (recommended):
```bash
cd ~/gastown/stroma
ln -s /Users/matt/src/github.com/roder/stroma/.beads .beads
```

**Option B: Configure each polecat** (if symlink doesn't work):
```bash
# In each polecat's configuration, set beads path
for polecat in ~/gastown/stroma/polecats/*; do
  if [ -f "$polecat/.claude/config.yaml" ]; then
    echo "beads_path: /Users/matt/src/github.com/roder/stroma/.beads" >> "$polecat/.claude/config.yaml"
  fi
done
```

### Step 5: Update Mayor Configuration

```bash
# Update mayor/rig/.beads symlink to point to project .beads
cd ~/gastown/stroma/mayor/rig
rm -f .beads  # Remove old symlink if exists
ln -s /Users/matt/src/github.com/roder/stroma/.beads .beads
```

### Step 6: Verify Beads Accessibility

```bash
# Test beads access from Gas Town workspace
cd ~/gastown/stroma
ls -la .beads/*.bead

# Test beads access from polecats
cd ~/gastown/stroma/polecats/rust
# (Run whatever command polecats use to access beads)

# Verify project repo still has beads
cd /Users/matt/src/github.com/roder/stroma
ls -la .beads/*.bead
```

### Step 7: Clean Up Project Repo

```bash
cd /Users/matt/src/github.com/roder/stroma

# Remove any leftover Gas Town files
rm -f .events.jsonl .feed.jsonl

# Verify gitignore is working
git status

# Expected: mayor/, polecats/, refinery/, witness/, crew/ should NOT appear
```

### Step 8: Update Documentation

Update project README and AGENTS.md to reflect new structure:

```bash
# Add note to README.md about Gas Town separation
cat >> README.md <<EOF

## Development Environment

This project uses Gas Town for AI agent coordination. Gas Town infrastructure lives in
a separate workspace at \`~/gastown/stroma/\` to keep the project repo clean for external
contributors.

**For developers using Gas Town:**
- Gas Town workspace: \`~/gastown/stroma/\`
- Beads (architectural docs): \`.beads/\` in project repo
- See [GASTOWN-SEPARATION-PLAN.md](docs/GASTOWN-SEPARATION-PLAN.md) for setup details

EOF
```

---

## 4. What Gets Tracked in Project Repo

### Architectural Documentation (.beads/)

**Tracked (committed to git):**
- `*.bead` files (23 architectural constraint documents)
  - Example: `architecture-decisions.bead`, `technology-stack.bead`, `security-constraints.bead`
- `issues.jsonl` - Team collaboration issue tracker (if used for project-wide issues)
- `formulas/` - Project-specific formula files (33 formulas)
- `.gitignore` - Beads runtime file exclusions
- `config.yaml` - Beads configuration
- `metadata.json` - Beads metadata
- `PRIME.md`, `README.md` - Beads documentation
- `routes.jsonl` - Routing configuration (if project-specific)

**Why track these?**
- Beads serve as **architectural documentation** for the project
- `*.bead` files define **immutable design constraints** that guide development
- `issues.jsonl` tracks **team collaboration** on architectural decisions
- External contributors benefit from reading beads to understand design rationale
- Beads are **human-readable markdown** (not binary), suitable for version control

### Project Code

**Tracked:**
- All source code (`/src/`, `/daemon/`, `/stroma/`, `/plugins/`)
- Documentation (`/docs/`, `/README.md`, `/AGENTS.md`)
- Build configuration (`/Cargo.toml`, `/Cargo.lock`)
- Tests, examples, benchmarks

---

## 5. What Gets Ignored in Project Repo

### Gas Town Infrastructure

**Ignored (NEVER committed):**
```gitignore
# Gas Town workspace configuration
.gastown/

# Gas Town infrastructure (moved to ~/gastown/stroma/)
mayor/
polecats/
refinery/
witness/
crew/
```

**Why ignore?**
- These are **operational infrastructure** for Gas Town coordination
- Polecats are **git worktrees** that shouldn't be nested in project repo
- Mayor and refinery are **git clones** of other repos
- External contributors don't need Gas Town to build/run Stroma
- Keeps project repo focused on **project code**, not tooling

### Beads Runtime Files

**Ignored (via `.beads/.gitignore`):**
```gitignore
# SQLite databases (runtime state, NOT architectural docs)
*.db
*.db-journal
*.db-wal
*.db-shm

# Daemon runtime files (transient)
daemon.lock
daemon.log
daemon.pid
bd.sock
sync-state.json
last-touched

# Local version tracking (per-machine)
.local_version

# Worktree redirect file (per-clone)
redirect

# Merge artifacts (temporary)
beads.base.jsonl
beads.left.jsonl
beads.right.jsonl
```

**Why ignore?**
- `beads.db` is a **SQLite runtime database** (520KB), not documentation
- Daemon files are **transient** and machine-specific
- Sync state is **per-clone**, would conflict across developers
- These files are **generated** from the tracked `.bead` and `.jsonl` files

### Other Runtime Files

**Ignored (via project `.gitignore`):**
```gitignore
# Runtime state
.runtime/
.logs/
logs/
.events.jsonl
.feed.jsonl

# Build artifacts (Rust)
/target
```

---

## 6. Benefits of Separation

### For External Contributors

1. **Clean repo checkout**
   - No Gas Town infrastructure cluttering workspace
   - Only project code and documentation visible
   - Standard Rust project structure (`cargo build` just works)

2. **No Gas Town dependencies**
   - Don't need to install or understand Gas Town
   - Can contribute without AI agent coordination system
   - Build, test, and run Stroma independently

3. **Clear architectural docs**
   - `.beads/*.bead` files serve as **design documentation**
   - Understand design rationale without running Gas Town
   - Track architectural evolution through git history

### For Gas Town Users

1. **Dedicated workspace**
   - Gas Town infrastructure in `~/gastown/stroma/`
   - Can manage multiple projects from single Gas Town instance
   - Clear separation between **project** and **tooling**

2. **Shared beads access**
   - Polecats access project `.beads/` via symlink
   - Mayor and refinery read beads from project repo
   - Single source of truth for architectural constraints

3. **No git conflicts**
   - Gas Town infrastructure changes don't trigger git status
   - Can update polecats without affecting project repo
   - Cleaner git workflow for project contributions

### For Project Maintainers

1. **Beads as living documentation**
   - Track architectural decisions in `.bead` files
   - Use `issues.jsonl` for team-wide architectural discussions
   - Evolution visible in git history

2. **Flexible tooling**
   - Gas Town is **optional** for contributors
   - Can switch to different AI coordination tools
   - Project isn't locked into specific workflow

3. **Cleaner git history**
   - Project commits focus on **code changes**
   - No Gas Town infrastructure noise
   - Easier code review for external PRs

---

## 7. Testing the Separation

### Test 1: Clean Checkout

```bash
# Simulate external contributor experience
cd /tmp
git clone /Users/matt/src/github.com/roder/stroma stroma-clean
cd stroma-clean

# Verify clean workspace
ls -la
# Should NOT see: mayor/, polecats/, refinery/, witness/, crew/
# SHOULD see: src/, docs/, .beads/, Cargo.toml, README.md

# Verify beads are present
ls -la .beads/*.bead
# Should see 23 .bead files

# Verify build works
cargo build
# Should compile without errors
```

### Test 2: Beads Access from Gas Town

```bash
# Test beads access from Gas Town workspace
cd ~/gastown/stroma

# Verify symlink works
ls -la .beads/*.bead
# Should show project beads via symlink

# Test polecat beads access
cd ~/gastown/stroma/polecats/rust
# Run polecat command that reads beads
# Should successfully access beads from project repo

# Test mayor beads access
cd ~/gastown/stroma/mayor/rig
ls -la .beads/*.bead
# Should show project beads via symlink
```

### Test 3: Beads Modification Workflow

```bash
# Modify a bead from project repo
cd /Users/matt/src/github.com/roder/stroma
echo "# New section" >> .beads/architecture-decisions.bead
git add .beads/architecture-decisions.bead
git commit -m "Update architecture decisions"

# Verify change visible from Gas Town workspace
cd ~/gastown/stroma
cat .beads/architecture-decisions.bead | tail -5
# Should show new section

# Modify issues.jsonl from Gas Town (via polecat)
cd ~/gastown/stroma/polecats/rust
# (Run polecat command that adds an issue)

# Verify change visible in project repo
cd /Users/matt/src/github.com/roder/stroma
git status
# Should show .beads/issues.jsonl as modified
```

### Test 4: Git Status Cleanliness

```bash
cd /Users/matt/src/github.com/roder/stroma

# Create Gas Town infrastructure in project (simulate accident)
mkdir -p mayor/rig
mkdir -p polecats/rust

# Check git status
git status
# Should NOT show mayor/ or polecats/ (ignored)

# Clean up
rm -rf mayor polecats
```

### Test 5: Build and Run

```bash
cd /Users/matt/src/github.com/roder/stroma

# Verify build works
cargo build --release

# Verify tests work
cargo test

# Verify beads are accessible to Stroma binary (if it uses them)
./target/release/stroma --help
# Should run without errors
```

### Test 6: Multi-Project Gas Town

```bash
# Test Gas Town managing multiple projects
mkdir -p ~/gastown/other-project
cd ~/gastown/other-project

# Configure to use different beads
cat > .gastown/config.yaml <<EOF
workspace_name: other-project
project_path: /path/to/other-project
beads_path: /path/to/other-project/.beads
EOF

# Verify stroma workspace still works
cd ~/gastown/stroma
ls -la .beads/*.bead
# Should still show stroma beads
```

---

## 8. Rollback Plan

If migration causes problems, rollback is straightforward:

```bash
cd /Users/matt/src/github.com/roder/stroma

# Move Gas Town infrastructure back
mv ~/gastown/stroma/mayor ./
mv ~/gastown/stroma/polecats ./
mv ~/gastown/stroma/refinery ./
mv ~/gastown/stroma/witness ./
mv ~/gastown/stroma/crew ./ 2>/dev/null || true

# Remove Gas Town workspace
rm -rf ~/gastown/stroma

# Revert .gitignore changes
git checkout .gitignore

# Verify workspace back to original state
git status
```

---

## 9. Migration Checklist

**Pre-Migration:**
- [ ] Commit any uncommitted changes in project repo
- [ ] Backup `.beads/` directory: `cp -r .beads .beads.backup`
- [ ] Verify Gas Town is not running: `ps aux | grep gastown`
- [ ] Document current polecat configurations

**Migration:**
- [ ] Create `~/gastown/stroma/` workspace directory
- [ ] Create `.gastown/config.yaml` with project paths
- [ ] Move `mayor/` to `~/gastown/stroma/mayor/`
- [ ] Move `polecats/` to `~/gastown/stroma/polecats/`
- [ ] Move `refinery/` to `~/gastown/stroma/refinery/`
- [ ] Move `witness/` to `~/gastown/stroma/witness/`
- [ ] Move `crew/` to `~/gastown/stroma/crew/` (if exists)
- [ ] Update project `.gitignore` with Gas Town exclusions
- [ ] Create symlink: `~/gastown/stroma/.beads -> project/.beads`
- [ ] Update mayor/rig `.beads` symlink
- [ ] Clean up project repo (remove `.events.jsonl`, `.feed.jsonl`)

**Post-Migration Testing:**
- [ ] Test: Clean checkout builds successfully
- [ ] Test: Beads accessible from Gas Town workspace
- [ ] Test: Polecats can read/write beads
- [ ] Test: Mayor can access issues
- [ ] Test: Git status shows no Gas Town infrastructure
- [ ] Test: Project builds and runs correctly
- [ ] Test: Beads modifications sync between project and Gas Town

**Documentation:**
- [ ] Update README.md with Gas Town workspace location
- [ ] Update AGENTS.md with new structure
- [ ] Commit migration changes to project repo
- [ ] Update polecat documentation (if any)

---

## 10. Future Considerations

### Phase 2: Full Workspace Abstraction

In the future, consider enhancing Gas Town to support multiple projects more elegantly:

```
~/gastown/
├── workspaces/
│   ├── stroma/
│   │   ├── mayor/
│   │   ├── polecats/
│   │   └── .gastown/
│   └── other-project/
│       ├── mayor/
│       └── polecats/
├── global/
│   ├── config.yaml      # Global Gas Town config
│   └── shared-rigs/     # Shared rig repositories
└── bin/
    └── gt                # Gas Town CLI
```

This would allow:
- Managing multiple projects from single Gas Town installation
- Sharing rigs across projects
- Centralized Gas Town configuration
- Project-specific polecat configurations

### Phase 3: Beads as Submodule

For projects with shared beads across multiple repos:

```bash
# In project repo
git submodule add https://github.com/org/shared-beads.git .beads

# In Gas Town workspace
cd ~/gastown/stroma
ln -s /Users/matt/src/github.com/roder/stroma/.beads .beads
```

This allows:
- Shared architectural constraints across multiple projects
- Centralized bead management
- Versioned beads (git submodule tracking)

---

## 11. FAQ

**Q: Will external contributors see beads in the repo?**

A: Yes! Beads are **intentionally tracked** as architectural documentation. They're human-readable markdown files that explain design constraints and rationale. This helps contributors understand the project's architecture.

**Q: Why not move beads to Gas Town workspace too?**

A: Beads serve as **project documentation**, not Gas Town tooling. They should be version-controlled with the project so contributors can read them without installing Gas Town.

**Q: What if a polecat modifies issues.jsonl?**

A: Changes sync immediately because Gas Town workspace symlinks to project `.beads/`. The modification appears in `git status` in the project repo, ready to commit.

**Q: Can I still use Gas Town without this migration?**

A: Yes! This is **optional**. The migration improves repo cleanliness for external contributors but doesn't change Gas Town functionality.

**Q: What if I want beads private (not tracked in git)?**

A: Add `.beads/` to `.gitignore` and move beads to Gas Town workspace. However, you lose the benefit of beads as **architectural documentation** for contributors.

**Q: How do I share beads between multiple Gas Town projects?**

A: Use symlinks from each Gas Town workspace to the respective project's `.beads/` directory. Or create a shared beads repo and symlink all projects to it.

---

## 12. Summary

**Model B** separates Gas Town infrastructure from the project repo while keeping beads as architectural documentation.

**Key Principles:**
1. **Gas Town infrastructure** → `~/gastown/stroma/` (mayor, polecats, refinery, witness)
2. **Beads** → `.beads/` in project repo (architectural docs, tracked in git)
3. **Gas Town workspace** → Symlink to project `.beads/` for access
4. **Project repo** → Ignore Gas Town directories, track beads

**Benefits:**
- Clean repo for external contributors
- Beads serve as living architectural documentation
- Gas Town can manage multiple projects
- No git noise from Gas Town operations

**Next Steps:**
1. Review this plan
2. Decide whether to proceed with migration
3. If yes, follow migration checklist (Section 9)
4. Test thoroughly (Section 7)
5. Update team documentation

---

**Status**: Plan complete, awaiting approval to execute migration.

**Author**: Polecat (Claude Sonnet 4.5)
**Date**: 2026-02-01
