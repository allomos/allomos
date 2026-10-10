# Task-A05-03-taskbag-setup-wizard

*Card type: coding. Area A05 (Project setup & import). Requested by the owner (2026-10-11, revised the same day): make setting up a project easier with a short **wizard** that explains the taskbag, places it, and then names the project's source folder.*

## What gets built (plain words)

Today the first screen shows a short text (`welcome.title`, `welcome.body`) and a folder picker titled "Choose a folder for the taskbag" (`picker.title`); Allomos then creates the taskbag right away or after asking, and the project folder is set separately later ("Choose the project folder", `folder.pick`). A new person is not told what a taskbag is, and the two choices are far apart. This card puts them into one guided dialog.

| Piece | What it is in everyday words | Where |
|---|---|---|
| First screen | When the app starts with no taskbag, it shows one **Open Taskbag** button, with the hint "Select a folder where you want to keep your tasks". | `apps/web/src/App.tsx`, `i18n/*` |
| Wizard, step 1 — what a taskbag is | Pressing the button opens a wizard dialog. Its first step says in plain words what a taskbag is (the folder `.allomos/` that holds the project's tasks, areas and progress as files, kept with the project in Git) and offers a button to **choose the folder** where `.allomos` will be created. | new `apps/web/src/SetupWizard.tsx`, `i18n/*` |
| Step 2 — the `.allomos` folder | If the chosen folder already has `.allomos`, the step says so and offers only a **Change** button (choose another folder). If it does not, the step offers a **Create** button that makes `.allomos/` with an empty taskbag there. | `SetupWizard.tsx`, `apps/desktop/src-tauri/src/lib.rs` |
| Step 3 — the project source | The person may pick the **project's source folder** (where the code and documents are). It starts as the folder chosen in step 1; another folder can be chosen. It is recorded with `record_project_folder`, as today. | `SetupWizard.tsx`, `lib.rs`, `crates/store/src/setup.rs` |
| After the wizard | The window opens the taskbag, as opening a folder does today; the existing offers (ask agents to add instructions, move existing tasks in — [`A03-03`](./Task-A03-03-app-asks-agents-to-set-up.md), [`A05-02`](./Task-A05-02-convert-existing-project.md)) follow as they do now. | `App.tsx` |

**What the owner can see at the end:** start the app fresh and see only **Open Taskbag**; press it and read what a taskbag is; choose an empty test folder and see **Create**; create it; on the next step see that folder already filled in as the project source, keep it or choose another; finish and see the empty taskbag. Running the wizard on a folder that already has `.allomos` shows **Change** instead of **Create**.

## Details for the worker

- **Outcome.** A new person sets up a taskbag through one guided dialog: learn what it is, place it, and name the project's source folder.
- **Decisive unknowns.**
  - Whether File → Open Folder… (⌘O) and Open Recent open the wizard or keep opening a folder directly — owner; assume an existing taskbag still opens directly and only a folder without `.allomos` goes through the wizard.
  - Whether the wizard replaces today's "create after asking when the folder has other files" step and the separate `folder.pick` control — worker proposes; owner decides. Keep a way to change the project folder later.
  - Whether step 3 may be skipped, leaving no project folder recorded — owner; assume yes, with the step-1 folder as the default.
  - Whether **Change** in step 2 may point at a folder that already has a different taskbag (then step 3 shows that taskbag's recorded project folder) — worker checks with `project_folder_of`.
- **Files owned.** New `apps/web/src/SetupWizard.tsx` (and its test); `apps/web/src/App.tsx`, `apps/web/src/types.ts`, `apps/web/src/i18n/*`; `apps/desktop/src-tauri/src/lib.rs`; `crates/store/src/setup.rs` if the store needs a "does this folder have `.allomos`" check. Shared: the welcome and picker strings in `i18n/*`.
- **External operations.** None.
- **Done means mechanically.**
  - Web tests: fresh start shows only Open Taskbag; step 2 shows Create for a folder without `.allomos` and only Change for one with it; step 3 defaults to the step-1 folder and records the chosen one.
  - Store/desktop tests for any new command (create in a chosen folder; record the project folder).
  - Manual check in the built app, in test folders under `~/allomos-tests/a05-03-setup-wizard/`: fresh start → Create → keep default source; again with another source folder; again on a folder that already has `.allomos` (Change only); restart reopens the taskbag.

## Approvals

| Date | What was approved | By |
|---|---|---|

## Notes

- 2026-10-11 — Card written from the owner's request (first as "open the project folder first").
- 2026-10-11 — Owner changed the idea: start from the taskbag with a wizard (what a taskbag is → create or change `.allomos` → choose the project source, defaulting to the same folder). Card rewritten and renamed from `Task-A05-03-open-project-first.md`.
