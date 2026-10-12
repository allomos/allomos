# Task-A01-27-new-window

*Card type: coding. Area A01 (Tracking). Written 2026-10-12 at the owner's request while checking [`A01-25`](./Task-A01-25-taskbag-format-upgrade.md): "change the 'Open another folder...' and the file menu's 'open folder' item to 'New window...', which will open a new window and show the start page of the app."*

## What gets built (plain words)

Today Allomos has one window. **Open another folder…** (the dark button at the top right) and **File → Open Folder…** (⌘O) replace the project shown in that window. After this card both become **New window…**: it opens a second Allomos window showing the start page, where the person chooses a folder, and the first window keeps its project. Each window shows its own project.

| Piece | What it is in everyday words | Where it lives |
|---|---|---|
| New window | **New window…** in the top bar and in the File menu opens another window on the start page | `apps/desktop` (Tauri windows, menu), `apps/web` |
| One project per window | Each window opens, follows and checks its own project (task list, agent line, update notice, `alm confirm` popups for its project) | `apps/web`, `apps/desktop` |
| Texts | "New window…" in en, zh-CN (新窗口…) and ja (新しいウインドウ…) (R94) | language files |

At the end the owner has one project open, chooses **New window…**, picks another project in the new window, and sees both projects side by side.

## Details for the worker

- **Outcome.** **New window…** (top bar and File menu) opens a new window on the start page; each window shows its own project.
- **Decisive unknowns** (owner to decide).
  - The keyboard shortcut: ⌘N (the usual one for a new window), or keep ⌘O.
  - **File → Open Recent**: opens the folder in the current window (as now), or in a new window.
  - When the app starts: reopen only the last project (as now), or every window that was open.
  - Opening a project that another window already shows: bring that window forward, or open it twice.
  - The start page's **Choose a folder…** in a window that already shows a project (after Cancel goes back to the start page) stays as it is.
- **Files owned.** Window creation and the File menu in `apps/desktop/src-tauri`; the top bar and per-window state in `apps/web/src/App.tsx`; language files; [OPEN-FOLDER-FLOW.md](../product/OPEN-FOLDER-FLOW.md) and the "Running the app" part of [CODING_RULES](../workflow/CODING_RULES.md); this card.
- **External operations.** None.
- **Done means mechanically.**
  - Both **New window…** entries open a new window on the start page; the first window is unchanged.
  - Two windows show two projects, each following its own changes; an `alm confirm` question appears only in the window showing that project.
  - Typecheck, build and the missing-text test pass; the owner checks it in the app.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-12 | Asked for: "New window…" replaces "Open another folder…" and File → Open Folder… | owner (chat) |

## Notes

- 2026-10-12 — Written as its own card (one outcome per card), not added to A01-25. Next: the owner answers the unknowns and approves the card.
