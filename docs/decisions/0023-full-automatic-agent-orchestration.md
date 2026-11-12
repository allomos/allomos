# 0023 — Allomos may orchestrate agents fully automatically

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Accepted** (direction only; how it is done is left to later tasks) | 2026-11-12 | 2026-11-12 | will.w.hui |

Card: task `A00-mohair597`. Written from the approver's discussion of 2026-11-12 about where AI coding is heading.

## In plain words

Until now the plan said Allomos would **not** run agents on its own: it would track the work, and start an agent only when a person asked ([PLAN § 3](../product/PLAN.md#3-non-goals-for-now)). That is no longer the plan.

**Decided:** Allomos may run the agents **fully automatically**. It may decide by itself which ready task goes next, give it to an agent and an account, pass the work from one agent to the next, and switch to another agent or account when one runs out of quota. People mainly **review the results**, instead of starting every session by hand.

What does **not** change: anything with an effect outside Allomos and the project's own working copies still needs the approver's exact approval, item by item, and so does every release. Full automation only hands out work and passes it along inside Allomos.

## The approver's words

On 2026-11-12, in the conversation, the approver said **"可以改"** ("it may be changed") about the non-goal "Orchestrating agents automatically (scheduling, auto-assigning, chaining agents)", and from three options chose **「全自动」 (fully automatic)**. The two options not chosen were:

- *only suggest*: Allomos proposes the next task and agent, a person clicks Start;
- *start automatically within limits*: Allomos starts sessions on its own, but only inside limits a person sets beforehand.

This approval covers changing the plan and writing this record. It does **not** approve any particular implementation: each piece is its own task, planned and approved as usual.

## Context

The discussion of 2026-11-12 looked at the trend:

- running several agents at once, coordinated by a tool, is becoming the normal way to work;
- the bottleneck moves from writing code to a **person reviewing** what the agents did;
- agents increasingly run in the cloud or in the background, without a person watching;
- quota and cost can run away when many agents run unattended.

Allomos already went part of the way. **Watch ready tasks** (自动监测就绪任务) starts an agent session for each ready task nobody holds, once a person has started the watch ([A03-moth285](#relation-to-earlier-decisions-and-work)), and can share the ready tasks among several agents ([A03-fruit467](#relation-to-earlier-decisions-and-work)). The non-goal was therefore already partly out of date.

## What "fully automatic" includes

| Part | Meaning |
|---|---|
| **Scheduling** | Allomos chooses which ready task runs next and when, following its dependencies (blocked tasks, iterations, the order `alm next` gives). |
| **Assigning** | Allomos gives a task to an agent (Claude Code, Codex, …) and to one of its accounts, and starts the session itself, on this computer or in a cloud session. |
| **Chaining** | When a session finishes, releases a task or reaches a handoff gate, Allomos starts the next session: on the next task, or on the same task from its handoff note and working branch. |
| **Switching on quota** | When an agent or account is out of quota (the figures reported with `alm quota`), Allomos goes on with another agent or account, which picks up from the handoff note. This is the main purpose of [0014](./0014-main-purpose-and-three-sets.md) done without a person's hands. |
| **People review** | A person's main work becomes reviewing: the human checks, the AI decisions to review, the parked questions and the approvals (`alm human`). |

## What does not change (the boundaries)

1. **Outside effects still need the approver's exact approval**, item by item, recorded in the task's description: hosted resources (Supabase, databases, hosting, DNS), applying SQL to a real target, deployments, secrets, sending email, creating repositories, adding or changing a remote, and anything touching other people's data ([AGENTS.md § Always-on boundaries](../../AGENTS.md#always-on-boundaries)). An automatically started session follows these rules exactly like one a person started.
2. **Releases still need the approver's approval.**
3. **Decisions stay the approver's.** Real questions (data formats, removing things, permissions and security, cost, outside effects, changes to the plan or another task) are still parked for the approver with `alm wait`; routine choices are still the agent's, recorded with `alm ai-decided`.
4. **Making a task ready stays a person's choice.** Full automation schedules **ready** tasks; it does not make backlog tasks ready on its own. The one existing exception stays as it is: in a goal run the goal approves the plan in advance (`alm guide goal`).
5. **The run mode is still chosen by a person.** Before an automatic run, a person chooses quiet or normal mode (`alm guide run-modes`), as today.
6. **"`alm` tells; the agent checks"** still holds. Orchestration lives in the Allomos app (the hub), which starts sessions; it is not a new check inside `alm`. Any check `alm` would make on its own still needs the approver's approval first.
7. **Takeover stays as in [0022](./0022-task-heartbeat-and-takeover.md).** Allomos may give a **lost** task to a new session (that is an ordinary takeover); it does not take a task from a holder that is still working (`--take`) unless a person asks.
8. **Approving a card still authorizes only its local work.** Starting it automatically adds no approval.

## Relation to earlier decisions and work

| Earlier | Relation |
|---|---|
| [0005](./0005-start-and-open-agent-sessions.md) — start and open agent sessions | 0005 starts a session when a person clicks Start; this record lets Allomos also start one on its own. R30 and R31 stay; 0005's runner design (still proposed) is one way to start sessions automatically. R33 (only permitted members start agents, every start recorded) applies to automatic starts as well: the person who turns automation on is the one starting them. |
| [0014](./0014-main-purpose-and-three-sets.md) — main purpose and three sets | Full automation belongs to the **main set (Switch)**: it is switching between agents and accounts without a person in the loop. The parked question of showing each account's quota and suggesting when to switch (task `A03-02`) becomes part of this direction. |
| [0016](./0016-waiting-and-blocked-states.md) — waiting and blocked | A blocked task already goes on by itself when its last blocker finishes; chaining builds on that. A waiting task waits on a person and is never started automatically until it is answered. |
| [0022](./0022-task-heartbeat-and-takeover.md) — heartbeat and takeover | The heartbeat, lost tasks and takeovers are the safety net that makes unattended chaining safe: a session that vanishes loses its task after 30 minutes, and another session goes on from what was pushed. |
| `A03-moth285` — Watch ready tasks (one button, terminal Claude Code / Codex) | The **first step** of this direction: Allomos starts sessions for ready tasks unattended, after a person starts the watch. |
| `A03-fruit467` — Watch ready tasks with several agents | The **second step**: the ready tasks are shared among several agents. What is still missing for "fully automatic": choosing accounts, switching on quota, and the limits below. |

## Follow-up (not decided here)

How it is done is left to later tasks, each planned and approved on its own:

- a **budget or quota cap**: how much an unattended run may spend, and what happens when it is reached;
- **how many sessions run at once**, per agent and per account;
- **retrying after a failure**: how often, and when a task is given to another agent instead;
- **when to stop and call a person**: which situations end a run instead of parking a question and going on;
- **switching on quota**: reading each account's figures and choosing the next agent or account (with `A03-02`);
- **cloud and background sessions**: starting agents that do not run on this computer.

## Consequences

- [PLAN.md](../product/PLAN.md): § 3 no longer lists automatic orchestration as a non-goal; § 2 gains it as a goal; § 4 gains a row for this record.
- [REQUIREMENTS.md](../product/REQUIREMENTS.md): new requirement R96 (Allomos may orchestrate agents fully automatically, inside the boundaries above).
- [0005](./0005-start-and-open-agent-sessions.md) gains a note pointing here.
- No change to the taskbag's format, to `alm`'s commands or to AGENTS.md: those come with the follow-up tasks, if they need any.

## Decided

2026-11-12, will.w.hui: **Accepted** in the conversation ("可以改", option 「全自动」). The record's wording, the boundaries listed as points 4–8 (drawn from rules that already hold) and the number 0023 were written by the agent and are recorded for review.
