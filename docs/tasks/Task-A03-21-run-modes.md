# Task-A03-21-run-modes

*Card type: coding (`alm`'s own texts and one guide topic).*

## What gets built (plain words)

Before an agent starts an automatic or loop run (a card run, `/loop`, several tasks one after another), it asks the person how it should work: **quiet mode** (it goes on alone, decides routine choices itself and parks real questions) or **normal mode** (it stops and asks as usual). Every agent shows the person the same explanation, which `alm` itself prints in the person's language, and writes the choice in its handoff.

| Piece | What it is in everyday words | Where it lives |
|---|---|---|
| The explanation | For each mode: what the agent will and won't do, when the person is interrupted, how long it can run alone. Quiet mode ends with what to do afterwards: `alm human` (To answer → answer, then `alm ready`; To review: AI decisions → keep or overturn, then `alm reviewed`; To check → follow the steps, then `alm checked`) and read the run's handoff. Normal mode asks the person to stay reachable | `alm guide run-modes` (`crates/store/src/guide.rs`), in English, Japanese and Simplified Chinese |
| The language | Printed in `--language`, else the taskbag's task language, else the app's language; any other language gets the English text with a first line asking the agent to translate it faithfully | `crates/cli/src/lib.rs` (`alm guide`) |
| The rule for agents | Ask before the run, show the explanation as it is, do not start until the person chooses, write the choice in the handoff and follow it | `alm brief` (section v10), `alm guide workflow` step 8, `AGENTS.md`, `docs/workflow/CARDS.md` |

## Details for the worker

- **Outcome.** Before an automatic or loop run, the agent shows the person `alm`'s explanation of quiet and normal mode, in the task language, and starts only once the person has chosen; the choice is in the handoff.
- **Decisive unknowns.** None.
- **Files owned.** `crates/store/src/guide.rs` (run-modes texts, workflow step 8, tests), `crates/store/src/lib.rs` (`agent_section`, exports), `crates/store/src/versions.rs` (`SECTION_VERSION` 9 → 10), `crates/cli/src/lib.rs` (`alm guide --language`), `crates/cli/tests/alm.rs`, `AGENTS.md` (one always-on line; the Allomos section regenerated from `alm brief`), `docs/workflow/CARDS.md`, `docs/workflow/CODING_RULES.md` (the `alm guide` row), this card.
- **External operations.** None.
- **Done means mechanically.** `alm show A03-21` "Done means"; a test that the run-modes text exists in every language the app speaks (en, ja, zh-CN) with both modes and the after-run commands; every named check in `docs/workflow/CODING_RULES.md` passes; `alm list --json` reports no issues; `git diff --check`.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-15 | The task: agents ask for quiet or normal mode before an automatic or loop run, with explanations `alm` provides | owner (chat, task A03-21) |

## Notes

- 2026-10-15 — Decided: the explanation is a new guide topic, `alm guide run-modes [--language <code>]`, printed in the task language; `alm guide` with no topic includes its English text (one place for every agent to get the same words; the other topics stay English because only agents read them).
- 2026-10-15 — Decided: it is written in en, ja and zh-CN, the languages the app speaks (`pick_ui_language`); any other task language (for example `fr`, `zh-TW`) gets the English text with a first line asking the agent for a faithful translation, and `--json` says `written_in_language: false` (no machine translation inside `alm`).
- 2026-10-15 — Decided: the text has a short note for the agent above a `---` line, and the part for the person below it, written as the agent speaking ("I"), so the agent can show it as it is.
- 2026-10-15 — Decided: the choice is written in the handoff under the template's existing *Approvals in force* line, as `Run mode: quiet` or `Run mode: normal` with who chose it and when; the handoff template is not changed (changing every project's template is the approver's call).
- 2026-10-15 — Decided: for a task that fails its check, the text says to write what is wrong with `alm note <id> "…"` and add a task for the fix, because `alm` cannot reopen a finished task (the task's description says "reopen it with a note"). A reopen command would be its own task; for the approver to decide.
- 2026-10-15 — The choices above are also recorded with `alm ai-decided A03-21`, so they show in `alm human` under To review: AI decisions.
- 2026-10-15 — **Written (claude/a03-21-run-modes), locally verified.** In plain words: `alm guide run-modes` now prints the explanation of quiet and normal mode for the person, in English, Japanese or Simplified Chinese depending on the task language (other languages: English plus a request to translate). `alm brief` (section version 9 → 10) and `alm guide workflow` (step 8) tell the agent to show it before an automatic or loop run, wait for the choice, and write it in the handoff; this repository's `AGENTS.md` section was regenerated from the new `alm brief`, and its always-on line and `CARDS.md` (Routine choices; Claiming protocol step 6) say the same. Evidence: `cargo fmt --check` PASS; `cargo clippy --workspace --all-targets -- -D warnings` PASS; `cargo test --workspace` PASS (197 passed, 0 failed, 1 ignored; new `run_modes_is_written_in_every_language` checks en, ja and zh-CN each carry both modes, the four points per mode, and the after-run commands; new CLI test `run_modes_follow_the_task_language`; the version test now expects v10 and treats v9 as out of date); `npm --prefix apps/web run typecheck` PASS, `build` PASS, `check:i18n` PASS, `test` PASS; `git diff --check` PASS; `alm list --json` reports no issues; five `alm ai-decided A03-21` entries pushed to the taskbag (`0d9d9d4`…`2a2f09e`). origin/main had not moved since c37036b. Next: the approver reads the explanation (`alm guide run-modes --language ja`, `zh-CN`, `en`) and the AI decisions (human check).
