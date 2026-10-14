# Task-A01-37-describe-and-note

*Card type: coding. Area A01 (Tracking), Iteration 2. Written 2026-10-14. Follows [`A01-18`](./Task-A01-18-edit-task-in-app.md), which added these two changes to the app's task pane and said matching `alm` commands would be their own card; needs [`A01-38`](./Task-A01-38-no-git-in-alm.md) first (no `--push`; `alm` prints the Git commands).*

## What gets built (plain words)

Until now an agent could not change a task's **description** (the text in its `description.md`) or its **handoff note** (the short "what the next one needs" line) with `alm`: it had to edit the taskbag's files by hand, which skips the rules and leaves no trace in the task's history. On 2026-10-14 that happened while recording the owner's answers on A01-36. After this card, two `alm` commands make those changes, with exactly the rules the app's task pane uses.

| Piece | What it is | Where |
|---|---|---|
| `alm describe <id> --text "…"` / `--from <file.md>` | A new description; an empty text (`--text ""`) removes `description.md` | `crates/cli/src/lib.rs` |
| `alm note <id> "…"` | A new handoff note; no text clears it | `crates/cli/src/lib.rs` |
| The same rules as the app | Both call `set_description` / `set_handoff_note` in `crates/core/src/ops.rs` through the shared locked writer (`allomos_store::change_task`): same holder rule, same refusals, one history event each, named `describe` / `note` like the app's | `crates/core` (unchanged), `crates/store` (unchanged) |
| Git | As every change since A01-38: `alm` prints the `git add / commit / push` commands naming only the files the change wrote | `crates/cli` |
| Texts | `alm --help`, `alm guide commands`, `alm brief` (section version 6, this repository's AGENTS.md renewed from it), `alm task-language` help, CODING_RULES `alm` table | `crates/cli`, `crates/store/src/{guide,lib,versions}.rs`, `AGENTS.md`, `docs/workflow/CODING_RULES.md` |

**What you can see at the end:** `alm --as me describe A01-01 --text "New words."` rewrites the task's description, prints the Git commands naming only `description.md` and one `events/…` file, and `alm history A01-01` lists a `describe` change; `alm --as someone-else note A01-01 "x"` is refused ("held by me") and changes nothing.

## Details for the worker

- **Outcome.** Agents change a task's description and handoff note without touching the taskbag's files by hand, under the app's rules, recorded in history.
- **Decisive unknowns.** None: the names were decided (see Approvals and Choices).
- **Choices.** Names `alm describe <id> --text "…" | --from <file.md>` and `alm note <id> "…"`: they follow `alm`'s one-verb-per-change style (`alm needs`, `alm title`), and match the history event names `describe` / `note` the app already writes, so `alm history` reads the same whoever made the change. `alm note` with no text clears the note, as `alm needs` does.
- **Files owned.** `crates/cli/src/lib.rs`, `crates/cli/tests/alm.rs`, `crates/store/src/guide.rs`, `crates/store/src/lib.rs` (the brief), `crates/store/src/versions.rs`, `docs/workflow/CODING_RULES.md`, `AGENTS.md` (the Allomos section only), this card.
- **External operations.** None.
- **Done means mechanically.**
  - The named checks pass (`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm --prefix apps/web run typecheck`, `build`, `check:i18n`, `test`).
  - Tests: description changed from text and from a file; cleared (file removed); handoff note changed and cleared; refused when another session holds the task (no file changed); history shows `describe` / `note`; the printed Git commands list exactly the changed files.

## Approvals

| What | By | When | Source |
|---|---|---|---|
| Command names `alm describe <id> --text "…"` or `--from <file.md>`, and `alm note <id> "…"` | owner | 2026-10-14 | chat (recorded in the task's next dependency) |
| No `--push`: print the Git commands as A01-38 does | owner | 2026-10-14 | chat (same) |

## Progress notes

- 2026-10-14 — **Built and locally verified** (session claude/a01-37-describe-note).

  **In plain words.** Two new `alm` commands. `alm describe` replaces a task's description with the text given (`--text`) or the contents of a Markdown file (`--from`); exactly one of the two is required, and an empty text removes `description.md`. `alm note` sets the task's handoff note, or clears it when no text is given. Neither has rules of its own: each hands the change to the same rule the app's task pane uses, under the same lock, so a task held by another session is refused with the same message, the change is written as one history event named `describe` or `note` (as the app names them), and, like every change since A01-38, `alm` prints the Git commands naming only the files it wrote. The brief that goes into AGENTS.md gained one line listing `alm title`, `alm describe` and `alm note`, so its version went from 5 to 6, and this repository's AGENTS.md section was renewed from `alm brief`.

  **Evidence.** `cargo fmt --check` PASS; `cargo clippy --workspace --all-targets -- -D warnings` PASS; `cargo test --workspace` PASS (cli 2 + 44, core 40, store 64 + 13 integration, desktop 24 + 1 ignored); `npm --prefix apps/web run typecheck` PASS, `build` PASS, `check:i18n` PASS (338 texts, 69 codes), `test` PASS (64). New test `describe_and_note_change_a_task_like_the_app` (text with trailing spaces trimmed, from a file, emptied → `description.md` gone and named; note set and cleared; history `note, note, describe, describe, describe` by `a`; refused for another holder, no holder, unknown task, missing / both options, unreadable file, with every taskbag file compared before and after; human output names `task.json` and not `description.md`). `each_change_names_exactly_the_files_it_wrote` now also runs `note`, `describe --text`, `describe --from`, `describe --text ""`. No GitHub workflow was started.

  **Human check (steps).** In any test taskbag with the branch's `target/debug/alm`: `alm --as me claim A01-01`, then `alm --as me describe A01-01 --text "New words."` and `alm --as me note A01-01 "Next: tests"`; read the printed Git commands (only that task's files and one event each), then `alm history A01-01` (lists `note` and `describe`) and `alm --as other note A01-01 x` (refused).
