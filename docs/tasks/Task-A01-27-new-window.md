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
- **Decided (owner, 2026-10-12).**
  - **Shortcut ⌘N** for New window…; **⌘O is removed** (there is no Open Folder item any more).
  - **File → Open Recent** opens the folder in the **current window**, as now.
  - **When the app starts**, it reopens **only the last project**, as now.
  - **A project another window already shows**: that window is **brought forward**; it is not opened twice.
  - The start page's **Choose a folder…** stays as it is.
- **Queued** after [`A03-06`](./Task-A03-06-agent-handoff-rules.md) in iteration I02 (owner, 2026-10-12).
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
| 2026-10-12 | ⌘N ("use this"); ⌘O removed ("remove this"); Open Recent in the current window; reopen only the last project; bring an already-open project's window forward; queued after A03-06 in this iteration | owner (chat) |

## Notes

- 2026-10-12 — Written as its own card (one outcome per card), not added to A01-25. The owner answered every unknown the same day and queued it after A03-06.
- 2026-10-12 — Built on `claude/a01-27-new-window`: File → New Window… (⌘N) and the top bar's **New window…** open a window on the start page (`index.html?new`, which skips reopening the last folder); ⌘O and File → Open Folder… are gone. Menu commands go to the window in front (`emit_to`), and each window listens for its own; when a window comes forward it sets the shared menu (Connect, Open Recent). The desktop side keeps which project each window shows; opening one that another window shows brings that window forward (also from the wizard). A window's title is its project's name. Capabilities apply to every window. Next: the owner's check.
- 2026-10-12 — **Owner verified** ("all passed"): New window… from the button and ⌘N, a project opened in the new window, File menu without Open Folder…, bringing an already-open project's window forward, Open Recent and Connect acting on the window in front, a restart reopening only the last project, and the texts in zh-CN and ja. Checks: `cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`, `cargo test --workspace` (core 27, store 51, desktop 16 + 1 ignored, alm 26), web typecheck, build, `check:i18n` (249 texts), `npm test` (17): all PASS.
