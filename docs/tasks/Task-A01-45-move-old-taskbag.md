# Task-A01-45-move-old-taskbag

*Card type: coding. Area A01 (Tracking), Iteration 2. Written 2026-10-16 from the task's description in the taskbag (`alm show A01-45`, the approver's decisions of 2026-10-15). Replaces the guided migration of [`A01-32`](./Task-A01-32-guided-migration.md) and the prompt fix of [`A01-34`](./Task-A01-34-migrate-refused.md); extends [`A00-21`](./Task-A00-21-rename-to-allomos.md) (old FlipHopper folders are moved in by an agent) to every older Allomos format.*

## What gets built (plain words)

A **taskbag** is the folder of small files where a project's tasks live (`.allomos/taskbag/`). Its **format** (`allomos/1`, `/2`, `/3`, today `/4`) says which files and fields it has. Until now, a taskbag in an older format was either "too old" (formats 1 and 2) or locked for work while an AI agent followed per-version **migration entries** (`alm migrate`, A01-32).

After this card, **no format is too old**. A taskbag from any older Allomos is moved into a new taskbag by an AI agent, the same way a project that tracks its tasks in another system is moved in ([`A05-02`](./Task-A05-02-convert-existing-project.md)):

1. The app sees the older taskbag and shows a notice with **Move the tasks in…** (and the setup wizard's step 2 shows the same button instead of **Next**).
2. Clicking it **renames the old folder aside**, keeping the old format in its name (`.allomos/taskbag/` → `.allomos/taskbag-allomos-3/`), and **creates a new, empty taskbag** in its place with the old project's ID, name and project folder. Nothing inside the old folder changes.
3. The Connect step opens; the chosen agent gets the **move-in request**: the conversion request's steps, told that the tasks to move are the old folder's, with everything in it: tasks, history, handoff notes, project notes.
4. The agent reads the old files directly, writes the new taskbag only with `alm`, shows its whole plan once with `alm confirm`, and **removes the old folder only as part of that confirmed plan**.

`alm` on an older taskbag reads and changes nothing: every command stops with `taskbag_to_move_in`, saying to move it in from the app, or with the request `alm move-in` prints.

| Piece | What it is | Where |
|---|---|---|
| Older means "to move in" | Any `allomos/<n>` below this Allomos's is not read: one problem, `taskbag_to_move_in` (found, current). `OLDEST_FORMAT_VERSION`, `MIGRATIONS` and the 3 → 4 entry are gone. | `crates/core/src/{model,validate,problem}.rs`, `crates/store/src/versions.rs` |
| Rename aside | `move_aside`: old folder → `<name>-allomos-<n>` (then `-2`, `-3` …), new taskbag with the old ID, name and project folder; renamed back if the new one cannot be made. `move_in` says whether tasks are still to move in (the taskbag itself, or an old folder beside a new taskbag with no tasks yet). | `crates/store/src/versions.rs` |
| The move-in request | `agent_move_in_request_in` (`MOVE_IN_REQUEST_VERSION` 1), in en, zh-CN, ja: a title, what the old folder is (renamed by the app, or to be renamed by the agent after Go ahead), what to move from each part of it, and the changes to the conversion steps (no early commit; the old folder is the old status to remove; remove it only when Git history holds it). | `crates/store/src/convert.rs` |
| `alm` | `alm move-in [--language]` prints the request and changes nothing; every other command on an older taskbag (except `init`, `brief`, `guide`, `confirm`) is refused. `alm migrate`, `--finish`, the "being updated" lock and its Git handling are gone. | `crates/cli` |
| The app | Notice and button **Move the tasks in…**; wizard step 2 for an older taskbag; Connect panel titled **Move the tasks in**; desktop command `move_taskbag_aside`; the Connect request becomes the move-in request while tasks are to move in. The section-only notice's button is now **Update the instructions…**. | `apps/desktop/src-tauri/src/lib.rs`, `apps/web/src/{App,SetupWizard,AgentsPanel}.tsx`, `versionFlow.ts`, `messenger.ts`, `types.ts`, `i18n/*` |
| Old note name | `finished_without_owner` (the `allomos/3` name of `without_human_check`) is no longer read, and `finished_without_owner_renamed` is gone: only the guided migration needed them. Old events naming it still show in history. | `crates/core`, `crates/cli`, `apps/web` |

## Details for the worker

- **Outcome.** A taskbag written by any older Allomos is moved into a new one by an AI agent, like another task system; nothing is too old to open.
- **Files owned.** `crates/core/src/{lib,model,validate,problem,ops,event,decision}.rs`, `crates/store/src/{versions,convert,setup,guide,lib}.rs`, `crates/store/tests/edits.rs`, `crates/cli/src/lib.rs`, `crates/cli/tests/alm.rs`, `apps/desktop/src-tauri/src/lib.rs`, `apps/web/src/{App.tsx,SetupWizard.tsx,AgentsPanel.tsx,versionFlow.ts,stopFlow.ts,messenger.ts,types.ts}`, `apps/web/src/i18n/*`, `apps/web/tests/{versionFlow,stopFlow}.test.ts`, `docs/product/{FILE-FORMAT,OPEN-FOLDER-FLOW}.md`, `docs/workflow/{CODING_RULES,CARDS}.md`, this card.
- **External operations.** None.
- **Done means mechanically.** The named checks in [CODING_RULES](../workflow/CODING_RULES.md#checks-before-a-commit) pass; tests cover: every older format refused with `taskbag_to_move_in` and nothing written (core, store, `alm`); rename aside keeps the old folder byte for byte and makes a new taskbag with the old ID, name and project folder; a taken aside name gets a number; asking twice changes nothing; the move is offered until the first task is moved in; the request in three languages, prepared and not; the app's notice states; a manual trial with an `allomos/1` and an `allomos/3` taskbag.

## Decisions

- Decided: `alm rename label` stays. It is a general edit of the ladder (like `alm rename area`), not tied to the migration, and the move-in request does not need it.
- Decided: the command that prints the request is `alm move-in [--language]`; it changes nothing (like `alm migrate` did without `--finish`).
- Decided: one problem code for an older taskbag, `taskbag_to_move_in` (found, current), replacing `taskbag_older`, `taskbag_too_old`, `migration_missing` and `migration_unfinished`; plus `nothing_to_move_in` when Move the tasks in… finds nothing to move.
- Decided: on an older taskbag every `alm` command is refused, reading ones too (only `init`, `brief`, `guide`, `confirm` and `move-in` run), since the old format is not read at all.
- Decided: the new taskbag keeps the old project's ID, name and recorded project folder; its ladder is the default, and the agent sets the old ladder with `alm labels` as part of the plan (renaming a top step "owner verified" to "human verified").
- Decided: the aside name is `<the folder's name>-allomos-<n>`, then `-2`, `-3` … when taken.
- Decided: while an old folder sits beside a new taskbag that has no tasks yet, the app keeps offering Move the tasks in… (it opens the Connect step again), and `alm move-in` prints the request for that folder.
- Decided: the old history (`events/`) is not copied (`alm` records only its own changes); it stays in Git history, and the report and `project-notes.md` name the last commit holding the old taskbag. The agent removes the old folder only when Git history holds it.
- Decided: the move-in request is the conversion request's steps (A05-02) with a new title and two paragraphs in front (what the old folder is; what to move and the changes to the steps), versioned separately (`MOVE_IN_REQUEST_VERSION` 1), so the conversion request (v8) does not change.
- Decided: clicking Move the tasks in… renames at once, without a second question: the notice says what happens, the rename is undone by renaming back, and the agent's plan is still confirmed with `alm confirm`.
- Decided: the section-only notice's button, **Migrate the taskbag…** until now, reads **Update the instructions…** (zh-CN 更新说明…, ja 説明を更新…), since it no longer touches the taskbag.
- Decided: `finished_without_owner` and `finished_without_owner_renamed` are removed with the migration that needed them.
- Decided: `alm brief`'s text did not change, so the section stays version 10; this repository's `AGENTS.md` section already matches `alm brief`.

## Notes

- 2026-10-16 (claude/a01-45-move-old-taskbag): built and checked; evidence below.
- Evidence, 2026-10-16:
  - `cargo fmt --check` PASS; `cargo clippy --workspace --all-targets -- -D warnings` PASS; `cargo test --workspace` PASS (191 passed, 1 ignored; new: `an_older_or_newer_format_is_not_read`, `an_older_taskbag_is_never_read_or_changed`, `moving_aside_keeps_the_old_folder_and_makes_a_new_taskbag`, `an_aside_name_already_taken_gets_a_number`, `move_in_request_names_the_old_folder_and_what_it_holds`, `an_older_taskbag_is_not_changed`, `every_older_taskbag_is_to_be_moved_in`, `move_in_names_the_old_folder_beside_a_new_taskbag`, `move_in_on_a_current_taskbag_changes_nothing`, `an_older_taskbag_gets_the_move_in_request`); `npm --prefix apps/web run typecheck` PASS; `build` PASS; `check:i18n` PASS (441 texts, 68 codes); `npm --prefix apps/web test` PASS (95).
  - Manual trial in `~/allomos-tests/a01-45-old-taskbag/` (Git repositories with a committed `allomos/1` and `allomos/3` taskbag: three tasks, one waiting, one done without the human check, a handoff and project notes on `/3`, ladder topped by "owner verified"): `alm list` and `alm claim` exit 1 with `taskbag_to_move_in` and `git status` stays clean; `alm move-in` prints the request (not prepared: rename to `.allomos/taskbag-allomos-3` and `alm init` after Go ahead), still nothing changed. The app's rename (`versions::move_aside`, the function behind `move_taskbag_aside`, run from a scratch program) renamed `.allomos/taskbag` to `taskbag-allomos-1` / `-3`, identical to before (`diff -r`), and created a new empty taskbag (`alm list`: "No tasks."; `alm project`: "Bakery 3 (bakery3)"); `alm move-in` then printed the prepared request naming the old folder; a second rename changed nothing.
  - Not run: a live agent following the move-in request through the app's `alm confirm` popup (it needs a person to press Go ahead), and the app's notice and wizard on screen. Untouched copies for that check: `bakery-allomos-1-app`, `bakery-allomos-3-app`.
