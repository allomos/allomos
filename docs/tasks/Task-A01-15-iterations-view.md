# Task-A01-15-iterations-view

*Card type: coding. Area A01 (Tracking), Iteration 2 (first card). Requested by the owner (2026-10-08): "i also want a view to show tasks by iterations … i want it to be the next task".*

## What gets built (plain words)

An **iteration** is *when* a task is planned: a numbered step of the roadmap that ends in a release (Iteration 2 → v0.2). Requirement [R89](../product/REQUIREMENTS.md) already says every task may have one, next to its area; until now the seabag could not record it. After this card, a task can be placed in an iteration, and the app's list can be grouped **by iteration** as well as by area.

| Piece | What it is | Where |
|---|---|---|
| Iterations in the seabag | One small file per iteration: ID, name, order, and optionally start date, end date and the release it produces. | `.fliphopper/seabag/iterations/<id>.json` |
| A task's iteration | An optional `iteration` field in each task's `task.json`. A task without one is **Unplanned**. | `.fliphopper/seabag/tasks/<id>/task.json` |
| `fh` | Add an iteration; put a task in an iteration or take it out; list tasks grouped by iteration. Proposed: `fh add iteration I02 --name "v0.2 — macOS + Windows" --release v0.2`, `fh plan A01-16 I02` (`--none` to take it out), `fh list --by iteration`. | `crates/cli` |
| The app's list | A switch above the list: **By area · By iteration**. By iteration shows one group per iteration in order, each with its tasks (state, label, holder, waiting-on as today), then **Unplanned** last. The choice is remembered. View only, like the rest of the list until editing arrives. | `apps/web` |
| FlipHopper's own plan | FlipHopper's seabag gets its 15 iterations (from [PLAN § 9](../product/PLAN.md#9-roadmap--releases)), and each open card is placed in its iteration. Finished cards of v0.1 go in Iteration 1. | `.fliphopper/seabag/` (with `fh`, on `main`) |

**What you can see at the end:** open FlipHopper's own repository in the app, switch to **By iteration**: Iteration 1 (v0.1) holds the finished v0.1 cards, Iteration 2 (v0.2) holds this card and the task pane card, and the backlog ideas sit under Unplanned.

## Details for the worker

- **Outcome.** The owner sees the project's tasks grouped by iteration in the app; agents place tasks in iterations with `fh`.
- **Already decided.** The data shape is in [PLAN § 5](../product/PLAN.md#5-data-model-first-cut) (iteration: `id`, title, order, optional start/end dates and release; a task has at most one iteration) and [FILE-FORMAT.md](../product/FILE-FORMAT.md) ("Iterations (R89) will add an optional `iteration` field"). A seabag without an `iterations/` folder stays valid, and older tasks without the field are Unplanned.
- **Out of scope.** Renaming "Iteration" to sprint, milestone, etc. ([R90](../product/REQUIREMENTS.md)); per-iteration progress bars and burndown (Iterations 4 and 5); editing iterations in the app (comes with editing in Iteration 2).
- **Decisive unknowns.**
  - Command and ID names (`fh plan`, `I02`): proposed above; the owner may rename them at the check.
- **Files owned.** `crates/core`, `crates/store`, `crates/cli` (and their tests), `apps/desktop/src-tauri/src/lib.rs` if the list command changes, `apps/web/src/App.tsx`, `apps/web/src/types.ts`, `apps/web/src/i18n/*`, `docs/product/FILE-FORMAT.md` (iterations section), `docs/workflow/CODING_RULES.md` (the `fh` command lines), `docs/workflow/CARDS.md` (how cards are placed in iterations), this card. The seabag on `main` only through `fh`.
- **External operations.** None.
- **Done means mechanically.**
  - These pass: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm --prefix apps/web run typecheck`, `npm --prefix apps/web run build`, `npm --prefix apps/web run check:i18n`.
  - Tests: add an iteration; plan a task into it and out again; an unknown iteration is refused; a seabag with no `iterations/` still opens; `fh list --by iteration` puts tasks without an iteration under Unplanned, last.
  - In the built app, FlipHopper's own repository shows the grouping described above; switching back to By area shows the list as before.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-08 | A view of tasks by iteration, as the next task ("i want it to be the next task") | owner (chat) |

## Notes

- 2026-10-08 — Card written by the agent from the owner's request.
