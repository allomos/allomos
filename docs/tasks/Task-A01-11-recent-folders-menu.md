# Task-A01-11-recent-folders-menu

*Card type: coding. Area A01 (Tracking), Iteration 1. Requested by the owner after checking [`Task-A01-10`](./Task-A01-10-file-new-window.md) (2026-10-07).*

## What gets built (plain words)

The File menu gets **Open Recent**: a list of the folders opened most recently, newest first. Choosing one opens it in the window, as if it had been picked with Open Folder…. **Clear Menu** at the bottom empties the list.

| Piece | What it is | Where |
|---|---|---|
| The list | Up to **10** folders, newest first, without repeats. A folder joins the list each time it opens (picked, from the menu, or given at start). Kept on this computer only, next to the "last folder" the app already remembers. | `apps/web/src/App.tsx` |
| The submenu | File → **Open Recent** → one item per folder, showing the project folder's path (not the hidden `.fliphopper/seabag` part), then **Clear Menu**. Empty list: the submenu shows a greyed "No recent folders". Words follow the app's language. | `apps/desktop/src-tauri/src/lib.rs`, `apps/web/src/i18n/*` |
| Opening one | The menu tells the interface which folder; the interface opens it like any other. A folder that is gone shows the usual "not a folder" message and leaves the list. | `lib.rs`, `messenger.ts`, `App.tsx` |

**What you can see at the end:** open two or three folders; File → Open Recent lists them, newest first; choosing one opens it; Clear Menu empties the list.

## Details for the worker

- **Outcome.** A person goes back to a project they used recently in two clicks.
- **Defaults chosen by the agent** (the owner may change them on review): 10 entries; newest first; Clear Menu included; a missing folder leaves the list when chosen.
- **Decisive unknowns.** None.
- **Files owned.** `apps/desktop/src-tauri/src/lib.rs`, `apps/web/src/App.tsx`, `apps/web/src/messenger.ts`, `apps/web/src/i18n/*`, this card.
- **External operations.** None.
- **Done means mechanically.**
  - These pass: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm --prefix apps/web run typecheck`, `npm --prefix apps/web run build`, `npm --prefix apps/web run check:i18n`.
  - Built app: the submenu lists opened folders newest first; choosing one opens it; Clear Menu empties it; the words follow the language. Recorded on the card; then the owner checks.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-07 | "also need a new feature: the recently opened folders list in the file menu" | owner (chat) |

## Notes

- 2026-10-07 — Card written by the agent from the owner's request, with the defaults above, and worked in the card run (`claude/a01-run`).

## Result (2026-10-07)

**In plain words.** File → **Open Recent** lists up to 10 recently opened project folders, newest first, then **Clear Menu**; with no folders it shows a greyed "No recent folders". A folder joins the top of the list whenever it opens. Choosing one opens it like Open Folder…; a folder that is gone shows the usual message and leaves the list. The list is kept in the app's own storage on this computer; the interface sends it to the Rust side, which rebuilds the submenu. All its words follow the app's language.

- **Where:** `fill_recent`, `set_recent_folders`, the extended `set_menu_texts` and the menu events in `apps/desktop/src-tauri/src/lib.rs`; `onOpenRecentMenu`, `onClearRecentMenu`, `setRecentFolders` in `messenger.ts`; the list in `App.tsx`; three new texts in three languages. No new package or permission.

| Check | Result |
|---|---|
| `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` | PASS |
| `npm --prefix apps/web run typecheck`, `build`, `check:i18n` (79 texts) | PASS |
| Built app (in Japanese): ファイル holds フォルダーを開く…, 最近使った項目を開く, Close Window, Close All | PASS |
| Opened this repository, then `~/fliphopper-check`: the list shows `~/fliphopper-check` first, then this repository | PASS |
| Chose this repository from the list: the window showed it, and it moved to the top | PASS |
| Clear Menu: the submenu shows only 最近使ったフォルダーはありません, greyed | PASS |
| A folder that no longer exists leaves the list when chosen | not tried (code path only) |

**Owner check:** File → Open Recent shows the folders you opened, newest first; choosing one opens it; Clear Menu empties the list.
