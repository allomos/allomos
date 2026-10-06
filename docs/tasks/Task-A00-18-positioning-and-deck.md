# Task-A00-18-positioning-and-deck

*Card type: decision + docs. Area A00 (Planning). Requested by the owner, 2026-10-06, after the rename ([`Task-A00-17`](./Task-A00-17-rename-to-crossjump.md)).*

## Why

The owner wants FlipHopper's **main purpose** stated differently: **it helps people keep a project's tasks going across AI agents and agent accounts.** When one agent or account runs out (quota, a model change, a provider change) or the work passes to another person, the project's tasks, progress and handoff notes move with it, so the next agent picks up where the last one stopped. Everything else is an additional feature.

The owner's first grouping of the other features (to be organized more carefully):
- **AI skills** — help people learn to use AI agents, switch between them, run the vibe-coding process well, and get better results.
- **Progress management** — show progress, the burndown chart, the forecast finish date, and so on.

Today the deck leads with "Land every vibe-coded project" (tracking first), and switching agents is one bullet among many.

## Structure: three sets (owner, 2026-10-06)

The owner chose **three sets: the main set, AI skills, and progress management**. The tables below are the agent's proposal for what goes in each, for the owner's review.

**Set 1 (main) — carry the project between agents.** "Switch agents, keep the project."

| What it does | Exists or planned in |
|---|---|
| One **seabag** per project, in the repository: tasks, states, labels, who holds what, handoff notes. Any agent reads and writes it, so nothing lives inside one agent's memory or one account. | A01-01 (done), A01-03 (`fh`) |
| **The `fh` command / MCP** — the same small set of actions for Claude Code, Codex and others: next, claim, label, wait, done, release. | A01-03, Iteration 3 |
| **Handoff and resume** — each session keeps a short handoff; a new session (other agent, other account, other person) gets the exact resume prompt. | today as Markdown practice ([AI-WORKFLOW](../workflow/AI-WORKFLOW.md)); to be built into the app |
| **Start or reopen a session** in a chosen agent from a task. | [0005](../decisions/0005-start-and-open-agent-sessions.md), Iteration 8 |
| **Hand to a person**: teammates, roles and invitations, cloud projects (paid). Working together is part of the main set, not a set of its own. | [0004](../decisions/0004-team-accounts-and-roles.md), [0006](../decisions/0006-local-and-cloud-projects.md), Iterations 13–15 |
| *To decide:* show each agent account's remaining quota and suggest when to switch. | new idea; needs research (what each provider exposes) |

**Set 2 — AI skills.** Help people get good at working with agents.

| What it does | Exists or planned in |
|---|---|
| Learn: short guides on each agent's strengths, and setting one up for a project (`AGENTS.md` / `CLAUDE.md`). | new |
| Switch: a checklist for moving a task from one agent to another (what to hand over). | new; builds on handoff |
| Run the process: one card per outcome, honest labels, worktrees per session, owner approvals. | today's practice ([CARDS](../workflow/CARDS.md)); AI-generated project setup ([AI-GENERATED-PROJECTS](../product/AI-GENERATED-PROJECTS.md), Iteration 11) |
| Improve results: choose model and effort per task; review risky changes. | [0005](../decisions/0005-start-and-open-agent-sessions.md) open question 4 |

**Set 3 — Progress management.** Show where the project stands.

| What it does | Exists or planned in |
|---|---|
| Progress by area, states and labels; what waits on you. | A01-01 (done), Iteration 1–2 |
| Burndown chart and forecast finish date. | [0002](../decisions/0002-burndown-counts-plus-sizes.md), Iteration 5 |
| On the phone and in the browser (companions). | [0009](../decisions/0009-desktop-hub-with-web-and-mobile-companions.md) |

## Questions for the owner

1. ~~Main purpose~~ Answered 2026-10-06: switching between AI agents and teammates, as discussed for the name.
2. ~~Groups~~ Answered 2026-10-06: three sets — main, AI skills, progress management.
3. ~~AI-skills content~~ Answered 2026-10-06: keep "being designed" for now; decide later.
4. ~~Quota idea~~ Answered 2026-10-06: not sure yet; recorded as the parked card [`Task-A03-02`](./Task-A03-02-account-quota-idea.md).
5. ~~Plan order~~ Answered 2026-10-06: the order of releases stays; a handoff note (`fh release --note`) and `fh next` are added to [`Task-A01-03`](./Task-A01-03-gb-command.md), so switching agents works from the first release.

## What gets built (after the owner answers)

| Piece | Where |
|---|---|
| Decision record: FlipHopper's main purpose and feature groups | `docs/decisions/0014-…` (new) |
| Product documents lead with the main purpose | `README.md`, `AGENTS.md` ("What FlipHopper is"), [PLAN.md](../product/PLAN.md) § 1, [REQUIREMENTS.md](../product/REQUIREMENTS.md) grouping if useful |
| Both decks rewritten around the main purpose and the groups, republished to the **same addresses** | `docs/pitch/roadshow-en.html`, `roadshow-zh.html`, [pitch/README.md](../pitch/README.md) |

## Details for the worker

- **Outcome.** A reader of the deck or the README understands in one sentence that FlipHopper keeps a project going across AI agents and accounts, and sees the other features in a few clear groups.
- **Files owned.** The files in the table above, and this card.
- **External operations.** Republishing the two decks to their current addresses (owner asked for the change, 2026-10-06).
- **Done means mechanically.** The decision record exists; README, AGENTS.md and PLAN § 1 state the main purpose; both decks are republished and their content rules in [pitch/README.md](../pitch/README.md#rules-for-the-content) still hold (claim only what is decided); links check clean; the owner accepts.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-06 | Main set serves switching between AI agents and teammates; the deck's content is organized by the three sets | owner (chat) |
| 2026-10-06 | Change the deck's main topic to keeping project tasks between AI agent accounts, with other features grouped (first ideas: AI skills, progress management), organized more carefully | owner (chat) |

## Notes

- 2026-10-06 — Card written by the agent with a proposed structure. The owner answered question 2 (three sets); working together placed in the main set by the agent. Waiting on answers 1, 3, 4, 5 and review of the sets' contents.
- 2026-10-06 — Claimed. The owner answered question 1 (switching between AI agents and teammates) and asked for the deck to follow the three sets. Done: [0014](../decisions/0014-main-purpose-and-three-sets.md); README, `AGENTS.md`, PLAN § 1 and [pitch/README.md](../pitch/README.md) lead with the main purpose; both decks rewritten as ten slides (hero and hop diagram, problem, the three sets, switch agents, switch teammates, AI skills, progress management, pricing, roadmap tagged by set, close) and republished to the same addresses (version 2). Questions 3–5 are still open; until answered, the deck marks set 2 as being designed, the quota idea is left out, and the plan's order is unchanged. Waiting on the owner's review of the decks.
- 2026-10-06 — Owner answers: keep "being designed" for AI skills; quota idea parked as [`Task-A03-02`](./Task-A03-02-account-quota-idea.md); plan order kept, handoff note and `fh next` added to A01-03. Owner asked for a seabag that looks like a seabag on the first slide and a mark of a person hopping with it: both drawn ([hopper-mark.svg](../pitch/hopper-mark.svg)), decks republished (version 3). Putting the mark on the app itself is [`Task-A07-01`](./Task-A07-01-app-icon-hopper.md).
- 2026-10-06 — Owner accepted the decks (version 3). Card done.
