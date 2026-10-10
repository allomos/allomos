# Task-A05-03-open-project-first

*Card type: coding. Area A05 (Project setup & import). Requested by the owner (2026-10-11): make setting up a project simpler by starting from **the project's own folder**, not from the taskbag.*

## What gets built (plain words)

Today the first screen asks for "a folder for the taskbag" (`picker.title`, `welcome.body`), and only afterwards may the person name the project folder ("Choose the project folder", `folder.pick`). That order is backwards for most people: they know where their project's files are, not where a taskbag should go. This card turns it around.

| Piece | What it is in everyday words | Where |
|---|---|---|
| First screen | When the app starts with no project, it shows one **Open Project** button, with the hint "Select a folder where your project files are". | `apps/web/src/App.tsx`, `i18n/*` |
| Project folder | The chosen folder becomes the project's **source folder**. Its `.allomos` folder is expected inside it (`<project>/.allomos/`). | `App.tsx`, `apps/desktop/src-tauri/src/lib.rs` |
| No `.allomos` yet | The window shows two buttons: **Create** (make `.allomos/` with an empty taskbag inside the project folder) and **Select** (choose an existing `.allomos` folder somewhere else; the project folder is recorded in it, as `record_project_folder` does today). | `App.tsx`, `lib.rs`, `crates/store/src/setup.rs` |
| `.allomos` found or set | Only a **Change** button, to pick a different `.allomos` folder for this project. | `App.tsx` |
| Reopening | The app reopens the last project folder (as it reopens the last folder today) and finds its `.allomos` again, also when it was selected from elsewhere. | `App.tsx`, `lib.rs` |

**What the owner can see at the end:** start the app fresh, see only **Open Project**; pick a code folder without `.allomos`, see **Create** and **Select**; press Create and see the empty taskbag with only **Change** left; restart and land in the same project.

## Details for the worker

- **Outcome.** A person opens their project folder first; Allomos then finds, creates or lets them select its `.allomos` folder.
- **Decisive unknowns.**
  - How the app remembers a `.allomos` selected from **outside** the project folder, so reopening the project finds it again (app setting keyed by project folder, or a small pointer file in the project) — worker proposes, owner decides.
  - What **Change** does to the old `.allomos` (leave it untouched is the safe default) — owner.
  - Whether File → Open Folder… and Open Recent become "Open Project…" with the same meaning — owner; assume yes.
  - How this fits [`A05-02`](./Task-A05-02-convert-existing-project.md)'s convert offer and the agents panel, which today trigger after a taskbag is created — keep them firing after **Create**.
- **Files owned.** `apps/web/src/App.tsx`, `apps/web/src/i18n/*`, `apps/web/src/types.ts`, `apps/desktop/src-tauri/src/lib.rs`, `crates/store/src/setup.rs` (and its tests). Shared: the welcome and folder-picker strings in `i18n/*`.
- **External operations.** None.
- **Done means mechanically.**
  - Store tests: finding `.allomos` inside a project folder; recording a project folder in a `.allomos` selected elsewhere; reopening finds it.
  - Web tests for the three screens: no project (Open Project only), project without `.allomos` (Create + Select), project with `.allomos` (Change only).
  - Manual check in the built app, in a test folder under `~/allomos-tests/a05-03-open-project/`: fresh start, Create, Select, Change, restart.

## Approvals

| Date | What was approved | By |
|---|---|---|

## Notes

- 2026-10-11 — Card written from the owner's request.
