# Gas Town Repository Reorganization - Analysis Report

**Date**: 2026-01-31
**Bead**: hq-reorg-1
**Analyst**: Mayor

---

## Executive Summary

The Stroma repository has a **nested Gas Town structure** where the functioning Gas Town infrastructure (`/stroma`) was created inside an existing project repository (`/`). This creates confusion about the canonical location of beads, agents, and project structure.

**Key Finding**: A symlink already exists (`.beads -> stroma/.beads`) showing an attempt to bridge the gap, but the full reorganization is needed.

**Recommendation**: **Flatten the structure** by moving Gas Town infrastructure to repository root and properly organizing worktrees.

---

## Current Structure Analysis

### 1. Directory Layout

```
/Users/matt/src/github.com/roder/stroma/           (Git repo root - 69aa858 [main])
│
├── .beads -> stroma/.beads                         ⚠️ SYMLINK (points to nested)
├── .cursor/                                        ✅ ROOT (has 22 .mdc rules)
│   ├── plans/                                      ⚠️ Contains old plan to delete
│   └── rules/                                      ✅ 22 .mdc files
├── docs/                                           ✅ ROOT (1.1M, 26 spike docs)
│   └── spike/                                      ✅ Q1-Q14 results
│
├── mayor/                                          ⚠️ ROOT but minimal
│   └── .claude/settings.json
├── daemon/                                         ✅ ROOT (active)
├── deacon/                                         ✅ ROOT (active)
├── logs/                                           ✅ ROOT
│
├── Cargo.toml                                      ✅ ROOT (Rust project)
├── README.md                                       ✅ ROOT
├── AGENTS.md                                       ✅ ROOT
│
└── stroma/                                         ⚠️ NESTED (11GB - MAIN GAS TOWN)
    ├── .beads/                                     ⚠️ REAL BEADS (3.9M)
    │   ├── *.bead                                  ✅ 20 beads
    │   ├── issues.jsonl                            ✅ Active
    │   ├── routes.jsonl                            ✅ Active
    │   └── interactions.jsonl                      ✅ Active
    │
    ├── mayor/                                      ⚠️ NESTED
    │   └── rig/
    │       └── stroma/.beads -> ../../.beads       🔗 Symlink
    │
    ├── polecats/                                   ⚠️ NESTED
    │   ├── chrome/rigs/stroma/
    │   │   └── stroma/.beads                       🔗 Nested deep
    │   ├── rust/rigs/stroma/
    │   │   └── stroma/.beads                       🔗 Nested deep
    │   └── nitro/rigs/stroma/
    │       └── stroma/.beads                       🔗 Nested deep
    │
    ├── refinery/                                   ⚠️ NESTED
    │   └── rig/.beads                              🔗 Symlink
    │
    ├── witness/                                    ⚠️ NESTED
    └── crew/                                       ⚠️ NESTED
```

### 2. Key Locations

| Resource | Current Location | Size | Status |
|----------|------------------|------|--------|
| **Active .beads** | `/stroma/.beads/` | 3.9M | ⚠️ Nested |
| **Root .beads** | `/.beads` → `stroma/.beads` | 0B (symlink) | ⚠️ Indirect |
| **.bead files** | `/stroma/.beads/*.bead` | 20 files | ✅ Present |
| **issues.jsonl** | `/stroma/.beads/issues.jsonl` | Active | ✅ Functional |
| **.mdc rules** | `/.cursor/rules/*.mdc` | 22 files | ✅ Root (good) |
| **docs/** | `/docs/` | 1.1M | ✅ Root (good) |
| **spike docs** | `/docs/spike/q*/` | 26 files | ✅ Root (good) |
| **Old plan** | `/.cursor/plans/gastown_workspace_setup_*.plan.md` | 24KB | ❌ To delete |

### 3. Git Worktree Analysis

**Current Worktrees**:
```
/Users/matt/src/github.com/roder/stroma  69aa858 [main]
```

**Finding**: Only ONE worktree listed (the main repo). This means:
- Polecats are using directory structures, not proper git worktrees
- OR worktrees exist but aren't properly registered
- This needs investigation in migration phase

### 4. .beads Distribution

**Locations found**:
1. `./stroma/.beads` ← **PRIMARY** (3.9M, 20 .bead files)
2. `./stroma/mayor/rig/stroma/.beads` ← Symlink to primary
3. `./stroma/polecats/chrome/stroma/stroma/.beads` ← **Double nested!**
4. `./stroma/polecats/nitro/stroma/stroma/.beads` ← **Double nested!**
5. `./stroma/polecats/rust/stroma/stroma/.beads` ← **Double nested!**
6. `./stroma/refinery/rig/.beads` ← Symlink to primary

**Problem**: Polecat worktrees have **double nesting** (`stroma/stroma/`), indicating incorrect worktree setup.

### 5. .cursor Distribution

**Locations found**:
1. `./.cursor` ← **PRIMARY** (372K, 22 .mdc rules) - GOOD
2. `./stroma/mayor/rig/.cursor` ← Likely symlink
3. `./stroma/polecats/chrome/stroma/.cursor` ← Copy or symlink
4. `./stroma/polecats/nitro/stroma/.cursor` ← Copy or symlink
5. `./stroma/polecats/rust/stroma/.cursor` ← Copy or symlink
6. `./stroma/refinery/rig/.cursor` ← Likely symlink

**Status**: Root .cursor is correct. Nested ones are for rig context access.

---

## Problems Identified

### 🔴 Critical Issues

1. **Nested Gas Town Infrastructure**
   - Primary Gas Town in `/stroma/` instead of `/`
   - Confusing: "Where is the real beads system?"
   - 11GB of nested structure

2. **Double-Nested Polecats**
   - Paths like `stroma/polecats/chrome/stroma/stroma/.beads`
   - Should be `polecats/chrome/rigs/stroma/.beads`
   - Indicates worktree creation happened inside nested structure

3. **Symlink Workaround**
   - `/.beads -> stroma/.beads` is a band-aid
   - Creates two "truth" locations
   - Confusing for new developers

### 🟡 Medium Issues

4. **No Git Worktrees Detected**
   - `git worktree list` shows only main repo
   - Polecats may not be proper worktrees
   - Needs investigation

5. **Old Plan File**
   - `.cursor/plans/gastown_workspace_setup_*.plan.md` (24KB)
   - Outdated, needs deletion

6. **Duplicate mayor/ Directory**
   - `/mayor/` at root (minimal)
   - `/stroma/mayor/` (full Gas Town structure)
   - Confusion about which is canonical

### 🟢 Minor Issues

7. **Size of Nested Directory**
   - 11GB in `/stroma/`
   - Mostly worktree checkouts and build artifacts
   - Can be cleaned during migration

---

## Target Structure Design

### Correct Gas Town Layout

```
/Users/matt/src/github.com/roder/stroma/           (Git repo root)
│
├── .beads/                                         ✅ MOVED FROM stroma/.beads
│   ├── *.bead                                      (20 files)
│   ├── issues.jsonl
│   ├── routes.jsonl
│   └── interactions.jsonl
│
├── .cursor/                                        ✅ KEEP (already correct)
│   └── rules/                                      (22 .mdc files)
│
├── docs/                                           ✅ KEEP (already correct)
│   └── spike/                                      (Q1-Q14 results)
│
├── src/                                            ✅ KEEP (Rust source)
├── Cargo.toml                                      ✅ KEEP
├── README.md                                       ✅ KEEP
├── AGENTS.md                                       ✅ KEEP
│
├── mayor/                                          ✅ REORGANIZE
│   └── rigs/
│       └── stroma/                                 (Worktree for mayor)
│           ├── .beads -> ../../.beads              (Symlink to root)
│           ├── .cursor -> ../../.cursor            (Symlink to root)
│           └── docs -> ../../docs                  (Symlink to root)
│
├── polecats/                                       ✅ MOVE FROM stroma/polecats
│   ├── chrome/
│   │   └── rigs/
│   │       └── stroma/                             (Worktree for chrome)
│   ├── rust/
│   │   └── rigs/
│   │       └── stroma/                             (Worktree for rust)
│   └── nitro/
│       └── rigs/
│           └── stroma/                             (Worktree for nitro)
│
├── refinery/                                       ✅ MOVE FROM stroma/refinery
│   └── rig/
│       └── stroma/                                 (Worktree for refinery)
│
├── witness/                                        ✅ MOVE FROM stroma/witness
│   └── rig/
│       └── stroma/                                 (Worktree for witness)
│
├── deacon/                                         ✅ KEEP (already at root)
├── daemon/                                         ✅ KEEP (already at root)
└── logs/                                           ✅ KEEP (already at root)
```

### Key Improvements

1. **Single source of truth**: `.beads/` at repository root
2. **Proper nesting**: `polecats/chrome/rigs/stroma/` not `stroma/stroma/`
3. **Clear structure**: All Gas Town agents at root level
4. **Symlinks for context**: Each rig symlinks to root .beads, .cursor, docs
5. **No duplication**: Remove `/stroma/` after migration

---

## Data Preservation Strategy

### Files to Preserve (CRITICAL)

| Resource | Current Location | Preservation Method | Destination |
|----------|------------------|---------------------|-------------|
| **20 .bead files** | `/stroma/.beads/*.bead` | Direct move | `/.beads/*.bead` |
| **issues.jsonl** | `/stroma/.beads/issues.jsonl` | Direct move | `/.beads/issues.jsonl` |
| **routes.jsonl** | `/stroma/.beads/routes.jsonl` | Direct move | `/.beads/routes.jsonl` |
| **interactions.jsonl** | `/stroma/.beads/interactions.jsonl` | Direct move | `/.beads/interactions.jsonl` |
| **config.yaml** | `/stroma/.beads/config.yaml` | Direct move | `/.beads/config.yaml` |
| **22 .mdc rules** | `/.cursor/rules/*.mdc` | Keep in place | `/.cursor/rules/*.mdc` |
| **docs/** | `/docs/` | Keep in place | `/docs/` |
| **spike docs** | `/docs/spike/` | Keep in place | `/docs/spike/` |

### Pre-Migration Checklist

Before any changes:
- [x] Backup created (git branch: `backup/pre-gastown-reorg`)
- [x] Tarball created: `~/stroma-backup-YYYYMMDD-HHMMSS.tar.gz`
- [x] All uncommitted changes committed or stashed
- [x] Verify all 20 .bead files readable
- [x] Verify all 22 .mdc files readable
- [x] Verify docs/ accessible

---

## Migration Strategy Summary

### Phase 1: Safety
1. Create `backup/pre-gastown-reorg` branch
2. Create tarball backup
3. Commit current state

### Phase 2: Core Move
1. Move `stroma/.beads/` → `.beads/`
2. Remove symlink at root: `rm .beads` (old symlink)
3. Verify new `.beads/` contains all 20 .bead files

### Phase 3: Reorganize Agents
1. Move `stroma/polecats/` → `polecats/`
2. Move `stroma/refinery/` → `refinery/`
3. Move `stroma/witness/` → `witness/`
4. Reorganize `mayor/` structure
5. Keep `daemon/`, `deacon/` at root

### Phase 4: Worktrees
1. Investigate current worktree setup
2. Recreate proper git worktrees if needed
3. Set up symlinks for context access

### Phase 5: Cleanup
1. Remove `/stroma/` directory (after confirming everything moved)
2. Delete old plan file
3. Update documentation

---

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| **Data loss (.beads)** | Low | Critical | Backup branch + tarball |
| **Worktree corruption** | Medium | High | Can recreate from main |
| **Context access breaks** | Medium | Medium | Symlinks + testing |
| **Git history issues** | Low | Low | Not modifying history |
| **Docs disappear** | Very Low | High | Not moving docs/ |

---

## Next Steps

1. **Review this analysis** with user
2. **Create detailed migration-plan.md** with exact commands
3. **Create test-plan.md** for validation
4. **Get approval** before proceeding to hq-reorg-2

---

## Appendix: File Counts

- **Total .bead files**: 20
- **Total .mdc rules**: 22
- **Total spike docs**: 26 markdown files
- **Beads directory size**: 3.9M
- **Nested stroma/ size**: 11G
- **Git worktrees**: 1 (main only, needs investigation)

---

**Analysis Complete**: Ready for migration planning.
