# Task-A01-15-iterations-view

*Card type: coding. Area A01 (Tracking), Iteration 2 (first card). Requested by the owner (2026-10-08): "i also want a view to show tasks by iterations … i want it to be the next task".*

## What gets built (plain words)

An **iteration** is *when* a task is planned: a numbered step of the roadmap that ends in a release (Iteration 2 → v0.2). Requirement [R89](../product/REQUIREMENTS.md) already says every task may have one, next to its area; until now the seabag could not record it. After this card, a task can be placed in an iteration, and the app's list can be grouped **by iteration** as well as by area.

| Piece | What it is | Where |
|---|---|---|
| Iterations in the seabag | One small file per iteration: ID, name, order, and optionally start date, end date and the release it produces. | `.fliphopper/seabag/iterations/<id>.json` |
| A task's iteration | An optional `iteration` field in each task's `task.json`. A task without one is **Unplanned**. | `.fliphopper/seabag/tasks/<id>/task.json` |
| `fh` | Add an iteration; put a task in an iteration or take it out; list tasks grouped by iteration: `fh add iteration I02 "macOS + Windows" --release v0.2`, `fh plan A01-16 I02` (no iteration takes it out), `fh list --by iteration`. | `crates/cli` |
| The app's list | A switch above the list: **By area · By iteration**. By iteration shows one group per iteration in order, each with its tasks (state, label, holder, waiting-on as today), then **Unplanned** last. The choice is remembered. View only, like the rest of the list until editing arrives. | `apps/web` |
| FlipHopper's own plan | FlipHopper's seabag gets its 15 iterations (from [PLAN § 9](../product/PLAN.md#9-roadmap--releases)), and each open card is placed in its iteration. Finished cards of v0.1 go in Iteration 1. | `.fliphopper/seabag/` (with `fh`, on `main`) |

**What you can see at the end:** open FlipHopper's own repository in the app, switch to **By iteration**: Iteration 1 (v0.1) holds the finished v0.1 cards, Iteration 2 (v0.2) holds this card and the task pane card, and the backlog ideas sit under Unplanned.

## Details for the worker

- **Outcome.** The owner sees the project's tasks grouped by iteration in the app; agents place tasks in iterations with `fh`.
- **Already decided.** The data shape is in [PLAN § 5](../product/PLAN.md#5-data-model-first-cut) (iteration: `id`, title, order, optional start/end dates and release; a task has at most one iteration) and [FILE-FORMAT.md](../product/FILE-FORMAT.md) ("Iterations (R89) will add an optional `iteration` field"). A seabag without an `iterations/` folder stays valid, and older tasks without the field are Unplanned.
- **Out of scope.** Renaming "Iteration" to sprint, milestone, etc. ([R90](../product/REQUIREMENTS.md)); per-iteration progress bars and burndown (Iterations 4 and 5); editing iterations in the app (comes with editing in Iteration 2).
- **Decisive unknowns.**
  - Command and ID names (`fh plan`, `I02`): chosen by the worker to match `fh add area` and `fh needs`; the owner may rename them at the check.
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
- 2026-10-08 — Worked in `claude/a01-15-iterations`. Commit `0ba721c` holds the whole change (Rust, interface and docs), although its message names only the Rust part: a lint command run in the background committed everything in the worktree when it finished. Not rewritten, because pushed commits are never force-pushed.

## Result (2026-10-08)

**In plain words.** A project can now say *when* each task is planned. FlipHopper's seabag keeps one small file per iteration (`iterations/I02.json`: name, order, release, optional dates), and a task may name its iteration. Agents add iterations with `fh add iteration`, place tasks with `fh plan`, and see them with `fh list --by iteration`. In the app, a switch above the list, **By area · By iteration**, groups the same tasks by iteration in order, each with its release and progress bar, and puts tasks with no iteration last under **Unplanned**. The choice is remembered. In the by-area view, each task now also says which iteration it is in.

FlipHopper's own seabag got its 15 iterations (named by what they deliver, release `v0.1` … `v1.0`). Every finished task went into Iteration 1. Open cards went into their roadmap iteration: A01-15 and A01-16 → 2, A02-01 → 5, A05-01 → 7, A03-01 → 8, A04-01 → 9. Open decision records (A00-11, A00-14, A00-15), the quota idea (A03-02) and A05-02 (another session's card) stay unplanned.

**Older versions:** an older `fh` or app refuses a task that has an `iteration` field (`field_unknown`), so `fh` was reinstalled from this branch and the owner's check uses the new build.

| Check | Result |
|---|---|
| `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (core: dates, iteration parsing, cross-file checks, `new_iteration`, `plan`; store: iterations written, read, sorted, missing folder valid; `fh`: new `plan_tasks_in_iterations_and_list_by_iteration`) | PASS |
| `npm --prefix apps/web run typecheck`, `build`, `check:i18n` (121 texts, 41 codes) | PASS |
| `fh` by hand on a copy of FlipHopper's seabag: 15 iterations added, tasks planned, `fh list --by iteration` grouped as described, no issues | PASS |
| The interface in a browser with a copy of FlipHopper's seabag: the switch, Iteration 1 with 34 done tasks, empty iterations, Unplanned last, iteration shown in the by-area view | PASS (the built app's window could not be screenshotted here; same interface code) |
| Built app `target/debug/bundle/macos/FlipHopper.app` on FlipHopper's own repository: By iteration, then back to By area | PASS: owner verified ("checked", 2026-10-08) |
