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
