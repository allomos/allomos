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
| Step 4 — connect with the agents | The last step of the wizard, **Connect with the agents**, replaces today's two separate offers — "Tell AI agents about Allomos?" ([`A03-03`](./Task-A03-03-app-asks-agents-to-set-up.md), [`A03-05`](./Task-A03-05-agents-panel-refinements.md)) and "move existing tasks into Allomos" ([`A05-02`](./Task-A05-02-convert-existing-project.md)) — with **one** request. The chosen agent opens in the project folder with one request typed in: add Allomos's instructions to `AGENTS.md` and `CLAUDE.md`, **and** look for tasks the project already keeps. If it finds some, the person still decides whether to move them, exactly as the current UI asks: the agent shows its plan in the app (`alm confirm`) and the person chooses go ahead or cancel. Nothing is sent until the person sends it; **Not now** skips the step. The current choices stay: which agents, terminal or app, and "Write the files for me". | `SetupWizard.tsx`, `AgentsPanel.tsx`, `i18n/*`; the combined request in `crates/store/src/lib.rs` (built from `agent_setup_request_in` and the conversion request) |
| After the wizard | The window opens the taskbag, as opening a folder does today. Later, **File → Connect with the Agents…** opens the same step. It is the only menu item for this: File → Agents… and File → Move Tasks Into Allomos… are removed (owner, 2026-10-11). | `App.tsx`, menu in `apps/desktop/src-tauri/src/lib.rs`, `i18n/*` |
| Restart notice | When `AGENTS.md` or `CLAUDE.md` did not exist before and was just created through this setup (by Allomos or by an agent it asked), the app tells the person: only **new** agent sessions will know about the setup; AI agent sessions already running in this project read these files when they start, so they must be restarted. Not shown when the files already existed and were only changed. | `App.tsx`, `AgentsPanel.tsx`, `i18n/*`; existence check in `lib.rs` |

**What the owner can see at the end:** start the app fresh and see only **Open Taskbag**; press it and read what a taskbag is; choose an empty test folder and see **Create**; create it; on the next step see that folder already filled in as the project source, keep it or choose another; finish and see the empty taskbag. Running the wizard on a folder that already has `.allomos` shows **Change** instead of **Create**.

## Details for the worker

- **Outcome.** A new person sets up a taskbag through one guided dialog: learn what it is, place it, name the project's source folder, and connect AI agents with one request that adds Allomos's instructions and offers to move existing tasks in.
- **Decisive unknowns.**
  - Whether File → Open Folder… (⌘O) and Open Recent open the wizard or keep opening a folder directly — owner; assume an existing taskbag still opens directly and only a folder without `.allomos` goes through the wizard.
  - Whether the wizard replaces today's "create after asking when the folder has other files" step and the separate `folder.pick` control — worker proposes; owner decides. Keep a way to change the project folder later.
  - Whether step 3 may be skipped, leaving no project folder recorded — owner; assume yes, with the step-1 folder as the default.
  - *Decided (owner, 2026-10-11):* when several agents are chosen in step 4, the **first** chosen agent gets the combined request and the others get only the instructions request (two agents moving tasks would add every task twice).
  - *Decided (owner, 2026-10-11):* the File menu has a single **Connect with the Agents…**; File → Agents… and File → Move Tasks Into Allomos… are removed.
  - How the app knows `AGENTS.md` / `CLAUDE.md` was **just created**: note whether each file exists before setup or before asking the agents, and notice when it appears — worker.
  - Whether **Change** in step 2 may point at a folder that already has a different taskbag (then step 3 shows that taskbag's recorded project folder) — worker checks with `project_folder_of`.
- **Files owned.** New `apps/web/src/SetupWizard.tsx` (and its test); `apps/web/src/App.tsx`, `apps/web/src/AgentsPanel.tsx`, `apps/web/src/types.ts`, `apps/web/src/i18n/*`; `apps/desktop/src-tauri/src/lib.rs`; `crates/store/src/lib.rs` (the combined request); `crates/store/src/setup.rs` if the store needs a "does this folder have `.allomos`" check. Shared: the welcome and picker strings in `i18n/*`.
- **External operations.** None.
- **Done means mechanically.**
  - Web tests: fresh start shows only Open Taskbag; step 2 shows Create for a folder without `.allomos` and only Change for one with it; step 3 defaults to the step-1 folder and records the chosen one.
  - Store test: the combined request contains the instructions part and the find-and-move part, in each app language, and the move part still requires `alm confirm` before any change. Web test: step 4 gives the combined request to the first chosen agent and the instructions request to the others, and Not now skips it; File → Connect with the Agents… opens the same step, and the menu no longer has Agents… or Move Tasks Into Allomos….
  - Web test: the restart notice appears when `AGENTS.md` or `CLAUDE.md` did not exist before setup and exists after; it does not appear when both already existed.
  - Store/desktop tests for any new command (create in a chosen folder; record the project folder).
  - Manual check in the built app, in test folders under `~/allomos-tests/a05-03-setup-wizard/`: fresh start → Create → keep default source; again with another source folder; again on a folder that already has `.allomos` (Change only); restart reopens the taskbag; in a folder without `AGENTS.md` / `CLAUDE.md`, the restart notice appears once they are created; in a test copy of a project with a Markdown task board, step 4's agent adds the instructions, finds the board, and changes nothing until the plan is confirmed in the app.

## Approvals

| Date | What was approved | By |
|---|---|---|

## Notes

- 2026-10-11 — Card written from the owner's request (first as "open the project folder first").
- 2026-10-11 — Owner changed the idea: start from the taskbag with a wizard (what a taskbag is → create or change `.allomos` → choose the project source, defaulting to the same folder). Card rewritten and renamed from `Task-A05-03-open-project-first.md`.
- 2026-10-11 — Owner added: when this setup has just created `AGENTS.md` or `CLAUDE.md`, tell the person that only new agent sessions will know about the setup.
- 2026-10-11 — Owner added: after the folders are set up, combine "tell agents about Allomos" and "find and move existing tasks" into one step, **Connect with the agents**; the person still decides whether tasks are moved, as the current UI asks.
- 2026-10-11 — Owner decided: the first chosen agent gets the combined request, the others only the instructions; File → Connect with the Agents… replaces File → Agents…, and File → Move Tasks Into Allomos… stays but opens the same step.
- 2026-10-11 — Owner changed the menu decision: only File → Connect with the Agents…; no separate Move Tasks item.
- 2026-10-11 — Implemented on `claude/a05-03-setup-wizard`. How the open questions were settled in code (owner may change any):
  - **Open Folder / Open Recent / start-up folder:** a folder with `.allomos` opens directly; one without starts the wizard at step 2. The old "No taskbag in this folder yet" card and the automatic create in an empty folder are gone: the wizard's **Create** does both.
  - **Step 3 has no separate Skip:** the default folder is filled in, so **Next** keeps it. **Cancel** leaves it unrecorded, and the page then asks "Which folder holds this project?" as before (still used when a taskbag is opened directly).
  - **Restart notice:** the app notes which of `AGENTS.md` / `CLAUDE.md` exist just before asking agents (or writing the files), then looks every 2 s for 30 min and tells once per new file.
  - **Connect with the agents** opens by itself after opening a taskbag when an agent is not told **or** there are no tasks (one "Not now" for both). The agent line's button now says "Connect with the agents…".
  - The conversion request is v6 (its "start again" hint names File → Connect with the Agents…); the connect request is v1.
  - Contracts updated: [OPEN-FOLDER-FLOW.md](../product/OPEN-FOLDER-FLOW.md), [CODING_RULES.md](../workflow/CODING_RULES.md) (new check `npm --prefix apps/web test`, Node's own test runner, no new packages).
- 2026-10-11 — Checks: `cargo fmt --check` PASS · `cargo clippy --workspace --all-targets -D warnings` PASS · `cargo test --workspace` PASS (store 32, desktop 12 + 1 ignored, cli 20, core 24; new: `connect_request_tells_agents_then_offers_to_move_tasks`, `the_first_agent_also_looks_for_tasks`) · `npm --prefix apps/web test` PASS (5) · `typecheck` PASS · `build` PASS · `check:i18n` PASS (166 texts). Debug app built: `target/debug/bundle/macos/Allomos.app` in this worktree. **Manual check in the app: NOT RUN** (the agent was not given access to the window); test folders are ready in `~/allomos-tests/a05-03-setup-wizard/` (`code-app`: a Git repo with a TASK_BOARD.md and no AGENTS.md/CLAUDE.md; `has-taskbag`: already has `.allomos`). Not covered by any check: the first screen after a truly fresh start (the window storage is shared with other Allomos builds on this Mac, so it reopens the last folder).
- 2026-10-11 — Owner's first check: the wording needed refining, and after the last step the main window asked for the project folder again. **Cause:** the window loaded the setup state when the wizard opened the taskbag (before step 3 recorded the project folder) and never reloaded it. **Fix:** the wizard hands the recorded state to the window, and the window reads it again when the wizard closes. Step 4 now shows the project folder without **Change…** (it was just chosen in step 3). **Wording:** one term each, in all three languages — *taskbag* (the `.allomos` folder of tasks), *project folder* (code, documents, `AGENTS.md` / `CLAUDE.md`; where agents work), *add Allomos's instructions to* a file, and *create* only for a file that did not exist. The wizard explains what a taskbag holds, says "Taskbag created" after Create, labels the project folder, and the connect step says what every agent is asked and, separately, what the first one also does. The restart notice says which files are new and why open sessions don't know.
