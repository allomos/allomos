# Task-A01-41-code-folder-links

*Card type: coding. Area A01 (Tracking). Written 2026-10-14. Builds on [`A01-16`](./Task-A01-16-task-pane.md) (links in a task's description). Task text: `alm show A01-41`.*

## What gets built (plain words)

Allomos's own taskbag lives in its own folder (`allomos-taskbag/`) beside the code (`allomos/`), and every task description starts with a link to its card in the code folder. Clicking such a link was refused ("Allomos opens only web links and text files (.md, .txt) inside this project"). The link was right, but the app counted only the taskbag's folder as "the project". After this card the code folder recorded as `project_folder` in the taskbag's `project.json` counts too, so the card opens in the editor.

| Piece | Where |
|---|---|
| The check of a description link (`resolve_task_link`) also accepts files inside the recorded code folder | `crates/store/src/lib.rs` |
| Test case: a text file beside the taskbag opens once its folder is recorded | same file, `task_links_open_only_text_files_in_the_project_and_web_pages` |

## Details for the worker

- **Outcome.** A card link in a task description opens when the taskbag is kept beside the code.
- **Choices.** Links stay relative, as written by agents today; making them absolute would not help (absolute paths are refused on purpose, and the target would still be outside the taskbag's folder). Everything else stays refused: other schemes, absolute paths, files that are not `.md`/`.markdown`/`.txt`, files outside both folders.
- **Files owned.** `crates/store/src/lib.rs` (`resolve_task_link` and its test); this card.
- **External operations.** None.
- **Done means mechanically.** `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` pass.

## Approvals

| What | By | When | Source |
|---|---|---|---|
| Open the card and do the work | approver | 2026-10-14 | chat with session claude/a01-41-code-folder-links |

## Progress notes

- 2026-10-14 — **Built and locally verified** (session claude/a01-41-code-folder-links).

  **In plain words.** A description link is still read from the task's folder, as before. It is now accepted when it leads to a text file inside either the taskbag's folder or the code folder recorded in `project.json` (`project_folder`). For this project that is `../allomos`, so the card links in every task now open. Links that lead anywhere else, to a non-text file, to another scheme or to an absolute path are refused as before.

  **Evidence.** `cargo fmt --check` PASS; `cargo clippy --workspace --all-targets -- -D warnings` PASS; `cargo test --workspace` PASS (cli 2 + 44, core 40, store 64 + 4 + 4 + 5, desktop 25 + 1 ignored). Against the real taskbag (a scratch program calling `resolve_task_link`): the A07-04 card link → `Ok(File(".../allomos/docs/tasks/Task-A07-04-v0-2-exit-check.md"))`; `../../../../../allomos/Cargo.toml` → `LinkRefused`. Clicking in the app: NOT RUN by the agent; human check below.

  **Human check (steps).** Open the debug app (`target/debug/bundle/macos/Allomos.app`) on `../allomos-taskbag`, select A07-04, click the card link at the top of its description: the card opens in your editor.

- 2026-10-14 — **Human verified** by the approver: in the debug app, the A07-04 card link opened the card. Task done.
