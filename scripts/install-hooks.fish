#!/usr/bin/env fish
# Install Gastown CI/CD Protection Hooks (Fish Shell)
# Installs pre-push hooks for all repositories in gastown workspace

# Set GASTOWN_ROOT with default
if set -q GASTOWN_ROOT
    set gastown_root $GASTOWN_ROOT
else
    set gastown_root $HOME/gt
end

set hook_source "$gastown_root/scripts/hooks/pre-push"

# Check if hook source exists
if not test -f "$hook_source"
    echo "Error: Hook source not found at $hook_source"
    exit 1
end

echo "Installing Gastown CI/CD protection hooks..."
echo "Source: $hook_source"
echo ""

# Function to install hook in a repository
function install_hook
    set repo_dir $argv[1]
    set hook_dest "$repo_dir/.git/hooks/pre-push"

    # Skip if not a git repo
    if not test -d "$repo_dir/.git"
        return
    end

    # Check if it has a GitHub remote
    set github_remote (cd "$repo_dir" 2>/dev/null; and git remote get-url origin 2>/dev/null | grep -i github)
    if test -z "$github_remote"
        return
    end

    echo "📥 Installing in: $repo_dir"

    # Create hooks directory if it doesn't exist
    mkdir -p "$repo_dir/.git/hooks"

    # Remove existing hook (symlink or file)
    rm -f "$hook_dest"

    # Copy hook to repository
    cp "$hook_source" "$hook_dest"
    chmod +x "$hook_dest"

    # Verify
    if test -f "$hook_dest"; and test -x "$hook_dest"
        echo "✅ Installed (copied)"
    else
        echo "⚠️  Warning: Installation may have failed"
    end
end

# Install for specific repository if provided
if test (count $argv) -gt 0
    install_hook $argv[1]
    exit 0
end

# Otherwise, install for all GitHub repos in gastown
echo "Scanning for GitHub repositories in $gastown_root..."
echo ""

set count 0
for git_dir in (find "$gastown_root" -name ".git" -type d -maxdepth 6 2>/dev/null)
    set repo_dir (dirname "$git_dir")
    install_hook "$repo_dir"
    set count (math $count + 1)
end

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
echo "See $gastown_root/scripts/README.md for documentation"
