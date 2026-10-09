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
  - How the pane behaves in a narrow window: proposed and built **over the list**, with a close button (below 720 px wide).
- **Files owned.** `apps/web/src/App.tsx` (or a new `TaskPane.tsx`), `apps/web/src/types.ts`, `apps/web/src/i18n/*`, `apps/desktop/src-tauri/src/lib.rs` (reading one task's description, opening a linked file), `crates/store` if a read function is missing, this card.
- **External operations.** None.
- **Done means mechanically.**
  - These pass: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm --prefix apps/web run typecheck`, `npm --prefix apps/web run build`, `npm --prefix apps/web run check:i18n`.
  - In the built app: clicking a task opens the pane with its lines; the card link opens the card file; the pane works by keyboard (arrow keys move through tasks, Esc closes).

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-08 | Write the card; a pane beside the list; release v0.2 "ok for now" | owner (chat) |
| 2026-10-08 | The pane's lines as listed ("accept, start it") | owner (chat) |

## Notes

- 2026-10-08 — Card written by the agent from the owner's request.
- 2026-10-08 — Owner accepted the pane's lines; worked in `claude/a01-16-task-pane`.

## Result (2026-10-08)

**In plain words.** Clicking a task in the list opens a pane beside it with the task's ID and title, area, iteration (with its release), state, label, holder, what it waits on, its next need, its handoff note, its description and a history line. Empty fields show a dash. The description is shown as formatted text (headings, lists, code, bold, links). Clicking the card link opens the card file in the user's editor. Up and down arrows move through the tasks in the order the list shows them (by area or by iteration), Esc or × closes the pane, and clicking the selected task again closes it too. In a narrow window the pane covers the list instead of sitting beside it. History says it is not recorded yet; it fills in once editing records changes.

**Opening links safely.** A description is plain text anyone can write, so the app opens only two kinds of link: web pages (`http`, `https`) in the browser, and `.md`, `.markdown` or `.txt` files inside the project folder in the user's editor. Any other link (a program, a folder, a file outside the project, `file:` or `mailto:`) is refused with a plain message (`link_refused`; a missing file: `link_not_found`). The check is in Rust (`resolve_task_link` in `crates/store`); the Markdown is turned into page elements directly, never into HTML text, so a description cannot add markup. No new package was needed.

| Check | Result |
|---|---|
| `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (store: new `task_links_open_only_text_files_in_the_project_and_web_pages`) | PASS |
| `npm --prefix apps/web run typecheck`, `build`, `check:i18n` (137 texts, 43 codes) | PASS |
| The interface in a browser with FlipHopper's own tasks and descriptions: click opens the pane; ↓ ↓ ↑ move the selection; the card link asks the app to open `../../../../docs/tasks/Task-A00-03-….md`; Esc closes; narrow window: pane over the list; dark mode readable | PASS |
| Built app on FlipHopper's own repository: click a task, open its card link, keyboard | PASS: owner (2026-10-09), with one change asked: the scroll bar belongs at the list's right edge, not the pane's |
| Change (2026-10-09): the list scrolls on its own, beside the pane, which scrolls separately; the window no longer scrolls as a whole. In the browser: the list's scroll bar is at its right edge (x 642, pane from x 656); opening the pane keeps the list's place | PASS; owner re-check in the rebuilt app |
