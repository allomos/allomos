//! What `alm guide` tells an AI agent (Task-A05-02). The short brief in a project's `AGENTS.md`
//! ([`crate::agent_section`]) names these topics; an agent reads them from `alm` when it needs
//! them, so the details always match the installed `alm`, not a copy that went stale. English:
//! agents read it, the app does not show it. Two exceptions are written in every language the
//! Allomos app speaks: `run-modes` (Task-A03-21), which the agent shows to the person
//! ([`run_modes`]), and `agent-files` (Task-A03-24), the one-time procedure that puts every
//! agent's shared rules in AGENTS.md, which the person may read along ([`agent_files`]); and `goal`
//! (Task-A03-25-01), the goal prompt Allomos types into an agent for the person ([`goal`]).
//! `github-release` (Task-A03-planet530), which the agent follows when the approver writes
//! `alm guide github release` in the conversation, is printed in the task language like those, but
//! so far written only in English ([`github_release`]).

/// The topics, in the order `alm guide` prints them.
pub const GUIDE_TOPICS: [&str; 10] = [
    "terms",
    "customize",
    "tasks",
    "commands",
    "workflow",
    "run-modes",
    "agent-files",
    "goal",
    "github-release",
    "move-cards",
];

/// The topics written in every language of [`RUN_MODES_LANGUAGES`], printed in the task language.
pub const LOCALIZED_TOPICS: [&str; 4] = ["run-modes", "agent-files", "goal", "github-release"];

/// The languages `alm guide run-modes`, `alm guide agent-files` and `alm guide goal` are written in: those of the Allomos app (`en`, `ja`,
/// `zh-CN`, as [`crate::pick_ui_language`] picks them).
pub const RUN_MODES_LANGUAGES: [&str; 3] = ["en", "ja", "zh-CN"];

const TERMS: &str = "# Allomos terms

Allomos relies on these terms. Use them as defined here, in commands and when you talk
about the project's tasks.

- **Taskbag**: the folder holding the project's tasks (`.allomos/taskbag/`): readable JSON and
  Markdown files kept in your version control tool (Git, for example). Change it only with `alm`, except the project notes.
- **Project notes**: the taskbag's `project-notes.md`: what every session must know whatever its
  task (what is running, pitfalls, where the code and the taskbag are kept online, where
  credentials live, never their values; no personal details such as email addresses or account
  names, only placeholders). Edited directly and kept true while the project is built; `alm
  next` shows its Running, Pitfalls and Repositories.
- **Project folder**: the folder with the project's code and its AGENTS.md / CLAUDE.md. Usually
  the folder holding `.allomos/`; `project_folder` in the taskbag's project.json records it.
- **Task**: one observable outcome: what a person or agent can do when it is done. It has an
  ID, a title, an area, a state, and optionally a label, an iteration, a holder, what it waits on
  (and who), the tasks that block it, its next dependency, a handoff note and a description
  (description.md).
- **Area**: *what* part of the product a task belongs to. Every task has exactly one.
- **Iteration**: *when* a task is planned: a time-box that ends in a release. Optional; a task
  without one is unplanned.
- **State**: where a task is in its life: backlog, ready (approved to start), in progress (held
  by someone), waiting (a person or an outside event must act: its waiting-on says what, its
  optional who says who; no who means the approver), blocked (another task in this taskbag must
  finish first: its blocked-by lists them), done, or dropped (will not be done). A task blocked by
  a task and also waiting on a person is blocked; the person part goes in its next dependency.
- **Label**: the highest step of the project's progress **ladder** the task has proven, for
  example implemented → locally verified → human verified. It is the only progress claim; test
  counts support a label but are never one.
- **Human check**: a person tries the finished task by hand. Its step is the ladder's top label.
  A task finished without it (in an automatic or loop run) carries why, and the steps for the
  person who checks it; it is **to check** until someone marks it checked.
- **Approver**: the person who decides and approves (cards, plans, external effects). A waiting
  task without a who waits for the approver.
- **AI decision**: a routine choice an agent made on its own inside a task (a name, a layout, a
  small detail with an obvious default), recorded with `alm ai-decided` in the task's history. It
  is **to review** until the approver has reviewed it; overturning one is an ordinary change to
  the task.
- **Holder**: the person or agent session working a task (`--as <name>`). One holder at a time.
- **Next dependency**: the one thing needed before the task's next label.
- **Handoff note**: what the next holder needs to know: where the work stopped, the one next
  step, who it waits on. Written with `alm handoff` (or left when a task is released); a handoff
  lives in the task itself, not in a file of its own.
- **Task language**: the language new and changed tasks are written in (titles, descriptions,
  notes). `alm task-language` tells you which; by default it follows the language the person
  uses Allomos in.
";

const CUSTOMIZE: &str = "# What a project may change, and what is fixed

| Term | This project may change | How |
|---|---|---|
| Areas | which areas exist, their IDs and names | `alm add area <id> \"<name>\"`, `alm rename area`, `alm delete area` (only an empty one) |
| Iterations | which exist, their names, releases and dates; which task is in which | `alm add iteration`, `alm change iteration`, `alm rename iteration` (its ID, with its tasks), `alm move iteration` (its place in the plan), `alm delete iteration` (only an empty one), `alm plan` |
| Label ladder | its steps and their order | `alm labels \"<a,b,…>\"` (refused if a task's label would be left off); `alm rename label \"<old>\" \"<new>\"` renames a step and the tasks that carry it |
| Task IDs and titles | any ID of letters, digits, `-` or `_` (for example A01-02) | `alm add task <id> --area <id> --title \"…\"` |
| Description | free Markdown in description.md | written with `alm add task --description` |
| Task language | `ui` (follow the Allomos app's language, the default) or a language code | `alm task-language <code or ui>`; only when the person asks |

Fixed for every project:

- **The seven states**: backlog, ready, in progress, waiting, blocked, done, dropped.
- **The words \"area\" and \"iteration\"**: a project cannot rename them yet (planned: requirement
  R90). Use these words with `alm`, even if the team says \"workstream\" or \"sprint\".
- **The file format**: never edit the taskbag's files by hand (except `project-notes.md`); `alm`
  checks every change.

Read the project's current areas, iterations and tasks with `alm list` (`--by iteration`), and its
ladder in the taskbag's project.json.
";

const TASKS: &str = "# How to add a task

Read this before you add a task, so it fits this project.

0. **Language.** Run `alm task-language` and write the title and description in that language.
   IDs, commands, code and file names stay as they are.
1. **Look first.** `alm list` shows the areas and the task IDs already used; `alm show <id>` shows
   how an existing task is written. Follow what you see.
2. **Area.** Use an area that exists. Add a new one (`alm add area <id> \"<name>\"`) only when the
   person asks for it or agrees to it, or in a goal run (below).
3. **ID.** Leave it out: `alm add task --area <area> --title \"…\"` gives the task a new ID in the
   area: a random short word and three digits, for example `A01-tiger042`, so tasks added on
   other branches never get the same one. Older numbered IDs (`A01-53`) stay as they are. Give
   an ID only to follow a pattern the project already uses, for example a child task
   `A01-03-01` under `A01-03`.
4. **Title.** One observable outcome: what a person can do or see when it is done (\"A customer
   orders a cake for a pick-up day\"), not a technical step (\"add an orders table\").
5. **Description** (`--description`, Markdown), in three short parts:
   - **Outcome.** One or two plain sentences.
   - **Done means.** The checks or demonstrations that prove it.
   - **Notes.** Anything the next person needs: links, open questions, who must decide. Mark as
     open for the person only what is theirs to decide: what users will see and get used to,
     anything hard to undo (data formats, removing things), anything that changes the plan or
     another task, anything with a cost or an outside effect. A routine choice (a name, a file
     layout, how it is built, a small detail with an obvious default) is not an open question:
     whoever works the task decides it and records it with `alm ai-decided`.
   The description is the task's record. A project that keeps a card file per task keeps it
   beside the task, as `tasks/<id>/card.md` (`alm card <id> --from <file>`), linked from the
   description's `Card:` line, never in the project folder (`alm guide move-cards`). As the work
   goes on, keep it up to date with `alm describe <id> --from <file.md>` (its approvals, and what was
   checked, with numbers and logs, under an **Evidence** heading).
6. **Branch.** Say which branch you are on: `--branch <name>` (as your version control tool
   shows it; in Git: `git branch --show-current`; alm reads no version control). The new task gets a base branch, by its real name: the branch it starts
   from and merges back into. When the branch you are on is a task's working branch, the new
   task shares that task's base; otherwise its base is the branch you are on. alm prints which
   and why; `--base <name>` chooses another. When you hold a task on another base, alm says so:
   decide which is right, and change it with `alm base <id> <branch>` if needed.
7. **State.** A new task starts in backlog, in the active iteration: the one marked active, or
   else the first planned one (`--iteration <id>` names another; with no planned iteration
   left, in none). `alm add task` then prints the next steps
   (Task-A03-33, A01-root343): plan it elsewhere if it belongs there (`alm plan <id>
   <iteration>`), or in one at all when no iteration is active, then check it for anything that
   is the person's to decide (what users will see and get used to, anything hard to undo, anything that
   changes the plan or another task, anything with a cost or an outside effect). Nothing: make it
   ready (`alm ready <id>`). Something: leave it in backlog, name each open question in the
   description's Notes, and tell the person. A ready task that turns out to need rethinking goes
   back the same way: `alm backlog <id>` (unheld only: release a held one first), with the open
   questions in its Notes.
8. **One outcome per task.** If it is really two outcomes, add two tasks. More work on the
   outcome of a done or dropped task is not a new task: reopen that one
   (`alm reopen <id> --note \"<why>\"`).

**The exception: a goal run** (approver, 2026-10-16). When the person has given you a goal, an app
to build from their description, and you follow `alm guide goal`, the person approved the plan in
advance by submitting the goal. Then you add the areas and iterations, plan the tasks in them and
make them ready yourself (`alm ready <id>`). Record that approval once, on the goal task (its
title starts with `Goal:`). Anything with an outside effect still waits for the approver
(`alm wait`).

When the project shares its tasks through a version control tool, save and share the files
`alm add task` lists (it prints the Git commands as the example), so others see the new task.
";

const COMMANDS: &str = "# alm commands

Every command takes `--taskbag <path>` (or ALLOMOS_TASKBAG; default: the nearest
`.allomos/taskbag/` above the current folder), `--as <name>` (or ALLOMOS_HOLDER), `--json`
(one JSON object on standard output) and `--request-id <id>` (a one-time number of your own
for a change: run the same command again with the same number, after a crash for example, and
alm returns the first result instead of doing it twice; the same number with a different command
is refused). Exit codes: 0 done, 1 refused by a rule, 2 wrong usage, 3 taskbag busy or not
writable.

`alm` never runs or reads your version control tool. After a change it prints \"Save this change
in your version control tool and share it\", the files that change wrote, and, \"For Git:\", the
exact `git add`, `git commit` and `git push` commands for them (`--json`: `changed_files` and
`git_commands`); save and share them yourself and leave other changes as they are. `alm claim`
asks you to share at once, before you start the work. Changes the person made in the Allomos app
and not shared yet come with their own files and commands, to save as the person's. `alm init`
prints none: save the new taskbag yourself.

See:
- `alm next [--iteration <id>]`: every ready task no one holds (in that iteration), each with its
  handoff note, and the rule for choosing one; then the blocked tasks whose blockers are all
  done. `alm` chooses and moves nothing; you do.
- `alm list [--area <id>] [--iteration <id>] [--state <state>] [--mine] [--by area|iteration]`
  (`--state blocked`, `--state waiting`, …);
  `--without-human-check`: only tasks finished without the human check, not checked since.
  `--by iteration` lists every iteration, empty ones too, each starting with its place in the
  plan (1, 2, 3…, as the app shows it; `place` in `--json`), counted from the plan's order, so it
  changes after `alm move iteration`, and stays the same with `--iteration <id>`.
- `alm human [--iteration <id>] [--area <id>] [--steps]`: what needs a person: **To check**
  (finished without the human check, each with why; `--steps` adds how to check it) and **To
  answer** (waiting on a person, each with what; backlog tasks are not listed), then
  **To review: AI decisions** (tasks in any state with decisions not reviewed yet, each listed).
  `--json`: `to_check`, `to_answer`, `to_review` (each task with its `ai_decisions`).
- `alm show <id>`: one task in full, with its description.
- `alm history <id>`: every recorded change to the task, newest first: when, who, what changed.

Work a task:
- `alm claim <id> --branch <the branch you are on> [--working <name>] [--session-agent <agent>]
  [--session-id <id>] [--session-kind <kind>] [--app-session-id <id>]`: you hold it; state in
  progress. It records the working branch (`--working`, default the branch you are on), gives a
  task without a base the branch you are on, prints the base to start from and merge into with
  the Git commands as the example (`--json`: `branches`, `branch_commands`, `branch_facts`), and says so when
  you are not on the base. `--take` only when its holder has stopped. While card files are kept
  outside the taskbag it refuses (`claim_cards_outside`): move them in first (`alm guide
  move-cards`). The `--session-…` options record your session, so the Allomos app can reopen it
  from the task (stored as given, never checked).
- `alm session <id> [--session-agent <agent>] [--session-id <id>] [--session-kind <kind>]
  [--app-session-id <id>]`: fill in or correct the session a task records, afterwards: each option
  given replaces that part, the others stay. Any state, by the holder or after done
  (`session_missing` when nothing is given).
- After `claim`, `session`, `label`, `wait`, `block`, `release`, `handoff` and `done`, `alm` lists
  as Fact lines what the task is missing (`--json`: `missing`, each with `code`, `fact` and `fix`):
  session details the app needs to reopen the session, the working branch of a task that was
  claimed, and the handoff note of a task done to check. It refuses nothing; run the fix.
- `alm label <id> \"<label>\"`: a step of the ladder.
- `alm needs <id> \"<what>\"`: its next dependency (no text clears it).
- `alm note <id> \"<what the next one needs>\"`: its handoff note (no text clears it).
- `alm wait <id> --on \"<what>\" [--who \"<who>\"]`: state waiting: a person (or an outside
  event) must act; without `--who`, the approver.
- `alm block <id> --by <task-id> [<task-id>…]`: state blocked: those tasks must finish first.
  `alm list`, `show` and `next` show each blocker's state; a dropped blocker is flagged and never
  counts as done. When all are done, `alm next` says so, and moves nothing: check, then
  `alm unblock <id>` (ready, or in progress when someone holds it).
- `alm release <id> --note \"<what the next one needs>\"`: stop short of done.
- `alm done <id> [--without-human-check \"<why>\" --steps <file.md>]`: finished (see
  `alm guide workflow`, step 7). The steps are Markdown for the person who checks it: the test folder, what to
  open, what to click, what they should see. On a task already done, it only records its human
  check again (the old `--without-owner` still works).
- `alm checked <id>`: a person has checked the finished task: it gets the top label and leaves
  To check; the note and steps stay in its history. Run it only when the person says so.
  It does not review the task's AI decisions.
- `alm recheck <id> --reason \"<what changed>\" [--steps <file.md>]`: the work of a checked task
  changed afterwards, so the person's check no longer covers it: the task goes back to To check
  with your reason (and new steps, or the earlier ones), one label below the top. You decide
  when; `alm` does not look at the work.
- `alm ai-decided <id> \"<what and why>\"`: record a routine choice you made on your own in the
  task, in one line, for example `alm ai-decided A01-37 \"Names: alm describe, alm note — match
  alm title\"`. Any task, any state; the task's files do not change, the history keeps it.
- `alm reviewed <id>`: the person has reviewed the task's AI decisions: they leave To review; the
  history keeps them. Run it only when the person says so.
  `alm drop <id>`; `alm ready <id>` (an unheld backlog, waiting or blocked task);
  `alm backlog <id>` (back to backlog, to think it over again: an unheld ready, waiting or blocked
  task; write the open questions in its description's Notes).
- `alm reopen <id> --note \"<why>\"`: more work on a done or dropped task's outcome (it failed its
  check, or the person wants more): it becomes ready, held by nobody, and the history records who
  reopened it, when and why. Its label and its human-check record are cleared (the history keeps
  them); its description and handoff note stay. Any other state is refused. Claim it as usual.

Plan:
- `alm add area [<id>] \"<name>\"`, `alm add iteration [<id>] \"<name>\" [--release …] [--start …]
  [--end …]`: without an ID, the next free one: an area gets A08 after A07, an iteration an
  unused labor word, like I-knit.
- `alm rename area <id> \"<name>\"`; `alm change iteration <id> [--name …] [--release …]
  [--start …] [--end …]` (an empty value clears a release or date). IDs stay.
- `alm rename iteration <old> <new>`: give an iteration a new ID (`I-<word>`, like I-knit); every
  task planned in it moves along.
- `alm move iteration <id> --before <id>` (or `--after <id>`): put an iteration in another place
  in the plan; only the order changes.
- `alm delete area <id>`, `alm delete iteration <id>`: refused while any task is in it, done and
  dropped ones too (plan them elsewhere first).
- `alm add task [<id>] --area <id> --title \"<title>\" [--description \"<markdown>\"]
  --branch <the branch you are on> [--base <name>]`: without an ID, a new word ID in the
  area (`A01-tiger042`); the base branch as `alm guide tasks` says. Read `alm guide tasks` first.
- `alm base <id> <branch>`: change the branch a task starts from and merges into.
- `alm title <id> \"<title>\"`; `alm describe <id> --text \"<markdown>\"` or
  `alm describe <id> --from <file.md>`: a new title or description (`--text \"\"` removes the
  description). Same holder rule as `alm label`; each is recorded in `alm history`. Add
  `--expected-version <v>` (the `version` `alm show` printed) to save only if nobody changed the
  task since you read it; otherwise nothing is saved and alm shows the task as it is now.
- `alm cards`: the card files still outside the taskbag: where a task's `Card:` link points
  outside its folder, and what agents reported (the app shows them as \"N tasks keep their card
  files in the project folder\"). `alm cards found
  <id>=<path> …`: report the card files you found there (paths relative to the project folder);
  `<id>=` withdraws a report. `alm card <id> [--from <file>]`: move a task's card file in, as
  `tasks/<id>/card.md`, and point the description's `Card:` link to it; a different card already
  there is replaced only with `--replace` (to keep a moved card up to date). Same holder rule as
  `alm describe`. See `alm guide move-cards`.
- `alm plan <id> [<iteration>]`; `alm labels \"<a,b,…>\"`; `alm rename label \"<old>\" \"<new>\"`
  (a step of the ladder, and every task that carries it)
- `alm task-language`: the language to write new and changed tasks in (run it before
  `alm add task`, `alm title` and `alm describe`);
  `alm task-language <code|ui>` sets it.

Project folders (decision 0019: one taskbag, several folders, each with its own AGENTS.md):
- `alm upgrade [--confirm]`: move an `allomos/5` taskbag to `allomos/6`, which folders need.
  Without `--confirm` it only says what it will do; afterwards older Allomos versions refuse the
  taskbag, so the person installs the newest Allomos first. Only when the person asks.
- `alm folder list`: each folder's name, path, whether it is there, whether its AGENTS.md has
  Allomos's section, and how many tasks name it.
- `alm folder add <path> [--name <name>]`: bind a folder and write Allomos's section, pointing
  back to the taskbag, into its AGENTS.md (and a CLAUDE.md that imports it, when there is none).
- `alm folder rename <name> <new name>`: here and in every task that names it; the folder on
  disk keeps its name. `alm folder locate <name> <path>`: where it is now (moves no files).
- `alm folder remove <name> [--confirm]`: unbind it. Without `--confirm` it says what it will do;
  tasks stay (the name is taken off them), files stay, only Allomos's section leaves AGENTS.md.
  These print the files changed in the folder, to save in that folder's own repository.
- `alm add task … --folder <name>` (repeatable) or `--no-folder`: required while the taskbag has
  folders; you decide, alm does not guess. `alm folders <id> <name>…` or `--none` changes them.
  `alm list --folder <name>`; inside a folder `alm next` lists only its tasks (`--all-folders`).

Handoffs (what a session leaves in each task so whoever continues does not start blind):
- `alm handoff-rules`: when to write one (quota and context gates, with any override for you;
  they apply only while you hold a task, and reaching one means handoff, then `alm release`);
  `--quota <n> --context <n> [--write-when \"<text>\"] [--agent <name>] [--clear]` change them.
- `alm handoff <id> --as <your session name> --text \"<handoff>\"` (or `--from <file.md>`)
  `[--evidence \"<what>\"]`: hand the task off. The text replaces its handoff note (where it
  stopped, the one next step, who it waits on); `--evidence` adds, dated and signed, what was done
  that must not run twice (an outside effect) under its description's **Evidence**; its history
  records who handed it off. Several tasks: once for each. It reminds you to update
  `project-notes.md` if what is running or a pitfall changed. `alm next` and `alm show <id>` show
  the note. There are no handoff files: old ones in the taskbag's `handoffs/` folder mean the
  taskbag needs updating (`alm next` says so; **Update the taskbag…** or `alm move-in`).

Ask the person:
- `alm confirm --title \"<heading>\" --plan <file.md> [--evidence <file.md>] [--task <id>]`:
  shows your plan in the Allomos app, with its evidence (what you checked and the results) under
  it, and waits; exit 0 means go ahead, anything else means change nothing. A Go ahead covers
  only what was shown (its fingerprint is printed); with `--task` it is recorded in that task's
  history. If the plan changes afterwards, ask again. Use it only on a job the person gave you
  from the Allomos app; otherwise ask in the conversation.
- `alm progress --title \"<job>\" --step \"<what now>\" [--done <n> --of <total>]`, then
  `alm progress --step …` at each step and `alm progress --finished \"<what happened>\"` at the
  end: on a job the person gave you from the Allomos app, the app shows your progress in a popup.
  `alm` only passes it on.
- `alm quota --agent <id> --left <percent> [--resets <when>] [--window 5h|week] [--account <name>]`:
  how much of your account's quota is left, from the figure you read at the handoff gate's quota
  check (Claude Code: `get_usage`). The app shows it beside the agent: the agent that reported
  last comes first, coloured by the share left. `--account` only when you can tell one, never an
  email address or user name; `--resets` is local time (`2026-10-20T17:00`). Kept on this
  computer beside the app's settings, not in the taskbag: nothing to save or share. `alm quota`
  alone lists the reports.

About Allomos:
- `alm brief`: the section for AGENTS.md. `alm guide [terms|customize|tasks|commands|workflow]`:
  this guide.
- `alm guide run-modes [--language <code>]`: the explanation of quiet and normal mode to show the
  person before an automatic or loop run, in the task language (or `--language`); written in en,
  ja and zh-CN, otherwise in English with a line asking you to translate it.
- `alm guide agent-files [--language <code>]`: the one-time procedure that moves the rules every
  AI agent shares into AGENTS.md and leaves each agent's own file a pointer to it; same languages.
- `alm guide goal [--language <code>]`: the goal prompt: given a person's description of an app,
  plan it into areas, iterations and ready tasks and build it release by release, recording every
  step with `alm`; same languages.
- `alm guide github-release`: what to do when the approver writes `alm guide github release` in the
  conversation: publish a new GitHub release with release notes, check it, and (with `install`)
  install it on this computer, from the facts in the project notes' `## Releases` section. Words
  after the topic (`minor`, `major`, `install`) are the guide's, for you to read. English only so
  far.
- `alm guide move-cards`: how to find card files kept outside the taskbag, report them, and
  move them in beside their tasks after the person's Go ahead.
- `alm init`: create a taskbag (only when there is none), in `.allomos/taskbag/` of the current
  folder, or where `--taskbag` says.
- `alm move-in [--language <code>]`: the request the Allomos app's **Update the taskbag…** types
  into an agent. When `alm` says the taskbag was written by an older Allomos
  (`taskbag_to_move_in`): moving its tasks, and their card files, into a new taskbag, like
  another task system; tell the person first. Otherwise: moving card files kept outside the
  taskbag in beside their tasks (`alm guide move-cards`). It changes nothing; the person confirms
  the plan with `alm confirm`. `--translate [<code>] [--name <name>]` also asks to translate the
  taskbag's contents into that language (alone: the request's language) and make it the task
  language, as the app's language menu there does.
- `alm translate <code> [--name <name>] [--language <code>]`: the request the Allomos app's
  **Translate the taskbag…** types into an agent: rewrite every task's title, description and
  handoff note, the card files and the project notes in that language (any language code, such
  as `vi` or `pt-BR`), and make it the task language (`alm task-language <code>`). It changes
  nothing; the person confirms the plan with `alm confirm`.
";

/// `alm guide move-cards` (Task-A00-30, approver 2026-10-16): Allomos reads only the tasks'
/// `Card:` links (Task-A01-51); the agent finds the other card files, reports them with
/// `alm cards found`, and moves them in after the person's Go ahead (`alm confirm`).
const MOVE_CARDS: &str = "# Moving card files in beside their tasks

A card file holds one task's detail in a file of its own (its outcome, approvals, notes and
evidence). In Allomos it lives in the taskbag, beside the task's own files, as
`tasks/<id>/card.md`, and the task's description links to it with a line
`Card: [card.md](card.md)`, so a task is kept in one place and the Allomos app opens its card.
`alm` and the app read only each task's `Card:` link: `alm cards` lists the tasks whose link
points outside the task's folder, and the app shows them. You find the rest, and nothing moves
until the person says Go ahead.

1. Look first, change nothing. Start from `alm cards`: the tasks whose `Card:` link points
   outside the taskbag, and what agents reported. Then find the descriptions that link to a card
   file in another way it does not read (`alm list --json` gives every description). Also look in the project folder for card files named after task IDs (for
   example `docs/tasks/Task-A01-02-*.md`) that no description links to. A card file belongs to the
   task its link or its name names. Card files without a task are never moved: list them for the
   person. A description that holds a whole card merged into it may be split back the same way:
   the card's part becomes the card file, the description keeps the rest and the `Card:` link.
2. Report what you found that `alm cards` does not list yet, or where a link names the wrong
   file, so the person sees it even before you ask: `alm cards found <id>=<path> <id>=<path> …`
   (each path relative to the project folder; several in one command; a report wins over the
   task's link). The Allomos app shows \"N tasks keep their card files in the project folder\"
   with **Update the taskbag…**, which opens an agent with this guide's request typed in (`alm
   move-in` prints it). `alm cards` lists them; `alm cards found <id>=` withdraws a report.
3. The one question. Write the plan in Markdown to a file outside the project: every card file
   and the task it goes to (`tasks/<id>/card.md`); the links that change (each description's
   `Card:` link; links inside a card that are relative to its old place; links in other documents
   to a card's old place); the originals you will remove from the project folder; card files
   without a task; tasks you cannot change (an open task held by another session: its holder
   moves it later); and how you will commit. Then `alm confirm --title \"Update the taskbag\"
   --plan <file>`. Exit code 0 means go ahead with the whole plan; any other means change
   nothing: tell the person and stop. (In a quiet run, do not use `alm confirm`: park the plan as
   a question with `alm wait`.)
4. Move each card. When it links to other files by relative paths, first write a copy outside the
   project whose links work from `tasks/<id>/`: a link to another card becomes `../<ID>/card.md`;
   a link to a project file becomes its path from the task's folder (the form the `Card:` links
   used, for example `../../../../docs/x.md` when the taskbag is the project's
   `.allomos/taskbag/`); a link to a file that no longer exists becomes plain text. Then
   `alm card <id> --from <copy>` (without `--from`: the reported file, else the linked one, as it
   is). `alm` writes
   `tasks/<id>/card.md`, points the description's `Card:` link to it (adding one at the top when
   there is none) and clears the report; it never replaces a different `card.md` (`card_exists`).
   A finished or dropped task can be changed whoever is named as its holder.
5. Finish what the plan names: remove the originals from the project folder (in Git: `git rm`), change the
   documents that linked to a card's old place, and remove the cards folder once it is empty.
   Save and share the taskbag's changes (the files `alm` listed; it printed the Git commands as the
   example) and the project's changes, each in its own repository. Check: `alm cards` lists none, `alm list --json` reports no issues, and
   every moved task's `Card:` link opens its `card.md`.
6. Tell the person what moved, what did not and why, and the card files without a task.
";

const WORKFLOW: &str = "# How to work the tasks

1. Start with `alm next` (or `alm list`). Choose by its rule: the task the person named, if any;
   otherwise one from the active iteration (listed first: the one marked active, or else the
   first planned one), else the earliest iteration, whose next dependency is met. Read its handoff
   note first (`alm show <id>`), and the project notes' Running, Pitfalls and Repositories, which
   `alm next` shows (the whole file is the taskbag's `project-notes.md`).
   **Taking over from a handoff** (`alm next` and `alm show` repeat these two rules with a
   handoff note): check the real state before trusting it: the working copy, branch and running processes it
   names (as your version control tool shows them; in Git: `git worktree list`, `git status -sb`;
   and listening ports). What the previous session still
   holds is not yours to change or remove; if unsure, ask the person (in a quiet run, park the
   question with `alm wait`). What the task's **Evidence** lists as already done is not done
   again: a change of agent, account or session keeps the approvals
   and what was done; it does not authorize repeating it.
2. Hold one task at a time. Pull first (get the latest changes), then `alm claim <id> --as <your
   session name> --branch <the branch you are on>` (as your version control tool shows it; in
   Git: `git branch --show-current`; alm reads no version control), and save and share the claim
   right away with your version control tool (it prints the Git commands as the example): until
   your claim is shared, other sessions do not see it and may take the task too. A task someone else holds is not yours to change.
   When you claim, pass what the Allomos app needs to reopen your session: `--session-agent
   <your agent's id>`, `--session-id <your session's ID>`, and whatever else your agent needs to
   reopen it (for example, `--session-kind desktop --app-session-id <its local_… ID>` when you
   run in the Claude desktop app). alm stores what you pass and checks nothing; the task keeps
   it after release and done, and the next claim replaces it.
   **Branches.** Every task has a base branch, by its real name (`main`, `master`,
   `storage-v2`…): it starts from it and merges back into it; merging one base branch into
   another is a task of its own. The claim records your working branch, the one you work on
   (`--working <name>` when it is not the one you are on, for example a new branch for a
   worktree of its own),
   prints which base to start from and merge into with the Git commands as the example, and says
   so when you
   are not on the base. A task without a base gets the branch you are on.
   **Always work a task in a worktree of its own** (a separate working copy on the task's own
   branch; in Git: `git worktree add <folder> -b <working branch> <base>`, which `alm claim`
   prints), never in the main checkout: the main checkout stays on the base branch, so no
   session leaves it on a task's branch and several sessions can work at once. Merge from the
   main checkout, where the base is checked out. Once the task's branch is merged into its
   base, remove your own worktree and the merged branch; never another session's.
   **Before you plan: the size of the files you will change** (Task-A03-tide388; `alm claim`
   repeats it). A source file your plan changes that is over about 1000 lines (tests do not
   count; the project notes may name another limit) needs a look: is the part you will change
   tangled with unrelated things? If so, the plan includes a refactoring of only that part (for
   example, moved into a module of its own), never the whole file, so files get smaller task by
   task. By default it is done in the same task, as a commit of its own before the feature
   commit, with the tests passing and the behaviour unchanged after it. When the refactoring is
   bigger than the feature itself, or reaches files outside the task, add a task for it first
   and link them with `alm block <this task> --by <new task>`. alm only reminds: it measures no
   file and stops nothing.
3. As you prove each step, `alm label <id> \"<label>\"`, and keep its next dependency current
   with `alm needs`.
4. Stopped: when a person (or an outside event) must act, `alm wait <id> --on \"<what>\"
   [--who \"<who>\"]`; when another task must finish first, `alm block <id> --by <task-id>` (if
   a person must act too, say so with `alm needs`). Blocked and waiting tasks are never in
   `alm next`'s list: they are skipped by their state. When `alm next` says all of a task's
   blockers are done, check it, then `alm unblock <id>`.
5. Write your handoff when `alm handoff-rules` says (near the quota or context gate while you
   hold a task, then release it with `alm release <id> --note` naming the working branch, so the
   next session continues it; and after every meaningful step), into each task you worked: `alm handoff <id> --as <your session name>
   --text \"<where it stopped, the one next step, who it waits on>\"`, in plain words first, for
   any reader: you do not know who takes over, an agent or a person. It replaces the task's
   handoff note, so write the whole of it each time. Each operation with an outside effect that
   must not run twice goes in with `--evidence \"<what>\"` (under the description's
   **Evidence**); other evidence with `alm describe`.
6. Stopping before it is done (for example your session ends): `alm release <id> --note \"<what
   the next one needs>\"`.
7. Finished: once the task has the top label of the ladder (a person has checked it: the
   human check), `alm done <id>`. When the work is finished and only a person's hands-on check
   is left, finish it below the top label, in quiet and normal mode alike (step 8):
   `alm done <id> --without-human-check \"<why>\" --steps <file.md>`; say so in your handoff.
   Do not leave such a task waiting: `alm wait` is only for a real question, an approval or an
   outside effect (step 4). Write the steps so a person can pick it up and test
   it: the test folder, what to open, what to click, what they should see. The task keeps its
   label, the note and the steps, so a person finds it with `alm human`; once they have checked
   it, `alm checked <id>`. When a finished task needs more work on the same outcome (it failed
   its check, or the person asks for more), reopen it instead of adding a task:
   `alm reopen <id> --note \"<why>\"` makes it ready again, held by nobody, with the reason in
   its history; then claim it as usual.
8. Before an automatic or loop run begins (a card run, `/loop`, several tasks one after
   another), ask the person to choose quiet or normal mode: show them what
   `alm guide run-modes` prints (in the task language), as it is, and do not start until they
   choose. Write the choice in your handoff (in the task's handoff note: \"Run mode: quiet\" or
   \"Run mode: normal\", who chose it and when) and follow it until the run ends. Both modes
   finish a task the same way (step 7); they differ only in what you do when you need the
   person during the run. In normal mode you stop and ask. In quiet mode (a quiet run),
   never stop to wait for the person. A routine choice: decide it,
   record it with `alm ai-decided <id> \"<what and why>\"`, and carry on. A real question for the
   person: park that task with `alm wait <id> --on \"<the question>\"` (it shows in `alm human`
   under To answer) and go on with the next ready task. Do not use `alm confirm` in such a run: it
   would wait until it times out.
9. **A goal run** (`alm guide goal`): when the person gives you a goal, an app to build from their
   description, follow the goal prompt `alm guide goal` prints (in the task language). It is the
   one exception to asking the person before planning (approver, 2026-10-16): submitting the goal
   approved the plan in advance, recorded once on the goal task (its title starts with `Goal:`),
   so you add the areas and iterations and make your own tasks ready. Anything with an outside
   effect still waits for the approver (`alm wait`). There is no spend cap: the quota gate of
   `alm handoff-rules` is the limit. Given the goal again, continue from `alm next`.

Rules:
- Never edit the taskbag's files by hand, and never invent progress, verification or approvals.
  One exception: keep `project-notes.md` true. Update it when you start or stop a server or process, create or change
  a hosted resource or a remote repository, learn a pitfall that would change what the next
  session does, or a credential moves; change lines in place, remove lines no longer true, and
  save and share that file with your version control tool yourself.
- Never write credentials or personal details (a person's or an account's email address, account
  or user name, phone number, home address, local network address) into the project notes, a
  handoff, a task, a document or the code: write a placeholder such as
  `<CLOUDFLARE_ACCOUNT_EMAIL>` and how to look it up (`npx wrangler whoami`). Version history
  keeps whatever was written, even after it is removed.
- Every new piece of work (a new card, a new outcome) gets its own task: `alm guide tasks`, then
  `alm add task`.
- Before adding a task or changing its title or description, run `alm task-language` and write in
  that language.
- A change the person must agree to (an outside effect, or anything that is not a routine choice:
  below): ask in the conversation. Only on a job the person gave you from the Allomos app (they do
  not see your conversation) show your plan with `alm confirm` instead, with the evidence
  (`--evidence`) and the task (`--task`), and change nothing unless it exits 0. In a quiet run,
  park it with `alm wait` instead (step 8). A Go ahead covers only the plan shown: if it changes,
  ask again.
- Routine choices inside a task (a name, a file layout, how something is built inside the task, a
  small detail with an obvious default; anything cheap to undo) are yours: decide, carry on, and
  record each with `alm ai-decided <id> \"<what and why>\"`, so the approver can review it with
  `alm human`. What users will see and get used to, anything hard to undo (data formats, removing
  things), anything that changes the plan or another task, and anything with a cost or an outside
  effect goes to the approver first (`alm wait`). In a goal run (step 9) the plan was approved in
  advance; outside effects still go to the approver.
- After every `alm` change, save and share the files it lists with your version control tool (it
  prints the Git commands as the example): only that change's files. Your other work is saved as
  usual.
";

const RUN_MODES_EN: &str = "# Run modes: quiet or normal

For the agent: before an automatic or loop run begins (a card run, `/loop`, several tasks one
after another), show the person everything below the line, as it is, and ask them to choose
quiet or normal mode. Do not start until they choose. Write the choice in your handoff (in the
task's handoff note: \"Run mode: quiet\" or \"Run mode: normal\", who chose it and when) and
follow it until the run ends.

---

Before this run starts, please choose how I will work: **quiet mode** or **normal mode**.

In both modes, a task whose work is finished and only waits for you to try it by hand ends the
same way: it is marked done, to check, with the steps for checking it
(`--without-human-check`). The modes differ only in what I do when I need you during the run:
stop and ask, or park the question and go on.

## Quiet mode: I go on without you

- **What I do:** I work the tasks one after another and never stop to ask you. I make routine
  choices myself (a name, a file layout, a small detail with an obvious default) and record each
  one (`alm ai-decided`), so you can review it later.
- **What I don't do:** I don't decide what is yours to decide: what users will see and get used
  to, anything hard to undo, anything that changes the plan or another task, anything with a cost
  or an outside effect. Such a question parks its task as waiting (`alm wait`), and I go on with
  the next ready task without waiting for an answer.
- **When you are interrupted:** not at all until the run is over.
- **How long it can run unattended:** until no ready task is left, or until my session's quota or
  context runs out. I keep my handoff up to date, so anyone can pick the work up.

When the run is over:

1. Run `alm human` (or the Needs a human filter in the Allomos app) and work through its three
   groups:
   - **To answer:** answer each parked question, then mark the task ready again
     (`alm ready <id>`), so a later run picks it up.
   - **To review: AI decisions:** read each decision; keep it, or overturn it by changing the
     task; then mark them reviewed (`alm reviewed <id>`, or the Reviewed button in the app).
   - **To check:** follow each task's check steps. When it works, mark it checked
     (`alm checked <id>`, or the Checked button in the app). When it does not, reopen it with
     what is wrong (`alm reopen <id> --note \"<what is wrong>\"`, or the Reopen… button in the
     app): it becomes ready again, with the reason in its history, so a later run picks it up.
2. Read the handoff notes of the tasks the run worked (`alm show <id>`; `alm history <id>` shows
   who handed each off) for anything left running or unfinished.

## Normal mode: I ask as usual

- **What I do:** I work as usual. Routine choices I still make and record myself; whenever I
  need you (a question only you can answer, an approval, an outside effect), I stop and ask. A
  task that only waits for your check by hand does not stop the run: it is marked done, to
  check, as above.
- **What I don't do:** I don't go past a question without your answer.
- **When you are interrupted:** at each such question or approval.
- **How long it can run unattended:** only until the next question: the run pauses there until
  you answer.

Please stay reachable while it runs: it pauses at each question until you answer. When it is
over, check the finished tasks in `alm human` (To check), as described above.
";

const RUN_MODES_JA: &str = "# 実行モード：静かモードか通常モードか

エージェントへ：自動実行やループ実行（カード・ラン、`/loop`、複数のタスクを続けて進めるもの）を
始める前に、下の線より後ろをそのまま相手に見せ、静かモードか通常モードかを選んでもらいます。
選ばれるまで始めません。選ばれたモードはハンドオフに書き（タスクの引き継ぎメモに
「Run mode: quiet」または「Run mode: normal」と、誰がいつ選んだか）、実行が終わるまでそれに従います。

---

この実行を始める前に、私の進め方を選んでください：**静かモード**か**通常モード**か。

どちらのモードでも、作業が終わり、あなたが手で試す確認だけが残ったタスクは同じように終えます：
確認の手順を付けて、完了・確認待ちにします（`--without-human-check`）。二つのモードの違いは、
実行中にあなたが必要になったとき、止まって聞くか、質問を残して先へ進むかだけです。

## 静かモード：あなたを待たずに進めます

- **すること：** タスクを一つずつ続けて進め、あなたに聞くために止まることはありません。決まりきった
  選択（名前、ファイルの配置、自然な既定がある小さな点）は私が決め、あとで見直せるよう一つずつ記録
  します（`alm ai-decided`）。
- **しないこと：** あなたが決めるべきことは決めません。利用者が目にして慣れていくもの、元に戻しにくい
  こと、計画や他のタスクを変えること、費用や外部への影響があること。そうした質問があれば、その
  タスクを待機中にして（`alm wait`）、答えを待たずに次の準備完了のタスクへ進みます。
- **あなたが呼ばれるとき：** 実行が終わるまで一度もありません。
- **無人で続けられる長さ：** 準備完了のタスクがなくなるまで、またはこのセッションの利用枠や
  コンテキストが尽きるまで。誰でも引き継げるよう、ハンドオフは常に最新にしておきます。

実行が終わったら：

1. `alm human`（または「肩代」アプリの「人の対応が必要」フィルター）を開き、三つのグループを
   順に片付けます。
   - **To answer（回答待ち）：** 待機中の質問に一つずつ答え、そのタスクを準備完了に戻します
     （`alm ready <id>`）。次の実行がそれを拾います。
   - **To review: AI decisions（レビュー待ちの AI の決定）：** 決定を一つずつ読み、そのままにするか、
     タスクを変えて覆します。終わったらレビュー済みにします（`alm reviewed <id>`、またはアプリの
     「レビュー済み」ボタン）。
   - **To check（確認待ち）：** 各タスクの確認手順に従って試します。動けば確認済みにします
     （`alm checked <id>`、またはアプリの「確認済み」ボタン）。動かなければ、何がおかしいかを書いて
     再開します（`alm reopen <id> --note \"<何がおかしいか>\"`、またはアプリの「再開…」ボタン）。タスクは理由を履歴に残して準備完了に戻り、次の実行がそれを拾います。
2. この実行で扱ったタスクの引き継ぎメモ（`alm show <id>`。誰が引き継いだかは `alm history <id>`）を読み、
   動いたままのものや終わっていないものがないか確かめます。

## 通常モード：いつもどおり聞きます

- **すること：** いつもどおり進めます。決まりきった選択は私が決めて記録しますが、あなたが必要な
  とき（あなたにしか答えられない質問、承認、外部への影響）は止まって聞きます。手で試す確認だけを
  待つタスクでは実行は止まりません。上のとおり、完了・確認待ちにします。
- **しないこと：** あなたの答えなしに質問の先へは進みません。
- **あなたが呼ばれるとき：** そうした質問や承認のたびに。
- **無人で続けられる長さ：** 次の質問まで。そこで実行は止まり、あなたの答えを待ちます。

実行中は連絡がつくようにしておいてください。質問のたびに、答えるまで実行は止まります。実行が
終わったら、上で説明したとおり、`alm human`（To check）で終わったタスクを確認してください。
";

const RUN_MODES_ZH_CN: &str = "# 运行模式：安静模式还是普通模式

给智能体：在自动运行或循环运行（卡片连跑、`/loop`、一个接一个地做多个任务）开始之前，把下面
分隔线之后的内容原样给对方看，请对方选择安静模式或普通模式。对方选定之前不要开始。把选择写进
你的交接（在任务的交接备注里写「Run mode: quiet」或「Run mode: normal」，以及谁、何时选的），
并一直照此运行到结束。

---

这次运行开始之前，请选择我的工作方式：**安静模式**还是**普通模式**。

两种模式下，工作已做完、只等你亲手检查的任务，都以同一种方式结束：标为已完成、待检查，并附上
检查步骤（`--without-human-check`）。两种模式的区别只在于运行中需要你时，我是停下来问你，还是
把问题搁置、接着往下做。

## 安静模式：我不等你，继续往下做

- **我会做的：** 一个接一个地处理任务，从不停下来问你。常规的选择（名称、文件布局、有明显默认值的
  小细节）由我自己决定，并逐条记录（`alm ai-decided`），方便你之后审阅。
- **我不会做的：** 不替你决定该由你决定的事：用户会看到并习惯的东西、难以撤销的事、会改变计划或
  其他任务的事、有费用或对外有影响的事。遇到这类问题，我把那个任务设为等待中（`alm wait`），
  不等回答，接着做下一个就绪的任务。
- **什么时候打扰你：** 运行结束之前一次也不会。
- **能无人值守运行多久：** 直到没有就绪的任务，或者我这个会话的额度或上下文用完为止。我会让交接
  一直保持最新，谁都可以接着做。

运行结束后：

1. 运行 `alm human`（或在「换肩」应用里用「需要人处理」筛选），依次处理三组：
   - **To answer（待回答）：** 逐个回答被搁置的问题，然后把任务重新设为就绪（`alm ready <id>`），
     之后的运行会接着做。
   - **To review: AI decisions（待审阅的 AI 决定）：** 逐条阅读决定；保留它，或修改任务来推翻它；
     然后标为已审阅（`alm reviewed <id>`，或应用里的「已审阅」按钮）。
   - **To check（待检查）：** 按每个任务的检查步骤去试。没问题就标为已检查（`alm checked <id>`，
     或应用里的「已检查」按钮）。有问题就写明哪里不对并重新打开它（`alm reopen <id> --note \"<哪里不对>\"`，
     或应用里的「重新打开…」按钮）：它会重新变为就绪，原因记在历史里，之后的运行会接着做。
2. 阅读这次运行做过的任务的交接备注（`alm show <id>`；谁交接的见 `alm history <id>`），看看有没有仍在运行或没做完的东西。

## 普通模式：照常提问

- **我会做的：** 照常工作。常规的选择仍由我决定并记录；每当需要你（只有你能回答的问题、批准、
  对外有影响的事）时，我会停下来问你。只等你亲手检查的任务不会让运行停下：像上面说的那样，标为
  已完成、待检查。
- **我不会做的：** 没有你的回答，不会越过问题继续做。
- **什么时候打扰你：** 每遇到这样的问题或批准时。
- **能无人值守运行多久：** 只到下一个问题为止：运行会停在那里，直到你回答。

运行期间请保持能联系到你：每遇到一个问题，运行都会停下，直到你回答。运行结束后，像上面说的那样，
在 `alm human`（To check）里检查已完成的任务。
";

/// `alm guide run-modes` in `language` (a task language such as `ja` or `zh-CN`), or `None` when
/// it is not written in that language (the agent then translates the English text faithfully).
/// The person reads it, so it is written in every language of [`RUN_MODES_LANGUAGES`].
pub fn run_modes(language: &str) -> Option<&'static str> {
    match written_in(language)? {
        "ja" => Some(RUN_MODES_JA),
        "zh-CN" => Some(RUN_MODES_ZH_CN),
        _ => Some(RUN_MODES_EN),
    }
}

const AGENT_FILES_EN: &str = "# Agent files: one set of shared rules, in AGENTS.md

Every AI agent working on this project reads one set of shared rules, in AGENTS.md. Each
agent's own rule file only points to AGENTS.md and holds what is truly its own. Do the steps
below once for each agent: when Allomos is set up in a project (for the agent doing the setup
and every agent file already there), and whenever you are an agent not listed under
`## Agents set up` in the project notes, or your own rule file does not point to AGENTS.md. Do it
before other work. `alm` checks nothing here: you look and change the files; the person settles
contradictions.

1. **Find every agent rule file in the project.** For example: AGENTS.md, CLAUDE.md, GEMINI.md,
   `.cursor/rules/` (or an older `.cursorrules`), `.github/copilot-instructions.md`,
   `.windsurfrules`, and any other file an AI tool reads as its instructions. These are
   examples, not a closed list: you know your own tool's file; include it even if it is not
   named here. Look in the project folder (and the subfolders a tool reads), not in the
   person's home folder.
2. **Move the rules every agent shares into AGENTS.md.** A rule that applies to any agent on the
   project (how to build, test and commit, where tasks live, what needs approval) belongs in
   AGENTS.md, once. Each agent's own file keeps only:
   - a pointer to AGENTS.md: an import line where the tool supports one (Claude Code: a line
     holding just `@AGENTS.md`), otherwise one sentence: \"Read and follow AGENTS.md.\";
   - what applies to that agent alone (its model or effort settings, its own tools and
     commands).
   A tool that reads AGENTS.md by itself needs no pointer, but shared rules in its own file
   still move. Create a file for your own tool only when it reads nothing else. Do not change
   what any rule means, and do not copy Allomos's section (`alm brief`) into an agent's own file.
3. **Contradictions are not yours to settle.** When two files say different things about the
   same matter (one says push after every commit, another says never push), do not choose
   between them: leave both where they are, list each pair for the person (the files, and the
   two rules word for word) and ask which one holds. In a quiet run, park the question with
   `alm wait <id> --on \"<the question>\"` on the task you hold, and move everything else.
4. **Record the agent as set up** in the taskbag's `project-notes.md`, under `## Agents set up`
   (add that section when it is missing): one line per agent, with its file and the date, for
   example `- Claude Code: CLAUDE.md (imports AGENTS.md), 2026-10-16`. Then save only those
   files in your version control tool. In the project: AGENTS.md and the agent files you changed
   (in Git: `git add <those files> && git commit -m \"Shared agent rules in AGENTS.md\" -- <those files>`).
   In the taskbag: `project-notes.md`
   (in Git: `git -C <taskbag> add project-notes.md && git -C <taskbag> commit -m \"Agents set up\" -- project-notes.md`).
   Leave every other change as it is. Share (push) as the project's rules say; when they say
   nothing, only when the person asks.
5. **Tell the person** what moved, which files now point to AGENTS.md, and the contradictions
   waiting for their answer.

From then on:
- Shared rules live only in AGENTS.md; never add a shared rule to one agent's own file.
- After changing shared rules, tell the person which running sessions still use the old ones:
  those holding tasks in progress (`alm list --state in_progress`) and the working copies they work
  in (in Git: `git worktree list`). A session reads the new rules when it starts again.
";

const AGENT_FILES_ZH_CN: &str = "# 智能体文件：一套共用规则，放在 AGENTS.md

在这个项目上工作的每个 AI 智能体都读同一套共用规则，放在 AGENTS.md 里。每个智能体自己的规则文件
只指向 AGENTS.md，并只保留真正属于它自己的内容。下面的步骤对每个智能体做一次：在项目里设置
Allomos 时（为做设置的智能体，以及已有的每个智能体文件），以及每当你是一个没有列在项目笔记
`## Agents set up` 一节下的智能体，或者你自己的规则文件没有指向 AGENTS.md 时。在做其他工作之前
完成。`alm` 在这里什么都不检查：由你查看并修改文件；有矛盾时由用户决定。

1. **找出项目里的每个智能体规则文件。** 例如：AGENTS.md、CLAUDE.md、GEMINI.md、
   `.cursor/rules/`（或较旧的 `.cursorrules`）、`.github/copilot-instructions.md`、
   `.windsurfrules`，以及任何 AI 工具当作自己说明来读的其他文件。这些只是例子，不是完整的清单：
   你知道你自己工具的文件；即使这里没有列出，也要算上。在项目文件夹（以及工具会读的子文件夹）里
   找，不要去用户的主文件夹找。
2. **把每个智能体共用的规则移到 AGENTS.md。** 适用于项目上任何智能体的规则（如何构建、测试和
   提交，任务放在哪里，什么需要批准）放在 AGENTS.md 里，只写一次。每个智能体自己的文件只保留：
   - 指向 AGENTS.md 的一行：工具支持导入时用导入行（Claude Code：只含 `@AGENTS.md` 的一行），
     否则用一句话：“Read and follow AGENTS.md.”；
   - 只适用于这个智能体的内容（它的模型或推理强度设置，它自己的工具和命令）。
   自己会读 AGENTS.md 的工具不需要指向行，但它自己文件里的共用规则仍要移走。只有当你的工具不读
   其他任何文件时，才为它新建文件。不要改变任何规则的意思，也不要把 Allomos 的一节（`alm brief`）
   复制到某个智能体自己的文件里。
3. **矛盾不由你来定。** 两个文件对同一件事说法不同时（一个说每次提交后推送，另一个说从不推送），
   不要二选一：两条都原样留着，把每一对列给用户（哪两个文件，两条规则逐字照录），问哪条为准。
   在安静运行中，用 `alm wait <id> --on \"<the question>\"` 把问题挂在你持有的任务上，其余照常移动。
4. **记录这个智能体已设置好**：在任务包的 `project-notes.md` 里 `## Agents set up` 一节下（没有就
   添加这一节），每个智能体一行，写明它的文件和日期，例如
   `- Claude Code: CLAUDE.md (imports AGENTS.md), 2026-10-16`。然后用你的版本控制工具只保存这些文件。在项目里：
   AGENTS.md 和你改过的智能体文件
   （用 Git 时：`git add <those files> && git commit -m \"Shared agent rules in AGENTS.md\" -- <those files>`）。
   在任务包里：`project-notes.md`
   （用 Git 时：`git -C <taskbag> add project-notes.md && git -C <taskbag> commit -m \"Agents set up\" -- project-notes.md`）。
   其他改动保持原样。按项目规则共享（推送）；规则没说时，只在用户要求时共享。
5. **告诉用户**移动了什么，哪些文件现在指向 AGENTS.md，以及哪些矛盾在等他们回答。

此后：
- 共用规则只放在 AGENTS.md；不要把共用规则加到某个智能体自己的文件里。
- 改了共用规则之后，告诉用户哪些正在运行的会话还在用旧规则：持有进行中任务的会话
  （`alm list --state in_progress`）和它们工作的工作副本（用 Git 时：`git worktree list`）。会话重新开始时
  才会读到新规则。
";

const AGENT_FILES_JA: &str = "# エージェントのファイル：共通ルールは一つ、AGENTS.md に

このプロジェクトで働くすべての AI エージェントは、AGENTS.md にある一つの共通ルールを読みます。
各エージェント自身のルールファイルは AGENTS.md を指すだけで、本当にそのエージェントだけのことを
持ちます。下の手順はエージェントごとに一度行います。プロジェクトに Allomos を設定するとき（設定を
するエージェントと、すでにあるすべてのエージェントのファイルについて）、そして、プロジェクトノートの
`## Agents set up` の節に載っていないエージェントであるとき、または自分のルールファイルが
AGENTS.md を指していないときです。ほかの作業の前に行います。ここで `alm` は何も確かめません。
ファイルを見て変えるのはあなたで、食い違いを決めるのはユーザーです。

1. **プロジェクトのエージェント用ルールファイルをすべて見つけます。** たとえば AGENTS.md、
   CLAUDE.md、GEMINI.md、`.cursor/rules/`（古い `.cursorrules`）、
   `.github/copilot-instructions.md`、`.windsurfrules`、そのほか AI ツールが自分への指示として
   読むファイル。これは例であって、すべてではありません。自分のツールのファイルは知っているはず
   なので、ここになくても含めます。探すのはプロジェクトのフォルダー（とツールが読むサブフォルダー）
   で、ユーザーのホームフォルダーではありません。
2. **すべてのエージェントに共通のルールを AGENTS.md に移します。** プロジェクトのどのエージェント
   にも当てはまるルール（ビルド、テスト、コミットのしかた、タスクの置き場所、承認が要るもの）は
   AGENTS.md に一度だけ書きます。各エージェント自身のファイルに残すのは次だけです。
   - AGENTS.md を指す行：ツールが読み込みに対応していれば読み込みの行（Claude Code では
     `@AGENTS.md` だけの行）、そうでなければ一文「Read and follow AGENTS.md.」。
   - そのエージェントだけに当てはまること（モデルや推論の強さの設定、そのツール自身の道具や
     コマンド）。
   AGENTS.md を自分で読むツールには指す行は要りませんが、そのファイルにある共通ルールは移します。
   自分のツール用のファイルを新しく作るのは、そのツールがほかに何も読まないときだけです。どの
   ルールも意味を変えず、Allomos の節（`alm brief`）をエージェント自身のファイルにコピーしません。
3. **食い違いはあなたが決めません。** 二つのファイルが同じことについて違うことを言っているとき
   （一方はコミットのたびにプッシュ、もう一方はプッシュしない）、どちらかを選びません。両方を
   そのまま残し、組ごとにユーザーへ示して（どのファイルか、二つのルールをそのまま）、どちらに従うか
   尋ねます。静かな実行では、持っているタスクに `alm wait <id> --on \"<the question>\"` で質問を
   残し、ほかはそのまま移します。
4. **エージェントを設定済みとして記録します。** タスクバッグの `project-notes.md` の
   `## Agents set up` の節（なければ加えます）に、エージェントごとに一行、そのファイルと日付を
   書きます。たとえば `- Claude Code: CLAUDE.md (imports AGENTS.md), 2026-10-16`。それから、
   それらのファイルだけをバージョン管理ツールで保存します。プロジェクトでは AGENTS.md と変えたエージェントのファイル
   （Git なら：`git add <those files> && git commit -m \"Shared agent rules in AGENTS.md\" -- <those files>`）。
   タスクバッグでは `project-notes.md`
   （Git なら：`git -C <taskbag> add project-notes.md && git -C <taskbag> commit -m \"Agents set up\" -- project-notes.md`）。
   ほかの変更はそのままにします。共有（プッシュ）はプロジェクトのルールに従い、ルールに何もなければ
   ユーザーに頼まれたときだけにします。
5. **ユーザーに伝えます：** 何を移したか、どのファイルが AGENTS.md を指すようになったか、どの
   食い違いが答えを待っているか。

これから先：
- 共通ルールは AGENTS.md にだけ置きます。共通ルールをあるエージェント自身のファイルに足しません。
- 共通ルールを変えたら、まだ古いルールで動いているセッションをユーザーに伝えます：進行中の
  タスクを持つセッション（`alm list --state in_progress`）と、それが作業している作業コピー
  （Git なら：`git worktree list`）。セッションは次に始めるときに新しいルールを読みます。
";

/// `alm guide agent-files` in `language`, or `None` when it is not written in that language.
pub fn agent_files(language: &str) -> Option<&'static str> {
    match written_in(language)? {
        "ja" => Some(AGENT_FILES_JA),
        "zh-CN" => Some(AGENT_FILES_ZH_CN),
        _ => Some(AGENT_FILES_EN),
    }
}

/// The version of the goal prompt (`alm guide goal`, Task-A03-25-01). Raise it whenever the
/// prompt's wording changes; every text names it as `goal prompt v<n>`, and the goal task records
/// the version it was planned with.
///
/// 1: the four phases (clarify once, plan, build, finish), `/goal` with a condition judged from the
/// conversation alone, and continuing from `alm next` (Task-A03-25-01).
/// 2: every finished task, the goal task too, ends done and to check
/// (`--without-human-check`) in either run mode, not waiting (Task-A03-31).
pub const GOAL_PROMPT_VERSION: u32 = 3;

const GOAL_EN: &str = "# Goal run: build an app from a description (goal prompt v3)

For the agent: this is the goal prompt. A person has described an app they want (their words
come with this prompt). You plan it into areas, iterations and tasks, make the tasks ready
yourself, and build it iteration by iteration, recording every step with `alm`, so the person, or
another agent taking over, sees exactly where the work stands. Follow the four phases in order.
Write tasks in the task language (`alm task-language`); IDs, commands and code stay as they are.

**Goal-run mode.** Submitting the goal approved the plan in advance (approver, 2026-10-16): in
this run you add the areas and iterations and make your own tasks ready, without asking. Record
that approval once, on the goal task (phase 2). Everything else in AGENTS.md still holds: anything
with an outside effect (creating accounts or a remote repository, API keys, payment, deployment, sending
anything) waits for the approver with `alm wait`, and you go on with other ready tasks. There is
no spend cap: the quota is the limit (`alm handoff-rules`: near 3% left, write your handoff and
stop; the same prompt continues later).

**Already started?** First run `alm list`. If a task whose title starts with `Goal:` is there, the
goal is already planned: do not plan it again. Read its latest handoff (`alm show <id>`) and the
project notes, then continue from `alm next` (phase 3), or with phase 4 when every other task of
the goal is finished. If the words given now ask for something the goal task's spec does not hold,
add it as tasks in a new iteration and add it to the spec.

## 1. Clarify, only if needed

If the description leaves something essential open (what the app is for, who uses it, where it
must run), ask the person once, right now, all your questions together in one message, and wait
for the answer. If the request that started you named no run mode, ask in the same message: show
what `alm guide run-modes` prints. Ask nothing else, then or later: every other choice is yours (a
name, a stack, a layout, a detail with a sensible default). Make it and record it with
`alm ai-decided <id> \"<what and why>\"`, for the person to review later; it never blocks the run.

## 2. Plan

1. **The goal task.** `alm add area \"<the app's name>\"` for the app as a whole, then
   `alm add task --area <id> --title \"Goal: <the app, in one line>\" --branch <the branch you
   are on>`. Its description (`alm describe <id> --from <file.md>`) holds the spec: what the app
   does and for whom, and the acceptance criteria the finished app must meet, each one a thing a
   person can try. End it with this line: \"Goal run (goal prompt v3): the person approved this
   plan in advance by submitting the goal on <date>; the agent makes the areas, iterations and
   ready tasks. Outside effects still wait for the approver.\" The title's `Goal:` marks the goal
   task for every later session: keep it.
2. **Areas** from the app's parts (for example its screens, its data, its server):
   `alm add area \"<name>\"`.
3. **Iterations** as a roadmap of releases, each ending in a version the person can try:
   `alm add iteration \"<name>\" --release v0.1`, then v0.2 and so on; dates empty unless known.
   v0.1 is the smallest app that works end to end; each later release adds to it.
4. **Tasks** in them: `alm add task --area <id> --title \"<outcome>\" --branch <the branch you
   are on>` (as `alm guide tasks` says: one observable outcome each, about one session and one
   commit), then `alm plan <id> <iteration>`. The last task of each iteration, in the app's area,
   is that release's acceptance check: try the release against its part of the spec, and write
   the steps for the person to try it.
5. **Ready, then order.** `alm ready <id>` for every task, the goal task too. Then
   `alm block <id> --by <task-id>` for each task that needs another first: each acceptance check
   by the other tasks of its iteration, the first tasks of an iteration by the previous
   iteration's check, and the goal task by the last check.

## 3. Build

Iteration by iteration, one task at a time: `alm next`, then `alm claim <id> --as <your session
name> --branch <the branch you are on>` (save and share the claim; it prints the Git commands as the example), do the work and save it
it, `alm label <id> \"<label>\"` as each step is proven, and finish it the same way in either
run mode: once the work is finished and only the person's check by hand is left,
`alm done <id> --without-human-check \"Goal run\" --steps <file.md>`, the steps telling the
person how to try it (at the top label, once the person has checked it: `alm done <id>`).

**Checking a release.** Prove each check with automated tests and command-line checks first: the
project's test suite, `curl` against the running server, a headless test runner. Use the browser
(clicking, or running scripts in the page) only when nothing else can prove the result, and never
run page scripts on a local network address such as `192.168.x.x`: the browser asks the person to
approve every script there, and the run would stall. When an acceptance check still needs the
browser, write that in the task's steps for the person, finish the task as above, and go on.

When `alm next` says all of a task's blockers are done, check it, then `alm unblock <id>`. A task
with an outside effect: `alm wait <id> --on \"<what the approver must do>\"`, then go on with the
next ready task. A task that needs another one first: add that task if it is missing, then
`alm block`. A task given up: `alm drop <id>`, with the reason in its note (`alm note`). Write your
handoff (`alm handoff`) after every task, and follow `alm handoff-rules`.

## 4. Finish

When the last iteration's acceptance check is done: `alm unblock` the goal task, check the whole
app against the spec's acceptance criteria, and finish the goal task the same way, done and to
check by the person:
`alm done <id> --without-human-check \"Goal run: try the app\" --steps <file.md>`, the steps
telling the person how to try the whole app. If the goal turns out to be impossible, the
goal task waits on the person instead, with the reason. Then print `alm list --json` and stop.

## Keep going until it is finished

Where your tool has `/goal` (Claude Code, Codex), the goal is set with this condition, which can be
judged from the conversation alone:

    The latest `alm list --json` printed in this conversation shows every task of the goal done,
    dropped or waiting on the person (a task blocked only by waiting tasks counts as waiting).

The request that started you may have set it already; otherwise set it yourself where your tool
lets you. Without `/goal`, the same holds: keep going, task after task, until that condition is
true, and print `alm list --json` to show it. Stop earlier only at the quota gate, with your
handoff written; given this prompt again, you continue from `alm next`.
";

const GOAL_ZH_CN: &str = "# 目标运行：根据描述构建一个应用 (goal prompt v3)

给智能体：这是目标提示。有人描述了他们想要的应用（他们的原话随这段提示一起给出）。你把它规划成
领域、迭代和任务，自己把任务设为就绪，然后一个迭代接一个迭代地把它做出来，每一步都用 `alm`
记录，这样那个人，或者接手的另一个智能体，都能确切看到工作进行到了哪里。按顺序完成下面四个阶段。
任务用任务语言书写（`alm task-language`）；ID、命令和代码保持原样。

**目标运行模式。** 提交目标就是事先批准了计划（批准人，2026-10-16）：在这次运行中，你自己添加
领域和迭代，自己把任务设为就绪，不用询问。把这项批准在目标任务上记录一次（阶段 2）。AGENTS.md 里
的其他规则照旧：任何对外有影响的事（创建账号或远程仓库、API 密钥、付款、部署、发送任何东西）
都用 `alm wait` 等批准人处理，你接着做其他就绪的任务。没有花费上限：额度就是限制
（`alm handoff-rules`：剩余接近 3% 时，写好交接并停下；之后用同一段提示继续）。

**已经开始过了？** 先运行 `alm list`。如果其中有标题以 `Goal:` 开头的任务，说明目标已经规划过：
不要重新规划。读它最新的交接（`alm show <id>`）和项目笔记，然后从 `alm next` 继续（阶段 3）；
如果目标的其他任务都已结束，就进入阶段 4。如果这次给出的话要求了目标任务的规格里没有的东西，
把它作为新迭代里的任务添加进来，并补进规格。

## 1. 澄清，仅在需要时

如果描述里有关键的东西没说清（应用是做什么的、谁来用、必须在哪里运行），现在就问那个人一次：把所有
问题放在一条消息里一起问，然后等回答。如果启动你的请求没有指定运行模式，就在同一条消息里问：给对方
看 `alm guide run-modes` 输出的内容。除此之外，现在和以后都不再问：其他每个选择都由你来做（名称、
技术栈、布局、有合理默认值的细节）。做出选择，并用 `alm ai-decided <id> \"<what and why>\"` 记录，
供那个人之后审阅；它从不阻挡运行。

## 2. 规划

1. **目标任务。** 先用 `alm add area \"<the app's name>\"` 为整个应用添加一个领域，再运行
   `alm add task --area <id> --title \"Goal: <the app, in one line>\" --branch <the branch you
   are on>`。它的描述（`alm describe <id> --from <file.md>`）写规格：应用做什么、为谁做，以及完成的
   应用必须满足的验收标准，每一条都是一个人能亲手试的事。最后加上这一行：
   “Goal run (goal prompt v3)：这个人在 <date> 提交目标，就是事先批准了这个计划；领域、迭代和
   就绪的任务由智能体来建。对外有影响的事仍然等批准人处理。”标题里的 `Goal:` 让之后的每个会话都能认出目标任务：保留它。
2. **领域**，按应用的组成部分划分（例如它的界面、数据、服务器）：`alm add area \"<name>\"`。
3. **迭代**，作为一份发布路线图，每个迭代结束时都有一个对方能试用的版本：
   `alm add iteration \"<name>\" --release v0.1`，然后 v0.2，依此类推；日期不知道就留空。v0.1 是
   能从头到尾跑通的最小应用；之后的每次发布在它上面增加功能。
4. **任务**，放进这些迭代：`alm add task --area <id> --title \"<outcome>\" --branch <the branch
   you are on>`（按 `alm guide tasks` 所说：每个任务一个可观察的结果，大约一个会话、一次提交），
   然后 `alm plan <id> <iteration>`。每个迭代的最后一个任务放在应用的领域里，是这次发布的验收检查：
   对照规格中属于它的部分试用这个版本，并写下让那个人试用的步骤。
5. **先就绪，再排顺序。** 对每个任务运行 `alm ready <id>`，目标任务也一样。然后对每个需要先等别的
   任务的任务运行 `alm block <id> --by <task-id>`：每个验收检查被它所在迭代的其他任务阻塞，每个迭代
   最先做的任务被上一个迭代的检查阻塞，目标任务被最后一个检查阻塞。

## 3. 构建

一个迭代接一个迭代，一次一个任务：`alm next`，然后 `alm claim <id> --as <your session name>
--branch <the branch you are on>`（保存并共享认领；它以 Git 命令为例），做完工作并保存（提交），每证明一步就
`alm label <id> \"<label>\"`，两种运行模式下都用同一种方式完成它：工作做完、只差那个人亲手检查时，
`alm done <id> --without-human-check \"Goal run\" --steps <file.md>`，步骤告诉那个人怎么试用
（达到最高标签、那个人检查过之后：`alm done <id>`）。

**检查一个版本。** 每项检查先用自动测试和命令行来证明：项目的测试套件、对运行中的服务器用 `curl`、
无界面的测试运行器。只有别的办法都证明不了时才用浏览器（点击，或在页面里运行脚本），并且不要在
`192.168.x.x` 这样的局域网地址上运行页面脚本：浏览器在那里每运行一段脚本都要那个人批准，运行会卡住。
如果某个验收检查仍然需要浏览器，就把这一点写进这个任务给那个人的步骤里，照上面的方式完成任务，接着往下做。

当 `alm next` 说某个任务的阻塞任务都已完成时，检查一下，然后 `alm unblock <id>`。对外有影响的任务：
`alm wait <id> --on \"<what the approver must do>\"`，然后接着做下一个就绪的任务。需要先做另一个任务
的任务：如果那个任务还没有，就添加它，然后 `alm block`。放弃的任务：`alm drop <id>`，把原因写进它的
备注（`alm note`）。每完成一个任务就写交接（`alm handoff`），并遵守 `alm handoff-rules`。

## 4. 收尾

最后一个迭代的验收检查完成后：对目标任务运行 `alm unblock`，对照规格的验收标准检查整个应用，然后用同一种
方式完成目标任务，标为已完成、待那个人检查：
`alm done <id> --without-human-check \"Goal run: try the app\" --steps <file.md>`，步骤告诉那个人怎么试用整个应用。如果发现目标做不到，目标
任务改为等待那个人，并写明原因。然后打印 `alm list --json`，停下。

## 一直做到完成

如果你的工具有 `/goal`（Claude Code、Codex），就用下面这个条件设定目标，它只看对话本身就能判断：

    The latest `alm list --json` printed in this conversation shows every task of the goal done,
    dropped or waiting on the person (a task blocked only by waiting tasks counts as waiting).

（意思是：这次对话里最新打印的 `alm list --json` 显示，目标的每个任务都已完成、已放弃或在等待那个人；
只被等待中的任务阻塞的任务算作等待。）启动你的请求可能已经设定了它；否则在你的工具允许时自己设定。
没有 `/goal` 时也一样：一个任务接一个任务地做下去，直到这个条件成立，并打印 `alm list --json` 来表明。
只有到了额度关口才提前停下，并写好交接；再次收到这段提示时，从 `alm next` 继续。
";

const GOAL_JA: &str = "# ゴール実行：説明からアプリを作る (goal prompt v3)

エージェントへ：これはゴールのプロンプトです。ある人が、欲しいアプリを説明しました（その人の言葉は
このプロンプトと一緒に渡されます）。あなたはそれをエリア、イテレーション、タスクに計画し、タスクを
自分で準備完了にし、イテレーションごとに作っていきます。一歩ごとに `alm` で記録するので、その人も、
引き継ぐ別のエージェントも、作業がどこまで進んだかを正確に見られます。下の四つの段階を順に進めます。
タスクはタスクの言語で書きます（`alm task-language`）。ID、コマンド、コードはそのままです。

**ゴール実行モード。** ゴールを出したことで、計画は前もって承認されています（承認者、2026-10-16）。
この実行では、エリアとイテレーションを自分で加え、自分のタスクを聞かずに準備完了にします。その承認は
ゴールのタスクに一度だけ記録します（段階 2）。AGENTS.md のほかのルールはそのままです。外部への影響が
あること（アカウントやリモートリポジトリの作成、API キー、支払い、デプロイ、何かを送ること）は
`alm wait` で承認者を待ち、ほかの準備完了のタスクを進めます。費用の上限はありません。上限は利用枠です
（`alm handoff-rules`：残りが 3% 近くになったらハンドオフを書いて止まり、同じプロンプトであとで続けます）。

**もう始まっている？** まず `alm list` を実行します。タイトルが `Goal:` で始まるタスクがあれば、
ゴールはもう計画済みです。計画し直しません。その最新のハンドオフ（`alm show <id>`）とプロジェクト
ノートを読み、`alm next` から続けます（段階 3）。ゴールのほかのタスクがすべて終わっていれば段階 4 に
進みます。今回の言葉が、ゴールのタスクの仕様にないものを求めているなら、新しいイテレーションの
タスクとして加え、仕様にも加えます。

## 1. 確かめる（必要なときだけ）

説明に、欠かせないことが抜けているなら（アプリが何のためか、誰が使うか、どこで動く必要があるか）、
今すぐその人に一度だけ、すべての質問を一つのメッセージにまとめて聞き、答えを待ちます。あなたを
始めた依頼に実行モードがなければ、同じメッセージで聞きます。`alm guide run-modes` の出力を見せます。
それ以外は、今もあとも聞きません。ほかの選択はすべてあなたのものです（名前、技術の組み合わせ、配置、
自然な既定がある細かな点）。決めて、`alm ai-decided <id> \"<what and why>\"` で記録し、あとでその人が
見直せるようにします。実行を止めることはありません。

## 2. 計画する

1. **ゴールのタスク。** アプリ全体のために `alm add area \"<the app's name>\"`、続けて
   `alm add task --area <id> --title \"Goal: <the app, in one line>\" --branch <the branch you
   are on>`。その説明（`alm describe <id> --from <file.md>`）に仕様を書きます。アプリが何をし、
   誰のためのものか、そして完成したアプリが満たすべき受け入れ基準。基準はどれも、人が試せることに
   します。最後にこの一行を書きます：「Goal run (goal prompt v3)：この人は <date> にゴールを出して、
   この計画を前もって承認しました。エリア、イテレーション、準備完了のタスクはエージェントが作ります。
   外部への影響があることは、引き続き承認者を待ちます。」タイトルの `Goal:` は、あとのすべての
   セッションにゴールのタスクを示す印です。消さずに残します。
2. **エリア**はアプリの部分から（たとえば画面、データ、サーバー）：`alm add area \"<name>\"`。
3. **イテレーション**はリリースの道筋として作り、どれも最後にその人が試せる版で終わります：
   `alm add iteration \"<name>\" --release v0.1`、続けて v0.2 と順に。日付は分からなければ空のまま。
   v0.1 は最初から最後まで動く一番小さなアプリで、あとのリリースはそこに足していきます。
4. **タスク**をそこに：`alm add task --area <id> --title \"<outcome>\" --branch <the branch you
   are on>`（`alm guide tasks` のとおり、一つのタスクに一つの見て分かる結果、だいたい一つの
   セッションと一つのコミット）、続けて `alm plan <id> <iteration>`。各イテレーションの最後の
   タスクは、アプリのエリアに置く、そのリリースの受け入れ確認です。仕様のうちその部分に照らして
   リリースを試し、その人が試すための手順を書きます。
5. **準備完了にしてから、順番を付けます。** すべてのタスクに `alm ready <id>`、ゴールのタスクにも。
   それから、先にほかのタスクが要るタスクごとに `alm block <id> --by <task-id>`。各受け入れ確認は
   同じイテレーションのほかのタスクで、各イテレーションの最初のタスクは前のイテレーションの確認で、
   ゴールのタスクは最後の確認でブロックします。

## 3. 作る

イテレーションごとに、一度に一つのタスク：`alm next`、続けて `alm claim <id> --as <your session
name> --branch <the branch you are on>`（担当を保存して共有します。Git のコマンドが例として表示されます）、作業をして保存（コミット）
し、一歩ずつ確かめるたびに `alm label <id> \"<label>\"`、そしてどちらの実行モードでも同じように
終えます：作業が終わり、その人が手で試す確認だけが残ったら
`alm done <id> --without-human-check \"Goal run\" --steps <file.md>`。手順にはその人が試すしかたを
書きます（一番上のラベルで、その人が確認したあとなら `alm done <id>`）。

**リリースを確かめる。** どの確認も、まず自動テストとコマンドラインで証明します：プロジェクトの
テスト、動いているサーバーへの `curl`、画面を出さないテストランナー。ブラウザ（クリックや、ページでの
スクリプトの実行）は、ほかに証明する方法がないときだけ使います。`192.168.x.x` のようなローカル
ネットワークのアドレスでは、ページのスクリプトを実行しません。そこではスクリプトごとにブラウザがその人の
承認を求め、実行が止まってしまうからです。それでもブラウザが要る受け入れ確認は、そのタスクのその人向けの
手順にそう書き、上のとおりタスクを終えて先へ進みます。

`alm next` があるタスクのブロック元がすべて完了したと言ったら、確かめてから `alm unblock <id>`。
外部への影響があるタスク：`alm wait <id> --on \"<what the approver must do>\"`、そして次の準備完了の
タスクへ。先に別のタスクが要るタスク：そのタスクがなければ加え、`alm block`。やめたタスク：
`alm drop <id>`、理由はメモに（`alm note`）。タスクを一つ終えるたびにハンドオフを書き
（`alm handoff`）、`alm handoff-rules` に従います。

## 4. 仕上げる

最後のイテレーションの受け入れ確認が終わったら：ゴールのタスクを `alm unblock` し、仕様の受け入れ
基準に照らしてアプリ全体を確かめ、ゴールのタスクも同じように、完了・その人の確認待ちにして終えます：
`alm done <id> --without-human-check \"Goal run: try the app\" --steps <file.md>`。手順には
アプリ全体を試すしかたを書きます。ゴールが無理だと分かったら、代わりにゴールのタスクを
理由を付けてその人の待ちにします。それから `alm list --json` を表示して止まります。

## 終わるまで続ける

あなたのツールに `/goal` があれば（Claude Code、Codex）、ゴールは次の条件で設定します。会話だけを
見て判断できる条件です：

    The latest `alm list --json` printed in this conversation shows every task of the goal done,
    dropped or waiting on the person (a task blocked only by waiting tasks counts as waiting).

（意味：この会話で最後に表示した `alm list --json` で、ゴールのタスクがすべて完了、中止、または
その人の待ちになっている。待機中のタスクだけにブロックされているタスクは待ちに数える。）あなたを
始めた依頼がもう設定しているかもしれません。そうでなければ、ツールが許すなら自分で設定します。
`/goal` がなくても同じです。この条件が成り立つまでタスクを次々に進め、`alm list --json` を表示して
示します。それより早く止まるのは利用枠の関門だけで、そのときはハンドオフを書いてからです。この
プロンプトをもう一度渡されたら、`alm next` から続けます。
";

/// `alm guide goal` in `language`, or `None` when it is not written in that language (the agent
/// then follows a faithful translation of the English text). Allomos types it into the agent for
/// the person, who may read along, so it is written in every language of [`RUN_MODES_LANGUAGES`].
pub fn goal(language: &str) -> Option<&'static str> {
    match written_in(language)? {
        "ja" => Some(GOAL_JA),
        "zh-CN" => Some(GOAL_ZH_CN),
        _ => Some(GOAL_EN),
    }
}

/// The part of the goal request that follows the connect request when a person starts a new app
/// from the start page's **Build a new app…** (Task-A03-25-02): the person's description, the run
/// mode they chose, then the goal prompt ([`goal`]) in `language` (English when it is not written
/// in that language). Allomos types it in; the person sends it.
pub fn goal_request_in(language: &str, description: &str, quiet: bool) -> String {
    let words = description
        .trim()
        .lines()
        .map(|line| format!("> {line}").trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let prompt = goal(language).unwrap_or(GOAL_EN);
    let (heading, intro, mode, said) = match written_in(language) {
        Some("zh-CN") => (
            "然后：构建应用（第一、二部分完成后）",
            "这个人在 Allomos 的“构建新应用…”中开始了这项工作，并在下面描述了想要的应用。请按照后面的目标提示，把它做出来。",
            if quiet {
                "运行模式：安静模式（Run mode: quiet），这个人在 Allomos 中开始这个目标时选择的。"
            } else {
                "运行模式：普通模式（Run mode: normal），这个人在 Allomos 中开始这个目标时选择的。"
            },
            "这个人的描述：",
        ),
        Some("ja") => (
            "そのあと：アプリを作る（第一部と第二部が終わってから）",
            "この人は Allomos の「新しいアプリを作る…」からこの作業を始め、欲しいアプリを下に書きました。続くゴールのプロンプトに従って、それを作ってください。",
            if quiet {
                "実行モード：静かモード（Run mode: quiet）。この人が Allomos でこのゴールを始めるときに選びました。"
            } else {
                "実行モード：通常モード（Run mode: normal）。この人が Allomos でこのゴールを始めるときに選びました。"
            },
            "この人の説明：",
        ),
        _ => (
            "Then: build the app (after parts 1 and 2)",
            "The person started this from Allomos's Build a new app… and described the app they \
want below. Build it by following the goal prompt that comes next.",
            if quiet {
                "Run mode: quiet (the person chose it in Allomos when starting this goal)."
            } else {
                "Run mode: normal (the person chose it in Allomos when starting this goal)."
            },
            "The person's description:",
        ),
    };
    format!("## {heading}\n\n{intro}\n\n{mode}\n\n{said}\n\n{words}\n\n{prompt}")
}

/// What `alm add task` prints after its Git commands (Task-A03-33), in the task language
/// (`en`, `zh-CN`, `ja`; English otherwise): check the iteration it went into (`planned_in`, the
/// active one or the one asked for, Task-A01-root343), or plan it in one from `iterations` (one
/// line each, as `alm list` heads them) when it went into none, then make it ready unless
/// something in it is the person's to decide. alm tells; the agent decides.
pub fn added_task_steps(
    language: &str,
    id: &str,
    iterations: &[String],
    planned_in: Option<&str>,
) -> String {
    let listed: String = iterations
        .iter()
        .flat_map(|i| ["     ", i.as_str(), "\n"])
        .collect();
    match language {
        "zh-CN" => {
            let list = if iterations.is_empty() {
                "   还没有迭代：先用 `alm add iteration` 添加一个。\n".to_string()
            } else {
                listed
            };
            let step1 = match planned_in {
                Some(i) => {
                    format!("1. 它已放进迭代 {i}。如果它属于别的迭代：`alm plan {id} <迭代>`。\n")
                }
                None => format!(
                    "1. 没有进行中的迭代（也没有计划中的迭代），所以它不在任何迭代里。把它计划进合适的迭代：`alm plan {id} <迭代>`（有进行中的迭代时，新任务会自动放进去：标为进行中的那个，没有就是第一个计划中的迭代）：\n{list}"
                ),
            };
            format!(
                "接下来（给添加它的智能体）：
{step1}2. 检查任务里有没有该由人决定的事：用户会看到并习惯的东西、难以撤销的事、改变计划或其他任务的事、有花费或对外影响的事。没有：把它设为就绪，`alm ready {id}`。有：留在储备，在描述的备注里写明每个待定问题，并告诉那个人。
"
            )
        }
        "ja" => {
            let list = if iterations.is_empty() {
                "   イテレーションがまだありません：先に `alm add iteration` で追加します。\n"
                    .to_string()
            } else {
                listed
            };
            let step1 = match planned_in {
                Some(i) => format!(
                    "1. イテレーション {i} に入れました。別のイテレーションのものなら：`alm plan {id} <イテレーション>`。\n"
                ),
                None => format!(
                    "1. 進行中のイテレーションがない（計画中のイテレーションもない）ので、どのイテレーションにも入っていません。合うものに計画します：`alm plan {id} <イテレーション>`（進行中のイテレーションがあれば、新しいタスクは自動でそこに入ります：進行中にしたもの、なければ最初の計画中のもの）：\n{list}"
                ),
            };
            format!(
                "次に（追加したエージェントへ）：
{step1}2. 人が決めるべきことがタスクにないか確かめます：ユーザーが目にして慣れていくもの、元に戻しにくいこと、計画やほかのタスクを変えること、費用や外への影響があること。なければ準備完了にします：`alm ready {id}`。あれば backlog のまま残し、説明の備考に未決の問いを一つずつ書いて、その人に伝えます。
"
            )
        }
        _ => {
            let list = if iterations.is_empty() {
                "   There is no iteration yet: add one first with `alm add iteration`.\n"
                    .to_string()
            } else {
                listed
            };
            let step1 = match planned_in {
                Some(i) => format!(
                    "1. It went into iteration {i}. If it belongs in another: `alm plan {id} <iteration>`.\n"
                ),
                None => format!(
                    "1. No iteration is active (none is planned either), so it is in none. Plan it in the one that fits: `alm plan {id} <iteration>` (while one is active, new tasks go into it: the one marked active, or else the first planned one):\n{list}"
                ),
            };
            format!(
                "Next, for the agent who added it:
{step1}2. Check the task for anything that is the person's to decide: what users will see and get used to, anything hard to undo, anything that changes the plan or another task, anything with a cost or an outside effect. Nothing: make it ready, `alm ready {id}`. Something: leave it in backlog, name each open question in the description's Notes, and tell the person.
"
            )
        }
    }
}

/// What `alm claim` prints after its Git commands (Task-A03-tide388, approver 2026-10-22), in the
/// task language (`en`, `zh-CN`, `ja`; English otherwise): before planning the task, look at the
/// size of the source files the plan will change, and put a refactoring of only the part to be
/// changed into the plan when one is too big. alm reminds; it measures nothing and stops nothing.
pub fn claimed_task_steps(language: &str) -> String {
    match language {
        "zh-CN" => "做计划之前（alm 提醒，你来检查）：看一下计划要改的源文件有多大。超过约 1000 行（测试文件不算；项目说明 `project-notes.md` 可以定别的行数）就看一看：要改的那部分是否和不相干的东西缠在一起。是的话，把重构写进这次的计划：只把本任务要改的那部分拆出来（例如移到自己的模块），不重构整个文件，文件随一个个任务逐步变小。默认在本任务里做：重构单独一个提交，放在功能提交之前，提交后测试通过、行为不变。重构比功能本身还大，或要动本任务范围外的文件时，先另开一个任务做重构，再用 `alm block <本任务> --by <新任务>` 连上。
".to_string(),
        "ja" => "計画を立てる前に（alm は知らせるだけ、確かめるのはあなた）：計画で変える元のファイルの大きさを見ます。約 1000 行を超えるもの（テストのファイルは数えません。`project-notes.md` で別の行数を決めてもかまいません）は一度見て、変える部分が関係のないものと絡み合っていないか判断します。絡み合っていれば、リファクタリングを今回の計画に入れます。このタスクで変える部分だけを切り出し（たとえば専用のモジュールへ移す）、ファイル全体は書き直しません。ファイルはタスクごとに少しずつ小さくなります。ふつうはこのタスクの中で行います：リファクタリングを機能のコミットより前の独立したコミットにし、その後もテストが通り、動きが変わらないこと。リファクタリングが機能そのものより大きいとき、またはこのタスクの範囲外のファイルに及ぶときは、先にリファクタリングのタスクを別に追加し、`alm block <このタスク> --by <新しいタスク>` でつなぎます。
".to_string(),
        _ => "Before you plan this task (alm reminds; you check): look at the size of the source files your plan will change. One over about 1000 lines (tests do not count; the project notes, `project-notes.md`, may name another limit) needs a look: is the part you will change tangled with unrelated things? If so, put a refactoring into this plan: only the part this task changes (for example, move it into a module of its own), never the whole file, so files get smaller task by task. By default it is done in this task, as a commit of its own before the feature commit, with the tests passing and the behaviour unchanged after it. When the refactoring is bigger than the feature itself, or reaches files outside this task, add a task for it first and link them with `alm block <this task> --by <new task>`.
".to_string(),
    }
}

/// `alm guide github-release` (Task-A03-planet530): the procedure an agent follows when the
/// approver writes `alm guide github release` in the conversation: publish a new GitHub release
/// with release notes, check it, and (only with the word `install`) install it on this computer
/// (Task-A03-hen326). The same text for every project:
/// it names no repository, address or file; the project's facts come from the project notes'
/// `## Releases` section.
const GITHUB_RELEASE_EN: &str = "# GitHub release: publish a new release, and install it on this computer when asked

The approver writes `alm guide github release` (or `alm guide github-release`) in the
conversation; you ran it, and this is what to do. One outcome: **a new release on GitHub, with
notes that say what changed** (and, when the command says `install`, installed on this computer).
The command in the conversation is the approval for this release, and with `install` for
installing it here too. Anything it does not cover (another version jump than its words say,
publishing somewhere new, installing without `install`) still goes to the approver.

Words after the command, which you read from the approver's line (alm does not act on them):
- `minor` or `major`: raise that part of the version instead of the patch.
- `install`: after step 6, also download and install the release on this computer (step 7).
  Without it, skip step 7: the release is published and checked, nothing is downloaded or
  installed here.
The words combine, for example `alm guide github release minor install`.
Any other word: ask the approver what it means before you tag.

This text names no repository, address or file: every fact of the project comes from the
taskbag's `project-notes.md`, its `## Releases` section (step 0). Where the project's rules
(AGENTS.md) say something else, they win.

## 0. Read the project's release facts

Read the `## Releases` section of the project notes. It is a table with these keys:

| Key | What it says |
|---|---|
| `REMOTE` | the GitHub repository |
| `BASE_BRANCH` | the branch releases are cut from |
| `VERSION_FILES` | every file that holds the version, how it is written there, and what derives from it (a lockfile) |
| `TAG_FORMAT` | how a tag is spelled |
| `BUMP` | which part of the version goes up by default |
| `RELEASE_WORKFLOW` | what builds and publishes on a tag push, how long it takes, how to run it again for an existing tag |
| `NOTES_DIR` | where release-notes pages live, the file name pattern, and the shape of a page |
| `NOTES_LANGUAGES` | every language a page must exist in |
| `NOTES_INDEX` | the index pages that list releases and gain a line |
| `NOTES_URL` | the published address of a release's page, per language |
| `NOTES_PUBLISH` | what publishes the pages, how long it takes, and how to build them locally |
| `GH_NOTES` | who writes the GitHub release's own notes, and whether they link to the pages |
| `INSTALL_ASSET` | the asset to install on this computer, and its checksum file (used only with `install`) |
| `INSTALL_COMMAND` | how it is installed, and whether a person must approve (a password, a dialog) (only with `install`) |
| `VERIFY_INSTALLED` | how to read the installed version afterwards (only with `install`) |
| `APPROVAL_RECORD` | where this release's approval and evidence are written |

When the section is missing, or a key in it: derive what the repository settles (the last tag
and its shape, the workflow under `.github/workflows` that runs on a tag, the files whose version
equals the last tag's, the folder holding the last release's notes page, the assets of the last
GitHub release for this computer), write the section into the project notes, and show it to the
approver before tagging. Ask only for what nothing in the repository settles. Next time the
section is the source.

## 1. Preflight: stop on any failure and say which

- You are on `BASE_BRANCH`, the working tree is clean, pulled, and nothing is ahead of the remote.
- The last release (`gh release list --limit 1`) and the last tag agree.
- No run of `RELEASE_WORKFLOW` is still going (`gh run list --workflow <file> --limit 1`).
- There are commits since the last tag. None: say so and stop, there is nothing to release.
- `gh auth status` passes; with `install`, so does every tool `INSTALL_COMMAND` needs.

## 2. Collect what changed

- `git log <last tag>..HEAD --no-merges` lists the commits; a version bump is not a change.
- When a commit names a task, read it (`alm show <id>`), so the notes say what a person using the
  product notices, not what the code did.
- Sort the changes under **New**, **Fixed** and **Also** (or the headings the last release's page
  uses): one bullet per outcome, a bold lead-in, plain words, present tense; no commit hashes and
  no task IDs in the text. A change nobody using the product can notice (refactoring, tests, agent
  rules) goes under Also in one line, or is left out; say in your handoff what you left out.
- Write the notes in each of `NOTES_LANGUAGES`, translating meaning, not words; product and
  command names stay as they are.

## 3. Choose the version

`BUMP` (or the command's `minor` / `major`) applied to the last tag's version. Say the version you
chose before you write anything: until the tag exists, everything is easy to undo.

## 4. Write the notes

- One page per language in `NOTES_LANGUAGES`, in the same shape as the last release's page (front
  matter, headings, the download line, the date).
- A line for the new release at the top of every page in `NOTES_INDEX`.
- Build the pages locally (`NOTES_PUBLISH` says how), so a broken page is found before it is
  published.
- When `GH_NOTES` links to the pages, every language's page exists before the tag: a missing page
  is a dead link on the GitHub release.

## 5. Bump, commit, tag

- Change every file in `VERSION_FILES` to the new version; update what derives from them (a
  lockfile) with the project's own tool, never by hand.
- One commit holds the notes and the bump, in the project's commit style with the attribution line
  its rules ask for. Push `BASE_BRANCH` and confirm it landed.
- Tag in `TAG_FORMAT`, annotated (`git tag -a <tag> -m <tag>`), and push the tag. The tag push is
  the outside effect; everything before it is local.

## 6. Wait and check

- Wait for the release run and for the notes' publish in the background
  (`gh run watch <run id> --exit-status`); do not poll by hand.
- A failed run: read its log, fix the cause on `BASE_BRANCH`, and run it again the way
  `RELEASE_WORKFLOW` says (often `workflow_dispatch` with the tag). Never delete a tag or a release
  and tag again; say what happened instead.
- Check, and keep the evidence: `gh release view <tag>` lists every expected asset and checksum
  file; the release's own notes point to the pages (`GH_NOTES`); `NOTES_URL` for the new version
  answers 200 in every language, with the version in the page's title.

## 7. Download and install (only with `install`)

Without `install` in the command, skip this step and go on with step 8.

- Download into a new, empty folder outside the project:
  `gh release download <tag> --pattern <INSTALL_ASSET> --pattern <its checksum file>`.
- Check the checksum first; a mismatch stops the install, and you report it.
- Install with `INSTALL_COMMAND`. When it asks for a password or an approval in a dialog, open it
  for the approver and say what to click; never type a password.
- `VERIFY_INSTALLED`: the installed version equals the released one. Report what it shows.

## 8. Record and report

- Write the approval (the approver's command line, and when) and the evidence where
  `APPROVAL_RECORD` says, and in your handoff.
- Report in plain words first: the version, the notes' headings and how many items each has, the
  links to the release and to the notes, and the version installed (without `install`: that it
  was not installed here). Then the evidence: the run IDs, the asset list, and with `install` the
  checksum result. Last, what was not done or not checked.

## What you never do here

- Release from a working tree with changes, from another branch than `BASE_BRANCH`, or while a
  release run is going.
- Guess a value the repository does not settle: ask the approver.
- Force-push, delete a tag or a release, or tag again.
- Type a password or approve an installer's dialog.
";

/// `alm guide github-release` in `language`, or `None` when it is not written in that language
/// (the agent then follows a faithful translation of the English text). Only English so far.
pub fn github_release(language: &str) -> Option<&'static str> {
    match written_in(language)? {
        "en" => Some(GITHUB_RELEASE_EN),
        _ => None,
    }
}

/// The topic `words` name, joined by hyphens (`github release` is `github-release`), and the words
/// after it, which the guide defines and the agent reads (`alm guide github release minor`). The
/// longest run of leading words that names a topic wins; `None` when no leading words name one.
pub fn guide_topic(words: &[String]) -> Option<(&'static str, Vec<String>)> {
    (1..=words.len()).rev().find_map(|n| {
        let name = words[..n].join("-").to_lowercase();
        GUIDE_TOPICS
            .iter()
            .find(|t| **t == name)
            .map(|t| (*t, words[n..].to_vec()))
    })
}

/// A topic of [`LOCALIZED_TOPICS`] in `language`, or `None` when the topic is not one of them or
/// is not written in that language (the agent then translates the English text faithfully).
pub fn localized(topic: &str, language: &str) -> Option<&'static str> {
    match topic {
        "run-modes" => run_modes(language),
        "agent-files" => agent_files(language),
        "goal" => goal(language),
        "github-release" => github_release(language),
        _ => None,
    }
}

/// Which of [`RUN_MODES_LANGUAGES`] a task language is written in, if any.
fn written_in(language: &str) -> Option<&'static str> {
    let l = language.trim().to_lowercase().replace('_', "-");
    let is = |code: &str| l == code || l.starts_with(&format!("{code}-"));
    if is("en") {
        Some("en")
    } else if is("ja") {
        Some("ja")
    } else if l == "zh" || l == "zh-cn" || l == "zh-sg" || l.starts_with("zh-hans") {
        Some("zh-CN")
    } else {
        None
    }
}

/// One topic of the guide, or all of them when `topic` is `None`. `None` for an unknown topic.
pub fn guide(topic: Option<&str>) -> Option<String> {
    let text = |t: &str| match t {
        "terms" => Some(TERMS),
        "customize" => Some(CUSTOMIZE),
        "tasks" => Some(TASKS),
        "commands" => Some(COMMANDS),
        "workflow" => Some(WORKFLOW),
        "run-modes" => Some(RUN_MODES_EN),
        "agent-files" => Some(AGENT_FILES_EN),
        "goal" => Some(GOAL_EN),
        "github-release" => Some(GITHUB_RELEASE_EN),
        "move-cards" => Some(MOVE_CARDS),
        _ => None,
    };
    match topic {
        Some(t) => text(t).map(String::from),
        None => Some(
            GUIDE_TOPICS
                .iter()
                .filter_map(|t| text(t))
                .collect::<Vec<_>>()
                .join("\n"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_topic_is_there_and_the_brief_names_them() {
        for topic in GUIDE_TOPICS {
            assert!(
                guide(Some(topic)).is_some_and(|t| t.starts_with("# ")),
                "{topic}"
            );
            assert!(
                crate::agent_section().contains(&format!("alm guide {topic}")),
                "{topic}"
            );
        }
        assert_eq!(guide(Some("nope")), None);
        let all = guide(None).unwrap();
        assert!(all.contains("# Allomos terms") && all.contains("# How to work the tasks"));
        // The fixed parts are said plainly.
        assert!(
            guide(Some("customize"))
                .unwrap()
                .contains("The seven states")
        );
        for text in [guide(Some("commands")).unwrap(), crate::agent_section()] {
            assert!(text.contains("alm block <id> --by"), "{text}");
            assert!(text.contains("alm unblock <id>"), "{text}");
            assert!(text.contains("--who"), "{text}");
        }
        assert!(
            guide(Some("tasks"))
                .unwrap()
                .contains("Nothing: make it\n   ready (`alm ready <id>`)")
        );
        assert!(guide(Some("tasks")).unwrap().contains("alm task-language"));
        assert!(crate::agent_section().contains("run `alm task-language`"));
        // The rule for finishing, with its automatic/loop exception (Task-A03-15).
        for text in [guide(Some("workflow")).unwrap(), crate::agent_section()] {
            assert!(text.contains("top label"), "{text}");
            assert!(text.contains("automatic or loop run"), "{text}");
            assert!(text.contains("--without-human-check"), "{text}");
            assert!(text.contains("--steps <file.md>"), "{text}");
            assert!(text.contains("alm checked <id>"), "{text}");
            assert!(text.contains("alm human"), "{text}");
            assert!(!text.contains("owner"), "{text}");
            assert!(text.contains("handoff"), "{text}");
        }
        assert!(crate::agent_section().contains("every ready task nobody holds"));
        // In a quiet run the agent never waits: decide, or park the question (Task-A00-29).
        let flat = |t: String| t.split_whitespace().collect::<Vec<_>>().join(" ");
        for text in [guide(Some("workflow")).unwrap(), crate::agent_section()] {
            let text = flat(text);
            assert!(text.contains("never stop to wait for the person"), "{text}");
            assert!(
                text.contains("alm wait <id> --on \"<the question>\""),
                "{text}"
            );
            assert!(text.contains("go on with the next ready task"), "{text}");
            assert!(text.contains("it would wait until it times out"), "{text}");
            assert!(text.contains("changes the plan or another task"), "{text}");
        }
        let tasks = flat(guide(Some("tasks")).unwrap());
        assert!(
            tasks.contains("A routine choice (a name, a file layout"),
            "{tasks}"
        );
        assert!(tasks.contains("alm ai-decided"), "{tasks}");
        // Routine choices are recorded for review (Task-A03-19).
        for text in [guide(None).unwrap(), crate::agent_section()] {
            assert!(text.contains("alm ai-decided <id>"), "{text}");
            assert!(text.contains("alm reviewed <id>"), "{text}");
            assert!(text.contains("To review: AI decisions"), "{text}");
        }
        // Before an automatic or loop run, the person chooses quiet or normal mode, from the
        // words `alm guide run-modes` gives, and the choice goes in the handoff (Task-A03-21).
        for text in [guide(Some("workflow")).unwrap(), crate::agent_section()] {
            let text = flat(text);
            assert!(text.contains("alm guide run-modes"), "{text}");
            assert!(text.contains("quiet or normal mode"), "{text}");
            assert!(text.contains("do not start until they choose"), "{text}");
            assert!(text.contains("handoff"), "{text}");
            assert!(text.contains("Run mode: quiet"), "{text}");
        }
        // Task-A03-22: the agent tells alm the branch it is on; every task has a base branch by
        // its real name, which the claim names with the Git commands. Guide and brief agree.
        for text in [guide(Some("workflow")).unwrap(), crate::agent_section()] {
            let text = flat(text);
            assert!(
                text.contains(
                    "alm claim <id> --as <your session name> --branch <the branch you are on>"
                ),
                "{text}"
            );
            assert!(text.contains("base branch, by its real name"), "{text}");
            assert!(text.contains("merges back into it"), "{text}");
            assert!(text.contains("working branch"), "{text}");
        }
        // Task-A03-23: the one taking over checks the real state and repeats nothing done.
        let workflow = flat(guide(Some("workflow")).unwrap());
        for words in [
            "Taking over from a handoff",
            "check the real state before trusting it",
            "git worktree list",
            "is not yours to change or remove; if unsure, ask the person",
            "What the task's **Evidence** lists as already done is not done again",
            "`alm handoff <id> --as <your session name> --text",
            "keeps the approvals and what was done; it does not authorize repeating it",
        ] {
            assert!(workflow.contains(words), "{words}");
        }
        let flat_all = flat(guide(None).unwrap());
        for words in [
            "A task without a base gets the branch you are on",
            "merging one base branch into another is a task of its own",
            "--branch <the branch you are on> [--base <name>]",
            "alm base <id> <branch>",
            "--working <name>",
        ] {
            assert!(flat_all.contains(words), "{words}");
        }
        assert!(
            flat(crate::agent_section()).contains("`--base` overrides it"),
            "brief"
        );
        // Decision 0017: no `--commit` / `--push`; get the latest changes before claiming, share
        // the claim at once in the version control tool, with the Git commands as the example
        // (Task-A01-60).
        for text in [guide(None).unwrap(), crate::agent_section()] {
            assert!(!text.contains("--push"), "{text}");
            assert!(!text.contains("--commit"), "{text}");
            let text = flat(text);
            assert!(text.contains("latest changes"), "{text}");
            assert!(text.contains("until your claim is shared"), "{text}");
            assert!(text.contains("version control tool"), "{text}");
            assert!(text.contains("Git commands as the example"), "{text}");
        }
    }

    /// `alm guide agent-files` (Task-A03-24): the one-time procedure, written in every language
    /// the app speaks, general (examples, not a closed list), with the same steps and commands;
    /// the brief carries the three standing rules and names the topic.
    #[test]
    fn agent_files_is_written_in_every_language_and_the_brief_carries_its_rules() {
        let flat = |t: &str| t.split_whitespace().collect::<Vec<_>>().join(" ");
        for language in RUN_MODES_LANGUAGES {
            let text = agent_files(language).unwrap_or_else(|| panic!("{language}"));
            assert!(text.starts_with("# "), "{language}");
            for words in [
                "AGENTS.md",
                "CLAUDE.md",
                "GEMINI.md",
                "`.cursor/rules/`",
                "`.github/copilot-instructions.md`",
                "`.windsurfrules`",
                "`@AGENTS.md`",
                "Read and follow AGENTS.md.",
                "`alm brief`",
                "`alm wait <id> --on \"<the question>\"`",
                "`project-notes.md`",
                "`## Agents set up`",
                "-- project-notes.md`",
                "-- <those files>`",
                "`alm list --state in_progress`",
                "`git worktree list`",
            ] {
                assert!(text.contains(words), "{language}: {words}");
            }
            // Five numbered steps, then two standing rules.
            for step in 1..=5 {
                assert!(
                    text.contains(&format!("\n{step}. **")),
                    "{language}: {step}"
                );
            }
            assert!(!text.contains("\n6. "), "{language}");
            assert_eq!(text.matches("\n- ").count(), 2, "{language}");
            assert_eq!(text.matches("\n   - ").count(), 2, "{language}");
            assert_eq!(localized("agent-files", language), Some(text));
        }
        let en = flat(agent_files("en").unwrap());
        for words in [
            "These are examples, not a closed list",
            "Contradictions are not yours to settle",
            "do not choose between them",
            "list each pair for the person",
            "save only those files in your version control tool",
            "Shared rules live only in AGENTS.md; never add a shared rule to one agent's own file",
        ] {
            assert!(en.contains(words), "{words}");
        }
        assert_ne!(agent_files("ja"), agent_files("en"));
        assert_ne!(agent_files("zh-CN"), agent_files("en"));
        assert_eq!(agent_files("zh-Hans-CN"), agent_files("zh-CN"));
        assert_eq!(agent_files("fr"), None);
        assert_eq!(localized("terms", "en"), None);
        assert_eq!(guide(Some("agent-files")).as_deref(), agent_files("en"));
        assert!(guide(None).unwrap().contains("# Agent files"));
        for topic in LOCALIZED_TOPICS {
            assert!(GUIDE_TOPICS.contains(&topic), "{topic}");
        }
        // The brief's three standing rules.
        let brief = flat(&crate::agent_section());
        for words in [
            "Shared rules live only in AGENTS.md; never add a shared rule to one agent's own file",
            "If you are an agent not listed under `## Agents set up` in the project notes, or your \
             own rule file does not point to AGENTS.md, run `alm guide agent-files` and do it once \
             for yourself before other work",
            "After changing shared rules, tell the person which running sessions still use the \
             old ones",
            "`alm list --state in_progress`",
            "`git worktree list`",
        ] {
            assert!(brief.contains(&flat(words)), "{words}");
        }
    }

    /// `alm guide github-release` (Task-A03-planet530): one text for every project, naming no
    /// repository, address or file; it reads the project's facts from the project notes, covers
    /// every step, the words after the command, and what it never does. Written in English only.
    #[test]
    fn github_release_is_generic_and_complete() {
        let text = guide(Some("github-release")).unwrap();
        assert_eq!(github_release("en"), Some(GITHUB_RELEASE_EN));
        assert_eq!(
            localized("github-release", "en-GB"),
            Some(GITHUB_RELEASE_EN)
        );
        assert_eq!(github_release("ja"), None);
        assert_eq!(github_release("zh-CN"), None);
        let low = text.to_lowercase();
        for word in ["allomos", "github.io", "docs/guide"] {
            assert!(!low.contains(word), "{word}");
        }
        for words in [
            "`## Releases`",
            "## 0. Read the project's release facts",
            "## 1. Preflight",
            "## 2. Collect what changed",
            "## 3. Choose the version",
            "## 4. Write the notes",
            "## 5. Bump, commit, tag",
            "## 6. Wait and check",
            "## 7. Download and install (only with `install`)",
            "## 8. Record and report",
            "`git log <last tag>..HEAD --no-merges`",
            "`gh auth status`",
            "`gh run watch <run id> --exit-status`",
            "`minor` or `major`",
            "- `install`: after step 6",
            "Without `install` in the command, skip this step",
            "never type a password",
            "Force-push, delete a tag or a release, or tag again.",
        ] {
            assert!(text.contains(words), "{words}");
        }
        for key in [
            "REMOTE",
            "BASE_BRANCH",
            "VERSION_FILES",
            "TAG_FORMAT",
            "BUMP",
            "RELEASE_WORKFLOW",
            "NOTES_DIR",
            "NOTES_LANGUAGES",
            "NOTES_INDEX",
            "NOTES_URL",
            "NOTES_PUBLISH",
            "GH_NOTES",
            "INSTALL_ASSET",
            "INSTALL_COMMAND",
            "VERIFY_INSTALLED",
            "APPROVAL_RECORD",
        ] {
            assert!(text.contains(&format!("| `{key}` |")), "{key}");
        }
        // The brief makes a line starting with `alm ` in the conversation a command to run.
        let flat = |t: &str| t.split_whitespace().collect::<Vec<_>>().join(" ");
        let brief = flat(&crate::agent_section());
        assert!(brief.contains(&flat(
            "A line in the conversation that starts with `alm ` followed by one of its commands"
        )));
        assert!(brief.contains("when it prints a guide, follow that guide"));
    }

    /// Task-A03-tide388: the size reminder, in every language the app speaks and in the workflow
    /// guide; only a reminder.
    #[test]
    fn the_file_size_reminder_is_written_in_every_language_and_in_the_workflow() {
        for (language, words) in [
            (
                "en",
                [
                    "1000 lines",
                    "only the part this task changes",
                    "a commit of its own before the feature commit",
                    "`alm block <this task> --by <new task>`",
                    "project-notes.md",
                ],
            ),
            (
                "zh-CN",
                [
                    "1000 行",
                    "只把本任务要改的那部分拆出来",
                    "放在功能提交之前",
                    "`alm block <本任务> --by <新任务>`",
                    "project-notes.md",
                ],
            ),
            (
                "ja",
                [
                    "1000 行",
                    "このタスクで変える部分だけ",
                    "機能のコミットより前",
                    "`alm block <このタスク> --by <新しいタスク>`",
                    "project-notes.md",
                ],
            ),
        ] {
            let text = claimed_task_steps(language);
            for w in words {
                assert!(text.contains(w), "{language}: {w}");
            }
        }
        assert_eq!(claimed_task_steps("fr"), claimed_task_steps("en"));
        let workflow = guide(Some("workflow")).unwrap();
        for w in [
            "**Before you plan: the size of the files you will change**",
            "about 1000 lines",
            "never the whole file",
            "alm only reminds",
        ] {
            assert!(workflow.contains(w), "{w}");
        }
        // AGENTS.md's section is not changed (approver, 2026-10-22).
        assert!(!crate::agent_section().contains("1000 lines"));
    }

    /// A topic written as separate words is the same topic; the words after it are the guide's.
    #[test]
    fn a_topic_may_be_written_as_separate_words() {
        let words = |s: &str| s.split(' ').map(String::from).collect::<Vec<_>>();
        assert_eq!(
            guide_topic(&words("github release")),
            Some(("github-release", vec![]))
        );
        assert_eq!(
            guide_topic(&words("github-release")),
            Some(("github-release", vec![]))
        );
        assert_eq!(
            guide_topic(&words("GitHub release minor")),
            Some(("github-release", words("minor")))
        );
        assert_eq!(
            guide_topic(&words("github release minor install")),
            Some(("github-release", words("minor install")))
        );
        assert_eq!(
            guide_topic(&words("run modes")),
            Some(("run-modes", vec![]))
        );
        assert_eq!(guide_topic(&words("terms")), Some(("terms", vec![])));
        assert_eq!(guide_topic(&words("github")), None);
        assert_eq!(guide_topic(&words("nope release")), None);
    }

    /// `alm guide goal` (Task-A03-25-01): the versioned goal prompt, written in every language the
    /// app speaks, each with the four phases, the commands of each phase, the `/goal` condition
    /// judged from the conversation alone, the rule for continuing, and the goal-run exception.
    #[test]
    fn goal_is_written_in_every_language_with_its_phases_and_condition() {
        let flat = |t: &str| t.split_whitespace().collect::<Vec<_>>().join(" ");
        let condition = "The latest `alm list --json` printed in this conversation shows every \
                         task of the goal done, dropped or waiting on the person (a task blocked \
                         only by waiting tasks counts as waiting).";
        for language in RUN_MODES_LANGUAGES {
            let text = goal(language).unwrap_or_else(|| panic!("{language}"));
            assert!(text.starts_with("# "), "{language}");
            assert!(
                text.lines()
                    .next()
                    .unwrap()
                    .contains(&format!("(goal prompt v{GOAL_PROMPT_VERSION})")),
                "{language}"
            );
            // The four phases, in order, and the part on keeping going.
            let mut at = 0;
            for phase in 1..=4 {
                let found = text[at..]
                    .find(&format!("\n## {phase}. "))
                    .unwrap_or_else(|| panic!("{language}: phase {phase}"));
                at += found + 1;
            }
            assert!(!text.contains("\n## 5. "), "{language}");
            assert_eq!(text.matches("\n## ").count(), 5, "{language}");
            for words in [
                // Clarify: one round, everything else recorded.
                "`alm guide run-modes`",
                "`alm ai-decided <id> \"<what and why>\"`",
                // Plan.
                "`alm add area \"<the app's name>\"`",
                "--title \"Goal: <the app, in one line>\"",
                "`alm describe <id> --from <file.md>`",
                "Goal run (goal prompt v3)",
                "`alm add area \"<name>\"`",
                "`alm add iteration \"<name>\" --release v0.1`",
                "`alm plan <id> <iteration>`",
                "`alm guide tasks`",
                "`alm ready <id>`",
                "`alm block <id> --by <task-id>`",
                // Build.
                "`alm next`",
                "--branch <the branch you are on>",
                "`alm label <id> \"<label>\"`",
                "`alm done <id> --without-human-check \"Goal run\" --steps <file.md>`",
                "`alm done <id> --without-human-check \"Goal run: try the app\" --steps <file.md>`",
                "`alm done <id>`",
                "`alm unblock <id>`",
                "`alm wait <id> --on \"<what the approver must do>\"`",
                "`alm drop <id>`",
                "`alm handoff`",
                "`alm handoff-rules`",
                // Finish.
                // Keep going: `/goal`, its condition, and continuing.
                "`/goal`",
                "Claude Code",
                "Codex",
                "`alm list --json`",
                "`alm list`",
                "`Goal:`",
                "`alm show <id>`",
                "3%",
                "2026-10-16",
                "AGENTS.md",
            ] {
                assert!(text.contains(words), "{language}: {words}");
            }
            // The condition is given word for word in English, so a checker reads the same text.
            assert!(flat(text).contains(condition), "{language}");
            assert_eq!(localized("goal", language), Some(text));
        }
        let en = flat(goal("en").unwrap());
        for words in [
            "ask the person once, right now, all your questions together in one message",
            "Ask nothing else, then or later",
            "Submitting the goal approved the plan in advance",
            "make your own tasks ready, without asking",
            "Record that approval once, on the goal task",
            "waits for the approver with `alm wait`, and you go on with other ready tasks",
            "There is no spend cap",
            "do not plan it again",
            "continue from `alm next`",
            "each ending in a version the person can try",
            "dates empty unless known",
            "is that release's acceptance check",
            "finish the goal task the same way, done and to check by the person",
            "the same way in either run mode",
            "which can be judged from the conversation alone",
            "Without `/goal`, the same holds: keep going",
        ] {
            assert!(en.contains(words), "{words}");
        }
        assert_ne!(goal("ja"), goal("en"));
        assert_ne!(goal("zh-CN"), goal("en"));
        assert_eq!(goal("zh-Hans-CN"), goal("zh-CN"));
        assert_eq!(goal("fr"), None);
        assert_eq!(guide(Some("goal")).as_deref(), goal("en"));
        assert!(guide(None).unwrap().contains("# Goal run"));
        // The exception it makes, said by `alm guide tasks`, `alm guide workflow` and the brief:
        // the agent adds areas and iterations and readies its own tasks, approved in advance by
        // the goal, recorded once on the goal task; outside effects still wait; no spend cap.
        let tasks = flat(&guide(Some("tasks")).unwrap());
        let workflow = flat(&guide(Some("workflow")).unwrap());
        let brief = flat(&crate::agent_section());
        for text in [&tasks, &workflow, &brief] {
            for words in [
                "alm guide goal",
                "approved the plan in advance",
                "(its title starts with `Goal:`)",
                "still waits for the approver",
            ] {
                assert!(text.contains(words), "{words}: {text}");
            }
        }
        assert!(tasks.contains("make them ready yourself (`alm ready <id>`)"));
        assert!(tasks.contains("Record that approval once, on the goal task"));
        assert!(tasks.contains("or in a goal run (below)"), "{tasks}");
        for text in [&workflow, &brief] {
            assert!(text.contains("recorded once on the goal task"), "{text}");
            assert!(text.contains("make your own tasks ready"), "{text}");
        }
        for text in [&workflow, &brief] {
            assert!(text.contains("spend cap"), "{text}");
            assert!(
                text.contains("the quota gate of `alm handoff-rules`"),
                "{text}"
            );
        }
        assert!(workflow.contains("Given the goal again, continue from `alm next`"));
        assert!(
            flat(&guide(Some("commands")).unwrap())
                .contains("`alm guide goal [--language <code>]`")
        );
    }

    /// The run-modes explanation exists in every language the app speaks (so every task language
    /// `alm` supports), each with both modes and the same after-run steps and commands.
    #[test]
    fn goal_request_carries_the_description_the_mode_and_the_prompt() {
        for language in RUN_MODES_LANGUAGES {
            let quiet = goal_request_in(language, "  A to-do list\n\nwith dates  \n", true);
            assert!(quiet.starts_with("## "), "{language}");
            assert!(
                quiet.contains("> A to-do list\n>\n> with dates\n"),
                "{language}"
            );
            assert!(quiet.contains("Run mode: quiet"), "{language}");
            assert!(!quiet.contains("Run mode: normal"), "{language}");
            assert!(quiet.ends_with(goal(language).unwrap()), "{language}");
            let normal = goal_request_in(language, "x", false);
            assert!(normal.contains("Run mode: normal"), "{language}");
        }
        // A language the prompt is not written in gets the English request.
        assert_eq!(
            goal_request_in("fr", "x", true),
            goal_request_in("en", "x", true)
        );
    }

    #[test]
    fn run_modes_is_written_in_every_language() {
        for language in RUN_MODES_LANGUAGES {
            let text = run_modes(language).unwrap_or_else(|| panic!("{language}"));
            assert!(text.starts_with("# "), "{language}");
            assert!(
                text.contains("\n---\n"),
                "{language}: the line before the person's part"
            );
            for words in [
                "`/loop`",
                "Run mode: quiet",
                "Run mode: normal",
                "`alm ai-decided`",
                "`alm wait`",
                "`--without-human-check`",
                "`alm human`",
                "To answer",
                "`alm ready <id>`",
                "To review: AI decisions",
                "`alm reviewed <id>`",
                "To check",
                "`alm checked <id>`",
                "`alm reopen <id> --note",
                "`alm show <id>`",
            ] {
                assert!(text.contains(words), "{language}: {words}");
            }
            // Task-A01-50: a finished task can be reopened now.
            assert!(!text.contains("cannot reopen"), "{language}");
            // Two modes, each with the four things the person must know.
            assert_eq!(text.matches("\n## ").count(), 2, "{language}");
            assert_eq!(text.matches("- **").count(), 8 + 3, "{language}");
        }
        // Every language the app picks has its own text, not the English one.
        for tag in ["en-US", "ja-JP", "zh-Hans-CN", "zh_CN"] {
            let picked = crate::pick_ui_language(&[tag]);
            assert!(RUN_MODES_LANGUAGES.contains(&picked), "{tag}");
            assert_eq!(run_modes(tag), run_modes(picked), "{tag}");
        }
        assert_ne!(run_modes("ja"), run_modes("en"));
        assert_ne!(run_modes("zh-CN"), run_modes("en"));
        assert_eq!(run_modes("fr"), None);
        assert_eq!(run_modes("zh-TW"), None);
        // Task-A01-50: the commands and the workflow say how to reopen a finished task.
        for topic in ["commands", "workflow", "tasks"] {
            let text = guide(Some(topic)).unwrap();
            assert!(text.contains("`alm reopen <id> --note"), "{topic}");
            assert!(!text.contains("cannot reopen"), "{topic}");
        }
        // `alm guide` (all topics) includes the English text.
        assert!(
            guide(None)
                .unwrap()
                .contains("# Run modes: quiet or normal")
        );
        assert_eq!(guide(Some("run-modes")).as_deref(), run_modes("en"));
    }

    /// Release checks in a goal run prefer tests and the command line, avoid page scripts on a
    /// local network address, and put a browser-only check in the person's steps (Task-A03-36).
    #[test]
    fn goal_prompt_checks_releases_without_browser_prompts() {
        for (language, words) in [
            (
                "en",
                [
                    "automated tests and command-line checks first",
                    "never\nrun page scripts on a local network address",
                    "write that in the task's steps for the person",
                ],
            ),
            (
                "zh-CN",
                [
                    "先用自动测试和命令行来证明",
                    "不要在\n`192.168.x.x` 这样的局域网地址上运行页面脚本",
                    "写进这个任务给那个人的步骤里",
                ],
            ),
            (
                "ja",
                [
                    "まず自動テストとコマンドラインで証明します",
                    "ページのスクリプトを実行しません",
                    "手順にそう書き",
                ],
            ),
        ] {
            let prompt = goal(language).unwrap();
            for w in words {
                assert!(prompt.contains(w), "{language}: {w}");
            }
            // Before the build loop's unblocking rules, inside phase 3.
            let check = prompt.find(words[0]).unwrap();
            assert!(check > prompt.find("## 3.").unwrap(), "{language}");
            assert!(check < prompt.find("## 4.").unwrap(), "{language}");
        }
    }
}
