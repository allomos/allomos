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
  - Which fields the owner wants changeable first (all eight above are proposed); and whether claiming/releasing a task from the app is wanted (proposed: not now: holders are agent sessions).
  - The owner's name for the history: proposed as a one-time question in the app.
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

## Notes

- 2026-10-09 — Card written by the agent. Next: the owner approves the card (`ready`); it starts after A01-17.
