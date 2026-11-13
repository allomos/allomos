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

The positioning was worked out with the approver in the conversation, point by point. What the approver said at each step, in their own words:

| When | The approver said | What it settled |
|---|---|---|
| 2026-11-12 | 「这个要优先考虑。因为它也涉及到审批：目前allomos里面记录的东西，我看起来很累，不适合给人看。」 ("This must come first. It is about approving too: what Allomos records today is tiring for me to read, not fit for people.") | Records that people, newcomers included, can read are part of layer 2 and come first (task `A01-cup419`, Review desk first in the plan). |
| 2026-11-12 | 「我觉得这是最容易被人抄袭和替代的功能。」 ("I think this is the feature most easily copied and replaced.") — about approvals and readable records | Approvals are **not** the moat. The moat is layer 1; approvals are the value people see, built on it. |
| 2026-11-12 | 「中立」可以换一个词吗？ ("Can 'neutral' be another word?") | 中立 is not used (see Wording). |
| 2026-11-12 | 「调度，容易让人想到像magpie这样的ai切换工具。有别的词吗？」 ("'Scheduling' makes people think of AI-switching tools like Magpie.") | 调度 is not used (see Wording). |
| 2026-11-12 | 「这两个动词看起来可以」 ("these two verbs look fine") — 记录 (keep a record) and 接力 (hand on) | The two verbs stay. |
| 2026-11-12 | The approver wrote three candidate taglines and chose 「AI 时代的开发，换谁来做都接得住。」 (「好」); approved the English *hand it to anyone. Nothing drops.* (「可以」) and asked to add *Development in the AI era* (「我想加上」); asked the Japanese to be settled at once (「现在就定」), and it was settled as shown above. | The tagline. |
| 2026-11-12 | 「要」 ("yes") — to writing a new decision record that replaces 0014 | This record. |
| 2026-11-12 | 「不必了」 ("not needed") — to researching whether switching accounts fits each vendor's terms | No separate research (see Not decided here). |
| 2026-11-13 | Confirmed 不绑定 (to users) and 独立 (for who we are); confirmed 随身审批; chose that AI skills is **kept as in 0014** | Wording; AI skills (see Relation to earlier decisions). |

These approvals cover writing this record and changing the documents; they do **not** approve any particular implementation: each piece is its own task, planned and approved as usual.

## Three layers

| Layer | What it is | Why |
|---|---|---|
| **1. Foundation (the moat)** | **Not tied** to any one agent, and local first: switching across vendors and accounts; one record of the work across vendors, kept as open, readable files in your own git (the taskbag); assigning quota and accounts across vendors. | The large vendors do not want to help users move to a rival, see only their own agent's work, and make their money in the cloud. Allomos is **independent** of every vendor, so it can do what they will not. This holds the large vendors off, not other small companies: against those, what counts is focus and speed. |
| **2. The value people see** | **Approvals people can read and give on the go**: handoffs, evidence and AI decisions that people (newcomers and approvers) can read without effort; a review workbench; **随身审批**, approving on a phone or the web, away from the computer. | The first bottleneck is now "too much to review, hard to trust it". On its own this layer is easy to copy (the approver's point); Allomos's version sees the work of **every** agent and account, because of layer 1: the difference is in the data, not the screen. |
| **3. The engine** | **Fully automatic assigning and handing on** ([0023](./0023-full-automatic-agent-orchestration.md)). | It makes the output grow, and the more output there is, the more layers 1 and 2 are worth. Assigning **across vendors** is what a single vendor's orchestration cannot do. It is not a selling point: we do not compete with the large vendors on orchestration. |

## Who it is for

Small teams of 2 to 5 people and solo developers, each running several AI agents and accounts at once. The "anyone" in "hand it to anyone" (「换谁来做」) means both AI agents and teammates.

## Kept, but behind

- **Progress management** (boards, burndown, forecast) is still built, as views, not as the lead feature: this avoids competing head-on with Linear and similar trackers.
- **Open formats and standards**: the taskbag stays open and readable now (layer 1); promoting it as a standard for other tools is a long-term strategy, once there are many users.
- **AI skills** (0014's set 2) is kept as it is in 0014 (approver, 2026-11-13).
- **Letting people who do not write code build software** waits until "people can read it" (layer 2) is done well.

## Wording

| Say | Do not say | Why, and who decided |
|---|---|---|
| **不绑定** / *not tied to any one agent* (to users); **独立** / *independent* (for who we are) | 中立 / *neutral* | The approver asked for another word than 中立 (2026-11-12); the agent proposed these two and the approver confirmed them (2026-11-13). |
| **记录** and **接力** / *keep a record*, *hand on*; for the step of giving tasks out, **分派和接力** / *assign and hand on* | 调度 / *scheduling* | The approver said 调度 makes people think of AI-switching tools like Magpie (2026-11-12) and accepted the verbs 记录 and 接力 (2026-11-12). 分派和接力 is the agent's proposal, recorded for review. |
| **随身审批** / *approve on the go* | 远程审批 / *remote approval* | 远程审批 read as one person approving another person's work; it means approving the agents' (and later teammates') work while away from the computer. The agent proposed 随身审批; the approver confirmed it (2026-11-13). |

The English and Japanese wording follows the same rules; `A01-aunt631` holds the product copy.

## Relation to earlier decisions

| Earlier | Relation |
|---|---|
| [0014](./0014-main-purpose-and-three-sets.md) — main purpose and three sets | **Replaced.** Its main set (Switch) becomes layer 1 and is widened to "not tied to any one agent". Approvals people can read, which 0014 had no place for, become layer 2. Its set 3 (progress management) is kept behind, as views. Its set 2 (**AI skills**) is **kept as it is in 0014**, unchanged (approver, 2026-11-13): it was not part of this discussion. The questions 0014 left open (task `A00-18`, `A03-02`) stay where they are. |
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
- New feature cards name which layer they serve, or AI skills (instead of which of 0014's sets).
- No change to the taskbag's format or to `alm`'s commands.
- The pitch decks ([docs/pitch/](../pitch/README.md)) are still organized by 0014's three sets; reorganizing them is a task of its own.

## Decided

2026-11-12, will.w.hui: **Accepted** in the conversation ("要"); corrected 2026-11-13 to match the discussion exactly (AI skills kept as in 0014; who decided each word; the open taskbag in layer 1; why the engine matters; the approver's words). The positioning, the tagline and the wording are the approver's, as the table under **The approver's words** shows; the record's English phrasing, the number 0024 and the term 分派和接力 were written by the agent and are recorded for review.
