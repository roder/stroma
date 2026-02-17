# Extensible Capability Interface

**Stroma as a trust-gated capability platform.**

---

## The Generalization

Stroma is a decentralized trust network that stores its [trust graph as a Freenet contract](FREENET_IMPLEMENTATION.md) and governs itself through [consensus via Signal Polls](USER-GUIDE.md#Configuration). Currently, the `/propose` command supports two hardcoded sub-interfaces: `stroma` and `signal`.

But the trust network is not the final product -- it is the **substrate** that other capabilities compose onto. [Trust Topology Plans](TRUST-TOPOLOGY-PLATFORM.md) need Freenet contracts and the governance system. [Proof of Impact](MUTUALAI-CONVERGENCE.md) needs Freenet contracts and the governance system. Future capabilities nobody has imagined yet will need the same things.

Rather than hardcoding each new capability into Stroma's `GroupConfig` and `/propose` command, we introduce **one extensible interface**: capabilities.

**This is scope reduction, not scope creep.** One extensible interface replaces N special cases. `GroupConfig` no longer grows a new field for each integration. `/propose` no longer needs new subcommands compiled in. Everything composes through the same manifest pattern.

---

## Two Kinds of Capabilities

| | Contract Capability | Agent Capability |
|---|---|---|
| **Example** | Phyllotaxis trust topology | MutualAI bot |
| **What it is** | A Freenet WASM contract | A vouched Signal bot (separate process) |
| **State** | On Freenet (contract hash) | In the bot's own storage |
| **Activation** | Deploy WASM to Freenet | Register namespace + commands |
| **Runtime** | Sandboxed by Freenet's contract execution | Runs as its own process with its own permissions |
| **Binary** | WASM module (sandboxed) | None needed -- the bot is already running |
| **Discovery** | Manifest at a URL | Stroma PMs the vouched bot with `/capabilities` |

Both types share the same manifest format and the same governance flow (Signal Poll to approve). The difference is where the code runs and where the state lives.

---

## Capability Manifest (v1)

A YAML file that declares what a capability provides. Human-readable, machine-parseable, versionable.

```yaml
manifest_version: 1
type: contract              # or: agent
namespace: phyllotaxis       # unique namespace for /propose commands
description: |
  Phyllotaxis trust topology: golden-angle growth pattern for 
  organic community formation. Anti-clique property ensures no 
  faction dominates. Self-similar at every scale.
url: https://github.com/roder/phyllotaxis/blob/main/README.md

# --- For contract capabilities: WASM module ---
wasm:
  url: https://github.com/roder/phyllotaxis/releases/v1.0.0/phyllotaxis.wasm
  checksum: sha256:0f98f71ca152ca89...

# --- Commands this capability exposes ---
commands:
  - name: enable
    type: binary             # creates Approve/Reject Signal Poll
    
  - name: mode
    type: binary
    options: [on, off, passive]

# --- Config keys registered under this namespace ---
config:
  - key: ring_count
    type: integer
    default: 5
  - key: growth_rate
    type: float
    default: 1.618
```

### Command Types

Three types cover all governance patterns:

| Type | What it does | Example |
|------|-------------|---------|
| **binary** | Creates an Approve/Reject Signal Poll | `/propose phyllotaxis enable` |
| **command** | Direct action -- Stroma bot verifies standing and executes immediately | `/record-impact "delivered 200 lbs squash"` |
| **proposal** | Creates a custom poll with specified options, timeout, quorum | `/propose @mutualai verify-impact ...` |

Most interactions are `command` (immediate, trust-gated). Governance decisions are `binary` or `proposal`.

---

## Activation Flow

### Contract Capability (e.g., Phyllotaxis)

```
1. Any member:  /propose capability https://example.com/phyllotaxis.yml
2. Stroma bot:  Fetches manifest, validates schema
3. Stroma bot:  Creates Signal Poll: "Enable capability: Phyllotaxis? 
                 [description from manifest]"
4. Group:       Votes (existing threshold + quorum rules)
5. If approved: Stroma deploys WASM to Freenet, registers namespace
6. Now available: /propose phyllotaxis enable
                  /propose phyllotaxis mode on
```

### Agent Capability (e.g., MutualAI)

**Prerequisite**: The bot must already be vouched into the trust network via the standard admission flow (`/invite @mutualaibot`, cross-cluster assessment, `/vouch @mutualaibot`). Trust boundary is admission, not the manifest.

```
1. Any member:  /propose capability @mutualaibot
2. Stroma bot:  PMs @mutualaibot with: /capabilities
3. @mutualaibot: Responds with YAML manifest
4. Stroma bot:  Creates Signal Poll: "Enable capability: MutualAI?
                 [description from manifest]"
5. Group:       Votes
6. If approved: Namespace registered, commands available
7. Now available: /record-impact "matched surplus with kitchen"
                  /propose @mutualaibot verify-impact ...
```

---

## Security Model

### No Native Binaries

The original proposal included native binary distribution with checksums. This has been **removed** after design review. The security surface is too large.

- **Contract capabilities**: Run as WASM inside Freenet's sandboxed execution environment. Freenet's `ContractInterface` trait constrains what contracts can do (validate state, merge deltas, summarize). No filesystem access, no network calls, no arbitrary execution.
- **Agent capabilities**: No binary needed. The agent is already a separate process, already vouched into the network. The manifest only declares what commands and config it exposes. Stroma doesn't load or execute anything on the agent's behalf.

### Trust-Gated Admission

Agent bots must be **vouched** before their capabilities can be proposed. The admission flow (cross-cluster vouching, assessor evaluation) is the primary security gate. A human sponsors the bot, stakes their reputation. Another human from a different cluster independently evaluates it. Only then can the bot be proposed as a capability.

### Governance Approval

Capability activation always requires a group vote. Even if a bot is vouched, the group must separately approve loading its capabilities. Two gates: trust (admission) and governance (capability vote).

### WASM Sandboxing

Contract capabilities execute in Freenet's WASM sandbox:
- Cannot access the host filesystem
- Cannot make network calls
- Cannot access other contracts except through Freenet's defined interfaces
- Must implement `ComposableState` (mergeable, commutative deltas)
- State is CBOR-serialized, following Stroma's existing contract patterns

---

## Example #1: Phyllotaxis Trust Topology

The [Trust Topology Platform](TRUST-TOPOLOGY-PLATFORM.md) proposes matchmaking algorithms based on natural growth patterns. Each topology is a **contract capability** -- a WASM module that stores ring assignments, connection patterns, and health metrics on Freenet.

**Manifest:**

```yaml
manifest_version: 1
type: contract
namespace: phyllotaxis
description: |
  Golden-angle trust topology. Members organized into concentric rings 
  following Fibonacci proportions. Anti-clique property ensures maximum 
  diversity of connections. Self-similar at every scale.
url: https://github.com/roder/phyllotaxis/blob/main/README.md
wasm:
  url: https://github.com/roder/phyllotaxis/releases/v1.0.0/phyllotaxis.wasm
  checksum: sha256:a1b2c3d4...
commands:
  - name: enable
    type: binary
  - name: mode
    type: binary
    options: [on, off]
config:
  - key: ring_count
    type: integer
    default: 5
```

**Usage after activation:**

```
/propose phyllotaxis enable     -- group votes to turn on
/propose phyllotaxis mode on    -- group votes to activate
/mesh phyllotaxis               -- view the spiral topology
```

Myriad natural patterns give way to endless possibilities. Mycelial for resource flow, stigmergy for emergent coordination, coral reef for symbiotic mutualism. Each is a separate contract capability, activated by the group, composing onto the same trust network.

---

## Example #2: MutualAI Agent

[MutualAI](MUTUALAI-CONVERGENCE.md) is a collective intelligence system that sits adjacent to Stroma. A MutualAI bot is an **agent capability** -- a vouched Signal bot that provides AI-assisted coordination, Proof of Impact recording, and community memory.

**Prerequisite:** The bot has been vouched into the trust network:

```
/invite @mutualaibot "Community coordination AI, self-hosted Llama"
(assessor evaluates and vouches)
```

**Capability registration:**

```
/propose capability @mutualaibot
```

Stroma PMs @mutualaibot with `/capabilities`. The bot responds with:

```yaml
manifest_version: 1
type: agent
namespace: mutualai
description: |
  MutualAI provides AI-assisted coordination for collective action.
  Records Proof of Impact on a Freenet ledger. Suggests next steps
  based on community history. Treats Signal as a social connector.
url: https://github.com/roder/mutualai/blob/main/README.md
commands:
  - name: enable
    type: binary
  - name: record-impact
    type: command
    description: Record a Proof of Impact claim (bot verifies standing, writes to Freenet)
  - name: verify-impact
    type: proposal
    description: Group votes on whether a claimed impact is verified
    options: [Yes, No]
    timeout: 1d
    quorum: 0.51
  - name: suggest
    type: command
    description: Ask the AI for the most valuable next step
config:
  - key: poi_contract
    type: string
    description: Freenet contract hash for the Proof of Impact ledger
```

**Usage after activation:**

```
/record-impact "Matched 200 lbs surplus squash with Community Kitchen"
/propose mutualai verify-impact --question "Was the delivery completed?"
/mutualai suggest
/propose mutualai poi_contract <freenet-hash>
```

MutualAI can also create Freenet contracts (like the PoI ledger) through Stroma's infrastructure. The agent doesn't need its own Freenet integration -- it uses Stroma's. The `poi_contract` config key stores the contract hash that Stroma manages on the agent's behalf.

---

## How This Changes Stroma's Architecture

### Before (hardcoded)

```
GroupConfig {
    min_vouches: u32,
    max_flags: u32,
    open_membership: bool,
    // ... trust settings ...
    federation_contracts: Vec<ContractHash>,  // hardcoded for federation
    poi_contract: Option<ContractHash>,       // hardcoded for MutualAI
    // Each new integration = new field
}

ProposalSubcommand {
    Config { key, value },
    Stroma { key, value },
    Signal { key, value },
    // Each new integration = new variant
}
```

### After (extensible)

```
GroupConfig {
    min_vouches: u32,
    max_flags: u32,
    open_membership: bool,
    // ... trust settings ...
    capabilities: Vec<ActiveCapability>,      // all integrations
}

ActiveCapability {
    namespace: String,
    manifest: CapabilityManifest,
    contract_hash: Option<ContractHash>,       // for contract capabilities
    agent_member: Option<MemberHash>,          // for agent capabilities
    config: HashMap<String, String>,           // capability-specific config
}

// /propose <namespace> <command> works for any registered capability
// No new variants needed -- the manifest defines the commands
```

Stroma's core stays small. Capabilities compose onto it. The group decides what to enable.

---

## Relationship to Other Documents

- **[Trust Topology Platform](TRUST-TOPOLOGY-PLATFORM.md)**: Trust topologies are contract capabilities. Each topology provides its own WASM contract, health metric, and matchmaking algorithm. Activated via `/propose capability <url>`.
- **[MutualAI Convergence](MUTUALAI-CONVERGENCE.md)**: The interface between Stroma and MutualAI is the capability manifest. MutualAI is an agent capability. The PoI ledger is a Freenet contract managed through the capability's `poi_contract` config key.
- **[Federation](FEDERATION.md)**: Federation contracts could themselves be capabilities -- proposed, voted on, and activated through the same mechanism.

---

## Design Principles

**One interface, not N**: Every new integration uses the same manifest + governance flow. No compile-time changes to Stroma.

**Trust first, capability second**: Agents must be vouched before they can be proposed as capabilities. Two gates: admission (trust) and activation (governance).

**No native execution**: WASM for contracts (sandboxed), nothing for agents (they run themselves). Stroma never loads or executes arbitrary code.

**Community decides**: Capability activation always requires a group vote. The group controls what its trust network is used for.

**Namespace isolation**: Each capability owns its namespace. `/propose phyllotaxis ...` and `/propose mutualai ...` cannot collide. Config keys are scoped to the capability.

---

*Last Updated: 2026-02-17*
