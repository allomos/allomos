# Taskbag file format — `allomos/4`

A **contract**: the files inside a taskbag. The Allomos app, the `alm` command and AI agents must all read and write exactly this. Code: `crates/core` (rules) and `crates/store` (reading and writing). Decided by [0007](../decisions/0007-git-like-project-sync.md) and [0010](../decisions/0010-project-data-folder.md).

## In plain words

A taskbag is an ordinary folder of small text files. There is one file for the project, one file per area, one file per iteration (if the project plans by iteration), and one folder per task. Each task folder holds a JSON file for its fields and a Markdown file for its description. Every change is also written down as one small **event** file in `events/`: what changed, who did it and when, so a task's history can be shown. Because they are plain text, Git can show exactly what changed, and anyone can read them.

## Where the taskbag is

- **Always `.allomos/taskbag/`** at the root of the folder the user picks: a code repository, a folder with other files, or an empty folder ([0013](../decisions/0013-product-name-fliphopper.md); renamed from `.fliphopper/seabag/` 2026-10-10, [0015](../decisions/0015-product-name-allomos.md)). `.allomos/` leaves room for other Allomos files later.
- A folder that already holds `project.json` directly is opened as a taskbag as it is; Allomos never creates one there.

- **`README-allomos.md`** next to `.allomos/`: when the app creates a taskbag in a folder that **looks empty** (no entries, or only hidden ones such as `.DS_Store` or `.git`), it also writes this short introduction in the interface's language, so the folder does not look empty to a person. It is written only there, never over an existing file, and never named `README.md`. It is not part of the taskbag; the user may edit or delete it (task `A01-05`).

- **`AGENTS.md` and `CLAUDE.md`** next to `.allomos/`: `alm init` asks the AI agent that runs it to add a short Allomos section, merged with what the files already say (task `A01-14`). When the app creates a taskbag, it offers to open the AI agents it finds with the same request (task `A03-03`); only when the person chooses "Write the files for me", or `alm init --write-agent-files` runs, does Allomos add the section itself telling AI agents to work the tasks with `alm`, headed `## Tasks: Allomos` (no marker lines, owner 2026-10-08). Its text is what `alm brief` prints: a short brief naming the terms Allomos relies on and pointing agents to `alm guide` for details (owner, 2026-10-10). A file is taken to have it already when any heading names Allomos. A missing `AGENTS.md` is created with it; an existing one gets it appended once. A missing `CLAUDE.md` is created as `@AGENTS.md` (Claude Code then reads AGENTS.md); an existing one is left alone when it imports `@AGENTS.md` or has such a heading, and otherwise gets it appended. Nothing already in either file is changed (task `A01-13`).

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
      card.md       optional: the task's card file (A00-30)
    A01-02/
      task.json
      description.md
  handoff-template.md   the outline every handoff follows (allomos/2)
  project-notes.md      what every session must know: running, pitfalls, where credentials live
  handoffs/             one file per session (allomos/2)
    2026-10-12-1130-claude-a03-06.md
  events/               one file per change: the history (allomos/3)
    .keep
    2026-10-13-140205-123-k7f3q2.json
```

- File and folder names use the item's ID.
- Names starting with `.` are not items: readers skip them. `alm init` writes empty `areas/.keep` and `tasks/.keep` so the folders survive a Git clone.
- Files are UTF-8 JSON, pretty-printed with a final newline, so that diffs stay readable.

## `project.json`

| Field | Required | Meaning |
|---|---|---|
| `format` | yes | `"allomos/4"` for this version (`"allomos/3"` before the `blocked` state, a task's `blocked_by` and `who`, A01-31, and the human check's `without_human_check` and `human_check_steps`, A03-18; `"allomos/2"` before events, A01-17, and a task's `finished_without_owner`, A03-15; `"allomos/1"` before the handoff files, A03-06): `allomos/<n>`, where the number goes up whenever the taskbag changes shape ([§ Format versions](#format-versions)). Other values are refused, including FlipHopper's `"fliphopper/1"` and the earlier `"glidebox/1"`: a FlipHopper project is moved in by an AI agent like any other task system ([0015](../decisions/0015-product-name-allomos.md)), and so is a taskbag in any older `allomos/<n>` ([§ Format versions](#format-versions)). |
| `id` | yes | Project ID: 1–64 letters, digits, `-` or `_`. |
| `name` | yes | Display name. |
| `labels` | no | The progress ladder, lowest step first. Default: `implemented`, `locally verified`, `deployed and verified`, `human verified` (`owner verified` before `allomos/4`; an existing ladder is renamed only when the person agrees, with `alm rename label`). The top step is the **human check**. Changed with `alm labels`. |
| `project_folder` | no | The project folder (code, `AGENTS.md`, `CLAUDE.md`), relative to the folder holding `.allomos/` (`"."` for that folder, `"../app"` for a sibling), or absolute when no relative path exists. Written by the app ([OPEN-FOLDER-FLOW](./OPEN-FOLDER-FLOW.md)). Older `alm` builds refuse a taskbag that has it (`field_unknown`): reinstall `alm`. |
| `task_language` | no | The language new and changed tasks are written in: `"ui"` (follow the language the Allomos app shows; the default, also when the field is missing) or a language code such as `"zh-CN"`. Read and set with `alm task-language`. The app saves its interface language in Allomos's per-user settings (`settings.json` in `~/Library/Application Support/io.allomos` on macOS), outside the taskbag (task `A01-21`). |

- `handoff` (no, `allomos/2`): when a handoff must be written (task `A03-06`). Every field is optional and missing ones take their defaults: `quota_gate` (percentage of the 5-hour quota left; default 3), `context_gate` (percentage of context used; default 90), `write_when` (text; default "after every meaningful step: a task finished, a finding, a commit, a process started or stopped"), and `overrides`: an agent's or account's own `quota_gate` / `context_gate`, by name (`codex`, `claude/work`); a holder `codex/a01-03` gets the override `codex`. Gates are whole numbers from 0 to 100 (`invalid_percent`). Changed with `alm handoff-rules` or the app's Handoff settings.

## Handoffs (`allomos/2`)

Added by task `A03-06`. A handoff is the short note a session leaves so whoever continues, an agent or a person, does not start blind; it is written for any reader, because the writer does not know who takes over (owner, 2026-10-12).

- **`handoffs/<date>-<time>-<writer>.md`**: one file per session (`alm handoff --as <session>` updates the same file; a person writing in the app makes a new one each time). It starts with a header, then the text:

  ```text
  ---
  writer: claude/a03-06-handoffs
  tasks: A03-06, A01-27
  written: 2026-10-12T11:30Z
  updated: 2026-10-12T13:05Z
  ---
  ```

  `tasks` lists the tasks it covers (empty for a session with no task). Times are UTC. `alm show`, `alm next`, the task pane and the Handoffs tab find a task's handoffs through this list. Agents fill in their own file directly: one of the two exceptions to "never edit the taskbag by hand" (the other is `project-notes.md`).
- **`handoff-template.md`**: the outline. Part 1, for anyone in plain words; part 2, restart details. Replaced with `alm handoff-template --set` or in the app.
- **`project-notes.md`**: what every session must know whatever its task: what is running, pitfalls, where credentials live (never their values, and no personal details such as email addresses or account names: placeholders only, task `A03-29`). A standard part of every taskbag (task `A03-20`): `alm init` creates it; agents edit it directly (the other exception to "never edit the taskbag by hand") and keep it true while the project is built, as Allomos's section in AGENTS.md (`alm brief`, v7) says. `alm next` shows its `## Running` and `## Pitfalls` sections (the first `##` heading starting with that word, any case) and the day the file last changed (its modification time on this computer, UTC); a missing file is said plainly, not an error. `alm handoff` reminds to update it. Both add `project_notes` to `--json` (`file`, `path`, `found`, `changed`, `running`, `pitfalls`).
- A task's `handoff_note` stays: the one line left on release.

## Events (`allomos/3`)

Added by task `A01-17` (R7). Every change Allomos makes to a task, an area, an iteration or `project.json` is also written down as one **event**. History is shown from events, and later the burndown and the timeline are computed from them. Owner, 2026-10-13: one file per change; nothing is invented for changes made before a taskbag started recording.

- **`events/<date>-<time>-<ms>-<random>.json`**, for example `events/2026-10-13-140205-123-k7f3q2.json`: the UTC time of the change, to the millisecond, then six random letters or digits. Two people's changes therefore never share a file name, and Git merges their `events/` folders without a conflict (each side only adds files). Files are never changed or removed once written. `events/.keep` keeps the folder in Git before the first change.

  ```json
  {
    "at": "2026-10-13T14:02:05Z",
    "by": "claude/a01-17-history",
    "via": "alm",
    "command": "claim",
    "kind": "task",
    "id": "A01-17",
    "changes": [
      { "field": "state", "from": "ready", "to": "in_progress" },
      { "field": "holder", "to": "claude/a01-17-history" }
    ]
  }
  ```

| Field | Required | Meaning |
|---|---|---|
| `at` | yes | When, in UTC, to the second. |
| `by` | no | Who: the `--as` name of the agent session or person. Left out when nobody said (the app does not know the person's name yet). |
| `via` | yes | Which writer made the change: `alm` or `app`. |
| `command` | yes | What was run: the `alm` command (`claim`, `add task`, `labels`, `init`…; `migrate` in taskbags updated by A01-32's guided migration) or the app's action (`handoff-rules`, `connect project folder`). |
| `kind` | yes | What changed: `task`, `area`, `iteration` or `project`. |
| `id` | no | The item's ID; for the project, the project's ID. |
| `changes` | yes | One entry per field that changed: `field`, then `from` (left out when the field was empty, or the item new) and `to` (left out when it was emptied). Values are as they are stored, for example `"in_progress"` or a list of labels. A new item lists every field it was created with except its ID; a deleted area or iteration (`delete area`, `delete iteration`, task `A01-29`) lists every field it had, with only `from`. A task's fields come in the task pane's order. |

- **One change, one event, under the same lock.** The writer writes the event, then the changed file; if that write fails, the event is removed. A refused change writes no event, and neither does a change that changes nothing.
- **Where history starts.** `alm init` (and the app's new taskbag) writes the first event, `init`. A taskbag upgraded from `allomos/2` by an earlier Allomos got `events/` and one `upgrade` event; that is when its history starts. A task's History shows "recorded since" the taskbag's oldest event.
- **AI decisions** (task `A03-19`): `alm ai-decided <id> "<text>"` writes an event (command `ai-decided`) with one change, `{ "field": "ai_decision", "to": "<text>" }`, and changes no other file. `alm reviewed <id>` (or the app's **Reviewed** button) writes one event (command `reviewed`) with a change `{ "field": "ai_decision", "from": "<text>" }` for each decision it reviews. A task's decisions **to review** are its `ai_decision` changes with a `to` after its last one without. They are worked out from the events alone, so `task.json` has no field for them, and an older Allomos reads these events like any other (the history shows the field by its stored name).
- **Reopening** (task `A01-50`): `alm reopen <id> --note "<why>"` (or the app's **Reopen…**) turns a `done` or `dropped` task into `ready`, held by nobody, and clears its `label`, `without_human_check` and `human_check_steps`; its description, handoff note and next dependency stay. Its event (command `reopen`) lists those changes plus `{ "field": "reopen_note", "to": "<why>" }`, which, like `ai_decision`, lives only in the history, so `task.json` gains no field and the format does not change.
- **Branches** (task `A03-22`): a task's **base branch** (the Git branch it starts from and merges back into, by its real name) and **working branch** (the branch the work is done on) are event changes named `base_branch` and `working_branch`: in the `add task` event (the base `alm add task` chose, or the person picked in the app), in the `claim` event (the working branch, and the base of a task that had none), and in a `base` event (`alm base`). A task's branches are the last value each field was given; a change with only a `from` empties it. Like AI decisions they are worked out from the events alone, so `task.json` has no field for them and the format stays `allomos/4`; an older Allomos reads these events like any other. Tasks made before have no base until they are claimed.
- **Not events:** handoffs and the handoff template (each handoff carries its own writer and times), and reading.
- **Checks.** An event file that is not valid, or has an unknown field, is reported for that file (`alm history --json` → `issues`) and left out; other events still show.

## `areas/<id>.json`

| Field | Required | Meaning |
|---|---|---|
| `id` | yes | Area ID, unique in the project (e.g. `A01`). |
| `name` | yes | What part of the product this is. |
| `order` | no | Sort order on screen; default `0`, then by ID. |

## `iterations/<id>.json`

An **iteration** is *when* tasks are planned: a time-box that ends in a release (R89). Optional: a taskbag without this folder is valid, and a task without an iteration is *unplanned*. Added by task `A01-15` (2026-10-08).

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
| `state` | yes | One of the seven states: `backlog`, `ready`, `in_progress`, `waiting` (a person or an outside event must act), `blocked` (another task in this taskbag must finish first; `allomos/4`), `done`, `dropped` ([0016](../decisions/0016-waiting-and-blocked-states.md)). A task blocked by a task and also waiting on a person is `blocked`; the person part goes in `next_dependency`. |
| `label` | no | Highest step of the project's ladder the task has proven. Must be a step on the ladder. |
| `holder` | no | Who holds the task: a person or an agent session. |
| `waiting_on` | no | For a `waiting` task: what a person (or an outside event) must do, in plain words. **Required when `state` is `waiting`.** |
| `who` | no (`allomos/4`) | For a `waiting` task: who must act (`alm wait <id> --on "<what>" --who "<who>"`). Without it the approver is meant, and the app says "Waiting for you: <what>"; with it, "Waiting for <who>: <what>". Cleared when the task leaves `waiting`. |
| `blocked_by` | no (`allomos/4`) | For a `blocked` task: the IDs of the tasks in this taskbag that must finish first, as a list (`alm block <id> --by <task-id>…`). **At least one when `state` is `blocked`**; each a task of this taskbag and not the task itself. A dropped blocker is allowed but flagged ("A01-30 (dropped; still blocked?)") and never counts as done. When every blocker is done, `alm next` says so; nothing moves the task by itself (`alm unblock <id>` does). Left out when empty; cleared when the task leaves `blocked`. |
| `next_dependency` | no | The single thing needed for the next label. |
| `iteration` | no | ID of an existing iteration the task is planned in. Without it the task is unplanned. |
| `handoff_note` | no | A short note left by the last holder on release (`alm release --note`) for whoever picks the task up next. Each release replaces it, or removes it when no note is given. |
| `without_human_check` | no (`allomos/4`) | Why the task was finished without the human check, and in which run (an automatic or loop run): written by `alm done <id> --without-human-check "<text>"` (the old flag `--without-owner` still works); empty text is refused (`without_human_check_missing`). While a `done` task has it, the task is **to check**: `alm human` lists it and the app shows "Not human-checked". Marking it checked (`alm checked <id>`, the app's **Checked** button, or the top label set on the done task) clears it together with `human_check_steps`; the task's history keeps both. `alm` records what the agent says; it does not check whether the run was automatic (task `A03-15`, task `A03-18`). |
| `human_check_steps` | no (`allomos/4`) | How a person checks the task, in Markdown: the test folder, what to open, what to click, what they should see. Written with `alm done <id> --without-human-check "<why>" --steps <file.md>` (an empty file is refused, `human_check_steps_empty`); without it, `alm done` warns `human_check_steps_missing` and shows how to write them, but does not refuse. Shown in the task pane and by `alm show` / `alm human --steps`. |

Empty optional fields are left out rather than written as `null`. **Unknown fields are refused**, so mistyped names are caught.

## `tasks/<id>/description.md`

Free Markdown: the task's outcome and details. Optional; it is not stored inside `task.json`.

## `tasks/<id>/card.md` (optional)

Added by task `A00-30` (approver, 2026-10-16), **without a format change**: a project that keeps a card file per task (the task's detail, approvals and evidence in a file of its own) keeps it here, beside the task, and the description links to it with a line `Card: [card.md](card.md)`, which the app opens. Free Markdown, written only with `alm card <id> --from <file>` (a different `card.md` is replaced only with `--replace`). Older Allomos versions read only `task.json` and `description.md` in a task's folder and ignore other files, so they still open a taskbag that has card files.

- **Card files still in the project folder.** Allomos reads only each task's `Card:` link in `description.md` (task `A01-51`): when it points outside the task's own folder (a Markdown link, read from `tasks/<id>/`; or a plain path such as `docs/tasks/A01-02.md`, read from the project folder and counted only when the file is there), the task's card is still outside, and the app shows the notice at once. An AI agent, told by Allomos's instructions (`alm brief`, `alm next`, `alm guide move-cards`), finds any others and reports them with `alm cards found <id>=<path>`; each report is an event (`command` `cards found`, change `card_outside` with the path, relative to the project folder, as `to`; a report withdrawn, or the card moved in, has only `from`). The app shows "N tasks keep their card files in the project folder" with **Move the cards in…** from the links and those events alone (a report wins over the task's link; each task counts once), so neither adds a file or a field. `alm cards` and `alm list --json` list the same, as `cards_outside` (`source` `link` or `report`). Moving a card in is one `card` event: the description's change, `card_outside` emptied, and `card` set to `card.md`.

## Writing rules (how writers take turns)

Owner's choice A, 2026-10-06 (task `A01-03`).

- **One short lock per change.** Every Allomos writer (`alm` now, the app when it starts editing) takes an exclusive operating-system lock on `.lock` for one change only: read the taskbag, check the rules, write, let go. A second writer waits up to 5 seconds, then gives up with `taskbag_busy`. The operating system drops the lock if the process dies, so a crash never leaves the taskbag locked.
- **Whole files only.** A file is written to a temporary `.tmp-…` file in the same folder, flushed to disk, then renamed over the old one; a new task folder is filled under a temporary name and renamed into place. A reader sees the old file or the new one, never half of one. On macOS a reader that already opened the old file keeps reading the old file whole. On Windows another program (an editor, a virus scanner) that has the file open can make the rename fail for a moment, so it is tried again for up to one second while the writer still holds the lock (task `A07-03`).
- **Readers need no lock.** The app and `alm list` / `show` / `next` read without waiting.
- **`.gitignore`** in the taskbag lists `.lock` and `.tmp-*`. `alm init` and the app both write it; since 2026-10-09 the app, like `alm init`, creates an empty taskbag ([OPEN-FOLDER-FLOW](./OPEN-FOLDER-FLOW.md)).
- **Line endings** (task `A07-03`). Allomos always writes `\n`. It reads `\r\n` the same as `\n`, since Git on Windows may check text files out that way. A new taskbag gets a **`.gitattributes`** (`*.json`, `*.md`, `.gitignore` and `.gitattributes` with `text eol=lf`) so Git keeps `\n` on every computer; `alm init`, the app and the starter project write it. A taskbag made before 2026-10-14 has none; reading still works, and one can be added by hand.
- **Paths in files** (task `A07-03`). A relative path Allomos writes into a shared file (`project_folder` in `project.json`, the taskbag's place in `AGENTS.md`) uses `/`, so the same file works on macOS and Windows; a relative `project_folder` written with `\` is read the same. A path on another Windows drive cannot be relative and is kept as it is.
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
  - every `blocked` task names at least one blocking task in `blocked_by` (`blocked_by_missing`), each a task of this taskbag (`blocker_unknown`) and not the task itself (`blocked_by_self`);
  - an iteration's `start` and `end` are real dates written `YYYY-MM-DD`.

## Format versions

Added by task `A01-25`. Allomos reads `format` before anything else in `project.json`, because a newer taskbag may have fields this Allomos does not know.

- **Newer** than this Allomos (`allomos/5` read by an Allomos that writes `allomos/4`): nothing is read or written. `alm` stops with `taskbag_newer` (exit code 4) and tells the agent to let the person choose: update Allomos now, or let the agent go on working with the files from its own understanding of the newer format. The app shows the problem ("update Allomos now").
- **Older**, in **any** format below this Allomos's (today `allomos/1`, `/2` and `/3`): nothing is read or written, and no format is "too old" (owner, 2026-10-15, task `A01-45`). Every `alm` command except `init`, `brief`, `guide`, `confirm` and `move-in` stops with `taskbag_to_move_in` (exit code 1), which says the taskbag must be moved in from the app, or with the request `alm move-in` prints.
- **Moving an older taskbag in** works like moving another task system in (`A05-02`): an AI agent does it, not code steps per version.
  1. The app offers **Move the tasks in…** (in the notice above the task list, and in the setup wizard's step 2). It **renames the old folder aside**, keeping the old format in its name, `.allomos/taskbag/` → `.allomos/taskbag-allomos-3/` (`-2`, `-3` … added when that name is taken), and **creates a new, empty taskbag** in its place with the old project's `id`, `name` and `project_folder`. Nothing inside the old folder is changed. If the new taskbag cannot be created, the old folder is renamed back.
  2. The Connect step then gives the chosen agent the **move-in request** (`agent_move_in_request_in`, versioned `MOVE_IN_REQUEST_VERSION`): the conversion request's steps, told that the tasks to move are the old folder's, with everything it holds: `project.json` (ladder, task language, handoff rules), areas, iterations, tasks with their descriptions and status, `handoffs/` and `project-notes.md`. The old `events/` (the history) are not copied, since `alm` records only its own changes: they stay in Git history, and the report and `project-notes.md` name the last commit that holds the old taskbag. An old top label "owner verified" becomes "human verified"; a done task's `finished_without_owner` becomes `without_human_check`.
  3. The agent reads the old files directly, writes the new taskbag only through `alm`, and asks the person once with `alm confirm`. It **removes the old folder only as part of that confirmed plan**, and only when Git history holds it.
  4. Until a task is moved in, a new taskbag with an older one renamed aside beside it (`<name>-allomos-…`) keeps offering **Move the tasks in…**, which opens the Connect step again. `alm move-in` prints the same request; run on the old taskbag itself, the request has the agent rename it aside and create the new taskbag with `alm init` after the Go ahead.
- A taskbag that A01-32's guided migration was updating (`alm migrate`, `alm migrate --finish`, the "being updated" lock, `taskbag_older`, `taskbag_too_old`, `migration_missing`) is now simply older: it is moved in like the others. Those commands and problems are gone.
- **Known limit:** another computer still running an older Allomos keeps writing to the old folder's name until it is updated; people install the new Allomos first.

Allomos's section in a project's `AGENTS.md` / `CLAUDE.md` carries a version too, in its heading: `## Tasks: Allomos (v14)` (v2: the handoff lines, A03-06; v3: the rule for finishing a task and `alm next` listing every free ready task, A03-15; v4: `alm wait --who`, `alm block` / `unblock` and the seven states, A01-31; the human check, `--without-human-check` with `--steps`, `alm human` and `alm checked`, A03-18; v5 to v14, up to card files beside their tasks, A00-30: listed at `SECTION_VERSION` in `crates/store/src/versions.rs`). A heading without one, or with a lower one, is out of date: `alm` adds the warning `instructions_outdated` to every command, telling the agent to replace the section with `alm brief`'s output, and the app shows **Update the instructions…**. Both checks are made only once an agent is connected: the section is in an instruction file of the project folder (the one recorded in `project_folder`, else the folder holding `.allomos/`).

## Not yet in this version

**Older Allomos versions** refuse a task with an `iteration` field (`field_unknown`), because unknown fields are refused. Update `alm` and the app before planning tasks in iterations.
