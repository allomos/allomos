# Task board — glidebox

> **Last updated:** 2026-10-03

The index of all work and the **only home of status**. Card detail lives in [docs/tasks/](./tasks/); rules for cards, labels and claiming are in [workflow/CARDS.md](./workflow/CARDS.md). Restart mechanics (worktrees, servers, pitfalls) are in [AGENT_HANDOFF.md](./AGENT_HANDOFF.md). When another document disagrees with this board, the board is right — correct the other document in place.

**To resume:** read Now, Sessions and Waiting on the owner, then open your card.

## Now

**Nothing is claimed; six cards are open; no application code exists yet.** Glidebox is in planning (milestone M0). The plan, requirements, decision records, and the agent workflow and handoff rules are drafted and wait for the owner's review ([`Task-W00-01`](./tasks/Task-W00-01-planning-docs.md)).

**What is next.** The owner reviews the documents and answers the questions below. The first code card, [`Task-W01-01`](./tasks/Task-W01-01-core-model.md) (core model, burndown and forecast), is safe under every option of the open Glidelines decision and can start once the owner reviews it. The research spike on how to launch and resume Claude Code and Codex sessions ([`Task-W07-01`](./tasks/Task-W07-01-agent-launch-research.md)) is independent and can run in parallel.

## Sessions

The current holder of every claimed card. Empty means nothing is claimed.

| Session (branch / worktree) | Card | Claimed | Next user-visible step |
|---|---|---|---|
| `main` — Claude Code docs-review session | [`Task-W00-10`](./tasks/Task-W00-10-docs-consistency-review.md) | 2026-10-04 | Owner reviews the corrected documents |

## Waiting on the owner

The single list of open questions and approvals. Remove a row when it is answered, and record the answer in the linked document.

| Question or approval | Record | Unblocks |
|---|---|---|
| Review and accept the planning documents and agent workflow | [`Task-W00-01`](./tasks/Task-W00-01-planning-docs.md) | everything |
| GitHub repository for this project (e.g. `mypals-ml/glidebox`) — create it, then approve adding it as `origin` and pushing | [`Task-W00-02`](./tasks/Task-W00-02-github-remote.md) | pushing; teammates; worktree sessions across machines |
| Relationship to Glidelines: A extend · B standalone · C shared core · D Glidelines as a view | [0003](./decisions/0003-relationship-to-glidelines.md), [`Task-W00-03`](./tasks/Task-W00-03-glidelines-relationship.md) | app stack and repo layout (W03–W04) |
| Roles: separate approver, or always the owner? Sign-in method (magic link / password / GitHub / Google)? Agent tokens per session or per provider? Membership per project or per organization? | [0004](./decisions/0004-team-accounts-and-roles.md) | W03, W06 |
| Agent sessions: which surfaces first (CLI / desktop / cloud)? Runners on whose machines, and may teammates start agents on yours? Create worktrees automatically? Set model and effort per task? | [0005](./decisions/0005-start-and-open-agent-sessions.md) | W07 |
| Local storage technology and packaging; Git-like push/pull/merge proposal | [0006](./decisions/0006-local-and-cloud-projects.md), [0007](./decisions/0007-git-like-project-sync.md) | M2 storage and local app design |
| Interface platforms/packaging and release order: desktop/mobile scope accepted; browser/human CLI considered; shared commands and client patterns proposed | [INTERFACES](./product/INTERFACES.md), [research](./research/interface-patterns.md) | interface implementation and packaging cards |
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
| [`Task-W01-01-core-model`](./tasks/Task-W01-01-core-model.md) | backlog | — | — | owner card review |
| [`Task-W02-01-ssr-board-import`](./tasks/Task-W02-01-ssr-board-import.md) | backlog | — | — | `Task-W01-01` model; owner card review |
| [`Task-W00-10-docs-consistency-review`](./tasks/Task-W00-10-docs-consistency-review.md) | in progress | — | `main` — Claude Code docs-review session | fixes in progress |
| [`Task-W07-01-agent-launch-research`](./tasks/Task-W07-01-agent-launch-research.md) | backlog | — | — | owner card review |

### Recently finished

| Card | Label | When |
|---|---|---|

### Roadmap — not yet carded

Human/agent interface scope (R60–R62) is recorded; shared command contracts and desktop/mobile/browser/CLI packaging will receive bounded cards when next, using [INTERFACES.md](./product/INTERFACES.md) and [research](./research/interface-patterns.md).

AI-generated project setup (R50–R51) is accepted; its schema/prompt and import implementation will receive a separate card when next, using [the feature contract](./product/AI-GENERATED-PROJECTS.md).

Milestones M2–M6 in [PLAN § 9](./product/PLAN.md#9-milestones), by workstream in [CARDS.md § Workstreams](./workflow/CARDS.md#workstreams). A card is written when its milestone becomes next.

## Blocked

Nothing is blocked beyond the owner questions above.
