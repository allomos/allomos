# AGENT_HANDOFF

Updated: 2026-10-01

**What this file is:** restart mechanics for the next session — which worktree, which server, which pitfall. It is **not** status: status lives on [the board](./TASK_BOARD.md). Contents and timing rules: [workflow/AI-WORKFLOW.md § Handoff](./workflow/AI-WORKFLOW.md#handoff-timing-and-contents).

## Read order to resume

1. [AGENTS.md](../AGENTS.md) (loaded automatically).
2. [Board](./TASK_BOARD.md): Now → Sessions → Waiting on the owner → your card file.
3. This file: your block in [Active resume](#active-resume), then [Live state](#live-state) and [Pitfalls](#pitfalls).
4. What [AGENTS.md § Read by activity](../AGENTS.md#read-by-activity) requires for your task.

## How to update this file

- **One block per claimed card** under Active resume, **replaced in place**, never appended; about 10 lines: checkout, approvals in force, uncommitted changes, running processes, unrecorded findings, the single next action.
- Each session edits **only its own block**.
- **Evidence goes on the card**, not here. Write "see card".
- **When you release or finish a card, delete its block** in the same commit.
- Update [Live state](#live-state) when you start or stop a server, create a hosted resource or change a remote. Add to [Pitfalls](#pitfalls) only a lesson that changes what the next agent does.
- **Never write secrets.** Credential locations only, under [Current truth](#current-truth).

### Block template

```markdown
### Task-W…-… — <session name>
- Checkout: <worktree path> · branch <name> · last commit <hash> · pushed: yes/no
- Approvals in force: <what is approved; what is NOT>
- Uncommitted: <file — why> (or none)
- Running: <process, port, restart command, safe to stop?> (or none)
- Unrecorded findings: <…> (or none)
- Next action: <one step>
```

## Active resume

One block per card in the board's Sessions table. None claimed.

## Live state

Checked 2026-10-01.

| What | Where | Notes |
|---|---|---|
| Main checkout | `/Users/wanghui/coding/glidepace/glidebox`, branch `main` | Documentation only; no application code, no dependencies installed |
| Git remote | **none** | Do not add or push until [`Task-W00-02`](./tasks/Task-W00-02-github-remote.md) is approved |
| Worktrees | `/Users/wanghui/coding/glidepace/glidebox-local-cloud`, branch `codex/local-cloud-requirements` | Local/cloud requirements, research and sync proposal; no processes |
| Servers / hosted resources | none | — |

## Pitfalls

Lessons that change what you do.

- **Reference projects are read-only** and some live outside this repo ([REFERENCES.md](./REFERENCES.md)). The SSR project changes daily; re-read a file before quoting a count or rule from it, and cite the date read.
- **The SSR board's rules evolved on 2026-09-28:** status now lives only on each card and the board is generated (`check-board.mjs`, pre-commit hook). Earlier notes in [analysis/ssr-task-board.md](./analysis/ssr-task-board.md) describe the 2026-09-27 state.
- **zsh expands a word starting with `=`** (e.g. `echo =====`) as a command lookup and fails; quote such strings in shell commands.

## Current truth

Decisions and boundaries the board does not carry. If a bullet contradicts the board, the board is right — correct the bullet.

- **Git identity** (repo-local config, copied from `glidepace-vibe-cc`): `will.w.hui <willwhui@gmail.com>`. SSH to GitHub authenticates as **`wanghui2sea`**; the owner's other projects push to the `mypals-ml` organization.
- **Accepted decisions:** [0001](./decisions/0001-database-source-of-truth.md) database is the source of truth; [0002](./decisions/0002-burndown-counts-plus-sizes.md) burndown by counts plus optional sizes; [0004](./decisions/0004-team-accounts-and-roles.md) roles, owner/admin invite by email; [0005](./decisions/0005-start-and-open-agent-sessions.md) start/open agent sessions (requirement). [0006](./decisions/0006-local-and-cloud-projects.md) local-only projects without sign-in and optional signed-in cloud projects (requirement; storage and sync design open). **Open:** [0003](./decisions/0003-relationship-to-glidelines.md) relationship to Glidelines.
- **Until glidebox can track itself (M3)**, this project's cards are Markdown in this repo; see [AI-WORKFLOW § dogfooding](./workflow/AI-WORKFLOW.md#after-glidebox-can-track-itself-dogfooding).
- **Credentials:** none are used by this project yet. When one is introduced, record its *location* here (never its value) and ignore its file in `.gitignore`.

## Do not assume

- That a decision marked *Proposed* or *Open* is approved. [0007](./decisions/0007-git-like-project-sync.md) records the owner’s Git-like sync question as a proposal; it now recommends actual Git and structured files as a candidate; 0001 remains accepted until explicitly superseded. It does not authorize implementation.
- That the SSR board's current rules or counts match what [the analysis](./analysis/ssr-task-board.md) recorded — it is a dated snapshot.
- That glidebox will extend the Glidelines repository — that is decision 0003, still open.
- That a remote exists or a push landed — check `git remote -v` and the remote itself.
