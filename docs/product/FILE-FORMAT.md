# Taskbag file format — `allomos/3`

A **contract**: the files inside a taskbag. The Allomos app, the `alm` command and AI agents must all read and write exactly this. Code: `crates/core` (rules) and `crates/store` (reading and writing). Decided by [0007](../decisions/0007-git-like-project-sync.md) and [0010](../decisions/0010-project-data-folder.md).

## In plain words

A taskbag is an ordinary folder of small text files. There is one file for the project, one file per area, one file per iteration (if the project plans by iteration), and one folder per task. Each task folder holds a JSON file for its fields and a Markdown file for its description. Because they are plain text, Git can show exactly what changed, and anyone can read them.

## Where the taskbag is

- **Always `.allomos/taskbag/`** at the root of the folder the user picks: a code repository, a folder with other files, or an empty folder ([0013](../decisions/0013-product-name-fliphopper.md); renamed from `.fliphopper/seabag/` 2026-10-10, [0015](../decisions/0015-product-name-allomos.md)). `.allomos/` leaves room for other Allomos files later.
- A folder that already holds `project.json` directly is opened as a taskbag as it is; Allomos never creates one there.

- **`README-allomos.md`** next to `.allomos/`: when the app creates a taskbag in a folder that **looks empty** (no entries, or only hidden ones such as `.DS_Store` or `.git`), it also writes this short introduction in the interface's language, so the folder does not look empty to a person. It is written only there, never over an existing file, and never named `README.md`. It is not part of the taskbag; the user may edit or delete it ([`Task-A01-05`](../tasks/Task-A01-05-readme-in-new-folder.md)).

- **`AGENTS.md` and `CLAUDE.md`** next to `.allomos/`: `alm init` asks the AI agent that runs it to add a short Allomos section, merged with what the files already say ([`Task-A01-14`](../tasks/Task-A01-14-init-asks-the-agent.md)). When the app creates a taskbag, it offers to open the AI agents it finds with the same request ([`Task-A03-03`](../tasks/Task-A03-03-app-asks-agents-to-set-up.md)); only when the person chooses "Write the files for me", or `alm init --write-agent-files` runs, does Allomos add the section itself telling AI agents to work the tasks with `alm`, headed `## Tasks: Allomos` (no marker lines, owner 2026-10-08). Its text is what `alm brief` prints: a short brief naming the terms Allomos relies on and pointing agents to `alm guide` for details (owner, 2026-10-10). A file is taken to have it already when any heading names Allomos. A missing `AGENTS.md` is created with it; an existing one gets it appended once. A missing `CLAUDE.md` is created as `@AGENTS.md` (Claude Code then reads AGENTS.md); an existing one is left alone when it imports `@AGENTS.md` or has such a heading, and otherwise gets it appended. Nothing already in either file is changed ([`Task-A01-13`](../tasks/Task-A01-13-agent-instructions-on-init.md)).

Allomos never creates a Git repository inside a taskbag; the surrounding repository versions it.

## Layout

```text
.allomos/taskbag/
  .gitignore        keeps .lock and temporary files out of Git
  .gitattributes    keeps `\n` line endings on every computer
  .lock             Allomos's writing lock (never committed)
  project.json
  areas/
    A01.json
    A02.json
  iterations/       optional: a taskbag without it has no iterations
    I01.json
    I02.json
  tasks/
    A01-01/
      task.json
      description.md
    A01-02/
      task.json
      description.md
  handoff-template.md   the outline every handoff follows (allomos/2)
  project-notes.md      what every session must know: running, pitfalls, where credentials live
  handoffs/             one file per session (allomos/2)
    2026-10-12-1130-claude-a03-06.md
```

- File and folder names use the item's ID.
- Names starting with `.` are not items: readers skip them. `alm init` writes empty `areas/.keep` and `tasks/.keep` so the folders survive a Git clone.
- Files are UTF-8 JSON, pretty-printed with a final newline, so that diffs stay readable.

## `project.json`

| Field | Required | Meaning |
|---|---|---|
| `format` | yes | `"allomos/3"` for this version (`"allomos/2"` before a task's `finished_without_owner`, A03-15; `"allomos/1"` before the handoff files, A03-06): `allomos/<n>`, where the number goes up whenever the taskbag changes shape ([§ Format versions](#format-versions)). Other values are refused, including FlipHopper's `"fliphopper/1"` and the earlier `"glidebox/1"`: a FlipHopper project is moved in by an AI agent like any other task system ([0015](../decisions/0015-product-name-allomos.md)). |
| `id` | yes | Project ID: 1–64 letters, digits, `-` or `_`. |
| `name` | yes | Display name. |
| `labels` | no | The progress ladder, lowest step first. Default: `implemented`, `locally verified`, `deployed and verified`, `owner verified`. Changed with `alm labels`. |
| `project_folder` | no | The project folder (code, `AGENTS.md`, `CLAUDE.md`), relative to the folder holding `.allomos/` (`"."` for that folder, `"../app"` for a sibling), or absolute when no relative path exists. Written by the app ([OPEN-FOLDER-FLOW](./OPEN-FOLDER-FLOW.md)). Older `alm` builds refuse a taskbag that has it (`field_unknown`): reinstall `alm`. |
| `task_language` | no | The language new and changed tasks are written in: `"ui"` (follow the language the Allomos app shows; the default, also when the field is missing) or a language code such as `"zh-CN"`. Read and set with `alm task-language`. The app saves its interface language in Allomos's per-user settings (`settings.json` in `~/Library/Application Support/io.allomos` on macOS), outside the taskbag ([`Task-A01-21`](../tasks/Task-A01-21-task-language.md)). |

- `handoff` (no, `allomos/2`): when a handoff must be written ([`Task-A03-06`](../tasks/Task-A03-06-agent-handoff-rules.md)). Every field is optional and missing ones take their defaults: `quota_gate` (percentage of the 5-hour quota left; default 3), `context_gate` (percentage of context used; default 90), `write_when` (text; default "after every meaningful step: a task finished, a finding, a commit, a process started or stopped"), and `overrides`: an agent's or account's own `quota_gate` / `context_gate`, by name (`codex`, `claude/work`); a holder `codex/a01-03` gets the override `codex`. Gates are whole numbers from 0 to 100 (`invalid_percent`). Changed with `alm handoff-rules` or the app's Handoff settings.

## Handoffs (`allomos/2`)

Added by [`Task-A03-06`](../tasks/Task-A03-06-agent-handoff-rules.md). A handoff is the short note a session leaves so whoever continues, an agent or a person, does not start blind; it is written for any reader, because the writer does not know who takes over (owner, 2026-10-12).

- **`handoffs/<date>-<time>-<writer>.md`**: one file per session (`alm handoff --as <session>` updates the same file; a person writing in the app makes a new one each time). It starts with a header, then the text:

  ```text
  ---
  writer: claude/a03-06-handoffs
  tasks: A03-06, A01-27
  written: 2026-10-12T11:30Z
  updated: 2026-10-12T13:05Z
  ---
  ```

  `tasks` lists the tasks it covers (empty for a session with no task). Times are UTC. `alm show`, `alm next`, the task pane and the Handoffs tab find a task's handoffs through this list. Agents fill in their own file directly: the one exception to "never edit the taskbag by hand".
- **`handoff-template.md`**: the outline. Part 1, for anyone in plain words; part 2, restart details. Replaced with `alm handoff-template --set` or in the app.
- **`project-notes.md`**: what every session must know whatever its task: what is running, pitfalls, where credentials live (never their values).
- A task's `handoff_note` stays: the one line left on release.

## `areas/<id>.json`

| Field | Required | Meaning |
|---|---|---|
| `id` | yes | Area ID, unique in the project (e.g. `A01`). |
| `name` | yes | What part of the product this is. |
| `order` | no | Sort order on screen; default `0`, then by ID. |

## `iterations/<id>.json`

An **iteration** is *when* tasks are planned: a time-box that ends in a release (R89). Optional: a taskbag without this folder is valid, and a task without an iteration is *unplanned*. Added by [`Task-A01-15`](../tasks/Task-A01-15-iterations-view.md) (2026-10-08).

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
| `handoff_note` | no | A short note left by the last holder on release (`alm release --note`) for whoever picks the task up next. Each release replaces it, or removes it when no note is given. |
| `finished_without_owner` | no (`allomos/3`) | Why the task was finished without the owner's check, and in which run (an automatic or loop run): written only by `alm done <id> --without-owner "<text>"`; empty text is refused (`without_owner_missing`). It stays when the task is later labelled at the top step, so such tasks remain searchable (`alm list --without-owner`); the owner's check shows in the label. `alm` records what the agent says; it does not check whether the run was automatic ([`Task-A03-15`](../tasks/Task-A03-15-done-rule-instead-of-warning.md)). |

Empty optional fields are left out rather than written as `null`. **Unknown fields are refused**, so mistyped names are caught.

## `tasks/<id>/description.md`

Free Markdown: the task's outcome and details. Optional; it is not stored inside `task.json`.

## Writing rules (how writers take turns)

Owner's choice A, 2026-10-06 ([`Task-A01-03`](../tasks/Task-A01-03-gb-command.md)).

- **One short lock per change.** Every Allomos writer (`alm` now, the app when it starts editing) takes an exclusive operating-system lock on `.lock` for one change only: read the taskbag, check the rules, write, let go. A second writer waits up to 5 seconds, then gives up with `taskbag_busy`. The operating system drops the lock if the process dies, so a crash never leaves the taskbag locked.
- **Whole files only.** A file is written to a temporary `.tmp-…` file in the same folder, flushed to disk, then renamed over the old one; a new task folder is filled under a temporary name and renamed into place. A reader sees the old file or the new one, never half of one. On macOS a reader that already opened the old file keeps reading the old file whole. On Windows another program (an editor, a virus scanner) that has the file open can make the rename fail for a moment, so it is tried again for up to one second while the writer still holds the lock ([`Task-A07-03`](../tasks/Task-A07-03-windows-works.md)).
- **Readers need no lock.** The app and `alm list` / `show` / `next` read without waiting.
- **`.gitignore`** in the taskbag lists `.lock` and `.tmp-*`. `alm init` and the app both write it; since 2026-10-09 the app, like `alm init`, creates an empty taskbag ([OPEN-FOLDER-FLOW](./OPEN-FOLDER-FLOW.md)).
- **Line endings** ([`Task-A07-03`](../tasks/Task-A07-03-windows-works.md)). Allomos always writes `\n`. It reads `\r\n` the same as `\n`, since Git on Windows may check text files out that way. A new taskbag gets a **`.gitattributes`** (`*.json`, `*.md`, `.gitignore` and `.gitattributes` with `text eol=lf`) so Git keeps `\n` on every computer; `alm init`, the app and the starter project write it. A taskbag made before 2026-10-14 has none; reading still works, and one can be added by hand.
- **Paths in files** ([`Task-A07-03`](../tasks/Task-A07-03-windows-works.md)). A relative path Allomos writes into a shared file (`project_folder` in `project.json`, the taskbag's place in `AGENTS.md`) uses `/`, so the same file works on macOS and Windows; a relative `project_folder` written with `\` is read the same. A path on another Windows drive cannot be relative and is kept as it is.
- The lock only orders Allomos's own writers; a person editing a file by hand is not blocked. The checks below still catch a bad file.

## Checks Allomos makes

Each problem names the **file** and, where it can, the **field**, with a plain explanation. Valid tasks still show when other files have problems.

- **Within one file:** valid JSON; required fields present; no unknown fields; valid IDs; a known `state`; the right `format`.
- **Across files:**
  - area IDs and task IDs are unique;
  - iteration IDs are unique;
  - every task's `area` exists, and its `iteration`, when it has one;
  - every `label` is on the project's ladder;
  - every `waiting` task says what it waits on;
  - an iteration's `start` and `end` are real dates written `YYYY-MM-DD`.

## Format versions

Added by [`Task-A01-25`](../tasks/Task-A01-25-taskbag-format-upgrade.md). Allomos reads `format` before anything else in `project.json`, because a newer taskbag may have fields this Allomos does not know.

- **Newer** than this Allomos (`allomos/3` read by an Allomos that writes `allomos/2`): nothing is read or written. `alm` stops with `taskbag_newer` (exit code 4) and tells the agent to let the person choose: update Allomos now, or let the agent go on working with the files from its own understanding of the newer format. The app shows the problem ("update Allomos now").
- **Older**: `alm` refuses every command that works on the taskbag with `taskbag_older`, telling the agent to run `alm upgrade` and then repeat the command. The app shows **Update the taskbag…**.
- **`alm upgrade`** goes one step at a time (`crates/store/src/versions.rs`, `STEPS`); each step adds only what the newer format has, with its defaults, and the only other change is the `format` line. A step that is missing changes nothing (`upgrade_missing`). Steps: `allomos/1` → `allomos/2` adds `handoff-template.md`, `project-notes.md` and `handoffs/` (the rules in `project.json` are optional); `allomos/2` → `allomos/3` adds nothing (a task's `finished_without_owner` is optional; older Allomos builds would refuse it as an unknown field, which is why the format goes up).
- **Allomos never upgrades by itself** (owner, 2026-10-12): the running AI agent does, with `alm upgrade`, when `alm` or the app's request tells it to.

Allomos's section in a project's `AGENTS.md` / `CLAUDE.md` carries a version too, in its heading: `## Tasks: Allomos (v3)` (v2: the handoff lines, A03-06; v3: the rule for finishing a task and `alm next` listing every free ready task, A03-15). A heading without one, or with a lower one, is out of date: `alm` adds the warning `instructions_outdated` to every command, telling the agent to replace the section with `alm brief`'s output, and the app shows **Update the taskbag…**. Both checks are made only once an agent is connected: the section is in an instruction file of the project folder (the one recorded in `project_folder`, else the folder holding `.allomos/`).

## Not yet in this version

- **Events** (the history of changes, R7) arrive with editing (Iteration 2). They will go in `events/`.

This is an addition; existing files stay valid.

**Older Allomos versions** refuse a task with an `iteration` field (`field_unknown`), because unknown fields are refused. Update `alm` and the app before planning tasks in iterations.
