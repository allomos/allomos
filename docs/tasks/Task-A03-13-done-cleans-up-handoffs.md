# Task-A03-13-done-cleans-up-handoffs

*Card type: coding. Area A03 (Agents). Planned in I02 and started 2026-10-13 at the owner's request ("set A03-13 as current iteration task and proceed it").*

## What gets built (plain words)

Handoffs (the notes a session leaves for whoever continues) pile up in the taskbag's `handoffs/` folder even after their work is finished. After this card, `alm` gives the agent the rule for cleaning them up, and the agent does the checking itself:

1. Get the list with `alm handoffs --task <id>` (or `alm handoffs` for all).
2. For each handoff, look up every task it names (`alm show <id>`).
3. If they are all done or dropped, remove the handoff: `alm handoff --remove <file> --push`.
4. Otherwise leave it.

`alm done <id>` always ends with this rule for the task just finished. `alm handoffs --check` prints the same rule on its own (with `--task`, for one task). `alm` itself looks at no handoff and gives no verdict: **`alm` tells, the agent checks** (owner, 2026-10-13, now a rule in AGENTS.md and CODING_RULES).

Removing goes through `alm` too (the new `alm handoff --remove <file>`), because taskbag files are never edited by hand. The file is deleted; Git history keeps it if anyone needs it back. Once removed, the app's Handoffs tab and task pane no longer show it, because they list whatever files are in `handoffs/`.

| Piece | What it is | Where |
|---|---|---|
| Removal | `remove` (delete one handoff file, refusing anything outside `handoffs/`) | `crates/store/src/handoff.rs` |
| Command | `alm done` ends with the rule (`--json`: `handoff_check.instruction`); `alm handoffs --check` prints only the rule (`--json`: `instruction`); `alm handoff --remove <file>` (`--json`: `removed`) | `crates/cli/src/lib.rs` |
| Refusal | `handoff_not_found`, with its words in three languages | `crates/core/src/problem.rs`, `apps/web/src/i18n/` |
| Contract, rule and guide | the command rows and the "alm tells, the agent checks" rule; `alm guide commands` | `AGENTS.md`, `docs/workflow/CODING_RULES.md`, `crates/store/src/guide.rs` |

## Details for the worker

- **Outcome.** `alm done <id>` and `alm handoffs --check` give the agent the rule; the agent gets the list, checks each handoff and removes finished ones with `alm handoff --remove`.
- **Decisions (owner, 2026-10-13, in chat).** Removal is an `alm` command, not a hand deletion. The file is deleted outright; Git history is the copy. The agent does the checking, not `alm`; any check `alm` makes itself needs the owner's approval first.
- **Finished means** done or dropped; anything else (including a task that no longer exists) is the agent's judgement.
- **Files owned.** The files above; `crates/cli/tests/alm.rs`; this card.
- **External operations.** None.
- **Done means mechanically.**
  - Tests: `alm done` always ends with the rule for that task (text and `--json`); `alm handoffs --check` gives only the rule, with or without `--task`, and lists nothing; removing by listed or bare name; a missing file or a path outside `handoffs/` refused; `alm list --json` has no `issues` after removal.
  - Checked by hand: after `alm handoff --remove`, the desktop app's Handoffs tab and task pane no longer show the handoff.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-13 | `alm done` and `alm handoffs --check` only give the rule; the agent retrieves the list via `alm` and checks | owner (chat) |
| 2026-10-13 | `alm done` only asks the agent to check; rule: most checks are the agent's, and a check made by `alm` needs owner approval | owner (chat) |
| 2026-10-13 | Add `alm handoffs --check`, which asks the agent to go through the handoffs one by one and remove those whose tasks are all done | owner (chat) |
| 2026-10-13 | Plan in the current iteration (I02) and build it; removal via `alm handoff --remove`; delete the file, Git keeps history | owner (chat) |
