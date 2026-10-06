# 0014 — FlipHopper's main purpose, and its three sets of features

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Accepted** (purpose and sets); contents of sets 2 and 3 proposed | 2026-10-06 | 2026-10-06 | will.w.hui |

## In plain words

FlipHopper's main job is **switching**: a project hops between **AI agents** (Claude Code, Codex, several accounts) and between **teammates**, and nothing is lost on the way. When one agent or account runs out, another picks up where it stopped; when the work passes to a person, the tasks, their proven steps and the handoff notes go with it. This is the idea behind the name ([0013](./0013-product-name-fliphopper.md)).

Everything else is grouped into two more sets. All three work on the same seabag.

| Set | What it is for | Features (existing or planned) |
|---|---|---|
| **Main set — Switch** | Hop between AI agents and teammates without losing the project | the seabag in the repository (R87); the same `fh` command and MCP for every agent (R88); handoff and resume; starting or reopening a session in a chosen agent ([0005](./0005-start-and-open-agent-sessions.md)); handing work to a teammate, with roles, invitations and cloud projects ([0004](./0004-team-accounts-and-roles.md), [0006](./0006-local-and-cloud-projects.md)) |
| **Set 2 — AI skills** | Help people get better at working with AI agents | learning each agent and setting it up for a project; switching cleanly; running the vibe-coding process (one card per outcome, a worktree per session, proven steps, owner approval); improving results (model and effort per task, review of risky changes); AI-generated project setup ([AI-GENERATED-PROJECTS](../product/AI-GENERATED-PROJECTS.md)) |
| **Set 3 — Progress management** | Show where the project stands and when it will land | progress by area and label; what waits on the owner; burndown and forecast ([0002](./0002-burndown-counts-plus-sizes.md)); desktop, browser and phone ([0009](./0009-desktop-hub-with-web-and-mobile-companions.md)) |

## Decided by the owner (2026-10-06)

- The main purpose: switching between AI agents and teammates, as discussed for the name.
- Three sets: the main set, AI skills, progress management. Working together with teammates belongs to the main set, not to a set of its own.
- The road show deck is organized by these three sets.

## Not decided yet

Recorded on [`Task-A00-18`](../tasks/Task-A00-18-positioning-and-deck.md) until the owner answers:
- whether AI-skills content is written by us inside the app, or links to outside material;
- whether showing each agent account's remaining quota (and suggesting when to switch) is researched now or later;
- whether the plan's order changes (for example handoff and resume earlier). **Until then the order in [PLAN § 9](../product/PLAN.md#9-roadmap--releases) stands**; only the message changes.

## Consequences

- README, `AGENTS.md`, [PLAN.md](../product/PLAN.md) and the [road show decks](../pitch/README.md) lead with the main purpose.
- Every new feature card names which set it serves.
