# Task-A01-19-create-in-app

*Card type: coding. Area A01 (Tracking), Iteration 2. Written 2026-10-09 at the owner's request for the rest of v0.2. Builds on [`A01-18`](./Task-A01-18-edit-task-in-app.md) (editing in the app).*

## What gets built (plain words)

After [`A01-18`](./Task-A01-18-edit-task-in-app.md) the owner can change tasks in the app, but new tasks, areas and iterations still need `fh add`. After this card, the owner adds them in the app too, with the same checks `fh add` makes (a unique ID, an existing area, a real date).

| Piece | What it is | Where |
|---|---|---|
| New task | A "New task" button in each area (and each iteration) group: ID (suggested as the next free number in that area, for example `A01-20`), title, description, iteration. It starts in backlog, like `fh add task`. | `apps/web` |
| New area | "New area" at the end of the by-area list: ID and name. | `apps/web` |
| New iteration | "New iteration" at the end of the by-iteration list: ID, name, release, optional dates. | `apps/web` |
| Same rules, recorded | The same Rust rules as `fh add`; each creation is recorded in history ([`A01-17`](./Task-A01-17-change-history.md)). | `apps/desktop/src-tauri` |

**What you can see at the end:** in the app, add an iteration and a task in it; the list shows them where they belong, and `fh list --by iteration` agrees.

## Details for the worker

- **Outcome.** The owner plans new work without a terminal.
- **Decisive unknowns.**
  - Suggested IDs: proposed as the next free number in the area (`A01-20`) or iteration (`I16`); the owner may type another.
  - Dropping a task (`fh drop`) from the app: proposed in A01-18's state list, not here.
- **Files owned.** `apps/web/src/App.tsx` (or a new component), `apps/web/src/messenger.ts`, `apps/web/src/types.ts`, `apps/web/src/i18n/*`, `apps/desktop/src-tauri/src/lib.rs`, tests, this card.
- **External operations.** None.
- **Done means mechanically.**
  - The six named checks pass.
  - Tests: a duplicate ID, an unknown area and a bad date are refused with the same codes as `fh add`; a creation writes one event.
  - In the built app: create an area, an iteration and a task; `fh list` shows them.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-09 | Write the card (not start it) | owner (chat) |

## Notes

- 2026-10-09 — Card written by the agent. Next: the owner approves the card (`ready`); it starts after A01-18.
