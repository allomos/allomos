# Opening a folder: FlipHopper folder, project folder, agent instructions, tasks

A **contract** for what the desktop app does when a person opens a folder. Written from the owner's specification of 2026-10-09 ([`Task-A05-02`](../tasks/Task-A05-02-convert-existing-project.md)); the owner's words are the source, the details below are how the app follows them.

## In plain words

Every FlipHopper folder belongs to a **project folder**: the folder with the project's code and its agent instructions (`AGENTS.md`, `CLAUDE.md`). Often they are the same folder. When a person opens a folder, the app walks four checks in order and stops at the first one that needs the person:

1. Is this a FlipHopper folder? If not, make it one.
2. Which project folder does it belong to? If unknown, find it or ask.
3. Do the project's agent instructions mention FlipHopper? If not, offer to ask an AI agent to add them.
4. Does the seabag have tasks? If not, offer to ask an AI agent to find the project's existing tasks and move them in.

## Terms

| Term | Meaning |
|---|---|
| **FlipHopper folder** | The folder the person opens. It holds `.fliphopper/seabag/`. |
| **Project folder** | The folder with the project's code and its `AGENTS.md` / `CLAUDE.md`. Recorded in the seabag's `project.json` as `project_folder` ([FILE-FORMAT](./FILE-FORMAT.md#projectjson)). |

## The flow

### 1. Open a folder as the FlipHopper folder

- The person chooses a folder (File → Open Folder…, Open Recent, the start-up folder).
- The app checks whether it holds `.fliphopper/seabag/` (or is a seabag itself).
- If not, it initializes it as before: a folder that looks empty gets a seabag at once (and `README-fliphopper.md`); a folder with other files first shows "No seabag in this folder yet" with **Create seabag** and **Cancel**.
- **The new seabag is empty**: the project file only, no example areas or tasks. *Why:* step 4 asks whether the seabag has tasks; example tasks would always answer yes, and the migration step would never run. (Until 2026-10-09 the app wrote a starter project with three example tasks.)
- Then step 2.

### 2. Project folder

- If `project.json` records a project folder, use it, and go to step 3.
- Otherwise, if `AGENTS.md` or `CLAUDE.md` is in the FlipHopper folder (beside `.fliphopper/`), that folder is the project folder: record it and go to step 3.
- Otherwise, ask the person: **"Which folder holds this project?"** with **Use this folder** and **Choose another folder…**. Record the answer and go to step 3. Nothing else is offered until a project folder is recorded.
- **Recorded as** a path relative to the FlipHopper folder (`"."` when they are the same, `"../app"` for a sibling), so moving or cloning both folders together keeps it working; an absolute path only when no relative one exists (another disk).
- The project folder can be changed later with **Change…** beside it in the agents panel; the change is recorded the same way.

### 3. Agent instructions

- The app looks for FlipHopper's section (`## Tasks: FlipHopper`, or any heading naming FlipHopper, or `fh` commands) in the project folder's `AGENTS.md` and `CLAUDE.md` (a `CLAUDE.md` that imports `@AGENTS.md` counts when `AGENTS.md` has it).
- If either agent is not told, the app opens the panel **"Tell AI agents about FlipHopper?"** ([`Task-A03-03`](../tasks/Task-A03-03-app-asks-agents-to-set-up.md)): the person picks agents, and each opens in the project folder with the request typed in.
- Whether the person asks or chooses **Not now**, the app goes on to step 4. **Not now** holds until the folder is opened again.

### 4. Existing tasks

- If the seabag has any task, the flow ends: the person sees the tasks.
- If it has none, go to step 5.

### 5. Task migration

- The app opens the panel **"Find this project's tasks and move them into FlipHopper?"**: the person picks **one** AI agent, which opens in the project folder with the conversion request typed in ([the request](../tasks/Task-A05-02-convert-existing-project.md#the-conversion-prompt--what-it-asks-the-agent-to-do), version 3).
- The agent looks for existing tasks in the project folder. If it finds none, it says so and stops.
- If it finds some, it asks the person whether to move them: it writes its plan and runs `fh confirm`, and the app shows the plan in a popup with **Go ahead** and **Cancel** ([CODING_RULES § The `fh` command](../workflow/CODING_RULES.md#the-fh-command)).
- Only after **Go ahead** does the agent move the tasks into the seabag, with `fh` commands only.
- **Not now** holds until the folder is opened again.
- The same step is in the menu: **File → Move Tasks Into FlipHopper…**, enabled only while the open seabag has a recorded project folder.

## What the app never does

- It never moves or edits tasks itself, and never edits `AGENTS.md` or `CLAUDE.md` without the person's choice ("Write the files for me", when no agent is found).
- It never sends a request to an agent: the agent opens with the request typed in, and the person sends it.
- It never commits or pushes.
