# Task-A01-18-edit-task-in-app

*Card type: coding. Area A01 (Tracking), Iteration 2. Written 2026-10-09 at the owner's request for the rest of v0.2 ([PLAN § 9](../product/PLAN.md#9-roadmap--releases): "create and edit tasks in the app … recorded as events like CLI changes"). Needs [`A01-17`](./Task-A01-17-change-history.md) first, so app changes are recorded too.*

## What gets built (plain words)

Today the app only shows tasks; every change is made with `alm` in a terminal. After this card, the owner changes a task right in the **task pane** ([`A01-16`](./Task-A01-16-task-pane.md)): its state, label, what it waits on, its next need, its handoff note, its iteration, its title and its description. The app follows exactly the same rules as `alm` (for example, a task someone else holds cannot be labelled), and each change is recorded in the task's history.

| Piece | What it is | Where |
|---|---|---|
| Editing in the pane | Each line of the pane becomes changeable: a choice list for state, label and iteration; a text box for the rest; Save / Cancel. Refusals are shown in plain words, the same as `alm` gives. | `apps/web` |
| The same rules | The app calls the same Rust rules as `alm` (`crates/core` operations), never its own. | `apps/desktop/src-tauri` |
| Taking turns with `alm` | The app takes the taskbag's lock for each change, so it and agents using `alm` never write at the same moment ([FILE-FORMAT § Writing rules](../product/FILE-FORMAT.md)). | `crates/store` |
| Who made the change | The app's changes are recorded under the owner's name (asked once, kept in the app's settings). | `apps/web`, `apps/desktop/src-tauri` |
| Commit and push | Not in this card: the app does not run Git until Iteration 6. A change stays in the files until committed (for example with `alm … --push` or by hand). | — |

**What you can see at the end:** in the app, open a task, change its label and its handoff note, save: the list shows the new label, `alm show` in a terminal shows the same, and History lists the change under your name.

## Details for the worker

- **Outcome.** The owner changes tasks without a terminal, under the same rules agents follow.
- **Decisive unknowns.**
  - Decided (owner, 2026-10-14): all eight fields above are changeable; no claiming or releasing from the app (holders are agent sessions).
  - Decided (owner, 2026-10-14): the owner's name for the history is asked once in the app and kept in its settings.
- **Files owned.** `apps/web/src/TaskPane.tsx`, `apps/web/src/App.tsx`, `apps/web/src/messenger.ts`, `apps/web/src/types.ts`, `apps/web/src/i18n/*`, `apps/desktop/src-tauri/src/lib.rs`, `crates/core/src/ops.rs` (any missing operation, such as setting the title or description), `crates/store`, tests, this card.
- **External operations.** None.
- **Done means mechanically.**
  - The six named checks pass.
  - Tests: every edit goes through the same rule as its `alm` command; a refused edit changes no file; the app waits for and then takes the lock while `alm` holds it.
  - In the built app: each field can be changed and saved; `alm show` agrees; History records it; a refusal shows its plain message.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-09 | Write the card (not start it) | owner (chat) |
| 2026-10-14 | Approve the card (ready): all eight fields changeable; no claiming or releasing from the app; the app asks the owner's name once and keeps it | owner (chat) |

## Notes

- 2026-10-09 — Card written by the agent. Next: the owner approves the card (`ready`); it starts after A01-17.
- 2026-10-14 — Owner approved the card; ready. Starts once A01-17 (change history) is done.
- 2026-10-14 — Built in `claude/a01-18-edit-task`; locally verified. Next: the owner tries it in the built app (below).

## Result (2026-10-14)

**In plain words.** Each changeable line of the task pane now has a **Change** link: a choice list for state, label and iteration, a text box for the title, waiting on, next need, handoff note and description, then Save / Cancel. The first time, the pane asks for your name once and keeps it in Allomos's settings on this computer (`settings.json`, beside the interface language); every change is recorded in the task's History under that name, marked as made in the app. The app never decides by itself whether a change is allowed: it hands the change to the same Rust rule `alm` uses, under the same taskbag lock, and a refusal comes back as the same problem `alm` reports, shown in the interface's language. Holder, area and "finished without the owner's check" stay read-only (holders are agent sessions; no claiming or releasing from the app).

How each field maps to `alm`'s rule (`crates/core/src/ops.rs`, `edit`): state *ready* → `ready`, *waiting* → `wait` (asks what it waits on), *done* → `done`, *dropped* → `drop`; *in progress* and *backlog* are not offered (and refused, `cannot_set_state`, if asked); label → `label`; waiting on → `wait` (so it also makes the task waiting); next need → `needs`; iteration → `plan`; title → `title`. Handoff note and description have no `alm` command of their own, so two operations were added with the same holder rule as `title` (`set_handoff_note`, recorded as `note`; `set_description`, recorded as `describe`, which rewrites `description.md` or removes it when emptied). Adding matching `alm` commands would be its own card.

The locked, recorded write that `alm` did inside `crates/cli` moved to `allomos_store::change_task`, and `alm`'s task commands now call it too, so the two writers share one code path (format check → lock, waiting up to 5 s → load → rule → event + files). This touched `crates/cli/src/lib.rs` (its `change` and `taskbag_of` now come from the store) and `apps/web/src/styles.css` (a few lines for the editor), outside the card's listed files; `apps/web/src/editFlow.ts` is new (the pane's choices, tested as `CODING_RULES` asks).

| Check | Result |
|---|---|
| `cargo fmt --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --workspace` | PASS (core 33, store 55 + 4 new in `tests/edits.rs`, cli 34, desktop 17) |
| `npm --prefix apps/web run typecheck`, `build`, `check:i18n`, `test` | PASS (43 web tests, 271 texts, 58 Rust codes) |
| Every edit goes through the same rule as its `alm` command | PASS: `each_app_edit_is_its_alm_rule` (core: each edit's result and refusal equal the direct rule's, and it names that command), `every_field_changes_and_is_recorded_under_the_persons_name` (store) |
| A refused edit changes no file | PASS: `a_refused_edit_changes_no_file` (every file in the taskbag compared before and after eight refusals: held task, label off the ladder, empty title, unknown iteration, waiting without text, in progress, unknown task), `an_older_taskbag_is_not_changed` |
| The app waits for and then takes the lock while `alm` holds it | PASS: `the_app_waits_for_the_lock_then_takes_it` (another holder of the same `.lock` `alm` takes: a short wait gives `taskbag_busy` and changes nothing; the normal 5 s wait succeeds once it lets go, ~300 ms later). Tested in-process with the store's `Lock`, the one `alm` uses; not with a separate `alm` process holding it |
| The Tauri command | PASS: `the_task_pane_changes_a_task_under_the_persons_name` (edit as the interface sends it; refused without a name and on a held task; recorded `by` the name, `via: app`, command `label`) |
| Built app | Built: `npm --prefix apps/desktop run tauri build -- --debug --bundles app` → `target/debug/bundle/macos/Allomos.app` in the worktree. Not driven by the agent (no screen access) |
| In the built app: each field changed and saved, `alm show` agrees, History records it, a refusal shows its message | NOT RUN — needs the owner's eyes |

**For the owner's check.** A throwaway project is at `~/allomos-tests/a01-18-edit-task/` (A01-01 is ready and free; A01-02 is held by `claude/trial`, to see a refusal; iterations I01 and I02). Open it with the branch's build:

```bash
open /Users/wanghui/coding/glidepace/allomos/.claude/worktrees/a01-18-edit-task/target/debug/bundle/macos/Allomos.app --args ~/allomos-tests/a01-18-edit-task
```

Then: click A01-01, **Change** its label (the pane asks your name the first time), its handoff note, and each other line; check the list, `alm show A01-01 --taskbag ~/allomos-tests/a01-18-edit-task` and History agree; on A01-02, change its label and see "Task A01-02 is held by claude/trial." (Use the worktree's `target/debug/alm`, or an `alm` reinstalled from this branch, for the terminal side.) Delete the folder once the card is done.
