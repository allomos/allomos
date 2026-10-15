# Task-A01-43-open-recent

*Card type: coding. Area A01 (Tracking), Iteration I02. Requested by the owner 2026-10-15; the dialog version decided the same day after mockups in chat.*

## What gets built (plain words)

The start page keeps its white card exactly as it is. Under it, at the card's left edge, is an **Open recent…** button. It opens a small window (a dialog) titled **Open recent** that lists the taskbags opened recently on this computer, newest first: the project's name, its folder, and when it was last opened. Clicking one opens it in this window. Pointing at one (or reaching it with Tab) shows an **×** that takes it off the recent list at once, without asking; nothing on disk changes, and File › Open Recent drops it too. A folder that no longer exists is greyed with "folder not found": it can be removed but not opened. With no recent taskbags the dialog says "No recent taskbags." The dialog closes with its own ×, Esc, or a click outside it.

The list is the one File › Open Recent already uses ([`A01-11`](./Task-A01-11-recent-folders-menu.md)), kept in the window's storage. What it did not keep was each project's name and when it was opened, so Allomos now notes both whenever a taskbag opens (`allomos.recentInfo`, only for folders still on the list). Folders opened before this change show their folder's name and no time until they are opened again.

## Details for the worker

- **Outcome.** From the start page, a person reopens or tidies a recent taskbag in two clicks.
- **Choices made by the agent** (owner rule A00-29; the owner may change them on review):
  - The button stays visible when the list is empty, so the empty dialog ("No recent taskbags.") can be seen and the start page does not change shape.
  - A missing folder's line cannot be clicked (it is greyed); only its × works. So opening it never shows an error. Removing is left to the person, as A01-28 decided for the menu.
  - Missing folders are checked each time the dialog opens (one quick look per folder, at most 10).
  - The time is shown in the interface's language with the system's date format (`Intl.DateTimeFormat`, medium date, short time), so it needs no translated words.
  - × is always focusable (only invisible until hover or focus), so the keyboard reaches it; Esc closes; the focus goes back to the button.
  - Words: "Open recent…" / 打开最近的… / 最近使った項目を開く…; the dialog title follows the menu's words in Japanese.
- **Files owned.** `apps/web/src/recentFlow.ts` (the list's decisions, no UI), `apps/web/tests/recentFlow.test.ts`, `apps/web/src/RecentDialog.tsx`, `apps/web/src/App.tsx` (the start page button, the notes on opening, one shared "open a recent folder"), `apps/web/src/styles.css`, `apps/web/src/i18n/{en,zh-CN,ja}.json`, this card. No Rust change; no new package.
- **External operations.** None.
- **Done means mechanically.**
  - These pass: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm --prefix apps/web run typecheck`, `build`, `check:i18n`, `test`.
  - Debug build: the button under the card; the dialog lists recent taskbags with name, folder and time; click opens; × removes from the dialog and from File › Open Recent; a deleted folder is greyed "folder not found"; empty state; closes with ×, Esc and a click outside; in English, Chinese and Japanese. Windows untested.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-15 | A list of recent taskbags on the start page, then changed to: a button under the card opening an "Open recent" dialog (mockups checked in chat) | owner (chat) |
| 2026-10-15 | Card approved; routine choices are the building agent's (A00-29) | owner (chat) |

## Notes

- 2026-10-15 — Built and locally verified by `claude/a01-43-open-recent`. Next: the owner tries it in the debug build.

  Evidence: `cargo fmt --check` PASS; `cargo clippy --workspace --all-targets -- -D warnings` PASS; `cargo test --workspace` PASS; `npm --prefix apps/web run typecheck` PASS; `build` PASS; `check:i18n` PASS (374 texts); `npm --prefix apps/web test` PASS (86, of which 5 new in `recentFlow.test.ts`). Debug app built: `target/debug/bundle/macos/Allomos.app`. Test taskbags: `~/allomos-tests/a01-43-open-recent/{bakery,garden,deleted-later}`. NOT RUN by the agent: looking at the window (the screen tool reaches only the installed Allomos, which shares the app ID). Windows: untested. Note: the debug build shares the window's storage with the installed Allomos, so the test folders appear in its recent list too; remove them with × after the check.
