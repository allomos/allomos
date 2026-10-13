# Task-A01-19-create-in-app

*Card type: coding. Area A01 (Tracking), Iteration 2. Written 2026-10-09 at the owner's request for the rest of v0.2. Builds on [`A01-18`](./Task-A01-18-edit-task-in-app.md) (editing in the app).*

## What gets built (plain words)

After [`A01-18`](./Task-A01-18-edit-task-in-app.md) the owner can change tasks in the app, but new tasks, areas and iterations still need `alm add`. After this card, the owner adds them in the app too, with the same checks `alm add` makes (a unique ID, an existing area, a real date).

| Piece | What it is | Where |
|---|---|---|
| New task | A "New task" button in each area group and each iteration group: ID (suggested as the next free number in the area, for example `A01-30`), title, description, iteration. In an iteration group the form also asks for the area, and the suggested ID follows the area picked. It starts in backlog, like `alm add task`. | `apps/web` |
| New area | "New area" at the end of the by-area list: ID (suggested as the next free one, for example `A10`) and name. | `apps/web` |
| New iteration | "New iteration" at the end of the by-iteration list: ID (suggested as the next free one, for example `I16`), name, release, optional dates. | `apps/web` |
| Same rules, recorded | The same Rust rules as `alm add`; each change is recorded in history ([`A01-17`](./Task-A01-17-change-history.md)). A task created with an iteration is two steps under one lock, `alm add task` then `alm plan`, so history shows two events. | `apps/desktop/src-tauri` |
| `alm` matches | `alm add area` and `alm add iteration` take the ID as optional, like `alm add task`: left out, the next free one is used. | `crates/core`, `crates/cli` |
| No Git | The app does not commit or push until Iteration 6, as in A01-18. | — |

**What you can see at the end:** in the app, add an iteration and a task in it; the list shows them where they belong, and `alm list --by iteration` agrees.

## Details for the worker

- **Outcome.** The owner plans new work without a terminal.
- **Decisive unknowns.**
  - Decided (owner, 2026-10-14): suggested IDs for tasks, areas and iterations, each the next free one; the owner may type another. `alm add area` and `alm add iteration` get the same optional ID.
  - Decided (owner, 2026-10-14): a task created with an iteration is `add` then `plan` under one lock: two events, the same as typing both commands. No new combined operation.
  - Decided (owner, 2026-10-14): dropping a task belongs to A01-18's state list (approved there).
  - Out of scope: renaming, re-dating and deleting areas and iterations: [`A01-29`](./Task-A01-29-rename-delete-areas-iterations.md).
- **Files owned.** `apps/web/src/App.tsx` (or a new component), `apps/web/src/messenger.ts`, `apps/web/src/types.ts`, `apps/web/src/i18n/*`, `apps/desktop/src-tauri/src/lib.rs`, `crates/core/src/ops.rs` (next free area and iteration ID), `crates/cli` (optional ID), tests, this card.
- **External operations.** None.
- **Done means mechanically.**
  - The six named checks pass.
  - Tests: a duplicate ID, an unknown area and a bad date are refused with the same codes as `alm add`; creating an area, an iteration or a task writes one event, and a task with an iteration writes two (created, planned); `alm add area` and `alm add iteration` without an ID take the next free one.
  - In the built app: create an area, an iteration and a task; `alm list` shows them.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-09 | Write the card (not start it) | owner (chat) |
| 2026-10-14 | Approve the card (ready): suggested IDs for all three, also optional in `alm add area` / `alm add iteration`; task with iteration = add then plan, two events; area choice in iteration groups; rename/delete moved to A01-29 | owner (chat) |

## Notes

- 2026-10-09 — Card written by the agent. Next: the owner approves the card (`ready`); it starts after A01-18.
- 2026-10-14 — Discussed with the owner; decisions above recorded, card approved; ready. Starts once A01-18 is done.
- 2026-10-14 — Built in `claude/a01-19-create-in-app`; locally verified. Next: the owner tries it in the built app (below).

## Result (2026-10-14)

**In plain words.** The list now has a **+ New task** link in each group's header: in an area group the task goes in that area; in an iteration group (and Unplanned) the form also asks for the area, and the task is planned in that iteration. The form fills in the ID with the next free one in the area (for example `A01-30`) and changes it when another area is picked, unless you typed your own; an emptied ID box also means "the next free one". **+ New area** sits at the end of the by-area list and **+ New iteration** at the end of the by-iteration list, each with the next free ID filled in (`A08` after `A07`, `I16` after `I15`), a name, and for an iteration a release and two optional dates. The suggestions are worked out in Rust (`next_ids`), not in the interface. Adding is recorded in History under your name like A01-18's changes (asked once if the app does not know it yet); a new task opens in the task pane. The group-by buttons now show even in an empty project, so **By iteration** (and New iteration) can be reached before any task exists.

The app does not decide anything itself: each form goes to the same Rust code as `alm add`. The locked, recorded add that `alm` did inside `crates/cli` moved to `allomos_store::add_area`, `add_iteration` and `add_task` (format check → lock, waiting up to 5 s → load → `crates/core` rule → event + files), and `alm add` now calls them too, so both share one path, as A01-18 did for changes. A task with an iteration is `add task` then `plan` under the one lock: two events; both rules are checked before anything is written, so an unknown iteration adds nothing. `alm add area [<id>] <name>` and `alm add iteration [<id>] <name>` now take the ID as optional (with only a name: the next free one; `ops::next_area_id` / `next_iteration_id`: the last ID's letters, one more than the highest number with them, as wide as the widest, at least two digits). `alm add area A07 Later` works as before.

Outside the files listed above: `crates/store/src/lib.rs` (the shared add path, `NewTask`), `crates/store/src/guide.rs` and `docs/workflow/CODING_RULES.md` (the two commands' new optional ID), `apps/web/src/styles.css` (the forms); new: `apps/web/src/CreateForm.tsx`, `apps/web/src/createFlow.ts` (the forms' choices, tested as `CODING_RULES` asks), `crates/store/tests/adds.rs`.

| Check | Result |
|---|---|
| `cargo fmt --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --workspace` | PASS (core 34, store 55 + adds 4 + edits 4, cli 35, desktop 24 + 1 ignored as before) |
| `npm --prefix apps/web run typecheck`, `build`, `check:i18n`, `test` | PASS (52 web tests, 298 texts, 58 Rust codes) |
| A duplicate ID, an unknown area and a bad date are refused with the same codes as `alm add` | PASS: `refusals_have_alms_codes_and_write_nothing` (store: `duplicate_area_id`, `invalid_id`, `area_name_missing`, `duplicate_iteration_id`, `invalid_date`, `duplicate_task_id`, `unknown_area` with and without an ID, `unknown_iteration`, `task_title_missing`; every file compared before and after: unchanged); `add_area_and_iteration_without_an_id_take_the_next_free_one` (cli: the same codes from `alm add`) |
| Creating an area, an iteration or a task writes one event; a task with an iteration two (created, planned) | PASS: `each_new_item_writes_one_event_and_a_planned_task_two` (`add area`, `add iteration`, `add task`; then `add task` + `plan`, the plan event changing only `iteration`, all `by` Will `via` app) |
| `alm add area` / `alm add iteration` without an ID take the next free one | PASS: `the_next_area_and_iteration_ids_follow_the_pattern` (core), `add_area_and_iteration_without_an_id_take_the_next_free_one` (cli: A03 after A02, A08 after a given A07, I01, I02; one event each) |
| The app waits for `alm`'s lock | PASS: `the_app_waits_for_alms_lock` (busy → `taskbag_busy`, nothing added; free → added) |
| The Tauri commands, with the arguments as the interface sends them | PASS: `the_list_adds_areas_iterations_and_tasks_under_the_persons_name` (`next_ids` A01/I01 then I08 and A01-01; no name → `holder_missing`; empty ID box → suggested ID; `{task: {id: "", area, title, description, iteration}}` → added and planned, history newest first `plan`, `add task`, by Will via app; unknown area refused, nothing added) |
| Built app | Built: `npm --prefix apps/desktop run tauri build -- --debug --bundles app` → `target/debug/bundle/macos/Allomos.app` in the worktree. Not driven by the agent (no screen access) |
| In the built app: create an area, an iteration and a task; `alm list` shows them | NOT RUN — needs the owner's eyes |

**For the owner's check.** A throwaway project is at `~/allomos-tests/a01-19-create-in-app/` (areas A01 Tracking and A02 Charts, iteration I01 First, task A01-01 planned in I01; made with this branch's `alm`, including `alm add area "Charts"` without an ID). Open it with the branch's build:

```bash
open /Users/wanghui/coding/glidepace/allomos/.claude/worktrees/a01-19-create-in-app/target/debug/bundle/macos/Allomos.app --args ~/allomos-tests/a01-19-create-in-app
```

Then: **By area** → **+ New area** (ID shows `A03`) → name it → Add. **By iteration** → **+ New iteration** (`I02`), give a release and dates → Add. In the I02 group, **+ New task**: pick area A02 and see the ID turn to `A02-01`, add a title → Add; the task opens in the pane, planned in I02, and its History shows two entries (added, planned) under your name. Try a refusal: New area with ID `A01` ("Another area already uses the ID"), or New task with an ID already used. Then compare in Terminal:

```bash
/Users/wanghui/coding/glidepace/allomos/.claude/worktrees/a01-19-create-in-app/target/debug/alm list --by iteration --taskbag ~/allomos-tests/a01-19-create-in-app
```

Delete the folder once the card is done.
