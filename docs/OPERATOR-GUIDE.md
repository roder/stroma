# Stroma Operator Guide

**For Bot Administrators**

This guide explains how to deploy and maintain a Stroma bot for your community.

## Understanding the Operator Role

### What You Are
A **service runner** who maintains the bot infrastructure.

### What You're NOT
- A privileged admin who controls membership
- Someone who can override group decisions
- Someone who can bypass trust protocols
- Someone with access to cleartext member identities

**After bootstrap, you're just another member with service maintenance duties.**

## Prerequisites

### Required
- **Linux server** (VPS or dedicated)
- **Signal account** (phone number required for bot)
- **Rust 1.93+** (musl 1.2.5 with improved DNS)
- **freenet-core v0.1.107+** (decentralized state storage)
- **Static IP or stable connection** (for freenet-core node)

### Recommended
- **systemd** (for auto-restart on crashes)
- **Monitoring** (journalctl, prometheus, or similar)
- **Backup phone number** (in case of Signal ban)

## Installation

### Step 1: Install Rust 1.93+

```bash
# Install or update Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable

# Verify version
rustc --version  # Should show 1.93+

# Add MUSL target for static binaries
rustup target add x86_64-unknown-linux-musl
```

**Why Rust 1.93+:**
- Bundled musl 1.2.5 with major DNS resolver improvements
- More reliable networking for Signal and freenet-core
- Critical for production deployments

### Step 2: Install freenet-core

```bash
# Clone freenet-core
git clone https://github.com/freenet/freenet-core.git
cd freenet-core

# Install freenet-core binary
cargo install --path crates/core

# Verify installation
freenet --version
```

### Step 3: Install Stroma

```bash
# Clone Stroma
git clone https://github.com/roder/stroma.git
cd stroma

# Build production binary (static MUSL)
cargo build --release --target x86_64-unknown-linux-musl

# Binary location
./target/x86_64-unknown-linux-musl/release/stroma
```

## Configuration

### Create Config File

```bash
# Create config directory
mkdir -p ~/.config/stroma

# Create config.toml
cat > ~/.config/stroma/config.toml <<'EOF'
[signal]
phone_number = "+1234567890"  # Bot's Signal phone number
data_dir = "/var/lib/stroma/signal"

[freenet]
node_address = "127.0.0.1:8080"
contract_key = ""  # Will be set after bootstrap

[group]
pepper_file = "/var/lib/stroma/pepper.secret"  # Group-secret for HMAC

[logging]
level = "info"
file = "/var/log/stroma/bot.log"
EOF
```

### Generate Group Pepper (CRITICAL)

```bash
# Generate random pepper (KEEP THIS SECRET)
mkdir -p /var/lib/stroma
openssl rand -base64 32 > /var/lib/stroma/pepper.secret
chmod 600 /var/lib/stroma/pepper.secret

# NEVER commit this file to git
# NEVER share this file
# Backup securely (if lost, member hashes won't match)
```

## Bootstrap Process (One-Time)

### Step 1: Start freenet-core Node

```bash
# Start freenet-core in dark mode (anonymous)
freenet --mode dark &

# Verify node is running
curl http://127.0.0.1:8080/health
```

### Step 2: Create Seed Group (Manual - 3 Members)

```bash
# You need 3 people to start (including yourself)
# Manually create Signal group and add all 3
```

**Seed Group Requirements:**
- Must be exactly 3 members
- All 3 must trust each other
- All 3 will become initial Validators

### Step 3: Initialize Freenet Contract

```bash
# Run Stroma bootstrap command
./stroma bootstrap \
  --config ~/.config/stroma/config.toml \
  --seed-members @Alice,@Bob,@Carol

# Bot will:
# - Hash all 3 Signal IDs
# - Create initial vouch graph (triangle: everyone vouches everyone)
# - Deploy contract to freenet-core
# - Return contract_key
```

### Step 4: Update Config with Contract Key

```bash
# Add contract_key to config.toml
# (Output from bootstrap command)
nano ~/.config/stroma/config.toml

# Should look like:
# [freenet]
# contract_key = "0x123abc..."
```

### Step 5: Start Bot Service

```bash
# Run bot
./stroma run --config ~/.config/stroma/config.toml

# Bot is now running and monitoring Freenet state stream
```

**After this point, you have NO special privileges. All membership changes are automatic based on Freenet contract.**

## Service Management

### systemd Service (Recommended)

```bash
# Create systemd service file
sudo nano /etc/systemd/system/stroma-bot.service
```

```ini
[Unit]
Description=Stroma Trust Network Bot
After=network.target freenet-core.service
Requires=freenet-core.service

[Service]
Type=simple
User=stroma
Group=stroma
WorkingDirectory=/opt/stroma
ExecStart=/opt/stroma/stroma run --config /etc/stroma/config.toml
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

```bash
# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable stroma-bot
sudo systemctl start stroma-bot

# Check status
sudo systemctl status stroma-bot
```

### freenet-core Service

```bash
# Create systemd service for freenet-core
sudo nano /etc/systemd/system/freenet-core.service
```

```ini
[Unit]
Description=Freenet Core Node
After=network.target

[Service]
Type=simple
User=stroma
Group=stroma
ExecStart=/usr/local/bin/freenet --mode dark
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

```bash
# Enable and start
sudo systemctl enable freenet-core
sudo systemctl start freenet-core
```

## Monitoring & Maintenance

### View Logs

```bash
# Stroma bot logs
journalctl -u stroma-bot -f

# freenet-core logs
journalctl -u freenet-core -f

# Both services
journalctl -u stroma-bot -u freenet-core -f
```

### Check Health

```bash
# Bot health
curl http://127.0.0.1:9090/health  # If health endpoint implemented

# freenet-core health
curl http://127.0.0.1:8080/health

# Service status
systemctl status stroma-bot freenet-core
```

### Restart Services

```bash
# Restart bot only
sudo systemctl restart stroma-bot

# Restart freenet-core (WARNING: may cause brief state sync delay)
sudo systemctl restart freenet-core

# Restart both
sudo systemctl restart freenet-core stroma-bot
```

## What Operators Cannot Do

### Membership Operations (Automatic Only)
- ❌ Cannot manually add members to Signal group (except 3-member bootstrap)
- ❌ Cannot manually remove members
- ❌ Cannot bypass vetting process
- ❌ Cannot override ejection protocol

**Why**: Bot executes membership changes automatically based on Freenet contract state. Operator has no authority to override.

### Configuration Changes (Requires Group Vote)
- ❌ Cannot change consensus thresholds unilaterally
- ❌ Cannot modify GroupConfig without Signal Poll
- ❌ Cannot approve federation without group vote

**Why**: All configuration stored in Freenet contract, changeable only via group consensus.

### Trust Operations (Cannot Execute)
- ❌ Cannot vouch for people
- ❌ Cannot flag members
- ❌ Cannot view who vouched for whom

**Why**: Operator is a service runner, not a privileged admin.

### Identity Access (Privacy-Protected)
- ❌ Cannot see cleartext Signal IDs (only hashes)
- ❌ Cannot correlate hashes to real identities
- ❌ Cannot view social graph structure

**Why**: Privacy is paramount. Memory dumps contain only hashed identifiers.

## What Operators Can Do

### Service Maintenance (Your Actual Job)
- ✅ Start/stop bot service
- ✅ Monitor logs for errors
- ✅ Restart on crashes (automated via systemd)
- ✅ Ensure bot stays online
- ✅ Update bot software (with group notification)
- ✅ Handle Signal bans (re-register with backup number)

### Monitoring & Auditing
- ✅ View service logs (no cleartext IDs)
- ✅ Monitor resource usage (CPU, memory, network)
- ✅ Check Freenet node health
- ✅ View operator audit trail (members can query with `/audit operator`)

**Your role is to keep the service running. The group governs itself.**

## Troubleshooting

### Bot Not Responding to Commands

**Check:**
```bash
# Is bot service running?
systemctl status stroma-bot

# Is freenet-core running?
systemctl status freenet-core

# Check logs for errors
journalctl -u stroma-bot -n 50
```

**Common Issues:**
- freenet-core node not running → start it
- Signal credentials expired → re-authenticate
- Network connectivity issues → check firewall

### Bot Banned from Signal

**Response:**
1. Register new bot account with backup phone number
2. Update config.toml with new phone number
3. Restart bot service
4. Bot will recover state from freenet-core

**Prevention**: Follow Signal's terms of service, avoid spam-like behavior

### State Sync Issues

**Symptoms:**
- Signal group doesn't match Freenet state
- Members not being added/removed automatically

**Fix:**
```bash
# Force state sync (if implemented)
./stroma sync --force

# Or restart bot (will re-sync on startup)
systemctl restart stroma-bot
```

### Performance Issues

**Symptoms:**
- High CPU usage
- Slow response times
- Memory leaks

**Check:**
```bash
# Monitor resources
htop
journalctl -u stroma-bot | grep -i "error\|warn"

# Check freenet-core health
curl http://127.0.0.1:8080/metrics
```

## Updates & Maintenance

### Updating Stroma Bot

```bash
# Pull latest code
cd /opt/stroma
git pull origin main

# Rebuild
cargo build --release --target x86_64-unknown-linux-musl

# Stop service
sudo systemctl stop stroma-bot

# Replace binary
sudo cp target/x86_64-unknown-linux-musl/release/stroma /usr/local/bin/

# Start service
sudo systemctl start stroma-bot

# Verify
sudo systemctl status stroma-bot
```

**Notify group before updates** (courtesy, not required)

### Updating freenet-core

```bash
# Update freenet-core
cd /path/to/freenet-core
git pull origin main
cargo install --path crates/core

# Restart node (brief sync delay expected)
sudo systemctl restart freenet-core
```

## Security Best Practices

### Operator Security
- ✅ Use strong SSH keys (no password auth)
- ✅ Enable firewall (only Signal + Freenet traffic)
- ✅ Run bot as non-root user
- ✅ Seccomp sandbox for bot process
- ✅ Regular security updates (OS packages)
- ✅ Monitor for unauthorized access

### Pepper Security
- ✅ Store pepper in secure location (`/var/lib/stroma/pepper.secret`)
- ✅ Set restrictive permissions (600)
- ✅ Backup securely (encrypted off-server)
- ❌ NEVER commit to git
- ❌ NEVER share with anyone

### Signal Credentials
- ✅ Store securely (environment variable or config file with restricted permissions)
- ✅ Have backup phone number ready
- ❌ NEVER commit to git

## Disaster Recovery

### Bot Goes Offline

**Impact**: Temporary disruption, state preserved

**Recovery:**
1. Restart bot service
2. Bot re-syncs from freenet-core automatically
3. No data loss (Freenet is persistent)

**Time to Recovery**: < 5 minutes

### freenet-core Node Failure

**Impact**: Bot cannot read/write state

**Recovery:**
1. Restart freenet-core node
2. Wait for state sync (may take minutes)
3. Bot automatically reconnects

**Time to Recovery**: 5-15 minutes

### Server Failure

**Impact**: Total outage

**Recovery:**
1. Set up new server
2. Install freenet-core and Stroma
3. Restore pepper.secret from backup
4. Restore config.toml
5. Start services
6. Bot re-syncs state from Freenet network

**Time to Recovery**: 30-60 minutes

**Critical**: You MUST have `pepper.secret` backup or member hashes won't match!

### Signal Ban

**Impact**: Bot cannot send/receive messages

**Recovery:**
1. Register new Signal account (backup phone number)
2. Update config.toml with new credentials
3. Restart bot
4. Bot continues with new Signal identity

**Note**: May require group notification and re-adding bot to Signal group

## Operator Audit Trail

Members can query your actions with `/audit operator`. They'll see:

```
Operator: @OperatorHash_42

Recent Actions:
- 2026-01-25 08:15 UTC: ServiceRestart (reason: server maintenance)
- 2026-01-20 14:30 UTC: ServiceStart (reason: initial deployment)

Note: Operator has NO special privileges for membership changes.
All bot actions are automatic based on Freenet contract state.
```

This builds trust by demonstrating you're not manipulating the system.

## Costs & Resources

### Server Requirements
- **CPU**: 2 cores (1 for bot, 1 for freenet-core)
- **RAM**: 2GB (1GB for bot, 1GB for freenet-core)
- **Storage**: 10GB (grows with group size)
- **Network**: Stable connection, ~100GB/month bandwidth

### Estimated Monthly Costs
- **VPS**: $10-20/month (DigitalOcean, Linode, Hetzner)
- **Signal**: Free (just need phone number)
- **Total**: $10-20/month

### Scaling
- Small group (3-50 members): 1 CPU, 1GB RAM
- Medium group (50-200 members): 2 CPUs, 2GB RAM
- Large group (200-1000 members): 4 CPUs, 4GB RAM

## Frequently Asked Questions

### Can I run multiple bots on one server?
Yes, but each bot needs its own freenet-core node and Signal account. Run separate systemd services.

### Can I move the bot to a new server?
Yes. Stop services, backup `pepper.secret` and config, move to new server, restore, restart.

### What if I lose the pepper.secret file?
**Critical failure**. All member hashes will be different, breaking the trust network. **Always backup pepper.secret!**

### Can I change the pepper later?
No. Changing pepper invalidates all existing hashes. The pepper must remain constant for the group's lifetime.

### How do I upgrade without downtime?
Currently not supported in MVP. Brief downtime (< 5 minutes) acceptable for updates. Federation (Phase 4+) will enable zero-downtime updates.

### What's my liability as an operator?
You're a service runner, not a controller. The group makes all decisions via Signal Polls. You cannot override or bypass protocols. However, you should follow local laws regarding communication services.

### Can I see who's in the group?
You can see the Signal group members in the Signal app, but the bot only stores/uses hashed identifiers. You cannot correlate hashes to real identities via the bot.

---

## Support & Community

- **Issues**: [GitHub Issues](https://github.com/roder/stroma/issues)
- **Docs**: [Documentation](../README.md)
- **Roadmap**: [TODO.md](TODO.md)

---

**See Also:**
- [User Guide](USER-GUIDE.md) - For group members
- [Developer Guide](DEVELOPER-GUIDE.md) - For contributors
- [Spike Week Briefing](SPIKE-WEEK-BRIEFING.md) - Technology validation
