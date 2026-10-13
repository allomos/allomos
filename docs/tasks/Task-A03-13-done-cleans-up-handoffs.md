# Task-A03-13-done-cleans-up-handoffs

*Card type: coding. Area A03 (Agents). Planned in I02 and started 2026-10-13 at the owner's request ("set A03-13 as current iteration task and proceed it").*

## What gets built (plain words)

Handoffs (the notes a session leaves for whoever continues) pile up in the taskbag's `handoffs/` folder even after their work is finished. After this card, when an agent finishes a task with `alm done <id>`, `alm` looks at every handoff that names that task and says what to do with each:

- **All its tasks are finished** (done or dropped): remove it, and `alm` prints the exact command, `alm handoff --remove handoffs/<file>.md --push`.
- **Some of its tasks are still open:** leave it as it is; `alm` names the open ones.
- **No handoff names the task:** `alm done` prints nothing extra.

Removing goes through `alm` too (the new `alm handoff --remove <file>`), because taskbag files are never edited by hand. The file is deleted; Git history keeps it if anyone needs it back. Once removed, the app's Handoffs tab and task pane no longer show it, because they list whatever files are in `handoffs/`.

| Piece | What it is | Where |
|---|---|---|
| Follow-ups and removal | `followups` (handoffs naming a task, with their unfinished tasks), `remove` (delete one handoff file, refusing anything outside `handoffs/`) | `crates/store/src/handoff.rs` |
| Command | `alm done` adds the follow-up lines (`--json`: `handoffs`); `alm handoff --remove <file>` (`--json`: `removed`) | `crates/cli/src/lib.rs` |
| Refusal | `handoff_not_found`, with its words in three languages | `crates/core/src/problem.rs`, `apps/web/src/i18n/` |
| Contract and guide | the two command rows; `alm guide commands` | `docs/workflow/CODING_RULES.md`, `crates/store/src/guide.rs` |

## Details for the worker

- **Outcome.** `alm done <id>` tells the agent, for each handoff that names the task, to remove it (all its tasks finished, with the exact command) or leave it (tasks still open); `alm handoff --remove` removes it.
- **Decisions (owner, 2026-10-13, in chat).** Removal is an `alm` command, not a hand deletion. The file is deleted outright; Git history is the copy.
- **Finished means** done or dropped; a task no longer in the taskbag also counts as finished.
- **Files owned.** The files above; `crates/cli/tests/alm.rs`; this card.
- **External operations.** None.
- **Done means mechanically.**
  - Tests: no related handoff (no extra output, no `handoffs` field); a handoff whose tasks are all finished (`remove` with the exact command, in JSON and in words); a handoff with tasks still open (`keep`, naming them); removing by listed or bare name; a missing file or a path outside `handoffs/` refused; `alm list --json` has no `issues` after removal.
  - Checked by hand: after `alm handoff --remove`, the desktop app's Handoffs tab and task pane no longer show the handoff.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-13 | Plan in the current iteration (I02) and build it; removal via `alm handoff --remove`; delete the file, Git keeps history | owner (chat) |
