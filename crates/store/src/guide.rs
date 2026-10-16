//! What `alm guide` tells an AI agent (Task-A05-02). The short brief in a project's `AGENTS.md`
//! ([`crate::agent_section`]) names these topics; an agent reads them from `alm` when it needs
//! them, so the details always match the installed `alm`, not a copy that went stale. English:
//! agents read it, the app does not show it. Two exceptions are written in every language the
//! Allomos app speaks: `run-modes` (Task-A03-21), which the agent shows to the person
//! ([`run_modes`]), and `agent-files` (Task-A03-24), the one-time procedure that puts every
//! agent's shared rules in AGENTS.md, which the person may read along ([`agent_files`]).

/// The topics, in the order `alm guide` prints them.
pub const GUIDE_TOPICS: [&str; 7] = [
    "terms",
    "customize",
    "tasks",
    "commands",
    "workflow",
    "run-modes",
    "agent-files",
];

/// The topics written in every language of [`RUN_MODES_LANGUAGES`], printed in the task language.
pub const LOCALIZED_TOPICS: [&str; 2] = ["run-modes", "agent-files"];

/// The languages `alm guide run-modes` and `alm guide agent-files` are written in: those of the Allomos app (`en`, `ja`,
/// `zh-CN`, as [`crate::pick_ui_language`] picks them).
pub const RUN_MODES_LANGUAGES: [&str; 3] = ["en", "ja", "zh-CN"];

const TERMS: &str = "# Allomos terms

Allomos relies on these terms. Use them as defined here, in commands and when you talk
about the project's tasks.

- **Taskbag**: the folder holding the project's tasks (`.allomos/taskbag/`): readable JSON and
  Markdown files tracked by Git. Change it only with `alm`, except the project notes and your
  own handoff file.
- **Project notes**: the taskbag's `project-notes.md`: what every session must know whatever its
  task (what is running, pitfalls, where credentials live, never their values). Edited directly
  and kept true while the project is built; `alm next` shows its Running and Pitfalls.
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
- **Handoff note**: what the next holder needs to know, left when a task is released.
- **Task language**: the language new and changed tasks are written in (titles, descriptions,
  notes). `alm task-language` tells you which; by default it follows the language the person
  uses Allomos in.
";

const CUSTOMIZE: &str = "# What a project may change, and what is fixed

| Term | This project may change | How |
|---|---|---|
| Areas | which areas exist, their IDs and names | `alm add area <id> \"<name>\"`, `alm rename area`, `alm delete area` (only an empty one) |
| Iterations | which exist, their names, releases and dates; which task is in which | `alm add iteration`, `alm change iteration`, `alm delete iteration` (only an empty one), `alm plan` |
| Label ladder | its steps and their order | `alm labels \"<a,b,…>\"` (refused if a task's label would be left off); `alm rename label \"<old>\" \"<new>\"` renames a step and the tasks that carry it |
| Task IDs and titles | any ID of letters, digits, `-` or `_` (for example A01-02) | `alm add task <id> --area <id> --title \"…\"` |
| Description | free Markdown in description.md | written with `alm add task --description` |
| Task language | `ui` (follow the Allomos app's language, the default) or a language code | `alm task-language <code or ui>`; only when the person asks |

Fixed for every project:

- **The seven states**: backlog, ready, in progress, waiting, blocked, done, dropped.
- **The words \"area\" and \"iteration\"**: a project cannot rename them yet (planned: requirement
  R90). Use these words with `alm`, even if the team says \"workstream\" or \"sprint\".
- **The file format**: never edit the taskbag's files by hand (except `project-notes.md` and your
  own handoff file); `alm` checks every change.

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
   person asks for it or agrees to it.
3. **ID.** Leave it out: `alm add task --area <area> --title \"…\"` takes the next free number in
   the area (A01-04 after A01-03). Give an ID only to follow a pattern the project already uses,
   for example a child task `A01-03-01` under `A01-03`.
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
   The description is the task's whole record: there is no separate card file. As the work goes
   on, keep it up to date with `alm describe <id> --from <file.md>` (its approvals, and what was
   checked, with numbers and logs, under an **Evidence** heading).
6. **Branch.** Say which Git branch you are on: `--branch <name>` (`git branch --show-current`;
   alm reads no Git). The new task gets a base branch, by its real name: the branch it starts
   from and merges back into. When the branch you are on is a task's working branch, the new
   task shares that task's base; otherwise its base is the branch you are on. alm prints which
   and why; `--base <name>` chooses another. When you hold a task on another base, alm says so:
   decide which is right, and change it with `alm base <id> <branch>` if needed.
7. **State.** A new task starts in backlog. Do not make it ready yourself: the person approves it
   (`alm ready <id>`). Plan it in an iteration (`alm plan <id> <iteration>`) only when asked.
8. **One outcome per task.** If it is really two outcomes, add two tasks.

When the project shares its tasks through Git, run the Git commands `alm add task` prints, so
others see the new task.
";

const COMMANDS: &str = "# alm commands

Every command takes `--taskbag <path>` (or ALLOMOS_TASKBAG; default: the nearest
`.allomos/taskbag/` above the current folder), `--as <name>` (or ALLOMOS_HOLDER), `--json`
(one JSON object on standard output). Exit codes: 0 done, 1 refused by a rule, 2 wrong usage,
3 taskbag busy or not writable.

`alm` never runs or reads Git. After a change it prints \"Commit and push this change:\" and the
exact `git add`, `git commit` and `git push` commands naming only the files that change wrote
(`--json`: `changed_files` and `git_commands`); run them yourself and leave other changes as they
are. `alm claim` asks you to push at once, before you start the work. Changes the person made in
the Allomos app and not shared yet come with their own commands, to commit as the person's.
`alm init` prints none: commit the new taskbag yourself.

See:
- `alm next [--iteration <id>]`: every ready task no one holds (in that iteration), each with its
  latest handoff, and the rule for choosing one; then the blocked tasks whose blockers are all
  done. `alm` chooses and moves nothing; you do.
- `alm list [--area <id>] [--iteration <id>] [--state <state>] [--mine] [--by area|iteration]`
  (`--state blocked`, `--state waiting`, …);
  `--without-human-check`: only tasks finished without the human check, not checked since.
- `alm human [--iteration <id>] [--area <id>] [--steps]`: what needs a person: **To check**
  (finished without the human check, each with why; `--steps` adds how to check it) and **To
  answer** (waiting on a person, each with what; backlog tasks are not listed), then
  **To review: AI decisions** (tasks in any state with decisions not reviewed yet, each listed).
  `--json`: `to_check`, `to_answer`, `to_review` (each task with its `ai_decisions`).
- `alm show <id>`: one task in full, with its description.
- `alm history <id>`: every recorded change to the task, newest first: when, who, what changed.

Work a task:
- `alm claim <id> --branch <the branch you are on> [--working <name>]`: you hold it; state in
  progress. It records the working branch (`--working`, default the branch you are on), gives a
  task without a base the branch you are on, prints the base to start from and merge into with
  the Git commands (`--json`: `branches`, `branch_commands`, `branch_facts`), and says so when
  you are not on the base. `--take` only when its holder has stopped.
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
- `alm done <id> [--without-human-check \"<why, which run>\" --steps <file.md>]`: finished (see
  `alm guide workflow`, step 7); it ends with the rule for checking the handoffs that name the
  task; follow it. The steps are Markdown for the person who checks it: the test folder, what to
  open, what to click, what they should see. On a task already done, it only records its human
  check again (the old `--without-owner` still works).
- `alm checked <id>`: a person has checked the finished task: it gets the top label and leaves
  To check; the note and steps stay in its history. Run it only when the person says so.
  It does not review the task's AI decisions.
- `alm ai-decided <id> \"<what and why>\"`: record a routine choice you made on your own in the
  task, in one line, for example `alm ai-decided A01-37 \"Names: alm describe, alm note — match
  alm title\"`. Any task, any state; the task's files do not change, the history keeps it.
- `alm reviewed <id>`: the person has reviewed the task's AI decisions: they leave To review; the
  history keeps them. Run it only when the person says so.
  `alm drop <id>`; `alm ready <id>` (an unheld backlog, waiting or blocked task).

Plan:
- `alm add area [<id>] \"<name>\"`, `alm add iteration [<id>] \"<name>\" [--release …] [--start …]
  [--end …]`: without an ID, the next free one, like A08 or I16.
- `alm rename area <id> \"<name>\"`; `alm change iteration <id> [--name …] [--release …]
  [--start …] [--end …]` (an empty value clears a release or date). IDs stay.
- `alm delete area <id>`, `alm delete iteration <id>`: refused while any task is in it, done and
  dropped ones too (plan them elsewhere first).
- `alm add task [<id>] --area <id> --title \"<title>\" [--description \"<markdown>\"]
  --branch <the branch you are on> [--base <name>]`: without an ID, the next free one in the
  area; the base branch as `alm guide tasks` says. Read `alm guide tasks` first.
- `alm base <id> <branch>`: change the branch a task starts from and merges into.
- `alm title <id> \"<title>\"`; `alm describe <id> --text \"<markdown>\"` or
  `alm describe <id> --from <file.md>`: a new title or description (`--text \"\"` removes the
  description). Same holder rule as `alm label`; each is recorded in `alm history`.
- `alm plan <id> [<iteration>]`; `alm labels \"<a,b,…>\"`; `alm rename label \"<old>\" \"<new>\"`
  (a step of the ladder, and every task that carries it)
- `alm task-language`: the language to write new and changed tasks in (run it before
  `alm add task`, `alm title` and `alm describe`);
  `alm task-language <code|ui>` sets it.

Handoffs (the note a session leaves so whoever continues does not start blind):
- `alm handoff-rules`: when to write one (quota and context gates, with any override for you);
  `--quota <n> --context <n> [--write-when \"<text>\"] [--agent <name>] [--clear]` change them.
- `alm handoff --tasks <id,id> --as <your session name>`: this session's handoff file, made from
  the template the first time (later calls update the same file); fill it in, or pass
  `--from <file.md>`. It prints the file's path, and reminds you to update `project-notes.md` if
  what is running or a pitfall changed.
- `alm handoffs [--task <id>] [--latest]`: handoffs, newest first. `alm next` and `alm show <id>`
  show a task's latest one.
- `alm handoffs --check [--task <id>]`: the rule for checking the handoffs yourself, one by one,
  and removing those whose tasks are all done or dropped.
- `alm handoff --remove <file>`: remove a handoff whose tasks are all finished (Git keeps it).
- `alm handoff-template [--set <file.md>]`: show or replace the project's template.

Ask the person:
- `alm confirm --title \"<heading>\" --plan <file.md>`: shows your plan in the Allomos app and
  waits; exit 0 means go ahead, anything else means change nothing.

About Allomos:
- `alm brief`: the section for AGENTS.md. `alm guide [terms|customize|tasks|commands|workflow]`:
  this guide.
- `alm guide run-modes [--language <code>]`: the explanation of quiet and normal mode to show the
  person before an automatic or loop run, in the task language (or `--language`); written in en,
  ja and zh-CN, otherwise in English with a line asking you to translate it.
- `alm guide agent-files [--language <code>]`: the one-time procedure that moves the rules every
  AI agent shares into AGENTS.md and leaves each agent's own file a pointer to it; same languages.
- `alm init`: create a taskbag (only when there is none), in `.allomos/taskbag/` of the current
  folder, or where `--taskbag` says.
- `alm move-in [--language <code>]`: when `alm` says the taskbag was written by an older
  Allomos (`taskbag_to_move_in`), the request for moving its tasks into a new taskbag, like
  another task system; tell the person first. It changes nothing; the person confirms the plan
  with `alm confirm`.
";

const WORKFLOW: &str = "# How to work the tasks

1. Start with `alm next` (or `alm list`). Choose by its rule: the task the person named, if any;
   otherwise one from the earliest iteration whose next dependency is met. Read its handoff
   first (`alm show <id>`), and the project notes' Running and Pitfalls, which `alm next` shows
   (the whole file is the taskbag's `project-notes.md`).
   **Taking over from a handoff** (`alm next` and `alm show` repeat these two rules with it):
   check the real state before trusting it: the worktree, branch and running processes it names
   (`git worktree list`, `git status -sb`, listening ports). What the previous session still
   holds is not yours to change or remove; if unsure, ask the person (in a quiet run, park the
   question with `alm wait`). What the handoff lists as already done (its **Already done, do not
   repeat** line) is not done again: a change of agent, account or session keeps the approvals
   and what was done; it does not authorize repeating it.
2. Hold one task at a time. Pull first, then `alm claim <id> --as <your session name>
   --branch <the branch you are on>` (`git branch --show-current`; alm reads no Git), and run
   the Git commands it prints right away: until your claim is pushed, other sessions do not see
   it and may take the task too. A task someone else holds is not yours to change.
   **Branches.** Every task has a base branch, by its real name (`main`, `master`,
   `storage-v2`…): it starts from it and merges back into it; merging one base branch into
   another is a task of its own. The claim records your working branch, the one you work on
   (`--working <name>` when it is not the one you are on, for example a new branch for a
   worktree of its own),
   prints which base to start from and merge into with the Git commands, and says so when you
   are not on the base. A task without a base gets the branch you are on. Work in the current
   checkout by default; make a worktree of its own only when the task needs one (for example
   when several sessions work at once). Once the task's branch is merged into its base, remove
   your own worktree and the merged branch; never another session's.
3. As you prove each step, `alm label <id> \"<label>\"`, and keep its next dependency current
   with `alm needs`.
4. Stopped: when a person (or an outside event) must act, `alm wait <id> --on \"<what>\"
   [--who \"<who>\"]`; when another task must finish first, `alm block <id> --by <task-id>` (if
   a person must act too, say so with `alm needs`). Blocked and waiting tasks are never in
   `alm next`'s list: they are skipped by their state. When `alm next` says all of a task's
   blockers are done, check it, then `alm unblock <id>`.
5. Write your handoff when `alm handoff-rules` says (near the quota or context gate, and after
   every meaningful step): `alm handoff --tasks <id> --as <your session name>`, then fill in the
   file it names, in plain words first, for any reader: you do not know who takes over, an agent
   or a person; under **Already done, do not repeat**, list each operation with an outside
   effect that must not run twice (or none). Evidence goes in the task's description (`alm describe`, under an **Evidence**
   heading), not in the handoff.
6. Stopping before it is done (for example your session ends): `alm release <id> --note \"<what
   the next one needs>\"`.
7. Finished: only once the task has the top label of the ladder (a person has checked it: the
   human check), then `alm done <id>`. The one exception: in an automatic or loop run the
   person set up to go on without them (a quiet run, step 8), you may finish a task
   below the top label with
   `alm done <id> --without-human-check \"<why, which run>\" --steps <file.md>`; say so in
   your handoff. Write the steps so a person can pick it up and test
   it: the test folder, what to open, what to click, what they should see. The task keeps its
   label, the note and the steps, so a person finds it with `alm human`; once they have checked
   it, `alm checked <id>`.
8. Before an automatic or loop run begins (a card run, `/loop`, several tasks one after
   another), ask the person to choose quiet or normal mode: show them what
   `alm guide run-modes` prints (in the task language), as it is, and do not start until they
   choose. Write the choice in your handoff (under Approvals in force: \"Run mode: quiet\" or
   \"Run mode: normal\", who chose it and when) and follow it until the run ends. In normal mode
   you work as usual: you stop and ask whenever you need the person. In quiet mode (a quiet run),
   never stop to wait for the person. A routine choice: decide it,
   record it with `alm ai-decided <id> \"<what and why>\"`, and carry on. A real question for the
   person: park that task with `alm wait <id> --on \"<the question>\"` (it shows in `alm human`
   under To answer) and go on with the next ready task. Do not use `alm confirm` in such a run: it
   would wait until it times out.

Rules:
- Never edit the taskbag's files by hand, and never invent progress, verification or approvals.
  Two exceptions: fill in your own handoff file, the one `alm handoff` names; and keep
  `project-notes.md` true. Update it when you start or stop a server or process, create or change
  a hosted resource or a Git remote, learn a pitfall that would change what the next session
  does, or a credential moves; change lines in place, remove lines no longer true, and commit and
  push that file yourself.
- Every new piece of work (a new card, a new outcome) gets its own task: `alm guide tasks`, then
  `alm add task`.
- Before adding a task or changing its title or description, run `alm task-language` and write in
  that language.
- Before a change the person must agree to, show your plan with `alm confirm` and change nothing
  unless it exits 0 (in a quiet run, park it with `alm wait` instead: step 8).
- Routine choices inside a task (a name, a file layout, how something is built inside the task, a
  small detail with an obvious default; anything cheap to undo) are yours: decide, carry on, and
  record each with `alm ai-decided <id> \"<what and why>\"`, so the approver can review it with
  `alm human`. What users will see and get used to, anything hard to undo (data formats, removing
  things), anything that changes the plan or another task, and anything with a cost or an outside
  effect goes to the approver first (`alm wait`).
- After every `alm` change, run the Git commands it prints: they name only that change's files.
  Your other work is committed as usual.
";

const RUN_MODES_EN: &str = "# Run modes: quiet or normal

For the agent: before an automatic or loop run begins (a card run, `/loop`, several tasks one
after another), show the person everything below the line, as it is, and ask them to choose
quiet or normal mode. Do not start until they choose. Write the choice in your handoff (under
Approvals in force: \"Run mode: quiet\" or \"Run mode: normal\", who chose it and when) and
follow it until the run ends.

---

Before this run starts, please choose how I will work: **quiet mode** or **normal mode**.

## Quiet mode: I go on without you

- **What I do:** I work the tasks one after another and never stop to ask you. I make routine
  choices myself (a name, a file layout, a small detail with an obvious default) and record each
  one (`alm ai-decided`), so you can review it later.
- **What I don't do:** I don't decide what is yours to decide: what users will see and get used
  to, anything hard to undo, anything that changes the plan or another task, anything with a cost
  or an outside effect. Such a question parks its task as waiting (`alm wait`), and I go on with
  the next ready task without waiting for an answer.
- **When you are interrupted:** not at all until the run is over. A task I finish without your
  check is marked to check, with the steps for checking it (`--without-human-check`).
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
     (`alm checked <id>`, or the Checked button in the app). When it does not, say what is wrong
     in its note (`alm note <id> \"<what is wrong>\"`) and add a task for the fix: Allomos cannot
     reopen a finished task yet.
2. Read the run's handoff (`alm handoffs` lists them, newest first) for anything left running or
   unfinished.

## Normal mode: I ask as usual

- **What I do:** I work as usual. Routine choices I still make and record myself; whenever I
  need you (a question only you can answer, an approval, a check by hand), I stop and ask.
- **What I don't do:** I don't go past a question without your answer.
- **When you are interrupted:** at each such question, approval or check.
- **How long it can run unattended:** only until the next question: the run pauses there until
  you answer.

Please stay reachable while it runs: it pauses at each question until you answer.
";

const RUN_MODES_JA: &str = "# 実行モード：静かモードか通常モードか

エージェントへ：自動実行やループ実行（カード・ラン、`/loop`、複数のタスクを続けて進めるもの）を
始める前に、下の線より後ろをそのまま相手に見せ、静かモードか通常モードかを選んでもらいます。
選ばれるまで始めません。選ばれたモードはハンドオフに書き（Approvals in force に
「Run mode: quiet」または「Run mode: normal」と、誰がいつ選んだか）、実行が終わるまでそれに従います。

---

この実行を始める前に、私の進め方を選んでください：**静かモード**か**通常モード**か。

## 静かモード：あなたを待たずに進めます

- **すること：** タスクを一つずつ続けて進め、あなたに聞くために止まることはありません。決まりきった
  選択（名前、ファイルの配置、自然な既定がある小さな点）は私が決め、あとで見直せるよう一つずつ記録
  します（`alm ai-decided`）。
- **しないこと：** あなたが決めるべきことは決めません。利用者が目にして慣れていくもの、元に戻しにくい
  こと、計画や他のタスクを変えること、費用や外部への影響があること。そうした質問があれば、その
  タスクを待機中にして（`alm wait`）、答えを待たずに次の準備完了のタスクへ進みます。
- **あなたが呼ばれるとき：** 実行が終わるまで一度もありません。あなたの確認なしに終えたタスクは、
  確認の手順を付けて確認待ちとして残します（`--without-human-check`）。
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
     （`alm checked <id>`、またはアプリの「確認済み」ボタン）。動かなければ、何がおかしいかをメモに
     書き（`alm note <id> \"<何がおかしいか>\"`）、直すためのタスクを追加します。終わったタスクを
     開き直す機能は、「肩代」にはまだありません。
2. この実行のハンドオフ（`alm handoffs` で新しい順に一覧できます）を読み、動いたままのものや
   終わっていないものがないか確かめます。

## 通常モード：いつもどおり聞きます

- **すること：** いつもどおり進めます。決まりきった選択は私が決めて記録しますが、あなたが必要な
  とき（あなたにしか答えられない質問、承認、手で試す確認）は止まって聞きます。
- **しないこと：** あなたの答えなしに質問の先へは進みません。
- **あなたが呼ばれるとき：** そうした質問、承認、確認のたびに。
- **無人で続けられる長さ：** 次の質問まで。そこで実行は止まり、あなたの答えを待ちます。

実行中は連絡がつくようにしておいてください。質問のたびに、答えるまで実行は止まります。
";

const RUN_MODES_ZH_CN: &str = "# 运行模式：安静模式还是普通模式

给智能体：在自动运行或循环运行（卡片连跑、`/loop`、一个接一个地做多个任务）开始之前，把下面
分隔线之后的内容原样给对方看，请对方选择安静模式或普通模式。对方选定之前不要开始。把选择写进
你的交接（在 Approvals in force 下写「Run mode: quiet」或「Run mode: normal」，以及谁、何时选的），
并一直照此运行到结束。

---

这次运行开始之前，请选择我的工作方式：**安静模式**还是**普通模式**。

## 安静模式：我不等你，继续往下做

- **我会做的：** 一个接一个地处理任务，从不停下来问你。常规的选择（名称、文件布局、有明显默认值的
  小细节）由我自己决定，并逐条记录（`alm ai-decided`），方便你之后审阅。
- **我不会做的：** 不替你决定该由你决定的事：用户会看到并习惯的东西、难以撤销的事、会改变计划或
  其他任务的事、有费用或对外有影响的事。遇到这类问题，我把那个任务设为等待中（`alm wait`），
  不等回答，接着做下一个就绪的任务。
- **什么时候打扰你：** 运行结束之前一次也不会。没有经过你检查就完成的任务，会标为待检查，并附上
  检查步骤（`--without-human-check`）。
- **能无人值守运行多久：** 直到没有就绪的任务，或者我这个会话的额度或上下文用完为止。我会让交接
  一直保持最新，谁都可以接着做。

运行结束后：

1. 运行 `alm human`（或在「换肩」应用里用「需要人处理」筛选），依次处理三组：
   - **To answer（待回答）：** 逐个回答被搁置的问题，然后把任务重新设为就绪（`alm ready <id>`），
     之后的运行会接着做。
   - **To review: AI decisions（待审阅的 AI 决定）：** 逐条阅读决定；保留它，或修改任务来推翻它；
     然后标为已审阅（`alm reviewed <id>`，或应用里的「已审阅」按钮）。
   - **To check（待检查）：** 按每个任务的检查步骤去试。没问题就标为已检查（`alm checked <id>`，
     或应用里的「已检查」按钮）。有问题就在它的备注里写明哪里不对（`alm note <id> \"<哪里不对>\"`），
     再添加一个修复任务：「换肩」目前还不能重新打开已完成的任务。
2. 阅读这次运行的交接（`alm handoffs` 按从新到旧列出），看看有没有仍在运行或没做完的东西。

## 普通模式：照常提问

- **我会做的：** 照常工作。常规的选择仍由我决定并记录；每当需要你（只有你能回答的问题、批准、
  亲手检查）时，我会停下来问你。
- **我不会做的：** 没有你的回答，不会越过问题继续做。
- **什么时候打扰你：** 每遇到这样的问题、批准或检查时。
- **能无人值守运行多久：** 只到下一个问题为止：运行会停在那里，直到你回答。

运行期间请保持能联系到你：每遇到一个问题，运行都会停下，直到你回答。
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
   example `- Claude Code: CLAUDE.md (imports AGENTS.md), 2026-10-16`. Then commit only those
   files. In the project: AGENTS.md and the agent files you changed
   (`git add <those files> && git commit -m \"Shared agent rules in AGENTS.md\" -- <those files>`).
   In the taskbag: `project-notes.md`
   (`git -C <taskbag> add project-notes.md && git -C <taskbag> commit -m \"Agents set up\" -- project-notes.md`).
   Leave every other change as it is. Push as the project's rules say; when they say nothing,
   only when the person asks.
5. **Tell the person** what moved, which files now point to AGENTS.md, and the contradictions
   waiting for their answer.

From then on:
- Shared rules live only in AGENTS.md; never add a shared rule to one agent's own file.
- After changing shared rules, tell the person which running sessions still use the old ones:
  those holding tasks in progress (`alm list --state in_progress`) and the worktrees they work in
  (`git worktree list`). A session reads the new rules when it starts again.
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
   `- Claude Code: CLAUDE.md (imports AGENTS.md), 2026-10-16`。然后只提交这些文件。在项目里：
   AGENTS.md 和你改过的智能体文件
   （`git add <those files> && git commit -m \"Shared agent rules in AGENTS.md\" -- <those files>`）。
   在任务包里：`project-notes.md`
   （`git -C <taskbag> add project-notes.md && git -C <taskbag> commit -m \"Agents set up\" -- project-notes.md`）。
   其他改动保持原样。按项目规则推送；规则没说时，只在用户要求时推送。
5. **告诉用户**移动了什么，哪些文件现在指向 AGENTS.md，以及哪些矛盾在等他们回答。

此后：
- 共用规则只放在 AGENTS.md；不要把共用规则加到某个智能体自己的文件里。
- 改了共用规则之后，告诉用户哪些正在运行的会话还在用旧规则：持有进行中任务的会话
  （`alm list --state in_progress`）和它们工作的工作树（`git worktree list`）。会话重新开始时
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
   それらのファイルだけをコミットします。プロジェクトでは AGENTS.md と変えたエージェントのファイル
   （`git add <those files> && git commit -m \"Shared agent rules in AGENTS.md\" -- <those files>`）。
   タスクバッグでは `project-notes.md`
   （`git -C <taskbag> add project-notes.md && git -C <taskbag> commit -m \"Agents set up\" -- project-notes.md`）。
   ほかの変更はそのままにします。プッシュはプロジェクトのルールに従い、ルールに何もなければ
   ユーザーに頼まれたときだけにします。
5. **ユーザーに伝えます：** 何を移したか、どのファイルが AGENTS.md を指すようになったか、どの
   食い違いが答えを待っているか。

これから先：
- 共通ルールは AGENTS.md にだけ置きます。共通ルールをあるエージェント自身のファイルに足しません。
- 共通ルールを変えたら、まだ古いルールで動いているセッションをユーザーに伝えます：進行中の
  タスクを持つセッション（`alm list --state in_progress`）と、それが作業しているワークツリー
  （`git worktree list`）。セッションは次に始めるときに新しいルールを読みます。
";

/// `alm guide agent-files` in `language`, or `None` when it is not written in that language.
pub fn agent_files(language: &str) -> Option<&'static str> {
    match written_in(language)? {
        "ja" => Some(AGENT_FILES_JA),
        "zh-CN" => Some(AGENT_FILES_ZH_CN),
        _ => Some(AGENT_FILES_EN),
    }
}

/// A topic of [`LOCALIZED_TOPICS`] in `language`, or `None` when the topic is not one of them or
/// is not written in that language (the agent then translates the English text faithfully).
pub fn localized(topic: &str, language: &str) -> Option<&'static str> {
    match topic {
        "run-modes" => run_modes(language),
        "agent-files" => agent_files(language),
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
                .contains("Do not make it ready yourself")
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
            "Already done, do not repeat",
            "is not done again",
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
        // Decision 0017: no `--commit` / `--push`; pull before claiming, push the claim at once,
        // and run the Git commands each change prints.
        for text in [guide(None).unwrap(), crate::agent_section()] {
            assert!(!text.contains("--push"), "{text}");
            assert!(!text.contains("--commit"), "{text}");
            assert!(
                text.contains("Pull first") || text.contains("pull first"),
                "{text}"
            );
            assert!(text.contains("until your claim is pushed"), "{text}");
            assert!(text.contains("Git commands"), "{text}");
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
            "commit only those files",
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

    /// The run-modes explanation exists in every language the app speaks (so every task language
    /// `alm` supports), each with both modes and the same after-run steps and commands.
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
                "`alm handoffs`",
            ] {
                assert!(text.contains(words), "{language}: {words}");
            }
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
        // `alm guide` (all topics) includes the English text.
        assert!(
            guide(None)
                .unwrap()
                .contains("# Run modes: quiet or normal")
        );
        assert_eq!(guide(Some("run-modes")).as_deref(), run_modes("en"));
    }
}
