#!/usr/bin/env fish
# Install Gastown CI/CD Protection Hooks (Fish Shell)
# Installs pre-push hook from current repository to its .git/hooks

# Get the repository root (where this script is located)
set script_dir (dirname (status --current-filename))
set repo_root (cd "$script_dir/.." 2>/dev/null; and pwd)
set hook_source "$repo_root/scripts/hooks/pre-push"

# Check if hook source exists
if not test -f "$hook_source"
    echo "Error: Hook source not found at $hook_source"
    echo "This script must be run from within the repository"
    exit 1
end

echo "Installing Gastown CI/CD protection hooks..."
echo "Repository: $repo_root"
echo "Hook source: $hook_source"
echo ""

# Verify this is a git repository
if not test -d "$repo_root/.git"
    echo "Error: Not a git repository"
    exit 1
end

# Install hook in current repository
set hook_dest "$repo_root/.git/hooks/pre-push"

echo "📥 Installing pre-push hook..."

# Create hooks directory if it doesn't exist
mkdir -p "$repo_root/.git/hooks"

# Remove existing hook (symlink or file)
rm -f "$hook_dest"

# Copy hook to .git/hooks
cp "$hook_source" "$hook_dest"
chmod +x "$hook_dest"

# Verify
if test -f "$hook_dest"; and test -x "$hook_dest"
    echo "✅ Installation complete!"
else
    echo "❌ Error: Installation failed"
    exit 1
end

echo ""
echo "✅ Hook installed at: $hook_dest"
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
