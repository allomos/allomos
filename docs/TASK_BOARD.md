# Task board — FlipHopper

> **Last updated:** 2026-10-06

The index of all work and the **only home of status**. Card detail lives in [docs/tasks/](./tasks/); rules for cards, labels and claiming are in [workflow/CARDS.md](./workflow/CARDS.md). Restart mechanics (worktrees, servers, pitfalls) are in [AGENT_HANDOFF.md](./AGENT_HANDOFF.md). When another document disagrees with this board, the board is right — correct the other document in place.

**To resume:** read Now, Sessions and Waiting on the owner, then open your card.

## Now

**No card is claimed; fourteen cards are open.** **Iteration 1 — v0.1 macOS local tracker** is under way ([PLAN § 9](./product/PLAN.md#9-roadmap--releases)). Five cards are done, all owner verified 2026-10-06:
- a macOS app that opens a seabag and shows its tasks by area ([`Task-A01-01`](./tasks/Task-A01-01-walking-skeleton.md)), now in English, Simplified Chinese and Japanese ([`Task-A01-02`](./tasks/Task-A01-02-ui-languages.md));
- the **`fliphopper` / `fh` command** ([`Task-A01-03`](./tasks/Task-A01-03-gb-command.md)): agents create a seabag and claim, label, park, finish and release tasks, leaving a handoff note for the next one. A short lock for each change means two agents cannot take the same task, and the open app follows the changes;
- a **Show in Finder / Explorer** link for the open seabag ([`Task-A01-06`](./tasks/Task-A01-06-show-seabag-in-file-manager.md));
- a new **app icon**: two pairs of balls hopping through one spark, jade and terracotta on sand ([`Task-A07-01`](./tasks/Task-A07-01-app-icon-hopper.md)).

**Where the product stands.**
- The owner has decided to **build FlipHopper ourselves** rather than start from an open-source tracker ([`Task-A00-07`](./tasks/Task-A00-07-open-source-base-decision.md)).
- **The local app is free**; payment is only for cloud features ([0012](./decisions/0012-free-local-app-paid-cloud.md)).
- The product shape: a **desktop app for macOS and Windows as the hub**, built with **Tauri 2** and sharing one interface code base with the browser and phone ([`A00-12`](./tasks/Task-A00-12-desktop-platforms-and-shared-code.md)), with web and mobile companions that connect to it to view progress and status ([0009](./decisions/0009-desktop-hub-with-web-and-mobile-companions.md)). Each project's data lives in a **folder the user chooses**, standalone or inside an existing project folder ([`A00-11`](./tasks/Task-A00-11-project-data-folder.md)), as **readable text files tracked by Git** ([`A00-06`](./tasks/Task-A00-06-git-sync-proposal.md)).
- Requirements for local and cloud projects, AI-generated project setup and interfaces are recorded ([`A00-04`](./tasks/Task-A00-04-local-and-cloud-requirements.md), [`A00-08`](./tasks/Task-A00-08-ai-generated-project-setup.md), [`A00-09`](./tasks/Task-A00-09-interface-requirements.md)).
- The 2026-10-04 review has fixed the documents ([`Task-A00-10`](./tasks/Task-A00-10-docs-consistency-review.md)).

**What is next.**
- The owner reviews the documents and answers the questions below.
- The research card [`Task-A04-01`](./tasks/Task-A04-01-remote-access-research.md) looks for a free or very cheap way for companions to reach the desktop app from outside, and a free email service for the pairing code.
- **Renamed to FlipHopper** (done 2026-10-06): a person or project hops back and forth between AI agents (反复横跳) and is handed between people; a project's data is a **seabag** in `.fliphopper/seabag/` ([`Task-A00-17`](./tasks/Task-A00-17-rename-to-crossjump.md), [0013](./decisions/0013-product-name-fliphopper.md)).
- **New main message** ([0014](./decisions/0014-main-purpose-and-three-sets.md)): switch between AI agents and teammates and keep the project; AI skills and progress management are the two other sets. Owner accepted the rewritten decks (done 2026-10-06) ([`Task-A00-18`](./tasks/Task-A00-18-positioning-and-deck.md)).
- Ready: the road show decks get the app's new mark ([`Task-A00-19`](./tasks/Task-A00-19-decks-new-mark.md)).
- Ready: when FlipHopper fills a folder that looks empty, it adds `README-fliphopper.md` ([`Task-A01-05`](./tasks/Task-A01-05-readme-in-new-folder.md)).
- A small fix, waiting for the owner's review: starting FlipHopper with a folder should open that folder even when an option comes first, and say so when the folder is wrong ([`Task-A01-04`](./tasks/Task-A01-04-startup-folder-argument.md)).
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
| Republish both road show decks at their existing private addresses with the new mark | [`Task-A00-19`](./tasks/Task-A00-19-decks-new-mark.md) | A00-19's last step |
| Cloud hosting target (Vercel or Cloudflare) and domain | [PLAN § 8](./product/PLAN.md#8-architecture-proposed) | Iteration 13 |
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
| [`Task-A00-19-decks-new-mark`](./tasks/Task-A00-19-decks-new-mark.md) | ready | — | — | a session claims it; the republish needs the owner's approval |
| [`Task-A01-05-readme-in-new-folder`](./tasks/Task-A01-05-readme-in-new-folder.md) | ready | — | — | a session claims it (after A00-17 merges) |
| [`Task-A01-04-startup-folder-argument`](./tasks/Task-A01-04-startup-folder-argument.md) | waiting | drafted | — | owner reviews the card |
| [`Task-A02-01-burndown-and-forecast`](./tasks/Task-A02-01-burndown-and-forecast.md) | backlog | — | — | Iteration 5 (burndown and forecast) |
| [`Task-A05-01-ssr-board-import`](./tasks/Task-A05-01-ssr-board-import.md) | backlog | — | — | Iteration 7 |
| [`Task-A04-01-remote-access-research`](./tasks/Task-A04-01-remote-access-research.md) | backlog | — | — | owner card review |
| [`Task-A03-02-account-quota-idea`](./tasks/Task-A03-02-account-quota-idea.md) | backlog | — | — | owner decides when to research it |
| [`Task-A03-01-agent-launch-research`](./tasks/Task-A03-01-agent-launch-research.md) | backlog | — | — | owner card review |

### Recently finished

| Card | Label | When |
|---|---|---|
| [`Task-A07-01-app-icon-hopper`](./tasks/Task-A07-01-app-icon-hopper.md) | **owner verified — Done**: app icon and window mark, design Y2 in colours D4 (flat); `make-icons.sh` rebuilds the set | 2026-10-06 |
| [`Task-A01-06-show-seabag-in-file-manager`](./tasks/Task-A01-06-show-seabag-in-file-manager.md) | **owner verified — Done**: Show in Finder / Explorer for the open seabag | 2026-10-06 |
| [`Task-A01-03-gb-command`](./tasks/Task-A01-03-gb-command.md) | **owner verified — Done**: the `fh` command for agents, a lock for each change, handoff notes, and the app following changes | 2026-10-06 |
| [`Task-A00-18-positioning-and-deck`](./tasks/Task-A00-18-positioning-and-deck.md) | **owner accepted — Done**: main purpose is switching between AI agents and teammates (0014); decks rebuilt around the three sets, with the seabag and hopper mark | 2026-10-06 |
| [`Task-A00-17-rename-to-crossjump`](./tasks/Task-A00-17-rename-to-crossjump.md) | **owner verified — Done**: the product is FlipHopper; the app makes a seabag in `.fliphopper/seabag/` and says so in three languages; decks republished at new addresses | 2026-10-06 |
| [`Task-A01-02-ui-languages`](./tasks/Task-A01-02-ui-languages.md) | **owner verified — Done**: the app speaks English, Simplified Chinese and Japanese, follows the computer's language, and keeps a chosen one | 2026-10-06 |
| [`Task-A00-02-github-remote`](./tasks/Task-A00-02-github-remote.md) | done — `mypals-ml/glidebox` (renamed `mypals-ml/fliphopper` 2026-10-06), pushed by the owner | 2026-10-04 |
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
