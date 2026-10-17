# Moving Allomos's tasks into the new taskbag (2026-10-12)

*Since 2026-10-17 (`A00-30`) each card file lives in the taskbag beside its task, as `tasks/<id>/card.md`, and the description's link is `Card: [card.md](card.md)`; the card links below are history.*

**What happened, in plain words:** until today this project's tasks lived in a taskbag inside this repository (`.allomos/taskbag/`). They were copied, with their exact status, into the taskbag Allomos set up next to this repository, `../allomos-taskbag/.allomos/taskbag` (its own Git repository, branch `master`). The old in-repository taskbag was then removed, so status has one home again. Every task was written with the `alm` command; no taskbag file was edited by hand. The owner approved the plan in the Allomos app (`alm confirm`, "Move tasks into Allomos").

## What was copied

- **8 areas** (A00–A07, same names and order) and **15 iterations** (I01–I15, same names and releases).
- **64 tasks**, one per card file below. Each task's state, label, holder, waiting-on, next dependency, iteration and handoff note were compared field by field with the old `task.json` after copying: **0 differences**.
- **The label ladder:** drafted → implemented → locally verified → deployed and verified → owner verified (the new taskbag lacked "drafted"; set with `alm labels`).
- **4 new backlog tasks** for planned work that has no card yet ([TASK_BOARD § Roadmap — not yet carded](./TASK_BOARD.md#roadmap--not-yet-carded)). Their description says "No card yet": `A01-22` Desktop hub (W04), `A01-23` Shared commands and the human CLI (R60–R68), `A04-02` Companion viewing (W10, R69–R71), `A05-04` AI-generated project setup (W09, R50–R55). They are unplanned (no iteration), because the board names none.
- Each task's description links to its card: `Card: [docs/tasks/<file>](../../../../../allomos/docs/tasks/<file>)`.

Nothing was dropped or duplicated: 64 card files, 64 copied tasks, 64 old tasks.

## Card files and the tasks they became

| Card file | Task | Status copied (state · label · holder) |
|---|---|---|
| task `A00-01` | `A00-01` | done · owner verified |
| task `A00-02` | `A00-02` | done |
| task `A00-03` | `A00-03` | done · owner verified |
| task `A00-04` | `A00-04` | done · owner verified |
| task `A00-05` | `A00-05` | done |
| task `A00-06` | `A00-06` | done · owner verified |
| task `A00-07` | `A00-07` | done · owner verified |
| task `A00-08` | `A00-08` | done · owner verified |
| task `A00-09` | `A00-09` | done · owner verified |
| task `A00-10` | `A00-10` | done · owner verified |
| task `A00-11` | `A00-11` | waiting · drafted |
| task `A00-12` | `A00-12` | done · owner verified |
| task `A00-13` | `A00-13` | done · owner verified |
| task `A00-14` | `A00-14` | waiting · drafted |
| task `A00-15` | `A00-15` | waiting · drafted |
| task `A00-16` | `A00-16` | done |
| task `A00-17` | `A00-17` | done · owner verified |
| task `A00-18` | `A00-18` | done · owner verified |
| task `A00-19` | `A00-19` | done · owner verified · claude/a00-19-republish |
| task `A00-20` | `A00-20` | done · owner verified |
| task `A00-21` | `A00-21` | done · owner verified · claude/a00-21-allomos |
| task `A00-22` | `A00-22` | done · owner verified · claude/a00-22-leftover-names |
| task `A00-23` | `A00-23` | waiting |
| task `A00-24` | `A00-24` | in progress · deployed and verified · claude/a00-24-decks |
| task `A01-01` | `A01-01` | done · owner verified |
| task `A01-02` | `A01-02` | done · owner verified |
| task `A01-03` | `A01-03` | done · owner verified |
| task `A01-04` | `A01-04` | done · owner verified |
| task `A01-05` | `A01-05` | done · owner verified |
| task `A01-06` | `A01-06` | done · owner verified |
| task `A01-07` | `A01-07` | done · owner verified |
| task `A01-08` | `A01-08` | done · owner verified |
| task `A01-09` | `A01-09` | done · owner verified |
| task `A01-10` | `A01-10` | done · owner verified |
| task `A01-11` | `A01-11` | done · owner verified |
| task `A01-12` | `A01-12` | done · owner verified |
| task `A01-13` | `A01-13` | done · owner verified |
| task `A01-14` | `A01-14` | done · owner verified |
| task `A01-15` | `A01-15` | done · owner verified · claude/a01-15-iterations |
| task `A01-16` | `A01-16` | done · owner verified · claude/a01-16-task-pane |
| task `A01-17` | `A01-17` | backlog |
| task `A01-18` | `A01-18` | backlog |
| task `A01-19` | `A01-19` | backlog |
| task `A01-20` | `A01-20` | done · owner verified |
| task `A01-21` | `A01-21` | done · owner verified · claude/a01-21-task-language |
| task `A02-01` | `A02-01` | backlog |
| task `A03-01` | `A03-01` | waiting · drafted |
| task `A03-02` | `A03-02` | backlog |
| task `A03-03` | `A03-03` | done · owner verified |
| task `A03-04` | `A03-04` | done · owner verified |
| task `A03-05` | `A03-05` | done · owner verified |
| task `A03-06` | `A03-06` | backlog |
| task `A03-07` | `A03-07` | ready |
| task `A03-08` | `A03-08` | done · owner verified · claude/a03-08-step-commits |
| task `A04-01` | `A04-01` | waiting · drafted |
| task `A05-01` | `A05-01` | dropped |
| task `A05-02` | `A05-02` | done · owner verified · claude/a05-02-convert-project |
| task `A05-03` | `A05-03` | in progress · implemented · claude/a05-03-setup-wizard |
| task `A07-01` | `A07-01` | done · owner verified |
| task `A07-02` | `A07-02` | done · owner verified · claude/a07-02-script-run |
| task `A07-03` | `A07-03` | backlog |
| task `A07-04` | `A07-04` | backlog |
| task `A07-05` | `A07-05` | done · owner verified · claude/a07-05-windows-agents |
| task `A07-06` | `A07-06` | done · owner verified · claude/a07-06-installer-alm |

## Values that could not be copied

| Value | Where | Why, and what to do |
|---|---|---|
| Project ID `allomos` and name `Allomos` | `.allomos/taskbag/project.json` lines 3–4 (removed; see Git history) | The new taskbag was created as `allomos-taskbag` / "allomos-taskbag", and `alm` has no command to rename a project. Rename it in the Allomos app if wanted. |
| Task language `en` (fixed) | `.allomos/taskbag/project.json` line 13 | The new taskbag keeps `ui` (follows the app's language; it gives `en` today), as the plan said. To fix it to English: `alm task-language en`. |
| `project_folder` `.` | `.allomos/taskbag/project.json` line 12 | Not a task value; the new taskbag already names `../allomos`. |
| Who changed each task and when | Git history of `.allomos/taskbag/` in this repository (the `alm: …` commits) | Taskbag history cannot be copied; it stays in this repository's history. The new taskbag's history starts at its commit `0d26e76`. |

Done dates, approvals and dependencies were never stored in the old taskbag. They stay in the card files' Approvals and Notes, unchanged.

## Things to know

- **The new taskbag's repository has no remote**, so `alm … --push` commits but warns `push_failed`. Adding a remote is the owner's call.
- **Older branches and worktrees** (`claude/a00-24-decks`, `claude/a05-03-setup-wizard`, `claude/a01-run`, the detached `run` worktree) still contain the old `.allomos/taskbag/`, which `alm` finds by default there. Use `ALLOMOS_TASKBAG=/Users/wanghui/coding/glidepace/allomos-taskbag/.allomos/taskbag`. When such a branch is merged, keep the old taskbag files deleted.
- Decision records [0010](./decisions/0010-project-data-folder.md) and [0015](./decisions/0015-product-name-allomos.md) describe the taskbag's default place; as dated records they were not changed.

## Check

`alm --taskbag ../allomos-taskbag/.allomos/taskbag list --json` reports `"issues": []`.

## Undo

`git revert -m 1 <merge commit>` in this repository brings back the old taskbag and the old rules; `git -C ../allomos-taskbag revert 0d26e76` empties the new taskbag again.
