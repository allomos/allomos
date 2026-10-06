# Task board — glidebox

> **Last updated:** 2026-10-06

The index of all work and the **only home of status**. Card detail lives in [docs/tasks/](./tasks/); rules for cards, labels and claiming are in [workflow/CARDS.md](./workflow/CARDS.md). Restart mechanics (worktrees, servers, pitfalls) are in [AGENT_HANDOFF.md](./AGENT_HANDOFF.md). When another document disagrees with this board, the board is right — correct the other document in place.

**To resume:** read Now, Sessions and Waiting on the owner, then open your card.

## Now

**No card is claimed; thirteen cards are open.** **Iteration 1 — v0.1 macOS local tracker** is under way ([PLAN § 9](./product/PLAN.md#9-roadmap--releases)). Two cards are done: a macOS app that opens a task box and shows its tasks by area ([`Task-A01-01`](./tasks/Task-A01-01-walking-skeleton.md)), now in English, Simplified Chinese and Japanese ([`Task-A01-02`](./tasks/Task-A01-02-ui-languages.md)); both owner verified 2026-10-06.

**Where the product stands.**
- The owner has decided to **build glidebox ourselves** rather than start from an open-source tracker ([`Task-A00-07`](./tasks/Task-A00-07-open-source-base-decision.md)).
- **The local app is free**; payment is only for cloud features ([0012](./decisions/0012-free-local-app-paid-cloud.md)).
- The product shape: a **desktop app for macOS and Windows as the hub**, built with **Tauri 2** and sharing one interface code base with the browser and phone ([`A00-12`](./tasks/Task-A00-12-desktop-platforms-and-shared-code.md)), with web and mobile companions that connect to it to view progress and status ([0009](./decisions/0009-desktop-hub-with-web-and-mobile-companions.md)). Each project's data lives in a **folder the user chooses**, standalone or inside an existing project folder ([`A00-11`](./tasks/Task-A00-11-project-data-folder.md)), as **readable text files tracked by Git** ([`A00-06`](./tasks/Task-A00-06-git-sync-proposal.md)).
- Requirements for local and cloud projects, AI-generated project setup and interfaces are recorded ([`A00-04`](./tasks/Task-A00-04-local-and-cloud-requirements.md), [`A00-08`](./tasks/Task-A00-08-ai-generated-project-setup.md), [`A00-09`](./tasks/Task-A00-09-interface-requirements.md)).
- The 2026-10-04 review has fixed the documents ([`Task-A00-10`](./tasks/Task-A00-10-docs-consistency-review.md)).

**What is next.**
- The owner reviews the documents and answers the questions below.
- The research card [`Task-A04-01`](./tasks/Task-A04-01-remote-access-research.md) looks for a free or very cheap way for companions to reach the desktop app from outside, and a free email service for the pairing code.
- The next approved Iteration 1 card is ready to claim (owner, 2026-10-06):
  - The **`glidebox` / `gb` command** for agents ([`Task-A01-03`](./tasks/Task-A01-03-gb-command.md)). `gb` and the running app take turns writing a task box with a short lock for each change (owner, 2026-10-06).
- **Rename to CrossJump**, waiting for the owner's answers: the product's new name says that a person or project can jump between AI agents and be handed between people ([`Task-A00-17`](./tasks/Task-A00-17-rename-to-crossjump.md)). Recommended before A01-03, so the command is built once under its final name.
- A small fix, waiting for the owner's review: starting glidebox with a folder should open that folder even when an option comes first, and say so when the folder is wrong ([`Task-A01-04`](./tasks/Task-A01-04-startup-folder-argument.md)).
- The research card [`Task-A03-01`](./tasks/Task-A03-01-agent-launch-research.md) is independent and can run in parallel.

## Sessions

The current holder of every claimed card. Empty means nothing is claimed.

| Session (branch / worktree) | Card | Claimed | Next user-visible step |
|---|---|---|---|

## Waiting on the owner

The single list of open questions and approvals. Remove a row when it is answered, and record the answer in the linked document.

| Question or approval | Record | Unblocks |
|---|---|---|
| Review and accept the planning documents and agent workflow, as corrected on 10-04 | [`Task-A00-01`](./tasks/Task-A00-01-planning-docs.md), [`Task-A00-10`](./tasks/Task-A00-10-docs-consistency-review.md) | everything |
| Reuse of the owner's own Glidelines code: A extend · B standalone · C shared core · D Glidelines as a view (open-source bases are ruled out by 0008) | [0003](./decisions/0003-relationship-to-glidelines.md), [`Task-A00-03`](./tasks/Task-A00-03-glidelines-relationship.md) | app stack and repo layout (W03–W04) |
| Roles: separate approver, or always the owner? Sign-in method (magic link / password / GitHub / Google)? Agent tokens per session or per provider? Membership per project or per organization? | [0004](./decisions/0004-team-accounts-and-roles.md) | W03, W06 |
| Agent sessions: which surfaces first (CLI / desktop / cloud)? Runners on whose machines, and may teammates start agents on yours? Create worktrees automatically? Set model and effort per task? | [0005](./decisions/0005-start-and-open-agent-sessions.md) | W07 |
| **Free local app** (0012 accepted): open-source the local app or keep it closed but free? Companions away from home: free via the user's own tunnel account, a paid hosted relay, or both? What else is paid besides cloud copy and teams? Any limits on free use (recommended: none)? | [0012](./decisions/0012-free-local-app-paid-cloud.md) | Iterations 10, 13–15 |
| Local projects: how a local project records who did what, and whether companions may start agents | [0005](./decisions/0005-start-and-open-agent-sessions.md), [0006](./decisions/0006-local-and-cloud-projects.md) | W05, W07 |
| Cloud hosting target (Vercel or Cloudflare) and domain | [PLAN § 8](./product/PLAN.md#8-architecture-proposed) | Iteration 13 |
| Rename to CrossJump: command short name (`cj`?), data folder and format tag, app ID, which old documents keep the old name, order before A01-03; GitHub repository and local folder rename | [`Task-A00-17`](./tasks/Task-A00-17-rename-to-crossjump.md) | the rename; A01-03's command name |
| Importer: one-time import of the SSR board, or keep syncing during the transition? | [PLAN § 9](./product/PLAN.md#9-roadmap--releases) | Iteration 7 |

## Cards

**Session:** `—` means unclaimed. **State** and **Label** follow [CARDS.md](./workflow/CARDS.md#states-and-labels).

### Open

| Card | State | Label | Session | Next dependency |
|---|---|---|---|---|
| [`Task-A00-01-planning-docs`](./tasks/Task-A00-01-planning-docs.md) | waiting | drafted | — | owner review of the documents |
| [`Task-A00-03-glidelines-relationship`](./tasks/Task-A00-03-glidelines-relationship.md) | waiting | drafted | — | owner decision on 0003 |
| [`Task-A00-10-docs-consistency-review`](./tasks/Task-A00-10-docs-consistency-review.md) | waiting | drafted | — | owner review of the corrected documents |
| [`Task-A00-11-project-data-folder`](./tasks/Task-A00-11-project-data-folder.md) | waiting | drafted | — | owner review of 0010 and R72–R76 |
| [`Task-A00-14-areas-and-naming`](./tasks/Task-A00-14-areas-and-naming.md) | waiting | drafted | — | owner reviews the new areas |
| [`Task-A00-15-free-local-app`](./tasks/Task-A00-15-free-local-app.md) | waiting | drafted | — | owner answers 0012's open questions |
| [`Task-A01-03-gb-command`](./tasks/Task-A01-03-gb-command.md) | ready | — | — | a session claims it (recommended after A00-17, the rename) |
| [`Task-A00-17-rename-to-crossjump`](./tasks/Task-A00-17-rename-to-crossjump.md) | waiting | drafted | — | owner answers the card's questions |
| [`Task-A01-04-startup-folder-argument`](./tasks/Task-A01-04-startup-folder-argument.md) | waiting | drafted | — | owner reviews the card |
| [`Task-A02-01-burndown-and-forecast`](./tasks/Task-A02-01-burndown-and-forecast.md) | backlog | — | — | Iteration 5 (burndown and forecast) |
| [`Task-A05-01-ssr-board-import`](./tasks/Task-A05-01-ssr-board-import.md) | backlog | — | — | Iteration 7 |
| [`Task-A04-01-remote-access-research`](./tasks/Task-A04-01-remote-access-research.md) | backlog | — | — | owner card review |
| [`Task-A03-01-agent-launch-research`](./tasks/Task-A03-01-agent-launch-research.md) | backlog | — | — | owner card review |

### Recently finished

| Card | Label | When |
|---|---|---|
| [`Task-A01-02-ui-languages`](./tasks/Task-A01-02-ui-languages.md) | **owner verified — Done**: the app speaks English, Simplified Chinese and Japanese, follows the computer's language, and keeps a chosen one | 2026-10-06 |
| [`Task-A00-02-github-remote`](./tasks/Task-A00-02-github-remote.md) | done — `mypals-ml/glidebox`, pushed by the owner | 2026-10-04 |
| [`Task-A01-01-walking-skeleton`](./tasks/Task-A01-01-walking-skeleton.md) | **owner verified — Done**: the macOS app opens a task box and shows its tasks by area | 2026-10-06 |
| [`Task-A00-16-road-show-deck`](./tasks/Task-A00-16-road-show-deck.md) | done — English and Chinese decks published; addresses in [pitch/](./pitch/README.md) | 2026-10-04 |
| [`Task-A00-13-iterative-release-roadmap`](./tasks/Task-A00-13-iterative-release-roadmap.md) | owner confirmed — 15 iterations, each a usable release | 2026-10-04 |
| [`Task-A00-12-desktop-platforms-and-shared-code`](./tasks/Task-A00-12-desktop-platforms-and-shared-code.md) | owner accepted — Tauri 2, macOS + Windows, one interface code base | 2026-10-04 |
| [`Task-A00-06-git-sync-proposal`](./tasks/Task-A00-06-git-sync-proposal.md) | owner accepted — Git text files are the source of truth | 2026-10-04 |
| [`Task-A00-07-open-source-base-decision`](./tasks/Task-A00-07-open-source-base-decision.md) | owner accepted — build our own | 2026-10-04 |
| [`Task-A00-09-interface-requirements`](./tasks/Task-A00-09-interface-requirements.md) | owner accepted (R60–R62); design proposed | 2026-10-03 |
| [`Task-A00-08-ai-generated-project-setup`](./tasks/Task-A00-08-ai-generated-project-setup.md) | owner accepted (R50–R51); details proposed | 2026-10-01 |
| [`Task-A00-05-competitor-research`](./tasks/Task-A00-05-competitor-research.md) | evidence saved at the owner's request | 2026-10-01 |
| [`Task-A00-04-local-and-cloud-requirements`](./tasks/Task-A00-04-local-and-cloud-requirements.md) | owner accepted (R40–R43) | 2026-10-01 |

### Roadmap — not yet carded

Iterations 1–15 (v0.1–v1.0) in [PLAN § 9](./product/PLAN.md#9-roadmap--releases), by area in [CARDS.md § Areas](./workflow/CARDS.md#areas). These include:
- the desktop hub (W04);
- companion viewing (W10, R69–R71);
- AI-generated project setup (W09, R50–R55);
- shared commands and the human CLI (R60–R68).

A card is written when its milestone becomes next.

## Blocked

Nothing is blocked beyond the owner questions above.
