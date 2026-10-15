# Task-A01-39-iteration-list

*Card type: coding. Area A01 (Tracking), Iteration 2. Written 2026-10-15 by the worker session `claude/a01-39-iteration-list`, from the task's description in the taskbag (`alm show A01-39`). Builds on [`A01-18`](./Task-A01-18-edit-task-in-app.md) (the task pane's Change control), [`A01-19`](./Task-A01-19-create-in-app.md) (creating iterations in the app) and [`A01-29`](./Task-A01-29-rename-delete-areas-iterations.md).*

## What gets built (plain words)

In the task pane, **Change** next to the iteration opens a drop-down list. Its first item is **New…**, the second **No iteration** (the task is unplanned), then every iteration of the project, each shown by its ID and name (and its release, if it has one). The task's current iteration is already selected when the list opens. Picking an iteration and pressing **Save** moves the task there, by the same rule as `alm plan`, and the move is recorded in the task's history. Picking **New…** shows a small form: the new iteration's name and two optional dates (first and last day). **Save** then creates the iteration, by the same rule as `alm add iteration` (it gets the next free ID, which the form names), and puts the task in it; **Cancel** changes nothing.

| Piece | What it is | Where |
|---|---|---|
| The drop-down and the New… form | The list's order and choices, and what is sent to Rust, worked out in `editFlow.ts` (tested); the pane shows them | `apps/web` |
| Create and plan in one go | `allomos_store::add_iteration_and_plan`: under one taskbag lock, the `alm add iteration` rule, then the `alm plan` rule; both are checked before anything is written; two history events (`add iteration`, `plan`) | `crates/store` |
| The app's command | `add_iteration_and_plan` Tauri command, under the person's saved name, like the other changes | `apps/desktop/src-tauri` |

## Details for the worker

- **Outcome.** The person moves a task to any iteration, or to a brand-new one, from one list in the task pane.
- **Decisive unknowns.**
  - Decided (owner, 2026-10-14): "No iteration" is second, right after "New…".
  - Left to the agent (owner): whether New… asks for dates. **Chosen:** name plus two optional dates (first and last day); the release is not asked (it can be set later with the iteration's Edit form, A01-29). The ID is not asked either: it is the next free one, as `alm add iteration <name>` gives, and the form says which.
  - Agent's choice (routine, A00-29): choosing from the list does not save at once; **Save** does, as for every other field of the pane. Saving on choice would fire on arrow keys in a closed list on Windows.
- **Files owned.** `apps/web/src/TaskPane.tsx`, `apps/web/src/editFlow.ts`, `apps/web/src/messenger.ts`, `apps/web/src/App.tsx` (wiring only), `apps/web/src/i18n/{en,zh-CN,ja}.json`, `apps/web/src/styles.css` (two lines), `apps/web/tests/editFlow.test.ts`, `crates/store/src/lib.rs`, `crates/store/tests/adds.rs`, `apps/desktop/src-tauri/src/lib.rs`, this card.
- **External operations.** None.
- **Done means mechanically.**
  - The seven named checks of [CODING_RULES](../workflow/CODING_RULES.md#checks-before-a-commit) pass.
  - Tests: the list's order (New…, No iteration, then each iteration; an unknown current iteration still shown); the current iteration selected; choosing an iteration sends the `plan` edit; New… needs a name; New… adds then plans under one lock as two events; a refusal of either (no name, bad date, taken ID, unknown task) changes no file; it waits for `alm`'s lock.
  - Built app: the debug app builds; the owner tries it with the steps below.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-14 | "No iteration" is the second item, right after "New…"; whether "New…" asks for dates is the agent's choice | owner (task notes) |
| 2026-10-14 | Routine choices inside a card are the agent's (A00-29) | owner |

## Notes

- 2026-10-15 — Built in `claude/a01-39-iteration-list`; locally verified. Next: the owner tries it in the built app (below).

## Result (2026-10-15)

**In plain words.** The pane's iteration Change control is now the drop-down described above. Nothing in the interface decides whether a change is allowed: choosing an iteration goes to the same `plan` rule A01-18 already used; New… goes to a new store function that runs `alm add iteration`'s rule and then `alm plan`'s rule under one lock, writing two history entries, the same as typing both commands. Both rules are checked before anything is written, so a refused New… (for example an end date like "soon") leaves no half-made iteration behind. The new words are in English, Simplified Chinese and Japanese.

| Check | Result |
|---|---|
| `cargo fmt --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --workspace` | PASS (store `adds` 5 incl. new `a_new_iteration_from_the_task_pane_is_added_then_planned`; desktop 26 + 1 ignored as before, `the_list_adds_areas_iterations_and_tasks_under_the_persons_name` extended with the new command) |
| `npm --prefix apps/web run typecheck`, `build` | PASS |
| `npm --prefix apps/web run check:i18n` | PASS (371 texts in en, zh-CN and ja) |
| `npm --prefix apps/web test` | PASS (83 tests; 2 new in `editFlow.test.ts`) |
| Built app | Built: `npm --prefix apps/desktop run tauri build -- --debug --bundles app` → `target/debug/bundle/macos/Allomos.app` in the worktree. Not driven by the agent (no screen access) |
| In the built app: the list, choosing, New…, Cancel | NOT RUN — needs the owner's eyes |
| Windows | NOT RUN (untested; nothing platform-specific was added) |

**For the owner's check.** A throwaway project is at `~/allomos-tests/a01-39-iteration-list/` (area A01; iterations I01 First with dates, I02 Second; A01-01 unplanned, A01-02 in I01). Open it with the branch's build:

```bash
open -n /Users/wanghui/coding/glidepace/allomos/.claude/worktrees/a01-39-iteration-list/target/debug/bundle/macos/Allomos.app --args ~/allomos-tests/a01-39-iteration-list
```

Then: click A01-02 → **Change** next to Iteration: the list reads New…, No iteration, I01 First, I02 Second, with I01 selected. Pick I02 → Save: the pane and the list show I02, History shows the plan under your name (asked once). Change again → **New…**: a name box, two date boxes and "Saving adds iteration I03 …" appear; type a name, press **Cancel**: nothing changed. Again with a name → Save: I03 exists and A01-02 is in it; History shows the plan; `alm list --by iteration` agrees:

```bash
/Users/wanghui/coding/glidepace/allomos/.claude/worktrees/a01-39-iteration-list/target/debug/alm list --by iteration --taskbag ~/allomos-tests/a01-39-iteration-list
```

Optionally switch the interface to 中文 or 日本語 and look at the list's first two items. Delete the folder once the card is done.
