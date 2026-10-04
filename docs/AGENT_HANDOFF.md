# AGENT_HANDOFF

Updated: 2026-10-04

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

One block per card in the board's Sessions table.

### Task-W00-11 — `main` (Claude Code docs session)
- Checkout: `/Users/wanghui/coding/glidepace/glidebox` · branch `main` · pushed: no (no remote)
- Approvals in force: record the owner's data-folder requirement (local docs only). Nothing external.
- Uncommitted: decision 0010 and related edits, until the next commit
- Running: none
- Unrecorded findings: none
- Next action: commit the record, then the owner reviews it

## Live state

Checked 2026-10-04.

| What | Where | Notes |
|---|---|---|
| Main checkout | `/Users/wanghui/coding/glidepace/glidebox`, branch `main` | Documentation only; no application code, no dependencies installed |
| Git remote | **none** | Do not add or push until [`Task-W00-02`](./tasks/Task-W00-02-github-remote.md) is approved |
| Unused worktree | `/Users/wanghui/coding/glidepace/glidebox-local-cloud`, branch `codex/local-cloud-requirements` | Left by the 10-01 to 10-03 Codex session. Clean, at the same commit as `main` as of 2026-10-04, no card claimed. The agent's removal was blocked by the permission system. **Owner to remove:** `git worktree remove ../glidebox-local-cloud` then `git branch -d codex/local-cloud-requirements` |
| Servers / hosted resources | none | — |

## Pitfalls

Lessons that change what you do.

- **Reference projects are read-only** and some live outside this repo ([REFERENCES.md](./REFERENCES.md)). The SSR project changes daily; re-read a file before quoting a count or rule from it, and cite the date read.
- **The SSR board's rules evolved on 2026-09-28:** status now lives only on each card and the board is generated (`check-board.mjs`, pre-commit hook). Earlier notes in [analysis/ssr-task-board.md](./analysis/ssr-task-board.md) describe the 2026-09-27 state.
- **Do not grow one card into a catch-all.** On 10-01 to 10-03, six different outcomes were added to `Task-W00-01`; on 10-04 they had to be split back out. Open a new card for each outcome ([AGENTS.md § Working procedure](../AGENTS.md#working-procedure)).
- **A handoff updates this file and the board** — never a new "handoff" document.
- **zsh expands a word starting with `=`** (e.g. `echo =====`) as a command lookup and fails; quote such strings in shell commands.

## Current truth

Decisions and boundaries the board does not carry. If a bullet contradicts the board, the board is right — correct the bullet.

- **Git identity** (repo-local config, copied from `glidepace-vibe-cc`): `will.w.hui <willwhui@gmail.com>`. SSH to GitHub authenticates as **`wanghui2sea`**; the owner's other projects push to the `mypals-ml` organization.
- **Decisions:**
  - Accepted: [0001](./decisions/0001-database-source-of-truth.md) database is the source of truth; [0002](./decisions/0002-burndown-counts-plus-sizes.md) burndown by counts plus optional sizes; [0008](./decisions/0008-build-our-own.md) **build our own**, no open-source base.
  - Accepted in part or as requirements: [0004](./decisions/0004-team-accounts-and-roles.md) roles, owner/admin invite by email; [0005](./decisions/0005-start-and-open-agent-sessions.md) start/open agent sessions; [0006](./decisions/0006-local-and-cloud-projects.md) local-only and optional cloud projects; [0009](./decisions/0009-desktop-hub-with-web-and-mobile-companions.md) **desktop hub**, with web and mobile companions viewing progress and status.
  - Open: [0003](./decisions/0003-relationship-to-glidelines.md) reuse of the owner's Glidelines code.
  - Proposed: [0007](./decisions/0007-git-like-project-sync.md) Git storage and sync.
- **Commercial intent:** the owner may sell glidebox as a service (2026-10-01). Model and source licence are undecided (board).
- **Until glidebox can track itself (M3)**, this project's cards are Markdown in this repo; see [AI-WORKFLOW § dogfooding](./workflow/AI-WORKFLOW.md#after-glidebox-can-track-itself-dogfooding).
- **Credentials:** none are used by this project yet. When one is introduced, record its *location* here (never its value) and ignore its file in `.gitignore`.

## Do not assume

- That a decision marked *Proposed* or *Open* is approved — in particular 0007 (Git storage) and 0003 (Glidelines reuse). 0001 stays in force until explicitly superseded.
- That companions may edit — 0009 accepts viewing; editing, remote access and pairing are open.
- That an open-source tracker may be forked or its code copied — 0008 rules that out.
- That interface, generated-project or packaging details marked *proposed* in [INTERFACES](./product/INTERFACES.md) and [AI-GENERATED-PROJECTS](./product/AI-GENERATED-PROJECTS.md) are accepted scope.
- That the SSR board's current rules or counts match what [the analysis](./analysis/ssr-task-board.md) recorded — it is a dated snapshot.
- That competitor table cells marked `?` mean "absent" — they mean "not established".
- That a remote exists or a push landed — check `git remote -v` and the remote itself.
