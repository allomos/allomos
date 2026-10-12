# Opening a folder: the setup wizard, project folder, connecting agents

A **contract** for what the desktop app does when a person opens a folder. Written from the owner's specification of 2026-10-09 ([`Task-A05-02`](../tasks/Task-A05-02-convert-existing-project.md)), changed by the owner on 2026-10-11 ([`Task-A05-03`](../tasks/Task-A05-03-taskbag-setup-wizard.md)): a setup wizard, and one **Connect with the agents** step instead of two. The owner's words are the source; the details below are how the app follows them.

## In plain words

Every Allomos folder belongs to a **project folder**: the folder with the project's code and its agent instructions (`AGENTS.md`, `CLAUDE.md`). Often they are the same folder.

A folder **without** `.allomos` is set up through the **setup wizard**: what a taskbag is → create it (or change the folder) → choose the project source folder → connect with the agents.

A folder **with** `.allomos` opens directly, and the app walks the same checks in order, stopping at the first that needs the person:

1. Which project folder does it belong to? If unknown, find it or ask.
2. Are the agents told about Allomos, and does the taskbag have tasks? If either is no, offer **Connect with the agents**.

## Terms

| Term | Meaning |
|---|---|
| **Allomos folder** | The folder the person opens. It holds `.allomos/taskbag/`. |
| **Project folder** | The folder with the project's code and its `AGENTS.md` / `CLAUDE.md`. Recorded in the taskbag's `project.json` as `project_folder` ([FILE-FORMAT](./FILE-FORMAT.md#projectjson)). |

## The flow

### The first screen

- With nothing open (the first start, or the last folder is gone), the window shows the road show's opening slide: the slogan "Switch agents. Switch teammates. *Keep the project.*", its short explanation and the drawing of a task shifting between agents and a teammate around the taskbag, and one button, **Start**, with the hint "Select a folder where you want to keep your tasks". **Start** opens the wizard at step 1. The top bar has no folder button on this page.

### The setup wizard

A dialog of four steps ("Step n of 4"). **Cancel** (steps 1–3) closes it and goes back to the **start page**: no taskbag is open, and the next start shows the start page too (owner, 2026-10-12). A taskbag created by then stays on disk.

1. **What a taskbag is.** A short explanation, and **Choose a folder…**: the folder where `.allomos` will be.
2. **The `.allomos` folder.** If the chosen folder already has `.allomos/taskbag/` (or is a taskbag), the step says so and offers **Next**; **Previous** goes back to step 1 to choose another folder (owner, 2026-10-12: no **Change…** here). If not, it offers **Create**, which makes an **empty** taskbag in `<folder>/.allomos/taskbag/` (and `README-allomos.md` when the folder looks empty); then the step says "Taskbag created" and offers **Next** like a found one. The step also says what happens with Git: the folder's repository, or that the folder will be made one. *Why empty:* the connect step asks whether the taskbag has tasks; example tasks would always answer yes. (Until 2026-10-09 the app wrote a starter project with three example tasks.)
   **Next** keeps the taskbag in Git without asking (owner, 2026-10-12): if the folder holding `.allomos/` is in no Git repository, the app runs `git init` there; then it commits the taskbag's files (and `README-allomos.md` when it left one) as "Add Allomos taskbag", with `git commit -- <those paths>`, so other staged or changed files are left alone. Nothing to commit makes no commit. It never pushes. Step 3's **Next** commits again ("Update Allomos taskbag", the recorded project folder), so the agent in step 4 starts from a committed taskbag. A failure (no `git`, no name and email set in Git, a hook refusing) is shown, and the wizard goes on. When Allomos ran `git init`, step 3 opens with a note saying so: the folder is now a Git repository with the taskbag committed, what Git keeps for you, that nothing was sent anywhere, and that someone new to Git can ask their AI agent "What is Git? What is it used for?". When no working `git` is found (none installed, or only the macOS stand-in that offers the developer tools), nothing is committed, and step 3 opens with a note: Git is not installed, the taskbag is saved without its history for now, install Git and then ask the agent to put the folder under Git; and the questions to ask the agent, including "How do I install Git?".
3. **The project source folder.** Offered first: the project folder already recorded in the taskbag, else the folder from step 1. **Choose another folder…** changes it; **Next** records it.
4. **Connect with the agents** — the panel below, inside the dialog. The project folder from step 3 is shown there without **Change…**. **Connect** or **Not now** closes the wizard, and the window shows the taskbag with the recorded project folder (it does not ask for it again).

Opening a folder that has no `.allomos` (File → Open Folder…, Open Recent, the start-up folder) starts the wizard at step 2 with that folder. A folder that has one opens directly.

### Opening a taskbag directly

The page has no setup cards of its own (owner, 2026-10-12): when something is missing, the **wizard** opens at that step.

- If `project.json` records a project folder, use it. Otherwise, if `AGENTS.md` or `CLAUDE.md` is in the Allomos folder (beside `.allomos/`), that folder is the project folder: record it.
- Still no project folder: the wizard opens at **step 3**. (Until 2026-10-12 a card on the page asked "Which folder holds this project?".)
- A project folder, but an agent is not told about Allomos or the taskbag has no tasks: the wizard opens at **step 4**.
- Before showing step 3 or 4 this way, the wizard keeps the taskbag in Git and commits it (as step 2's **Next** does), and that page says what it did.
- **Connect** keeps the taskbag open. **Not now** (step 4) goes back to the **start page** when no agent has ever been told about Allomos (neither `AGENTS.md` nor `CLAUDE.md` mentions it), like **Cancel**; when one has, it keeps the taskbag open (owner, 2026-10-12). With the taskbag open, the wizard does not open by itself again until the folder is opened again (opening the same folder again counts). **Cancel** goes back to the start page.
- **Recorded as** a path relative to the Allomos folder (`"."` when they are the same, `"../app"` for a sibling), so moving or cloning both folders together keeps it working; an absolute path only when no relative one exists (another disk).

### Connect with the agents

- Offered by itself after opening when either agent is not told about Allomos (the app looks for Allomos's section — `## Tasks: Allomos`, any heading naming Allomos, or `alm` commands — in the project folder's `AGENTS.md` and `CLAUDE.md`; a `CLAUDE.md` that imports `@AGENTS.md` counts when `AGENTS.md` has it) **or** the taskbag has no tasks. **Not now** holds until the folder is opened again.
- Also at any time from **File → Connect with the Agents…** (enabled while a taskbag is open) and from the agent line's button: both open the **wizard at step 4** (step 3 while no project folder is known), where the project folder can be changed with **Change…**. This one item replaces the earlier File → Tell AI Agents About This Project… and File → Move Tasks Into Allomos… (owner, 2026-10-11).
- The person chooses **one** agent (owner, 2026-10-12; until then several could be ticked). It opens in the project folder with the **connect request** typed in ([`agent_connect_request_in`](../../crates/store/src/convert.rs)): part 1 adds Allomos's section to `AGENTS.md` and `CLAUDE.md`; part 2 looks for tasks the project already keeps. (`connect_agents` still accepts several and gives the others only part 1, but the panel no longer offers that.)
- In part 2, an agent that finds no tasks says so and stops. One that finds some asks the person whether to move them: it writes its plan and runs `alm confirm`, and the app shows the plan in a popup with **Go ahead** and **Cancel** ([CODING_RULES § The `alm` command](../workflow/CODING_RULES.md#the-fh-command)). Only after **Go ahead** does it move the tasks, with `alm` commands only.
- With no agent found, **Write the files for me** writes the section into `AGENTS.md` and `CLAUDE.md` directly.

### Update notice

Added by [`Task-A01-25`](../tasks/Task-A01-25-taskbag-format-upgrade.md) (owner, 2026-10-12). Once an agent is connected with the project, the window checks whether the taskbag's format and Allomos's section in `AGENTS.md` / `CLAUDE.md` are current ([FILE-FORMAT § Format versions](./FILE-FORMAT.md#format-versions)): when a folder is opened (File → Open Folder…, Open Recent, at start, through the wizard) and every few seconds while it is open (so a `git pull` or the agent's own update shows without reopening).

- Out of date: a notice above the task list, "This taskbag uses an older format" or "Allomos's instructions in AGENTS.md are out of date", with **Update the taskbag…**. It opens the wizard at step 4, which then says **Update the taskbag**: the chosen agent opens with the **update request** typed in (run `alm upgrade`; replace the section with `alm brief`'s output; commit only those files). **File → Connect with the Agents…** does the same while something is out of date.
- Allomos changes none of these files itself: in update mode there is no **Write the files for me**; with no agent found, the panel says to start one and choose File → Connect with the Agents… again.
- A taskbag in a newer format cannot be read: the problem list says to update Allomos now.

### Restart notice

- Just before asking agents (or writing the files), the app notes whether the project folder has `AGENTS.md` and `CLAUDE.md`. For the next 30 minutes it looks every 2 seconds; when one that was missing appears, it says: just created, AI agent sessions already running in this project have not read it, only new sessions know about Allomos.
- Files that already existed and were only changed give no notice.

## What the app never does

- It never moves or edits tasks itself, and never edits `AGENTS.md` or `CLAUDE.md` without the person's choice ("Write the files for me", when no agent is found).
- It never sends a request to an agent: the agent opens with the request typed in, and the person sends it.
- It never pushes, and it commits only in the setup wizard's step 2 (the taskbag's own files, above).
