# Task box file format — `glidebox/1`

A **contract**: the files inside a task box. The glidebox app, the `gb` command and AI agents must all read and write exactly this. Code: `crates/core` (rules) and `crates/store` (reading and writing). Decided by [0007](../decisions/0007-git-like-project-sync.md) and [0010](../decisions/0010-project-data-folder.md).

## In plain words

A task box is an ordinary folder of small text files. There is one file for the project, one file per area, and one folder per task. Each task folder holds a JSON file for its fields and a Markdown file for its description. Because they are plain text, Git can show exactly what changed, and anyone can read them.

## Where the task box is

- Inside a repository or any folder with other files: **`.glidebox/`** at that folder's root (the default).
- Or a whole folder of its own, chosen by the user.

Glidebox never creates a Git repository inside a task box; the surrounding repository versions it.

## Layout

```text
.glidebox/
  project.json
  areas/
    A01.json
    A02.json
  tasks/
    A01-01/
      task.json
      description.md
    A01-02/
      task.json
      description.md
```

- File and folder names use the item's ID.
- Files are UTF-8 JSON, pretty-printed with a final newline, so that diffs stay readable.

## `project.json`

| Field | Required | Meaning |
|---|---|---|
| `format` | yes | Always `"glidebox/1"` for this version. Other values are refused. |
| `id` | yes | Project ID: 1–64 letters, digits, `-` or `_`. |
| `name` | yes | Display name. |
| `labels` | no | The progress ladder, lowest step first. Default: `implemented`, `locally verified`, `deployed and verified`, `owner verified`. |

## `areas/<id>.json`

| Field | Required | Meaning |
|---|---|---|
| `id` | yes | Area ID, unique in the project (e.g. `A01`). |
| `name` | yes | What part of the product this is. |
| `order` | no | Sort order on screen; default `0`, then by ID. |

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

Empty optional fields are left out rather than written as `null`. **Unknown fields are refused**, so mistyped names are caught.

## `tasks/<id>/description.md`

Free Markdown: the task's outcome and details. Optional; it is not stored inside `task.json`.

## Checks glidebox makes

Each problem names the **file** and, where it can, the **field**, with a plain explanation. Valid tasks still show when other files have problems.

- **Within one file:** valid JSON; required fields present; no unknown fields; valid IDs; a known `state`; the right `format`.
- **Across files:**
  - area IDs and task IDs are unique;
  - every task's `area` exists;
  - every `label` is on the project's ladder;
  - every `waiting` task says what it waits on.

## Not yet in this version

- **Events** (the history of changes, R7) arrive with editing (Iteration 2). They will go in `events/`.
- **Iterations** (R89) will add an optional `iteration` field.

Both are additions; existing files stay valid.
