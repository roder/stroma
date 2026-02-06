#!/bin/bash
# Gastown CI Failures Sync - Mayor Patrol Task
# Generic patrol task that works with any GitHub repository
#
# Polls GitHub for ci-failure issues across all gastown rigs
# and syncs them to the town-level beads database.
#
# This runs as a mayor patrol task to keep beads database mutations
# local (never in CI), while maintaining automation.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[gastown/ci-sync]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[gastown/ci-sync]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[gastown/ci-sync]${NC} $1"
}

log_error() {
    echo -e "${RED}[gastown/ci-sync]${NC} $1"
}

# Check if gh CLI is available
if ! command -v gh &> /dev/null; then
    log_error "gh CLI not found. Install with: brew install gh"
    exit 1
fi

# Check if bd CLI is available
if ! command -v bd &> /dev/null; then
    log_error "bd CLI not found. Install with: brew install beads"
    exit 1
fi

# Verify gh is authenticated
if ! gh auth status &> /dev/null; then
    log_error "gh CLI not authenticated. Run: gh auth login"
    exit 1
fi

# Function to sync failures for a specific repository
sync_repo_failures() {
    local repo=$1
    log_info "Checking $repo..."

    # Query GitHub for ci-failure issues with auto-alert label
    local failures=$(gh issue list \
        --repo "$repo" \
        --label "ci-failure" \
        --label "auto-alert" \
        --state open \
        --json number,title,url,createdAt \
        --limit 50 2>/dev/null || echo "[]")

    # Check if any failures found
    local failure_count=$(echo "$failures" | jq length)

    if [ "$failure_count" -eq 0 ]; then
        return 0
    fi

    log_warn "Found $failure_count CI failure(s) in $repo"

    # Process each failure
    echo "$failures" | jq -c '.[]' | while read -r issue_json; do
        local issue_num=$(echo "$issue_json" | jq -r .number)
        local title=$(echo "$issue_json" | jq -r .title)
        local url=$(echo "$issue_json" | jq -r .url)
        local created_at=$(echo "$issue_json" | jq -r .createdAt)

        log_info "Processing $repo#${issue_num}: ${title}"

        # Check if we already created a beads issue for this GH issue
        local existing=$(bd list --status=open --limit=0 | grep "$repo#${issue_num}" || true)

        if [ -n "$existing" ]; then
            log_warn "Beads issue already exists for $repo#${issue_num}, skipping"
            # Remove auto-alert label to prevent re-processing
            gh issue edit "$issue_num" --repo "$repo" --remove-label "auto-alert" 2>/dev/null || true
            continue
        fi

        # Create beads P0 issue
        log_info "Creating beads P0 issue for $repo#${issue_num}"

        bd create \
            --title="${title} ($repo#${issue_num})" \
            --type=bug \
            --priority=0 \
            --description="**CI Failure detected on main branch**

Repository: $repo
GitHub Issue: ${url}
Created: ${created_at}

This issue was auto-synced from GitHub by gastown patrol.
See the GitHub issue for full details and resolution steps.

**DO NOT MERGE MORE PRs UNTIL FIXED**

**Target resolution time**: 15 minutes" || {
            log_error "Failed to create beads issue for $repo#${issue_num}"
            continue
        }

        # Get the created beads issue ID (last created issue)
        local beads_id=$(bd list --status=open --limit=1 | grep "○" | awk '{print $2}' | tr -d '[]')

        if [ -n "$beads_id" ]; then
            log_success "Created beads issue ${beads_id} for $repo#${issue_num}"

            # Add comment to GitHub issue linking to beads
            gh issue comment "$issue_num" --repo "$repo" \
                --body "🤖 **Beads Issue Created**: ${beads_id}

This CI failure has been synced to the gastown beads database for tracking.

Beads issue will be automatically closed when this GitHub issue is resolved." \
                2>/dev/null || log_warn "Failed to comment on $repo#${issue_num}"
        else
            log_warn "Could not determine beads issue ID"
        fi

        # Remove auto-alert label so we don't re-create
        gh issue edit "$issue_num" --repo "$repo" --remove-label "auto-alert" 2>/dev/null || {
            log_warn "Failed to remove auto-alert label from $repo#${issue_num}"
        }

        # TODO: Fix mail notification in launchd environment
        # For now, mayor can see P0 issues with: bd list --priority=0 --status=open
        log_info "P0 beads issue created: ${beads_id} (mail notifications disabled)"

        log_success "✓ Processed $repo#${issue_num} → ${beads_id}"
    done
}

# Main execution
log_info "Starting gastown CI failures sync..."

# Auto-discover all GitHub repositories in gastown workspace
# Strategy: Find all .git directories and check if they have GitHub remotes
GASTOWN_ROOT="${GASTOWN_ROOT:-~gt}"

# Find all repos with GitHub remotes
while IFS= read -r git_dir; do
    repo_dir=$(dirname "$git_dir")

    # Get GitHub remote URL
    github_remote=$(cd "$repo_dir" && git remote get-url origin 2>/dev/null | grep -i github || true)

    if [ -n "$github_remote" ]; then
        # Extract owner/repo from URL
        repo_name=$(echo "$github_remote" | sed -E 's/.*[:/]([^/]+\/[^/]+)(\.git)?$/\1/' | sed 's/\.git$//')

        if [ -n "$repo_name" ]; then
            sync_repo_failures "$repo_name"
        fi
    fi
done < <(find "$GASTOWN_ROOT" -name ".git" -type d 2>/dev/null)

# Sync beads to JSONL (from gastown root where the database is)
log_info "Syncing beads to JSONL..."
(cd "$GASTOWN_ROOT" && bd sync --flush-only) 2>/dev/null || log_warn "Failed to sync beads"

log_success "Gastown CI failures sync complete"
