//! Allomos's section in a project's agent instruction files (`AGENTS.md`, `CLAUDE.md`) and the
//! requests that ask an agent to set a project up.

use super::*;

/// The heading of Allomos's section in a project's `AGENTS.md` / `CLAUDE.md`.
pub const AGENT_SECTION_HEADING: &str = "## Tasks: Allomos";

/// The section that tells an AI agent this project's tasks are in Allomos. English: it is
/// read by agents, not shown in the app.
pub fn agent_section() -> String {
    let version = versions::SECTION_VERSION;
    format!(
        "{AGENT_SECTION_HEADING} (v{version})

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
- Always work a task in a worktree of its own under the current project's `.worktrees/`
  directory (a separate working copy on the task's own branch; in Git:
  `git worktree add .worktrees/<short-name> -b <working branch> <base>`, which the claim prints;
  keep `/.worktrees/` in the project's `.gitignore`), never in the main checkout: the main
  checkout stays on the base branch. Merge from the main checkout; once merged, remove your
  worktree and the merged branch.
- While you hold a task (decision 0022): pass `--as <your session name>` to every `alm` command.
  Heartbeats apply only to sessions started by Allomos's automatic monitor (`alm claim --monitored`
  or `ALLOMOS_MONITORED=1`); manually started sessions do not send heartbeats and are never marked
  lost. In a monitored session, send a heartbeat at least every 10 minutes (the project's interval,
  `alm heartbeat-settings`):
  `alm beat --as <your session name>` (every monitored task you hold, in one change), and save and
  share it like any change: a heartbeat counts only once shared. A change you make to a monitored
  task beats along with it; reading never does, and `alm beat` writes nothing for manual tasks.
  Before a long step that runs no `alm` command (a long build
  or test run), declare it: `alm beat <id> --busy <length>`, as long as you judge it needs
  (`--busy 0m` ends it early). A monitored held task silent past the project's limit (30 minutes by
  default) with no busy window is lost: `alm next` lists it under \"Lost, can be taken over\" with
  the command. Take one over only to continue its work: claiming it is the takeover; go on on its
  working branch, and look at the former holder's worktree but change nothing there (that folder
  is of use only on its own computer; on another one, go by the working branch and the handoff
  note: only what was pushed counts). Exception: when the person chose \"Commit and push these
  changes for it first\" in the Allomos app's Take over… dialog, commit what is not committed there
  as WIP and push it to the working branch, and change nothing else there. Use `--take`
  only when the holder has stopped or a person asks for the takeover; on your own, take over only
  lost tasks. When `alm`
  says one of your tasks was taken over (`notice [taken_over]`), stop work on it at once (no more
  commits or pushes to its branch, do not claim it back), leave your worktree as it is, tell the
  person, and go on with another ready task or end. Details: `alm guide workflow`, step 2.
- While you hold a task, keep it ready for a takeover, also on another computer: only what is
  pushed reaches it. After each small step, update the handoff note (`alm handoff <id>`), not only
  at the end, and say what you are doing with your heartbeats: `alm beat --as <your session name>
  --doing \"<what, in a few words>\"`. While the project's setting to push work in progress at each
  heartbeat is on (`alm heartbeat-settings`; on by default), also commit and push to your working
  branch after each small step and at every heartbeat, half-finished work too, with WIP in the
  commit message, so at most one heartbeat interval of work is lost; `alm beat` reminds you and
  prints the commands. With the setting off it does not remind you, and those pushes are not asked
  for; the rest stays. A branch with many WIP commits may be merged with a squash; never force-push.
- Report progress: `alm label <id> \"<label>\"`.
- Stopped: waiting on a person: `alm wait <id> --on \"<what>\" [--who \"<who>\"]`. Another task
  must finish first: `alm block <id> --by <task-id>`. Blocked is not a state: the task keeps its
  state (backlog stays backlog) and is blocked while a blocker has not finished (done or
  dropped); `alm next` does not list it until then. When its blockers finish, nothing changes in
  it; read their AI decisions before you start it. To stop waiting for a blocker:
  `alm unblock <id>`.
- Stopping before it is done: `alm release <id> --note \"<what the next one needs>\"`.
- Before an automatic or loop run begins (a card run, `/loop`, several tasks one after
  another), ask the person to choose quiet or normal mode: show them what
  `alm guide run-modes` prints (in the task language), as it is, and do not start until they
  choose. Write the choice in your handoff (\"Run mode: quiet\" or \"Run mode: normal\", who
  chose it and when) and follow it until the run ends. In normal mode, ask as usual.
- Goal run: when the person gives you a goal (an app to build from their description), follow
  what `alm guide goal` prints (in the task language). Submitting the goal approved the plan in
  advance, recorded once on the goal task (its title starts with `Goal:`): you add the areas and
  iterations and make your own tasks ready. Anything with an outside effect still waits for the
  approver. No spend cap: the quota gate of `alm handoff-rules` is the limit.
- A line in the conversation that starts with `alm ` followed by one of its commands (for example
  `alm guide github release`) is a command: run it as written, and when it prints a guide, follow
  that guide. The word alm inside a sentence is not one.
- Finished: with a \"by human\" label (a person has checked it: the human check):
  `alm done <id>`. When the work is finished and only a person's hands-on check is left, in
  quiet and normal mode alike, finish it without one with `alm done <id> --without-human-check \"<why>\"
  --steps <file.md>` (the steps tell a person how to check it; it shows as done, to check, in
  `alm human`); say so in your handoff. Do not leave such a task waiting: `alm wait` is only for
  a real question, an approval or an outside effect.
  In an automatic or loop run the person set up to go on without them (a quiet run), never
  stop to wait for the person: decide a routine choice yourself, wording and look included
  (below); park a real question with `alm wait <id> --on \"<the question> Proposal: <yours>\"`
  (it shows in `alm human`, To answer) and go on with the next ready task; do not use
  `alm confirm` (it would wait until it times out). A parked question never holds back finished
  work: what is done and tested and does not depend on the answer is merged first, and a task
  whose work is all done ends with `alm done <id> --without-human-check`, its question recorded
  with `alm ai-decided` for review. A question about something cheap to undo, still unanswered
  when the run ends, is settled by your proposal, recorded with `alm ai-decided`. Details:
  `alm guide workflow`, step 8.
- Risk: before you finish a task (before `alm done`), record its risk yourself, with one
  sentence of why: `alm classify <id> --risk low|medium|high --touches
  production|others-data|none --why \"<one sentence: which rules apply, and what in the work
  makes them apply>\"`. You judge; alm records what you say and checks nothing, and the
  approver may change it when reviewing. When several rules apply, the highest one counts:
{risk_rules}  Read the rules this project added too (its settings, Risk management; kept in the
  taskbag's `project.json`, under `risk`, `rules`) and apply them the same way. This is about
  what is reviewed first once the work is done; what needs the approver before the work (below)
  stays as it is. The project may turn on two limits there (off by default; `alm classify <id>`
  lists those that are on): \"a high-risk task cannot be finished without the human check\"
  (`alm done --without-human-check` refuses a task recorded as high risk: once its work is done,
  park it for the check with `alm wait <id> --on \"Human check: <what to try>\"`), and \"in a
  quiet run, an agent does not merge a high-risk task itself\" (record the risk before you merge;
  for a high-risk task, push the finished, tested work to its working branch, do not merge it,
  park it with `alm wait <id> --on \"Merge <working branch> into <base>: high risk, the
  project's setting has the approver merge it. Proposal: merge it.\"` and go on with the next
  ready task).
- Needs a human: `alm human` lists the tasks to check, to answer and to review. When the person
  says they checked one: `alm checked <id>`. When its work changes after that, you decide: send
  it back to To check with `alm recheck <id> --reason \"<what changed>\"`.
- Approvals: ask the person in the conversation. Only on a job the person gave you from the
  Allomos app (they do not see your conversation) ask with `alm confirm`, showing the evidence
  (`--evidence <file.md>`) and naming the task (`--task <id>`); in a quiet run, `alm wait`
  instead. A Go ahead covers only the plan shown; if the plan changes afterwards, ask again.
- Routine choices inside a task (a name, a file layout, how something is built inside the task, a
  small detail with an obvious default; anything cheap to undo) are yours: decide, carry on, and
  record each one with `alm ai-decided <id> \"<what and why>\"`, so the approver can review it
  later (`alm human`, To review: AI decisions). When they say they reviewed them:
  `alm reviewed <id>`. Wording and look (what users see) are reviewed before release, not
  before the work: in a quiet run decide them, build and merge, and record them with
  `alm ai-decided` (with a screenshot when your agent can take one, saying where it is); the
  approver reviews them under To review and reopens the task to change them. What still goes to
  the approver first: data formats, removing things, permissions and security, anything with a
  cost or an outside effect, and anything that changes the plan or another task; a release still
  needs the approver's approval.
- Ask questions while planning, not while running: whoever makes a task ready first asks its
  open questions, so a ready task carries none. Write \"show the approver first\" into a task only
  when the approver asks for it.
- Adding a task: every new piece of work (a new card, a new outcome) gets its own task. Read
  `alm guide tasks` first, then `alm add task --area <area> --title \"<outcome>\" --branch
  <the branch you are on>`: on a task's working branch, the new task shares that task's base;
  otherwise the base is the branch you are on. alm prints which and why; `--base` overrides it.
  When the taskbag has project folders (several folders, each with its own AGENTS.md), decide
  which ones the task belongs to, or none, and say so: `--folder <name>` (repeatable) or
  `--no-folder`; alm does not guess. `alm folders <id> <name>…` (or `--none`) changes them later.
  Run inside a project folder, `alm next` lists only that folder's tasks (`--all-folders`: every
  task). Making it ready follows the project's setting \"new tasks wait for the approver\" (off
  by default; `alm new-task-settings`, and `alm add task` says which applies): off, make it ready
  yourself when it carries no open questions; on, leave it in backlog, tell the approver its ID
  and make it ready only once they agree (meanwhile go on with other work). In a goal run you
  make your tasks ready yourself either way: submitting the goal approved the plan in advance.
- Change a task's title, description or handoff note: `alm title <id> \"<title>\"`,
  `alm describe <id> --text \"<markdown>\"` (or `--from <file.md>`), `alm note <id> \"<text>\"`.
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
  name> --text \"<where it stopped, the one next step, who it waits on>\"` (or `--from <file.md>`)
  replaces the task's handoff note; `--evidence \"<what>\"` adds what was done that must not run
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
has proven) and **state** (backlog, ready, in progress, waiting, done, dropped). When
you need more, ask `alm` rather than guessing: `alm guide terms` (what they mean),
`alm guide customize` (what this project may change and what is fixed), `alm guide commands`,
`alm guide workflow`, `alm guide run-modes` (quiet or normal mode, for the person),
`alm guide agent-files` (one set of shared rules for every agent, done once per agent),
`alm guide goal` (the goal prompt: build an app from a description, release by release),
`alm guide github-release` (publish a GitHub release and install it, when the approver asks),
`alm guide move-cards` (card files: moving them in beside their tasks).
",
        risk_rules = classify::RISK_RULES
    )
}

/// A folder as a request shows it to the agent: on Windows with `\` throughout
/// ([`plain_path`]), so a taskbag at `C:\work\app` reads `C:\work\app\.allomos\taskbag`.
pub(crate) fn shown(path: &Path) -> String {
    plain_path(path).display().to_string()
}

/// The last line of a request: the agent takes Allomos's section from `alm brief`, so it always
/// matches the installed `alm`. `alm` must be usable where the agent works (owner, 2026-10-10), so
/// the request carries no copy of the section.
pub(crate) fn section_source(language: &str, taskbag: &Path) -> String {
    let brief = format!("alm --taskbag \"{}\" brief", shown(taskbag));
    match language {
        "zh-CN" => {
            format!("Allomos 的一节就是 `{brief}` 的输出：运行它，把输出原样（英文）作为这一节。")
        }
        "ja" => format!(
            "Allomos の節は `{brief}` の出力です。実行して、その出力をそのまま（英語のまま）節にします。"
        ),
        _ => format!(
            "Allomos's section is what `{brief}` prints: run it and use its output exactly as it is."
        ),
    }
}

/// [`agent_section`], plus where the taskbag is when it is not inside `project` (written relative
/// to `project`, so the file works on every computer that has both folders side by side), plus
/// where the code and the taskbag are kept online when the project notes say (Task-A03-30).
pub fn agent_section_for(project: &Path, taskbag: &Path) -> String {
    format!(
        "{}{}",
        taskbag_place(project, taskbag),
        repositories_lines(
            &handoff::project_notes(taskbag)
                .repositories
                .unwrap_or_default()
        )
    )
}

/// The lines naming where the code and the taskbag are kept online (the project notes'
/// `## Repositories` items, as written), for a copy without its taskbag beside it, as an agent in
/// the cloud has (Task-A03-30). Empty when the notes name none.
pub fn repositories_lines(items: &[String]) -> String {
    if items.is_empty() {
        return String::new();
    }
    let mut list = String::new();
    for item in items {
        list.push_str(&format!("- {item}\n"));
    }
    format!(
        "
Where this project is kept online (the project notes' `## Repositories`). Working from a copy
without the taskbag beside it, in the cloud for example, clone what is missing from here:
{list}"
    )
}

/// [`agent_section`], plus where the taskbag is when it is not inside `project`.
fn taskbag_place(project: &Path, taskbag: &Path) -> String {
    let section = agent_section();
    // Tidy `.` and `..` first: `/work/app/../tasks` is not inside `/work/app`.
    let (project, taskbag) = (&tidy(project), &tidy(taskbag));
    if taskbag.starts_with(project) {
        return section;
    }
    // Relative, it is written with `/`, so the same file works on macOS and Windows.
    let at = relative_path(project, taskbag)
        .map(|p| portable_path(&p))
        .unwrap_or_else(|| taskbag.display().to_string());
    format!(
        "{section}
This project's taskbag is not inside this folder: it is at `{at}`. Run every `alm` command with
`--taskbag \"{at}\"` (or set `ALLOMOS_TASKBAG` to that path).
"
    )
}

/// `path` without `.` and `..` parts, worked out from the text alone (the disk is not asked).
pub fn tidy(path: &Path) -> PathBuf {
    let mut clean = PathBuf::new();
    for part in path.components() {
        match part {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !clean.pop() {
                    clean.push("..");
                }
            }
            other => clean.push(other),
        }
    }
    clean
}

/// What `alm init` (and later the app) asks an AI agent to do: add [`agent_section`] to the
/// project's instruction files, merged with what is already there. Plain numbered steps, so any
/// agent can follow them.
pub fn agent_setup_request(project: &Path, taskbag: &Path) -> String {
    agent_setup_request_in("en", project, taskbag)
}

/// [`agent_setup_request`] in an interface language (`en`, `zh-CN`, `ja`; English otherwise).
/// The steps follow the language; the section itself stays English, so every agent and teammate
/// reads the same rules.
pub fn agent_setup_request_in(language: &str, project_path: &Path, taskbag_path: &Path) -> String {
    let steps = setup_steps_in(language, project_path, taskbag_path);
    let source = section_source(language, taskbag_path);
    format!("{steps}\n\n{source}\n")
}

/// The setup request's text without its last line (where Allomos's section comes from), so the
/// connect request can carry it as its first part.
pub(crate) fn setup_steps_in(language: &str, project_path: &Path, taskbag_path: &Path) -> String {
    setup_steps_with(language, project_path, taskbag_path, GitStep::Ask)
}

/// What the agent does when the project folder is not in a Git repository yet (Task-A03-35).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitStep {
    /// Ask the person first (decision 0017): connecting a project, and normal goal runs.
    Ask,
    /// Run `git init` itself and record it as an AI decision: a quiet goal run (approver,
    /// 2026-10-19), where a local repository is a routine choice, easy to undo.
    Create,
}

/// [`setup_steps_in`] with the Git step chosen.
/// The taskbag's other project folders (`allomos/6`, decision 0019) are named after the steps, so
/// one request sets up every folder (Task-A05-king717).
pub(crate) fn setup_steps_with(
    language: &str,
    project_path: &Path,
    taskbag_path: &Path,
    git: GitStep,
) -> String {
    let steps = main_setup_steps(language, project_path, taskbag_path, git);
    match other_folders_steps(language, &other_project_folders(project_path, taskbag_path)) {
        Some(more) => format!("{steps}\n\n{more}"),
        None => steps,
    }
}

/// The taskbag's project folders other than `project_path`, in `project.json`'s order; none when
/// it cannot be read or has no folders.
fn other_project_folders(project_path: &Path, taskbag_path: &Path) -> Vec<PathBuf> {
    let Some(bag) = load(taskbag_path).taskbag else {
        return Vec::new();
    };
    let same = |a: &Path, b: &Path| match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    };
    project_folder_paths(taskbag_path, &bag.project)
        .into_iter()
        .map(|(_, path)| tidy(&path))
        .filter(|path| !same(path, project_path))
        .collect()
}

/// What the agent does in the other project folders: steps 2 to 6 again in each, saving each
/// folder's changes in that folder. `None` without other folders.
fn other_folders_steps(language: &str, others: &[PathBuf]) -> Option<String> {
    if others.is_empty() {
        return None;
    }
    let list: String = others
        .iter()
        .map(|p| ["\n- ", &shown(p)].concat())
        .collect();
    Some(match language {
        "zh-CN" => format!(
            "这个任务包还服务于下面这些项目文件夹：{list}\n\n在每个文件夹中也做一遍第 2 到第 6 步（第 6 步保存的是那个文件夹自己的改动）。Allomos 已经在它们的 AGENTS.md 里加上了自己的一节：检查一下，再完成其余步骤。"
        ),
        "ja" => format!(
            "このタスクバッグは、次のプロジェクトのフォルダーにも使われています：{list}\n\nそれぞれのフォルダーでも手順 2〜6 を行ってください（手順 6 で保存するのは、そのフォルダー自身の変更です）。Allomos はすでに各フォルダーの AGENTS.md に自分の節を加えています。確かめてから残りの手順を進めてください。"
        ),
        _ => format!(
            "This taskbag also serves these project folders:{list}\n\nDo steps 2 to 6 in each of them as well (in step 6, save that folder's own changes). Allomos has already added its section to each one's AGENTS.md: check it, then do the rest."
        ),
    })
}

/// [`setup_steps_with`] for the main project folder alone.
fn main_setup_steps(
    language: &str,
    project_path: &Path,
    taskbag_path: &Path,
    git: GitStep,
) -> String {
    let (project, taskbag, heading) = (
        shown(project_path),
        shown(taskbag_path),
        AGENT_SECTION_HEADING,
    );
    let no_repo = match (language, git) {
        ("zh-CN", GitStep::Ask) => {
            "如果项目文件夹还没有纳入版本控制，先问用户是否要建立（用 Git 时：在项目文件夹中运行 `git init`）；用户不同意，就跳过下面所有关于版本控制的部分。"
        }
        ("zh-CN", GitStep::Create) => {
            "如果项目文件夹还没有纳入版本控制，自己建立（用 Git 时：自己在项目文件夹中运行 `git init`），不要停下来问：这是安静模式的运行，本地仓库很容易撤销。等目标任务建好后，在它上面记下来：`alm ai-decided <目标任务的 ID> \"项目文件夹没有纳入版本控制，建立了版本控制（git init）\"`。"
        }
        ("ja", GitStep::Ask) => {
            "プロジェクトのフォルダーがまだバージョン管理されていなければ、始めるかどうか（Git なら、プロジェクトのフォルダーで `git init`）をユーザーに尋ねます。断られたら、以下のバージョン管理の部分はすべて飛ばします。"
        }
        ("ja", GitStep::Create) => {
            "プロジェクトのフォルダーがまだバージョン管理されていなければ、尋ねずに自分で始めます（Git なら、プロジェクトのフォルダーで `git init` を実行します）。静かなモードの実行で、ローカルのリポジトリは簡単に元に戻せるからです。ゴールのタスクができたら、そのタスクに記録します：`alm ai-decided <ゴールのタスクの ID> \"プロジェクトのフォルダーがバージョン管理されていなかったので始めた（git init）\"`。"
        }
        (_, GitStep::Ask) => {
            "If the project folder is not under version control yet, ask the person whether to set it up (for Git: `git init` in the project folder); if they say no, skip every version control part below."
        }
        (_, GitStep::Create) => {
            "If the project folder is not under version control yet, set it up yourself (for Git: `git init` in the project folder) without stopping to ask: this run is quiet, and a local repository is easy to undo. Once the goal task exists, record it on that task: `alm ai-decided <goal task id> \"The project folder was not under version control, so I set it up (git init)\"`."
        }
    };
    match language {
        "zh-CN" => format!(
            "Allomos 已为这个项目建立了任务包。现在请把它告诉以后的 AI 智能体。

项目文件夹：{project}
任务包（Allomos 的任务文件）：{taskbag}

在项目文件夹中：

1. 版本控制：Allomos 自己从不运行版本控制工具。{no_repo}在版本控制下，先用你的版本控制工具单独保存任务包和它的改动（包括记录的项目文件夹），用 Git 时：`git -C \"{taskbag}\" add -- . && git -C \"{taskbag}\" commit -m \"Add Allomos taskbag\" -- .`（如果 Allomos 在 `.allomos/` 旁边留下了 README-allomos.md，在两条命令末尾都加上 `../../README-allomos.md`；不要带上其他文件，也不要共享（推送））。
2. 打开 AGENTS.md（如果没有就新建）。
3. 如果 AGENTS.md 已经有关于 Allomos 的说明（例如标题为“{heading}”的一节，或其他关于 Allomos 或 `alm` 的文字），用 Allomos 的一节（见最后一行）替换它们；否则添加这一节。
4. 如果 AGENTS.md 中有其他关于跟踪任务或状态的规则（例如 Markdown 任务看板），把它们改为指向 Allomos，避免规则冲突。不要改动无关内容，也不要重复添加相同的说明。
5. 共用规则放进 AGENTS.md，每个智能体做一次：运行 `alm --taskbag \"{taskbag}\" guide agent-files` 并照做，为你自己，也为项目里已有的每个智能体规则文件（CLAUDE.md、GEMINI.md、`.cursor/rules/`、`.github/copilot-instructions.md` 之类；这些只是例子）。共用规则移到 AGENTS.md；每个智能体自己的文件只保留指向 AGENTS.md 的一行和真正属于它自己的内容；互相矛盾的规则不要替用户选，列出来问用户；把每个智能体记在任务包 project-notes.md 的 `## Agents set up` 一节下。CLAUDE.md 要包含导入 AGENTS.md 的一行（半角 at 符号后接 AGENTS.md），让 Claude Code 读取 AGENTS.md（如果没有 CLAUDE.md，就新建一个只含这一行的文件）。不要把这一节复制到任何智能体自己的文件里。
6. 如果项目在版本控制下，用你的版本控制工具只保存 AGENTS.md、CLAUDE.md 和你改过的其他智能体文件，作为一次改动，用 Git 时：`git -C \"{project}\" add AGENTS.md CLAUDE.md <你改过的其他智能体文件> && git -C \"{project}\" commit -m \"Tell AI agents about Allomos\" -- AGENTS.md CLAUDE.md <你改过的其他智能体文件>`；再在任务包里只保存项目笔记，用 Git 时：`git -C \"{taskbag}\" add project-notes.md && git -C \"{taskbag}\" commit -m \"Agents set up\" -- project-notes.md`。其他改动保持原样；除非用户要求，不要共享（推送）。
7. 告诉用户你改了什么，以及你保存的改动（提交）。"
        ),
        "ja" => format!(
            "Allomos がこのプロジェクトのタスクバッグを用意しました。これからの AI エージェントにそれを伝えてください。

プロジェクトのフォルダー：{project}
タスクバッグ（Allomos のタスクファイル）：{taskbag}

プロジェクトのフォルダーで：

1. バージョン管理：Allomos 自身はバージョン管理ツールを一切実行しません。{no_repo}バージョン管理されていれば、まずタスクバッグ（記録されたプロジェクトのフォルダーを含む）だけをバージョン管理ツールで保存します。Git なら：`git -C \"{taskbag}\" add -- . && git -C \"{taskbag}\" commit -m \"Add Allomos taskbag\" -- .`（Allomos が `.allomos/` の隣に README-allomos.md を残していれば、両方のコマンドの最後に `../../README-allomos.md` を加えます。ほかのファイルは入れず、共有（プッシュ）もしません）。
2. AGENTS.md を開きます（なければ作成します）。
3. AGENTS.md にすでに Allomos の説明（たとえば「{heading}」という見出しの節や、Allomos または `alm` についてのほかの文）があれば、Allomos の節（最後の行を参照）で置き換えます。なければその節を追加します。
4. AGENTS.md にタスクや状況の管理についてのほかのルール（たとえば Markdown のタスクボード）があれば、ルールが食い違わないよう Allomos を指すように書き換えます。関係のない部分は変えず、同じ説明を二重に追加しないでください。
5. 共通ルールを AGENTS.md に置きます（エージェントごとに一度）：`alm --taskbag \"{taskbag}\" guide agent-files` を実行してそのとおりに行います。自分自身について、そしてプロジェクトにすでにあるすべてのエージェント用ルールファイル（CLAUDE.md、GEMINI.md、`.cursor/rules/`、`.github/copilot-instructions.md` など。これは例です）について。共通ルールは AGENTS.md に移し、各エージェント自身のファイルには AGENTS.md を指す行と本当にそのエージェントだけのことを残します。食い違うルールはどちらも選ばず、ユーザーに示して尋ねます。各エージェントをタスクバッグの project-notes.md の `## Agents set up` の節に記録します。CLAUDE.md には AGENTS.md を読み込む行（半角のアットマークに続けて AGENTS.md）があるようにします（Claude Code が AGENTS.md を読むためです。CLAUDE.md がなければその一行だけで作成します）。この節をどのエージェント自身のファイルにもコピーしないでください。
6. プロジェクトがバージョン管理されていれば、AGENTS.md、CLAUDE.md、変えたほかのエージェントのファイルだけを一つの変更としてバージョン管理ツールで保存します。Git なら：`git -C \"{project}\" add AGENTS.md CLAUDE.md <変えたほかのエージェントのファイル> && git -C \"{project}\" commit -m \"Tell AI agents about Allomos\" -- AGENTS.md CLAUDE.md <変えたほかのエージェントのファイル>`。続いてタスクバッグではプロジェクトノートだけを保存します。Git なら：`git -C \"{taskbag}\" add project-notes.md && git -C \"{taskbag}\" commit -m \"Agents set up\" -- project-notes.md`。ほかの変更はそのままにし、頼まれない限り共有（プッシュ）しません。
7. 何を変えたか、どんな変更を保存（コミット）したかをユーザーに見せてください。"
        ),
        _ => format!(
            "Allomos has set up this project's taskbag. Please now tell future AI agents about it.

Project folder: {project}
Taskbag (Allomos's task files): {taskbag}

In the project folder:

1. Version control: Allomos itself never runs your version control tool. {no_repo} Under version control, first save the taskbag \
on its own in your version control tool, with its changes so far (the recorded project folder too); \
for Git: `git -C \"{taskbag}\" add -- . && git -C \"{taskbag}\" commit -m \"Add Allomos taskbag\" -- .` \
(add `../../README-allomos.md` to the end of both commands when Allomos left that file beside \
`.allomos/`; never other files, and do not share (push) it).
2. Open AGENTS.md (create it if it is missing).
3. If AGENTS.md already has Allomos instructions (for example a section headed \
\"{heading}\", or any other text about Allomos or `alm`), replace them with \
Allomos's section (see the last line). Otherwise add that section.
4. If AGENTS.md has other rules about tracking tasks or status (for example a Markdown task \
board), change them to point to Allomos, so the rules do not conflict. Do not change \
unrelated text, and do not add the same instructions twice.
5. Shared rules in AGENTS.md, once per agent: run `alm --taskbag \"{taskbag}\" guide agent-files` \
and follow it, for yourself and for every agent rule file already in the project (CLAUDE.md, \
GEMINI.md, `.cursor/rules/`, `.github/copilot-instructions.md` and the like; these are examples). \
Shared rules move into AGENTS.md; each agent's own file keeps only a pointer to AGENTS.md and what \
is truly its own; rules that contradict each other are not chosen between but listed for the \
person, who decides; each agent is recorded under `## Agents set up` in the taskbag's \
project-notes.md. CLAUDE.md must have the line that imports AGENTS.md (a plain ASCII at sign \
followed by AGENTS.md), so Claude Code reads AGENTS.md (create CLAUDE.md with just that line if it \
is missing). Do not copy the section into any agent's own file.
6. If the project is under version control, save only AGENTS.md, CLAUDE.md and the other agent \
files you changed, as one change in your version control tool; for Git: `git -C \"{project}\" add AGENTS.md CLAUDE.md <other agent files you \
changed> && git -C \"{project}\" commit -m \"Tell AI agents about Allomos\" -- AGENTS.md CLAUDE.md \
<other agent files you changed>`; then, in the taskbag, only the project notes; for Git: \
`git -C \"{taskbag}\" add project-notes.md && git -C \"{taskbag}\" commit -m \"Agents set up\" -- \
project-notes.md`. Leave every other change as it is; do not share (push) unless the person asks.
7. Show the user what you changed and the changes (commits) you saved."
        ),
    }
}

/// Add Allomos's section for AI agents to `folder`'s `AGENTS.md` and `CLAUDE.md`, never
/// changing what is already there. `AGENTS.md`: created with the section, or the section is
/// appended unless it is there. `CLAUDE.md`: created as `@AGENTS.md` (Claude Code then reads
/// AGENTS.md); an existing one is left alone if it imports `@AGENTS.md` or holds the section,
/// otherwise the section is appended. Returns the names of the files it changed.
pub fn add_agent_instructions(folder: &Path) -> io::Result<Vec<String>> {
    let mut changed = Vec::new();
    let section = agent_section();
    let agents = folder.join("AGENTS.md");
    match fs::read_to_string(&agents) {
        Ok(text) if tells_agents(&text) => {}
        Ok(text) => {
            append(&agents, &text, &section)?;
            changed.push("AGENTS.md".to_string());
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            fs::write(&agents, format!("# Agent instructions\n\n{section}"))?;
            changed.push("AGENTS.md".to_string());
        }
        Err(e) => return Err(e),
    }
    let claude = folder.join("CLAUDE.md");
    match fs::read_to_string(&claude) {
        Ok(text) if tells_agents(&text) || imports_agents_md(&text) => {}
        Ok(text) => {
            append(&claude, &text, &section)?;
            changed.push("CLAUDE.md".to_string());
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            fs::write(&claude, "@AGENTS.md\n")?;
            changed.push("CLAUDE.md".to_string());
        }
        Err(e) => return Err(e),
    }
    Ok(changed)
}

/// Whether an instruction file's text tells AI agents about Allomos: a Markdown heading names
/// it, or the text mentions `alm next` or `alm claim`, or `.allomos/taskbag`. Agents may merge the
/// section in their own words, so any of these counts.
pub fn tells_agents(text: &str) -> bool {
    text.contains("alm next")
        || text.contains("alm claim")
        || text.contains(".allomos/taskbag")
        || text.lines().any(|l| {
            let l = l.trim_start();
            l.starts_with('#') && l.to_lowercase().contains("allomos")
        })
}

pub(crate) fn imports_agents_md(text: &str) -> bool {
    text.lines().any(|l| l.trim() == "@AGENTS.md")
}

/// Which AI agents' instruction files in `project` tell them about Allomos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AgentSetup {
    /// Codex reads `AGENTS.md`.
    pub codex: bool,
    /// Claude Code reads `CLAUDE.md`, and through `@AGENTS.md` also `AGENTS.md`.
    pub claude: bool,
}

/// Read `project`'s `AGENTS.md` and `CLAUDE.md` and say which agents are told about Allomos.
pub fn agent_setup(project: &Path) -> AgentSetup {
    let agents = fs::read_to_string(project.join("AGENTS.md")).unwrap_or_default();
    let claude = fs::read_to_string(project.join("CLAUDE.md")).unwrap_or_default();
    let codex = tells_agents(&agents);
    AgentSetup {
        codex,
        claude: tells_agents(&claude) || (imports_agents_md(&claude) && codex),
    }
}

/// Append `section` to a file whose current text is `text`, after one blank line.
pub(crate) fn append(path: &Path, text: &str, section: &str) -> io::Result<()> {
    let gap = if text.is_empty() || text.ends_with("\n\n") {
        ""
    } else if text.ends_with('\n') {
        "\n"
    } else {
        "\n\n"
    };
    let mut file = fs::OpenOptions::new().append(true).open(path)?;
    file.write_all(format!("{gap}{section}").as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp(name: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!("allomos-test-{name}-{n}"));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn a_sibling_taskbag_named_through_dot_dot_is_outside_the_project() {
        let project = PathBuf::from("/work/app");
        let taskbag = PathBuf::from("/work/app/../tasks/.allomos/taskbag");
        let section = agent_section_for(&project, &taskbag);
        assert!(
            section.contains("`--taskbag \"../tasks/.allomos/taskbag\"`"),
            "{section}"
        );
        assert_eq!(
            agent_section_for(&project, &project.join("./.allomos/taskbag")),
            agent_section()
        );
    }

    #[test]
    fn the_section_names_the_repositories_the_project_notes_list() {
        let base = tmp("brief-repos");
        let (project, taskbag) = (base.join("app"), base.join("tasks/.allomos/taskbag"));
        fs::create_dir_all(&project).unwrap();
        init(&taskbag, "p", "P").unwrap();
        // None listed yet: the section as before.
        let plain = agent_section_for(&project, &taskbag);
        assert!(!plain.contains("kept online"), "{plain}");
        fs::write(
            taskbag.join(handoff::NOTES_FILE),
            "## Repositories\n\n- Code: https://github.com/o/app\n- Taskbag: https://github.com/o/app-tasks\n",
        )
        .unwrap();
        let section = agent_section_for(&project, &taskbag);
        assert!(section.starts_with(&plain), "{section}");
        assert!(
            section.ends_with(
                "- Code: https://github.com/o/app\n- Taskbag: https://github.com/o/app-tasks\n"
            ),
            "{section}"
        );
        assert!(section.contains("in the cloud for example"), "{section}");
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn agent_setup_request_follows_the_language_but_keeps_the_section_english() {
        let (p, b) = (Path::new("/p"), Path::new("/p/.allomos/taskbag"));
        let zh = agent_setup_request_in("zh-CN", p, b);
        let ja = agent_setup_request_in("ja", p, b);
        // Folders as the request shows them: on Windows with `\` throughout.
        let (sp, sb) = (shown(p), shown(b));
        assert!(zh.contains("2. 打开 AGENTS.md") && zh.contains(&format!("项目文件夹：{sp}\n")));
        assert!(
            ja.contains("2. AGENTS.md を開きます")
                && ja.contains(&format!("タスクバッグ（Allomos のタスクファイル）：{sb}"))
        );
        for r in [&zh, &ja] {
            assert!(
                r.contains("Add Allomos taskbag") && r.contains("Tell AI agents about Allomos")
            );
            assert!(
                r.contains("brief`") && !r.contains("alm claim <id>"),
                "no copy of the section"
            );
        }
        assert_eq!(
            agent_setup_request_in("fr", p, b),
            agent_setup_request(p, b)
        );
    }

    #[test]
    fn agent_setup_request_carries_the_section_and_steps() {
        let (p, b) = (Path::new("/p"), Path::new("/p/.allomos/taskbag"));
        let r = agent_setup_request(p, b);
        assert!(r.contains("2. Open AGENTS.md") && r.contains("imports AGENTS.md"));
        // Each step commits its own work, limited to its own paths (Task-A03-08).
        assert!(r.contains("commit -m \"Add Allomos taskbag\" -- ."));
        assert!(r.contains("commit -m \"Tell AI agents about Allomos\" -- AGENTS.md CLAUDE.md"));
        // Step 5 is the one-time agent-files procedure, for every agent file already there, and
        // the project notes record each agent (Task-A03-24).
        for language in ["en", "zh-CN", "ja"] {
            let r = agent_setup_request_in(language, p, b);
            let step5 = r
                .split("\n5. ")
                .nth(1)
                .unwrap()
                .split("\n6. ")
                .next()
                .unwrap();
            assert!(
                step5.contains(&format!("alm --taskbag \"{}\" guide agent-files", shown(b))),
                "{language}: {step5}"
            );
            for words in [
                "GEMINI.md",
                "`.cursor/rules/`",
                "`## Agents set up`",
                "CLAUDE.md",
            ] {
                assert!(step5.contains(words), "{language}: {words}");
            }
            assert!(
                r.contains("commit -m \"Agents set up\" -- project-notes.md"),
                "{language}"
            );
            assert!(!r.contains('@'), "{language}");
        }
        // The Claude app turns a typed-in "@" into a full-width "＠", so the request avoids it.
        assert!(!r.contains('@'));
        assert!(
            r.contains(&format!("Project folder: {}\n", shown(p)))
                && r.contains(&format!("task files): {}\n", shown(b)))
        );
        assert!(r.contains("any other text about Allomos") && !r.contains("<!--"));
        assert!(
            r.contains("brief` prints"),
            "the section comes from `alm brief`"
        );
        assert!(!r.contains("alm claim <id>"), "no copy of the section");
        assert_eq!(r.matches(AGENT_SECTION_HEADING).count(), 1);
    }

    #[test]
    fn agent_setup_reads_the_instruction_files() {
        let dir = tmp("agent-setup");
        let none = AgentSetup {
            codex: false,
            claude: false,
        };
        assert_eq!(agent_setup(&dir), none);
        // Unrelated instructions do not count.
        fs::write(dir.join("AGENTS.md"), "# Rules\nRun the tests.\n").unwrap();
        assert_eq!(agent_setup(&dir), none);
        // Merged in other words: `alm next` counts; CLAUDE.md importing AGENTS.md follows it.
        fs::write(dir.join("AGENTS.md"), "## Workflow\nStart with alm next.\n").unwrap();
        fs::write(dir.join("CLAUDE.md"), "@AGENTS.md\n").unwrap();
        assert_eq!(
            agent_setup(&dir),
            AgentSetup {
                codex: true,
                claude: true
            }
        );
        // CLAUDE.md alone, mentioning the taskbag; AGENTS.md without it.
        fs::write(dir.join("AGENTS.md"), "# Rules\n").unwrap();
        fs::write(dir.join("CLAUDE.md"), "Tasks are in .allomos/taskbag/.\n").unwrap();
        assert_eq!(
            agent_setup(&dir),
            AgentSetup {
                codex: false,
                claude: true
            }
        );
        // A heading naming Allomos.
        fs::write(dir.join("AGENTS.md"), "### Using Allomos\n").unwrap();
        assert!(agent_setup(&dir).codex);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn agent_instructions_are_added_once_and_keep_existing_text() {
        // No files: both created; CLAUDE.md only imports AGENTS.md.
        let dir = tmp("agents-new");
        assert_eq!(
            add_agent_instructions(&dir).unwrap(),
            ["AGENTS.md", "CLAUDE.md"]
        );
        let agents = fs::read_to_string(dir.join("AGENTS.md")).unwrap();
        assert!(agents.contains(AGENT_SECTION_HEADING) && agents.contains("alm next"));
        assert!(!agents.contains("<!--"));
        assert_eq!(
            fs::read_to_string(dir.join("CLAUDE.md")).unwrap(),
            "@AGENTS.md\n"
        );
        // A second run changes nothing.
        assert!(add_agent_instructions(&dir).unwrap().is_empty());
        assert_eq!(fs::read_to_string(dir.join("AGENTS.md")).unwrap(), agents);
        fs::remove_dir_all(dir).unwrap();

        // Existing files without the section: it is appended once, earlier text kept.
        let dir = tmp("agents-old");
        fs::write(dir.join("AGENTS.md"), "# Ours\nKeep this.").unwrap();
        fs::write(dir.join("CLAUDE.md"), "Claude rules\n").unwrap();
        assert_eq!(
            add_agent_instructions(&dir).unwrap(),
            ["AGENTS.md", "CLAUDE.md"]
        );
        let agents = fs::read_to_string(dir.join("AGENTS.md")).unwrap();
        assert!(agents.starts_with("# Ours\nKeep this.\n\n## Tasks: Allomos"));
        let claude = fs::read_to_string(dir.join("CLAUDE.md")).unwrap();
        assert!(claude.starts_with("Claude rules\n\n## Tasks: Allomos"));
        assert_eq!(agents.matches(AGENT_SECTION_HEADING).count(), 1);
        assert!(add_agent_instructions(&dir).unwrap().is_empty());
        fs::remove_dir_all(dir).unwrap();

        // An Allomos section under another heading (no markers) is recognised too.
        let dir = tmp("agents-own-heading");
        fs::write(
            dir.join("AGENTS.md"),
            "# Ours\n\n## Using Allomos\nRun alm next.\n",
        )
        .unwrap();
        assert_eq!(add_agent_instructions(&dir).unwrap(), ["CLAUDE.md"]);
        fs::remove_dir_all(dir).unwrap();

        // CLAUDE.md that already imports AGENTS.md is left alone.
        let dir = tmp("agents-import");
        fs::write(dir.join("CLAUDE.md"), "# Claude\n@AGENTS.md\n").unwrap();
        assert_eq!(add_agent_instructions(&dir).unwrap(), ["AGENTS.md"]);
        assert_eq!(
            fs::read_to_string(dir.join("CLAUDE.md")).unwrap(),
            "# Claude\n@AGENTS.md\n"
        );
        fs::remove_dir_all(dir).unwrap();
    }
}
