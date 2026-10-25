# 0019 — One taskbag, several project folders

| Status | Raised | Decided | Approver |
|---|---|---|---|
| **Accepted** | 2026-10-25 | 2026-10-25 | will.w.hui |

Card: task `A00-pulse294`. Decided by the approver on 2026-10-25, in chat (the five answers below, with mockups of the Project folders dialog, the unbind question, the task pane's Folders field and the task list's folder filter).

## In plain words

A project is often more than one folder: a code repository, a website repository, a mobile repository. Each has its own AGENTS.md, the file that tells AI agents how to work there. Today a taskbag knows only one such folder, and a task does not say which folder it is about.

**Decision:** a taskbag lists several **project folders**, each with a short name. Each task lists the names of the folders it belongs to, or none. An agent working inside one folder sees only that folder's tasks.

## Context

- `project.json` holds one `project_folder`: a path to the folder with the project's code and its AGENTS.md / CLAUDE.md ([OPEN-FOLDER-FLOW.md](../product/OPEN-FOLDER-FLOW.md)). Tasks name no folder.
- The taskbag can sit in its own folder or inside a project folder (R73, [0010](./0010-project-data-folder.md)). Allomos itself keeps its taskbag in a separate repository beside the code.
- Adding a field that older Allomos versions would misread is a change of data format: a format step, as from `allomos/4` to `allomos/5`.

## Decision

1. **Project folders in `project.json`.** The single `project_folder` is replaced by a list, `project_folders`. Each entry has:
   - a **short name**, such as `allomos` or `allomos-site`, unique in the taskbag;
   - a **relative path**, resolved from the same place `project_folder` is today, so no personal path (a home folder, a user name) is written into the taskbag.

   One relative path per folder is enough for now; there is no path per computer. A teammate who keeps a folder somewhere else sees it as **not found** and points Allomos at it (**Locate…** in the app, `alm folder locate` on the command line).
2. **Folders on each task.** Each task gets a `folders` list of those names. An **empty list means the task belongs to no particular folder** (planning, naming, decisions), not to all folders.
3. **Format step to `allomos/6`.** Opening an `allomos/5` taskbag moves its `project_folder` into the list as the first entry. Allomos versions that do not know `allomos/6` refuse the taskbag with exit code 4, as they refuse any newer format today.
4. **AGENTS.md in every folder.** Every bound folder's AGENTS.md carries the Allomos section, which points back to the taskbag with a path relative to that folder.
5. **Description links** (`resolve_task_link`) may open files inside any bound folder, not only the first.
6. **What `alm next` lists.** Run inside a bound folder, `alm next` lists only the tasks that name that folder; tasks with no folder are not listed there. They are listed when `alm next` runs in the taskbag, outside every bound folder, or with `--all-folders`.
7. **The agent decides a new task's folders.** When an AI agent adds a task, the agent decides whether it belongs to one folder, several, or none, and says so on the command: `--folder <name>` (repeatable) or `--no-folder`. `alm` does not guess it from the folder the command runs in. The Allomos section of AGENTS.md tells agents to decide this.
8. **Unbinding a folder keeps its tasks.** Before unbinding, Allomos shows what will happen: how many tasks name the folder, how many of them are still open, that the folder's files stay where they are and only the Allomos section leaves its AGENTS.md. It unbinds only after the person confirms. Tasks are never deleted or moved to another folder; the folder's name is taken off them, so a task that named only that folder then has no folder.

### The approver's answers (2026-10-25)

| # | Question | Answer |
|---|---|---|
| 1 | How are folders and task folders recorded? | Each folder has a short name and a path relative to the taskbag in `project.json`; each task lists folder names. |
| 2 | What does an empty `folders` list mean? | The task belongs to no particular folder, not to all folders. |
| 3 | Inside a bound folder, what does `alm next` show? | Only that folder's tasks, without the tasks that have no folder. |
| 4 | What happens to tasks when a folder is unbound? | They are kept; Allomos shows what unbinding does and asks first; the folder's name is taken off the tasks (decision 8). |
| 5 | A path per computer? | Not needed now; one relative path per folder, and **Locate…** / `alm folder locate` for a folder kept elsewhere. |
| — | Who decides a new task's folder? | The AI agent that adds it, said on the command (decision 7). |

## Alternatives considered

| Option | Why not |
|---|---|
| Keep one project folder; put other repositories in subfolders | Many projects keep their repositories side by side, each with its own Git history and AGENTS.md. |
| One taskbag per folder | Splits one project's progress, burndown and handoffs across taskbags; switching agents or teammates (0014) would lose the whole picture. |
| An empty `folders` list means "all folders" | Planning and decision tasks would then show in every folder's `alm next`, crowding each repository's list with work no agent there can do. |
| `alm add` takes the folder from where it runs | Agents often add a task for another folder while working in one; a silent guess would file it wrongly. |
| A path per computer | More to keep in step for a case one Locate… step already covers. |

## Consequences

- **Requirements:** R95 states several project folders per taskbag; R73 is unchanged (a taskbag may still sit inside one of the folders).
- **Data format:** `allomos/6`, with the move from `project_folder` described above. Older apps and `alm` refuse it; the release that brings it must say so.
- **Work that builds it**, each its own task: `alm folder` commands; task folders in `alm` (`--folder`, `--no-folder`, the `alm next` filter); the Project folders dialog in the app; folders in the task pane and the task list's filter.
- **Shared rules:** the Allomos section of AGENTS.md (`alm brief`) gains the rule that the agent decides a new task's folders.
