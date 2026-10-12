# Moving Allomos's tasks into the new taskbag (2026-10-12)

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
| [Task-A00-01-planning-docs.md](tasks/Task-A00-01-planning-docs.md) | `A00-01` | done · owner verified |
| [Task-A00-02-github-remote.md](tasks/Task-A00-02-github-remote.md) | `A00-02` | done |
| [Task-A00-03-glidelines-relationship.md](tasks/Task-A00-03-glidelines-relationship.md) | `A00-03` | done · owner verified |
| [Task-A00-04-local-and-cloud-requirements.md](tasks/Task-A00-04-local-and-cloud-requirements.md) | `A00-04` | done · owner verified |
| [Task-A00-05-competitor-research.md](tasks/Task-A00-05-competitor-research.md) | `A00-05` | done |
| [Task-A00-06-git-sync-proposal.md](tasks/Task-A00-06-git-sync-proposal.md) | `A00-06` | done · owner verified |
| [Task-A00-07-open-source-base-decision.md](tasks/Task-A00-07-open-source-base-decision.md) | `A00-07` | done · owner verified |
| [Task-A00-08-ai-generated-project-setup.md](tasks/Task-A00-08-ai-generated-project-setup.md) | `A00-08` | done · owner verified |
| [Task-A00-09-interface-requirements.md](tasks/Task-A00-09-interface-requirements.md) | `A00-09` | done · owner verified |
| [Task-A00-10-docs-consistency-review.md](tasks/Task-A00-10-docs-consistency-review.md) | `A00-10` | done · owner verified |
| [Task-A00-11-project-data-folder.md](tasks/Task-A00-11-project-data-folder.md) | `A00-11` | waiting · drafted |
| [Task-A00-12-desktop-platforms-and-shared-code.md](tasks/Task-A00-12-desktop-platforms-and-shared-code.md) | `A00-12` | done · owner verified |
| [Task-A00-13-iterative-release-roadmap.md](tasks/Task-A00-13-iterative-release-roadmap.md) | `A00-13` | done · owner verified |
| [Task-A00-14-areas-and-naming.md](tasks/Task-A00-14-areas-and-naming.md) | `A00-14` | waiting · drafted |
| [Task-A00-15-free-local-app.md](tasks/Task-A00-15-free-local-app.md) | `A00-15` | waiting · drafted |
| [Task-A00-16-road-show-deck.md](tasks/Task-A00-16-road-show-deck.md) | `A00-16` | done |
| [Task-A00-17-rename-to-crossjump.md](tasks/Task-A00-17-rename-to-crossjump.md) | `A00-17` | done · owner verified |
| [Task-A00-18-positioning-and-deck.md](tasks/Task-A00-18-positioning-and-deck.md) | `A00-18` | done · owner verified |
| [Task-A00-19-decks-new-mark.md](tasks/Task-A00-19-decks-new-mark.md) | `A00-19` | done · owner verified · claude/a00-19-republish |
| [Task-A00-20-app-name-research.md](tasks/Task-A00-20-app-name-research.md) | `A00-20` | done · owner verified |
| [Task-A00-21-rename-to-allomos.md](tasks/Task-A00-21-rename-to-allomos.md) | `A00-21` | done · owner verified · claude/a00-21-allomos |
| [Task-A00-22-leftover-fliphopper-names.md](tasks/Task-A00-22-leftover-fliphopper-names.md) | `A00-22` | done · owner verified · claude/a00-22-leftover-names |
| [Task-A00-23-name-clearance.md](tasks/Task-A00-23-name-clearance.md) | `A00-23` | waiting |
| [Task-A00-24-decks-allomos-taskbag.md](tasks/Task-A00-24-decks-allomos-taskbag.md) | `A00-24` | in progress · deployed and verified · claude/a00-24-decks |
| [Task-A01-01-walking-skeleton.md](tasks/Task-A01-01-walking-skeleton.md) | `A01-01` | done · owner verified |
| [Task-A01-02-ui-languages.md](tasks/Task-A01-02-ui-languages.md) | `A01-02` | done · owner verified |
| [Task-A01-03-gb-command.md](tasks/Task-A01-03-gb-command.md) | `A01-03` | done · owner verified |
| [Task-A01-04-startup-folder-argument.md](tasks/Task-A01-04-startup-folder-argument.md) | `A01-04` | done · owner verified |
| [Task-A01-05-readme-in-new-folder.md](tasks/Task-A01-05-readme-in-new-folder.md) | `A01-05` | done · owner verified |
| [Task-A01-06-show-seabag-in-file-manager.md](tasks/Task-A01-06-show-seabag-in-file-manager.md) | `A01-06` | done · owner verified |
| [Task-A01-07-own-cards-in-seabag.md](tasks/Task-A01-07-own-cards-in-seabag.md) | `A01-07` | done · owner verified |
| [Task-A01-08-fh-fills-move-gaps.md](tasks/Task-A01-08-fh-fills-move-gaps.md) | `A01-08` | done · owner verified |
| [Task-A01-09-v0-1-exit-check.md](tasks/Task-A01-09-v0-1-exit-check.md) | `A01-09` | done · owner verified |
| [Task-A01-10-file-new-window.md](tasks/Task-A01-10-file-new-window.md) | `A01-10` | done · owner verified |
| [Task-A01-11-recent-folders-menu.md](tasks/Task-A01-11-recent-folders-menu.md) | `A01-11` | done · owner verified |
| [Task-A01-12-fh-commit-option.md](tasks/Task-A01-12-fh-commit-option.md) | `A01-12` | done · owner verified |
| [Task-A01-13-agent-instructions-on-init.md](tasks/Task-A01-13-agent-instructions-on-init.md) | `A01-13` | done · owner verified |
| [Task-A01-14-init-asks-the-agent.md](tasks/Task-A01-14-init-asks-the-agent.md) | `A01-14` | done · owner verified |
| [Task-A01-15-iterations-view.md](tasks/Task-A01-15-iterations-view.md) | `A01-15` | done · owner verified · claude/a01-15-iterations |
| [Task-A01-16-task-pane.md](tasks/Task-A01-16-task-pane.md) | `A01-16` | done · owner verified · claude/a01-16-task-pane |
| [Task-A01-17-change-history.md](tasks/Task-A01-17-change-history.md) | `A01-17` | backlog |
| [Task-A01-18-edit-task-in-app.md](tasks/Task-A01-18-edit-task-in-app.md) | `A01-18` | backlog |
| [Task-A01-19-create-in-app.md](tasks/Task-A01-19-create-in-app.md) | `A01-19` | backlog |
| [Task-A01-20-rename-seabag-to-taskcase.md](tasks/Task-A01-20-rename-seabag-to-taskcase.md) | `A01-20` | done · owner verified |
| [Task-A01-21-task-language.md](tasks/Task-A01-21-task-language.md) | `A01-21` | done · owner verified · claude/a01-21-task-language |
| [Task-A02-01-burndown-and-forecast.md](tasks/Task-A02-01-burndown-and-forecast.md) | `A02-01` | backlog |
| [Task-A03-01-agent-launch-research.md](tasks/Task-A03-01-agent-launch-research.md) | `A03-01` | waiting · drafted |
| [Task-A03-02-account-quota-idea.md](tasks/Task-A03-02-account-quota-idea.md) | `A03-02` | backlog |
| [Task-A03-03-app-asks-agents-to-set-up.md](tasks/Task-A03-03-app-asks-agents-to-set-up.md) | `A03-03` | done · owner verified |
| [Task-A03-04-agent-awareness-line.md](tasks/Task-A03-04-agent-awareness-line.md) | `A03-04` | done · owner verified |
| [Task-A03-05-agents-panel-refinements.md](tasks/Task-A03-05-agents-panel-refinements.md) | `A03-05` | done · owner verified |
| [Task-A03-06-agent-handoff-rules.md](tasks/Task-A03-06-agent-handoff-rules.md) | `A03-06` | backlog |
| [Task-A03-07-renew-agent-instructions.md](tasks/Task-A03-07-renew-agent-instructions.md) | `A03-07` | ready |
| [Task-A03-08-step-commits.md](tasks/Task-A03-08-step-commits.md) | `A03-08` | done · owner verified · claude/a03-08-step-commits |
| [Task-A04-01-remote-access-research.md](tasks/Task-A04-01-remote-access-research.md) | `A04-01` | waiting · drafted |
| [Task-A05-01-ssr-board-import.md](tasks/Task-A05-01-ssr-board-import.md) | `A05-01` | dropped |
| [Task-A05-02-convert-existing-project.md](tasks/Task-A05-02-convert-existing-project.md) | `A05-02` | done · owner verified · claude/a05-02-convert-project |
| [Task-A05-03-taskbag-setup-wizard.md](tasks/Task-A05-03-taskbag-setup-wizard.md) | `A05-03` | in progress · implemented · claude/a05-03-setup-wizard |
| [Task-A07-01-app-icon-hopper.md](tasks/Task-A07-01-app-icon-hopper.md) | `A07-01` | done · owner verified |
| [Task-A07-02-build-pipeline.md](tasks/Task-A07-02-build-pipeline.md) | `A07-02` | done · owner verified · claude/a07-02-script-run |
| [Task-A07-03-windows-works.md](tasks/Task-A07-03-windows-works.md) | `A07-03` | backlog |
| [Task-A07-04-v0-2-exit-check.md](tasks/Task-A07-04-v0-2-exit-check.md) | `A07-04` | backlog |
| [Task-A07-05-windows-agents-no-flash.md](tasks/Task-A07-05-windows-agents-no-flash.md) | `A07-05` | done · owner verified · claude/a07-05-windows-agents |
| [Task-A07-06-windows-installer-installs-alm.md](tasks/Task-A07-06-windows-installer-installs-alm.md) | `A07-06` | done · owner verified · claude/a07-06-installer-alm |

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
