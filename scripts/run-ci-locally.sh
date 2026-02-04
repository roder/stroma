#!/bin/bash
# Local CI Simulator - Run full CI suite locally before pushing
# Matches the checks in .github/workflows/ci.yml and security.yml

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print colored message
print_status() {
    local color=$1
    local message=$2
    echo -e "${color}${message}${NC}"
}

print_header() {
    echo ""
    echo "======================================"
    echo "$1"
    echo "======================================"
}

# Track overall success
FAILED_CHECKS=()

# Check if specific job requested
REQUESTED_JOB="$1"

# Job: Format Check
run_format_check() {
    print_header "Format Check (cargo fmt)"
    if cargo fmt --check; then
        print_status "$GREEN" "✓ Format check passed"
    else
        print_status "$RED" "✗ Format check failed"
        FAILED_CHECKS+=("format")
        return 1
    fi
}

# Job: Lint Check
run_lint_check() {
    print_header "Lint Check (cargo clippy)"
    if cargo clippy --all-targets --all-features -- -D warnings; then
        print_status "$GREEN" "✓ Clippy passed"
    else
        print_status "$RED" "✗ Clippy failed"
        FAILED_CHECKS+=("lint")
        return 1
    fi
}

# Job: Tests
run_tests() {
    print_header "Test Suite (cargo nextest)"

    # Check if nextest is installed
    if ! command -v cargo-nextest &> /dev/null; then
        print_status "$YELLOW" "Installing cargo-nextest..."
        cargo install cargo-nextest
    fi

    if cargo nextest run --all-features; then
        print_status "$GREEN" "✓ All tests passed"
    else
        print_status "$RED" "✗ Tests failed"
        FAILED_CHECKS+=("test")
        return 1
    fi
}

# Job: Coverage Check
run_coverage_check() {
    print_header "Coverage Check (cargo llvm-cov) - 100% Required"

    # Check if llvm-cov is installed
    if ! command -v cargo-llvm-cov &> /dev/null; then
        print_status "$YELLOW" "Installing cargo-llvm-cov..."
        cargo install cargo-llvm-cov
    fi

    # Run coverage
    if cargo llvm-cov nextest --all-features --fail-under-lines 100; then
        print_status "$GREEN" "✓ Coverage is 100%"
    else
        print_status "$RED" "✗ Coverage is below 100%"
        print_status "$YELLOW" "Run 'cargo llvm-cov nextest --all-features --html' to see gaps"
        FAILED_CHECKS+=("coverage")
        return 1
    fi
}

# Job: Dependency Check
run_dependency_check() {
    print_header "Dependency Check (cargo deny)"

    # Check if cargo-deny is installed
    if ! command -v cargo-deny &> /dev/null; then
        print_status "$YELLOW" "Installing cargo-deny..."
        cargo install cargo-deny
    fi

    if cargo deny check; then
        print_status "$GREEN" "✓ Dependency check passed"
    else
        print_status "$RED" "✗ Dependency check failed"
        FAILED_CHECKS+=("dependencies")
        return 1
    fi
}

# Job: Binary Size Check
run_binary_size_check() {
    print_header "Binary Size Check"

    print_status "$BLUE" "Building release binary for x86_64-unknown-linux-musl..."

    # Check if target is installed
    if ! rustup target list | grep -q "x86_64-unknown-linux-musl (installed)"; then
        print_status "$YELLOW" "Installing x86_64-unknown-linux-musl target..."
        rustup target add x86_64-unknown-linux-musl
    fi

    if cargo build --release --target x86_64-unknown-linux-musl; then
        # Find the binary
        BINARY=$(find target/x86_64-unknown-linux-musl/release -maxdepth 1 -type f -executable | head -1)
        if [ -n "$BINARY" ]; then
            SIZE=$(du -h "$BINARY" | cut -f1)
            print_status "$GREEN" "✓ Binary built successfully: $SIZE"
            print_status "$BLUE" "  Path: $BINARY"
        else
            print_status "$YELLOW" "⚠ Binary built but not found for size check"
        fi
    else
        print_status "$RED" "✗ Binary build failed"
        FAILED_CHECKS+=("binary-size")
        return 1
    fi
}

# Job: Security Constraints Check
run_security_constraints_check() {
    print_header "Security Constraints Check"

    local failed=false

    # Check 1: No presage-store-sqlite
    print_status "$BLUE" "Checking for banned crate: presage-store-sqlite..."
    if grep -r "presage-store-sqlite" Cargo.toml src/ 2>/dev/null; then
        print_status "$RED" "✗ Found banned crate: presage-store-sqlite"
        failed=true
    else
        print_status "$GREEN" "✓ No banned crates found"
    fi

    # Check 2: Unsafe blocks must have // SAFETY: comments
    print_status "$BLUE" "Checking unsafe blocks for documentation..."
    UNSAFE_WITHOUT_SAFETY=$(grep -r "unsafe" --include="*.rs" src/ 2>/dev/null | grep -v "// SAFETY:" | grep -v "unsafe impl" | grep -v "#\[deny(unsafe" || true)
    if [ -n "$UNSAFE_WITHOUT_SAFETY" ]; then
        print_status "$RED" "✗ Found unsafe blocks without // SAFETY: comments:"
        echo "$UNSAFE_WITHOUT_SAFETY"
        failed=true
    else
        print_status "$GREEN" "✓ All unsafe blocks documented"
    fi

    # Check 3: No cleartext Signal IDs (basic check)
    print_status "$BLUE" "Checking for potential cleartext Signal ID storage..."
    POTENTIAL_CLEARTEXT=$(grep -r "signal.*id" --include="*.rs" src/ 2>/dev/null | grep -v "mask_identity" | grep -v "Hash" | grep -v "// OK:" | grep -v "test" || true)
    if [ -n "$POTENTIAL_CLEARTEXT" ]; then
        print_status "$YELLOW" "⚠ Found potential cleartext Signal ID usage (manual review needed):"
        echo "$POTENTIAL_CLEARTEXT" | head -5
    fi

    if [ "$failed" = true ]; then
        print_status "$RED" "✗ Security constraints check failed"
        FAILED_CHECKS+=("security-constraints")
        return 1
    else
        print_status "$GREEN" "✓ Security constraints check passed"
    fi
}

# Main execution
print_status "$BLUE" "╔═══════════════════════════════════════╗"
print_status "$BLUE" "║   Local CI Simulator for Stromarig    ║"
print_status "$BLUE" "╚═══════════════════════════════════════╝"

# Change to project root
cd "$(dirname "$0")/.."

if [ -n "$REQUESTED_JOB" ]; then
    print_status "$BLUE" "Running specific job: $REQUESTED_JOB"
    echo ""

    case "$REQUESTED_JOB" in
        format)
            run_format_check
            ;;
        lint)
            run_lint_check
            ;;
        test)
            run_tests
            ;;
        coverage)
            run_coverage_check
            ;;
        deps|dependencies)
            run_dependency_check
            ;;
        binary|binary-size)
            run_binary_size_check
            ;;
        security|security-constraints)
            run_security_constraints_check
            ;;
        *)
            print_status "$RED" "Unknown job: $REQUESTED_JOB"
            echo ""
            echo "Available jobs:"
            echo "  format              - Format check"
            echo "  lint                - Clippy check"
            echo "  test                - Test suite"
            echo "  coverage            - Coverage check (100% required)"
            echo "  dependencies        - Dependency security check"
            echo "  binary-size         - Binary size check"
            echo "  security-constraints - Security policy check"
            echo ""
            echo "Run without arguments to execute all checks"
            exit 1
            ;;
    esac
else
    # Run all checks
    print_status "$BLUE" "Running full CI suite..."
    echo ""

    # Fast checks first
    run_format_check || true
    run_lint_check || true
    run_dependency_check || true
    run_security_constraints_check || true

    # Slower checks
    run_tests || true
    run_coverage_check || true
    run_binary_size_check || true
fi

# Summary
echo ""
print_header "Summary"

if [ ${#FAILED_CHECKS[@]} -eq 0 ]; then
    print_status "$GREEN" "✓ All CI checks passed!"
    print_status "$GREEN" "Ready to push to remote."
    exit 0
else
    print_status "$RED" "✗ ${#FAILED_CHECKS[@]} check(s) failed:"
    for check in "${FAILED_CHECKS[@]}"; do
        print_status "$RED" "  - $check"
    done
    echo ""
    print_status "$YELLOW" "Fix the failures above before pushing."
    print_status "$YELLOW" "You can run specific checks with: $0 <job-name>"
    exit 1
fi
