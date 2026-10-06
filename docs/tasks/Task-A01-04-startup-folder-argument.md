# Task-A01-04-startup-folder-argument

*Card type: coding. Area A01 (Tracking), Iteration 1. A small fix found while verifying [`Task-A01-02`](./Task-A01-02-ui-languages.md).*

## What gets built (plain words)

FlipHopper can be started with a folder to open, for example `fliphopper ~/my-project` or `open FlipHopper.app --args ~/my-project`. Today it only looks at the **first** thing written after the program name. If anything else comes first, such as a macOS option (`-AppleLanguages (ja)`), it ignores the folder and quietly reopens the last folder used instead. It also stays quiet when the folder given does not exist.

| Piece | What it is | Where |
|---|---|---|
| 2026-10-06 | Card reviewed; go ahead | owner (chat) |
| Finding the folder | Take the first argument that is not an option (does not start with `-`), and skip the value that follows a macOS option such as `-AppleLanguages (ja)` or `-NSDocumentRevisionsDebugMode YES`. `FLIPHOPPER_OPEN` still wins over arguments. | `apps/desktop/src-tauri/src/lib.rs` (`startup_folder`) |
| Saying when the folder is wrong | When a folder was given but does not exist or is not a folder, the window says so (naming the path) instead of silently opening the last folder. The text goes into all three language files, following the [interface-language rules](../workflow/CODING_RULES.md#interface-languages). | `startup_folder`, `crates/core/src/problem.rs` (one new code), `apps/web/src/App.tsx`, `apps/web/src/i18n/*` |
| Tests | Small Rust tests of the argument rule with made-up argument lists. | `apps/desktop/src-tauri` |

**What you can see at the end:** `open FlipHopper.app --args -AppleLanguages '(ja)' ~/some-folder` opens `~/some-folder` in Japanese, and `open FlipHopper.app --args ~/no-such-folder` shows a clear message naming that path.

## Details for the worker

- **Outcome.** Starting FlipHopper with a folder always opens that folder, whatever options come before it, and a wrong folder is reported instead of being silently replaced.
- **Decisive unknowns.**
  - Which arguments macOS itself adds when the app is opened from Finder or `open` on current macOS (older versions added `-psn_…`). The worker checks by printing the argument list once.
  - Whether the A01-03 `fh` command will start the app with a folder. If so, it uses the same rule; the worker notes this on A01-03 rather than changing it.
- **Files owned.** `apps/desktop/src-tauri/src/lib.rs`; `crates/core/src/problem.rs` and `apps/web/src/i18n/codes.json` (one new code); `apps/web/src/App.tsx`; `apps/web/src/i18n/en.json`, `zh-CN.json`, `ja.json`; the "Running the app" lines of `docs/workflow/CODING_RULES.md` if the usage changes; this card.
- **External operations.** None.
- **Done means mechanically.**
  - These pass: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm --prefix apps/web run typecheck`, `npm --prefix apps/web run build`, `npm --prefix apps/web run check:i18n`.
  - Unit tests cover: the folder alone; an option with a value before the folder; an option before a folder that does not exist; no arguments; `FLIPHOPPER_OPEN` set.
  - In the built app, the two commands under "What you can see at the end" behave as described. Recorded on the card.

## Approvals

| Date | What was approved | By |
|---|---|---|

## Notes

- 2026-10-06 — Card written by the agent for the owner's review. Found while checking A01-02: `open Glidebox.app --args -AppleLanguages '(ja)' <folder>` reopened the previous folder instead of `<folder>`.
