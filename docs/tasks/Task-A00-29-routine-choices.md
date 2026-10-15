# Task-A00-29-routine-choices

*Card type: docs (with `alm`'s own texts).*

## What gets built (plain words)

Agents stop asking the approver about small things they can decide themselves, and a loop that runs without the person never sits waiting for an answer.

| Piece | What it is in everyday words | Where it lives |
|---|---|---|
| The dividing line | Which choices the agent makes itself (names, file layout, how something is built inside the task, small details with an obvious default) and which go to the approver (what users will see and get used to, anything hard to undo, changes to the plan or another task, cost or outside effects) | `AGENTS.md` § Always-on boundaries; `docs/workflow/CARDS.md` § Routine choices and questions for the approver; `alm brief`; `alm guide workflow` |
| The card note | How a routine choice is written on the card: `Decided: <choice> (<reason>).`, plus `alm ai-decided` so the approver sees it under To review | `CARDS.md` (and its card template) |
| Open questions in new cards | Whoever writes a card marks only real questions for the person as open, not routine choices | `alm guide tasks`, step 5 (Notes); `CARDS.md` |
| The quiet-run rule | In an automatic or loop run that goes on without the person: decide routine choices and record them; park a real question with `alm wait <id> --on "<the question>"` and take the next ready task; never `alm confirm` (it would wait until it times out). How that mode is chosen is task A03-21, not built here | `alm brief` (beside the `--without-human-check` rule), `alm guide workflow` (step 8), `AGENTS.md`, `CARDS.md` |

## Details for the worker

- **Outcome.** An agent decides routine choices inside a task and notes them; only real product questions reach the approver. In a quiet run it never waits for the person.
- **Decisive unknowns.** None.
- **Files owned.** `AGENTS.md` (one always-on line; the Allomos section regenerated from `alm brief`), `docs/workflow/CARDS.md`, `crates/store/src/lib.rs` (`agent_section`), `crates/store/src/guide.rs` (tasks, workflow, tests), `crates/store/src/versions.rs` (`SECTION_VERSION` 8 → 9), `crates/cli/tests/alm.rs` (version test), this card.
- **External operations.** None.
- **Done means mechanically.** `alm show A00-29` "Done means"; documentation checks (links, paths, consistency, `git diff --check`); every named check in `docs/workflow/CODING_RULES.md` passes.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-14 | The rule: agents decide routine choices and record them; only real product questions go to the owner (origin: A01-37's loop asked about `alm` command names) | owner (chat) |
| 2026-10-15 | Extend it to loop runs: decide instead of asking, park real questions | owner (chat) |

## Notes

- 2026-10-15 — Decided: the quiet-run rule goes in the brief as a continuation of the "Finished" bullet, right after the `--without-human-check` loop rule, and in `alm guide workflow` as a new step 8 (the card asks for it beside that rule; a separate bullet would repeat "automatic or loop run").
- 2026-10-15 — Decided: `alm`'s own texts call it "a quiet run" and do not name task A03-21 (those texts serve every project using Allomos; this repository's task ID appears only in `AGENTS.md`, `CARDS.md` and the `SECTION_VERSION` comment).
- 2026-10-15 — Decided: `alm guide tasks` step 5 (Notes) is the right place for "what to mark as open for the person"; it is where a new task's open questions are written.
- 2026-10-15 — Decided: the existing stale section-version examples in `docs/product/FILE-FORMAT.md` (v4) and `docs/workflow/CODING_RULES.md` (v6) are left as they are (outside this card's outcome).
- 2026-10-15 — Decided: the new guide tests compare text with line breaks flattened, so rewrapping a paragraph does not break them.
- 2026-10-15 — The four choices above that touch the texts are also recorded with `alm ai-decided A00-29` (taskbag events pushed, `2eb343e`), so they show in `alm human` under To review: AI decisions.
- 2026-10-15 — **Written (claude/a00-29-routine-choices), locally verified.** In plain words: `AGENTS.md` now has an always-on line saying which choices an agent makes itself and which go to the approver, with the card-note format and the quiet-run rule; `CARDS.md` has the same in a new section, *Routine choices and questions for the approver*, and its card template mentions `Decided:` lines. In `alm`'s own texts (so every project gets them): `alm brief` (section version 8 → 9) adds the quiet-run rule right after the `--without-human-check` rule and the full dividing line to the routine-choices bullet; `alm guide workflow` gains step 8 (the quiet-run rule) and says `alm confirm` is not for a quiet run; `alm guide tasks` step 5 (Notes) says what to mark as open for the person. This repository's `AGENTS.md` section was regenerated from the new `alm brief`. Evidence: `cargo fmt --check` PASS; `cargo clippy --workspace --all-targets -- -D warnings` PASS; `cargo test --workspace` PASS (195 passed, 0 failed; new guide test for the quiet-run rule and the tasks note; the version test now expects v9 and treats v8 as out of date); `npm --prefix apps/web run typecheck` PASS, `build` PASS, `check:i18n` PASS, `test` PASS; `git diff --check` PASS; links checked (`CARDS.md#routine-choices-and-questions-for-the-approver`, `AGENTS.md#always-on-boundaries`, this card). origin/main had not moved since 7d5b46f. Next: the approver reads the rule and the AI decisions (human check); then A03-21 can be unblocked.
