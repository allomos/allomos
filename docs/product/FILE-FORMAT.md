# Seabag file format — `fliphopper/1`

A **contract**: the files inside a seabag. The FlipHopper app, the `fh` command and AI agents must all read and write exactly this. Code: `crates/core` (rules) and `crates/store` (reading and writing). Decided by [0007](../decisions/0007-git-like-project-sync.md) and [0010](../decisions/0010-project-data-folder.md).

## In plain words

A seabag is an ordinary folder of small text files. There is one file for the project, one file per area, one file per iteration (if the project plans by iteration), and one folder per task. Each task folder holds a JSON file for its fields and a Markdown file for its description. Because they are plain text, Git can show exactly what changed, and anyone can read them.

## Where the seabag is

- **Always `.fliphopper/seabag/`** at the root of the folder the user picks: a code repository, a folder with other files, or an empty folder ([0013](../decisions/0013-product-name-fliphopper.md)). `.fliphopper/` leaves room for other FlipHopper files later.
- A folder that already holds `project.json` directly is opened as a seabag as it is; FlipHopper never creates one there.

- **`README-fliphopper.md`** next to `.fliphopper/`: when the app creates a seabag in a folder that **looks empty** (no entries, or only hidden ones such as `.DS_Store` or `.git`), it also writes this short introduction in the interface's language, so the folder does not look empty to a person. It is written only there, never over an existing file, and never named `README.md`. It is not part of the seabag; the user may edit or delete it ([`Task-A01-05`](../tasks/Task-A01-05-readme-in-new-folder.md)).

- **`AGENTS.md` and `CLAUDE.md`** next to `.fliphopper/`: `fh init` asks the AI agent that runs it to add a short FlipHopper section, merged with what the files already say ([`Task-A01-14`](../tasks/Task-A01-14-init-asks-the-agent.md)). When the app creates a seabag, it offers to open the AI agents it finds with the same request ([`Task-A03-03`](../tasks/Task-A03-03-app-asks-agents-to-set-up.md)); only when the person chooses "Write the files for me", or `fh init --write-agent-files` runs, does FlipHopper add the section itself telling AI agents to work the tasks with `fh`, headed `## Tasks: FlipHopper` (no marker lines, owner 2026-10-08). A file is taken to have it already when any heading names FlipHopper. A missing `AGENTS.md` is created with it; an existing one gets it appended once. A missing `CLAUDE.md` is created as `@AGENTS.md` (Claude Code then reads AGENTS.md); an existing one is left alone when it imports `@AGENTS.md` or has such a heading, and otherwise gets it appended. Nothing already in either file is changed ([`Task-A01-13`](../tasks/Task-A01-13-agent-instructions-on-init.md)).

FlipHopper never creates a Git repository inside a seabag; the surrounding repository versions it.

## Layout

```text
.fliphopper/seabag/
  .gitignore        keeps .lock and temporary files out of Git
  .lock             FlipHopper's writing lock (never committed)
  project.json
  areas/
    A01.json
    A02.json
  iterations/       optional: a seabag without it has no iterations
    I01.json
    I02.json
  tasks/
    A01-01/
      task.json
      description.md
    A01-02/
      task.json
      description.md
```

- File and folder names use the item's ID.
- Names starting with `.` are not items: readers skip them. `fh init` writes empty `areas/.keep` and `tasks/.keep` so the folders survive a Git clone.
- Files are UTF-8 JSON, pretty-printed with a final newline, so that diffs stay readable.

## `project.json`

| Field | Required | Meaning |
|---|---|---|
| `format` | yes | Always `"fliphopper/1"` for this version. Other values are refused, including the earlier `"glidebox/1"`; old folders are not converted ([0013](../decisions/0013-product-name-fliphopper.md)). |
| `id` | yes | Project ID: 1–64 letters, digits, `-` or `_`. |
| `name` | yes | Display name. |
| `labels` | no | The progress ladder, lowest step first. Default: `implemented`, `locally verified`, `deployed and verified`, `owner verified`. |

## `areas/<id>.json`

| Field | Required | Meaning |
|---|---|---|
| `id` | yes | Area ID, unique in the project (e.g. `A01`). |
| `name` | yes | What part of the product this is. |
| `order` | no | Sort order on screen; default `0`, then by ID. |

## `iterations/<id>.json`

An **iteration** is *when* tasks are planned: a time-box that ends in a release (R89). Optional: a seabag without this folder is valid, and a task without an iteration is *unplanned*. Added by [`Task-A01-15`](../tasks/Task-A01-15-iterations-view.md) (2026-10-08).

| Field | Required | Meaning |
|---|---|---|
| `id` | yes | Iteration ID, unique in the project (e.g. `I02`). |
| `name` | yes | What the iteration delivers, e.g. `v0.2 — macOS + Windows`. |
| `order` | no | Sort order on screen; default `0`, then by ID. |
| `start` | no | First day, `YYYY-MM-DD`. |
| `end` | no | Last day, `YYYY-MM-DD`. |
| `release` | no | The release it produces, e.g. `v0.2`. |

## `tasks/<id>/task.json`

| Field | Required | Meaning |
|---|---|---|
| `id` | yes | Task ID, unique in the project (e.g. `A01-02`). |
| `title` | yes | One line. |
| `area` | yes | ID of an existing area. |
| `state` | yes | One of `backlog`, `ready`, `in_progress`, `waiting`, `done`, `dropped`. |
| `label` | no | Highest step of the project's ladder the task has proven. Must be a step on the ladder. |
| `holder` | no | Who holds the task: a person or an agent session. |
| `waiting_on` | no | What blocks the next step, in plain words. **Required when `state` is `waiting`.** |
| `next_dependency` | no | The single thing needed for the next label. |
| `iteration` | no | ID of an existing iteration the task is planned in. Without it the task is unplanned. |
| `handoff_note` | no | A short note left by the last holder on release (`fh release --note`) for whoever picks the task up next. Each release replaces it, or removes it when no note is given. |

Empty optional fields are left out rather than written as `null`. **Unknown fields are refused**, so mistyped names are caught.

## `tasks/<id>/description.md`

Free Markdown: the task's outcome and details. Optional; it is not stored inside `task.json`.

## Writing rules (how writers take turns)

Owner's choice A, 2026-10-06 ([`Task-A01-03`](../tasks/Task-A01-03-gb-command.md)).

- **One short lock per change.** Every FlipHopper writer (`fh` now, the app when it starts editing) takes an exclusive operating-system lock on `.lock` for one change only: read the seabag, check the rules, write, let go. A second writer waits up to 5 seconds, then gives up with `seabag_busy`. The operating system drops the lock if the process dies, so a crash never leaves the seabag locked.
- **Whole files only.** A file is written to a temporary `.tmp-…` file in the same folder, flushed to disk, then renamed over the old one; a new task folder is filled under a temporary name and renamed into place. A reader sees the old file or the new one, never half of one. On macOS a reader that already opened the old file keeps reading the old file whole. (Windows is checked when the app starts writing, Iteration 2.)
- **Readers need no lock.** The app and `fh list` / `show` / `next` read without waiting.
- **`.gitignore`** in the seabag lists `.lock` and `.tmp-*`. `fh init` and the app's starter project both write it.
- The lock only orders FlipHopper's own writers; a person editing a file by hand is not blocked. The checks below still catch a bad file.

## Checks FlipHopper makes

Each problem names the **file** and, where it can, the **field**, with a plain explanation. Valid tasks still show when other files have problems.

- **Within one file:** valid JSON; required fields present; no unknown fields; valid IDs; a known `state`; the right `format`.
- **Across files:**
  - area IDs and task IDs are unique;
  - iteration IDs are unique;
  - every task's `area` exists, and its `iteration`, when it has one;
  - every `label` is on the project's ladder;
  - every `waiting` task says what it waits on;
  - an iteration's `start` and `end` are real dates written `YYYY-MM-DD`.

## Not yet in this version

- **Events** (the history of changes, R7) arrive with editing (Iteration 2). They will go in `events/`.

This is an addition; existing files stay valid.

**Older FlipHopper versions** refuse a task with an `iteration` field (`field_unknown`), because unknown fields are refused. Update `fh` and the app before planning tasks in iterations.
