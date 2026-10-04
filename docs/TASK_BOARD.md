# Task board — glidebox

> **Last updated:** 2026-10-04

The index of all work and the **only home of status**. Card detail lives in [docs/tasks/](./tasks/); rules for cards, labels and claiming are in [workflow/CARDS.md](./workflow/CARDS.md). Restart mechanics (worktrees, servers, pitfalls) are in [AGENT_HANDOFF.md](./AGENT_HANDOFF.md). When another document disagrees with this board, the board is right — correct the other document in place.

**To resume:** read Now, Sessions and Waiting on the owner, then open your card.

## Now

**Nothing is claimed; ten cards are open; no application code exists yet.** Glidebox is in planning (milestone M0).

**Where the product stands.**
- The owner has decided to **build glidebox ourselves** rather than start from an open-source tracker ([`Task-W00-07`](./tasks/Task-W00-07-open-source-base-decision.md)).
- The product shape: a **desktop app as the hub**, with web and mobile companions that connect to it to view progress and status ([0009](./decisions/0009-desktop-hub-with-web-and-mobile-companions.md)). Each project's data lives in a **folder the user chooses**, standalone or inside an existing project folder ([`W00-11`](./tasks/Task-W00-11-project-data-folder.md)).
- Requirements for local and cloud projects, AI-generated project setup and interfaces are recorded ([`W00-04`](./tasks/Task-W00-04-local-and-cloud-requirements.md), [`W00-08`](./tasks/Task-W00-08-ai-generated-project-setup.md), [`W00-09`](./tasks/Task-W00-09-interface-requirements.md)).
- The 2026-10-04 review has fixed the documents ([`Task-W00-10`](./tasks/Task-W00-10-docs-consistency-review.md)).

**What is next.**
- The owner reviews the documents and answers the questions below. The ones that most shape the first code are: Git storage or a database (0007), and the desktop and companion questions (0009).
- The research card [`Task-W10-01`](./tasks/Task-W10-01-remote-access-research.md) looks for a free or very cheap way for companions to reach the desktop app from outside.
- The first code card, [`Task-W01-01`](./tasks/Task-W01-01-core-model.md) (core model, burndown and forecast), works under every open option and can start once the owner reviews it.
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
| GitHub repository for this project (e.g. `mypals-ml/glidebox`) — create it, then approve adding it as `origin` and pushing | [`Task-W00-02`](./tasks/Task-W00-02-github-remote.md) | pushing; teammates; worktree sessions across machines |
| Reuse of the owner's own Glidelines code: A extend · B standalone · C shared core · D Glidelines as a view (open-source bases are ruled out by 0008) | [0003](./decisions/0003-relationship-to-glidelines.md), [`Task-W00-03`](./tasks/Task-W00-03-glidelines-relationship.md) | app stack and repo layout (W03–W04) |
| Roles: separate approver, or always the owner? Sign-in method (magic link / password / GitHub / Google)? Agent tokens per session or per provider? Membership per project or per organization? | [0004](./decisions/0004-team-accounts-and-roles.md) | W03, W06 |
| Agent sessions: which surfaces first (CLI / desktop / cloud)? Runners on whose machines, and may teammates start agents on yours? Create worktrees automatically? Set model and effort per task? | [0005](./decisions/0005-start-and-open-agent-sessions.md) | W07 |
| **Desktop hub and companions:** mobile companion as an installed native app or a phone-friendly web page (two different answers on 10-04)? What do companions show when the desktop is off? Which desktop systems first? How should the desktop app send the code email? | [0009](./decisions/0009-desktop-hub-with-web-and-mobile-companions.md) | W04, W10, M2–M2b |
| **Data folder contents:** one database file, or readable text files tracked by Git? (Recommended: text files — this would accept 0007 and replace 0001.) | [0010](./decisions/0010-project-data-folder.md), [0007](./decisions/0007-git-like-project-sync.md) | local store design (W03) |
| **Milestone order:** confirm moving cloud projects from M2 to M4, so that M2 = desktop hub and M2b = companion viewing; confirm placing AI-generated setup (W09) with M2 | [PLAN § 9](./product/PLAN.md#9-milestones) | planning of M2 |
| **Selling glidebox:** commercial model and source licence (open client with paid cloud, or proprietary) | [0008](./decisions/0008-build-our-own.md) | release, licensing of dependencies |
| Local projects: how a local project records who did what, and whether companions may start agents | [0005](./decisions/0005-start-and-open-agent-sessions.md), [0006](./decisions/0006-local-and-cloud-projects.md) | W05, W07 |
| Cloud hosting target (Vercel or Cloudflare) and domain | [PLAN § 8](./product/PLAN.md#8-architecture-proposed) | M2 deployment |
| Importer: one-time import of the SSR board, or keep syncing during the transition? | [PLAN § 9](./product/PLAN.md#9-milestones) | W02 |
| UI languages (Glidelines has en / ja / zh-CN / zh-TW) | [REQUIREMENTS](./product/REQUIREMENTS.md) | W04 |
| Review card [`Task-W01-01`](./tasks/Task-W01-01-core-model.md) (and its proposed TypeScript + pnpm + Vitest toolchain) | the card | first code |

## Cards

**Session:** `—` means unclaimed. **State** and **Label** follow [CARDS.md](./workflow/CARDS.md#states-and-labels).

### Open

| Card | State | Label | Session | Next dependency |
|---|---|---|---|---|
| [`Task-W00-01-planning-docs`](./tasks/Task-W00-01-planning-docs.md) | waiting | drafted | — | owner review of the documents |
| [`Task-W00-02-github-remote`](./tasks/Task-W00-02-github-remote.md) | waiting | — | — | owner names/creates the repository and approves the push |
| [`Task-W00-03-glidelines-relationship`](./tasks/Task-W00-03-glidelines-relationship.md) | waiting | drafted | — | owner decision on 0003 |
| [`Task-W00-06-git-sync-proposal`](./tasks/Task-W00-06-git-sync-proposal.md) | waiting | drafted | — | owner decision on 0007 |
| [`Task-W00-10-docs-consistency-review`](./tasks/Task-W00-10-docs-consistency-review.md) | waiting | drafted | — | owner review of the corrected documents |
| [`Task-W00-11-project-data-folder`](./tasks/Task-W00-11-project-data-folder.md) | waiting | drafted | — | owner review of 0010 and R72–R76 |
| [`Task-W01-01-core-model`](./tasks/Task-W01-01-core-model.md) | backlog | — | — | owner card review |
| [`Task-W02-01-ssr-board-import`](./tasks/Task-W02-01-ssr-board-import.md) | backlog | — | — | `Task-W01-01` model; owner card review |
| [`Task-W10-01-remote-access-research`](./tasks/Task-W10-01-remote-access-research.md) | backlog | — | — | owner card review |
| [`Task-W07-01-agent-launch-research`](./tasks/Task-W07-01-agent-launch-research.md) | backlog | — | — | owner card review |

### Recently finished

| Card | Label | When |
|---|---|---|
| [`Task-W00-07-open-source-base-decision`](./tasks/Task-W00-07-open-source-base-decision.md) | owner accepted — build our own | 2026-10-04 |
| [`Task-W00-09-interface-requirements`](./tasks/Task-W00-09-interface-requirements.md) | owner accepted (R60–R62); design proposed | 2026-10-03 |
| [`Task-W00-08-ai-generated-project-setup`](./tasks/Task-W00-08-ai-generated-project-setup.md) | owner accepted (R50–R51); details proposed | 2026-10-01 |
| [`Task-W00-05-competitor-research`](./tasks/Task-W00-05-competitor-research.md) | evidence saved at the owner's request | 2026-10-01 |
| [`Task-W00-04-local-and-cloud-requirements`](./tasks/Task-W00-04-local-and-cloud-requirements.md) | owner accepted (R40–R43) | 2026-10-01 |

### Roadmap — not yet carded

Milestones M2–M6 in [PLAN § 9](./product/PLAN.md#9-milestones), by workstream in [CARDS.md § Workstreams](./workflow/CARDS.md#workstreams). These include:
- the desktop hub (W04);
- companion viewing (W10, R69–R71);
- AI-generated project setup (W09, R50–R55);
- shared commands and the human CLI (R60–R68).

A card is written when its milestone becomes next.

## Blocked

Nothing is blocked beyond the owner questions above.
