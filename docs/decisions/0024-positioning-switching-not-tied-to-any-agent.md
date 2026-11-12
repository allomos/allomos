# 0024 — What Allomos is: switching not tied to any one agent, plus approvals people can read and give on the go

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Accepted** (positioning); replaces [0014](./0014-main-purpose-and-three-sets.md) | 2026-11-12 | 2026-11-12 | will.w.hui |

Card: task `A00-attic003`. Written from the approver's discussion of 2026-11-12 about where AI coding is heading.

## In plain words

[0014](./0014-main-purpose-and-three-sets.md) said Allomos's main job is **switching**: a project hops between AI agents, their accounts and teammates, and nothing is lost. That stays, but it is no longer the whole story.

**Decided:** Allomos is **switching that is not tied to any one agent, plus approvals people can read and give wherever they are**. Fully automatic assigning and handing on of work ([0023](./0023-full-automatic-agent-orchestration.md)) is the **engine** underneath, not the thing we sell.

**In one sentence:** whichever agent or account you use, the tasks, progress and handoffs are kept in your own git; when one stops, another carries on and nothing is lost; you can always understand what happened, and approve it with a tap.

## Tagline (final)

| Language | Tagline |
|---|---|
| Chinese | 「换肩」 AI 时代的开发，换谁来做都接得住。 |
| English | Allomos — Development in the AI era: hand it to anyone. Nothing drops. |
| Japanese | 「肩代」 AI 時代の開発、誰が引き継いでも取りこぼさない。 |

Putting it into the app, website, guide and READMEs is task `A01-aunt631` (already merged).

## The approver's words

On 2026-11-12, in the conversation, the approver agreed to write a new decision record replacing 0014 (answer: **"要"**, "yes, do it"). The positioning below, the tagline and the wording rules were settled point by point in the same discussion. This approval covers writing this record and changing the documents; it does **not** approve any particular implementation: each piece is its own task, planned and approved as usual.

## Three layers

| Layer | What it is | Why |
|---|---|---|
| **1. Foundation (the moat)** | **Not tied** to any one agent, and local first: switching across vendors, one record of the work across vendors, sharing quota and accounts across vendors. | The large vendors do not want to help users move to a rival, see only their own agent's work, and make their money in the cloud. Allomos is **independent** of every vendor. |
| **2. The value people see** | **Approvals people can read and give on the go**: readable handoffs, evidence and AI decisions; a review workbench; approving away from the computer. | The first bottleneck is now "too much to review, hard to trust it". On its own this layer is easy to copy; Allomos's version sees the work of **every** agent, because of layer 1. |
| **3. The engine** | **Fully automatic assigning and handing on** ([0023](./0023-full-automatic-agent-orchestration.md)). | Needed to make layers 1 and 2 real, but not a selling point: we do not compete with the large vendors on orchestration. |

## Who it is for

Small teams of 2 to 5 people and solo developers, each running several AI agents and accounts at once. The "anyone" in "hand it to anyone" (「换谁来做」) means both AI agents and teammates.

## Kept, but behind

- **Progress management** (boards, burndown, forecast) is still built, as views, not as the lead feature: this avoids competing head-on with Linear and similar trackers.
- **Open formats and standards** are a long-term strategy.
- **Letting people who do not write code build software** waits until "people can read it" (layer 2) is done well.

## Wording (decided by the approver)

| Say | Do not say | Why |
|---|---|---|
| **不绑定** / *not tied to any one agent* (to users); **独立** / *independent* (for who we are) | 中立 / *neutral* | The approver's choice. |
| **接力**, **分派和接力** / *hand on*, *assign and hand on* | 调度 / *scheduling* (as a headline) | "Scheduling" is easily confused with model-switching tools. |
| **随身审批** / *approve on the go* | 远程审批 / *remote approval* | The approver's choice. |

The English and Japanese wording follows the same rules; `A01-aunt631` holds the product copy.

## Relation to earlier decisions

| Earlier | Relation |
|---|---|
| [0014](./0014-main-purpose-and-three-sets.md) — main purpose and three sets | **Replaced.** Its main set (Switch) becomes layer 1 and is widened to "not tied to any one agent". Approvals people can read, which 0014 had no place for, become layer 2. Its set 3 (progress management) is kept behind, as views. Its set 2 (AI skills) is not a layer of its own: what remains of it (setting agents up, the working process, Build a new app…) serves layers 1 and 2; the questions 0014 left open (task `A00-18`, `A03-02`) stay where they are. |
| [0023](./0023-full-automatic-agent-orchestration.md) — fully automatic orchestration | Becomes **layer 3, the engine**. Its direction and boundaries are unchanged; only its place in the story changes: it is not presented as a selling point. |
| [0012](./0012-free-local-app-paid-cloud.md) — free local app, paid cloud | Fits layer 1: local first, and free on your own computer. Approving on the go away from home may need the cloud; which of it is paid stays open, as 0012 says. |
| [0006](./0006-local-and-cloud-projects.md) — local and cloud projects | Unchanged. "Kept in your own git" describes local projects; cloud projects add teammates and approving from anywhere. |
| [0009](./0009-desktop-hub-with-web-and-mobile-companions.md) — desktop hub with companions | The web and phone companions are where "approve on the go" happens. |

## Not decided here

- Which approvals can be given from the phone, and what a person sees before approving (each its own task).
- Whether switching accounts within one vendor fits each vendor's terms: the approver decided on 2026-11-12 that it needs no separate research.

## Consequences

- [0014](./0014-main-purpose-and-three-sets.md) is marked **Superseded** and points here; the [decision index](./README.md) and [PLAN § 4](../product/PLAN.md#4-decisions) are updated.
- [PLAN § 1–2](../product/PLAN.md#1-problem), [REQUIREMENTS.md](../product/REQUIREMENTS.md), [AGENTS.md](../../AGENTS.md) ("What Allomos is") and [docs/README.md](../README.md) describe Allomos this way.
- New feature cards name which layer they serve (instead of which of 0014's sets).
- No change to the taskbag's format or to `alm`'s commands.
- The pitch decks ([docs/pitch/](../pitch/README.md)) are still organized by 0014's three sets; reorganizing them is a task of its own.

## Decided

2026-11-12, will.w.hui: **Accepted** in the conversation ("要"). The positioning, tagline and wording rules are the approver's; the record's English phrasing, the number 0024, and how 0014's set 2 maps onto the layers were written by the agent and are recorded for review.
