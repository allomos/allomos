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
| 2026-10-14 | History checked in the app ("looks good"); merge into `main`, upgrade Allomos's own taskbag to `allomos/3` and reinstall `alm` ("go") | owner (chat) |

## Notes

- 2026-10-09 — Card written by the agent. Next: the owner approves the card (`ready`).
- 2026-10-13 — Owner approved the card with both recommendations; moved to ready and claimed by `claude/a01-17-history`.
- 2026-10-13 — Built in `claude/a01-17-history`. The event layout is in [FILE-FORMAT § Events](../product/FILE-FORMAT.md#events-allomos3). Because the taskbag gains a folder, its format goes from `allomos/2` to `allomos/3` (the rule from [`A01-25`](./Task-A01-25-taskbag-format-upgrade.md)): an older taskbag is refused by `alm` until `alm upgrade` runs, and the app offers **Update the taskbag…**. This also means an older `alm` refuses an upgraded taskbag (`taskbag_newer`), so every writer records events. Changes to `project.json` (labels, name, task language, handoff rules, project folder), new areas and new iterations are recorded too; handoffs and the handoff template are not (each handoff carries its own writer and times).

## Result (2026-10-13)

**In plain words.** Every change made with `alm` is now also written down as one small file in the taskbag's `events/` folder: what changed, from what to what, who did it and when. `alm history <id>` lists a task's changes, newest first, and the task pane's **History** shows the same list in the app's language. A taskbag made before this card starts its history when it is upgraded; nothing earlier is made up.

| Check | Result |
|---|---|
| `cargo fmt --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --workspace` | PASS (core 30, store 53, cli 30, desktop 16) |
| `npm --prefix apps/web run typecheck`, `build`, `check:i18n`, `test` | PASS (39 web tests) |
| Every changing `alm` command writes exactly one event; refused and no-op changes write none | PASS: `every_change_writes_exactly_one_event_and_a_refusal_none` |
| Two taskbag copies with different events merge in Git without a conflict | PASS: `two_copies_with_different_events_merge_without_conflict` |
| An older taskbag without `events/` still opens, is upgraded, and its history starts then | PASS: `an_older_taskbag_without_events_is_upgraded_and_history_starts_then`, `the_real_upgrade_from_1_adds_the_handoff_files_and_events` |
| In the built app, a task changed with `alm` shows those changes under History | PASS: owner checked A01-01 in `~/allomos-tests/a01-17-history/` with the branch's build ("looks good", 2026-10-14). The agent did not look (screen access declined). |
