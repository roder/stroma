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
- **Backup strategy** (for Signal protocol store — see below)

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
# Path to Signal protocol store (CRITICAL - this IS your identity)
# Created during device linking, contains ACI keypair for all crypto operations
store_path = "/var/lib/stroma/signal-store"

# Device name shown in Signal's linked devices list
device_name = "Stroma Bot"

[freenet]
# Freenet node is EMBEDDED - no external service needed
# These settings configure the embedded node

# Network settings for the embedded Freenet node
listen_port = 0  # 0 = OS assigns available port
is_gateway = false

# Gateway nodes to connect to (at least one required)
# Get current gateway addresses from Freenet documentation
gateways = [
    "gateway1.freenetproject.org:12345",
]

# Contract state directory (stores encrypted trust network state)
state_dir = "/var/lib/stroma/freenet-state"

[logging]
level = "info"
file = "/var/log/stroma/bot.log"
EOF
```

**Important Notes:**
- **No phone number needed**: The bot links as a secondary device to YOUR existing Signal account via QR code
- **Freenet is embedded**: No separate Freenet service to run - it's built into Stroma
- **Contract key is automatic**: Created during member-initiated bootstrap, managed by the bot

### Signal Protocol Store (CRITICAL for Recovery)

**Your Signal protocol store IS your recovery identity.** No separate keypair needed.

The bot uses the Signal account's **ACI (Account Identity) key** for ALL cryptographic operations:
- Chunk encryption (AES-256-GCM key derived via HKDF)
- State signatures (using ACI identity key)
- Identity masking (HMAC key derived via HKDF)
- Persistence network identification

**Note**: Group pepper is DEPRECATED. All cryptographic keys are now derived from the Signal ACI identity, simplifying backup to just the Signal protocol store.

```bash
# Signal protocol store location (created during registration)
/var/lib/stroma/signal-store/

# CRITICAL: Backup this directory securely
tar -czf /secure-backup/stroma-signal-store-$(date +%Y%m%d).tar.gz /var/lib/stroma/signal-store/

# Store backup in:
# - Encrypted USB drive in safe location
# - Hardware security module (HSM)
# - Secure cloud backup (encrypted)
# - NOT on the same server as the bot
```

**If you lose this Signal store, you CANNOT recover your trust network.** The ACI identity key inside is the ONLY way to decrypt your fragments.

**What's in the Signal store:**
- ACI identity keypair (your cryptographic identity)
- PNI identity keypair (phone number identity)
- Session keys (for encrypted conversations)
- Pre-keys (for establishing new sessions)

**What's NOT stored (and doesn't need backup):**
- Message history (ephemeral by design)
- Contact database (not used)

See [PERSISTENCE.md](PERSISTENCE.md) for details on the recovery process.

### Persistence Configuration

The Reciprocal Persistence Network ensures your trust map survives bot crashes:

```toml
# Add to config.toml
[persistence]
# Chunk size for state distribution (default: 64KB)
chunk_size = 65536
# Replication factor - copies per chunk (default: 3)
replication_factor = 3
```

**Note**: No separate keypair file or persistence identity needed — your Signal protocol store (configured in `[signal].store_path`) IS your persistence identity. Encryption keys are derived from your Signal ACI identity via HKDF. No heartbeat mechanism required. Replication Health is measured at write time based on successful chunk distribution acknowledgments.

## Signal Account Setup (One-Time)

Before linking Stroma, you need a Signal account. **How you obtain the Signal account is your responsibility** — Stroma only needs to link to it as a secondary device.

### Step 1: Create or Prepare a Signal Account

Choose how you want to set up the Signal account the bot will use:

| Option | Description | Best For |
|--------|-------------|----------|
| **Dedicated phone** | Install Signal on a separate phone with its own number | Production (recommended) |
| **VoIP service** | Register Signal using a virtual number (SMSPool, Twilio, Google Voice) | Testing (may be blocked) |
| **Existing account** | Use your personal Signal account | Development only |

**For production**, we recommend a dedicated Signal account (not your personal one). How you register that account is up to you:
- Prepaid SIM card in a cheap phone
- Dual-SIM phone with second number
- Virtual number via SMSPool, Twilio, Google Voice, etc. (may be blocked by Signal)

---

### Step 2: Link Stroma as Secondary Device

Once you have a Signal account (on any phone), link Stroma to it:

```bash
# 1. Start linking process
stroma link-device \
  --device-name "Stroma Bot" \
  --servers production

# 2. QR code appears in terminal
# ┌───────────────────┐
# │  █▀▀▀▀▀▀▀▀▀▀▀█   │
# │  █ ▄▄▄▄▄ █▀█ █   │
# │  ...QR CODE...   │
# │  █▄▄▄▄▄▄▄▄▄▄▄█   │
# └───────────────────┘
# Alternatively, use the URL: sgnl://linkdevice?uuid=...

# 3. On your phone (the one with the Signal account):
#    Signal → Settings → Linked Devices → Link New Device
#    Scan the QR code

# 4. Linking complete!
# Output: "Device linked. ACI: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
```

**After Linking**:
- The bot inherits the Signal account's identity (ACI)
- Contacts and groups sync from the primary device
- **BACKUP the Signal store immediately** (see section above)
- The bot can perform ALL operations needed for Stroma

---

### Understanding the Device Model

Signal supports **one primary device** (Android/iOS phone) with **up to 5 linked devices**:

| Aspect | Primary Device | Linked Device (Stroma) |
|--------|----------------|------------------------|
| Registration | SMS/Voice verification | QR code scan |
| Identity | Owns phone number | Shares identity |
| Capabilities | Full (incl. link/unlink) | Full messaging & groups |
| What Stroma uses | N/A | ✅ This one |

**Key insight**: Linked devices have **full messaging and group management capabilities** — they can send messages, create groups, add/remove members, and perform admin operations. The only limitation is they cannot link/unlink other devices (which Stroma doesn't need).

---

### Signal Account Considerations
- **Prepaid SIM card** - Most reliable, works anywhere, ~$10-20/month
- **Dual-SIM phone** - Use second slot for bot number
- **VoIP service** (SMSPool, Twilio, Google Voice) - May be blocked by Signal

**Signal Terms of Service**:
- Signal allows bots/automated clients
- Avoid spamming or abusive behavior
- Rate limit your messages (the bot does this automatically)

**If Signal Bans Your Number**:
1. Your trust network data is **safe** (fragments on persistence network)
2. Get a new phone number
3. Link new number as new device to existing Signal account (if possible)
4. If account fully banned: This requires a new ACI identity, see "Signal Ban" in Disaster Recovery section

**CRITICAL**: Your Signal protocol store contains the ACI identity key used to encrypt your persistence fragments. A NEW ACI identity CANNOT decrypt fragments encrypted with the OLD identity. Backup your Signal store before any issues arise.

## Bootstrap Process (Member-Initiated)

### Important: Embedded Freenet
Stroma includes an **embedded Freenet kernel** - you don't need to install or run freenet-core separately. Everything is in one binary.

### Key Principle: Member-Initiated Bootstrap

**Bootstrap is controlled by members, not the operator.** Your role is to start the service - a member will initiate the seed group via Signal commands.

### Step 1: Start the Bot Service

```bash
# Run bot - it will await member-initiated bootstrap
stroma run --config /etc/stroma/config.toml

# Bot starts in "awaiting bootstrap" state:
# ✅ Signal connected
# ✅ Embedded Freenet kernel initialized
# ⏳ Awaiting bootstrap...
#    A member must initiate with: /create-group "Group Name"
```

**Optional**: Prompt a specific user to start bootstrap:
```bash
stroma run --config /etc/stroma/config.toml --bootstrap-contact @FirstMember
```

### Step 2: Member Initiates Bootstrap (Via Signal)

A member (not you) starts the process:

```
Member → Bot (PM): /create-group "Mission Control"

Bot → Member (PM): 
"🌱 Creating new Stroma group: 'Mission Control'

You are seed member #1. Invite 2 more trusted people:
  /add-seed @MemberB
  /add-seed @MemberC"

Member → Bot (PM): /add-seed @Bob
Member → Bot (PM): /add-seed @Carol

Bot → Group:
"🎉 'Mission Control' is now live!
All 3 seed members have 2 vouches each."
```

### Step 3: Bot Completes Automatically

Once 3 seed members are added:
1. Bot creates Signal group with all 3 members
2. Bot deploys Freenet contract with mutual vouches
3. Bot enters normal operation mode
4. Bot announces completion to the group

**After this point, you have NO special privileges. All membership changes are automatic based on Freenet contract state.**

### What You DON'T Do

- ❌ Specify seed members via CLI (member-controlled)
- ❌ Create the Signal group manually (bot creates it)
- ❌ Run any "bootstrap" command with member names
- ✅ Just start the service and let members handle bootstrap

## Deployment Methods

### Method 1: Container Deployment (Recommended)

Container deployment requires a two-phase process:
1. **Link device** (interactive, one-time) - Display QR code for you to scan with Signal
2. **Run service** (daemonized) - Normal operation after linking

#### Phase 1: Link Device (One-Time)

First, link the bot as a secondary device to your Signal account:

```bash
# Create directories for persistent data
mkdir -p ./stroma-data/signal-store
mkdir -p ./stroma-data/freenet-state

# Run linking in interactive mode (displays QR code)
docker run -it --rm \
  -v $(pwd)/stroma-data/signal-store:/data/signal-store \
  ghcr.io/roder/stroma:latest link-device --device-name "Stroma Bot"
```

This displays a QR code in your terminal. Open Signal on your phone:
1. Go to **Settings** → **Linked Devices**
2. Tap **Link New Device**
3. Scan the QR code displayed in the terminal

Once linked, the container exits. Your Signal identity is now stored in `./stroma-data/signal-store/`.

**⚠️ CRITICAL**: Back up `./stroma-data/signal-store/` immediately. This directory contains your ACI identity keypair — losing it means losing access to your trust network forever.

#### Phase 2: Create Configuration

Create `config.toml` with your settings:

```bash
cat > ./stroma-data/config.toml <<'EOF'
[signal]
store_path = "/data/signal-store"
device_name = "Stroma Bot"

[freenet]
listen_port = 0
is_gateway = false
gateways = ["gateway1.freenetproject.org:12345"]
state_dir = "/data/freenet-state"

[logging]
level = "info"
file = "/data/logs/bot.log"

[persistence]
chunk_size = 65536
replication_factor = 3
EOF
```

#### Phase 3: Run Service

##### Using docker-compose (Recommended)

```yaml
# docker-compose.yml
version: '3.8'

services:
  stroma:
    image: ghcr.io/roder/stroma:latest
    container_name: stroma
    restart: unless-stopped
    command: run --config /data/config.toml
    volumes:
      # Signal protocol store - CRITICAL, back this up!
      - ./stroma-data/signal-store:/data/signal-store
      # Freenet state directory
      - ./stroma-data/freenet-state:/data/freenet-state
      # Configuration (read-only)
      - ./stroma-data/config.toml:/data/config.toml:ro
      # Logs
      - ./stroma-data/logs:/data/logs
    security_opt:
      - no-new-privileges:true
    cap_drop:
      - ALL
    read_only: true
    tmpfs:
      - /tmp
```

```bash
# Start service
docker-compose up -d

# View logs
docker-compose logs -f stroma

# Stop service
docker-compose down
```

##### Using docker/podman directly

```bash
# Run service (awaits member-initiated bootstrap if new group)
docker run -d \
  --name stroma \
  --restart unless-stopped \
  -v $(pwd)/stroma-data/signal-store:/data/signal-store \
  -v $(pwd)/stroma-data/freenet-state:/data/freenet-state \
  -v $(pwd)/stroma-data/config.toml:/data/config.toml:ro \
  -v $(pwd)/stroma-data/logs:/data/logs \
  --security-opt no-new-privileges:true \
  --cap-drop ALL \
  --read-only \
  ghcr.io/roder/stroma:latest run --config /data/config.toml

# Optional: prompt a specific user to start bootstrap
docker run -d \
  --name stroma \
  --restart unless-stopped \
  -v $(pwd)/stroma-data/signal-store:/data/signal-store \
  -v $(pwd)/stroma-data/freenet-state:/data/freenet-state \
  -v $(pwd)/stroma-data/config.toml:/data/config.toml:ro \
  -v $(pwd)/stroma-data/logs:/data/logs \
  ghcr.io/roder/stroma:latest run --config /data/config.toml --bootstrap-contact @FirstMember
```

**Note**: 
- Bootstrap is member-initiated via Signal commands (`/create-group`, `/add-seed`), not via CLI
- The embedded Freenet node starts automatically — no separate Freenet service required

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

### Method 3: Multi-Group Deployment

**Architecture**: One bot instance per Stroma group (1:1 relationship)

If you operate multiple Stroma groups, each requires:
- Separate bot process
- Separate Signal account (linked via QR code)
- Separate Freenet contract (created during bootstrap)
- Separate configuration and data directories

**Example: Running 3 Groups**

#### Step 1: Prepare 3 Signal Accounts

Each bot needs its own Signal account. You have several options:

**Option A: Use separate phones/SIMs**
- Get 3 prepaid SIMs
- Install Signal on 3 phones (or use one phone, register each account, then unlink)
- Each account can then have the bot linked as a secondary device

**Option B: Use an SMS-to-email service (SMSPool, etc.)**
- Provision 3 phone numbers for registration
- Complete Signal registration for each
- See the [Signal Account Setup](#signal-account-setup-one-time) section for details

**Option C: Use existing Signal accounts**
- If you have team members willing to link their accounts
- Each person links the bot as a secondary device on their account

#### Step 2: Link Each Bot Instance

For each group, run the device linking process:

```bash
# Create directories for each group
sudo mkdir -p /var/lib/stroma/{mission-control,activists-nyc,mutual-aid-sf}/signal-store
sudo mkdir -p /var/lib/stroma/{mission-control,activists-nyc,mutual-aid-sf}/freenet-state

# Link each bot (run one at a time, scan QR with the corresponding Signal account)
stroma link-device --device-name "Stroma: Mission Control" \
  --store-path /var/lib/stroma/mission-control/signal-store

stroma link-device --device-name "Stroma: Activists NYC" \
  --store-path /var/lib/stroma/activists-nyc/signal-store

stroma link-device --device-name "Stroma: Mutual Aid SF" \
  --store-path /var/lib/stroma/mutual-aid-sf/signal-store
```

**⚠️ CRITICAL**: Back up each `signal-store` directory immediately after linking.

#### Step 3: Create systemd Service Template

```bash
# Create template service file
sudo nano /etc/systemd/system/stroma-bot@.service
```

```ini
[Unit]
Description=Stroma Bot - %i
After=network.target
Wants=network-online.target

[Service]
Type=simple
User=stroma
Group=stroma
WorkingDirectory=/var/lib/stroma/%i
ExecStart=/usr/local/bin/stroma run --config /etc/stroma/groups/%i/config.toml
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/stroma/%i /var/log/stroma

[Install]
WantedBy=multi-user.target
```

#### Step 4: Create Per-Group Configurations

```bash
# Create config directories
sudo mkdir -p /etc/stroma/groups/{mission-control,activists-nyc,mutual-aid-sf}

# Example config for mission-control
cat > /etc/stroma/groups/mission-control/config.toml <<'EOF'
[signal]
store_path = "/var/lib/stroma/mission-control/signal-store"
device_name = "Stroma: Mission Control"

[freenet]
listen_port = 0
is_gateway = false
gateways = ["gateway1.freenetproject.org:12345"]
state_dir = "/var/lib/stroma/mission-control/freenet-state"

[logging]
level = "info"
file = "/var/log/stroma/mission-control.log"

[persistence]
chunk_size = 65536
replication_factor = 3
EOF

# Repeat for other groups with their respective paths
# activists-nyc uses /var/lib/stroma/activists-nyc/...
# mutual-aid-sf uses /var/lib/stroma/mutual-aid-sf/...
```

**Each config has its own:**
- `signal.store_path` — Different Signal store per group (created during linking)
- `freenet.state_dir` — Different Freenet state directory per group
- `logging.file` — Different log file per group

#### Step 5: Start All Services (Each Awaits Bootstrap)

**Note**: Bootstrap is member-initiated. Each bot starts in "awaiting bootstrap" state until a member initiates via Signal (`/create-group`, `/add-seed`).

#### Step 6: Enable and Start Services

```bash
# Enable and start each service
sudo systemctl enable --now stroma-bot@mission-control
sudo systemctl enable --now stroma-bot@activists-nyc
sudo systemctl enable --now stroma-bot@mutual-aid-sf

# Check status of all
sudo systemctl status 'stroma-bot@*'
```

#### Step 7: Monitor All Groups

```bash
# View logs for specific group
journalctl -u stroma-bot@mission-control -f

# View logs for all groups
journalctl -u 'stroma-bot@*' -f

# Check status
systemctl list-units 'stroma-bot@*'
```

**Resource Requirements:**

| Groups | RAM | Storage | Network |
|--------|-----|---------|---------|
| 1 | ~100MB | ~50MB | Minimal |
| 10 | ~1GB | ~500MB | Minimal |
| 100 | ~10GB | ~5GB | Minimal |

**Acceptable for <100 groups on single server.**

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

### Persistence Monitoring

Monitor the Reciprocal Persistence Network health:

```bash
# Check persistence status
stroma persistence-status

# Output:
# ✅ Persistence State: ACTIVE
# ✅ Chunks Replicated: 8/8 (all 3/3 copies)
# ✅ Last Verification: 2h ago
# ✅ Network Size: 47 bots
# ✅ Fragments Held By Us: 5 (from 2 bots)
```

**Persistence States to Watch:**

| Alert | Meaning | Action |
|-------|---------|--------|
| `state=ACTIVE` | Normal operation | None required |
| `state=PROVISIONAL` | No suitable peers (yet) | Wait for network growth |
| `state=DEGRADED` | Lost holder, writes blocked | Bot auto-recovers |
| `state=ISOLATED` | Only bot in network | Consider adding peers |
| `holders<2` | Insufficient fragments | Check network connectivity |
| `verification_failed` | Holder may have deleted | Bot finds replacement |

**Recovery Monitoring:**
```bash
# View recovery logs
journalctl -u stroma | grep -i "persistence\|chunk\|recovery"
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

### Signal Protocol Store Security
- ✅ Store in secure location (`/var/lib/stroma/signal-store/`)
- ✅ Set restrictive permissions (700 on directory, 600 on files)
- ✅ Backup securely (encrypted off-server) — THIS IS YOUR ONLY RECOVERY PATH
- ❌ NEVER commit to git
- ❌ NEVER share with anyone
- ❌ NEVER lose this backup (you cannot decrypt fragments without it)

### Signal Credentials
- ✅ Store securely (environment variable or config file with restricted permissions)
- ✅ Have backup phone number ready
- ❌ NEVER commit to git

## Disaster Recovery

### Bot Goes Offline

**Impact**: Temporary disruption

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
2. Bot reconnects to Freenet network and Signal
3. Trust state recovered from Reciprocal Persistence Network (other bots hold your encrypted fragments)

**Note**: Freenet does NOT guarantee persistence on its own. Data can "fall off" if no peers are subscribed. Stroma's Reciprocal Persistence Network ensures your trust map survives by distributing encrypted fragments to other bots.

**Time to Recovery**: < 5 minutes

### Server Failure

**Impact**: Total outage until new server deployed

**Recovery:**
1. Set up new server
2. Install Stroma (container or binary)
3. Restore Signal protocol store from backup (CRITICAL — your only recovery path)
4. Restore `config.toml` from backup
5. Start service
6. Bot recovers trust state from Reciprocal Persistence Network

**Time to Recovery**: 30-60 minutes

**Critical Requirements:**
- ✅ MUST have Signal protocol store backup (contains ACI identity for decryption)
- ✅ MUST have `config.toml` backup (contract key needed)
- ⚠️ Persistence network must have your fragments (other bots hold them)
- ❌ Without Signal store backup, you CANNOT decrypt your fragments

### Crash Recovery (Automatic)

**Scenario**: Bot crashes or restarts, loses all in-memory state

**What Happens Automatically:**
The bot implements **automatic crash recovery** via the Reciprocal Persistence Network:

1. **On startup**, bot detects it has no local state
2. **Queries registry** to discover which bots hold its fragments
3. **Fetches all chunks** from holders (tries primary, falls back to secondary if unavailable)
4. **Reassembles and decrypts** state using ACI key from Signal protocol store
5. **Verifies signatures** to ensure data integrity
6. **Resumes normal operation** with full trust map intact

**Recovery Flow:**
```
Bot crashes → Loses all state in RAM
           ↓
Bot restarts → "Where's my state?"
           ↓
Query registry → "5 bots exist, I have 8 chunks"
           ↓
Compute holders → Chunk[0] held by bot-A and bot-B
           ↓
Fetch chunks → Try bot-A (success) ✅
           ↓
Decrypt → Use ACI key from Signal store
           ↓
Verify → HMAC signature valid ✅
           ↓
Recovered! → Full trust map restored
```

**Expected Recovery Time:**
- **Small state** (<100KB): <1 second
- **Medium state** (500KB): 1-3 seconds
- **Large state** (5MB): 5-10 seconds

**Fallback Behavior:**
If primary holder unavailable:
```
Try holder-A → Network error ❌
Try holder-B → Success ✅
```

Recovery succeeds if **any 1 of 2 replicas** is available for each chunk.

**Monitoring Recovery:**
```bash
# Watch recovery in real-time
journalctl -u stroma -f | grep -i recovery

# Expected output:
# Feb 03 16:42:10 stroma[1234]: INFO persistence::recovery: Starting state recovery
# Feb 03 16:42:10 stroma[1234]: INFO persistence::recovery: Fetched registry: 5 bots, epoch=3
# Feb 03 16:42:10 stroma[1234]: INFO persistence::recovery: Recovering 8 chunks...
# Feb 03 16:42:11 stroma[1234]: INFO persistence::recovery: Chunk 0/8 recovered
# Feb 03 16:42:11 stroma[1234]: INFO persistence::recovery: Chunk 1/8 recovered
# ...
# Feb 03 16:42:12 stroma[1234]: INFO persistence::recovery: Recovery complete: 8/8 chunks, 958ms
```

**When Recovery Fails:**

| Error | Cause | Action |
|-------|-------|--------|
| `ChunkFetchFailed` | All holders for a chunk are down | Wait and retry |
| `DecryptionFailed` | Wrong ACI key | Restore correct Signal store backup |
| `SignatureVerificationFailed` | Chunk tampered or corrupted | Contact holder to re-send chunk |
| `OwnerNotInRegistry` | Bot never registered | Check bot initialization |
| `InsufficientReplicas` | Network has <3 bots | Wait for more bots to join |

**Verifying Recovery Success:**
```bash
# Check bot is operational
stroma health

# Expected output:
# Status: ACTIVE
# Trust map: 47 members
# Last state update: 2 hours ago
# Replication health: 🟢 Replicated
```

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

# Restart (bot recovers from Reciprocal Persistence Network)
# Container:
docker start stroma

# Binary/systemd:
sudo systemctl start stroma
```

**Process:**
1. Bot starts with fresh Freenet kernel
2. Bot automatically triggers crash recovery (see above)
3. Bot fetches encrypted fragments from other bots in persistence network
4. Bot decrypts using ACI identity from Signal store
5. Trust state fully recovered

**CRITICAL**: This only works if you have your Signal protocol store (contains ACI key for decryption).

**Time to Recovery**: 10-30 minutes (depends on network size and fragment availability)

### Signal Ban

**Impact**: Bot cannot send/receive messages

**MVP Recovery (Manual):**

**Option A: If only phone number is banned (account survives)**
1. Get new phone number
2. Link new device to existing Signal account
3. Update config if needed
4. Restart bot — same ACI identity, can decrypt fragments

**Option B: If account is fully terminated (new ACI identity required)**
1. Register new Signal account (backup phone number)
2. Update config.toml with new credentials
3. Restart bot with NEW Signal store
4. **CRITICAL**: New ACI identity CANNOT decrypt old fragments
5. Trust network must be rebuilt from scratch (re-bootstrap)

**Why Option B requires rebuild**: Your persistence fragments are encrypted with a key derived from your ACI identity. A new Signal account = new ACI = new key. Old fragments cannot be decrypted.

**Prevention**: 
- Follow Signal's terms of service
- Avoid spam-like behavior
- Keep Signal store backup secure (for Option A scenarios)

**Note**: May require group notification and re-adding bot to Signal group

**Future (Phase 4+): Shadow Handover Protocol**

In Phase 4+, Stroma will support automated bot identity rotation via the Shadow Handover Protocol. This will provide cryptographic succession that allows trust context to transfer to a new identity.

See `.beads/federation-roadmap.bead` for protocol specification.

## Operator Audit Trail

### Overview

Stroma implements an **immutable, append-only operator audit trail** that logs all operator-related actions. This transparency mechanism allows group members to verify that operators are not abusing their service maintenance role.

**Key Principle**: Operators are service runners, not privileged admins. The audit trail proves this by showing you have no special powers over membership or trust decisions.

### What Gets Logged

The bot automatically logs all operator-related actions to the Freenet contract:

| Action Type | Description | Examples |
|-------------|-------------|----------|
| **Config Change** | Changes to group configuration via Signal Poll execution | `Updated min_vouches from 2 to 3`<br>`Changed max_flags from 3 to 5` |
| **Restart** | Bot service restarts | `Bot restarted for maintenance`<br>`Service restart after update` |
| **Manual Intervention** | Emergency operator actions | `Emergency ejection override` (rare) |
| **Bootstrap** | Initial group setup actions | `Created group "Mission Control"`<br>`Added seed member #1` |

**What's NOT Logged** (because operators can't do these):
- Member vouches or flags (member-driven, not operator actions)
- Vetting decisions (handled by trust protocol)
- Message content (ephemeral, never stored)

### How Members Query the Audit Trail

Any group member can query the operator audit trail using Signal commands:

```
Member → Bot (PM): /audit operator
```

**Example Output:**
```
📋 Operator Audit Trail

• 2 hours ago — Config Change (01010101…)
  Updated min_vouches from 2 to 3
  (via Signal Poll #42 approved by group)

• 3 days ago — Restart (01010101…)
  Bot restarted for security update

• 1 week ago — Bootstrap (01010101…)
  Created group "Mission Control"

Note: Operator has NO special privileges for membership changes.
All bot actions are automatic based on Freenet contract state.
```

### Privacy Protection

The audit trail uses **hashed identifiers**, not cleartext Signal IDs:

- **Actor hash**: First 4 bytes of MemberHash (e.g., `01010101…`)
- **No real identities**: Even operators can't correlate hashes to real Signal accounts
- **Immutable log**: Entries cannot be deleted or modified after creation

### Query Filters

Members can filter audit entries using command parameters:

```bash
# Show last 10 entries
/audit operator --limit=10

# Show only config changes
/audit operator --type=config

# Show entries from last 7 days
/audit operator --since=7d
```

### Operator Perspective

**As an operator, you cannot view the audit log directly** through server access, because:
1. Audit entries are stored in the Freenet contract (distributed state)
2. The bot only stores MemberHash values, not cleartext identities
3. You must query via `/audit operator` like any other member

**This is by design**: Operators have the same visibility as members, reinforcing that you're a service runner, not a privileged admin.

### Config Change Proposals

When the group approves a configuration change via Signal Poll, the bot:

1. **Executes the change** on the Freenet contract
2. **Creates an audit entry** with:
   - Actor: System actor (all-zeros MemberHash for governance-driven changes)
   - Details: Old and new values (e.g., `Updated min_vouches from 2 to 3`)
3. **Announces to group**: Change confirmation message

**You (the operator) DO NOT execute config changes manually.** The bot does this automatically when polls complete.

### Audit Trail Storage

| Aspect | Implementation |
|--------|----------------|
| **Storage location** | Freenet contract state (distributed) |
| **Persistence** | Replicated via Reciprocal Persistence Network |
| **Ordering** | Chronological by Unix timestamp |
| **Maximum size** | No limit (append-only, never pruned) |
| **Access control** | All group members can query |

### Trust Building

The audit trail builds trust in three ways:

1. **Transparency**: Members see all operator actions in real-time
2. **Immutability**: Historical record cannot be altered
3. **Accountability**: Operators cannot hide actions or claim "the bot did it"

**Expected operator actions:**
- Service restarts (maintenance, updates)
- Bootstrap actions (one-time, during group creation)

**Unexpected operator actions** (should be rare or never):
- Manual interventions
- Frequent config changes (these should come from group polls, not operators)

### Troubleshooting

**Q: Can I view the audit log via server files?**
A: No. The log is in the Freenet contract, not local files. Query via `/audit operator` in Signal.

**Q: Can I delete audit entries?**
A: No. The log is immutable by design. This protects member trust.

**Q: Members are questioning a config change I didn't make.**
A: Config changes come from Signal Polls, not operators. Show them the audit entry—it will show the system actor (00000000…) and reference the poll ID.

**Q: An audit entry shows "Manual Intervention" but I didn't do anything.**
A: Check bot logs. If the entry is legitimate, it means the bot performed an automatic action (e.g., emergency ejection due to flag threshold). If suspicious, investigate for compromise.

---

**Related Documentation:**
- [Trust Model](TRUST-MODEL.md) - How trust decisions are made (no operator control)
- [User Guide](USER-GUIDE.md) - Member commands including `/audit operator`

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
- Each bot needs separate config.toml and Signal protocol store
- Create multiple systemd units (stroma-group-a.service, stroma-group-b.service)
- Each bot has embedded Freenet kernel (no sharing)

### Can I move the bot to a new server?
Yes. Stop service, backup Signal protocol store and `config.toml`, move to new server, restore, restart. Bot will recover state from Reciprocal Persistence Network (other bots hold your encrypted fragments).

### What if I lose the Signal protocol store?
**Critical failure**. Your ACI identity key is used to:
1. Derive the encryption key for your persistence fragments
2. Derive the HMAC key for identity masking

Without it, you CANNOT decrypt your fragments or match member hashes. **Always backup your Signal protocol store!**

### Can I change the Signal account later?
No (in MVP). Changing Signal accounts means a new ACI identity, which:
1. Cannot decrypt fragments encrypted with the old identity
2. Produces different hashes for the same members

The Signal identity must remain constant for the group's lifetime. Phase 4+ Shadow Handover Protocol will address this limitation.

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
- **Roadmap**: [TODO.md](todo/TODO.md)

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
      - ./signal-store:/data/signal-store  # Signal protocol store (backup this!)
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

**3. Create Signal store directory**
```bash
mkdir -p signal-store
chmod 700 signal-store
```

**4. Create config.toml** (use template from Configuration section)

**5. Start service** (awaits member-initiated bootstrap)
```bash
docker-compose up -d

# View logs to see bootstrap status
docker-compose logs -f
```

**Note**: Bootstrap is member-initiated. Once the service starts, a member initiates via Signal (`/create-group "Name"`, then `/add-seed @User` twice).

**6. Verify running**
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

**3. Create Signal store directory**
```bash
sudo mkdir -p /var/lib/stroma/signal-store
sudo chown stroma:stroma /var/lib/stroma/signal-store
sudo chmod 700 /var/lib/stroma/signal-store
```

**4. Create config** (use template, save to `/etc/stroma/config.toml`)

**5. Create systemd service** (use template from Service Management section)

**6. Start service** (awaits member-initiated bootstrap)
```bash
sudo systemctl daemon-reload
sudo systemctl enable stroma
sudo systemctl start stroma
```

**Note**: Bootstrap is member-initiated. Once the service starts, a member initiates via Signal (`/create-group "Name"`, then `/add-seed @User` twice).

**7. Verify running**
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
# Recommended: Use cargo-binstall for faster installation (downloads precompiled binaries)
#   cargo install cargo-binstall
#   cargo binstall cargo-audit cargo-deny
# Or use cargo install (slower, builds from source):
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
- [Spike Week Briefing](spike/SPIKE-WEEK-BRIEFING.md) - Technology validation

---

**Last Updated**: 2026-01-27
