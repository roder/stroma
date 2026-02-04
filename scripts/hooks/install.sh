#!/bin/bash
# Install git hooks for CI/CD Protection
# Part of CI/CD Green Branch Protection (hq-a3nl5, hq-diq31)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "Installing CI/CD protection git hooks..."
echo "Repository: $REPO_ROOT"
echo ""

# Check if we're in a git repository
if [ ! -d "$REPO_ROOT/.git" ]; then
    echo "Error: Not a git repository"
    exit 1
fi

# Install pre-push hook
echo "📥 Installing pre-push hook..."
cp "$SCRIPT_DIR/pre-push" "$REPO_ROOT/.git/hooks/pre-push"
chmod +x "$REPO_ROOT/.git/hooks/pre-push"
echo "✅ Installed: .git/hooks/pre-push"

# Verify requirements
echo ""
echo "Checking requirements..."

# Check gh CLI
if command -v gh &> /dev/null; then
    echo "✅ gh CLI installed"
    if gh auth status &> /dev/null; then
        echo "✅ gh CLI authenticated"
    else
        echo "⚠️  gh CLI not authenticated"
        echo "   Run: gh auth login"
    fi
else
    echo "⚠️  gh CLI not installed"
    echo "   Install: brew install gh"
    echo "   The hook will gracefully skip CI checks if gh is not available"
fi

# Check bd CLI
if command -v bd &> /dev/null; then
    echo "✅ bd CLI installed"
else
    echo "⚠️  bd CLI not installed (optional)"
    echo "   Install: brew install beads"
    echo "   The hook will skip beads sync if bd is not available"
fi

echo ""
echo "✅ Installation complete!"
echo ""
echo "The pre-push hook will now:"
echo "  1. Sync beads JSONL (if bd available)"
echo "  2. Check CI status before pushing to main"
echo "  3. Block push if main CI is red"
echo ""
echo "Emergency bypass: git push --no-verify"
echo ""
echo "See scripts/hooks/README.md for full documentation"
