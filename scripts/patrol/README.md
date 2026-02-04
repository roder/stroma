# Mayor Patrol Scripts

This directory contains automated patrol tasks that run periodically to maintain gastown health and sync external systems.

## Available Patrol Tasks

### `ci-failures-sync.sh`

**Purpose**: Syncs GitHub CI failure issues to beads database

**What it does**:
1. Polls GitHub for issues labeled `ci-failure,auto-alert`
2. Creates corresponding P0 beads issues locally
3. Sends notifications to crew-approvals
4. Removes `auto-alert` label (prevents duplicates)
5. Syncs beads to JSONL

**Why**: Keeps CI failure tracking automated while maintaining beads database mutations on trusted local machine (not in CI runners).

**Part of**: CI/CD Green Branch Protection (hq-pzcn7, hq-diq31)

## Running Patrol Tasks

### Manual Execution

```bash
# Run CI failures sync once
./scripts/patrol/ci-failures-sync.sh

# Run with logging
./scripts/patrol/ci-failures-sync.sh 2>&1 | tee -a logs/patrol-ci-sync.log
```

### Automated Execution

#### Option 1: Cron Job (Recommended)

Add to your crontab:

```bash
# Edit crontab
crontab -e

# Add patrol tasks (runs every 5 minutes)
*/5 * * * * cd /Users/matt/gt/stromarig/mayor/rig && ./scripts/patrol/ci-failures-sync.sh >> logs/patrol-ci-sync.log 2>&1
```

#### Option 2: Launchd (macOS Native)

Create `~/Library/LaunchAgents/com.gastown.mayor.patrol.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.gastown.mayor.patrol</string>
    <key>ProgramArguments</key>
    <array>
        <string>/Users/matt/gt/stromarig/mayor/rig/scripts/patrol/ci-failures-sync.sh</string>
    </array>
    <key>WorkingDirectory</key>
    <string>/Users/matt/gt/stromarig/mayor/rig</string>
    <key>StartInterval</key>
    <integer>300</integer>
    <key>StandardOutPath</key>
    <string>/Users/matt/gt/stromarig/mayor/rig/logs/patrol-ci-sync.log</string>
    <key>StandardErrorPath</key>
    <string>/Users/matt/gt/stromarig/mayor/rig/logs/patrol-ci-sync.log</string>
</dict>
</plist>
```

Then load it:
```bash
launchctl load ~/Library/LaunchAgents/com.gastown.mayor.patrol.plist
launchctl start com.gastown.mayor.patrol
```

#### Option 3: Mayor Daemon Integration (Future)

When mayor daemon supports patrol plugins:

```bash
# Register patrol task
gt patrol register ci-failures-sync \
    --script ./scripts/patrol/ci-failures-sync.sh \
    --interval 5m

# List registered patrols
gt patrol list

# Manually trigger
gt patrol run ci-failures-sync
```

## Monitoring

### Check Patrol Logs

```bash
# View recent activity
tail -f logs/patrol-ci-sync.log

# Check for errors
grep ERROR logs/patrol-ci-sync.log

# View sync history
grep "Processed GH#" logs/patrol-ci-sync.log
```

### Verify Patrol is Running

```bash
# For cron
ps aux | grep patrol

# For launchd
launchctl list | grep gastown

# Check last execution
ls -lt logs/patrol-ci-sync.log
```

## Troubleshooting

### "gh CLI not found"
```bash
brew install gh
gh auth login
```

### "bd CLI not found"
```bash
brew install beads
```

### "gh CLI not authenticated"
```bash
gh auth login
# Follow prompts
```

### Script not executing from cron
```bash
# Cron needs absolute paths
which gh    # Use this in script
which bd    # Use this in script

# Or add to PATH in script
export PATH="/usr/local/bin:/opt/homebrew/bin:$PATH"
```

### No new failures detected (but you know there are some)

Check GitHub issue labels:
```bash
gh issue list --label ci-failure --label auto-alert
```

If missing `auto-alert` label, CI monitor workflow may not have run. Check workflow runs:
```bash
gh run list --workflow=ci-monitor.yml
```

## Adding New Patrol Tasks

1. Create script in `scripts/patrol/`
2. Make executable: `chmod +x scripts/patrol/your-task.sh`
3. Test manually: `./scripts/patrol/your-task.sh`
4. Add to cron/launchd with appropriate interval
5. Document in this README

## Best Practices

- **Idempotent**: Scripts should be safe to run multiple times
- **Logging**: Always log to `logs/patrol-*.log`
- **Error handling**: Use `set -e` and check command availability
- **Rate limits**: Be mindful of API rate limits (GitHub: 5000/hour)
- **Testing**: Test scripts manually before automation
