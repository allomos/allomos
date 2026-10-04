# Task board — glidebox

> **Last updated:** 2026-10-04

The index of all work and the **only home of status**. Card detail lives in [docs/tasks/](./tasks/); rules for cards, labels and claiming are in [workflow/CARDS.md](./workflow/CARDS.md). Restart mechanics (worktrees, servers, pitfalls) are in [AGENT_HANDOFF.md](./AGENT_HANDOFF.md). When another document disagrees with this board, the board is right — correct the other document in place.

**To resume:** read Now, Sessions and Waiting on the owner, then open your card.

## Now

**Nothing is claimed; ten cards are open; no application code exists yet.** Glidebox is in planning; the first iteration, **1 — v0.1 macOS local tracker**, is next ([PLAN § 9](./product/PLAN.md#9-roadmap--releases)).

**Where the product stands.**
- The owner has decided to **build glidebox ourselves** rather than start from an open-source tracker ([`Task-W00-07`](./tasks/Task-W00-07-open-source-base-decision.md)).
- The product shape: a **desktop app for macOS and Windows as the hub**, built with **Tauri 2** and sharing one interface code base with the browser and phone ([`W00-12`](./tasks/Task-W00-12-desktop-platforms-and-shared-code.md)), with web and mobile companions that connect to it to view progress and status ([0009](./decisions/0009-desktop-hub-with-web-and-mobile-companions.md)). Each project's data lives in a **folder the user chooses**, standalone or inside an existing project folder ([`W00-11`](./tasks/Task-W00-11-project-data-folder.md)), as **readable text files tracked by Git** ([`W00-06`](./tasks/Task-W00-06-git-sync-proposal.md)).
- Requirements for local and cloud projects, AI-generated project setup and interfaces are recorded ([`W00-04`](./tasks/Task-W00-04-local-and-cloud-requirements.md), [`W00-08`](./tasks/Task-W00-08-ai-generated-project-setup.md), [`W00-09`](./tasks/Task-W00-09-interface-requirements.md)).
- The 2026-10-04 review has fixed the documents ([`Task-W00-10`](./tasks/Task-W00-10-docs-consistency-review.md)).

**What is next.**
- The owner reviews the documents and answers the questions below.
- The research card [`Task-W10-01`](./tasks/Task-W10-01-remote-access-research.md) looks for a free or very cheap way for companions to reach the desktop app from outside, and a free email service for the pairing code.
- The first code card, [`Task-W04-01`](./tasks/Task-W04-01-walking-skeleton.md), is the Iteration 1 **walking skeleton**: the macOS desktop app opens a task box and shows its tasks. It can start once the owner reviews it.
- The research card [`Task-W07-01`](./tasks/Task-W07-01-agent-launch-research.md) is independent and can run in parallel.

## Sessions

The current holder of every claimed card. Empty means nothing is claimed.

| Session (branch / worktree) | Card | Claimed | Next user-visible step |
|---|---|---|---|

## Waiting on the owner

The single list of open questions and approvals. Remove a row when it is answered, and record the answer in the linked document.

| Question or approval | Record | Unblocks |
|---|---|---|
| Review and accept the planning documents and agent workflow, as corrected on 10-04 | [`Task-W00-01`](./tasks/Task-W00-01-planning-docs.md), [`Task-W00-10`](./tasks/Task-W00-10-docs-consistency-review.md) | everything |
| Reuse of the owner's own Glidelines code: A extend · B standalone · C shared core · D Glidelines as a view (open-source bases are ruled out by 0008) | [0003](./decisions/0003-relationship-to-glidelines.md), [`Task-W00-03`](./tasks/Task-W00-03-glidelines-relationship.md) | app stack and repo layout (W03–W04) |
| Roles: separate approver, or always the owner? Sign-in method (magic link / password / GitHub / Google)? Agent tokens per session or per provider? Membership per project or per organization? | [0004](./decisions/0004-team-accounts-and-roles.md) | W03, W06 |
| Agent sessions: which surfaces first (CLI / desktop / cloud)? Runners on whose machines, and may teammates start agents on yours? Create worktrees automatically? Set model and effort per task? | [0005](./decisions/0005-start-and-open-agent-sessions.md) | W07 |
| **Release roadmap:** confirm the 15 small iterations (v0.1 macOS → v0.2 + Windows → … → v1.0), each a usable release, and what goes into Iteration 1. Task box name on disk: `.glidebox/` (hidden) or `glidebox/` (visible)? Is a Windows machine available for Iteration 2? | [PLAN § 9](./product/PLAN.md#9-roadmap--releases), [`Task-W00-13`](./tasks/Task-W00-13-iterative-release-roadmap.md) | starting v0.1 |
| **Selling glidebox:** commercial model and source licence (open client with paid cloud, or proprietary) | [0008](./decisions/0008-build-our-own.md) | release, licensing of dependencies |
| Local projects: how a local project records who did what, and whether companions may start agents | [0005](./decisions/0005-start-and-open-agent-sessions.md), [0006](./decisions/0006-local-and-cloud-projects.md) | W05, W07 |
| Cloud hosting target (Vercel or Cloudflare) and domain | [PLAN § 8](./product/PLAN.md#8-architecture-proposed) | Iteration 13 |
| Importer: one-time import of the SSR board, or keep syncing during the transition? | [PLAN § 9](./product/PLAN.md#9-roadmap--releases) | Iteration 6 (W02) |
| UI languages (Glidelines has en / ja / zh-CN / zh-TW) | [REQUIREMENTS](./product/REQUIREMENTS.md) | W04 |
| Review the first code card [`Task-W04-01`](./tasks/Task-W04-01-walking-skeleton.md) (Iteration 1 walking skeleton: macOS Tauri app opens a task box and shows its tasks) | the card | first code |

## Cards

**Session:** `—` means unclaimed. **State** and **Label** follow [CARDS.md](./workflow/CARDS.md#states-and-labels).

### Open

| Card | State | Label | Session | Next dependency |
|---|---|---|---|---|
| [`Task-W00-01-planning-docs`](./tasks/Task-W00-01-planning-docs.md) | waiting | drafted | — | owner review of the documents |
| [`Task-W00-03-glidelines-relationship`](./tasks/Task-W00-03-glidelines-relationship.md) | waiting | drafted | — | owner decision on 0003 |
| [`Task-W00-10-docs-consistency-review`](./tasks/Task-W00-10-docs-consistency-review.md) | waiting | drafted | — | owner review of the corrected documents |
| [`Task-W00-11-project-data-folder`](./tasks/Task-W00-11-project-data-folder.md) | waiting | drafted | — | owner review of 0010 and R72–R76 |
| [`Task-W00-13-iterative-release-roadmap`](./tasks/Task-W00-13-iterative-release-roadmap.md) | waiting | drafted | — | owner confirms the release roadmap |
| [`Task-W04-01-walking-skeleton`](./tasks/Task-W04-01-walking-skeleton.md) | backlog | — | — | owner card review (first Iteration 1 card) |
| [`Task-W01-01-core-model`](./tasks/Task-W01-01-core-model.md) | backlog | — | — | Iteration 4 (burndown and forecast) |
| [`Task-W02-01-ssr-board-import`](./tasks/Task-W02-01-ssr-board-import.md) | backlog | — | — | Iteration 6 |
| [`Task-W10-01-remote-access-research`](./tasks/Task-W10-01-remote-access-research.md) | backlog | — | — | owner card review |
| [`Task-W07-01-agent-launch-research`](./tasks/Task-W07-01-agent-launch-research.md) | backlog | — | — | owner card review |

### Recently finished

| Card | Label | When |
|---|---|---|
| [`Task-W00-02-github-remote`](./tasks/Task-W00-02-github-remote.md) | done — `mypals-ml/glidebox`, pushed by the owner | 2026-10-04 |
| [`Task-W00-12-desktop-platforms-and-shared-code`](./tasks/Task-W00-12-desktop-platforms-and-shared-code.md) | owner accepted — Tauri 2, macOS + Windows, one interface code base | 2026-10-04 |
| [`Task-W00-06-git-sync-proposal`](./tasks/Task-W00-06-git-sync-proposal.md) | owner accepted — Git text files are the source of truth | 2026-10-04 |
| [`Task-W00-07-open-source-base-decision`](./tasks/Task-W00-07-open-source-base-decision.md) | owner accepted — build our own | 2026-10-04 |
| [`Task-W00-09-interface-requirements`](./tasks/Task-W00-09-interface-requirements.md) | owner accepted (R60–R62); design proposed | 2026-10-03 |
| [`Task-W00-08-ai-generated-project-setup`](./tasks/Task-W00-08-ai-generated-project-setup.md) | owner accepted (R50–R51); details proposed | 2026-10-01 |
| [`Task-W00-05-competitor-research`](./tasks/Task-W00-05-competitor-research.md) | evidence saved at the owner's request | 2026-10-01 |
| [`Task-W00-04-local-and-cloud-requirements`](./tasks/Task-W00-04-local-and-cloud-requirements.md) | owner accepted (R40–R43) | 2026-10-01 |

### Roadmap — not yet carded

Iterations 1–15 (v0.1–v1.0) in [PLAN § 9](./product/PLAN.md#9-roadmap--releases), by workstream in [CARDS.md § Workstreams](./workflow/CARDS.md#workstreams). These include:
- the desktop hub (W04);
- companion viewing (W10, R69–R71);
- AI-generated project setup (W09, R50–R55);
- shared commands and the human CLI (R60–R68).

A card is written when its milestone becomes next.

## Blocked

Nothing is blocked beyond the owner questions above.
