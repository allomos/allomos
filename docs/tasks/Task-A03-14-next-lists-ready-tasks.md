# Task-A03-14-next-lists-ready-tasks

*Card type: coding. Area A03 (Agents). Written 2026-10-13 at the owner's request, following the rule "`alm` tells; the agent checks" (A03-13).*

## What gets built (plain words)

Today `alm next` picks one task for the agent: the first ready task nobody holds, by area order and then ID. That is `alm` making a choice for the agent. After this card, `alm next` lists **all** ready tasks nobody holds, and gives the agent the rule for choosing one; the agent chooses. `alm next --iteration <id>` lists only the tasks planned in that iteration.

Everything else `alm next` shows stays: each task's latest handoff, and the handoff rules (quota and context gates).

## Details for the worker

- **Outcome.** `alm next [--iteration <id>]` lists the ready, unheld tasks (in that iteration, if given) with the rule for choosing one; `alm` chooses nothing.
- **Decisive unknowns.** The wording of the choosing rule, for the owner to confirm. Proposed: "Take the task the person named, if any. Otherwise take one from the earliest iteration whose next dependency is met, and read its handoff first (`alm show <id>`). Claim it with `alm claim <id> --as <your session name> --push`."
- **Refusal.** An iteration that does not exist: `unknown_iteration` (as `alm list --iteration`).
- **Files owned.** `crates/cli/src/lib.rs` (`next`), `crates/core/src/ops.rs` (`next` becomes a list), `crates/cli/tests/alm.rs`, `docs/workflow/CODING_RULES.md` (`alm next` row), `crates/store/src/guide.rs`, this card. The app's own use of `ops::next`, if any, keeps working.
- **External operations.** None.
- **Done means mechanically.**
  - Tests: several ready tasks are all listed, held and non-ready ones are not; `--iteration` narrows the list; an unknown iteration is refused; no ready task gives the rule's "nothing to take" line; the `--json` output has `tasks` (each with its latest handoff) and `instruction`.
  - Checked by hand: `alm next` and `alm next --iteration I02` on this project's taskbag.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-13 | `alm next` lists the ready tasks and gives the rule for choosing; add `--iteration` | owner (chat) |
