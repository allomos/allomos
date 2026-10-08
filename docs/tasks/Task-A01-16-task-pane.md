# Task-A01-16-task-pane

*Card type: coding. Area A01 (Tracking), Iteration 2. Requested by the owner (2026-10-08) after asking whether viewing a task's details was defined: it was named in [PLAN § 6](../product/PLAN.md#6-views) ("Task detail") but never given a release or a card.*

## What gets built (plain words)

Today the app lists tasks but clicking one does nothing. After this card, clicking a task opens a **pane beside the list** (owner, 2026-10-08) showing everything known about that one task. It is the place where editing will later happen.

| Line in the pane | Meaning | Example (task A00-19) |
|---|---|---|
| ID and title | Which task this is | A00-19 · Decks new mark |
| Area and iteration | What part of the product, and when it is planned (iteration from [`A01-15`](./Task-A01-15-iterations-view.md)) | A00 Planning · Unplanned |
| State | Where it is in the flow: backlog, ready, in progress, waiting, done, dropped | done |
| Label | How far it is proven, on the project's ladder | owner verified |
| Holder | Which person or agent session works on it | claude/a00-19-republish |
| Waiting on / next need | The one thing that blocks it | — |
| Handoff note | Where the next session picks up | "Local files done; only the republish is left" |
| Description | The task's own text, shown as formatted Markdown. Links work; for FlipHopper's own tasks it links to the card file in `docs/tasks/`, which opens in the user's editor. | Card: Task-A00-19-decks-new-mark.md |
| History | Every change with who and when. Shown once changes are recorded as events (editing, Iteration 2); until then the line says history is not recorded yet. | — |

**What you can see at the end:** open FlipHopper's own repository in the app, click any task: the pane opens beside the list with these lines; click another task and the pane follows; close it and the list takes the full width again.

## Details for the worker

- **Outcome.** The owner reads everything about one task without leaving the app or opening files.
- **Owner's choices (2026-10-08).** A pane beside the list: accepted. Release v0.2: "ok for now".
- **Not in the pane.** Approvals: the seabag has no approvals field; for FlipHopper's own tasks they stay in the card file, reached through the description link.
- **Decisive unknowns.**
  - The owner has not yet accepted the list of lines above (the first explanation was unclear; this table replaces it).
  - How the pane behaves in a narrow window (below or over the list): the worker proposes one at the check.
- **Files owned.** `apps/web/src/App.tsx` (or a new `TaskPane.tsx`), `apps/web/src/types.ts`, `apps/web/src/i18n/*`, `apps/desktop/src-tauri/src/lib.rs` (reading one task's description, opening a linked file), `crates/store` if a read function is missing, this card.
- **External operations.** None.
- **Done means mechanically.**
  - These pass: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm --prefix apps/web run typecheck`, `npm --prefix apps/web run build`, `npm --prefix apps/web run check:i18n`.
  - In the built app: clicking a task opens the pane with its lines; the card link opens the card file; the pane works by keyboard (arrow keys move through tasks, Esc closes).

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-08 | Write the card; a pane beside the list; release v0.2 "ok for now" | owner (chat) |

The card is in backlog until the owner accepts the pane's lines.

## Notes

- 2026-10-08 — Card written by the agent from the owner's request.
