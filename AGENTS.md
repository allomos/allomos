# Allomos — agent working guide

Shared rules for every AI agent working for the approver, **will.w.hui**.

- **Docs:** read [docs/workflow/CODING_RULES.md](docs/workflow/CODING_RULES.md) before changing code; see [docs/README.md](docs/README.md) for product plans, requirements and decisions (a decision with status *Open* or *Proposed* is not an approval).
- **`alm` tells; the agent checks:** never add a check to `alm` without the approver's approval; refusing invalid input is not such a check ([CODING_RULES § How `alm` behaves](docs/workflow/CODING_RULES.md#how-alm-behaves)).
- **Dependencies:** do not upgrade dependencies incidentally; pin versions deliberately in the card that introduces them.

## Tasks: Allomos (v44)

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
  and merge into, with the Git commands as the example. When you claim, pass what the Allomos
  app needs to reopen your session: `--session-agent <your agent's id>`, `--session-id <your
  session's ID>`, and whatever else your agent needs to reopen it. In the Claude desktop app
  you must also pass `--session-kind desktop --app-session-id <its local_… ID>`: without them
  the Allomos app resumes your session in a terminal instead of opening it in the Claude app
  (in a terminal, pass `--session-kind terminal`). After a command that changes a task, `alm`
  lists what the task is still missing (session details, working branch, a handoff note), each
  with the command that fixes it; run it (session details: `alm session <id> --session-agent …
  --session-id …`).
- Always work a task in a separate working copy (worktree) of its own — under the directory the
  project or tool uses (`.worktrees/<short-name>` by default; never hardcode `.claude/`; in Git:
  `git worktree add .worktrees/<short-name> -b <working branch> <base>`, which the claim prints;
  keep `/.worktrees/` in the project's `.gitignore`), never in the main checkout: the main
  checkout stays on the base branch. Name each working branch `<agent>/…` (for example `claude/…`,
  `codex/…`, `antigravity/…`). Right after every code commit, share it to the default remote
  (`origin` in Git), verify nothing is unpushed (in Git: `git status -sb` shows no "ahead"), and
  never push to any other remote. End every code commit message with a line naming the agent (for
  example `Co-Authored-By: Claude …` or `Agent: codex/<session-name>`). Merge from the main
  checkout; once merged, remove your worktree and the merged branch.
- Run version control operations tidily, one command at a time (in Git: `add`, `commit`, `switch`,
  `merge`, `push`, `status`, `worktree remove`, `branch -d` each on its own — never chained with
  `&&`, `;`, `||` or pipes). Keep remote sharing (`push`) separate from local steps; inside a
  working copy, use consistent command forms rather than ad-hoc path rewrites; when sharing
  taskbag changes, run the exact commands and paths `alm` prints, one by one, without rewriting
  the path or appending extra status checks.
- While you hold a task (decision 0022): pass `--as <your session name>` to every `alm` command.
  Send a heartbeat at least every 10 minutes (the project's interval, `alm heartbeat-settings`):
  `alm beat --as <your session name>` (every task you hold, in one change), and save and share it
  like any change: a heartbeat counts only once shared. A change you make to your task beats
  along with it; reading never does. Before a long step that runs no `alm` command (a long build
  or test run), declare it: `alm beat <id> --busy <length>`, as long as you judge it needs
  (`--busy 0m` ends it early). A held task silent past the project's limit (30 minutes by
  default) with no busy window is lost: `alm next` lists it under "Lost, can be taken over" with
  the command. Take one over only to continue its work: claiming it is the takeover; go on on its
  working branch, and look at the former holder's worktree but change nothing there (that folder
  is of use only on its own computer; on another one, go by the working branch and the handoff
  note: only what was pushed counts). Exception: when the person chose "Commit and push these
  changes for it first" in the Allomos app's Take over… dialog, commit what is not committed there
  as WIP and push it to the working branch, and change nothing else there. Use `--take`
  only when the holder has stopped or a person asks for the takeover; on your own, take over only
  lost tasks. When `alm`
  says one of your tasks was taken over (`notice [taken_over]`), stop work on it at once (no more
  commits or pushes to its branch, do not claim it back), leave your worktree as it is, tell the
  person, and go on with another ready task or end. Details: `alm guide workflow`, step 2.
- While you hold a task, keep it ready for a takeover, also on another computer: only what is
  pushed reaches it. After each small step, update the handoff note (`alm handoff <id>`), not only
  at the end, and say what you are doing with your heartbeats: `alm beat --as <your session name>
  --doing "<what, in a few words>"`. While the project's setting to push work in progress at each
  heartbeat is on (`alm heartbeat-settings`; on by default), also commit and push to your working
  branch after each small step and at every heartbeat, half-finished work too, with WIP in the
  commit message, so at most one heartbeat interval of work is lost; `alm beat` reminds you and
  prints the commands. With the setting off it does not remind you, and those pushes are not asked
  for; the rest stays. A branch with many WIP commits may be merged with a squash; never force-push.
- Report progress: `alm label <id> "<label>"`.
- Stopped: waiting on a person: `alm wait <id> --on "<what>" [--who "<who>"]`. Another task
  must finish first: `alm block <id> --by <task-id>`. Blocked is not a state: the task keeps its
  state (backlog stays backlog) and is blocked while a blocker has not finished (done or
  dropped); `alm next` does not list it until then. When its blockers finish, nothing changes in
  it; read their AI decisions before you start it. To stop waiting for a blocker:
  `alm unblock <id>`.
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
- Finished: with a "by human" label (a person has checked it: the human check):
  `alm done <id>`. When the work is finished and only a person's hands-on check is left, in
  quiet and normal mode alike, finish it without one with `alm done <id> --without-human-check "<why>"
  --steps <file.md>` (the steps tell a person how to check it; it shows as done, to check, in
  `alm human`); say so in your handoff. Do not leave such a task waiting: `alm wait` is only for
  a real question, an approval or an outside effect.
  In an automatic or loop run the person set up to go on without them (a quiet run), never
  stop to wait for the person: decide a routine choice yourself, wording and look included
  (below); park a real question with `alm wait <id> --on "<the question> Proposal: <yours>"`
  (it shows in `alm human`, To answer) and go on with the next ready task; do not use
  `alm confirm` (it would wait until it times out). A parked question never holds back finished
  work: what is done and tested and does not depend on the answer is merged first, and a task
  whose work is all done ends with `alm done <id> --without-human-check`, its question recorded
  with `alm ai-decided` for review. A question about something cheap to undo, still unanswered
  when the run ends, is settled by your proposal, recorded with `alm ai-decided`. Details:
  `alm guide workflow`, step 8.
- Risk: before you finish a task (before `alm done`), record its risk yourself, with one
  sentence of why: `alm classify <id> --risk low|medium|high --touches
  production|others-data|none --why "<one sentence: which rules apply, and what in the work
  makes them apply>"`. You judge; alm records what you say and checks nothing, and the
  approver may change it when reviewing. When several rules apply, the highest one counts:
  - High: H1 an outside effect (a hosted resource, SQL on a real target, a deployment, a secret,
    sending email, creating a repository, adding or changing a remote, other people's data);
    H2 a release; H3 a change of data format (the format of the files of tasks, events or the
    taskbag; a migration); H4 removing something (a feature, a command, data, files);
    H5 permissions and security (credentials, pairing, sign-in, signing, who may do what);
    H6 a change of shared rules (AGENTS.md, `alm guide`); H7 making `alm` do one more check of
    its own.
  - Medium: M1 a large change (say 500 lines or more, or 3 modules or more); M2 a check that
    was not run, or finishing with `--without-human-check`; M3 words and look that users see;
    M4 the principle of how something is built (the kind of AI decision the approver reviews).
  - Low: everything else: only documents, internal tidying or tests, with every check passed.
  Read the rules this project added too (its settings, Risk management; kept in the
  taskbag's `project.json`, under `risk`, `rules`) and apply them the same way. This is about
  what is reviewed first once the work is done; what needs the approver before the work (below)
  stays as it is. The project may turn on two limits there (off by default; `alm classify <id>`
  lists those that are on): "a high-risk task cannot be finished without the human check"
  (`alm done --without-human-check` refuses a task recorded as high risk: once its work is done,
  park it for the check with `alm wait <id> --on "Human check: <what to try>"`), and "in a
  quiet run, an agent does not merge a high-risk task itself" (record the risk before you merge;
  for a high-risk task, push the finished, tested work to its working branch, do not merge it,
  park it with `alm wait <id> --on "Merge <working branch> into <base>: high risk, the
  project's setting has the approver merge it. Proposal: merge it."` and go on with the next
  ready task).
- Needs a human: `alm human` lists the tasks to check, to answer and to review. When the person
  says they checked one: `alm checked <id>`. When its work changes after that, you decide: send
  it back to To check with `alm recheck <id> --reason "<what changed>"`.
- Approvals: approving or readying a task authorizes only its local work; any outside effect
  requires exact approval from the approver recorded in the task's description. Ask the person in
  the conversation. Only on a job the person gave you from the Allomos app (they do not see your
  conversation) ask with `alm confirm`, showing the evidence (`--evidence <file.md>`) and naming
  the task (`--task <id>`); in a quiet run, `alm wait` instead. A Go ahead covers only the plan
  shown; if the plan changes afterwards, ask again.
- Routine choices inside a task (a name, a file layout, how something is built inside the task, a
  small detail with an obvious default; anything cheap to undo) are yours: decide, carry on, and
  record each one with `alm ai-decided <id> "<what and why>"`, so the approver can review it
  later (`alm human`, To review: AI decisions). When they say they reviewed them:
  `alm reviewed <id>`. Wording and look (what users see) are reviewed before release, not
  before the work: in a quiet run decide them, build and merge, and record them with
  `alm ai-decided` (with a screenshot when your agent can take one, saying where it is); the
  approver reviews them under To review and reopens the task to change them. What still goes to
  the approver first: data formats, removing things, permissions and security, anything with a
  cost or an outside effect, and anything that changes the plan or another task; a release still
  needs the approver's approval.
- Ask questions while planning, not while running: whoever makes a task ready first asks its
  open questions, so a ready task carries none. Write "show the approver first" into a task only
  when the approver asks for it.
- Adding a task: every new piece of work (a new card, a new outcome) gets its own task. Read
  `alm guide tasks` first, then `alm add task --area <area> --title "<outcome>" --branch
  <the branch you are on>`: on a task's working branch, the new task shares that task's base;
  otherwise the base is the branch you are on. alm prints which and why; `--base` overrides it.
  When the taskbag has project folders (several folders, each with its own AGENTS.md), decide
  which ones the task belongs to, or none, and say so: `--folder <name>` (repeatable) or
  `--no-folder`; alm does not guess. `alm folders <id> <name>…` (or `--none`) changes them later.
  Run inside a project folder, `alm next` lists only that folder's tasks (`--all-folders`: every
  task). Making it ready follows the project's setting "new tasks wait for the approver" (off
  by default; `alm new-task-settings`, and `alm add task` says which applies): off, make it ready
  yourself when it carries no open questions; on, leave it in backlog, tell the approver its ID
  and make it ready only once they agree (meanwhile go on with other work). In a goal run you
  make your tasks ready yourself either way: submitting the goal approved the plan in advance.
- Change a task's title, description or handoff note: `alm title <id> "<title>"`,
  `alm describe <id> --text "<markdown>"` (or `--from <file.md>`), `alm note <id> "<text>"`.
- Before adding a task or changing its title or description, run `alm task-language` and write in
  that language.
- Card files: a task's card file (its detail in a file of its own) lives beside the task in the
  taskbag, as `tasks/<id>/card.md`, and the description's `Card:` link points to it. If tasks
  link to card files outside the taskbag, report them with `alm cards found <id>=<path>` and
  propose moving them in, following `alm guide move-cards`; the person can also start it with
  **Update the taskbag…** in the Allomos app (`alm move-in` prints that request).
- Handoff: `alm handoff-rules` says when to write one (quota and context gates). The gates apply
  only while you hold a task (from `alm claim` until `alm done` or `alm release`); reaching one,
  write the handoff, then release the task (`alm release <id> --note`, naming the working branch),
  so it is ready again for the next session or loop run, which continues on that branch. A
  handoff is written into each task, for any reader, agent or person: `alm handoff <id> --as <your session
  name> --text "<where it stopped, the one next step, who it waits on>"` (or `--from <file.md>`)
  replaces the task's handoff note; `--evidence "<what>"` adds what was done that must not run
  twice (an outside effect) under its description's Evidence; several tasks: once for each.
  Whoever continues reads it with `alm next` or `alm show <id>`. There are no handoff files: if
  `alm next` finds old ones in the taskbag's `handoffs/`, the taskbag needs updating (**Update
  the taskbag…** in the Allomos app, or `alm move-in`).
- Write for a person first. A person reads every handoff note, every AI decision and every set
  of check steps, so each keeps one order. First, one sentence in plain words that a person can
  act on: where the work stands, or what was decided, as they would see it. Then what the person
  has to do, or that there is nothing to do; check steps say about how many minutes they take
  and which step matters most. Last, the technical detail: commit IDs, commands, task IDs, file
  names. Never open with a commit ID, a command or a file name. Details: `alm guide workflow`,
  step 5.
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
Credentials: never print or commit credentials, tokens, cookies or local environment files (keep
`.env*` ignored). Personal details are kept out like credentials: never write a person's or an
account's email address, account or user name, phone number, home address or local network address
into the project notes, a handoff, a task, a document or the code. Write a placeholder such as
`<CLOUDFLARE_ACCOUNT_EMAIL>` and how to look it up (for example `npx wrangler whoami`) instead;
version history keeps whatever was written, even after it is removed.

`alm` never runs your version control tool. After every change it lists the changed files for
you to save and share with your version control tool, with the Git commands as the example
(`git add`, `git commit`, `git push`), naming only that change's files: run each printed command
on its own (no `&&`, `;`, `||` or pipes, and no path rewrites), save and share only those files,
and leave other changes as they are.

Allomos relies on these terms: **area** (what part of the product a task belongs to),
**iteration** (when it is planned), **label** (the step of this project's progress ladder a task
has proven) and **state** (backlog, ready, in progress, waiting, done, dropped). When
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
