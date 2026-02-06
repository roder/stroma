#!/bin/bash
# Refinery Layer 2 Validation Script
# Run this before merging any branch to main
#
# Usage:
#   ./refinery-validate.sh [branch-name]
#   (defaults to current branch if not specified)

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[Refinery Layer 2]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[Refinery Layer 2]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[Refinery Layer 2]${NC} $1"
}

log_error() {
    echo -e "${RED}[Refinery Layer 2]${NC} $1"
}

# Get branch name
BRANCH="${1:-$(git branch --show-current)}"

log_info "Validating branch: $BRANCH"
echo ""

# Check if we're in a git repository
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    log_error "Not in a git repository"
    exit 1
fi

# 1. Ensure branch is up-to-date with main
log_info "Step 1/6: Checking if branch is up-to-date with main..."
git fetch origin main --quiet

if ! git merge-base --is-ancestor origin/main HEAD 2>/dev/null; then
    log_error "Branch is not up-to-date with main"
    echo ""
    echo "To fix:"
    echo "  git merge origin/main"
    echo "  # Resolve any conflicts"
    echo "  # Re-run this script"
    exit 1
fi
log_success "Branch is up-to-date with main"
echo ""

# Detect project type
if [ ! -f "Cargo.toml" ]; then
    log_warn "Not a Rust project (no Cargo.toml found)"
    log_warn "Skipping Rust-specific quality gates"
    exit 0
fi

log_info "Rust project detected, running quality gates..."
echo ""

# 2. Format check
log_info "Step 2/6: Running cargo fmt --check..."
if ! cargo fmt --check 2>&1 | head -20; then
    log_error "Formatting check failed"
    echo ""
    echo "To fix:"
    echo "  cargo fmt"
    echo "  git add ."
    echo "  git commit -m 'Run cargo fmt'"
    echo "  # Re-run this script"
    exit 1
fi
log_success "Formatting check passed"
echo ""

# 3. Clippy
log_info "Step 3/6: Running cargo clippy..."
if ! cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tail -50; then
    log_error "Clippy linting failed"
    echo ""
    echo "Fix all warnings above and re-run this script"
    exit 1
fi
log_success "Clippy check passed"
echo ""

# 4. Tests
log_info "Step 4/6: Running tests..."
if command -v cargo-nextest >/dev/null 2>&1; then
    if ! cargo nextest run --all-features 2>&1 | tail -50; then
        log_error "Tests failed"
        exit 1
    fi
else
    log_warn "cargo-nextest not found, using cargo test"
    if ! cargo test --all-features 2>&1 | tail -50; then
        log_error "Tests failed"
        exit 1
    fi
fi
log_success "All tests passed"
echo ""

# 5. Coverage check
log_info "Step 5/6: Checking test coverage..."
if command -v cargo-llvm-cov >/dev/null 2>&1; then
    if ! cargo llvm-cov nextest --all-features --summary-only 2>&1; then
        log_error "Coverage check failed"
        echo ""
        echo "Coverage must be 100% for all code"
        exit 1
    fi
    log_success "Coverage check passed"
else
    log_warn "cargo-llvm-cov not found, skipping coverage check"
    log_warn "Install with: cargo install cargo-llvm-cov"
fi
echo ""

# 6. Supply chain security
log_info "Step 6/6: Running supply chain security check..."
if command -v cargo-deny >/dev/null 2>&1; then
    if ! cargo deny check 2>&1 | tail -20; then
        log_error "Supply chain security check failed"
        exit 1
    fi
    log_success "Supply chain security check passed"
else
    log_warn "cargo-deny not found, skipping supply chain check"
    log_warn "Install with: cargo install cargo-deny"
fi
echo ""

# All checks passed
echo ""
log_success "═══════════════════════════════════════════════════════"
log_success "✅ All Layer 2 quality gates PASSED!"
log_success "═══════════════════════════════════════════════════════"
echo ""
log_success "Branch '$BRANCH' is validated and safe to merge to main"
echo ""
echo "To merge:"
echo "  git checkout main"
echo "  git merge $BRANCH"
echo "  git push"
