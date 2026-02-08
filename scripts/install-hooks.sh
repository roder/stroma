#!/bin/bash
# Install Gastown CI/CD Protection Hooks
# Installs pre-push hook from current repository to its .git/hooks

set -e

# Get the repository root (where this script is located)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
HOOK_SOURCE="$REPO_ROOT/scripts/hooks/pre-push"

if [ ! -f "$HOOK_SOURCE" ]; then
    echo "Error: Hook source not found at $HOOK_SOURCE"
    echo "This script must be run from within the repository"
    exit 1
fi

echo "Installing Gastown CI/CD protection hooks..."
echo "Repository: $REPO_ROOT"
echo "Hook source: $HOOK_SOURCE"
echo ""

# Verify this is a git repository
if [ ! -d "$REPO_ROOT/.git" ]; then
    echo "Error: Not a git repository"
    exit 1
fi

# Install hook in current repository
HOOK_DEST="$REPO_ROOT/.git/hooks/pre-push"

echo "📥 Installing pre-push hook..."

# Create hooks directory if it doesn't exist
mkdir -p "$REPO_ROOT/.git/hooks"

# Remove existing hook (symlink or file)
rm -f "$HOOK_DEST"

# Copy hook to .git/hooks
cp "$HOOK_SOURCE" "$HOOK_DEST"
chmod +x "$HOOK_DEST"

# Verify
if [ -f "$HOOK_DEST" ] && [ -x "$HOOK_DEST" ]; then
    echo "✅ Installation complete!"
else
    echo "❌ Error: Installation failed"
    exit 1
fi

echo ""
echo "✅ Hook installed at: $HOOK_DEST"
echo ""
echo "The pre-push hook will now:"
echo "  1. Sync beads JSONL (if bd available)"
echo "  2. Run quality gates when Rust files change (fmt, clippy, nextest)"
echo "  3. Check CI status before pushing to main"
echo "  4. Block push if quality gates fail or CI is red"
echo ""
echo "Smart skipping:"
echo "  - Branch deletions (no quality gates)"
echo "  - No Rust files changed (no quality gates)"
echo "  - beads-sync branch (auto-validated)"
echo ""
echo "Emergency escape hatches:"
echo "  - git push --no-verify (skip all checks)"
echo "  - export GASTOWN_SKIP_QUALITY_GATES=1 (skip quality gates only)"
