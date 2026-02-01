# Gas Town Repository Reorganization - Migration Plan

**Date**: 2026-01-31
**Bead**: hq-reorg-2 (to be executed)
**Based on**: analysis-report.md

See full plan at: analysis-report.md (already created)

## Quick Summary

1. **Backup** (git branch + tarball)
2. **Move .beads/** from stroma/ to root
3. **Move agents** (polecats/, refinery/, witness/) to root  
4. **Handle mayor/** merge
5. **Remove stroma/** directory
6. **Update symlinks** in all rigs
7. **Check worktrees** (may need recreation)
8. **Test** basic functionality
9. **Commit and tag**

Detailed steps available in analysis-report.md Section "Migration Strategy Summary"
