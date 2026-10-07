# Task-A01-10-file-new-window

*Card type: coding. Area A01 (Tracking), Iteration 1 or 2. Requested by the owner while checking [`Task-A01-04`](./Task-A01-04-startup-folder-argument.md) (2026-10-07).*

## What gets built (plain words)

Today FlipHopper has one window, and it always reopens the last folder. After this card, **File → New Window** (⌘N on macOS, Ctrl+N on Windows) opens another FlipHopper window with **no folder open**, ready for the user to choose one. Each window then shows its own seabag, so two projects can be open side by side.

| Piece | What it is | Where |
|---|---|---|
| File menu | A **File** menu with **New Window** (⌘N / Ctrl+N), in the app's language where the system allows. The usual app, Edit and Window menus stay. | `apps/desktop/src-tauri/src/lib.rs` |
| A new, empty window | Opens the same interface in a new window that does **not** reopen the last folder and opens no start-up folder; it shows the "choose a folder" screen. | `lib.rs`, `apps/web/src/App.tsx` (how a window knows it is a new, empty one) |
| Remembering the last folder | Each window still saves the folder it opened, so the next app start reopens the most recent one. A new window never opens it. | `apps/web/src/App.tsx` |

**What you can see at the end:** with a project open, choose File → New Window (or press ⌘N). A second window appears with no folder open; choose another folder there. Both windows stay on their own projects, and each follows `fh` changes to its own seabag.

## Details for the worker

- **Outcome.** A person can open a second, empty FlipHopper window from the File menu and work on two projects at once.
- **Decisive unknowns** (owner; the worker proposes defaults on the card before building):
  1. Should the File menu also get **Open Folder…** (⌘O), which chooses a folder for the current window? Recommended: yes, it is the same menu and a small step.
  2. On macOS, does closing the last window quit the app, or keep it running in the Dock? Recommended: quit, as today.
  3. Should a folder already open in another window be refused, or simply opened twice? Recommended: bring that window to the front instead.
- **Files owned.** `apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src-tauri/capabilities/*` (if the new window needs a permission), `apps/desktop/src-tauri/tauri.conf.json` (if needed), `apps/web/src/App.tsx`, `apps/web/src/messenger.ts`, `apps/web/src/i18n/*` (menu words), this card.
- **External operations.** None.
- **Done means mechanically.**
  - These pass: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm --prefix apps/web run typecheck`, `npm --prefix apps/web run build`, `npm --prefix apps/web run check:i18n`.
  - In the built app: File → New Window and ⌘N each open an empty window; choosing a folder there opens it; the first window keeps its project; `fh` changes show in the right window; quitting and restarting reopens the most recent folder in one window. Recorded on the card; then the owner checks.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-07 | "add a new feature as a new task card: allow user choose 'file->new' to open a new window with no folder opened" (card requested; building waits on the owner's review) | owner (chat) |

## Notes

- 2026-10-07 — Card written by the agent from the owner's request.
