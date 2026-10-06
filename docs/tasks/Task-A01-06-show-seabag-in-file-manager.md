# Task-A01-06-show-seabag-in-file-manager

*Card type: coding. Area A01 (Tracking), Iteration 1. Requested by the owner while checking [`Task-A01-03`](./Task-A01-03-gb-command.md) (2026-10-06).*

## What gets built (plain words)

While a seabag is open, the top bar has a button that opens the seabag's folder in the computer's file manager: **Show in Finder** on macOS, **Show in Explorer** on Windows. A person can then look at, copy or back up the task files without hunting for the hidden `.fliphopper/` folder.

| Piece | What it is | Where |
|---|---|---|
| The command | A desktop command that opens the open seabag's folder (`.fliphopper/seabag/`) in the file manager. It refuses a path that is not the open seabag, so the window cannot be used to open arbitrary paths. | `apps/desktop/src-tauri/src/lib.rs` |
| The button | In the top bar, next to the seabag's path, shown only while a seabag is open. Its words depend on the system: Finder (macOS), Explorer (Windows), "file manager" elsewhere. | `apps/web/src/App.tsx`, `messenger.ts` |
| Words | The button and any error, in English, Simplified Chinese and Japanese (R94). | `apps/web/src/i18n/*.json` |

**What you can see at the end:** open a seabag, press **Show in Finder**: Finder opens at `.fliphopper/seabag/` showing `project.json`, `areas/` and `tasks/`.

## Details for the worker

- **Outcome.** From the open seabag, one press shows its files in Finder or Explorer.
- **Decisive unknowns.**
  - How to open the folder: the operating system's own command (`open` on macOS, `explorer` on Windows), with no new package, or Tauri's `opener` plugin (a new pinned package and a permission). Recommended: the system command, as it needs no new package.
  - Whether to open the seabag folder itself or the project folder with `.fliphopper` selected. Recommended: the seabag folder itself, because Finder hides `.fliphopper/` by default.
- **Builds on** [`Task-A01-03`](./Task-A01-03-gb-command.md), which changes the same files; start after it merges.
- **Files owned.** `apps/desktop/src-tauri/src/lib.rs` (and `capabilities/` only if a plugin is chosen); `apps/web/src/App.tsx`, `messenger.ts`, `styles.css`; `apps/web/src/i18n/en.json`, `zh-CN.json`, `ja.json`; `crates/core/src/problem.rs` and `apps/web/src/i18n/codes.json` if a new problem code is needed; this card.
- **External operations.** None.
- **Done means mechanically.**
  - These pass: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm --prefix apps/web run typecheck`, `npm --prefix apps/web run build`, `npm --prefix apps/web run check:i18n`.
  - A Rust test: the command refuses a path that is not the open seabag.
  - In the running app, the button opens Finder at the seabag. This is recorded on the card; then the owner checks.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-06 | "Add a feature to allow open current seabag in Finder/Explorer" | owner (chat, while checking A01-03) |

## What was built (2026-10-06)

**In plain words.** While a seabag is open, a small **Show in Finder** link sits under its path in the top bar. It reads **Show in Explorer** on Windows and **Show in file manager** elsewhere. Pressing it opens the seabag folder `.fliphopper/seabag/`.

- **How the folder is opened:** with the computer's own command (`open`, `explorer` or `xdg-open`), so no new package was needed.
- **Only seabag folders are opened:** the desktop app opens a folder only when it is a seabag. Anything else is refused with `not_a_seabag`, and a start failure is reported as `open_failed`. Both are worded in all three languages.

**Checks (2026-10-06):**

| Check | Result |
|---|---|
| `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --workspace` (36), including `file_manager_opens_only_a_seabag` | PASS |
| `npm --prefix apps/web run typecheck` / `build` / `check:i18n` (71 texts, 33 codes) | PASS |
| Pressing the button in the running app | owner check (the agent has no access to the window) |

## Notes

- 2026-10-06 — Card written by the agent from the owner's request. Not started: one card per session, and it follows A01-03.
