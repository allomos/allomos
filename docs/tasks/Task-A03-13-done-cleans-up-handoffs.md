# Task-A03-13-done-cleans-up-handoffs

*Card type: coding. Area A03 (Agents). Planned in I02 and started 2026-10-13 at the owner's request ("set A03-13 as current iteration task and proceed it").*

## What gets built (plain words)

Handoffs (the notes a session leaves for whoever continues) pile up in the taskbag's `handoffs/` folder even after their work is finished. After this card, when an agent finishes a task with `alm done <id>` and some handoffs name that task, `alm` asks the agent to check them: one line saying how many there are and to run `alm handoffs --check --task <id>`. When no handoff names the task, `alm done` prints nothing extra.

`alm handoffs --check` (with `--task`, only those naming it) lists each handoff with the state of every task it names, and the rule: remove a handoff whose tasks are all done or dropped with `alm handoff --remove <file> --push`; leave the others, and any handoff that names no task. The agent checks them one by one and runs the removals. `alm` itself gives no verdict: **`alm` tells, the agent checks** (owner, 2026-10-13, now a rule in AGENTS.md and CODING_RULES).

Removing goes through `alm` too (the new `alm handoff --remove <file>`), because taskbag files are never edited by hand. The file is deleted; Git history keeps it if anyone needs it back. Once removed, the app's Handoffs tab and task pane no longer show it, because they list whatever files are in `handoffs/`.

| Piece | What it is | Where |
|---|---|---|
| Removal | `remove` (delete one handoff file, refusing anything outside `handoffs/`) | `crates/store/src/handoff.rs` |
| Command | `alm done` asks the agent to check (`--json`: `handoff_check`); `alm handoffs --check` lists them all with their tasks' states for the agent to check (`--json`: `instruction`, `handoffs`); `alm handoff --remove <file>` (`--json`: `removed`) | `crates/cli/src/lib.rs` |
| Refusal | `handoff_not_found`, with its words in three languages | `crates/core/src/problem.rs`, `apps/web/src/i18n/` |
| Contract, rule and guide | the command rows and the "alm tells, the agent checks" rule; `alm guide commands` | `AGENTS.md`, `docs/workflow/CODING_RULES.md`, `crates/store/src/guide.rs` |

## Details for the worker

- **Outcome.** `alm done <id>` asks the agent to check the handoffs that name the task; `alm handoffs --check` lays them out; the agent removes finished ones with `alm handoff --remove`.
- **Decisions (owner, 2026-10-13, in chat).** Removal is an `alm` command, not a hand deletion. The file is deleted outright; Git history is the copy. The agent does the checking, not `alm`; any check `alm` makes itself needs the owner's approval first.
- **Finished means** done or dropped; a task no longer in the taskbag shows as `missing` and also counts as finished. This is the rule the agent applies, written in `alm`'s instruction.
- **Files owned.** The files above; `crates/cli/tests/alm.rs`; this card.
- **External operations.** None.
- **Done means mechanically.**
  - Tests: `alm done` with no related handoff (no extra output, no `handoff_check`); with two (one line asking to check, the command, the files, no verdict); removing by listed or bare name; a missing file or a path outside `handoffs/` refused; `alm list --json` has no `issues` after removal; `alm handoffs --check` lists each handoff with its tasks' states (done, dropped, open, none) and the instruction, and gives no verdict.
  - Checked by hand: after `alm handoff --remove`, the desktop app's Handoffs tab and task pane no longer show the handoff.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-13 | `alm done` only asks the agent to check; rule: most checks are the agent's, and a check made by `alm` needs owner approval | owner (chat) |
| 2026-10-13 | Add `alm handoffs --check`, which asks the agent to go through the handoffs one by one and remove those whose tasks are all done | owner (chat) |
| 2026-10-13 | Plan in the current iteration (I02) and build it; removal via `alm handoff --remove`; delete the file, Git keeps history | owner (chat) |
