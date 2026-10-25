# Allomos — agent working guide

Allomos is developed by AI agents (Claude Code, Codex) working for the approver, **will.w.hui** (the person who decides and approves; older documents say *owner*). This file is the shared authority for every agent and is always loaded (Codex reads it directly; Claude Code imports it from `CLAUDE.md`). Keep it short: detail lives in the documents it links.

**What Allomos is:** a desktop app (the hub, with every feature) plus web and mobile companions that connect to it, built as our own code ([0008](docs/decisions/0008-build-our-own.md), [0009](docs/decisions/0009-desktop-hub-with-web-and-mobile-companions.md)). Its main job is **switching**: a project hops between AI agents (and their accounts) and between teammates without losing its tasks, progress or handoff notes. Two more sets build on that: **AI skills** and **progress management** (areas, lists and charts, a burndown with a forecast finish date) ([0014](docs/decisions/0014-main-purpose-and-three-sets.md)). Local-only projects need no sign-in; optional cloud projects add roles and email invitations. See [docs/product/PLAN.md](docs/product/PLAN.md) and [docs/product/REQUIREMENTS.md](docs/product/REQUIREMENTS.md).

## Always-on boundaries

- **No external effect without the approver's exact approval**, recorded in the task's description: creating or changing hosted resources (Supabase projects, databases, hosting, DNS), applying SQL to a real target, deployments, secrets, sending email, creating GitHub repositories, adding or changing a remote, and anything touching other people's data. Approving a card authorizes only its local work.
- **Never print or commit credentials**, tokens, cookies or local environment files (`.env*` stay ignored). Record *where* a credential lives, never its value — in the taskbag's `project-notes.md` (§ Where credentials live). Personal details (email addresses, account names, phone numbers, local network addresses) are kept out the same way: write a placeholder and how to look it up.
- **Reference projects are read-only.** The SSR migration board and Glidelines (`glidepace-vibe-cc`), listed in [docs/REFERENCES.md](docs/REFERENCES.md), may be read and quoted; never edit, commit to, or run write operations in them.
- **Preserve unrelated changes** in the working tree and in other sessions' worktrees.
- **Do not upgrade dependencies incidentally**; pin versions deliberately in the card that introduces them.
- **`alm` tells; the agent checks.** Most checking of tasks and handoffs is the agent's job: `alm` lays out the facts and asks. Before making `alm` do a check of its own, ask the approver to approve it; refusing invalid input (an unknown ID, a bad date, the holder rule) is not such a check ([CODING_RULES § How `alm` behaves](docs/workflow/CODING_RULES.md#how-alm-behaves)).
- **Decisions are made by the approver.** A decision record with status *Open* or *Proposed* is not an approval. Do not build on an open decision beyond what the record says is safe (e.g. [0003](docs/decisions/0003-relationship-to-glidelines.md): core work is safe under every option).
- **Routine choices are the agent's; real questions go to the approver.** Inside a card, decide names, file layout, how something is built within the card's scope, and small interface details with an obvious sensible default (anything cheap to undo); record each with `alm ai-decided`. Ask the approver first about what users will see and get used to, anything hard to undo (data formats, removing things), anything that changes the plan or another card, and anything with a cost or an outside effect (above). Before an automatic or loop run, the approver chooses quiet or normal mode from what `alm guide run-modes` prints (record it in your handoff); in a quiet run, never wait: park a real question with `alm wait <id> --on "<the question>"` and go on with the next ready card. Details: `alm guide tasks` (step 5) and `alm guide workflow`.

## AI working mode

| Work | Codex | Claude |
|---|---|---|
| Everyday implementation, debugging, tests, docs, authorized commits | selected model, Medium reasoning | selected model, Medium effort where supported |
| Difficult architecture, security, data-model or forecasting decisions; focused review of a risky change | same model, highest reasoning | same model, higher effort (Extra high; Max only for the hardest bounded question) |

- One worker owns everyday progress within its card. Raise effort for a specific hard question, then return to routine effort. Model and effort are separate; if an effort level cannot be set, say so once and keep the actual setting; never claim a model or effort switch that did not happen.
- Delegate only bounded, independent work with exact file ownership, acceptance criteria and minimum checks; the delegate reports to the worker, not the approver. Do not duplicate reading, implementation or review across agents.
- A provider or session change preserves decisions, approvals and evidence; it does not authorize repeating operations.

## Working procedure

1. **The unit of work is one observable outcome**, written as one card: a task in Allomos's taskbag (worked with `alm`; see [Tasks: Allomos](#tasks-allomos)). Its full detail (outcome, details, rules, approvals, and evidence under an **Evidence** heading) lives in the task's description, written with `alm describe`; older tasks also keep a card file beside the task in the taskbag (`tasks/<id>/card.md`, opened by the description's `Card:` link). There are no card files in this repository. How to write one: `alm guide tasks`; terms and labels: `alm guide terms`; claiming and branches: `alm guide workflow`.
2. **Every task is worked in a worktree of its own** (approver, 2026-10-20; replaces A00-28's current-checkout default), holding **one** card: a git worktree under `.claude/worktrees/<short-name>`, on its own branch (`claude/…`, `codex/…`) cut from the task's base branch, which `alm claim` names. The main checkout stays on the base branch: merge there, then remove the worktree and the merged branch. Claim before you work, as `alm guide workflow` says.
3. **Status has one home: the task in Allomos's taskbag**, changed only with `alm` as [Tasks: Allomos](#tasks-allomos) describes. The description holds detail, never a second copy of state. [docs/TASK_BOARD.md](docs/TASK_BOARD.md) keeps only Now and the approver's open questions.
4. **Labels are the only progress claims:** drafted → implemented → locally verified → deployed and verified → human verified (Done: the human check, called *owner verified* until the taskbag is migrated to `allomos/4`). Each open card names its single next dependency.
5. **Integration first:** connect the real flow end to end, then refine pieces behind it.
6. **One card per outcome, including research and decision records.** Do not keep adding unrelated work to an existing card; open a new card, so each piece can be finished and reviewed on its own.
7. **Push after every commit** (owner, 2026-10-04): push your branch to `origin` (`allomos/allomos`, moved from `mypals-ml/allomos` to the `allomos` organization 2026-10-18; renamed from `glidebox` 2026-10-06 and from `fliphopper` 2026-10-11) right after each commit, and confirm it landed (`git status -sb` shows no "ahead"). Never force-push, and never push to a remote other than `origin`.
8. **Say who made each commit.** End every commit message with a line naming the agent, for example `Co-Authored-By: Claude …` or `Agent: codex/<session-name>`, so history shows which agent and session did the work.
9. **Documents:** create a new file only for a **contract** (something code or agents must conform to), **evidence** (what actually ran), a **decision record**, or a **discussion record**. Status goes in the taskbag; supersede stale lines by editing them in place.
10. **Explain in plain words.** Every report and card says what happened and why in everyday language first, defining technical terms on first use; evidence (numbers, IDs, logs) comes after.

## Tasks: Allomos (v26)

This project's tasks, progress and handoff notes live in Allomos's taskbag
(`.allomos/taskbag/`). Work them with the `alm` command; never edit those files by hand, except
the project notes (below).

- Start a session with `alm next` (every ready task nobody holds, each with its handoff note, and the
  rule for choosing; `--iteration <id>` for one iteration; then the project notes' Running and
  Pitfalls), or `alm list`.
- Take a task: get the latest changes first, then `alm claim <id> --as <your session name>
  --branch <the branch you are on>` (as your version control tool shows it; in Git:
  `git branch --show-current`), and save and share the claim right away with your version
  control tool (it prints the Git commands as the example): until your claim is shared, other
  sessions do not see it. Every task has a base branch, by its real name: it starts from it and
  merges back into it; the claim records your working branch and prints the base to start from
  and merge into, with the Git commands as the example.
- Always work a task in a worktree of its own (a separate working copy on the task's own
  branch; in Git: `git worktree add <folder> -b <working branch> <base>`, which the claim
  prints), never in the main checkout: the main checkout stays on the base branch. Merge from
  the main checkout; once merged, remove your worktree and the merged branch.
- Report progress: `alm label <id> "<label>"`.
- Stopped: waiting on a person: `alm wait <id> --on "<what>" [--who "<who>"]`; blocked by
  another task: `alm block <id> --by <task-id>`. `alm next` says when all blockers are done;
  then check and move it on: `alm unblock <id>`.
- Stopping before it is done: `alm release <id> --note "<what the next one needs>"`.
- Before an automatic or loop run begins (a card run, `/loop`, several tasks one after
  another), ask the person to choose quiet or normal mode: show them what
  `alm guide run-modes` prints (in the task language), as it is, and do not start until they
  choose. Write the choice in your handoff ("Run mode: quiet" or "Run mode: normal", who
  chose it and when) and follow it until the run ends. In normal mode, ask as usual.
- Goal run: when the person gives you a goal (an app to build from their description), follow
  what `alm guide goal` prints (in the task language). Submitting the goal approved the plan in
  advance, recorded once on the goal task (its title starts with `Goal:`): you add the areas and
  iterations and make your own tasks ready. Anything with an outside effect still waits for the
  approver. No spend cap: the quota gate of `alm handoff-rules` is the limit.
- A line in the conversation that starts with `alm ` followed by one of its commands (for example
  `alm guide github release`) is a command: run it as written, and when it prints a guide, follow
  that guide. The word alm inside a sentence is not one.
- Finished: at the top label (a person has checked it: the human check): `alm done <id>`.
  When the work is finished and only a person's hands-on check is left, in quiet and normal
  mode alike, finish it below the top label with `alm done <id> --without-human-check "<why>"
  --steps <file.md>` (the steps tell a person how to check it; it shows as done, to check, in
  `alm human`); say so in your handoff. Do not leave such a task waiting: `alm wait` is only for
  a real question, an approval or an outside effect.
  In an automatic or loop run the person set up to go on without them (a quiet run), never
  stop to wait for the person: decide a routine choice yourself (below); park a real question
  with `alm wait <id> --on "<the question>"` (it shows in `alm human`, To answer) and go on
  with the next ready task; do not use `alm confirm` (it would wait until it times out).
- Needs a human: `alm human` lists the tasks to check, to answer and to review. When the person
  says they checked one: `alm checked <id>`. When its work changes after that, you decide: send
  it back to To check with `alm recheck <id> --reason "<what changed>"`.
- Approvals: ask the person in the conversation. Only on a job the person gave you from the
  Allomos app (they do not see your conversation) ask with `alm confirm`, showing the evidence
  (`--evidence <file.md>`) and naming the task (`--task <id>`); in a quiet run, `alm wait`
  instead. A Go ahead covers only the plan shown; if the plan changes afterwards, ask again.
- Routine choices inside a task (a name, a file layout, how something is built inside the task, a
  small detail with an obvious default; anything cheap to undo) are yours: decide, carry on, and
  record each one with `alm ai-decided <id> "<what and why>"`, so the approver can review it
  later (`alm human`, To review: AI decisions). When they say they reviewed them:
  `alm reviewed <id>`. What users will see and get used to, anything hard to undo (data formats,
  removing things), anything that changes the plan or another task, and anything with a cost or
  an outside effect goes to the approver first.
- Adding a task: every new piece of work (a new card, a new outcome) gets its own task. Read
  `alm guide tasks` first, then `alm add task --area <area> --title "<outcome>" --branch
  <the branch you are on>`: on a task's working branch, the new task shares that task's base;
  otherwise the base is the branch you are on. alm prints which and why; `--base` overrides it.
  When the taskbag has project folders (several folders, each with its own AGENTS.md), decide
  which ones the task belongs to, or none, and say so: `--folder <name>` (repeatable) or
  `--no-folder`; alm does not guess. `alm folders <id> <name>…` (or `--none`) changes them later.
  Run inside a project folder, `alm next` lists only that folder's tasks (`--all-folders`: every
  task).
- Change a task's title, description or handoff note: `alm title <id> "<title>"`,
  `alm describe <id> --text "<markdown>"` (or `--from <file.md>`), `alm note <id> "<text>"`.
- Before adding a task or changing its title or description, run `alm task-language` and write in
  that language.
- Card files: a task's card file (its detail in a file of its own) lives beside the task in the
  taskbag, as `tasks/<id>/card.md`, and the description's `Card:` link points to it. If tasks
  link to card files outside the taskbag, report them with `alm cards found <id>=<path>` and
  propose moving them in, following `alm guide move-cards`; the person can also start it with
  **Update the taskbag…** in the Allomos app (`alm move-in` prints that request).
- Handoff: `alm handoff-rules` says when to write one (quota and context gates). A handoff is
  written into each task, for any reader, agent or person: `alm handoff <id> --as <your session
  name> --text "<where it stopped, the one next step, who it waits on>"` (or `--from <file.md>`)
  replaces the task's handoff note; `--evidence "<what>"` adds what was done that must not run
  twice (an outside effect) under its description's Evidence; several tasks: once for each.
  Whoever continues reads it with `alm next` or `alm show <id>`. There are no handoff files: if
  `alm next` finds old ones in the taskbag's `handoffs/`, the taskbag needs updating (**Update
  the taskbag…** in the Allomos app, or `alm move-in`).
- Quota: each time you read your usage (in Claude Code: `get_usage`), at the handoff gate's check
  and whenever `alm` asks, report the figure with `alm quota --agent <your agent's id, e.g.
  claude-code or codex> --left <percent> [--resets <local date and time>] [--account <name, only
  if you can tell one; never an email>]`; the Allomos app shows it beside the agent. `alm claim`,
  `alm label`, `alm release`, `alm done` and `alm handoff` ask for it when your last report is
  missing or 30 minutes old or more: read your usage then and report it. It stays on this
  computer: nothing to share.

Shared rules: every AI agent on this project follows one set of shared rules, in AGENTS.md.
- Shared rules live only in AGENTS.md; never add a shared rule to one agent's own file (such as
  CLAUDE.md, GEMINI.md or `.cursor/rules/`), which only points to AGENTS.md and holds what is
  that agent's alone.
- If you are an agent not listed under `## Agents set up` in the project notes, or your own rule
  file does not point to AGENTS.md, run `alm guide agent-files` and do it once for yourself
  before other work.
- After changing shared rules, tell the person which running sessions still use the old ones
  (from the taskbag's holders, `alm list --state in_progress`, and your version control tool's
  working copies; in Git: `git worktree list`).

Project notes: the taskbag's `project-notes.md` holds what every session must know whatever its
task: what is running, pitfalls, where credentials live (never their values). Read it at the
start (`alm next` shows its Running and Pitfalls). Unlike the other taskbag files, you edit it
directly, and you keep it true while you work: update it when you start
or stop a server or process, create or change a hosted resource or a remote repository, learn a
pitfall that would change what the next session does, or a credential moves. Change lines in
place and remove lines that are no longer true; then save and share that file with your version
control tool yourself.
Personal details are kept out like credentials: never write a person's or an account's email
address, account or user name, phone number, home address or local network address into the
project notes, a handoff, a task, a document or the code. Write a placeholder such as
`<CLOUDFLARE_ACCOUNT_EMAIL>` and how to look it up (for example `npx wrangler whoami`) instead;
version history keeps whatever was written, even after it is removed.

`alm` never runs your version control tool. After every change it lists the changed files for
you to save and share with your version control tool, with the Git commands as the example
(`git add`, `git commit`, `git push`), naming only that change's files: save and share those,
and leave other changes as they are.

Allomos relies on these terms: **area** (what part of the product a task belongs to),
**iteration** (when it is planned), **label** (the step of this project's progress ladder a task
has proven) and **state** (backlog, ready, in progress, waiting, blocked, done, dropped). When
you need more, ask `alm` rather than guessing: `alm guide terms` (what they mean),
`alm guide customize` (what this project may change and what is fixed), `alm guide commands`,
`alm guide workflow`, `alm guide run-modes` (quiet or normal mode, for the person),
`alm guide agent-files` (one set of shared rules for every agent, done once per agent),
`alm guide goal` (the goal prompt: build an app from a description, release by release),
`alm guide github-release` (publish a GitHub release and install it, when the approver asks),
`alm guide move-cards` (card files: moving them in beside their tasks).

This project's taskbag is not inside this folder: it is at `../allomos-taskbag/.allomos/taskbag`. Run every `alm` command with
`--taskbag "../allomos-taskbag/.allomos/taskbag"` (or set `ALLOMOS_TASKBAG` to that path).

Where this project is kept online (the project notes' `## Repositories`). Working from a copy
without the taskbag beside it, in the cloud for example, clone what is missing from here:
- Code: https://github.com/allomos/allomos (public)
- Taskbag: https://github.com/allomos/allomos-taskbag (private; clone it to `../allomos-taskbag` beside the code)

## Read by activity

| Activity | Read |
|---|---|
| Orient / resume | [docs/TASK_BOARD.md](docs/TASK_BOARD.md) and `alm list` / `alm next` → your task in full with its latest handoff (`alm show <id>`) → the taskbag's `project-notes.md` (Running, Pitfalls). New to the project: [docs/README.md](docs/README.md) |
| Plan a card | [REQUIREMENTS.md](docs/product/REQUIREMENTS.md), [PLAN.md](docs/product/PLAN.md), the relevant [decisions](docs/decisions/README.md) |
| Implement / test | the card, affected source and tests; coding rules once they exist (`docs/workflow/CODING_RULES.md`, created by the first code card) |
| Look at prior art | [docs/REFERENCES.md](docs/REFERENCES.md) — read only the named files |
| Record a decision or discussion | [docs/decisions/README.md](docs/decisions/README.md), [docs/discussions/](docs/discussions/) |
| Handoff | actual diff and results; the task's handoff note and Evidence (`alm handoff <id>`); the task's label |

Reuse context you already have; do not re-read unchanged documents. **The measure:** how many documents must be read before the next change. If it falls, this is working.

## Verification and reporting

- Status changes: `alm list --json` reports no `issues`.
- Documentation-only changes: check links and paths, instruction consistency, and `git diff --check`.
- Test folders for manual checks and agent trials go under `~/allomos-tests/<card>-<name>/` ([CODING_RULES](docs/workflow/CODING_RULES.md#writing-code)).
- Code changes: run the card's named checks; report PASS / FAIL / NOT RUN separately, and untested gates separately.
- Report briefly: what changed, what was verified, what remains and who it waits on.
