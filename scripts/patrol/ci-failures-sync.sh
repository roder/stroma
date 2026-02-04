#!/bin/bash
# CI Failures Sync - Mayor Patrol Task
#
# Polls GitHub for ci-failure issues and syncs them to beads.
# Part of CI/CD Green Branch Protection (hq-pzcn7, hq-diq31).
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
    echo -e "${BLUE}[ci-failures-sync]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[ci-failures-sync]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[ci-failures-sync]${NC} $1"
}

log_error() {
    echo -e "${RED}[ci-failures-sync]${NC} $1"
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

log_info "Checking for new CI failure alerts..."

# Query GitHub for ci-failure issues with auto-alert label
NEW_FAILURES=$(gh issue list \
    --label "ci-failure" \
    --label "auto-alert" \
    --state open \
    --json number,title,url,createdAt \
    --limit 50 2>/dev/null || echo "[]")

# Check if any failures found
FAILURE_COUNT=$(echo "$NEW_FAILURES" | jq length)

if [ "$FAILURE_COUNT" -eq 0 ]; then
    log_info "No new CI failures detected"
    exit 0
fi

log_warn "Found $FAILURE_COUNT new CI failure(s)"

# Process each failure
echo "$NEW_FAILURES" | jq -c '.[]' | while read -r issue_json; do
    issue_num=$(echo "$issue_json" | jq -r .number)
    title=$(echo "$issue_json" | jq -r .title)
    url=$(echo "$issue_json" | jq -r .url)
    created_at=$(echo "$issue_json" | jq -r .createdAt)

    log_info "Processing GitHub issue #${issue_num}: ${title}"

    # Check if we already created a beads issue for this GH issue
    existing=$(bd list --status=open --limit=0 | grep "GH#${issue_num}" || true)

    if [ -n "$existing" ]; then
        log_warn "Beads issue already exists for GH#${issue_num}, skipping"
        # Remove auto-alert label to prevent re-processing
        gh issue edit "$issue_num" --remove-label "auto-alert" 2>/dev/null || true
        continue
    fi

    # Create beads P0 issue
    log_info "Creating beads P0 issue for GH#${issue_num}"

    bd create \
        --title="${title} (GH#${issue_num})" \
        --type=bug \
        --priority=0 \
        --description="**CI Failure detected on main branch**

GitHub Issue: ${url}
Created: ${created_at}

This issue was auto-synced from GitHub by mayor patrol.
See the GitHub issue for full details and resolution steps.

**DO NOT MERGE MORE PRs UNTIL FIXED**

**Target resolution time**: 15 minutes" || {
        log_error "Failed to create beads issue for GH#${issue_num}"
        continue
    }

    # Get the created beads issue ID (last created issue)
    beads_id=$(bd list --status=open --limit=1 | grep "○" | awk '{print $2}' | tr -d '[]')

    if [ -n "$beads_id" ]; then
        log_success "Created beads issue ${beads_id} for GH#${issue_num}"

        # Add comment to GitHub issue linking to beads
        gh issue comment "$issue_num" \
            --body "🤖 **Beads Issue Created**: ${beads_id}

This CI failure has been synced to the local beads database for tracking.

Beads issue will be automatically closed when this GitHub issue is resolved." \
            2>/dev/null || log_warn "Failed to comment on GH#${issue_num}"
    else
        log_warn "Could not determine beads issue ID"
    fi

    # Remove auto-alert label so we don't re-create
    gh issue edit "$issue_num" --remove-label "auto-alert" 2>/dev/null || {
        log_warn "Failed to remove auto-alert label from GH#${issue_num}"
    }

    # Send notification to crew
    log_info "Sending notification to crew-approvals"
    gt mail send crew-approvals \
        -s "🚨 CI BROKEN - Auto-created P0 issue ${beads_id}" \
        -m "CI failure detected on main branch.

GitHub Issue: #${issue_num}
Beads Issue: ${beads_id}
Title: ${title}

View on GitHub: ${url}

**DO NOT MERGE MORE PRs** until this is resolved.

Target resolution: 15 minutes" 2>/dev/null || log_warn "Failed to send mail notification"

    log_success "✓ Processed GH#${issue_num} → ${beads_id}"
done

# Sync beads to JSONL
log_info "Syncing beads to JSONL..."
bd sync --flush-only 2>/dev/null || log_warn "Failed to sync beads"

log_success "CI failures sync complete"
