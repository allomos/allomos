//! Reads and creates taskbags on disk.
//!
//! A *taskbag* is a folder holding `project.json`, `areas/<id>.json`, optionally
//! `iterations/<id>.json`, and `tasks/<id>/task.json` (+ `description.md`). See `docs/product/FILE-FORMAT.md`.
//! This crate never runs or reads Git (decision 0017): a taskbag inside a repository is
//! versioned by that repository, and the agent commits each change itself. [`with_written_files`]
//! tells which files a change wrote, so `alm` can print the Git commands for exactly those.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use fs4::{FileExt, TryLockError};

use allomos_core::branch;
use allomos_core::decision;
use allomos_core::event::{self, Event, Kind};
use allomos_core::heartbeat;
use allomos_core::ops;
use allomos_core::{
    Area, FORMAT, Issue, Iteration, Loaded, Problem, Project, State, Task, Taskbag, parse_area,
    parse_iteration, parse_project, parse_task, validate_taskbag,
};
use serde::Serialize;

pub mod approvals;
pub mod cadence;
pub mod cards;
pub mod checkpoint;
pub mod classify;
pub mod codex_rules;
mod confirm;
mod convert;
pub mod effects;
pub mod events;
pub mod exceptions;
pub mod folders;
mod guide;
pub mod handoff;
pub mod identity;
pub mod links;
pub mod missing;
pub mod progress;
mod quota;
pub mod redact;
mod settings;
mod setup;
mod signing;
pub mod trail;
mod translate;
pub mod tsa;
mod update_taskbag;
pub mod versions;
pub use confirm::{
    ConfirmRequest, Extra as ConfirmExtra, MAX_PLAN_BYTES, REQUESTS_DIR, answer_confirm,
    pending_confirms, post_confirm, read_answer, requests_dir, shown_fingerprint, was_seen,
    withdraw_confirm,
};
pub use convert::{
    CONNECT_REQUEST_VERSION, CONVERT_REQUEST_VERSION, MOVE_IN_REQUEST_VERSION,
    agent_connect_request_in, agent_connect_request_with, agent_convert_request_in,
    agent_move_in_request_in, agent_move_tasks_in_request_in,
};
pub use events::Actor;
pub use guide::{
    GOAL_PROMPT_VERSION, GUIDE_TOPICS, LOCALIZED_TOPICS, RUN_MODES_LANGUAGES, added_task_steps,
    agent_files, claimed_task_steps, discuss_request_in, github_release, goal, goal_request_in,
    guide, guide_topic, localized, run_modes,
};
pub use quota::{
    ASK_AFTER_MINUTES as QUOTA_ASK_AFTER_MINUTES, QuotaAsk, QuotaReport, WINDOWS as QUOTA_WINDOWS,
    agent_id as quota_agent_id, new_report as new_quota_report, quota_ask, quota_reports,
    save_quota,
};
pub use settings::{
    StartWith, TaskLanguage, UpdateMemory, alm_offer_dismissed_for, auto_check, auto_download,
    dismiss_alm_offer, lan_viewing, pick_ui_language, restore_update_check, save_auto_check,
    save_auto_download, save_lan_viewing, save_owner_name, save_start_with, save_ui_language,
    save_update_check, saved_owner_name, saved_ui_language, settings_dir, skip_update, start_with,
    task_language, update_memory,
};
pub use setup::{
    SetupState, allomos_folder, project_folder_of, project_folder_paths, record_project_folder,
    relative_path, setup_state,
};
pub use signing::{
    Replaced, ReplacedIn, ThisComputer, ThisKey, public_key_export, replace_key, this_computer,
};
pub use translate::{agent_translate_request_in, translation_steps_in};
pub use update_taskbag::{
    Target as TranslateTarget, agent_update_taskbag_request_in, agent_update_taskbag_request_with,
};

/// Where a project's taskbag lives inside the folder the user picked.
/// Always used for a new taskbag, also in an empty folder, so every tool looks in one place.
pub const DEFAULT_DIR: &str = ".allomos/taskbag";
const PROJECT_FILE: &str = "project.json";
/// The file every writer locks for the length of one change. Never committed.
pub const LOCK_FILE: &str = ".lock";
/// Prefix of the temporary files and folders a writer renames into place.
const TMP_PREFIX: &str = ".tmp-";
/// Written into every new taskbag, so Git never sees Allomos's working files.
const GITIGNORE: &str = "# Allomos's own working files. Never commit them.\n.lock\n.tmp-*\n";
/// Written into every new taskbag, so Git on Windows checks the files out with `\n` line endings,
/// as Allomos writes them (Task-A07-03). Reading accepts `\r\n` too.
const GITATTRIBUTES: &str = "# Allomos writes these files with \\n line endings; keep them so on every computer.\n*.json text eol=lf\n*.md text eol=lf\n.gitignore text eol=lf\n.gitattributes text eol=lf\n";

/// `<folder>/.allomos/taskbag`, built part by part so the path uses the system's own separator
/// (`C:\p\.allomos\taskbag` on Windows, not `C:\p\.allomos/taskbag`).
pub fn default_root(folder: &Path) -> PathBuf {
    DEFAULT_DIR
        .split('/')
        .fold(folder.to_path_buf(), |p, part| p.join(part))
}

/// `text` with every `\r\n` turned into `\n`: Git on Windows may check text files out with
/// `\r\n`, and Allomos always writes `\n` (Task-A07-03).
pub fn lf(text: &str) -> String {
    text.replace("\r\n", "\n")
}

/// Read a text file, with `\r\n` line endings turned into `\n`.
pub fn read_text(path: &Path) -> io::Result<String> {
    fs::read_to_string(path).map(|t| lf(&t))
}

/// A path as the system's own programs want it: on Windows with `\` throughout and without the
/// `\\?\` prefix that [`Path::canonicalize`] adds (Explorer does not understand it). Elsewhere
/// the path as it is.
pub fn plain_path(path: &Path) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(plain_windows(&path.to_string_lossy()))
    } else {
        path.to_path_buf()
    }
}

/// [`plain_path`]'s text rule, kept apart so it is tested on every system: `/` becomes `\`,
/// `\\?\C:\x` becomes `C:\x` and `\\?\UNC\server\share` becomes `\\server\share`.
fn plain_windows(path: &str) -> String {
    let path = path.replace('/', "\\");
    if let Some(rest) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = path.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        path
    }
}

/// A path written with `/` between its parts, for files shared between computers (a
/// relative path recorded on Windows as `..\app` is stored as `../app`).
pub fn portable_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// What a folder the user picked turns out to be.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Inspection {
    /// The folder is a taskbag, or contains one in `.allomos/taskbag/`.
    Found { root: PathBuf },
    /// The folder looks empty (nothing, or only hidden entries such as `.DS_Store` or `.git`):
    /// a starter taskbag can be created in its `.allomos/taskbag/` without asking.
    Empty {
        folder: PathBuf,
        suggested_root: PathBuf,
    },
    /// The folder has other content but no taskbag: one can be created in `.allomos/taskbag/`.
    NoTaskbag {
        folder: PathBuf,
        suggested_root: PathBuf,
    },
    /// The path does not exist or is not a folder.
    NotAFolder { path: PathBuf },
}

/// Name of the short introduction Allomos leaves in a folder it fills from empty.
/// Never `README.md`, so it cannot clash with the user's own README (owner, 2026-10-06).
pub const README_FILE: &str = "README-allomos.md";

/// A folder looks empty when it has no entries, or only hidden ones (names starting with
/// `.`, such as `.DS_Store`, `.git` or `.allomos`). A folder that cannot be read does not.
pub fn looks_empty(folder: &Path) -> bool {
    fs::read_dir(folder)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .all(|e| e.file_name().to_string_lossy().starts_with('.'))
        })
        .unwrap_or(false)
}

/// The README's text in an interface language (`en`, `zh-CN`, `ja`); English otherwise.
pub fn readme_text(language: &str) -> &'static str {
    match language {
        "zh-CN" => README_ZH_CN,
        "ja" => README_JA,
        _ => README_EN,
    }
}

const README_EN: &str = "# Allomos project

This folder is an Allomos project. Its tasks, areas and progress are in
`.allomos/taskbag/`, a hidden folder.

- Open this folder in the Allomos app to see the tasks by area.
- AI agents and people can also work the tasks from a terminal with the `alm` command
  (`alm list`, `alm next`, `alm claim <task>`).
- The files are plain JSON and Markdown, so a version control tool (Git, for example) can track every change.

Allomos wrote this file because the folder was empty. You may edit or delete it.
";

const README_ZH_CN: &str = "# 换肩项目

此文件夹是一个「换肩」项目。它的任务、领域和进度保存在隐藏文件夹
`.allomos/taskbag/` 中。

- 在「换肩」应用中打开此文件夹，即可按领域查看任务。
- AI 智能体和人也可以在终端中用 `alm` 命令处理任务
  （`alm list`、`alm next`、`alm claim <任务>`）。
- 这些文件是普通的 JSON 和 Markdown，版本控制工具（例如 Git）可以记录每一处更改。

此文件夹原本是空的，所以「换肩」写了这个文件。你可以修改或删除它。
";

const README_JA: &str = "# 肩代プロジェクト

このフォルダーは「肩代」のプロジェクトです。タスク、エリア、進捗は隠しフォルダー
`.allomos/taskbag/` に入っています。

- 「肩代」アプリでこのフォルダーを開くと、タスクをエリアごとに見られます。
- AI エージェントや人は、ターミナルの `alm` コマンドでもタスクを扱えます
  （`alm list`、`alm next`、`alm claim <タスク>`）。
- ファイルはただの JSON と Markdown なので、バージョン管理ツール（たとえば Git）ですべての変更を記録できます。

このフォルダーが空だったので、「肩代」がこのファイルを書きました。編集や削除は自由です。
";

/// Write `README-allomos.md` into `folder` when the folder looks empty (see
/// [`looks_empty`]). Never overwrites a file. Returns whether the README was written.
pub fn add_readme_if_empty(folder: &Path, language: &str) -> io::Result<bool> {
    if !looks_empty(folder) {
        return Ok(false);
    }
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(folder.join(README_FILE))
    {
        Ok(mut file) => {
            file.write_all(readme_text(language).as_bytes())?;
            Ok(true)
        }
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(false),
        Err(e) => Err(e),
    }
}

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
- Always work a task in a worktree of its own (a separate working copy on the task's own
  branch; in Git: `git worktree add <folder> -b <working branch> <base>`, which the claim
  prints), never in the main checkout: the main checkout stays on the base branch. Merge from
  the main checkout; once merged, remove your worktree and the merged branch.
- While you hold a task (decision 0022): pass `--as <your session name>` to every `alm` command.
  Send a heartbeat at least every 10 minutes (the project's interval, `alm heartbeat-settings`):
  `alm beat --as <your session name>` (every task you hold, in one change), and save and share it
  like any change: a heartbeat counts only once shared. A change you make to your task beats
  along with it; reading never does. Before a long step that runs no `alm` command (a long build
  or test run), declare it: `alm beat <id> --busy <length>`, as long as you judge it needs
  (`--busy 0m` ends it early). A held task silent past the project's limit (30 minutes by
  default) with no busy window is lost: `alm next` lists it under \"Lost, can be taken over\" with
  the command. Take one over only to continue its work: claiming it is the takeover; go on on its
  working branch, and look at the former holder's worktree but change nothing there (that folder
  is of use only on its own computer; on another one, go by the working branch and the handoff
  note: only what was pushed counts). Use `--take`
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
- Stopped: waiting on a person: `alm wait <id> --on \"<what>\" [--who \"<who>\"]`; blocked by
  another task: `alm block <id> --by <task-id>`. When its last blocker finishes (done or
  dropped), it goes on by itself; read its blockers' AI decisions before you start it. To let it
  go on earlier: `alm unblock <id>`.
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
  task).
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
"
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

fn imports_agents_md(text: &str) -> bool {
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

/// Where a link in a task's description leads, once Allomos has agreed to open it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkTarget {
    /// An `http` or `https` address, for the web browser.
    Web(String),
    /// A text file inside the project, for the user's editor.
    File(PathBuf),
}

/// File types a description link may open: text the user reads or edits, never a program.
const LINKABLE: &[&str] = &["md", "markdown", "txt"];

/// Check a link from task `task`'s `description.md` in the taskbag at `root`. Web addresses
/// (`http`, `https`) are allowed. A relative path is read from the task's folder, the way a
/// Markdown viewer would, and must lead to an existing `.md`, `.markdown` or `.txt` file inside
/// the project: the folder holding `.allomos/` (or the taskbag itself when it stands alone), or
/// the code folder recorded as `project_folder` in project.json, for a taskbag kept beside the
/// code it tracks. Everything else is refused, so a description cannot be used to start a
/// program.
pub fn resolve_task_link(root: &Path, task: &str, link: &str) -> Result<LinkTarget, Problem> {
    let refused = || Problem::LinkRefused {
        link: link.to_string(),
    };
    let lower = link.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return Ok(LinkTarget::Web(link.to_string()));
    }
    // Any other scheme (`file:`, `mailto:`, `javascript:` …) and absolute paths are refused.
    if link.is_empty() || link.contains(':') || link.starts_with('/') || link.starts_with('\\') {
        return Err(refused());
    }
    if !allomos_core::ops::valid_id(task) {
        return Err(refused());
    }
    // A link may point into a file (`card.md#approvals`); the part after `#` is not a file name.
    let path = link.split('#').next().unwrap_or_default();
    let target = root.join("tasks").join(task).join(path);
    let Ok(target) = target.canonicalize() else {
        return Err(Problem::LinkNotFound {
            link: link.to_string(),
        });
    };
    let recorded = fs::read_to_string(root.join(PROJECT_FILE))
        .ok()
        .and_then(|text| parse_project(PROJECT_FILE, &text).ok())
        .and_then(|project| project_folder_of(root, &project));
    let inside = [
        Some(picked_folder(root).unwrap_or(root).to_path_buf()),
        recorded,
    ]
    .into_iter()
    .flatten()
    .filter_map(|folder| folder.canonicalize().ok())
    .any(|folder| target.starts_with(folder));
    let text = target
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| LINKABLE.contains(&e.to_ascii_lowercase().as_str()));
    if !inside || !target.is_file() || !text {
        return Err(refused());
    }
    Ok(LinkTarget::File(plain_path(&target)))
}

/// The folder the user picked, for a taskbag at `<folder>/.allomos/taskbag`; `None` for a
/// taskbag anywhere else.
pub fn picked_folder(root: &Path) -> Option<&Path> {
    if root.ends_with(DEFAULT_DIR) {
        root.parent()?.parent()
    } else {
        None
    }
}

/// Work out what `folder` is, without changing anything.
pub fn inspect(folder: &Path) -> Inspection {
    if !folder.is_dir() {
        return Inspection::NotAFolder {
            path: folder.to_path_buf(),
        };
    }
    if folder.join(PROJECT_FILE).is_file() {
        return Inspection::Found {
            root: folder.to_path_buf(),
        };
    }
    let nested = default_root(folder);
    if nested.join(PROJECT_FILE).is_file() {
        return Inspection::Found { root: nested };
    }
    if looks_empty(folder) {
        Inspection::Empty {
            folder: folder.to_path_buf(),
            suggested_root: nested,
        }
    } else {
        Inspection::NoTaskbag {
            folder: folder.to_path_buf(),
            suggested_root: nested,
        }
    }
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn read(root: &Path, path: &Path, issues: &mut Vec<Issue>) -> Option<String> {
    match read_text(path) {
        Ok(text) => Some(text),
        Err(e) => {
            issues.push(Issue::new(
                &rel(root, path),
                None,
                Problem::ReadFailed {
                    detail: e.to_string(),
                },
            ));
            None
        }
    }
}

fn hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with('.'))
}

/// Sorted entries of a folder, without hidden ones (the lock file and a writer's
/// temporary files); a missing folder is simply empty.
fn sorted_entries(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(dir)
        .map(|it| {
            it.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| !hidden(p))
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// Load a taskbag. Valid parts are returned even when other files have problems,
/// so the user sees their tasks plus a list of what to fix.
pub fn load(root: &Path) -> Loaded {
    let mut issues = Vec::new();
    let project_path = root.join(PROJECT_FILE);
    let project = match read(root, &project_path, &mut issues) {
        Some(text) => match parse_project(PROJECT_FILE, &text) {
            Ok(p) => Some(p),
            Err(mut e) => {
                issues.append(&mut e);
                None
            }
        },
        None => None,
    };
    let Some(project) = project else {
        return Loaded {
            taskbag: None,
            issues,
            active_iteration: None,
        };
    };

    let mut areas: Vec<(String, Area)> = Vec::new();
    for path in sorted_entries(&root.join("areas")) {
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let file = rel(root, &path);
        if let Some(text) = read(root, &path, &mut issues) {
            match parse_area(&file, &text) {
                Ok(a) => areas.push((file, a)),
                Err(mut e) => issues.append(&mut e),
            }
        }
    }

    // Optional: a taskbag without an `iterations/` folder simply has none.
    let mut iterations: Vec<(String, Iteration)> = Vec::new();
    for path in sorted_entries(&root.join("iterations")) {
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let file = rel(root, &path);
        if let Some(text) = read(root, &path, &mut issues) {
            match parse_iteration(&file, &text) {
                Ok(i) => iterations.push((file, i)),
                Err(mut e) => issues.append(&mut e),
            }
        }
    }

    let mut tasks: Vec<(String, Task)> = Vec::new();
    for dir in sorted_entries(&root.join("tasks")) {
        if !dir.is_dir() {
            continue;
        }
        let path = dir.join("task.json");
        let file = rel(root, &path);
        if !path.is_file() {
            issues.push(Issue::new(&file, None, Problem::TaskFileMissing));
            continue;
        }
        if let Some(text) = read(root, &path, &mut issues) {
            // Two changes that met (Task-A01-lawn320): said as such, not as broken JSON.
            if allomos_core::conflict::has_markers(&text) {
                issues.push(Issue::new(&file, None, Problem::MergeConflict));
                continue;
            }
            match parse_task(&file, &text) {
                Ok(mut t) => {
                    t.description = read_text(&dir.join("description.md")).unwrap_or_default();
                    tasks.push((file, t));
                }
                Err(mut e) => issues.append(&mut e),
            }
        }
    }

    issues.extend(validate_taskbag(&project, &areas, &iterations, &tasks));
    // Labels the tasks carry that the table does not list yet are shown as table entries; they
    // are written with the next change to `project.json` (decision 0020, 5).
    let mut project = project;
    for (_, t) in &tasks {
        allomos_core::labels::add_found(&mut project, &t.labels);
    }
    let mut areas: Vec<Area> = areas.into_iter().map(|(_, a)| a).collect();
    areas.sort_by(|a, b| a.order.cmp(&b.order).then(a.id.cmp(&b.id)));
    let mut iterations: Vec<Iteration> = iterations.into_iter().map(|(_, i)| i).collect();
    iterations.sort_by(|a, b| a.order.cmp(&b.order).then(a.id.cmp(&b.id)));
    let mut tasks: Vec<Task> = tasks.into_iter().map(|(_, t)| t).collect();
    // Numbered IDs first as before, then word IDs in the order they were made (Task-A01-54). The
    // history is read only when there is a word ID.
    if tasks.iter().any(|t| !allomos_core::ids::numbered(&t.id)) {
        let made = created(root);
        tasks.sort_by(|a, b| allomos_core::ids::compare(&a.id, &b.id, &made));
    }
    let taskbag = Taskbag {
        project,
        areas,
        iterations,
        tasks,
    };
    let active_iteration = allomos_core::ops::active_iteration(&taskbag).map(|i| i.id.clone());
    Loaded {
        taskbag: Some(taskbag),
        issues,
        active_iteration,
    }
}

/// The tasks whose `task.json` holds two claims that met (Task-A01-lawn320), in ID order. `me`:
/// the `--as` session. Read without a lock, like everything else; nothing is written.
pub fn claim_races(root: &Path, me: Option<&str>) -> Vec<allomos_core::conflict::ClaimRace> {
    use allomos_core::conflict;
    let mut out = Vec::new();
    for dir in sorted_entries(&root.join("tasks")) {
        let path = dir.join("task.json");
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let Some((one, two)) = conflict::sides(&text) else {
            continue;
        };
        let file = rel(root, &path);
        if let (Ok(one), Ok(two)) = (parse_task(&file, &one), parse_task(&file, &two)) {
            out.extend(conflict::ClaimRace::of(&one, &two, me));
        }
    }
    out
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut text = serde_json::to_string_pretty(value).map_err(io::Error::other)?;
    text.push('\n');
    fs::write(path, text)
}

/// The JSON stored in `task.json`: the task without its description (that lives in
/// `description.md`) and without empty optional fields.
fn task_file_json(task: &Task) -> serde_json::Value {
    let mut v = serde_json::to_value(task).expect("a task always serializes");
    if let Some(map) = v.as_object_mut() {
        map.remove("description");
        map.retain(|_, value| !value.is_null());
    }
    v
}

/// Create a small starter project at `root`. `root` must not exist yet, or be an empty folder.
/// Returns `root`.
pub fn create_starter(root: &Path, name: &str) -> io::Result<PathBuf> {
    if root.exists() && fs::read_dir(root)?.next().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} is not empty", root.display()),
        ));
    }
    fs::create_dir_all(root)?;
    fs::write(root.join(".gitignore"), GITIGNORE)?;
    fs::write(root.join(".gitattributes"), GITATTRIBUTES)?;
    let project = Project {
        project_folders: Default::default(),
        extra: Default::default(),
        format: FORMAT.to_string(),
        id: "project".to_string(),
        name: name.to_string(),
        labels: allomos_core::default_labels(),
        project_folder: None,
        task_language: None,
        handoff: None,
        audit: None,
        heartbeat: None,
    };
    write_json(&root.join(PROJECT_FILE), &project)?;
    let areas = [("A01", "Getting started", 1), ("A02", "First feature", 2)];
    for (id, name, order) in areas {
        write_json(
            &root.join("areas").join(format!("{id}.json")),
            &Area {
                extra: Default::default(),
                id: id.into(),
                name: name.into(),
                order,
            },
        )?;
    }
    let tasks = [
        (
            "A01-01",
            "Open this taskbag in Allomos",
            "A01",
            allomos_core::State::Done,
            Some("human verified"),
            "You picked a folder and Allomos created this starter project in it.\n",
        ),
        (
            "A01-02",
            "Rename the areas and tasks to your project's",
            "A01",
            allomos_core::State::Ready,
            None,
            "Edit the files in this folder, or use the `alm` command once it is available.\n",
        ),
        (
            "A02-01",
            "Describe the first outcome you want",
            "A02",
            allomos_core::State::Backlog,
            None,
            "One sentence about what a person can do when this task is done.\n",
        ),
    ];
    for (id, title, area, state, label, description) in tasks {
        let dir = root.join("tasks").join(id);
        let task = Task {
            folders: Default::default(),
            session: None,
            heartbeat: None,
            handover_request: None,
            extra: Default::default(),
            id: id.into(),
            title: title.into(),
            area: area.into(),
            state,
            labels: label.map(String::from).into_iter().collect(),
            holder: None,
            waiting_on: None,
            who: None,
            blocked_by: Vec::new(),
            next_dependency: None,
            iteration: None,
            difficulty: None,
            handoff_note: None,
            without_human_check: None,
            human_check_steps: None,
            description: String::new(),
        };
        write_json(&dir.join("task.json"), &task_file_json(&task))?;
        fs::write(dir.join("description.md"), description)?;
    }
    Ok(root.to_path_buf())
}

/// Find the taskbag for `start`: the nearest `.allomos/taskbag/` at or above it,
/// the way Git finds its repository.
pub fn find_taskbag(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .map(default_root)
        .find(|p| p.join(PROJECT_FILE).is_file())
}

/// A project ID made from a folder name: letters, digits, `-` and `_` kept, anything else
/// turned into `-`, at most 64 characters; `project` when nothing usable is left.
pub fn id_from_name(name: &str) -> String {
    let id: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .take(64)
        .collect();
    let id = id.trim_matches('-').to_string();
    if id.is_empty() {
        "project".to_string()
    } else {
        id
    }
}

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

thread_local! {
    /// The files written or removed by the change running on this thread, while
    /// [`with_written_files`] is listening.
    static WRITTEN: RefCell<Option<BTreeSet<PathBuf>>> = const { RefCell::new(None) };
}

/// Note that `path` was written or removed, when someone is listening.
pub(crate) fn note_written(path: &Path) {
    WRITTEN.with(|w| {
        if let Some(set) = w.borrow_mut().as_mut() {
            set.insert(path.to_path_buf());
        }
    });
}

/// Whether [`with_written_files`] is listening on this thread.
fn listening() -> bool {
    WRITTEN.with(|w| w.borrow().is_some())
}

/// Run `f`, and return with its result every file it wrote or removed through this crate
/// (decision 0017): a file rewritten with the same text is not counted. `alm` prints the Git
/// commands for exactly these files; nothing here runs Git.
pub fn with_written_files<T>(f: impl FnOnce() -> T) -> (T, Vec<PathBuf>) {
    let outer = WRITTEN.with(|w| w.borrow_mut().replace(BTreeSet::new()));
    let out = f();
    let mine = WRITTEN.with(|w| {
        let mut slot = w.borrow_mut();
        let mine = slot.take().unwrap_or_default();
        // Listening inside another listener: the outer one sees these files too.
        *slot = outer.map(|mut o| {
            o.extend(mine.iter().cloned());
            o
        });
        mine
    });
    (out, mine.into_iter().collect())
}

/// A name for a temporary file or folder next to `dir`'s contents, unique to this process.
fn tmp_path(dir: &Path, what: &str) -> PathBuf {
    let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    dir.join(format!("{TMP_PREFIX}{}-{n}-{what}", std::process::id()))
}

fn write_failed(path: &Path, e: impl std::fmt::Display) -> Problem {
    Problem::WriteFailed {
        path: path.display().to_string(),
        detail: e.to_string(),
    }
}

fn json_text<T: Serialize>(value: &T) -> String {
    let mut text = serde_json::to_string_pretty(value).expect("taskbag values always serialize");
    text.push('\n');
    text
}

/// How long replacing a file keeps trying while another program has it open (Windows).
const REPLACE_RETRY: Duration = Duration::from_millis(1000);
const REPLACE_PAUSE: Duration = Duration::from_millis(50);

/// Whether `e` may be a file that another program has open, which Windows reports as "access
/// denied" (5) or a sharing or lock violation (32, 33) until that program lets go.
fn may_be_in_use(e: &io::Error) -> bool {
    e.kind() == io::ErrorKind::PermissionDenied
        || (cfg!(windows) && matches!(e.raw_os_error(), Some(5 | 32 | 33)))
}

/// Run `attempt` until it works, a failure is not [`may_be_in_use`], or `limit` has passed,
/// pausing `pause` in between. A writer holds the taskbag's lock meanwhile, so the wait stays
/// well within the 5 seconds another writer waits for the lock.
fn retry_in_use<T>(
    limit: Duration,
    pause: Duration,
    mut attempt: impl FnMut() -> io::Result<T>,
) -> io::Result<T> {
    let start = Instant::now();
    loop {
        match attempt() {
            Err(e) if may_be_in_use(&e) && start.elapsed() < limit => std::thread::sleep(pause),
            result => return result,
        }
    }
}

/// Rename `from` over `to`. On Windows this fails while another program (an editor, a virus
/// scanner, Git) has `to` open without sharing it, so it is tried again for up to a second.
pub(crate) fn replace_file(from: &Path, to: &Path) -> io::Result<()> {
    retry_in_use(REPLACE_RETRY, REPLACE_PAUSE, || fs::rename(from, to))
}

/// Write `path` whole: the text goes to a temporary file in the same folder, is flushed to
/// disk, and is then renamed over `path`. A reader sees either the old file or the new one,
/// never a part-written one. Line endings are always written as `\n`.
fn write_atomic(path: &Path, text: &str) -> Result<(), Problem> {
    if listening() && fs::read_to_string(path).is_ok_and(|old| old == lf(text)) {
        // The same text again: the file is rewritten, but nothing changed to commit.
    } else {
        note_written(path);
    }
    let dir = path.parent().expect("taskbag files are inside a folder");
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let tmp = tmp_path(dir, name);
    let result = (|| {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(lf(text).as_bytes())?;
        f.sync_all()?;
        drop(f);
        replace_file(&tmp, path)
    })();
    result.map_err(|e| {
        let _ = fs::remove_file(&tmp);
        write_failed(path, e)
    })
}

/// Fill a new folder with files, then rename it to `target` in one step, so a reader never
/// sees it half-made. Refused when `target` already exists.
fn create_dir_atomic(target: &Path, files: &[(&str, String)]) -> Result<(), Problem> {
    let parent = target.parent().expect("a target has a parent");
    let name = target.file_name().and_then(|n| n.to_str()).unwrap_or("dir");
    let tmp = tmp_path(parent, name);
    let result = (|| {
        fs::create_dir_all(&tmp)?;
        for (rel_path, text) in files {
            let path = tmp.join(rel_path);
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir)?;
            }
            let mut f = fs::File::create(&path)?;
            f.write_all(lf(text).as_bytes())?;
            f.sync_all()?;
        }
        if target.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "already exists",
            ));
        }
        fs::rename(&tmp, target)
    })();
    if result.is_ok() {
        for (rel_path, _) in files {
            note_written(&target.join(rel_path));
        }
    }
    result.map_err(|e| {
        let _ = fs::remove_dir_all(&tmp);
        write_failed(target, e)
    })
}

/// Create an empty taskbag at `root` (normally `<repository>/.allomos/taskbag/`): `project.json`,
/// empty `areas/` and `tasks/`, a `.gitignore` for the lock and a `.gitattributes` keeping `\n`
/// line endings. Refused when `root` exists.
/// Never creates a Git repository.
/// This is how the app creates one, so its first event says `via: app`.
pub fn init(root: &Path, id: &str, name: &str) -> Result<PathBuf, Problem> {
    init_with_labels(root, id, name, Vec::new(), Actor::APP)
}

/// [`init`] with the project's own labels besides the built-in ones (decision 0020), made by
/// `actor`. The taskbag's first event records its creation, which is when its history starts.
pub fn init_with_labels(
    root: &Path,
    id: &str,
    name: &str,
    labels: Vec<String>,
    actor: Actor,
) -> Result<PathBuf, Problem> {
    if root.exists() {
        return Err(Problem::TaskbagExists {
            path: root.display().to_string(),
        });
    }
    let project = Project {
        project_folders: Default::default(),
        extra: Default::default(),
        format: FORMAT.to_string(),
        id: id.to_string(),
        name: name.to_string(),
        labels: allomos_core::labels::with_built_ins(
            labels
                .iter()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .map(|l| allomos_core::LabelEntry::custom(l, None))
                .collect(),
        ),
        project_folder: None,
        // New taskbags follow the app's language until someone sets another (Task-A01-21).
        task_language: Some(allomos_core::TASK_LANGUAGE_UI.to_string()),
        handoff: None,
        audit: None,
        heartbeat: None,
    };
    // Checked like any project file, so `init` never writes one that `load` refuses.
    let text = json_text(&project);
    if let Err(issues) = parse_project(PROJECT_FILE, &text) {
        return Err(issues[0].problem.clone());
    }
    let first = actor.event(
        "init",
        Kind::Project,
        Some(id),
        event::changes(None, &project, &[]),
    );
    let parent = root.parent().expect("a taskbag root has a parent");
    fs::create_dir_all(parent).map_err(|e| write_failed(parent, e))?;
    let first_file = events::new_file(root);
    create_dir_atomic(
        root,
        &[
            (PROJECT_FILE, text),
            (".gitignore", GITIGNORE.to_string()),
            (".gitattributes", GITATTRIBUTES.to_string()),
            ("areas/.keep", String::new()),
            ("tasks/.keep", String::new()),
            (handoff::NOTES_FILE, handoff::DEFAULT_NOTES.to_string()),
            (&first_file, json_text(&first)),
        ],
    )
    .map_err(|p| match p {
        Problem::WriteFailed { .. } if root.exists() => Problem::TaskbagExists {
            path: root.display().to_string(),
        },
        p => p,
    })?;
    // An `allomos/8` taskbag's history is signed from its first event (Task-A01-gusto404).
    if signing::wanted(root, &first) {
        let mut value = serde_json::to_value(&first).expect("events serialize");
        let new_key = signing::sign_event(root, &mut value, &first_file);
        let mut first = first;
        if let Some(sig) = value.get("sig") {
            first.extra.insert("sig".into(), sig.clone());
        }
        write_atomic(&root.join(&first_file), &json_text(&first))?;
        if let Some(key) = new_key {
            signing::add_key(root, Some(id), &key, &value)?;
        }
    }
    Ok(root.to_path_buf())
}

/// The taskbag's lock, held for one change: load, check the rules, write, then drop it.
/// Every write goes through a `Lock`, so two Allomos writers never change the taskbag at
/// the same moment. The operating system releases the lock if the process dies.
pub struct Lock {
    root: PathBuf,
    _file: fs::File,
}

impl Lock {
    /// Pack the history of whole earlier months from the 28th of a month (Task-A01-59):
    /// [`events::pack`].
    pub fn pack_history(&self) -> Result<Vec<events::Packed>, Problem> {
        events::pack(&self.root)
    }

    /// Take the lock, waiting up to `wait` for another writer to finish.
    pub fn acquire(root: &Path, wait: Duration) -> Result<Lock, Problem> {
        if !root.join(PROJECT_FILE).is_file() {
            return Err(Problem::TaskbagNotFound {
                path: root.display().to_string(),
            });
        }
        let path = root.join(LOCK_FILE);
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .map_err(|e| write_failed(&path, e))?;
        let start = Instant::now();
        loop {
            match FileExt::try_lock(&file) {
                Ok(()) => {
                    return Ok(Lock {
                        root: root.to_path_buf(),
                        _file: file,
                    });
                }
                Err(TryLockError::WouldBlock) if start.elapsed() < wait => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(TryLockError::WouldBlock) => {
                    return Err(Problem::TaskbagBusy {
                        path: root.display().to_string(),
                    });
                }
                Err(TryLockError::Error(e)) => return Err(write_failed(&path, e)),
            }
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Load the taskbag while holding the lock, so what is read is what the change builds on.
    pub fn load(&self) -> Loaded {
        load(&self.root)
    }

    /// Write a new area file. Refused when the file already exists.
    pub fn create_area(&self, area: &Area) -> Result<(), Problem> {
        let dir = self.root.join("areas");
        fs::create_dir_all(&dir).map_err(|e| write_failed(&dir, e))?;
        let path = dir.join(format!("{}.json", area.id));
        if path.exists() {
            return Err(Problem::DuplicateAreaId {
                id: area.id.clone(),
            });
        }
        write_atomic(&path, &json_text(area))
    }

    /// Write a new iteration file. Refused when the file already exists.
    pub fn create_iteration(&self, iteration: &Iteration) -> Result<(), Problem> {
        let dir = self.root.join("iterations");
        fs::create_dir_all(&dir).map_err(|e| write_failed(&dir, e))?;
        let path = dir.join(format!("{}.json", iteration.id));
        if path.exists() {
            return Err(Problem::DuplicateIterationId {
                id: iteration.id.clone(),
            });
        }
        write_atomic(&path, &json_text(iteration))
    }

    /// Write a new task folder: `task.json`, plus `description.md` when there is a description.
    pub fn create_task(&self, task: &Task) -> Result<(), Problem> {
        let dir = self.root.join("tasks");
        fs::create_dir_all(&dir).map_err(|e| write_failed(&dir, e))?;
        let target = dir.join(&task.id);
        if target.exists() {
            return Err(Problem::DuplicateTaskId {
                id: task.id.clone(),
            });
        }
        let mut files = vec![("task.json", json_text(&task_file_json(task)))];
        if !task.description.is_empty() {
            files.push(("description.md", task.description.clone()));
        }
        create_dir_atomic(&target, &files)
    }

    /// Rewrite an existing area's file (`alm rename area`, Task-A01-29).
    pub fn save_area(&self, area: &Area) -> Result<(), Problem> {
        let path = self.root.join("areas").join(format!("{}.json", area.id));
        write_atomic(&path, &json_text(area))
    }

    /// Rewrite an existing iteration's file (`alm change iteration`, Task-A01-29).
    pub fn save_iteration(&self, iteration: &Iteration) -> Result<(), Problem> {
        let path = self
            .root
            .join("iterations")
            .join(format!("{}.json", iteration.id));
        write_atomic(&path, &json_text(iteration))
    }

    /// Remove an area's or an iteration's file (`alm delete area|iteration`, Task-A01-29).
    /// `folder` is `areas` or `iterations`.
    pub fn remove_item(&self, folder: &str, id: &str) -> Result<(), Problem> {
        let path = self.root.join(folder).join(format!("{id}.json"));
        fs::remove_file(&path).map_err(|e| write_failed(&path, e))?;
        note_written(&path);
        Ok(())
    }

    /// Rewrite `project.json` (`alm labels`).
    pub fn save_project(&self, project: &Project) -> Result<(), Problem> {
        write_atomic(&self.root.join(PROJECT_FILE), &json_text(project))
    }

    /// Rewrite an existing task's `task.json`. Its description is left as it is.
    pub fn save_task(&self, task: &Task) -> Result<(), Problem> {
        let path = self.root.join("tasks").join(&task.id).join("task.json");
        write_atomic(&path, &json_text(&task_file_json(task)))
    }

    /// Rewrite an existing task's `description.md`, or remove it when the description is empty
    /// (the app's task pane, Task-A01-18).
    pub fn save_description(&self, task: &Task) -> Result<(), Problem> {
        let path = self
            .root
            .join("tasks")
            .join(&task.id)
            .join("description.md");
        if task.description.is_empty() {
            match fs::remove_file(&path) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => Err(write_failed(&path, e)),
                Err(_) => Ok(()),
                Ok(()) => {
                    note_written(&path);
                    Ok(())
                }
            }
        } else {
            write_atomic(&path, &task.description)
        }
    }

    /// Make one change and record it (Task-A01-17): write `event`, then run `write`. When `write`
    /// fails the event is taken back, so the history never shows a change that did not happen.
    /// An event with no changes is not written: nothing happened worth recording.
    pub fn record(
        &self,
        event: &Event,
        write: impl FnOnce(&Lock) -> Result<(), Problem>,
    ) -> Result<(), Problem> {
        if event.changes.is_empty() {
            return write(self);
        }
        let file = events::write(&self.root, event)?;
        write(self).inspect_err(|_| {
            let _ = fs::remove_file(&file);
        })
    }
}

/// A task as changed by [`change_task`], with what is worth telling the person.
#[derive(Debug, Clone, PartialEq)]
pub struct Changed {
    pub task: Task,
    /// Things worth telling that did not refuse the change (for example `took_over`).
    pub warnings: Vec<Problem>,
    /// Problems found in the taskbag's files while loading it.
    pub issues: Vec<Issue>,
    /// The blocked tasks this change let go on by themselves: their last blocker finished
    /// (Task-A01-tuna668).
    pub unblocked: Vec<Task>,
}

/// The taskbag loaded at `root`, or why it cannot be worked on.
pub fn taskbag_of(root: &Path, loaded: Loaded) -> Result<(Taskbag, Vec<Issue>), Problem> {
    match loaded.taskbag {
        Some(bag) => Ok((bag, loaded.issues)),
        None => Err(Problem::TaskbagUnreadable {
            path: root.display().to_string(),
            detail: loaded
                .issues
                .first()
                .map(|i| format!("{}: {}", i.file, i.problem.english()))
                .unwrap_or_default(),
        }),
    }
}

/// Before any change, for `alm` and the app alike: refuse a taskbag in a format this Allomos does
/// not write ([`versions::write_allowed`]: a newer one, or an older one whose tasks are to be moved
/// in, Task-A01-45); take the lock (waiting up to `wait` for another writer), then load what the
/// change builds on.
/// What [`upgrade`] did: the format before and after, the main folder's new name, and how the
/// labels moved (decision 0020).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Upgraded {
    pub from: String,
    pub to: String,
    pub folder: Option<String>,
    /// Tasks whose labels changed.
    pub tasks: usize,
    /// How many tasks carry each label after the upgrade, by label.
    pub labels: std::collections::BTreeMap<String, usize>,
    /// Tasks that lost a dropped label (`drafted`, `implemented`).
    pub dropped: usize,
    /// The checkpoint made right after the upgrade (`checkpoints/<name>.json`), which fixes the
    /// history recorded before signing; `None` without a signing key.
    pub checkpoint: Option<String>,
}

/// Upgrade an older taskbag to `allomos/8`, which only a person asks for (approver,
/// 2026-10-25). From `allomos/5`: `project_folder` becomes the first of `project_folders`, named
/// after the folder (decision 0019). From `allomos/5` or `allomos/6`: the ladder becomes the label
/// table and each task's `label` its `labels` (decision 0020, In detail 8): `drafted` and
/// `implemented` are dropped, `locally verified` and `deployed and verified` become "… by agent",
/// `human verified` (or `owner verified`) becomes `deployed and verified by human` when the task's
/// history shows `deployed and verified` before it, else `locally verified by human`, and a task
/// whose history has unreviewed AI decisions gets `AI decisions to review`. `None` when it is
/// already up to date.
pub fn upgrade(root: &Path, actor: Actor, wait: Duration) -> Result<Option<Upgraded>, Problem> {
    versions::write_allowed(root)?;
    let lock = Lock::acquire(root, wait)?;
    let (bag, _) = taskbag_of(root, lock.load())?;
    if bag.project.current() {
        return Ok(None);
    }
    let base = setup::project_folder_of(root, &bag.project)
        .and_then(|f| f.file_name().map(|n| n.to_string_lossy().into_owned()));
    let mut project = allomos_core::folders::upgraded(&bag.project, base.as_deref())
        .unwrap_or(bag.project.clone());
    project.format = FORMAT.to_string();
    let (history, _) = events::list(root);
    let deployed: std::collections::BTreeSet<String> = history
        .iter()
        .filter(|e| e.kind == Kind::Task)
        .filter(|e| {
            e.changes.iter().any(|c| {
                matches!(c.field.as_str(), "label" | "labels")
                    && c.to.as_ref().is_some_and(|v| {
                        v.as_str() == Some("deployed and verified")
                            || v.as_array().is_some_and(|a| {
                                a.iter()
                                    .any(|x| x.as_str() == Some("deployed and verified"))
                            })
                    })
            })
        })
        .filter_map(|e| e.id.clone())
        .collect();
    let unreviewed = decision::pending(&history);
    let mut changed = Vec::new();
    let mut dropped = 0;
    // From `allomos/7` on, labels already have their shape: only signing starts.
    let labels_done = allomos_core::format_version(&bag.project.format)
        .is_some_and(|n| n >= allomos_core::LABELS_FROM);
    for task in bag.tasks.iter().filter(|_| !labels_done) {
        let path = root.join("tasks").join(&task.id).join("task.json");
        let old = fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .and_then(|v| v.get("label").and_then(|l| l.as_str()).map(String::from));
        if old
            .as_deref()
            .is_some_and(|o| allomos_core::labels::upgraded_label(o, false).is_none())
        {
            dropped += 1;
        }
        let labels = allomos_core::labels::upgraded_labels(
            old.as_deref(),
            deployed.contains(&task.id),
            unreviewed.get(&task.id).is_some_and(|d| !d.is_empty()),
        );
        let mut next = task.clone();
        next.labels = labels;
        allomos_core::labels::add_found(&mut project, &next.labels);
        changed.push(next);
    }
    let before = Project {
        format: bag.project.format.clone(),
        ..bag.project.clone()
    };
    let event = actor.event(
        "upgrade",
        Kind::Project,
        Some(&project.id),
        event::changes(Some(&before), &project, &[]),
    );
    lock.record(&event, |l| l.save_project(&project))?;
    let mut tasks = 0;
    let mut counts = std::collections::BTreeMap::new();
    for next in &changed {
        for l in &next.labels {
            *counts.entry(l.clone()).or_insert(0) += 1;
        }
        // Every file is rewritten in the new shape, so the old `label` field goes.
        let raw_label = fs::read_to_string(root.join("tasks").join(&next.id).join("task.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .and_then(|v| v.get("label").cloned())
            .filter(|v| !v.is_null());
        let to = (!next.labels.is_empty()).then(|| serde_json::json!(next.labels));
        let mut changes = Vec::new();
        if raw_label.is_some() || to.is_some() {
            changes.push(allomos_core::event::Change {
                extra: Default::default(),
                field: "labels".into(),
                from: raw_label,
                to,
            });
        }
        if !changes.is_empty() {
            tasks += 1;
        }
        let event = actor.event("upgrade", Kind::Task, Some(&next.id), changes);
        lock.record(&event, |l| l.save_task(next))?;
    }
    if labels_done {
        for task in &bag.tasks {
            for l in &task.labels {
                *counts.entry(l.clone()).or_insert(0) += 1;
            }
        }
    }
    // The `upgrade` event is the first signed one; a checkpoint fixes everything written before
    // it (decision 0021 § 4, events before signing: option A). Without a key store, none.
    let checkpoint = checkpoint::make(root, None).ok().map(|m| m.file);
    Ok(Some(Upgraded {
        from: bag.project.format.clone(),
        to: project.format.clone(),
        folder: project.project_folders.first().map(|f| f.name.clone()),
        tasks,
        labels: counts,
        dropped,
        checkpoint,
    }))
}

pub(crate) fn lock_for_change(
    root: &Path,
    wait: Duration,
) -> Result<(Lock, Taskbag, Vec<Issue>), Problem> {
    versions::write_allowed(root)?;
    versions::upgraded_for_writing(root)?;
    let lock = Lock::acquire(root, wait)?;
    let (bag, issues) = taskbag_of(root, lock.load())?;
    Ok((lock, bag, issues))
}

/// Something added by [`add_area`], [`add_iteration`] or [`add_task`] (or renamed, changed or
/// deleted by [`rename_area`], [`change_iteration`], [`delete_area`], [`delete_iteration`]), with
/// the problems found in the taskbag's files while loading it.
#[derive(Debug, Clone, PartialEq)]
pub struct Added<T> {
    pub item: T,
    pub issues: Vec<Issue>,
}

/// A new area, the same way for `alm add area` and the app (Task-A01-19): under the lock, by the
/// `crates/core` rule, recorded as one `add area` event. Without an ID it takes the next free one.
pub fn add_area(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: Option<&str>,
    name: &str,
) -> Result<Added<Area>, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let id = id.map_or_else(|| ops::next_area_id(&bag), String::from);
    let area = ops::new_area(&bag, &id, name)?;
    let event = actor.event(
        "add area",
        Kind::Area,
        Some(&area.id),
        event::changes(None, &area, &[]),
    );
    lock.record(&event, |l| l.create_area(&area))?;
    Ok(Added { item: area, issues })
}

/// A new iteration, the same way for `alm add iteration` and the app (Task-A01-19): under the lock,
/// by the `crates/core` rule, recorded as one `add iteration` event. Without an ID it takes the
/// next free one.
pub fn add_iteration(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: Option<&str>,
    name: &str,
    details: ops::IterationDetails,
) -> Result<Added<Iteration>, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let id = id.map_or_else(|| ops::next_iteration_id(&bag), String::from);
    let iteration = ops::new_iteration(&bag, &id, name, details)?;
    let event = actor.event(
        "add iteration",
        Kind::Iteration,
        Some(&iteration.id),
        event::changes(None, &iteration, &[]),
    );
    lock.record(&event, |l| l.create_iteration(&iteration))?;
    Ok(Added {
        item: iteration,
        issues,
    })
}

/// The task pane's "New…" iteration (Task-A01-39): a new iteration, by the rule `alm add
/// iteration` uses (without an ID, the next free one), then task `task` planned in it by the rule `alm plan` uses,
/// under one lock, recorded as two events (`add iteration`, `plan`): the same as typing both
/// commands. Both rules are checked before anything is written, so a refusal of either (an empty
/// name, a bad date, an unknown task) writes nothing.
pub fn add_iteration_and_plan(
    root: &Path,
    actor: Actor,
    wait: Duration,
    task: &str,
    id: Option<&str>,
    name: &str,
    details: ops::IterationDetails,
) -> Result<(Iteration, Changed), Problem> {
    let (lock, mut bag, issues) = lock_for_change(root, wait)?;
    let id = id.map_or_else(|| ops::next_iteration_id(&bag), String::from);
    let iteration = ops::new_iteration(&bag, &id, name, details)?;
    bag.iterations.push(iteration.clone());
    let planned = ops::plan(&bag, task, Some(&iteration.id))?;
    let before = bag.tasks.iter().find(|t| t.id == planned.task.id).cloned();
    let added = actor.event(
        "add iteration",
        Kind::Iteration,
        Some(&iteration.id),
        event::changes(None, &iteration, &[]),
    );
    lock.record(&added, |l| l.create_iteration(&iteration))?;
    let plan = actor.event(
        "plan",
        Kind::Task,
        Some(&planned.task.id),
        event::changes(before.as_ref(), &planned.task, event::TASK_FIELDS),
    );
    lock.record(&plan, |l| l.save_task(&planned.task))?;
    Ok((
        iteration,
        Changed {
            task: planned.task,
            warnings: planned.warnings,
            issues,
            unblocked: Vec::new(),
        },
    ))
}

/// An area renamed, the same way for `alm rename area` and the app (Task-A01-29): under the lock,
/// by the `crates/core` rule, recorded as one `rename area` event. Its ID stays.
pub fn rename_area(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    name: &str,
) -> Result<Added<Area>, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let area = ops::rename_area(&bag, id, name)?;
    let before = bag.areas.iter().find(|a| a.id == area.id);
    let event = actor.event(
        "rename area",
        Kind::Area,
        Some(&area.id),
        event::changes(before, &area, &[]),
    );
    lock.record(&event, |l| l.save_area(&area))?;
    Ok(Added { item: area, issues })
}

/// What [`rename_iteration`] changed: the iteration under its new ID, and the tasks moved along.
pub type RenamedIteration = (Iteration, Vec<Task>);

/// An iteration given a new ID (`alm rename iteration`, Task-A00-rope803): under the lock, by the
/// `crates/core` rule, its file written under the new ID and the old one removed, recorded as one
/// `rename iteration` event for it (its `id` from the old to the new) and one for each task
/// planned in it. Renaming to the same ID writes nothing.
pub fn rename_iteration(
    root: &Path,
    actor: Actor,
    wait: Duration,
    old: &str,
    new: &str,
) -> Result<Added<RenamedIteration>, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let (iteration, tasks) = ops::rename_iteration(&bag, old, new)?;
    if iteration.id != old {
        let before = bag.iterations.iter().find(|i| i.id == old);
        let event = actor.event(
            "rename iteration",
            Kind::Iteration,
            Some(&iteration.id),
            event::changes(before, &iteration, &[]),
        );
        lock.record(&event, |l| {
            l.save_iteration(&iteration)?;
            l.remove_item("iterations", old)
        })?;
        for task in &tasks {
            let before = bag.tasks.iter().find(|t| t.id == task.id);
            let event = actor.event(
                "rename iteration",
                Kind::Task,
                Some(&task.id),
                event::changes(before, task, event::TASK_FIELDS),
            );
            lock.record(&event, |l| l.save_task(task))?;
        }
    }
    Ok(Added {
        item: (iteration, tasks),
        issues,
    })
}

/// What [`rename_label`] changed: the new ladder, and the tasks whose label changed.
pub type RenamedLabel = (Vec<String>, Vec<Task>);

/// A label renamed on the ladder together with every task that carries it (`alm rename label`,
/// Task-A01-32): under the lock, by the `crates/core` rule, recorded as one `rename label` event
/// for the project and one for each task changed. Returns the new ladder and the tasks changed.
pub fn rename_label(
    root: &Path,
    actor: Actor,
    wait: Duration,
    old: &str,
    new: &str,
) -> Result<Added<RenamedLabel>, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let (project, tasks) = ops::rename_label(&bag, old, new)?;
    let event = actor.event(
        "rename label",
        Kind::Project,
        Some(&project.id),
        event::changes(Some(&bag.project), &project, &[]),
    );
    lock.record(&event, |l| l.save_project(&project))?;
    for task in &tasks {
        let before = bag.tasks.iter().find(|t| t.id == task.id);
        let event = actor.event(
            "rename label",
            Kind::Task,
            Some(&task.id),
            event::changes(before, task, event::TASK_FIELDS),
        );
        lock.record(&event, |l| l.save_task(task))?;
    }
    Ok(Added {
        item: (project.label_names(), tasks),
        issues,
    })
}

/// An iteration changed (name, release, dates), the same way for `alm change iteration` and the
/// app (Task-A01-29): under the lock, by the `crates/core` rule, recorded as one
/// `change iteration` event. A change that changes nothing writes no event.
pub fn change_iteration(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    change: &ops::IterationChange,
) -> Result<Added<Iteration>, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let iteration = ops::change_iteration(&bag, id, change)?;
    let before = bag.iterations.iter().find(|i| i.id == iteration.id);
    let event = actor.event(
        "change iteration",
        Kind::Iteration,
        Some(&iteration.id),
        event::changes(before, &iteration, &[]),
    );
    lock.record(&event, |l| l.save_iteration(&iteration))?;
    Ok(Added {
        item: iteration,
        issues,
    })
}

/// An iteration moved before or after another one (`alm move iteration`, Task-A00-skate868): under
/// the lock, by the `crates/core` rule, recorded as one `move iteration` event for each iteration
/// whose order changed. Returns the iterations changed; a move that changes nothing writes none.
pub fn move_iteration(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    place: ops::Place<'_>,
) -> Result<Added<Vec<Iteration>>, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let moved = ops::move_iteration(&bag, id, place)?;
    for iteration in &moved {
        let before = bag.iterations.iter().find(|i| i.id == iteration.id);
        let event = actor.event(
            "move iteration",
            Kind::Iteration,
            Some(&iteration.id),
            event::changes(before, iteration, &[]),
        );
        lock.record(&event, |l| l.save_iteration(iteration))?;
    }
    Ok(Added {
        item: moved,
        issues,
    })
}

/// An area deleted, the same way for `alm delete area` and the app (Task-A01-29): refused while
/// any task is in it (done and dropped ones too); otherwise its file is removed under the lock,
/// recorded as one `delete area` event listing what it had. Returns the area as it was.
pub fn delete_area(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
) -> Result<Added<Area>, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let area = ops::delete_area(&bag, id)?;
    let event = actor.event(
        "delete area",
        Kind::Area,
        Some(&area.id),
        event::removed(&area, &[]),
    );
    lock.record(&event, |l| l.remove_item("areas", &area.id))?;
    Ok(Added { item: area, issues })
}

/// An iteration deleted, the same way for `alm delete iteration` and the app (Task-A01-29):
/// refused while any task is planned in it (done and dropped ones too); otherwise its file is
/// removed under the lock, recorded as one `delete iteration` event. Returns it as it was.
pub fn delete_iteration(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
) -> Result<Added<Iteration>, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let iteration = ops::delete_iteration(&bag, id)?;
    let event = actor.event(
        "delete iteration",
        Kind::Iteration,
        Some(&iteration.id),
        event::removed(&iteration, &[]),
    );
    lock.record(&event, |l| l.remove_item("iterations", &iteration.id))?;
    Ok(Added {
        item: iteration,
        issues,
    })
}

/// What a new task is made of (`alm add task`, the app's New task form).
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewTask {
    /// Without one, the next free ID in the area.
    #[serde(default)]
    pub id: Option<String>,
    pub area: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    /// The app only: plan it there too, as `alm plan` would (owner, 2026-10-14).
    #[serde(default)]
    pub iteration: Option<String>,
    /// The branch the agent is on (`alm add task --branch`), which chooses the base (Task-A03-22).
    #[serde(default)]
    pub branch: Option<String>,
    /// The base given (`--base`, or the person's pick in the app): it wins over `branch`.
    #[serde(default)]
    pub base: Option<String>,
    /// The project folders it belongs to (`alm add task --folder`, decision 0019); empty: none in
    /// particular. Each must be one of the taskbag's folders.
    #[serde(default)]
    pub folders: Vec<String>,
    /// How hard it is, 1 to 5 (`alm add task --difficulty`, Task-A01-actor813); `None`: not given,
    /// it counts as 3.
    #[serde(default)]
    pub difficulty: Option<u8>,
}

/// What [`add_task_with_base`] did: the task, the base it got and why, and warnings.
#[derive(Debug, Clone, PartialEq)]
pub struct AddedTask {
    pub task: Task,
    /// The base the task got, and why; `None` when neither a branch nor a base was given.
    pub base: Option<branch::Choice>,
    /// `branch_missing`, or `base_differs` for each task the adding session holds on another base.
    pub warnings: Vec<Problem>,
    pub issues: Vec<Issue>,
}

/// A new task in backlog, the same way for `alm add task` and the app (Task-A01-19): under the
/// lock, by the `crates/core` rule, recorded as one `add task` event. With an iteration it is then
/// planned under the same lock, by the rule `alm plan` uses, as a second `plan` event: the same as
/// typing both commands (owner, 2026-10-14). Both rules are checked before anything is written, so
/// a refusal of either writes nothing.
pub fn add_task(
    root: &Path,
    actor: Actor,
    wait: Duration,
    new: &NewTask,
) -> Result<Added<Task>, Problem> {
    add_task_with_base(root, actor, wait, new).map(|a| Added {
        item: a.task,
        issues: a.issues,
    })
}

/// [`add_task`], telling also which base the task got and why (Task-A03-22). The base is chosen
/// by [`branch::choose_base`] under the same lock and recorded in the `add task` event as a
/// `base_branch` change; `task.json` does not change shape.
pub fn add_task_with_base(
    root: &Path,
    actor: Actor,
    wait: Duration,
    new: &NewTask,
) -> Result<AddedTask, Problem> {
    let (lock, mut bag, issues) = lock_for_change(root, wait)?;
    let id = match &new.id {
        Some(id) => id.clone(),
        None => ops::next_task_id(&bag, &new.area)?,
    };
    let mut task = ops::new_task(&bag, &id, &new.area, &new.title, new.description.as_deref())?;
    task.folders = allomos_core::folders::check(&bag.project, &new.folders)?;
    if let Some(n) = new.difficulty {
        task.difficulty = Some(i64::from(allomos_core::difficulty::level(&n.to_string())?));
    }
    let recorded = branches(root);
    let filled = |v: &Option<String>| v.clone().filter(|b| !b.trim().is_empty());
    let base = branch::choose_base(
        &bag,
        &recorded,
        filled(&new.branch).as_deref(),
        filled(&new.base).as_deref(),
    )?;
    let mut warnings = Vec::new();
    match &base {
        Some(c) => warnings.extend(branch::other_bases(
            &bag, &recorded, actor.by, &task.id, &c.base,
        )),
        // The app adds without a branch when no open task has a base: nothing to say.
        None if actor.via == "alm" => warnings.push(Problem::BranchMissing {
            id: task.id.clone(),
        }),
        None => {}
    }
    bag.tasks.push(task.clone());
    let planned = match new
        .iteration
        .as_deref()
        .map(str::trim)
        .filter(|i| !i.is_empty())
    {
        Some(iteration) => Some(ops::plan(&bag, &task.id, Some(iteration))?.task),
        None => None,
    };
    let mut changes = event::changes(None, &task, event::TASK_FIELDS);
    if let Some(c) = &base {
        changes.extend(branch::change(branch::BASE, None, &c.base));
    }
    let added = actor.event("add task", Kind::Task, Some(&task.id), changes);
    lock.record(&added, |l| l.create_task(&task))?;
    let Some(planned) = planned else {
        return Ok(AddedTask {
            task,
            base,
            warnings,
            issues,
        });
    };
    let plan = actor.event(
        "plan",
        Kind::Task,
        Some(&task.id),
        event::changes(Some(&task), &planned, event::TASK_FIELDS),
    );
    lock.record(&plan, |l| l.save_task(&planned))?;
    Ok(AddedTask {
        task: planned,
        base,
        warnings,
        issues,
    })
}

/// One change to one task, the same way for `alm` and the app (Task-A01-18): refuse a taskbag in
/// another format, take the lock (waiting up to `wait` for another writer), load, apply `rule`
/// (a `crates/core` operation, which also names its command), then write the task and its event.
/// A refused change writes nothing.
pub fn change_task(
    root: &Path,
    actor: Actor,
    wait: Duration,
    rule: impl FnOnce(&Taskbag) -> Result<(allomos_core::ops::Outcome, &'static str), Problem>,
) -> Result<Changed, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let (mut outcome, command) = rule(&bag)?;
    // A label the table does not list yet is added to it, and `alm` says so (decision 0020, 5).
    let mut project = bag.project.clone();
    let added = allomos_core::labels::add_found(&mut project, &outcome.task.labels);
    if !added.is_empty() {
        let event = actor.event(
            "labels",
            Kind::Project,
            Some(&project.id),
            event::changes(Some(&bag.project), &project, &[]),
        );
        lock.record(&event, |l| l.save_project(&project))?;
        outcome
            .warnings
            .extend(added.into_iter().map(|label| Problem::LabelAddedToTable {
                id: outcome.task.id.clone(),
                label,
            }));
    }
    let before = bag.tasks.iter().find(|t| t.id == outcome.task.id);
    let described = before.is_some_and(|b| b.description != outcome.task.description);
    let mut changes = event::changes(before, &outcome.task, event::TASK_FIELDS);
    changes.extend(outcome.recorded.iter().cloned());
    // The holder's change is a heartbeat too (decision 0022 § 5): no extra commit, no event. A
    // change that changes nothing writes none either.
    if !changes.is_empty() {
        heartbeat::along(&mut outcome.task, actor.by, now_secs());
    }
    let event = actor.event(command, Kind::Task, Some(&outcome.task.id), changes);
    lock.record(&event, |l| {
        l.save_task(&outcome.task)?;
        if described {
            l.save_description(&outcome.task)?;
        }
        Ok(())
    })?;
    let unblocked = release_blocked(&lock, &actor, &bag, &outcome.task)?;
    Ok(Changed {
        task: outcome.task,
        warnings: outcome.warnings,
        issues,
        unblocked,
    })
}

/// Now, in seconds since 1970 (UTC); `ALLOMOS_NOW` fixes it in tests, as for events.
pub fn now_secs() -> i64 {
    heartbeat::parse_utc(&events::now_utc()).unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    })
}

/// What `alm beat` did ([`beat`]).
#[derive(Debug, Clone, PartialEq)]
pub struct Beaten {
    /// One entry for each task beaten for, in ID order: written, or too soon to write again.
    pub beats: Vec<heartbeat::Beat>,
    /// The project's heartbeat interval that applied, in seconds (Task-A01-zero387).
    pub interval_secs: i64,
    /// Whether the project asks for work in progress to be pushed at each heartbeat
    /// (Task-A01-butter895).
    pub push_work: bool,
    pub issues: Vec<Issue>,
}

/// `alm beat` (decision 0022, Task-A01-nurse164): write the heartbeat of task `id`, or with no
/// `id` of every task the session `actor.by` holds (in progress), in one change under the lock.
/// `busy`: a busy window of that many seconds. No event is written, and a task whose heartbeat is
/// younger than half the interval is left as it is. Refused without a session name
/// (`holder_missing`), for a task the session does not hold (`task_held`, `not_held`).
pub fn beat(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: Option<&str>,
    busy: Option<i64>,
    doing: Option<&str>,
) -> Result<Beaten, Problem> {
    let me = actor.by.ok_or(Problem::HolderMissing)?;
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let now = now_secs();
    let applied = heartbeat::applied(&bag.project);
    let interval = applied.interval_secs;
    let tasks: Vec<&Task> = match id {
        Some(id) => vec![
            bag.tasks
                .iter()
                .find(|t| t.id == id)
                .ok_or_else(|| Problem::TaskNotFound { id: id.to_string() })?,
        ],
        None => {
            let mut held: Vec<&Task> = bag
                .tasks
                .iter()
                .filter(|t| {
                    t.state == allomos_core::State::InProgress && t.holder.as_deref() == Some(me)
                })
                .collect();
            held.sort_by(|a, b| a.id.cmp(&b.id));
            held
        }
    };
    let beats = tasks
        .into_iter()
        .map(|t| heartbeat::beat(t, me, now, busy, doing, interval))
        .collect::<Result<Vec<_>, _>>()?;
    for b in &beats {
        if let heartbeat::Beat::Written(task) = b {
            lock.save_task(task)?;
        }
    }
    Ok(Beaten {
        beats,
        interval_secs: interval,
        push_work: applied.push_work,
        issues,
    })
}

/// After a change that leaves `task` finished (done or dropped): every blocked task whose
/// blockers have now all finished goes on by itself, each as its own `unblock` event in the same
/// locked change (Task-A01-tuna668, approver 2026-11-08). A task left blocked from before, whose
/// blockers all finished earlier, goes on at the same time. Any other change moves nothing.
fn release_blocked(
    lock: &Lock,
    actor: &Actor,
    bag: &Taskbag,
    task: &Task,
) -> Result<Vec<Task>, Problem> {
    if !matches!(
        task.state,
        allomos_core::State::Done | allomos_core::State::Dropped
    ) {
        return Ok(Vec::new());
    }
    let mut after = bag.clone();
    match after.tasks.iter_mut().find(|t| t.id == task.id) {
        Some(t) => *t = task.clone(),
        None => after.tasks.push(task.clone()),
    }
    write_released(lock, actor, &after)
}

/// Every blocked task in `bag` whose blockers have all finished goes on ([`ops::released`]), each
/// written as its own `unblock` event under the held `lock`.
fn write_released(lock: &Lock, actor: &Actor, bag: &Taskbag) -> Result<Vec<Task>, Problem> {
    let mut unblocked = Vec::new();
    for o in ops::released(bag) {
        let before = bag.tasks.iter().find(|t| t.id == o.task.id);
        let mut changes = event::changes(before, &o.task, event::TASK_FIELDS);
        changes.extend(o.recorded.iter().cloned());
        let event = actor.event("unblock", Kind::Task, Some(&o.task.id), changes);
        lock.record(&event, |l| l.save_task(&o.task))?;
        unblocked.push(o.task);
    }
    Ok(unblocked)
}

/// A task [`release_finished`] let go on, with the blockers that had finished.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Released {
    pub task: Task,
    /// Its blockers, as `blocked_by` listed them before it was cleared.
    pub blockers: Vec<String>,
}

/// Sweep the whole taskbag (Task-A01-steam936): every blocked task whose blockers have all
/// finished goes on now, as the last blocker's `alm done` or `alm drop` would have let it
/// ([`ops::released`]), each as its own `unblock` event in one locked change. The monitor calls it
/// when it starts and on Check now, and `alm next` before it lists. With no such task nothing is
/// written: not even the lock is taken, so a taskbag that cannot be written is only read.
pub fn release_finished(
    root: &Path,
    actor: Actor,
    wait: Duration,
) -> Result<Vec<Released>, Problem> {
    let (bag, _) = taskbag_of(root, load(root))?;
    if ops::released(&bag).is_empty() {
        return Ok(Vec::new());
    }
    let (lock, bag, _) = lock_for_change(root, wait)?;
    let unblocked = write_released(&lock, &actor, &bag)?;
    Ok(unblocked
        .into_iter()
        .map(|task| Released {
            blockers: bag
                .tasks
                .iter()
                .find(|t| t.id == task.id)
                .map(|t| t.blocked_by.clone())
                .unwrap_or_default(),
            task,
        })
        .collect())
}

/// Every task's branches (base and working), by task ID (Task-A03-22), from the history.
pub fn branches(root: &Path) -> std::collections::BTreeMap<String, branch::Branches> {
    branch::recorded(&events::list(root).0)
}

/// When each task last changed, by task ID (Task-A01-52): the newest `at` among its events, in
/// UTC. The Iterations tab sorts by it. A task with no recorded change is left out.
pub fn last_changed(root: &Path) -> std::collections::BTreeMap<String, String> {
    let mut out: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    for e in events::list(root).0 {
        let (Kind::Task, Some(id)) = (e.kind, e.id) else {
            continue;
        };
        let at = out.entry(id).or_default();
        if e.at > *at {
            *at = e.at;
        }
    }
    out
}

/// When each task was made, by task ID (Task-A01-54): the oldest `at` among its events, in UTC.
/// Lists order word IDs by it. A task with no recorded change is left out.
pub fn created(root: &Path) -> std::collections::BTreeMap<String, String> {
    let mut out: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    for e in events::list(root).0 {
        let (Kind::Task, Some(id)) = (e.kind, e.id) else {
            continue;
        };
        let at = out.entry(id).or_insert_with(|| e.at.clone());
        if e.at < *at {
            *at = e.at;
        }
    }
    out
}

/// What [`claim_task`] did: the change, and the task's branches after it.
#[derive(Debug, Clone, PartialEq)]
pub struct Claimed {
    pub changed: Changed,
    /// What the claim recorded, or found, about the task's branches.
    pub branches: branch::AtClaim,
    /// Set when the claim took the task over from another holder (Task-A01-sign613).
    pub took_over: Option<TookOver>,
}

/// A claim that took a task over from another holder (decision 0022 § 6, Task-A01-sign613): from
/// whom, whether it was lost (and since when), and where the former holder worked, when its claim
/// recorded that. The claim's event records the change of holder; this is what `alm claim` tells.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TookOver {
    /// The former holder.
    pub from: String,
    /// Set when the task was lost; `None` when it was taken with `--take` while not lost.
    pub lost: Option<heartbeat::Lost>,
    /// The folder the former holder's claim ran in, relative to the folder holding `.allomos/`,
    /// when it recorded one. It may still hold uncommitted changes.
    pub folder: Option<String>,
    /// The working branch the task had before the claim.
    pub working: Option<String>,
    /// What the former holder last said it was doing (`alm beat --doing`), and when, in UTC.
    pub doing: Option<String>,
    pub doing_at: Option<String>,
}

/// Claim task `id` (`alm claim`, Task-A03-22): the claim rule, plus its branches. `current` is the
/// branch the agent is on, `working` the branch the work is done on (default `current`). A task
/// without a base gets `current`. `session` is the agent's session as it passed it (Task-A03-hive049):
/// it replaces the task's last one, and nothing passed clears it. One `claim` event holds both the
/// task's changes and the branch changes.
#[allow(clippy::too_many_arguments)]
pub fn claim_task(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    take: bool,
    current: Option<&str>,
    working: Option<&str>,
    session: Option<allomos_core::AgentSession>,
) -> Result<Claimed, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    // A lost task may be claimed: claiming it takes it over (decision 0022 § 6, Task-A01-sign613).
    // One held by someone else and not lost is refused as before, unless `--take`.
    let now = now_secs();
    let found = bag.tasks.iter().find(|t| t.id == id);
    let former = found
        .filter(|t| !matches!(t.state, State::Done | State::Dropped))
        .and_then(|t| t.holder.clone())
        .filter(|h| Some(h.as_str()) != actor.by.map(str::trim));
    let lost = match (found, &former) {
        (Some(t), Some(_)) => {
            let applied = heartbeat::applied(&bag.project);
            let last = last_changed(root);
            heartbeat::lost(
                t,
                now,
                applied.lost_after_secs,
                last.get(&t.id).map(String::as_str),
            )
        }
        _ => None,
    };
    let mut outcome = ops::claim(&bag, id, actor.by, take || lost.is_some())?;
    if let Some(l) = &lost {
        for w in outcome.warnings.iter_mut() {
            if let Problem::TookOver { id, previous } = w {
                *w = Problem::TookOverLost {
                    id: id.clone(),
                    previous: previous.clone(),
                    since: l.since.clone(),
                };
            }
        }
    }
    let before = branches(root).remove(id).unwrap_or_default();
    let took_over = former.map(|from| TookOver {
        from,
        lost: lost.clone(),
        folder: found
            .and_then(|t| t.session.as_ref())
            .and_then(|s| s.folder.clone()),
        working: before.working.clone(),
        doing: found
            .and_then(heartbeat::doing)
            .map(|(words, _)| words.to_string()),
        doing_at: found
            .and_then(heartbeat::doing)
            .and_then(|(_, at)| at.map(str::to_string)),
    });
    // Work goes on on the task's working branch (decision 0022 § 6): a takeover keeps it unless
    // `--working` names another.
    let working = match (&took_over, working) {
        (_, Some(w)) => Some(w),
        (Some(t), None) => t.working.as_deref().or(current),
        (None, None) => None,
    };
    outcome.task.session = session
        .filter(|s| !s.is_empty())
        .map(|s| allomos_core::AgentSession {
            by: outcome.task.holder.clone(),
            ..s
        });
    let at = branch::at_claim(id, &before, current, working)?;
    let old = bag.tasks.iter().find(|t| t.id == outcome.task.id);
    let mut changes = event::changes(old, &outcome.task, event::TASK_FIELDS);
    changes.extend(at.changes.iter().cloned());
    if !changes.is_empty() {
        heartbeat::along(&mut outcome.task, actor.by, now);
    }
    let event = actor.event("claim", Kind::Task, Some(&outcome.task.id), changes);
    lock.record(&event, |l| l.save_task(&outcome.task))?;
    let mut warnings = outcome.warnings;
    warnings.extend(at.warnings.iter().cloned());
    Ok(Claimed {
        changed: Changed {
            task: outcome.task,
            warnings,
            issues,
            unblocked: Vec::new(),
        },
        branches: at,
        took_over,
    })
}

/// Ask the session holding task `id` to hand it over to `to` (Task-A01-minnow200): the app's
/// **Hand over to another agent…**. One `request-handover` event, under the lock.
pub fn request_handover(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    to: &str,
) -> Result<Changed, Problem> {
    let at = allomos_core::heartbeat::format_utc(now_secs());
    change_task(root, actor, wait, |bag| {
        allomos_core::handover::request(bag, id, actor.by, to, &at)
            .map(|o| (o, "request-handover"))
    })
}

/// Take back a request to hand task `id` over (Task-A01-minnow200). One `cancel-handover` event,
/// under the lock.
pub fn cancel_handover(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
) -> Result<Changed, Problem> {
    change_task(root, actor, wait, |bag| {
        allomos_core::handover::cancel(bag, id).map(|o| (o, "cancel-handover"))
    })
}

/// Change task `id`'s base to `name` (`alm base`, Task-A03-22): one `base` event, under the lock;
/// the task's files do not change. Returns the task and its branches after it.
pub fn set_base(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    name: &str,
) -> Result<(Task, branch::Branches, Vec<Issue>), Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let recorded = branches(root);
    let changes = branch::set_base(&bag, &recorded, id, actor.by, name)?;
    let task = ops::find_task(&bag, id)?;
    let event = actor.event(branch::SET_BASE, Kind::Task, Some(&task.id), changes);
    lock.record(&event, |_| Ok(()))?;
    let after = branches(root).remove(&task.id).unwrap_or_default();
    Ok((task, after, issues))
}

/// Every task's traces (the requirements and decisions it serves), by task ID
/// (Task-A01-fennel617), from the history.
pub fn traces(root: &Path) -> std::collections::BTreeMap<String, Vec<String>> {
    allomos_core::traces::recorded(&events::list(root).0)
}

/// Add and remove task `id`'s traces (`alm trace`, Task-A01-fennel617): one `trace` event, under
/// the lock, when the list changes; the task's files do not change. Returns the task and its
/// traces after it.
pub fn set_traces(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    add: &[String],
    remove: &[String],
) -> Result<(Task, Vec<String>, Vec<Issue>), Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let recorded = traces(root);
    let (changes, after) = allomos_core::traces::set(&bag, &recorded, id, actor.by, add, remove)?;
    let task = ops::find_task(&bag, id)?;
    if !changes.is_empty() {
        let event = actor.event(
            allomos_core::traces::TRACE,
            Kind::Task,
            Some(&task.id),
            changes,
        );
        lock.record(&event, |_| Ok(()))?;
    }
    Ok((task, after, issues))
}

/// What [`record_ai_decision`] or [`review_ai_decisions`] did (Task-A03-19).
#[derive(Debug, Clone, PartialEq)]
pub struct Decided {
    pub task: Task,
    /// The decisions the change recorded or marked reviewed.
    pub decisions: Vec<decision::Decision>,
    /// The task's decisions still to review after the change.
    pub pending: Vec<decision::Decision>,
    /// Problems found in the taskbag's files while loading it.
    pub issues: Vec<Issue>,
}

/// Every task's AI decisions not reviewed yet, by task ID (Task-A03-19), from the history.
pub fn ai_decisions(root: &Path) -> std::collections::BTreeMap<String, Vec<decision::Decision>> {
    decision::pending(&events::list(root).0)
}

/// Record a decision an agent made on its own in task `id` (`alm ai-decided`, Task-A03-19): one
/// `ai-decided` event, under the lock. The task's files do not change.
pub fn record_ai_decision(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    text: &str,
) -> Result<Decided, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let mut changes = decision::decide(&bag, id, text)?;
    let before = ops::find_task(&bag, id)?;
    // The task carries `AI decisions to review` until `alm reviewed` (decision 0020).
    let mut task = before.clone();
    task.add_label(allomos_core::labels::AI_DECISIONS);
    heartbeat::along(&mut task, actor.by, now_secs());
    changes.extend(event::changes(Some(&before), &task, event::TASK_FIELDS));
    let event = actor.event(decision::DECIDED, Kind::Task, Some(&task.id), changes);
    lock.record(&event, |l| {
        if task != before {
            l.save_task(&task)?;
        }
        Ok(())
    })?;
    let pending = ai_decisions(root).remove(&task.id).unwrap_or_default();
    let decisions = pending.last().cloned().into_iter().collect();
    Ok(Decided {
        task,
        decisions,
        pending,
        issues,
    })
}

/// Record in task `id`'s history that the person chose Go ahead on a plan shown through
/// `alm confirm --task` (Task-A03-37): since Task-A01-coach782 an approval record
/// ([`approvals::approve`], channel `confirm`) whose scope holds the SHA-256 of exactly the plan
/// text shown (`plan`, from the file `plan_file`). A changed plan is a new question.
pub fn record_approval(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    title: &str,
    plan_file: &str,
    plan: &str,
) -> Result<approvals::Approved, Problem> {
    let tasks = [id.to_string()];
    approvals::approve(
        root,
        actor,
        wait,
        &approvals::Input {
            tasks: &tasks,
            approver: None,
            channel: "confirm",
            words: Some(&format!("Go ahead: {title}")),
            expires: None,
            plan: Some((plan_file, plan)),
            shown: None,
        },
    )
}

/// Mark task `id`'s AI decisions reviewed (`alm reviewed`, the app's Reviewed button,
/// Task-A03-19): one `reviewed` event listing each, under the lock. Refused with
/// `nothing_to_review` when it has none. Overturning one is done by changing the task as usual.
pub fn review_ai_decisions(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
) -> Result<Decided, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let before = ops::find_task(&bag, id)?;
    let decisions = ai_decisions(root).remove(&before.id).unwrap_or_default();
    let mut task = before.clone();
    let had_label = task.remove_label(allomos_core::labels::AI_DECISIONS);
    heartbeat::along(&mut task, actor.by, now_secs());
    // A task carrying the label with nothing left in its history is cleared too (decision 0020).
    let mut changes = if decisions.is_empty() && had_label {
        Vec::new()
    } else {
        decision::review(&bag, id, &decisions)?
    };
    changes.extend(event::changes(Some(&before), &task, event::TASK_FIELDS));
    let event = actor.event(decision::REVIEWED, Kind::Task, Some(&task.id), changes);
    lock.record(&event, |l| {
        if task != before {
            l.save_task(&task)?;
        }
        Ok(())
    })?;
    Ok(Decided {
        task,
        decisions,
        pending: Vec::new(),
        issues,
    })
}

/// A fingerprint of the taskbag's files (names, sizes and change times). It changes when any
/// file is written, so a window can reload only when something changed.
pub fn stamp(root: &Path) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let mut add = |p: &Path| {
        if let Ok(m) = fs::metadata(p) {
            p.hash(&mut h);
            m.len().hash(&mut h);
            m.modified().ok().hash(&mut h);
        }
    };
    add(&root.join(PROJECT_FILE));
    add(&root.join(handoff::NOTES_FILE));
    // A new event changes its folder's time; reading every event file here would grow with history.
    add(&root.join(events::EVENTS_DIR));
    // An older taskbag's handoff files (Task-A03-polar770): the notice goes once they are moved.
    for folder in ["areas", "iterations", handoff::HANDOFFS_DIR] {
        for p in sorted_entries(&root.join(folder)) {
            add(&p);
        }
    }
    for dir in sorted_entries(&root.join("tasks")) {
        add(&dir);
        for p in sorted_entries(&dir) {
            add(&p);
        }
    }
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use allomos_core::ops;
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

    /// Task-A01-steam936: the sweep lets every left-over blocked task go on (in progress when
    /// held, else ready), each with an `unblock` event; with none, it writes nothing.
    #[test]
    fn the_sweep_unblocks_left_overs_and_otherwise_writes_nothing() {
        let dir = tmp("sweep");
        let root = dir.join(DEFAULT_DIR);
        init(&root, "p", "P").unwrap();
        let lock = Lock::acquire(&root, Duration::ZERO).unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.create_area(&ops::new_area(&bag, "A01", "First").unwrap())
            .unwrap();
        for id in ["A01-01", "A01-02", "A01-03", "A01-04"] {
            let bag = lock.load().taskbag.unwrap();
            lock.create_task(&ops::new_task(&bag, id, "A01", id, None).unwrap())
                .unwrap();
        }
        // A01-02 (held by b) and A01-03 blocked by the done A01-01; A01-04 by the open A01-03.
        let mut bag = lock.load().taskbag.unwrap();
        for t in bag.tasks.iter_mut() {
            match t.id.as_str() {
                "A01-01" => t.state = allomos_core::State::Done,
                "A01-02" | "A01-03" => {
                    t.state = allomos_core::State::Blocked;
                    t.blocked_by = vec!["A01-01".into()];
                    if t.id == "A01-02" {
                        t.holder = Some("b".into());
                    }
                }
                "A01-04" => {
                    t.state = allomos_core::State::Blocked;
                    t.blocked_by = vec!["A01-03".into()];
                }
                _ => {}
            }
            lock.save_task(t).unwrap();
        }
        drop(lock);
        let events = |root: &Path| events::list(root).0.len();
        let before = events(&root);
        let released = release_finished(&root, Actor::alm(Some("s")), Duration::ZERO).unwrap();
        let ids: Vec<&str> = released.iter().map(|r| r.task.id.as_str()).collect();
        assert_eq!(ids, ["A01-02", "A01-03"]);
        assert_eq!(released[0].blockers, ["A01-01"]);
        assert_eq!(events(&root), before + 2);
        let bag = load(&root).taskbag.unwrap();
        let state = |id: &str| bag.tasks.iter().find(|t| t.id == id).unwrap().clone();
        assert_eq!(state("A01-02").state, allomos_core::State::InProgress);
        assert_eq!(state("A01-03").state, allomos_core::State::Ready);
        assert!(state("A01-03").blocked_by.is_empty());
        assert!(
            state("A01-03")
                .handoff_note
                .unwrap()
                .starts_with(&ops::released_note(&["A01-01".into()]))
        );
        assert_eq!(state("A01-04").state, allomos_core::State::Blocked);
        let e = &events::history(&root, "A01-03").events[0];
        assert_eq!(e.command, "unblock");
        assert!(
            allomos_core::event::reason(e)
                .unwrap()
                .starts_with("automatic:")
        );
        // Nothing left to let go on: no change, no event.
        let before = events(&root);
        assert_eq!(
            release_finished(&root, Actor::alm(Some("s")), Duration::ZERO).unwrap(),
            Vec::new()
        );
        assert_eq!(events(&root), before);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn init_with_own_id_and_ladder() {
        let dir = tmp("init-ladder");
        let root = dir.join(DEFAULT_DIR);
        let ladder = vec!["shipped".to_string(), " ".to_string()];
        init_with_labels(&root, "allomos", "Allomos", ladder, Actor::APP).unwrap();
        let bag = load(&root).taskbag.unwrap();
        let names = bag.project.label_names();
        assert_eq!(bag.project.id, "allomos");
        assert_eq!(names.len(), 6, "{names:?}");
        assert_eq!(names[5], "shipped");
        let other = dir.join("other");
        init_with_labels(&other, "p", "P", vec![], Actor::APP).unwrap();
        assert_eq!(load(&other).taskbag.unwrap().project.labels.len(), 5);
        let other = dir.join("other-2");
        assert!(!other.exists());
        assert!(matches!(
            init_with_labels(&dir.join("x"), "bad id", "P", vec!["a".into()], Actor::APP),
            Err(Problem::InvalidId { .. })
        ));
        fs::remove_dir_all(dir).unwrap();
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

    #[test]
    fn hidden_only_folder_looks_empty() {
        let dir = tmp("hidden-only");
        assert!(looks_empty(&dir));
        fs::write(dir.join(".DS_Store"), "x").unwrap();
        fs::create_dir(dir.join(".git")).unwrap();
        assert!(looks_empty(&dir));
        assert_eq!(
            inspect(&dir),
            Inspection::Empty {
                folder: dir.clone(),
                suggested_root: dir.join(DEFAULT_DIR)
            }
        );
        fs::write(dir.join("notes.txt"), "x").unwrap();
        assert!(!looks_empty(&dir));
        assert!(matches!(inspect(&dir), Inspection::NoTaskbag { .. }));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn readme_goes_only_into_a_folder_that_looks_empty() {
        // Empty, and only `.DS_Store`: the taskbag and the README both arrive.
        for extra in [None, Some(".DS_Store")] {
            let dir = tmp("readme-empty");
            if let Some(name) = extra {
                fs::write(dir.join(name), "x").unwrap();
            }
            let root = dir.join(DEFAULT_DIR);
            create_starter(&root, "Demo").unwrap();
            assert_eq!(picked_folder(&root), Some(dir.as_path()));
            assert!(add_readme_if_empty(&dir, "en").unwrap());
            let text = fs::read_to_string(dir.join(README_FILE)).unwrap();
            assert!(text.contains(".allomos/taskbag/"));
            assert_eq!(inspect(&dir), Inspection::Found { root: root.clone() });
            // Now the folder has a visible file: a second call writes nothing.
            assert!(!add_readme_if_empty(&dir, "ja").unwrap());
            assert_eq!(fs::read_to_string(dir.join(README_FILE)).unwrap(), text);
            fs::remove_dir_all(dir).unwrap();
        }
        // A folder with a visible file gets no README.
        let dir = tmp("readme-code");
        fs::write(dir.join("main.rs"), "fn main() {}").unwrap();
        create_starter(&dir.join(DEFAULT_DIR), "Code").unwrap();
        assert!(!add_readme_if_empty(&dir, "en").unwrap());
        assert!(!dir.join(README_FILE).exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn readme_never_overwrites() {
        let dir = tmp("readme-keep");
        fs::create_dir(dir.join(".git")).unwrap();
        let path = dir.join(README_FILE);
        // An existing README is a visible file, so the folder no longer looks empty.
        fs::write(&path, "mine").unwrap();
        assert!(!add_readme_if_empty(&dir, "en").unwrap());
        assert_eq!(fs::read_to_string(&path).unwrap(), "mine");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn readme_speaks_three_languages() {
        assert!(readme_text("en").starts_with("# Allomos project"));
        assert!(readme_text("zh-CN").contains("此文件夹是一个「换肩」项目"));
        assert!(readme_text("ja").contains("このフォルダーは「肩代」のプロジェクトです"));
        assert_eq!(readme_text("fr"), readme_text("en"));
        for lang in ["en", "zh-CN", "ja"] {
            assert!(readme_text(lang).contains("`.allomos/taskbag/`"));
            assert!(readme_text(lang).contains("`alm"));
        }
        assert_eq!(picked_folder(Path::new("/x/taskbag")), None);
    }

    #[test]
    fn empty_folder_gets_starter_taskbag_in_default_dir() {
        let dir = tmp("empty");
        let root = dir.join(".allomos/taskbag");
        assert_eq!(
            inspect(&dir),
            Inspection::Empty {
                folder: dir.clone(),
                suggested_root: root.clone()
            }
        );
        create_starter(&root, "Demo").unwrap();
        assert_eq!(inspect(&dir), Inspection::Found { root: root.clone() });
        let loaded = load(&root);
        assert_eq!(
            loaded.taskbag.as_ref().unwrap().project.format,
            allomos_core::FORMAT
        );
        assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
        let tb = loaded.taskbag.unwrap();
        assert_eq!(tb.project.name, "Demo");
        assert_eq!(tb.areas.len(), 2);
        assert_eq!(tb.tasks.len(), 3);
        assert!(tb.tasks[0].description.contains("starter"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn repo_folder_gets_nested_taskbag_without_nested_git() {
        let repo = tmp("repo");
        fs::create_dir(repo.join(".git")).unwrap();
        fs::write(repo.join("README.md"), "code").unwrap();
        let Inspection::NoTaskbag { suggested_root, .. } = inspect(&repo) else {
            panic!("expected NoTaskbag")
        };
        assert_eq!(suggested_root, repo.join(".allomos/taskbag"));
        create_starter(&suggested_root, "Repo").unwrap();
        assert_eq!(
            inspect(&repo),
            Inspection::Found {
                root: repo.join(".allomos/taskbag")
            }
        );
        assert!(!suggested_root.join(".git").exists());
        fs::remove_dir_all(repo).unwrap();
    }

    #[test]
    fn refuses_to_overwrite_a_non_empty_folder() {
        let dir = tmp("full");
        fs::write(dir.join("notes.txt"), "keep me").unwrap();
        assert!(create_starter(&dir, "X").is_err());
        assert_eq!(
            fs::read_to_string(dir.join("notes.txt")).unwrap(),
            "keep me"
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn invalid_file_is_reported_and_valid_tasks_still_load() {
        let dir = tmp("broken");
        create_starter(&dir, "Demo").unwrap();
        fs::write(
            dir.join("tasks/A01-02/task.json"),
            r#"{"id":"A01-02","area":"A01","state":"ready"}"#,
        )
        .unwrap();
        let loaded = load(&dir);
        assert_eq!(loaded.issues.len(), 1);
        assert_eq!(loaded.issues[0].file, "tasks/A01-02/task.json");
        assert_eq!(loaded.issues[0].field.as_deref(), Some("title"));
        assert_eq!(loaded.taskbag.unwrap().tasks.len(), 2);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_project_file_is_an_issue_not_a_crash() {
        let dir = tmp("noproject");
        let loaded = load(&dir);
        assert!(loaded.taskbag.is_none());
        assert_eq!(loaded.issues[0].file, "project.json");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn not_a_folder() {
        let p = tmp("nf").join("missing");
        assert!(matches!(inspect(&p), Inspection::NotAFolder { .. }));
    }

    #[test]
    fn init_makes_an_empty_taskbag_found_from_below() {
        let repo = tmp("init");
        let root = repo.join(DEFAULT_DIR);
        init(&root, &id_from_name("my repo!"), "My repo").unwrap();
        assert_eq!(
            init(&root, "x", "X"),
            Err(Problem::TaskbagExists {
                path: root.display().to_string()
            })
        );
        let loaded = load(&root);
        assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
        let bag = loaded.taskbag.unwrap();
        assert_eq!(bag.project.id, "my-repo");
        assert!(bag.areas.is_empty() && bag.tasks.is_empty());
        assert!(
            fs::read_to_string(root.join(".gitignore"))
                .unwrap()
                .contains(".lock")
        );
        let deep = repo.join("src/a/b");
        fs::create_dir_all(&deep).unwrap();
        assert_eq!(find_taskbag(&deep), Some(root.clone()));
        assert_eq!(find_taskbag(&std::env::temp_dir()), None);
        // No temporary folder is left next to the taskbag.
        let left: Vec<_> = fs::read_dir(repo.join(".allomos")).unwrap().collect();
        assert_eq!(left.len(), 1);
        fs::remove_dir_all(repo).unwrap();
    }

    #[test]
    fn id_from_name_keeps_ids_valid() {
        assert_eq!(id_from_name("allomos"), "allomos");
        assert_eq!(id_from_name("我的"), "project");
        assert_eq!(id_from_name(&"a".repeat(80)).len(), 64);
    }

    #[test]
    fn writes_go_through_the_lock() {
        let dir = tmp("lock");
        let root = dir.join(DEFAULT_DIR);
        assert!(matches!(
            Lock::acquire(&root, Duration::ZERO),
            Err(Problem::TaskbagNotFound { .. })
        ));
        init(&root, "p", "P").unwrap();
        let before = stamp(&root);
        let lock = Lock::acquire(&root, Duration::ZERO).unwrap();
        // A second writer waits, then gives up while the first holds the lock.
        assert!(matches!(
            Lock::acquire(&root, Duration::from_millis(20)),
            Err(Problem::TaskbagBusy { .. })
        ));
        let bag = lock.load().taskbag.unwrap();
        let area = allomos_core::ops::new_area(&bag, "A01", "First").unwrap();
        lock.create_area(&area).unwrap();
        assert!(matches!(
            lock.create_area(&area),
            Err(Problem::DuplicateAreaId { .. })
        ));
        let bag = lock.load().taskbag.unwrap();
        let task =
            allomos_core::ops::new_task(&bag, "A01-01", "A01", "Do it", Some("Why")).unwrap();
        lock.create_task(&task).unwrap();
        assert!(matches!(
            lock.create_task(&task),
            Err(Problem::DuplicateTaskId { .. })
        ));
        let bag = lock.load().taskbag.unwrap();
        let claimed = allomos_core::ops::claim(&bag, "A01-01", Some("me"), false)
            .unwrap()
            .task;
        lock.save_task(&claimed).unwrap();
        drop(lock);
        assert!(Lock::acquire(&root, Duration::ZERO).is_ok());

        let loaded = load(&root);
        assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
        let t = &loaded.taskbag.unwrap().tasks[0];
        assert_eq!(t.holder.as_deref(), Some("me"));
        assert_eq!(t.description, "Why\n");
        let json = fs::read_to_string(root.join("tasks/A01-01/task.json")).unwrap();
        assert!(!json.contains("null") && !json.contains("description"));
        assert_ne!(stamp(&root), before);
        // Only the lock and the taskbag's own files: no temporary files left behind.
        let hidden: Vec<_> = fs::read_dir(&root)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|n| n.starts_with(TMP_PREFIX))
            .collect();
        assert!(hidden.is_empty(), "{hidden:?}");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn iterations_are_written_read_and_sorted() {
        use allomos_core::ops::{self, IterationDetails};
        let dir = tmp("iterations");
        let root = dir.join(DEFAULT_DIR);
        init(&root, "p", "P").unwrap();
        // A taskbag made before iterations existed has no `iterations/` folder: still valid.
        assert!(!root.join("iterations").exists());
        let loaded = load(&root);
        assert!(loaded.issues.is_empty() && loaded.taskbag.unwrap().iterations.is_empty());

        let before = stamp(&root);
        let lock = Lock::acquire(&root, Duration::ZERO).unwrap();
        let bag = lock.load().taskbag.unwrap();
        let two = ops::new_iteration(
            &bag,
            "I02",
            "v0.2",
            IterationDetails {
                release: Some("v0.2"),
                ..Default::default()
            },
        )
        .unwrap();
        lock.create_iteration(&two).unwrap();
        assert!(matches!(
            lock.create_iteration(&two),
            Err(Problem::DuplicateIterationId { .. })
        ));
        // Written by hand with a lower order: listed first.
        fs::write(
            root.join("iterations/I01.json"),
            r#"{"id":"I01","name":"v0.1","order":0}"#,
        )
        .unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.create_area(&ops::new_area(&bag, "A01", "First").unwrap())
            .unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.create_task(&ops::new_task(&bag, "A01-01", "A01", "Do it", None).unwrap())
            .unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.save_task(&ops::plan(&bag, "A01-01", Some("I02")).unwrap().task)
            .unwrap();
        drop(lock);

        let json = fs::read_to_string(root.join("iterations/I02.json")).unwrap();
        assert!(!json.contains("null") && !json.contains("start"), "{json}");
        let loaded = load(&root);
        assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
        let bag = loaded.taskbag.unwrap();
        let ids: Vec<_> = bag.iterations.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(ids, ["I01", "I02"]);
        assert_eq!(bag.tasks[0].iteration.as_deref(), Some("I02"));
        assert_ne!(stamp(&root), before);

        // A task planned in a missing iteration is reported, and the task still shows.
        fs::remove_file(root.join("iterations/I02.json")).unwrap();
        let loaded = load(&root);
        assert_eq!(loaded.issues[0].problem.code(), "unknown_iteration");
        assert_eq!(loaded.taskbag.unwrap().tasks.len(), 1);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn task_links_open_only_text_files_in_the_project_and_web_pages() {
        let dir = tmp("links");
        let root = dir.join(DEFAULT_DIR);
        init(&root, "p", "P").unwrap();
        fs::create_dir_all(dir.join("docs/tasks")).unwrap();
        fs::write(dir.join("docs/tasks/Card.md"), "# Card\n").unwrap();
        fs::write(dir.join("docs/run.sh"), "echo\n").unwrap();
        fs::create_dir_all(root.join("tasks/T1")).unwrap();
        let outside = tmp("links-outside");
        fs::write(outside.join("other.md"), "x").unwrap();

        // On Windows without the `\\?\` prefix canonicalize adds, so Explorer can open it.
        let card = plain_path(&dir.join("docs/tasks/Card.md").canonicalize().unwrap());
        let file = |link: &str| resolve_task_link(&root, "T1", link);
        assert_eq!(
            file("../../../../docs/tasks/Card.md"),
            Ok(LinkTarget::File(card.clone()))
        );
        assert_eq!(
            file("../../../../docs/tasks/Card.md#approvals"),
            Ok(LinkTarget::File(card))
        );
        assert_eq!(
            file("https://example.com/a"),
            Ok(LinkTarget::Web("https://example.com/a".into()))
        );
        let refused = |link: &str| Err(Problem::LinkRefused { link: link.into() });
        for link in [
            "../../../../docs/run.sh",
            "../../../../docs/tasks",
            "file:///etc/hosts",
            "mailto:a@b.c",
            "javascript:alert(1)",
            "/etc/hosts",
            "",
        ] {
            assert_eq!(file(link), refused(link), "{link}");
        }
        // Outside the project, even as a text file.
        let escape = format!(
            "../../../../../{}/other.md",
            outside.file_name().unwrap().to_str().unwrap()
        );
        assert_eq!(file(&escape), refused(&escape));
        assert_eq!(
            file("../../../../docs/tasks/Gone.md"),
            Err(Problem::LinkNotFound {
                link: "../../../../docs/tasks/Gone.md".into()
            })
        );
        assert!(matches!(
            resolve_task_link(&root, "../x", "a.md"),
            Err(Problem::LinkRefused { .. })
        ));
        // A taskbag kept beside its code: the recorded code folder counts as the project too.
        record_project_folder(&root, &outside).unwrap();
        let other = plain_path(&outside.join("other.md").canonicalize().unwrap());
        assert_eq!(file(&escape), Ok(LinkTarget::File(other)));
        fs::remove_dir_all(dir).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }

    #[test]
    fn hidden_entries_are_not_tasks() {
        let dir = tmp("hidden");
        init(&dir.join("s"), "p", "P").unwrap();
        let root = dir.join("s");
        fs::create_dir_all(root.join("tasks/.tmp-1-0-X")).unwrap();
        fs::write(root.join("areas/.tmp-1-1-A.json"), "{").unwrap();
        let loaded = load(&root);
        assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
        fs::remove_dir_all(dir).unwrap();
    }

    /// Every text file under `dir` rewritten with `\r\n` line endings, the way Git on Windows
    /// may check a taskbag out.
    fn to_crlf(dir: &Path) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                to_crlf(&path);
            } else if let Ok(text) = fs::read_to_string(&path) {
                fs::write(&path, text.replace('\n', "\r\n")).unwrap();
            }
        }
    }

    #[test]
    fn a_taskbag_checked_out_with_crlf_loads_and_is_written_back_with_lf() {
        let dir = tmp("crlf");
        let root = default_root(&dir);
        init(&root, "p", "P").unwrap();
        let lock = Lock::acquire(&root, Duration::ZERO).unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.create_area(&ops::new_area(&bag, "A01", "First").unwrap())
            .unwrap();
        let bag = lock.load().taskbag.unwrap();
        let mut task =
            ops::new_task(&bag, "A01-01", "A01", "Do it", Some("Why\nand how\n")).unwrap();
        lock.create_task(&task).unwrap();
        drop(lock);

        to_crlf(&root);
        assert!(
            fs::read_to_string(root.join("project.json"))
                .unwrap()
                .contains("\r\n")
        );
        let loaded = load(&root);
        assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
        let bag = loaded.taskbag.unwrap();
        assert_eq!(bag.tasks[0].description, "Why\nand how\n");

        // What Allomos writes back has `\n` only, even from text that came with `\r\n`.
        let lock = Lock::acquire(&root, Duration::ZERO).unwrap();
        task.holder = Some("me".into());
        lock.save_task(&task).unwrap();
        lock.create_task(&Task {
            id: "A01-02".into(),
            description: "One\r\nTwo\r\n".into(),
            ..task.clone()
        })
        .unwrap();
        drop(lock);
        for file in ["tasks/A01-01/task.json", "tasks/A01-02/description.md"] {
            let text = fs::read_to_string(root.join(file)).unwrap();
            assert!(!text.contains('\r'), "{file}: {text:?}");
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_new_taskbag_keeps_lf_line_endings_in_git() {
        let dir = tmp("attributes");
        let root = default_root(&dir);
        init(&root, "p", "P").unwrap();
        let text = fs::read_to_string(root.join(".gitattributes")).unwrap();
        assert!(text.contains("*.json text eol=lf"), "{text}");
        assert!(text.contains("*.md text eol=lf"), "{text}");
        create_starter(&dir.join("starter"), "S").unwrap();
        assert!(dir.join("starter/.gitattributes").is_file());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_path_with_backslashes_finds_its_project() {
        let base = tmp("backslash");
        let home = base.join("tasks-home");
        let app = base.join("app");
        fs::create_dir_all(&app).unwrap();
        let root = default_root(&home);
        init(&root, "p", "P").unwrap();
        // Written on Windows by an older Allomos, or by hand: `\` between the parts.
        let mut bag = load(&root).taskbag.unwrap();
        bag.project.project_folder = Some(r"..\app".into());
        assert_eq!(project_folder_of(&root, &bag.project), Some(app.clone()));
        bag.project.project_folder = Some(r"..\app\".into());
        assert_eq!(project_folder_of(&root, &bag.project), Some(app.clone()));
        // Recorded with `/`, so the taskbag works on every computer.
        assert_eq!(record_project_folder(&root, &app).unwrap(), "../app");
        // The taskbag found from the project folder, and the project folder from the taskbag.
        assert_eq!(picked_folder(&root), Some(home.as_path()));
        assert_eq!(find_taskbag(&home.join("src")), Some(root.clone()));
        // An AGENTS.md beside a sibling taskbag names it with `/`.
        let section = agent_section_for(&app, &root);
        assert!(
            section.contains("`--taskbag \"../tasks-home/.allomos/taskbag\"`"),
            "{section}"
        );
        assert_eq!(portable_path(Path::new(r"..\a\b")), "../a/b");
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn windows_paths_are_given_to_explorer_plainly() {
        assert_eq!(
            plain_windows(r"\\?\C:\p\.allomos\taskbag"),
            r"C:\p\.allomos\taskbag"
        );
        assert_eq!(
            plain_windows(r"C:\p\.allomos/taskbag"),
            r"C:\p\.allomos\taskbag"
        );
        assert_eq!(
            plain_windows(r"\\?\UNC\server\share\p"),
            r"\\server\share\p"
        );
        assert_eq!(plain_windows(r"\\server\share\p"), r"\\server\share\p");
    }

    #[test]
    fn replacing_a_file_that_is_open_for_reading_succeeds() {
        let dir = tmp("open-reader");
        let root = default_root(&dir);
        init(&root, "p", "P").unwrap();
        let lock = Lock::acquire(&root, Duration::ZERO).unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.create_area(&ops::new_area(&bag, "A01", "First").unwrap())
            .unwrap();
        let bag = lock.load().taskbag.unwrap();
        let mut task = ops::new_task(&bag, "A01-01", "A01", "Do it", None).unwrap();
        lock.create_task(&task).unwrap();
        let path = root.join("tasks/A01-01/task.json");
        let before = fs::read_to_string(&path).unwrap();

        // Another reader has the file open while it is replaced.
        let mut reader = fs::File::open(&path).unwrap();
        task.holder = Some("me".into());
        lock.save_task(&task).unwrap();
        let mut seen = String::new();
        io::Read::read_to_string(&mut reader, &mut seen).unwrap();
        // The reader sees the old file whole, or (where the system shares it) the new one whole.
        let after = fs::read_to_string(&path).unwrap();
        assert!(seen == before || seen == after, "{seen}");
        assert!(after.contains("\"holder\": \"me\""), "{after}");
        drop(reader);
        drop(lock);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn replacing_retries_while_the_file_is_in_use_then_gives_up() {
        let in_use = || io::Error::from(io::ErrorKind::PermissionDenied);
        // In use twice, then free: it works.
        let mut tries = 0;
        let result = retry_in_use(Duration::from_secs(1), Duration::ZERO, || {
            tries += 1;
            if tries < 3 { Err(in_use()) } else { Ok(tries) }
        });
        assert_eq!(result.unwrap(), 3);
        // Another failure is not retried.
        let mut tries = 0;
        let result: io::Result<()> = retry_in_use(Duration::from_secs(1), Duration::ZERO, || {
            tries += 1;
            Err(io::Error::from(io::ErrorKind::NotFound))
        });
        assert!(result.is_err());
        assert_eq!(tries, 1);
        // Still in use when the time is up: the failure is returned.
        let result: io::Result<()> =
            retry_in_use(Duration::from_millis(20), Duration::from_millis(5), || {
                Err(in_use())
            });
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
    }
}
