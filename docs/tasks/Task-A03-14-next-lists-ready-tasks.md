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

## Notes

- **2026-10-13, claude/a03-14-next-list — locally verified.** `alm next` now lists every ready task nobody holds, earliest iteration first (unplanned last), then area order and ID, each with its handoff note and latest handoff, followed by the owner-approved choosing rule (verbatim) and the handoff rules. `--iteration <id>` narrows the list; an unknown one is refused with `unknown_iteration`. With nothing free it says "Nothing to take: no ready task (in iteration X) is free. Ask the person what to do next." `--json`: `tasks` (each with `handoff`), `instruction`, `handoff_rules`; the old single `task`/`handoff` keys are gone. `ops::next(taskbag, iteration)` now returns the list; nothing in the app called it. The brief in `crates/store/src/lib.rs` still says "the next ready task" — outside this card's files (changing it bumps the brief version), left for a follow-up if wanted.
  - Checks: `cargo fmt --check` PASS; `cargo clippy --workspace --all-targets -- -D warnings` PASS; `cargo test --workspace` PASS (new: `next_lists_every_free_ready_task_with_the_rule_for_choosing`, `ops::tests::next_lists_free_ready_tasks_by_iteration_area_then_id`); web `typecheck`, `build`, `check:i18n`, `test` (35/35) PASS.
  - By hand on this project's taskbag: `alm next` and `alm next --iteration I02` both list `A03-15` (I02) with the rule and handoff rules; `--iteration I01` gives the "Nothing to take" line; `--iteration I99` exits 1 with `unknown_iteration`.
