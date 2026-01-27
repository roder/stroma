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

## Operator Threat Model

**Your Responsibility**: Protect the infrastructure from seizure while understanding you can't compromise member identities even if coerced.

### If You Are Compromised

**Scenario**: Police/adversary seizes your server or coerces you to hand over data.

**What They Get**:
- Hashed identifiers (can't be reversed to real identities)
- Group size and connection topology (but not who people actually are)
- Vouch counts (but not relationship details)

**What They DON'T Get**:
- Member identities (never stored in cleartext)
- Trust map in usable form (distributed across Freenet network)
- Vetting conversation history (ephemeral, deleted after admission)

**Three-Layer Defense** (built into the system):
1. **No Centralized Storage**: Trust map distributed across Freenet peers
2. **Cryptographic Privacy**: Only hashes in memory/storage, immediate zeroization
3. **Metadata Isolation**: All vetting in 1-on-1 PMs, you can't manually export data

**Your Job**: Keep the service running. The architecture protects members even if you're compromised.

## Prerequisites

### Required
- **Linux server** (VPS or dedicated, or Docker/Podman)
- **Signal account** (phone number required for bot)
- **Stable internet connection** (for embedded Freenet kernel and Signal)

### For Binary Installation
- **No Rust required** (download pre-built binary)
- **systemd** (recommended for auto-restart)

### For Container Installation
- **Docker or Podman** (container runtime)
- **docker-compose** (optional, for easier management)

### For Source Build (Advanced)
- **Rust 1.93+** (musl 1.2.5 with improved DNS)
- **Build tools** (gcc, make, etc.)

### Recommended for All Methods
- **Monitoring** (journalctl, logs, or prometheus)
- **Backup phone number** (in case of Signal ban)
- **Backup strategy** (for pepper.secret)

## Installation

**Choose your installation method based on your security/ease preference:**

### Method 1: Container Installation (Recommended for Most Operators)

**Easiest deployment with minimal tradeoff:**

```bash
# Pull verified image (wraps the same static binary)
docker pull ghcr.io/roder/stroma:latest

# Verify image signature (cosign)
cosign verify ghcr.io/roder/stroma:latest

# Run container
docker run -d \
  --name stroma \
  --restart unless-stopped \
  -v stroma-data:/data \
  -e SIGNAL_PHONE="+1234567890" \
  ghcr.io/roder/stroma:latest
```

**What's Inside the Container:**
- Same `stroma-x86_64-musl` static binary as Method 2
- Minimal distroless base (no shell, no package manager)
- Non-root user
- Read-only root filesystem

**Attack Surface**: Static binary + container runtime (~100KB overhead)  
**Security**: Very high (hardened container wraps secure binary)  
**Ease**: Maximum (single command)

→ **[Container Deployment Guide](#container-deployment)** below

---

### Method 2: Static Binary Installation (Maximum Security)

**Minimal attack surface, requires systemd setup:**

```bash
# Download verified release binary
wget https://github.com/roder/stroma/releases/download/v1.0.0/stroma-x86_64-musl
wget https://github.com/roder/stroma/releases/download/v1.0.0/stroma-x86_64-musl.sha256
wget https://github.com/roder/stroma/releases/download/v1.0.0/stroma-x86_64-musl.asc

# Verify checksum
sha256sum -c stroma-x86_64-musl.sha256

# Verify GPG signature
gpg --recv-keys [STROMA_GPG_KEY]
gpg --verify stroma-x86_64-musl.asc stroma-x86_64-musl

# Install
chmod +x stroma-x86_64-musl
sudo mv stroma-x86_64-musl /usr/local/bin/stroma

# Verify installation
stroma version
```

**What You Get:**
- Single static binary with embedded Freenet kernel
- No external dependencies
- No Rust installation required
- GPG-signed and checksummed

**Attack Surface**: Minimal (static binary only)  
**Security**: Maximum  
**Ease**: Medium (requires systemd configuration)

→ **[Binary Deployment Guide](#binary-deployment)** below

---

### Method 3: Build from Source (For Auditors/Developers)

**Maximum control and auditability:**

```bash
# Install Rust 1.93+ (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable
rustup target add x86_64-unknown-linux-musl

# Clone and verify source
git clone https://github.com/roder/stroma.git
cd stroma
git verify-commit HEAD  # Verify signed commit

# Audit dependencies
cargo audit
cargo deny check

# Build static binary (includes embedded Freenet kernel)
cargo build --release --target x86_64-unknown-linux-musl

# Binary location
./target/x86_64-unknown-linux-musl/release/stroma
```

**What You Build:**
- Same binary as distributed in Method 2
- Reproducible build (can verify against released binary)
- Full source audit possible

**Attack Surface**: Minimal (same as Method 2)  
**Security**: Maximum (verified source)  
**Ease**: Difficult (requires Rust knowledge)

→ **[Source Build Guide](#source-build)** below

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

### Important: Embedded Freenet
Stroma includes an **embedded Freenet kernel** - you don't need to install or run freenet-core separately. Everything is in one binary.

### Step 1: Create Seed Group (Manual - 3 Members)

```bash
# You need 3 people to start (including yourself)
# Manually create Signal group and add all 3 members
```

**Seed Group Requirements:**
- Must be exactly 3 members
- All 3 must trust each other
- All 3 will become initial Validators

### Step 2: Run Bootstrap Command

```bash
# Bootstrap initializes embedded Freenet kernel + contract
stroma bootstrap \
  --config /etc/stroma/config.toml \
  --signal-phone "+1234567890" \
  --seed-members @Alice,@Bob,@Carol \
  --group-name "My Trust Network"

# Bootstrap process:
# 1. Initializes embedded Freenet kernel (dark mode)
# 2. Hashes all 3 Signal IDs with group pepper
# 3. Creates initial vouch graph (triangle: everyone vouches everyone)
# 4. Deploys TrustNetworkState contract to embedded kernel
# 5. Writes contract_key to config.toml
# 6. Outputs contract key and confirmation
```

**Output:**
```
✅ Bootstrap Complete

Embedded Freenet kernel initialized (dark mode)
Contract deployed: 0x123abc...
Contract key written to /etc/stroma/config.toml

Seed members:
- @Alice (hashed: 0xabc...)
- @Bob (hashed: 0xdef...)  
- @Carol (hashed: 0x789...)

All 3 members have 2 vouches each (initial triangle).

Ready to start bot service.
```

### Step 3: Start Bot Service

```bash
# Run bot (embedded Freenet kernel starts automatically)
stroma run --config /etc/stroma/config.toml

# Bot is now:
# - Running embedded Freenet kernel
# - Monitoring Freenet state stream
# - Connected to Signal
# - Ready for member commands
```

**After this point, you have NO special privileges. All membership changes are automatic based on Freenet contract state.**

## Deployment Methods

### Method 1: Container Deployment (Recommended)

#### Using docker-compose

```yaml
# docker-compose.yml
version: '3.8'

services:
  stroma:
    image: ghcr.io/roder/stroma:latest
    container_name: stroma
    restart: unless-stopped
    volumes:
      - stroma-data:/data
      - ./config.toml:/data/config.toml:ro
      - ./pepper.secret:/data/pepper.secret:ro
    environment:
      - SIGNAL_PHONE=+1234567890
    security_opt:
      - no-new-privileges:true
    cap_drop:
      - ALL
    read_only: true
    tmpfs:
      - /tmp

volumes:
  stroma-data:
    driver: local
```

```bash
# Start service
docker-compose up -d

# View logs
docker-compose logs -f stroma

# Stop service
docker-compose down
```

#### Using docker/podman directly

```bash
# Bootstrap (one-time)
docker run --rm \
  -v stroma-data:/data \
  ghcr.io/roder/stroma:latest bootstrap \
  --signal-phone "+1234567890" \
  --seed-members @Alice,@Bob,@Carol \
  --group-name "My Network"

# Run service
docker run -d \
  --name stroma \
  --restart unless-stopped \
  -v stroma-data:/data \
  ghcr.io/roder/stroma:latest run
```

---

### Method 2: Binary Deployment (systemd)

#### Create systemd Service

```bash
# Create systemd service file
sudo nano /etc/systemd/system/stroma.service
```

```ini
[Unit]
Description=Stroma Trust Network Bot (with Embedded Freenet)
After=network.target
Wants=network-online.target

[Service]
Type=simple
User=stroma
Group=stroma
WorkingDirectory=/var/lib/stroma
ExecStart=/usr/local/bin/stroma run --config /etc/stroma/config.toml
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/stroma /var/log/stroma

[Install]
WantedBy=multi-user.target
```

```bash
# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable stroma
sudo systemctl start stroma

# Check status
sudo systemctl status stroma
```

**Note**: Single service (no separate freenet-core service needed - it's embedded)

## Monitoring & Maintenance

### View Logs

#### Container Deployment
```bash
# View logs
docker logs -f stroma

# Or with docker-compose
docker-compose logs -f stroma
```

#### Binary Deployment (systemd)
```bash
# View bot logs (includes embedded Freenet kernel logs)
journalctl -u stroma -f

# View recent logs
journalctl -u stroma -n 100

# Follow logs with grep
journalctl -u stroma -f | grep -i "error\|warn"
```

### Check Health

```bash
# Using CLI command
stroma status

# Output:
# ✅ Bot Status: Running
# ✅ Embedded Freenet Kernel: Active (dark mode)
# ✅ Signal Connection: Connected
# ✅ Contract State: Synced
# 
# Group: My Trust Network
# Members: 47
# Contract: 0x123abc...
# Uptime: 3 days, 5 hours
```

#### Container Deployment
```bash
# Container health
docker ps | grep stroma

# Logs
docker logs stroma --tail 50
```

#### Binary Deployment (systemd)
```bash
# Service status
systemctl status stroma

# Active check
systemctl is-active stroma
```

### Restart Service

#### Container Deployment
```bash
# Restart container
docker restart stroma

# Or with docker-compose
docker-compose restart stroma
```

#### Binary Deployment (systemd)
```bash
# Restart service (includes embedded Freenet kernel)
sudo systemctl restart stroma

# Note: Brief state sync delay (seconds) as kernel reconnects
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

#### Container Deployment
```bash
# Is container running?
docker ps | grep stroma

# Check logs
docker logs stroma --tail 100

# Restart if needed
docker restart stroma
```

#### Binary Deployment (systemd)
```bash
# Is service running?
systemctl status stroma

# Check logs
journalctl -u stroma -n 100

# Restart if needed
sudo systemctl restart stroma
```

**Common Issues:**
- Signal credentials expired → re-authenticate
- Network connectivity issues → check firewall
- Embedded Freenet kernel startup failure → check data_dir permissions

### Bot Banned from Signal

**Response:**
1. Register new bot account with backup phone number
2. Update config.toml with new phone number
3. Restart service
4. Bot will recover state from embedded Freenet kernel

**Prevention**: Follow Signal's terms of service, avoid spam-like behavior

### State Sync Issues

**Symptoms:**
- Signal group doesn't match Freenet state
- Members not being added/removed automatically

**Fix:**
```bash
# Check embedded kernel status
stroma status

# Force state sync (if implemented)
stroma sync --force

# Or restart (will re-sync on startup)
# Container:
docker restart stroma

# Binary/systemd:
sudo systemctl restart stroma
```

### Performance Issues

**Symptoms:**
- High CPU usage
- Slow response times
- Memory leaks

**Check:**

#### Container
```bash
# Monitor container resources
docker stats stroma

# Check logs for errors
docker logs stroma | grep -i "error\|warn"
```

#### Binary/systemd
```bash
# Monitor system resources
htop
systemctl status stroma

# Check logs
journalctl -u stroma | grep -i "error\|warn"
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

**Impact**: Temporary disruption, state preserved in embedded Freenet kernel

**Recovery:**

#### Container
```bash
docker restart stroma
```

#### Binary/systemd
```bash
sudo systemctl restart stroma
```

**Process:**
1. Bot restarts with embedded Freenet kernel
2. Kernel re-syncs from Freenet network automatically
3. No data loss (Freenet persistence embedded)

**Time to Recovery**: < 5 minutes

### Server Failure

**Impact**: Total outage until new server deployed

**Recovery:**
1. Set up new server
2. Install Stroma (container or binary)
3. Restore `pepper.secret` from backup (CRITICAL)
4. Restore `config.toml` from backup
5. Start service
6. Embedded kernel re-syncs state from Freenet network

**Time to Recovery**: 30-60 minutes

**Critical Requirements:**
- ✅ MUST have `pepper.secret` backup (member hashes won't match without it)
- ✅ MUST have `config.toml` backup (contract key needed)
- ⚠️ Network must have other nodes with contract state (or state is lost)

### Embedded Kernel Data Loss

**Symptoms:**
- `/var/lib/stroma/freenet` directory corrupted or deleted
- Kernel fails to start

**Recovery:**
```bash
# Stop service
# Container:
docker stop stroma

# Binary/systemd:
sudo systemctl stop stroma

# Remove corrupted data
rm -rf /var/lib/stroma/freenet

# Restart (kernel will re-sync from network)
# Container:
docker start stroma

# Binary/systemd:
sudo systemctl start stroma
```

**Time to Recovery**: 10-30 minutes (depends on network size and peer availability)

### Signal Ban

**Impact**: Bot cannot send/receive messages

**MVP Recovery (Manual):**
1. Register new Signal account (backup phone number)
2. Update config.toml with new credentials
3. Restart bot
4. Bot continues with new Signal identity

**Note**: May require group notification and re-adding bot to Signal group

**Future (Phase 4+): Shadow Handover Protocol**

In Phase 4+, Stroma will support automated bot identity rotation via the Shadow Handover Protocol:

```bash
# Future command (not available in MVP)
stroma rotate \
  --config /etc/stroma/config.toml \
  --new-phone "+0987654321" \
  --reason "Signal ban recovery"
```

**Shadow Handover Benefits**:
- Cryptographic proof of succession (old bot signs handover to new bot)
- Trust context preserved (members' vouches unchanged)
- Freenet contract validates transition (decentralized, not operator assertion)
- Seamless for members (bot announces identity change automatically)

See `.beads/federation-roadmap.bead` for protocol specification.

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

### Server Requirements (Embedded Kernel)
- **CPU**: 2 cores (bot + embedded Freenet kernel in single process)
- **RAM**: 1.5-2GB (single process with embedded kernel)
- **Storage**: 10GB (grows with group size)
- **Network**: Stable connection, ~100GB/month bandwidth

### Estimated Monthly Costs
- **VPS**: $10-20/month (DigitalOcean, Linode, Hetzner)
- **Signal**: Free (just need phone number)
- **Total**: $10-20/month

### Scaling (Single Binary with Embedded Kernel)
- Small group (3-50 members): 1 CPU, 1GB RAM
- Medium group (50-200 members): 2 CPUs, 2GB RAM
- Large group (200-1000 members): 4 CPUs, 4GB RAM

**Note**: Embedded kernel is more efficient than running separate processes (shared memory, no IPC overhead)

## Frequently Asked Questions

### Which deployment method should I use?
- **Most operators**: Container (easy deployment, wraps same secure binary)
- **Security-focused**: Static binary (absolute minimal attack surface)
- **Auditors/developers**: Source build (full control and verification)

**All methods use the same secure static binary** - container just wraps it for ease.

### Is the container less secure than standalone binary?
No significant difference. The container contains the **exact same static binary**. Attack surface difference is ~100KB of well-audited container runtime. We don't compromise member security for operator ease.

### Can I run multiple bots on one server?

**Container**: Yes, easy
```bash
docker run -d --name stroma-group-a -v data-a:/data ghcr.io/roder/stroma:latest
docker run -d --name stroma-group-b -v data-b:/data ghcr.io/roder/stroma:latest
```

**Binary/systemd**: Yes, but need separate services
- Each bot needs separate config.toml and pepper.secret
- Create multiple systemd units (stroma-group-a.service, stroma-group-b.service)
- Each bot has embedded Freenet kernel (no sharing)

### Can I move the bot to a new server?
Yes. Stop service, backup `pepper.secret` and `config.toml`, move to new server, restore, restart. Embedded Freenet kernel will re-sync state from network.

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

---

## Appendix: Detailed Deployment Guides

### Container Deployment (Full Guide)

#### Using docker-compose (Recommended)

**1. Create deployment directory**
```bash
mkdir -p ~/stroma
cd ~/stroma
```

**2. Create docker-compose.yml**
```yaml
version: '3.8'

services:
  stroma:
    image: ghcr.io/roder/stroma:latest
    container_name: stroma
    restart: unless-stopped
    volumes:
      - stroma-data:/data
      - ./config.toml:/data/config.toml:ro
      - ./pepper.secret:/data/pepper.secret:ro
    environment:
      - TZ=UTC
    security_opt:
      - no-new-privileges:true
    cap_drop:
      - ALL
    read_only: true
    tmpfs:
      - /tmp:size=64M,mode=1777

volumes:
  stroma-data:
    driver: local
```

**3. Generate pepper**
```bash
openssl rand -base64 32 > pepper.secret
chmod 600 pepper.secret
```

**4. Create config.toml** (use template from Configuration section)

**5. Bootstrap**
```bash
docker-compose run --rm stroma bootstrap \
  --config /data/config.toml \
  --signal-phone "+1234567890" \
  --seed-members @Alice,@Bob,@Carol \
  --group-name "My Network"
```

**6. Start service**
```bash
docker-compose up -d

# View logs
docker-compose logs -f
```

---

### Binary Deployment (Full Guide)

**1. System preparation**
```bash
# Create user and directories
sudo useradd -r -s /bin/false -d /var/lib/stroma -m stroma
sudo mkdir -p /var/log/stroma /etc/stroma
sudo chown stroma:stroma /var/lib/stroma /var/log/stroma
```

**2. Install binary**
```bash
# Download
cd /tmp
wget https://github.com/roder/stroma/releases/download/v1.0.0/stroma-x86_64-musl
wget https://github.com/roder/stroma/releases/download/v1.0.0/stroma-x86_64-musl.sha256
wget https://github.com/roder/stroma/releases/download/v1.0.0/stroma-x86_64-musl.asc

# Verify
sha256sum -c stroma-x86_64-musl.sha256
gpg --verify stroma-x86_64-musl.asc stroma-x86_64-musl

# Install
sudo install -m 755 stroma-x86_64-musl /usr/local/bin/stroma
```

**3. Generate pepper**
```bash
sudo -u stroma openssl rand -base64 32 > /var/lib/stroma/pepper.secret
sudo chmod 600 /var/lib/stroma/pepper.secret
```

**4. Create config** (use template, save to `/etc/stroma/config.toml`)

**5. Bootstrap**
```bash
sudo -u stroma stroma bootstrap \
  --config /etc/stroma/config.toml \
  --signal-phone "+1234567890" \
  --seed-members @Alice,@Bob,@Carol \
  --group-name "My Network"
```

**6. Create systemd service** (use template from Service Management section)

**7. Start service**
```bash
sudo systemctl daemon-reload
sudo systemctl enable stroma
sudo systemctl start stroma
sudo systemctl status stroma
```

---

### Source Build (Full Guide)

**1. Install Rust 1.93+**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup update stable
rustc --version  # Verify 1.93+
```

**2. Add MUSL target**
```bash
rustup target add x86_64-unknown-linux-musl

# Install MUSL tools
# Ubuntu/Debian:
sudo apt install musl-tools

# Fedora/RHEL:
sudo dnf install musl-gcc
```

**3. Clone and audit**
```bash
git clone https://github.com/roder/stroma.git
cd stroma

# Install audit tools
cargo install cargo-audit cargo-deny

# Run audits
cargo audit
cargo deny check
```

**4. Build**
```bash
# Build static binary (includes embedded Freenet kernel)
cargo build --release --target x86_64-unknown-linux-musl

# Binary will be at:
# target/x86_64-unknown-linux-musl/release/stroma
```

**5. Optional: Verify reproducible build**
```bash
# Your build should match official release
sha256sum target/x86_64-unknown-linux-musl/release/stroma

# Compare to official checksum
curl -L https://github.com/roder/stroma/releases/download/v1.0.0/stroma-x86_64-musl.sha256
```

**6. Install and deploy** (follow Binary Deployment steps)

---

## Security Analysis: Single Binary, Two Distributions

### The Key Insight

**We build ONE artifact, distribute TWO ways:**

```
┌──────────────────────────────────┐
│  Build Phase (GitHub Actions)   │
├──────────────────────────────────┤
│  cargo build --release --target │
│  x86_64-unknown-linux-musl      │
│                                  │
│  Output: stroma-x86_64-musl     │
│  (Static binary with embedded   │
│   Freenet kernel)                │
└──────────────────────────────────┘
              │
              ├──────────────────┬──────────────────┐
              ▼                  ▼                  ▼
    ┌─────────────────┐  ┌─────────────┐  ┌────────────────┐
    │  Distribution 1 │  │Distribution2│  │ Distribution 3 │
    │  Static Binary  │  │  Container  │  │  Source Code   │
    ├─────────────────┤  ├─────────────┤  ├────────────────┤
    │ + GPG sign      │  │ FROM scratch│  │ git clone      │
    │ + SHA256        │  │ COPY binary │  │ cargo build    │
    │ → GitHub Release│  │ → GHCR      │  │ → User builds  │
    └─────────────────┘  └─────────────┘  └────────────────┘
```

### Security Properties

**All three methods provide THE SAME binary:**
- Same static MUSL compilation
- Same embedded Freenet kernel
- Same security properties
- Verifiable via checksums

**Container is NOT a security compromise:**
- Contains the exact same binary as standalone
- Adds ~100KB of well-audited container runtime (containerd/runc)
- Distroless base has NO shell, NO package manager
- Just a packaging convenience

**Operators choose ease vs absolute minimal:**
- Container: Binary + 100KB runtime = 99.9% secure, 100% easy
- Standalone: Binary only = 100% secure, 80% easy

**Members' security is never compromised** - same binary in both cases.

---

**See Also:**
- [User Guide](USER-GUIDE.md) - For group members
- [Developer Guide](DEVELOPER-GUIDE.md) - For contributors
- [Spike Week Briefing](SPIKE-WEEK-BRIEFING.md) - Technology validation

---

**Last Updated**: 2026-01-27
