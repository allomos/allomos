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

### Task-A00-17 — claude/a00-17-rename
- Checkout: worktree `.claude/worktrees/a00-17`, branch `claude/a00-17-rename` · pushed: yes. Not merged into `main` yet.
- Approvals in force: the card and its external operations, for the name FlipHopper (includes updating the published pitch decks).
- Done: steps 1–4 and 6; step 5 sources. Step 4: code, seabag, texts in three languages, documents, decision 0013; all six card checks, links and `git diff --check` PASS (evidence on the card).
- Uncommitted: none · Running: none
- Next: step 5 is blocked — deck sources renamed and committed, but `Artifact read` finds neither published address (wrong account?). Owner chooses: republish from the owning account, or approve new addresses (then update `docs/pitch/README.md`). Step 6 passed (owner, 2026-10-06). Then merge into `main`. The owner's README idea is card A01-05, not this one.
- Resume prompt: "Resume Task-A00-17 (rename to FlipHopper) in /Users/wanghui/coding/glidepace/fliphopper/.claude/worktrees/a00-17. Read AGENTS.md, the board, the card docs/tasks/Task-A00-17-rename-to-crossjump.md and your block in docs/AGENT_HANDOFF.md. Continue with step 5 (pitch decks)."

## Live state

Checked 2026-10-06.

| What | Where | Notes |
|---|---|---|
| Main checkout | `/Users/wanghui/coding/glidepace/fliphopper` (was `…/glidebox` until 2026-10-06), branch `main` | Holds the merged app code (A01-01, A01-02); no dependencies installed and nothing built here (build in a card's worktree) |
| Git remote | `origin` = `https://github.com/mypals-ml/fliphopper.git` (repository renamed from `glidebox` 2026-10-06, A00-17; GitHub redirects the old address) | Agents push after every commit (owner, 2026-10-04); confirm with `git status -sb` that the push landed |
| Worktrees | none besides the main checkout (`a01-01` and `a01-02` removed after merging, 2026-10-06; their branches remain on GitHub) | The unused `glidebox-local-cloud` worktree and its branch were removed by the owner on 2026-10-04 |
| Servers / hosted resources | none | — |

## Pitfalls

Lessons that change what you do.

- **Reference projects are read-only** and some live outside this repo ([REFERENCES.md](./REFERENCES.md)). The SSR project changes daily; re-read a file before quoting a count or rule from it, and cite the date read.
- **The SSR board's rules evolved on 2026-09-28:** status now lives only on each card and the board is generated (`check-board.mjs`, pre-commit hook). Earlier notes in [analysis/ssr-task-board.md](./analysis/ssr-task-board.md) describe the 2026-09-27 state.
- **Do not grow one card into a catch-all.** On 10-01 to 10-03, six different outcomes were added to `Task-A00-01`; on 10-04 they had to be split back out. Open a new card for each outcome ([AGENTS.md § Working procedure](../AGENTS.md#working-procedure)).
- **A handoff updates this file and the board** — never a new "handoff" document.
- **zsh expands a word starting with `=`** (e.g. `echo =====`) as a command lookup and fails; quote such strings in shell commands.

## Current truth

Decisions and boundaries the board does not carry. If a bullet contradicts the board, the board is right — correct the bullet.

- **Git identity** (repo-local config): `will.w.hui <willwhui@gmail.com>`. On 2026-10-04 the owner briefly asked for `willwhui`, then chose to keep `will.w.hui`. The remote uses HTTPS. SSH to GitHub authenticates as `wanghui2sea`.
- **Decisions:**
  - Accepted: [0007](./decisions/0007-git-like-project-sync.md) **readable text files tracked by Git are the source of truth** (supersedes [0001](./decisions/0001-database-source-of-truth.md)); [0002](./decisions/0002-burndown-counts-plus-sizes.md) burndown by counts plus optional sizes; [0008](./decisions/0008-build-our-own.md) **build our own**, no open-source base.
  - Accepted in part or as requirements: [0004](./decisions/0004-team-accounts-and-roles.md) roles, owner/admin invite by email; [0005](./decisions/0005-start-and-open-agent-sessions.md) start/open agent sessions; [0006](./decisions/0006-local-and-cloud-projects.md) local-only and optional cloud projects; [0009](./decisions/0009-desktop-hub-with-web-and-mobile-companions.md) **desktop hub**. Web and mobile companions view progress and status and act through the desktop app. They connect from outside at free or very low cost, and pair by email and code. Cloud projects are still needed. [0010](./decisions/0010-project-data-folder.md) **user-chosen seabag**, standalone or inside an existing project folder. It is self-contained, uses the host repository's Git, and allows one writer. Every Git action runs the `git` command. Seabag default on disk: `.fliphopper/seabag/`. Iteration 2's Windows checks run on a workmate's machine. [0011](./decisions/0011-desktop-framework-and-shared-code.md) desktop for **macOS and Windows**; desktop window, browser and phone share **one interface code base** built with **Tauri 2** (project rules in Rust crates; interface TypeScript). Released together, unsigned until the first public release.
  - Open: [0003](./decisions/0003-relationship-to-glidelines.md) reuse of the owner's Glidelines code.
  - Proposed: 0007's sync and merge details (layout, push/pull experience, merge rules, offline claims).
- **Name** ([0013](./decisions/0013-product-name-fliphopper.md), 2026-10-06): the product is **FlipHopper** (`fliphopper`, command `fh`); a project's data is its **seabag**, always in `.fliphopper/seabag/`. Dated history keeps "glidebox".
- **Business model** ([0012](./decisions/0012-free-local-app-paid-cloud.md), 2026-10-04): the local app is free, with no account; payment is only for cloud features. Free users must cost (almost) nothing to serve. The source licence is undecided.
- **Until FlipHopper can track itself** (from the end of Iteration 1, decided by the owner 2026-10-04), this project's cards are Markdown in this repo; see [AI-WORKFLOW § dogfooding](./workflow/AI-WORKFLOW.md#after-fliphopper-can-track-itself-dogfooding).
- **Credentials:** none are used by this project yet. When one is introduced, record its *location* here (never its value) and ignore its file in `.gitignore`.

## Do not assume

- That a decision marked *Proposed* or *Open* is approved — in particular 0003 (Glidelines reuse) and 0007's sync and merge details. Only 0007's storage foundation is accepted.
- That a database is a source of truth — any database is a rebuildable index outside the seabag (0007).
- That companions change project data directly — they send requests to the desktop app, which is the only authority (R77).
- That an open-source tracker may be forked or its code copied — 0008 rules that out.
- That interface, generated-project or packaging details marked *proposed* in [INTERFACES](./product/INTERFACES.md) and [AI-GENERATED-PROJECTS](./product/AI-GENERATED-PROJECTS.md) are accepted scope.
- That the SSR board's current rules or counts match what [the analysis](./analysis/ssr-task-board.md) recorded — it is a dated snapshot.
- That competitor table cells marked `?` mean "absent" — they mean "not established".
- That a remote exists or a push landed — check `git remote -v` and the remote itself.
