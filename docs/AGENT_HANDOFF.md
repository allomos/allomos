# AGENT_HANDOFF

Updated: 2026-10-04

**What this file is:** restart mechanics for the next session — which worktree, which server, which pitfall. It is **not** status: status lives in the taskbag (`alm list`; [CARDS.md § Where status lives](./workflow/CARDS.md#where-status-lives)). Contents and timing rules: [workflow/AI-WORKFLOW.md § Handoff](./workflow/AI-WORKFLOW.md#handoff-timing-and-contents).

## Read order to resume

1. [AGENTS.md](../AGENTS.md) (loaded automatically).
2. [Board](./TASK_BOARD.md): Now → Waiting on the owner; then `alm list --state in_progress` → your card file.
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

One block per card that `alm list --state in_progress` shows held.

### Task-A03-06 — claude/a03-06-handoffs
- Checkout: `.claude/worktrees/a03-06-handoffs` · branch `claude/a03-06-handoffs` · pushed: yes
- Approvals in force: the card's local work (owner: "proceed with a03-06"). Not approved yet: upgrading Allomos's own taskbag to `allomos/2`, retiring `docs/AGENT_HANDOFF.md` (both asked).
- Uncommitted: none
- Running: none (the owner's A01-25 app may still hold port 1420; stop it before starting this one)
- Unrecorded findings: none; trial data in `~/allomos-tests/A03-06-handoffs/` (`bakery`: gates 5%/60%, codex 50%, two handoffs; `old-format`: allomos/1 with a v1 section).
- Next action: owner checks the app (Handoffs tab, task pane, settings, form, upgrade notice); then merge, reinstall `alm`, and the two asked steps.

## Live state

Checked 2026-10-08.

| What | Where | Notes |
|---|---|---|
| Main checkout | `/Users/wanghui/coding/glidepace/allomos` (was `…/glidebox` until 2026-10-06, `…/fliphopper` until 2026-10-11), branch `main` | Holds the merged app code (A01-01, A01-02); no dependencies installed and nothing built here (build in a card's worktree) |
| Git remote | `origin` = `https://github.com/mypals-ml/allomos.git` (repository renamed from `glidebox` 2026-10-06, A00-17, and from `fliphopper` 2026-10-11, A00-21; GitHub redirects the old address) | Agents push after every commit (owner, 2026-10-04); confirm with `git status -sb` that the push landed |
| Worktrees | `alm` in `~/.cargo/bin` was reinstalled from it on 2026-10-08: older `alm` or app builds refuse Allomos's own taskbag now that tasks have an `iteration` field (`field_unknown`), so reinstall with `cargo install --locked --path crates/cli` in any older checkout. `.claude/worktrees/a01-run`, branch `claude/a01-run`: the 2026-10-06/08 card run (A01-04…A01-14, A00-19 local part, A03-01, A03-03…A03-05, A04-01), all merged into `main`; nothing held. Its `target/debug/bundle/macos/Allomos.app` is the build the owner checks (rebuild there with `npx --prefix apps/desktop tauri build --debug --bundles app`). Remove it once A03-05 is checked and no newer build is needed (CARDS.md step 8). Older worktrees removed after merging |
| Servers / hosted resources | none | — |
| `alm` on this Mac | `~/.cargo/bin/fh`, installed from the a01-run build (includes A01-08, A01-12 `--push`, A01-14) | Agent shells may not have `~/.cargo/bin` on PATH: `export PATH="$HOME/.cargo/bin:$PATH"`. Reinstall after `alm` changes: `cargo install --path crates/cli --force` |

## Pitfalls

Lessons that change what you do.

- **Reference projects are read-only** and some live outside this repo ([REFERENCES.md](./REFERENCES.md)). The SSR project changes daily; re-read a file before quoting a count or rule from it, and cite the date read.
- **The SSR board's rules evolved on 2026-09-28:** status now lives only on each card and the board is generated (`check-board.mjs`, pre-commit hook). Earlier notes in [analysis/ssr-task-board.md](./analysis/ssr-task-board.md) describe the 2026-09-27 state.
- **Do not grow one card into a catch-all.** On 10-01 to 10-03, six different outcomes were added to `Task-A00-01`; on 10-04 they had to be split back out. Open a new card for each outcome ([AGENTS.md § Working procedure](../AGENTS.md#working-procedure)).
- **A handoff updates this file and the taskbag (with `alm`)** — never a new "handoff" document.
- **Use `--push` on every `alm` status change** (A01-12): it commits only the taskbag and pushes; check for a `commit_failed` / `push_failed` warning.
- **The Codex CLI on this Mac is broken** (`/opt/homebrew/bin/codex` 0.130.0: its platform binary is missing). The app does not offer it; reinstalling is the owner's call.
- **The taskbag is outside this repository** (since 2026-10-12). Branches made before then hold a stale copy at `.allomos/taskbag/`, which `alm` finds by default; set `ALLOMOS_TASKBAG=/Users/wanghui/coding/glidepace/allomos-taskbag/.allomos/taskbag` first ([CARDS.md](./workflow/CARDS.md#where-status-lives)).
- **zsh expands a word starting with `=`** (e.g. `echo =====`) as a command lookup and fails; quote such strings in shell commands.

## Current truth

Decisions and boundaries the board does not carry. If a bullet contradicts the taskbag or the board, they are right — correct the bullet.

- **Git identity** (repo-local config): `will.w.hui <willwhui@gmail.com>`. On 2026-10-04 the owner briefly asked for `willwhui`, then chose to keep `will.w.hui`. The remote uses HTTPS. SSH to GitHub authenticates as `wanghui2sea`.
- **Decisions:**
  - Accepted: [0007](./decisions/0007-git-like-project-sync.md) **readable text files tracked by Git are the source of truth** (supersedes [0001](./decisions/0001-database-source-of-truth.md)); [0002](./decisions/0002-burndown-counts-plus-sizes.md) burndown by counts plus optional sizes; [0008](./decisions/0008-build-our-own.md) **build our own**, no open-source base; [0003](./decisions/0003-relationship-to-glidelines.md) **Allomos is a standalone product** (2026-10-06): Glidelines is read for ideas only, no shared code.
  - Accepted in part or as requirements: [0004](./decisions/0004-team-accounts-and-roles.md) roles, owner/admin invite by email; [0005](./decisions/0005-start-and-open-agent-sessions.md) start/open agent sessions; [0006](./decisions/0006-local-and-cloud-projects.md) local-only and optional cloud projects; [0009](./decisions/0009-desktop-hub-with-web-and-mobile-companions.md) **desktop hub**. Web and mobile companions view progress and status and act through the desktop app. They connect from outside at free or very low cost, and pair by email and code. Cloud projects are still needed. [0010](./decisions/0010-project-data-folder.md) **user-chosen taskbag**, standalone or inside an existing project folder. It is self-contained, uses the host repository's Git, and allows one writer. Every Git action runs the `git` command. Taskbag default on disk: `.allomos/taskbag/`. Iteration 2's Windows checks run on a workmate's machine. [0011](./decisions/0011-desktop-framework-and-shared-code.md) desktop for **macOS and Windows**; desktop window, browser and phone share **one interface code base** built with **Tauri 2** (project rules in Rust crates; interface TypeScript). Released together, unsigned until the first public release.
  - Proposed: 0007's sync and merge details (layout, push/pull experience, merge rules, offline claims).
- **Main purpose** ([0014](./decisions/0014-main-purpose-and-three-sets.md), 2026-10-06): switch between AI agents and teammates and keep the project; sets 2 and 3 are AI skills and progress management.
- **Name** ([0015](./decisions/0015-product-name-allomos.md), 2026-10-10; before that [0013](./decisions/0013-product-name-fliphopper.md)): the product is **Allomos** (换肩, アロモス; `allomos`, command `alm`); a project's data is its **taskbag**, always in `.allomos/taskbag/`. Dated history keeps "FlipHopper" and "glidebox".
- **Business model** ([0012](./decisions/0012-free-local-app-paid-cloud.md), 2026-10-04): the local app is free, with no account; payment is only for cloud features. Free users must cost (almost) nothing to serve. The source licence is undecided.
- **Allomos tracks itself** (since 2026-10-06, [`Task-A01-07`](./tasks/Task-A01-07-own-cards-in-seabag.md)): card status is in the taskbag at `../allomos-taskbag/.allomos/taskbag` (moved out of this repository on 2026-10-12, [report](./allomos-conversion.md)), its own private repository `mypals-ml/allomos-taskbag` on GitHub since 2026-10-12 ([`Task-A01-24`](./tasks/Task-A01-24-taskbag-on-github.md)), changed only with `alm … --push`. `alm` must be on your PATH: `cargo install --path crates/cli` from any checkout. See [CARDS.md § Where status lives](./workflow/CARDS.md#where-status-lives).
- **Credentials:** none are used by this project yet. When one is introduced, record its *location* here (never its value) and ignore its file in `.gitignore`.

## Do not assume

- That a decision marked *Proposed* or *Open* is approved — in particular 0007's sync and merge details. Only 0007's storage foundation is accepted.
- That a database is a source of truth — any database is a rebuildable index outside the taskbag (0007).
- That companions change project data directly — they send requests to the desktop app, which is the only authority (R77).
- That an open-source tracker may be forked or its code copied — 0008 rules that out.
- That interface, generated-project or packaging details marked *proposed* in [INTERFACES](./product/INTERFACES.md) and [AI-GENERATED-PROJECTS](./product/AI-GENERATED-PROJECTS.md) are accepted scope.
- That the SSR board's current rules or counts match what [the analysis](./analysis/ssr-task-board.md) recorded — it is a dated snapshot.
- That competitor table cells marked `?` mean "absent" — they mean "not established".
- That a remote exists or a push landed — check `git remote -v` and the remote itself.
