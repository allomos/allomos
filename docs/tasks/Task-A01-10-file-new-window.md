# Task-A01-10-file-new-window

*Card type: coding. Area A01 (Tracking), Iteration 1. Requested by the owner while checking [`Task-A01-04`](./Task-A01-04-startup-folder-argument.md) (2026-10-07). The file name keeps the first idea (a new window); the owner then chose **File → Open Folder…** instead, and no new window for now.*

## What gets built (plain words)

Today the only way to switch projects is the "choose a folder" button. After this card, the app has a **File** menu with **Open Folder…** (⌘O on macOS, Ctrl+O on Windows). It asks for a folder and opens it in the window, exactly like the button: an existing seabag opens, an empty folder gets one, and a folder with other files asks first.

| Piece | What it is | Where |
|---|---|---|
| File menu | A **File** menu with **Open Folder…** (⌘O / Ctrl+O). The usual app, Edit and Window menus stay. Its words follow the app's language. | `apps/desktop/src-tauri/src/lib.rs`, `apps/web/src/i18n/*` |
| What the menu does | The menu tells the interface to run the same "choose a folder" step as the button. | `lib.rs`, `apps/web/src/messenger.ts`, `apps/web/src/App.tsx` |
| Closing the window | Closing the last window quits the app on macOS, as today. | unchanged; checked |

**What you can see at the end:** with a project open, press ⌘O (or choose File → Open Folder…), pick another folder, and the window shows that project.

## Details for the worker

- **Outcome.** A person switches the window to another project from the File menu or with ⌘O.
- **Owner's answers (2026-10-07).** Open Folder…: yes. File → New Window: **not for now**. Closing the last window quits: yes. A folder already open elsewhere is brought to the front: yes, which matters only once there are several windows, so it waits with New Window.
- **Decisive unknowns.** Whether the menu words can change when the user switches the app's language without restarting (Tauri menus are built in Rust). If not, the menu follows the language chosen at start; the worker notes which.
- **Files owned.** `apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src-tauri/capabilities/*` (if the menu event needs a permission), `apps/web/src/App.tsx`, `apps/web/src/messenger.ts`, `apps/web/src/i18n/*`, this card.
- **External operations.** None.
- **Done means mechanically.**
  - These pass: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm --prefix apps/web run typecheck`, `npm --prefix apps/web run build`, `npm --prefix apps/web run check:i18n`.
  - In the built app: File → Open Folder… and ⌘O each ask for a folder and open it; cancelling changes nothing; closing the window quits. Recorded on the card; then the owner checks.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-07 | "add a new feature as a new task card: allow user choose 'file->new' to open a new window with no folder opened" | owner (chat) |
| 2026-10-07 | Card reviewed: Open Folder… (⌘O) yes, "then we need no 'file -> new window' menu for now"; closing the last window quits: yes; bring an already-open folder's window to the front: yes (later) | owner (chat) |

## Notes

- 2026-10-07 — Card written by the agent from the owner's request, then narrowed to Open Folder… after the owner's answers.
- 2026-10-07 — Built in the card run (`claude/a01-run`).

## Result (2026-10-07)

**In plain words.** The app's File menu now starts with **Open Folder…** (⌘O on macOS, Ctrl+O on Windows), above the system's Close Window. Choosing it, or pressing the shortcut, runs the same "choose a folder" step as the button in the window. The menu's words follow the app's language: the interface sends them to the Rust side (`set_menu_texts`) at start and whenever the language changes, so no restart is needed. Closing the window still quits the app.

- **Decisive unknown answered:** menu words can change while the app runs (`set_text` on the menu items), so they follow a language change at once.
- **Where:** `build_menu`, `set_menu_texts` and the menu event in `apps/desktop/src-tauri/src/lib.rs`; `onOpenFolderMenu` and `setMenuTexts` in `messenger.ts`; two `useEffect`s in `App.tsx`; `menu.file` and `menu.open_folder` in three languages. No new package or permission.

| Check | Result |
|---|---|
| `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` | PASS |
| `npm --prefix apps/web run typecheck`, `build`, `check:i18n` (76 texts) | PASS |
| Built app: menu bar is FlipHopper, File, Edit, View, Window, Help; File holds Open Folder… (shortcut ⌘O), Close Window, Close All (read through macOS accessibility) | PASS |
| Built app: File → Open Folder… and ⌘O each open the folder dialog (the system's dialog service starts on the click and the key press); Escape cancels | PASS |
| Menu words in Chinese and Japanese after switching the app's language | not seen: the agent could not drive the language list — owner check |
| Picking a folder from the menu's dialog opens it | not seen (the dialog is drawn by a system process the agent cannot drive) — owner check |

**Owner check:** in the app, press ⌘O, pick another folder: the window shows it. Switch the app's language to 日本語: the menu reads ファイル → フォルダーを開く….
