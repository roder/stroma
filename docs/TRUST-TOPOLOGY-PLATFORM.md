# Trust Topology as a Platform

**The Long Horizon: What Comes After Federation**

---

## Roadmap: Where This Fits

Stroma's development follows a progression from individual groups to interconnected trust networks to something genuinely new -- a platform for experimenting with how trust organizes itself.

```
Phase 0-3: Single Group (NOW)
  Build the core trust protocol. One group, one bot, one contract.
  [DVR](ALGORITHMS.md#network-health-metrics-distinct-validator-ratio-dvr)-optimized admission, mesh health, proposals, persistence.
  
      |
      v

Phase 4-5: Federation (NEXT)
  Connect groups through shared members. Trust spans communities.
  Emergent discovery, blind rendezvous, cross-mesh vouching.
  
      |
      v

Phase 6+: Trust Topology Platform (VISION)
  Groups choose HOW trust organizes -- not just who is trusted,
  but what shape the web of trust takes. Different shapes produce
  different social experiences. The protocol becomes a laboratory.
```

**Current status**: Phases 0-3 are in active development. Federation (Phase 4-5) is the north star guiding all architectural decisions. Trust Topology is the horizon beyond federation -- the place where Stroma stops being just a tool and becomes something no one has built before.

---

## The Idea

Today, Stroma organizes trust for one purpose: **security**. The DVR algorithm creates networks that resist infiltration and single points of failure. It does this well. But it only answers one question: *"Is this network resilient?"*

Trust Topology asks a different question: **"What kind of community does this trust structure create?"**

We know from other fields that the *shape* of connections matters as much as their existence:

- **Urban planning**: Street layout shapes whether neighbors know each other. Cul-de-sacs isolate. Grids connect. Mixed-use integrates.
- **Ecology**: Ecosystem topology determines resilience to disruption. A forest with deep root networks survives storms that topple isolated trees.
- **Organizational theory**: Reporting structures shape culture, innovation, and morale. Flat organizations feel different from hierarchical ones.

Nobody has tested whether **deliberately organizing human trust relationships along natural growth patterns** produces different social outcomes than engineering them for optimal redundancy.

Stroma can be that test.

---

## How It Works

### The Core Architecture: Separate Graphs, Shared Identity

When a group activates a trust topology, it creates a **second trust graph** alongside the primary one. Same people, different relationships, different purpose.

```
Primary Contract (DVR)              Topology Contract (e.g., Phyllotaxis)
+-----------------------+           +---------------------------+
| Security Layer        |           | Community Formation Layer |
|                       |           |                           |
| - Admission vetting   |           | - Strategic introductions |
| - Standing tracking   |           | - Growth pattern          |
| - Ejection protocol   |           | - Topology-specific       |
| - DVR health metric   |           |   health metric           |
|                       |  shared   |                           |
| Members: {A,B,C,D,E}  |<-------->| Members: {A,B,C,D,E}     |
| (source of truth)     | identity | (mirrors primary)         |
+-----------------------+           +---------------------------+
```

**Key properties:**

- The **primary contract** remains the authority for membership. It handles admission, ejection, and standing. Security is never compromised.
- The **topology contract** is an overlay. It organizes the same members into a different pattern and suggests different introductions based on that pattern.
- Members exist in **both graphs simultaneously**. Your DVR graph might say you're a bridge between two clusters. Your topology graph might say you're in the third ring of a spiral. Both are true.
- If you're **ejected** from the primary contract, you're automatically removed from all topology contracts. Security always wins.

### Activating a Topology

A group chooses its topology through the existing proposal system:

```
/propose stroma phyllotaxis true
```

The group votes. If approved, the bot deploys a new Freenet contract for that topology and begins suggesting introductions based on its algorithm. The primary DVR system continues unchanged.

### Viewing Your Network

The `/mesh` command shows the primary DVR health view (unchanged). Adding a topology name shows that topology's view:

```
/mesh                  -- DVR security view (existing)
/mesh phyllotaxis      -- Phyllotaxis spiral view (new)
```

Each topology defines its own health metric and visualization, so members can see how the network looks through different lenses.

---

## The First Experiment: Phyllotaxis

**Phyllotaxis** (Greek: *phyllon* "leaf" + *taxis* "arrangement") is the science of how growth arranges itself in nature. Sunflower seed heads, nautilus shells, pinecone spirals -- they all follow the same mathematical pattern.

The core of that pattern is the **golden angle**: approximately 137.5 degrees. It's the most irrational angle -- the one that is maximally far from any simple fraction of a full circle. When successive elements are placed at this angle, they spread out as evenly as possible, never forming lines or clusters.

### What Phyllotaxis Trust Looks Like

Instead of the DVR algorithm's focus on non-overlapping voucher sets, the phyllotaxis algorithm organizes members into concentric **rings** based on their depth of connection to the network:

- **Ring 0 (core)**: 1-2 deeply embedded founding members
- **Ring 1**: A small group of well-connected early members
- **Ring 2, 3, 4...**: Successively larger rings, each approximately 1.618 times the size of the previous (the golden ratio)

The golden angle determines *which* members in the inner ring connect to *which* members in the outer ring, ensuring maximum spread and zero clustering.

```
         Ring 3 (outermost -- newest members)
       /                                     \
      Ring 2
     /        \
    Ring 1      ---  connections follow
   /     \          the golden angle
  Ring 0
  (core)
```

### The Health Metric: Fractal Deviation Score (FDS)

Just as DVR measures how well the network resists infiltration, the **Fractal Deviation Score** measures how closely the network matches the ideal spiral structure:

```
FDS = 1 - (deviation from ideal ring sizes) / total members
```

| FDS Range | Status | Meaning |
|-----------|--------|---------|
| 0% - 33% | Unstructured | Network hasn't formed spiral pattern yet |
| 33% - 66% | Forming | Spiral is emerging, some rings under/oversized |
| 66% - 100% | Spiral | Network closely matches golden ratio proportions |

### What Members Experience

- **New members** join at the outermost ring and gradually spiral inward as they build deeper trust relationships. This creates a natural metaphor: deepening trust feels like being drawn toward the center of something alive.

- **Strategic introductions** feel organic rather than mechanical. Instead of "The bot needs you to vouch for someone to fix a metric," the message is: "The network suggests you connect with someone who would strengthen the web's growth pattern."

- **The spiral visualization** (via `/mesh phyllotaxis`) shows your position in the network's growth. You can see where you are, where the newest growth is happening, and where connections are thin. The shape itself communicates health.

### Why It Matters Sociologically

The golden angle has a remarkable property: **it prevents cliques**. Because each new connection is placed at the most irrational angle possible, trust can never cluster. Every sub-group of the network has the same structural properties as the whole.

This means **the culture of the whole is present at every scale**. A group of 5 members within a phyllotaxis network has the same diversity of connection as the full network of 500. If the whole network is resilient and well-connected, every subset is too.

This is how organisms grow. A nautilus doesn't remodel its shell when it adds a chamber -- it extends the spiral. The new growth relates to the existing structure in the same proportional way as every previous addition. Growth doesn't disrupt what came before.

---

## The Vision: Multiple Topologies, Each Tied to Nature

Phyllotaxis is the first experiment. The platform is designed so groups can eventually choose from multiple topologies, each inspired by a different natural pattern and each exploring a different aspect of how trust works. Patterns extend from natural dualities and tensions.

Stroma's [philosophical foundations](../.beads/philosophical-foundations.bead) identify six core dualities that the system holds in tension. Each topology is an experiment in how one of those dualities plays out in practice:

| Topology | Natural Pattern | What It Investigates |
|----------|----------------|---------------------|
| *Primary (DVR)* | *Engineered resilience* | *Trust vs Anonymity* |
| **Phyllotaxis** | Golden spiral (sunflower, nautilus) | Inclusion vs Protection |
| **Mycelial** | Fungal network (the "Wood Wide Web") | Accountability vs Forgiveness |
| **Stigmergy** | Ant colony pheromone trails | Individual Agency vs Collective Integrity |
| **Physarum** | Slime mold path optimization | Fluidity vs Stability |
| **Coral** | Reef symbiosis | Autonomy vs Connection |

### What Each Would Explore

**Mycelial** -- Inspired by fungal networks that distribute nutrients underground. Trust flows like resources through hidden connections. When a connection dies, the network reroutes. When a member is ejected and returns, the mycelium finds new paths. The question: *Does trust resilience improve when the network optimizes for flow rather than structure?*

**Stigmergy** -- Inspired by ant colonies where no individual knows the plan, but collective intelligence emerges from pheromone trails. No algorithmic introductions at all. Members' vouch patterns leave traces that influence subsequent connections. The purest test of emergence. The question: *Does meaningful collective structure arise from purely individual actions?*

**Physarum** -- Inspired by the slime mold that famously solved the Tokyo railway problem. A trust network that continuously explores and prunes, finding the most efficient paths through constant reorganization. The topology is never "done." The question: *What happens when a community is always in motion?*

**Coral** -- Inspired by reef ecosystems where different organisms play different structural roles (coral polyps, cleaner fish, algae) in mutualistic symbiosis. Members naturally occupy different functional niches. Federation between groups is like reef systems connecting through ocean currents. The question: *Can trust networks develop functional specialization without hierarchy?*

Only phyllotaxis is specified today. The others are listed to show where the platform leads -- each one is a different hypothesis about how trust should organize, grounded in a pattern that already works in nature.

---

## The Hypothesis

The claim behind Trust Topology is both simple and testable:

> **Groups that grow in natural patterns will develop deeper, more resilient trust than groups engineered for optimal redundancy. Not because the math is better, but because the social experience of being in a naturally-growing organism is qualitatively different from being in an efficiently-engineered structure.**

Because each topology produces measurable health metrics (DVR, FDS, and future topology-specific scores), and because different groups can choose different topologies, Stroma becomes a **laboratory for collective intelligence** -- real data on questions about human social organization that have only ever been philosophical.

### What Success Looks Like

If this works, it means:

- **A third option** beyond hierarchy and flatness. Natural structure without bosses. Depth without rank.
- **Composable social DNA**. Small fragments of a fractal network carry the pattern with them. Five people from a phyllotaxis group can seed a new group that naturally reproduces the same trust properties.
- **Scale without degradation**. Corporations scale by adding management layers. Governments scale by adding bureaucracy. Both degrade the original culture. A fractal trust network scales by extending the spiral -- the pattern is scale-invariant, so the culture doesn't degrade as it grows.
- **A living protocol**. Not a fixed set of rules, but an evolving ecosystem of trust patterns that communities can choose, combine, and learn from.

---

## Relationship to Federation

Trust Topology sits *on top of* federation, not alongside it:

```
Layer 1: Single Group (Phases 0-3)
  One bot, one group, one contract. DVR admission and mesh health.

Layer 2: Federation (Phases 4-5)
  Multiple groups connected via shared members.
  Trust spans communities. Emergent discovery.

Layer 3: Trust Topology (Phase 6+)
  Within each group (federated or not), members can vote to activate
  topology experiments. The topology shapes HOW the group grows
  internally. Federation shapes how groups CONNECT externally.
```

A federated network of phyllotaxis groups would exhibit self-similarity at *two* scales: within each group (the spiral pattern) and across the federation (the same proportional relationships between groups). The same mathematical property that makes fractals beautiful in nature would make the federated network coherent as a whole.

---

## Security

Trust Topology does not weaken security in any way:

- **Admission** always goes through DVR-optimized vetting. Cross-cluster vouching remains mandatory. The topology contract has no say in who joins or leaves.
- **Ejection** always cascades from the primary contract. If your standing drops below zero or your vouches fall below the threshold, you're removed from everything.
- **Privacy** is maintained. Topology graphs use the same `MemberHash` identity masking as the primary contract. No cleartext Signal IDs, no message persistence.
- **Governance** follows the same proposal system. Activating a topology requires a group vote. The bot remains execute-only.

The topology experiments are *community formation* -- they shape the social experience of being in the group. The security layer is independent and inviolable.

---

## Summary

Trust Topology as a Platform is the vision for what Stroma becomes after federation: a system where communities don't just manage trust, they **choose the shape of trust itself**. Each shape is drawn from nature -- patterns that have evolved over billions of years to solve problems of growth, resilience, and coordination.

The protocol becomes a laboratory. The groups become experiments. And the results -- measurable, comparable, real -- tell us something new about how humans can organize trust at scale.

---

## Further Reading

- [How Stroma Works](HOW-IT-WORKS.md) -- The core trust protocol in everyday terms
- [Federation](FEDERATION.md) -- The north star: connecting groups through shared trust
- [Trust Model](TRUST-MODEL.md) -- Mathematical details of standing, vouching, and ejection
- [Algorithms](ALGORITHMS.md) -- DVR, cluster detection, and graph theory foundations

---

*Last Updated: 2026-02-14*
