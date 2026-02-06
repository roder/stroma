#!/bin/bash
# Install Gastown CI/CD Protection Hooks
# Installs pre-push hooks for all repositories in gastown workspace

set -e

GASTOWN_ROOT="${GASTOWN_ROOT:-/Users/matt/gt}"
HOOK_SOURCE="$GASTOWN_ROOT/scripts/hooks/pre-push"

if [ ! -f "$HOOK_SOURCE" ]; then
    echo "Error: Hook source not found at $HOOK_SOURCE"
    exit 1
fi

echo "Installing Gastown CI/CD protection hooks..."
echo "Source: $HOOK_SOURCE"
echo ""

# Function to install hook in a repository
install_hook() {
    local repo_dir=$1
    local hook_dest="$repo_dir/.git/hooks/pre-push"

    # Skip if not a git repo
    if [ ! -d "$repo_dir/.git" ]; then
        return
    fi

    # Check if it has a GitHub remote
    local github_remote=$(cd "$repo_dir" && git remote get-url origin 2>/dev/null | grep -i github || true)
    if [ -z "$github_remote" ]; then
        return
    fi

    echo "📥 Installing in: $repo_dir"

    # Create hooks directory if it doesn't exist
    mkdir -p "$repo_dir/.git/hooks"

    # Remove existing hook (symlink or file)
    rm -f "$hook_dest"

    # Copy hook to repository
    cp "$HOOK_SOURCE" "$hook_dest"
    chmod +x "$hook_dest"

    # Verify
    if [ -f "$hook_dest" ] && [ -x "$hook_dest" ]; then
        echo "✅ Installed (copied)"
    else
        echo "⚠️  Warning: Installation may have failed"
    fi
}

# Install for specific repository if provided
if [ -n "$1" ]; then
    install_hook "$1"
    exit 0
fi

# Otherwise, install for all GitHub repos in gastown
echo "Scanning for GitHub repositories in $GASTOWN_ROOT..."
echo ""

count=0
while IFS= read -r git_dir; do
    repo_dir=$(dirname "$git_dir")
    install_hook "$repo_dir"
    ((count++))
done < <(find "$GASTOWN_ROOT" -name ".git" -type d -maxdepth 6 2>/dev/null)

echo ""
echo "✅ Installation complete! Installed in $count repositories"
echo ""
echo "The pre-push hook will now:"
echo "  1. Sync beads JSONL (if bd available)"
echo "  2. Run quality gates for ALL pushes (fmt, clippy, tests) - Layer 1"
echo "  3. Check CI status before pushing to main"
echo "  4. Block push if quality gates fail or CI is red"
echo ""
echo "WIP escape hatches:"
echo "  - git push --no-verify (skip all checks)"
echo "  - export GASTOWN_SKIP_QUALITY_GATES=1 (skip quality gates only)"
echo ""
echo "See $GASTOWN_ROOT/scripts/README.md for documentation"
