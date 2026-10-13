# Task-A01-17-change-history

*Card type: coding. Area A01 (Tracking), Iteration 2. Written 2026-10-09 at the owner's request ("create these cards") for the rest of v0.2. Requirement [R7](../product/REQUIREMENTS.md): history, burndown and timeline are computed from recorded events.*

## What gets built (plain words)

Today Allomos keeps only each task's *current* fields; nobody can see who changed what, or when. After this card, every change, made with `alm` or later in the app, is also written down as an **event**: what changed, from what to what, who did it and when. The task pane's **History** line, which now says "not recorded yet", lists them.

| Piece | What it is | Where |
|---|---|---|
| Events in the taskbag | One small file per change, named so that files never clash when two people's changes are merged by Git. Format added to the file-format contract. | `.allomos/taskbag/events/` |
| Written by every change | Each `alm` command that changes something writes its event in the same locked step as the change itself. | `crates/core`, `crates/store`, `crates/cli` |
| History in the task pane | Newest first: when, who, what changed (for example "state: ready → in progress, by claude/a01-15-iterations, 9 Oct 14:02"). | `apps/web` |
| `alm history <id>` | The same list in the terminal, and in `--json`. | `crates/cli` |

**What you can see at the end:** run `alm claim` and `alm label` on a task, open it in the app: History lists both changes with who and when.

## Details for the worker

- **Outcome.** Anyone can see a task's history, and later cards (burndown, timeline) can compute from it instead of from guesses.
- **Decisive unknowns.**
  - The event file layout (one file per event vs. one file per task per day): the worker proposes one in the file-format contract before coding; the owner accepts it. Recommended: one file per event, named by time and a random suffix, so Git merges never conflict.
  - Changes made before this card have no events. Recommended: none are invented; History says when recording started.
- **Files owned.** `crates/core`, `crates/store`, `crates/cli` and their tests, `apps/web/src/TaskPane.tsx`, `apps/web/src/types.ts`, `apps/web/src/i18n/*`, `apps/desktop/src-tauri/src/lib.rs` if a read command is needed, `docs/product/FILE-FORMAT.md` (events section), `docs/workflow/CODING_RULES.md` (`alm history`), this card.
- **External operations.** None.
- **Done means mechanically.**
  - The six named checks pass (`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm --prefix apps/web run typecheck`, `build`, `check:i18n`).
  - Tests: every changing `alm` command writes exactly one event; a refused change writes none; two taskbag copies with different events merge in Git without conflict; an older taskbag without `events/` still opens.
  - In the built app, a task changed with `alm` shows those changes under History.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-09 | Write the card (not start it) | owner (chat) |
| 2026-10-13 | Card approved ("approve"), with the two recommendations: one event file per change, named by time and a random suffix; no history invented for changes made before this card | owner (chat) |

## Notes

- 2026-10-09 — Card written by the agent. Next: the owner approves the card (`ready`).
- 2026-10-13 — Owner approved the card with both recommendations; moved to ready and claimed by `claude/a01-17-history`.
