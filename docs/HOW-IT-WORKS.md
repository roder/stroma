# How Stroma Works: A Human Guide

**Understanding the Trust Protocol in Everyday Terms**

---

## The Problem Stroma Solves

Imagine you're part of a community organizing group, a mutual aid network, or an activist collective. You need to coordinate with trusted people, but you face a fundamental dilemma:

**To verify someone is trustworthy, you typically need to know who they are. But revealing identities creates vulnerability.**

Traditional solutions create new problems:

- **Open invite links**: Anyone can join — no way to know if they're trustworthy or infiltrating
- **One person as gatekeeper**: Creates hierarchy and single points of failure ("what if the admin is compromised?")
- **Trusting strangers**: How do you know they won't leak sensitive information?
- **Large groups become cliques**: Newcomers end up isolated while established members cluster together

Stroma resolves this by distributing trust across relationships rather than concentrating it in gatekeepers or central authorities.

---

## The Core Principle

**You can only join if two different people who are already members personally vouch for you.**

Not one person. Two. From different parts of the network. And that trust must be maintained — it's not a one-time badge.

Think of it like this: one person might be fooled, but it's much harder to deceive two independent people who know you from different contexts.

---

## How It Would Work Without Technology

Imagine implementing this protocol using only Signal (the app you already use) and peer-to-peer coordination — no central coordinator, no designated "admin."

### Trust as Mutual Arising

In a non-hierarchical group, trust isn't something a gatekeeper grants you. It's a **"mutual arising"** — something the group builds together by weaving individual relationships into a resilient web.

The key insight: **you don't need a coordinator if everyone understands the protocol.** The rules are simple enough that any member can help the process happen.

### Understanding Your Position in the Network

Before we get to the joining process, it helps to understand where people stand — because **who vouches for you matters as much as getting vouched**:

**Invitees (Outside the Group)**
- Have **one vouch** from the person who invited them
- NOT in the Signal group yet — they're being vetted
- Need one more vouch from a different member to join
- Once they get 2 vouches, they're admitted automatically

**Bridges (Minimum Members)**
- Have **two vouches** — the minimum to be in the group
- Full members with all privileges (can invite, vouch, flag, vote)
- At risk: if either voucher leaves the group OR flags them, they need a replacement vouch immediately or they're removed
- The glue that connects different friend clusters together

**Validators (Well-Connected Members)**
- Have **three or more vouches**
- Same privileges as Bridges (no special powers)
- More resilient — if one voucher leaves or flags, they still have enough
- The group has strong confidence in them through multiple independent relationships

**Key distinction**: You're either IN the group (2+ vouches) or OUTSIDE (0-1 vouches). There's no "at risk inside member with 1 vouch" — if you drop below 2 vouches while inside, you're removed immediately.

**Why this matters for joining**: Your second vouch MUST come from a **different cluster** than your inviter — same-cluster vouches don't count. This is a hard requirement, not a preference. Any member (Bridge or Validator) from a different cluster CAN be your second voucher. Validators are often good choices because:
- They're well-connected across the network (multiple independent relationships)
- They often bridge different friend circles themselves
- Their vouch carries implicit confidence from the group

**Why cross-cluster is mandatory**: Without this requirement, coordinated bad actors could infiltrate by rubber-stamping each other. If Alice invites Bob and Carol (Alice's close friend in the same cluster) vouches for Bob, Bob gets in with two same-cluster vouches. Repeat this pattern and an infiltration cluster forms. Cross-cluster enforcement prevents this attack by requiring independent verification from different social contexts.

**Bootstrap exception**: For small groups (3-5 members) that are just getting started, there's often only one cluster — everyone knows each other from the same context. In this case, cross-cluster enforcement isn't possible yet, so any two members can vouch. Once the group grows and develops 2+ distinct clusters (typically around 6+ members), cross-cluster vouching becomes mandatory. The group naturally transitions from "everyone vouches for everyone" to "vouches must come from different social circles."

### The Joining Process (Peer-to-Peer)

**Step 1: Someone Invites You**

Alice, who is already in the group, wants you to join. She reaches out to you directly:

> "Hey Jordan, I'd like to invite you to join our network. I'll vouch for you — that's your first vouch. But you'll need a second vouch from someone else in the group who knows you from a different context than me."

Alice's invitation counts as your **first vouch**.

**Step 2: Cross-Cluster Vetting (The Key to Network Health)**

This is where Stroma differs from casual "friend of a friend" trust. The goal isn't just to get *any* second vouch — it's to get a vouch from a **different part of the network** than Alice.

**Why cross-cluster matters:**
- If Alice and Bob are from the same friend cluster, they might both be fooled by the same social dynamics
- Cross-cluster vouching creates **intersecting perspectives** — harder to deceive
- It builds bridges across the network, preventing isolated cliques
- A stronger mesh means individual members are more resilient

**The optimal path: Alice initiates the cross-cluster introduction**

Alice should proactively look for someone from a different part of the network — ideally a well-connected member (a "Validator") who runs in different circles than her:

> "Jordan, I want to introduce you to Bob. He's connected to the housing rights folks — different crowd than how I know you from the garden project. Let me set up a conversation."

Alice makes the introduction, then steps back. You and Bob have a brief conversation — maybe 15-20 minutes, or a small shared task like working on something together.

**What about if Jordan already knows someone?**

If Jordan already has an independent relationship with another member (Carol), that can work — *but only if Carol is from a different cluster than Alice*. Same-cluster vouches simply don't count toward admission.

If Carol and Alice are in the same cluster, Carol's vouch would be **rejected**. Alice must find someone from a different cluster for the second vouch. This isn't optional — the protocol enforces it because same-cluster vouching is an infiltration vector.

**Step 3: The Second Witness Handshake**

After your conversation, Bob decides whether to vouch for you. This isn't a job interview — it's establishing a second point of contact. Bob is essentially saying: "I've met Jordan, and I'm willing to put my reputation on the line for them."

If Bob vouches, you now have **two cross-cluster vouches** from members in different parts of the network.

**Step 4: You're In**

With two cross-cluster vouches confirmed, any member can add you to the Signal group. There's no special "admin" required — the rule is simply: two vouches from members in different clusters, and you're in.

### Why No Central Coordinator?

**The protocol IS the coordinator.** When everyone understands the rules:

- Anyone can invite (their invitation = first vouch)
- Anyone can introduce newcomers to potential second vouchers
- Anyone can vouch for someone they trust
- Anyone can add a fully-vouched person to the group
- Power is **fluid** — today's active connector might be busy next month, and someone else steps up

**What keeps it working:**
- The rules are simple and consistent (always need 2 vouches from different clusters)
- **Inviters take responsibility** for finding cross-cluster second vouchers (not leaving it to chance)
- Members naturally look out for the network's health by building bridges, not cliques
- The "cross-cluster" principle ensures vouches come from independent perspectives
- No one person can unilaterally let someone in

### The Collective Trust Map

Instead of one person tracking vouches, the group can maintain a **shared, anonymous audit**:

- Periodically, everyone answers: "Who in this group would I vouch for?"
- This reveals where gaps exist (who are the "solo links" at risk of disconnection?)
- Strategic introductions can be suggested based on the gaps
- No need to explain *why* you trust someone — the protocol just looks for the **existence** of the link

This distributed approach means:
- No "in-crowd" forms (cross-cluster links prevent cliques)
- Power shifts naturally (whoever is available helps the process)
- Privacy is maintained (you don't have to justify your relationships)

---

## Maintaining Trust (Not Just Earning It)

Here's where Stroma differs from traditional approaches: **trust isn't a permanent badge — it's a continuous balance.**

### Your Trust Standing

Think of it like a simple ledger:

```
Your Standing = (Effective Vouches) − (Regular Flags)
```

Where:
- **Effective Vouches** = people currently vouching for you (excluding anyone who later flagged you)
- **Regular Flags** = people flagging you who *never* vouched for you

The crucial rule: **the same person can't both vouch for you AND flag you** — that would be contradictory. If someone who vouched for you later flags you, their vouch is cancelled, and their flag is treated as just "taking back their trust" (not as an additional strike against you).

**Example:**
- Alice and Bob vouched for you (+2 vouches)
- Carol flags you (she never vouched, so -1 flag)
- **Your standing: 2 - 1 = +1** — you stay in

**Another example:**
- Alice and Bob vouched for you (+2 vouches)
- Alice flags you (she DID vouch, so her vouch is cancelled)
- **Your standing: 1 - 0 = +1** — BUT you now only have 1 effective vouch, which is below the minimum of 2

This second example results in removal — not because your "score" is negative, but because you no longer have enough people vouching for you.

### Why Voucher-Flagging Works This Way

Imagine Alice vouched for you, but then you did something that made her distrust you. It would be inconsistent for her to simultaneously say "I trust Jordan" (vouch) AND "I don't trust Jordan" (flag).

When Alice flags you, her previous vouch is cancelled — she's essentially saying "I take back my trust." Her flag doesn't count as extra punishment; it's just the removal of her endorsement.

**This prevents manipulation**: Nobody can vouch for you just to later weaponize that vouch by flagging. The worst someone can do with a single action is cancel their own vouch — not cause you additional harm.

### The Two Reasons You Might Be Removed

**Reason 1: Standing goes negative** (too many flags from non-vouchers)
- You have 3 vouches but 5 people flag you (none of whom vouched for you)
- Score: 3 - 5 = -2 (negative)
- Multiple independent people decided you're not trustworthy
- Note: Standing = 0 is still OK — you need to go *below* zero

**Reason 2: Effective vouches drop below 2** (vouchers left or withdrew trust)
- You had 2 vouchers, but one left the group → you now have 1 vouch
- OR: One of your vouchers flagged you → their vouch is invalidated, you now have 1 effective vouch
- Either way, you're below the minimum threshold of 2

Both result in the same outcome: immediate removal. There's no "warning period" or "probation." You can be removed for Reason 2 (not enough vouches) even if your Standing is still positive.

### Why Immediate Removal?

This might seem harsh, but it reflects a key principle: **trust is current, not historical**.

Your membership depends on your current trust relationships, not on the fact that you were once trusted. If those relationships change, your status changes with them.

But importantly: **there are no permanent bans**. You can re-join immediately if you secure two new vouches.

---

## Making Group Decisions

In Stroma, no single person controls group settings or policies. Everything is decided by group vote.

### The Proposal System (Without Tech)

Want to change how the group operates? Any member can propose it:

**Step 1: Make a Proposal**

You post to the group (or a designated proposals channel):
> "I propose we change the group name to 'Community Solidarity Network'"

**Step 2: The Group Votes**

Anyone can create an anonymous poll in Signal (Signal has built-in anonymous polls). The poll runs for a set time (maybe 48 hours). Anyone who sees the proposal can set up the vote — there's no designated "poll creator."

**Step 3: The Outcome**

If the proposal gets enough votes (say, 70% approval), it passes. Anyone with admin access can then make the change — but only because the group approved it.

**What Can Be Proposed:**
- Group name and description
- How many vouches are required to join (minimum is 2, but groups can require more)
- How much agreement is needed for decisions (60%? 70%? 80%?)
- Whether to connect with another Stroma group (federation)

### Why No One Has Special Power

The protocol itself is the authority. Any member can:
- Make proposals
- Create polls
- Count votes
- Execute approved changes

But no member can:
- Add someone without two vouches (the rule applies to everyone)
- Remove someone who maintains their trust standing (that would violate the protocol)
- Change settings without a group vote (changes require consensus)
- Override a group decision (the vote is the decision)

**Power is distributed by design.** If one person is busy or unavailable, anyone else can keep the process moving. The consistency comes from the protocol, not from a coordinator.

---

## Growing Beyond One Group: Federation

What happens when your group reaches its natural size limit (Signal groups cap at about 1,000 people, though most communities work best at 50-200)?

### The Problem

Separate groups can't benefit from each other's trust networks. Someone trusted in Group A has to start from scratch to join Group B.

### The Federation Solution

Two groups can choose to **federate** — creating a bridge where trust can flow between them.

**How This Would Work (Without Tech):**

**Step 1: Discovery**

Group A and Group B realize they share several members. Maybe 10 people are in both groups.

**Step 2: Evaluation (Each Group Decides for Themselves)**

Each group asks: "Do we trust the other group enough to connect?" — and each group applies their **own** threshold:

- Group A (20 members) with a 30% threshold: "10 shared members is 50% — exceeds our 30% threshold ✅"
- Group B (100 members) with a 10% threshold: "10 shared members is only 10% — exactly meets our threshold ✅"

**Critical principle**: Each group evaluates against their *own* threshold, not a shared one. This protects smaller groups from being absorbed — a small group can require a higher overlap percentage than a large group.

**Step 3: Independent Voting**

Both groups vote separately on whether to federate. Even if the thresholds are mathematically satisfied, members can still vote no. Both groups must approve — mutual consent is required.

**Step 4: What Federation Means**

After federation:
- Members of Group B can vouch for people joining Group A (and vice versa)
- Trust "travels" — if you're well-trusted in Group A, that counts for something in Group B
- But each group still makes their own decisions

### Why This Matters

Federation allows networks to grow without:
- Becoming unwieldy (each group stays human-scale)
- Creating central authorities (no "super-admin" controls multiple groups)
- Forcing everyone into one big room (different communities maintain their character)

Think of it like neighborhood associations forming a citywide coalition. Each neighborhood keeps its identity, but they can collaborate and members can move between them more easily.

---

## Privacy: What's Protected

### The "Trust Map" Problem

If someone could capture the complete list of who trusts whom, they'd have a valuable map of your community's relationships. This is dangerous because:

- It reveals who the key connectors are (targets for pressure)
- It shows the network structure (how to disrupt it)
- It exposes who knows whom (relationship intelligence)

### How Privacy is Protected (The Human Version)

In a non-technical implementation, privacy is protected by **distributing knowledge** — no single person holds the complete trust map:

**Principle 1: Knowledge is Distributed**

No one person maintains a master spreadsheet of "Alice vouched for Bob, Carol vouched for Dave." Each person only knows:
- Who they've vouched for
- Who's vouched for them
- Who they've introduced to whom

The complete picture doesn't exist in any one place.

**Principle 2: Vetting Happens Privately**

Vetting conversations happen in private 1-on-1 chats, not in the group. Nobody else sees who's being vetted or what's said. Alice introduces Jordan to Bob, but the rest of the group doesn't know the details of that conversation.

**Principle 3: Context Stays Between People**

Alice knows "I vouched for Jordan because we worked together on the garden project for two years." But she doesn't broadcast that to the group. The protocol only tracks the **existence** of the vouch, not the **reason** behind it.

**Principle 4: Nothing Persists Unnecessarily**

Once someone is admitted (or not), the vetting conversations can be deleted. There's no need to keep transcripts of who talked to whom about what.

### What This Means

Because no one person holds the complete trust map:

- **No single point of compromise**: There's no "facilitator" whose capture reveals everything
- **Each person knows only their part**: Alice knows her vouches; Bob knows his vouches; the overlap isn't assembled anywhere
- **Attackers would need to compromise many people**: And even then, they'd only get fragments

What an attacker could learn (worst case):
- Who is currently in the group (visible to all members anyway)
- Fragments of who vouched for whom (from individual people they compromise)
- Not: the complete social graph, relationship history, or conversation content from any single source

---

## The Philosophical Foundation

Stroma is built on several principles that might seem paradoxical but work together:

### Trust AND Anonymity

You might think trust requires knowing someone's identity, and anonymity requires hiding it. Stroma maintains both:
- Members know their vouchers
- But outsiders (including attackers) can't reconstruct the trust map

### Accountability AND Forgiveness

Actions have immediate consequences (removal when trust is lost). But there's always a path back (no permanent bans, no cooldown periods). You can rejoin immediately if you rebuild trust.

### Individual Agency AND Collective Decisions

Any member can vouch, invite, or flag. But outcomes emerge from multiple people's actions:
- One person can't admit someone (needs 2 vouches)
- One person can't easily eject someone (their single flag rarely drops someone below threshold)
- Settings require group consensus

### Fluid Membership AND Stable Networks

Your membership is continuously earned, not a permanent credential. But the network itself persists — it's bigger than any individual member.

---

## Key Differences from Traditional Groups

| Traditional Approach | Stroma Approach |
|---------------------|-----------------|
| Admin decides who joins | Two members from different clusters must vouch |
| Trust is granted once | Trust is continuously maintained |
| Removal requires admin action | Removal is automatic when trust drops |
| Banned means banned forever | Immediate path back if trust rebuilt |
| Settings controlled by admin | Settings changed by group vote |
| One person coordinates everything | Anyone can move the process forward |
| Complete trust map exists somewhere | Knowledge is distributed, no single point of capture |
| Groups are isolated islands | Groups can federate while keeping autonomy |

---

## Getting Started

### As a Potential Member

1. **Get introduced**: Know someone in the group? Ask them to invite you.
2. **Meet someone from a different cluster**: Your second vouch must come from a different part of the network than your inviter. Same-cluster vouches don't count.
3. **Join**: Once you have two cross-cluster vouches, you're in.
4. **Build connections**: Don't stop at two vouches — more connections make you more resilient.

### As an Existing Member

1. **Invite carefully**: Your invitation is your first vouch. Only invite people you genuinely trust.
2. **Initiate cross-cluster introductions**: When you invite someone, proactively connect them with a member from a *different* part of the network than you. Validators (3+ vouches) are ideal because they're well-connected, but any cross-cluster member works. Don't leave it to chance — this is what creates a resilient mesh.
3. **Vouch thoughtfully**: When asked to be someone's second vouch, take it seriously. You're adding your perspective to someone else's invitation.
4. **Flag when necessary**: If someone violates trust, flag them. It's how the system stays healthy.
5. **Participate in decisions**: Vote on proposals. The group's character is shaped by who participates.

### As a Community Starting a New Group

1. **Seed with trust**: Start with 3 people who deeply trust each other (all vouch for all). This is the "bootstrap" phase — everyone is in one cluster, so cross-cluster isn't enforced yet.
2. **Grow carefully**: Each new member needs two existing members to vouch. As you grow past 5-6 members and develop distinct friend circles, cross-cluster vouching becomes mandatory — new members must be vouched by people from different parts of your network.
3. **Set your thresholds**: Decide how much consensus you want for decisions.
4. **Consider federation**: As you grow, you might connect with aligned communities.

---

## Summary

Stroma is a trust protocol that:

- **Distributes gatekeeping** across multiple people rather than concentrating it in one admin
- **Continuously maintains trust** rather than granting it once and forgetting
- **Enables immediate consequences** without permanent punishment
- **Keeps decisions with the group** rather than with a leader
- **Protects privacy** by minimizing what needs to be recorded
- **Allows networks to grow** through federation while preserving community autonomy

The technology (bots, cryptography, decentralized networks) exists to make this process faster, more reliable, and more private. But the underlying methodology is human: trust is earned through relationships, maintained through accountability, and distributed across the community rather than concentrated in authorities.

---

**Questions?** Ask an existing member, or check the [User Guide](USER-GUIDE.md) for detailed bot commands once you're in a group.
