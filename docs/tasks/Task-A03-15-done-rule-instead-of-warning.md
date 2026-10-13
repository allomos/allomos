# Task-A03-15-done-rule-instead-of-warning

*Card type: coding. Area A03 (Agents). Written 2026-10-13 at the owner's request, following the rule "`alm` tells; the agent checks" (A03-13).*

## What gets built (plain words)

Today `alm done` judges whether a task is really finished: when the task's label is below the top step of the ladder (for this project, *owner verified*), it still marks the task done but warns `label_below_top`. After this card, `alm` no longer judges this. Instead, the agent is given the rule for finishing **before** it runs `alm done`:

- Finish a task only when it has reached the top label (the owner has checked it).
- **Exception:** a task worked in an automatic or loop run (for example a card run or `/loop` the person set up to go on without them) may be finished at a lower label, without the owner's check. Say so in the handoff.

The label the task had when it was finished stays on it, so anyone can still see a task that was finished without the owner's check.

## Details for the worker

- **Outcome.** `alm done` gives no `label_below_top` warning; the finishing rule, with its automatic/loop exception, is in what agents read before finishing; a task finished without the owner's check carries a note in its file that `alm list --without-owner` finds.
- **The note.** A new optional `task.json` field, `finished_without_owner` (text: why, which run), written only by `alm done <id> --without-owner "<text>"` (empty text refused). Kept if the task is later labelled *owner verified*, so the history stays searchable; the owner's check then shows in the label. `alm` records what the agent says; it does not check whether the run was really automatic.
- **File format.** A new `task.json` field: older `alm` builds would refuse it (`field_unknown`), so the format goes up (`allomos/3`) with an upgrade step that adds nothing, and FILE-FORMAT.md gets the field. Coordinate with A01-17 (change history), which is in progress and may also change the format: whichever lands second takes the next number.
- **Where the rule goes.** `alm guide workflow` step 7 and the Allomos section for AGENTS.md / CLAUDE.md (`alm brief`, the "Finished" line). Changing `alm brief` raises its version, so `alm` will ask agents in every project to refresh that section (`instructions_outdated`); this project's own AGENTS.md section is refreshed in the same card.
- **Decisive unknowns.** None: whether the run was automatic is the agent's judgement; `alm` only records it.
- **Removed.** The `label_below_top` problem code (`crates/core/src/problem.rs`, `apps/web/src/i18n/*.json`, `codes.json`), unless the app still shows it elsewhere.
- **Files owned.** `crates/core/src/ops.rs` (`finish`), `crates/core/src/problem.rs`, `crates/store/src/guide.rs`, `crates/store/src/lib.rs` (the brief), `apps/web/src/i18n/`, `crates/cli/tests/alm.rs`, `docs/workflow/CODING_RULES.md` (`alm done` row), `AGENTS.md` (the Allomos section), `crates/core/src/model.rs` and `validate.rs` (the field), `crates/store/src/versions.rs` (upgrade step), `docs/product/FILE-FORMAT.md`, this card. The app showing the note (task pane) is out of scope unless trivial; say so if left out.
- **External operations.** None.
- **Done means mechanically.**
  - Tests: `alm done` on a task below the top label succeeds with no warning; `--without-owner "<why>"` writes the note to `task.json`, `alm show` shows it, `alm list --without-owner` finds only such tasks, an empty text is refused; `alm upgrade` from `allomos/2` works; the guide and the brief contain the rule and the exception; the translation check passes.
  - Checked by hand: `alm done` on a task at *locally verified* prints no warning.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-13 | A task finished without the owner's check is noted in its task file, for the owner to search and check later | owner (chat) |
| 2026-10-13 | Drop `label_below_top`; the rule tells the agent to reach the top label first; a task in an automatic/loop run may be finished without the owner's check | owner (chat) |
