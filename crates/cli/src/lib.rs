//! The `allomos` command, also installed as `alm`.
//!
//! It reads options, calls the rules in `allomos-core` and the writer in
//! `allomos-store`, and prints the result: plain lines for people, or one JSON object
//! for agents (`--json`). It never asks for typed input, and never runs or reads Git: after a
//! change it prints the Git commands for exactly the files that change wrote, and the agent runs
//! them (decision 0017). The contract is in `docs/workflow/CODING_RULES.md` § How `alm` behaves.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use allomos_core::decision::Decision;
use allomos_core::event::{self, Kind};
use allomos_core::ops::{self, Outcome};
use allomos_core::{
    FORMAT, Issue, Iteration, IterationState, Problem, Project, State, Task, Taskbag,
};
use allomos_store::handoff;
use allomos_store::{
    Actor, ConfirmExtra, GUIDE_TOPICS, LOCALIZED_TOPICS, Lock, MAX_PLAN_BYTES,
    add_agent_instructions, agent_section, agent_section_for, agent_setup_request, default_root,
    events, find_taskbag, guide, guide_topic, id_from_name, init_with_labels, load, localized,
    pick_ui_language, picked_folder, plain_path, post_confirm, project_folder_of, quota_ask,
    quota_reports, read_answer, read_text, requests_dir, saved_ui_language, task_language,
    taskbag_of, tidy, versions, was_seen, with_written_files, withdraw_confirm,
};
use clap::{Parser, Subcommand, error::ErrorKind};
use serde_json::{Value, json};

/// The version of the `--json` output. A new field is an addition; anything else changes it.
pub const SCHEMA: &str = "alm/1";

/// Exit codes.
pub const EXIT_OK: i32 = 0;
pub const EXIT_REFUSED: i32 = 1;
pub const EXIT_USAGE: i32 = 2;
pub const EXIT_BUSY: i32 = 3;
/// The taskbag was written by a newer Allomos: the person chooses what happens (Task-A01-25).
pub const EXIT_NEWER: i32 = 4;

/// How long a writer waits for another one to finish, unless `ALLOMOS_LOCK_WAIT_MS` says otherwise.
const LOCK_WAIT_MS: u64 = 5000;

#[derive(Parser)]
#[command(
    name = "alm",
    version,
    about = "Work an Allomos taskbag's tasks from a terminal.",
    after_help = "Exit codes: 0 done, 1 refused by a rule, 2 wrong usage, 3 taskbag busy or not writable, 4 the taskbag needs a newer Allomos."
)]
struct Cli {
    /// The taskbag folder, or a folder holding .allomos/taskbag/. Default: search upward from here.
    #[arg(long, global = true, env = "ALLOMOS_TASKBAG", value_name = "PATH")]
    taskbag: Option<PathBuf>,
    /// Who you are, for example claude/a01-03-alm.
    #[arg(
        long = "as",
        global = true,
        env = "ALLOMOS_HOLDER",
        value_name = "NAME"
    )]
    holder: Option<String>,
    /// Print one JSON result on standard output, and nothing else.
    #[arg(long, global = true)]
    json: bool,
    /// A one-time number of your own for this change (Task-A01-58). Run the same command again
    /// with the same number (after a crash, say) and alm returns the first result instead of
    /// doing it twice; the same number with a different command is refused.
    #[arg(long, global = true, value_name = "ID")]
    request_id: Option<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create an empty taskbag in this folder's .allomos/taskbag/ (or where --taskbag says).
    Init {
        /// The project's name. Default: the folder's name.
        #[arg(long)]
        name: Option<String>,
        /// The project's ID. Default: made from the folder's name.
        #[arg(long)]
        id: Option<String>,
        /// The label ladder, lowest step first, separated by commas.
        /// Default: implemented, locally verified, deployed and verified, human verified.
        #[arg(long, value_name = "A,B,…")]
        labels: Option<String>,
        /// Write Allomos's section for AI agents into AGENTS.md and CLAUDE.md yourself,
        /// instead of printing a request for the agent running `alm init` to do it.
        #[arg(long)]
        write_agent_files: bool,
    },
    /// Add an area or a task.
    #[command(subcommand)]
    Add(Add),
    /// Rename an area (`alm rename area <id> "<name>"`; its ID stays), give an iteration a new ID
    /// with every task planned in it (`alm rename iteration <old> <new>`), or rename a label of the
    /// ladder with every task that carries it (`alm rename label "<old>" "<new>"`).
    #[command(subcommand)]
    Rename(Rename),
    /// Change an iteration: `alm change iteration <id> [--name] [--release] [--start] [--end]
    /// [--state planned|active|closed]`.
    #[command(subcommand)]
    Change(ChangeWhat),
    /// Move an iteration to another place in the plan: `alm move iteration <id> --before <id>`
    /// (or `--after <id>`). Only the order changes.
    #[command(subcommand)]
    Move(MoveWhat),
    /// Delete an area or an iteration; refused while any task is in it, done and dropped ones too.
    #[command(subcommand)]
    Delete(Delete),
    /// Print Allomos's section for this project's AGENTS.md: the short brief agents read first.
    Brief,
    /// Report how much of your account's quota is left, at the handoff gate's quota check, from
    /// the figure you just read (for example Claude Code's get_usage). The Allomos app shows it
    /// beside the agent. Kept on this computer, not in the taskbag: nothing to save or share.
    /// Without --agent, it lists the reports.
    Quota {
        /// Your agent's id: claude-code, codex, or another agent's (not your session name; a
        /// word starting with claude or codex, or a session name claude/…, is read as that agent).
        #[arg(long, requires = "left")]
        agent: Option<String>,
        /// A name for the account, only when you can tell one; never an email address or user
        /// name.
        #[arg(long, requires = "agent")]
        account: Option<String>,
        /// The share left, 0 to 100.
        #[arg(long, value_name = "PERCENT", requires = "agent")]
        left: Option<u8>,
        /// When this window resets, local time (2026-10-20T17:00).
        #[arg(long, value_name = "WHEN", requires = "agent")]
        resets: Option<String>,
        /// Which limit the figure is for: 5h (the handoff gate's) or week.
        #[arg(long, default_value = "5h", requires = "agent")]
        window: String,
    },
    /// Print the details the brief points to: terms, what a project may customize, commands,
    /// how to work the tasks, the run modes to show the person before an automatic or loop run,
    /// the one-time procedure that puts every agent's shared rules in AGENTS.md, and the goal
    /// prompt that builds an app from a description, and how to publish a GitHub release and install
    /// it. All topics (in English) when none is named. A topic may be written as separate words
    /// (`alm guide github release`); words after the topic are the guide's, for the agent to read.
    Guide {
        /// The topic, as one word or several (joined with `-`), then any words the guide defines.
        words: Vec<String>,
        /// For `run-modes`, `agent-files`, `goal` and `github-release`: the language to print it in
        /// (default: the task language).
        #[arg(long, value_name = "CODE")]
        language: Option<String>,
    },
    /// The language to write new and changed tasks in. Without a value: show it (run this before
    /// adding a task or changing its title or description: `alm title`, `alm describe`). With one: set it, to a language code
    /// such as zh-CN, or to ui to follow the Allomos app's language.
    #[command(name = "task-language", alias = "task_language")]
    TaskLanguage {
        #[arg(value_name = "CODE|ui")]
        value: Option<String>,
    },
    /// Set the project's label ladder, lowest step first, separated by commas. Refused when a
    /// task's label would no longer be on it.
    Labels {
        #[arg(value_name = "A,B,…")]
        ladder: String,
    },
    /// Show a plan to the person in the Allomos app and wait for their answer: exit 0 means
    /// go ahead; exit 1 (confirm_declined, confirm_timeout) means change nothing.
    Confirm {
        /// The popup's heading.
        #[arg(long)]
        title: String,
        /// A Markdown file with the plan to show.
        #[arg(long, value_name = "FILE")]
        plan: PathBuf,
        /// How long to wait for the answer, in seconds.
        #[arg(long, default_value_t = 600, value_name = "SECONDS")]
        timeout: u64,
        /// A Markdown file with the evidence, shown under the plan: what was checked and the
        /// results.
        #[arg(long, value_name = "FILE")]
        evidence: Option<PathBuf>,
        /// The task the approval is for: a Go ahead is recorded in its history, for exactly the
        /// plan shown. If the plan changes afterwards, ask again.
        #[arg(long, value_name = "ID")]
        task: Option<String>,
    },
    /// Tell the person in the Allomos app what you are doing on a job it gave you: the app shows
    /// a progress popup. Start with --title and --step, report each step, and end with --finished.
    Progress {
        /// The job (needed to start one; later reports keep it).
        #[arg(long)]
        title: Option<String>,
        /// What you are doing now.
        #[arg(long)]
        step: Option<String>,
        /// Steps done so far, of --of.
        #[arg(long, value_name = "N")]
        done: Option<u32>,
        /// All steps, when you know the count.
        #[arg(long, value_name = "N")]
        of: Option<u32>,
        /// The job is over: what happened.
        #[arg(long, value_name = "TEXT")]
        finished: Option<String>,
    },
    /// List tasks, grouped by area (or by iteration with --by iteration).
    List {
        #[arg(long)]
        area: Option<String>,
        /// Only tasks planned in this iteration.
        #[arg(long)]
        iteration: Option<String>,
        /// Group by area (the default) or by iteration; tasks with no iteration come last, as unplanned.
        #[arg(long, value_parser = ["area", "iteration"], default_value = "area")]
        by: String,
        #[arg(long, value_parser = State::ALL.map(State::as_str))]
        state: Option<String>,
        /// Only tasks you hold.
        #[arg(long)]
        mine: bool,
        /// Only tasks finished without the human check and not checked since
        /// (`alm done --without-human-check`). `--without-owner` is its old name.
        #[arg(long, alias = "without-owner")]
        without_human_check: bool,
    },
    /// What needs a person: To check (tasks finished without the human check, each with why)
    /// and To answer (tasks waiting on a person, each with what). Backlog tasks are not listed.
    Human {
        /// Only tasks planned in this iteration.
        #[arg(long)]
        iteration: Option<String>,
        /// Only tasks in this area.
        #[arg(long)]
        area: Option<String>,
        /// Under each task to check, the steps for checking it.
        #[arg(long)]
        steps: bool,
    },
    /// A person has checked a finished task: it gets the top label and leaves To check; the note
    /// and the steps stay in its history. Run it only when the person says so.
    Checked { id: String },
    /// Send a finished task back to To check: its work changed after a person checked it, so
    /// the check no longer covers it. You decide when; say what changed. A task at the top label
    /// goes one step down.
    Recheck {
        id: String,
        /// What changed since the person checked it, in one line. Shown under To check.
        #[arg(long, value_name = "TEXT")]
        reason: String,
        /// A Markdown file with new steps for checking it (otherwise the earlier steps stay).
        #[arg(long, value_name = "FILE")]
        steps: Option<PathBuf>,
    },
    /// Record a decision you made on your own in a task (a routine choice: a name, a file layout,
    /// a small detail with an obvious default): what you decided and why, in one line. It is kept
    /// in the task's history; `alm human` lists it under To review: AI decisions until a person
    /// has reviewed it.
    #[command(name = "ai-decided")]
    AiDecided {
        id: String,
        #[arg(value_name = "WHAT AND WHY")]
        text: String,
    },
    /// A person has reviewed the task's AI decisions: they leave To review; the history keeps
    /// them. Run it only when the person says so.
    Reviewed { id: String },
    /// Show one task in full.
    Show { id: String },
    /// Show a task's history: every recorded change, newest first, with who and when.
    History { id: String },
    /// Hold a task and start it. Say which branch you are on (--branch): alm records the task's
    /// working branch and says which base to start from and merge into, with the Git commands as the
    /// example.
    Claim {
        id: String,
        /// Take the task over from its current holder.
        #[arg(long)]
        take: bool,
        /// The branch you are on, as your version control tool shows it (in Git:
        /// `git branch --show-current`; alm reads no version control). A task without a
        /// base gets it.
        #[arg(long, value_name = "NAME")]
        branch: Option<String>,
        /// The branch you will do the work on, when it is not the one you are on (for example a
        /// new branch for its own worktree).
        #[arg(long, value_name = "NAME")]
        working: Option<String>,
    },
    /// Change the branch a task starts from and merges into (its base), by its real name.
    Base {
        id: String,
        #[arg(value_name = "BRANCH")]
        branch: String,
    },
    /// Set a task's label (a step of the project's ladder).
    Label { id: String, label: String },
    /// Park a task as waiting: a person (or an outside event) must act. Say what, and who when it
    /// is not the approver.
    Wait {
        id: String,
        #[arg(long, value_name = "TEXT")]
        on: String,
        /// Who must act, when it is not the approver (for example Ann).
        #[arg(long, value_name = "NAME")]
        who: Option<String>,
    },
    /// Mark a task blocked: other tasks in this taskbag must finish first.
    Block {
        id: String,
        /// The blocking tasks' IDs (several, or separated by commas).
        #[arg(long, value_name = "TASK-ID", num_args = 1.., required = true)]
        by: Vec<String>,
    },
    /// Move a blocked task on (ready, or in progress when someone holds it). `alm` does not check
    /// its blockers: you do.
    Unblock { id: String },
    /// Finish a task. Finish it at the top label (a person has checked it); when only a person's
    /// hands-on check is left, in any run mode, finish it below it with --without-human-check and
    /// --steps. On a task already done, only its human check is recorded again.
    Done {
        id: String,
        /// Why the task is finished without the human check. Recorded in the task's file; `alm human` lists it under To check.
        /// `--without-owner` is its old name.
        #[arg(long, value_name = "TEXT", alias = "without-owner")]
        without_human_check: Option<String>,
        /// A Markdown file with the steps for the person who checks it: the test folder, what to
        /// open, what to click, what they should see.
        #[arg(long, value_name = "FILE", requires = "without_human_check")]
        steps: Option<PathBuf>,
    },
    /// Let go of a task, optionally leaving a handoff note for the next one.
    Release {
        id: String,
        #[arg(long, value_name = "TEXT")]
        note: Option<String>,
    },
    /// Set the task's next dependency (the one thing needed for its next label); no text clears it.
    Needs { id: String, text: Option<String> },
    /// Plan a task in an iteration; no iteration takes it out (it becomes unplanned).
    Plan {
        id: String,
        iteration: Option<String>,
    },
    /// Mark an unheld backlog, waiting or blocked task ready (for example, the approver approved it).
    Ready { id: String },
    /// Give a task a new title.
    Title {
        id: String,
        title: String,
        /// The version you read (`alm show`): if the task changed since, nothing is saved.
        #[arg(long, value_name = "VERSION")]
        expected_version: Option<String>,
    },
    /// Give a task a new description (its description.md): `--text "<markdown>"`, or `--from` a
    /// Markdown file. An empty text (`--text ""`) removes it.
    Describe {
        id: String,
        /// The new description, in Markdown.
        #[arg(
            long,
            value_name = "TEXT",
            required_unless_present = "from",
            conflicts_with = "from"
        )]
        text: Option<String>,
        /// A Markdown file with the new description.
        #[arg(long, value_name = "FILE")]
        from: Option<PathBuf>,
        /// The version you read (`alm show`): if the task changed since, nothing is saved.
        #[arg(long, value_name = "VERSION")]
        expected_version: Option<String>,
    },
    /// Set the task's handoff note (what the next one needs); no text clears it.
    Note { id: String, text: Option<String> },
    /// Card files still outside the taskbag (linked from a task's description, or reported);
    /// `alm cards found <id>=<path>…` reports others you found (see `alm guide move-cards`).
    Cards {
        #[command(subcommand)]
        what: Option<CardsWhat>,
    },
    /// Move a task's card file in beside it, as tasks/<id>/card.md, and point the description's
    /// `Card:` link to it. Without --from: the file reported with `alm cards found`, else the one
    /// the `Card:` link points to.
    Card {
        id: String,
        /// The card's text (for example a copy whose links you fixed). Default: the reported file.
        #[arg(long, value_name = "FILE")]
        from: Option<PathBuf>,
        /// Replace a different card.md already there (to keep a moved card up to date).
        #[arg(long)]
        replace: bool,
    },
    /// Change the project's name and/or ID; without either, show them.
    Project {
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        id: Option<String>,
    },
    /// Drop a task: it will not be done.
    Drop { id: String },
    /// Reopen a done or dropped task: it becomes ready, held by nobody, with why in its history.
    /// Its label and human-check record are cleared (both stay in the history).
    Reopen {
        id: String,
        /// Why it is reopened: the more work on the same outcome.
        #[arg(long, value_name = "TEXT")]
        note: String,
    },
    /// List the ready tasks that no one holds, with the rule for choosing one.
    Next {
        /// Only tasks planned in this iteration.
        #[arg(long)]
        iteration: Option<String>,
    },
    /// Print the request the app's Update the taskbag… gives an AI agent: for a taskbag written by
    /// an older Allomos, moving its tasks into a new taskbag, like another task system; otherwise,
    /// moving card files kept outside the taskbag in beside their tasks. Changes nothing.
    #[command(name = "move-in")]
    MoveIn {
        /// The language of the request (en, zh-CN, ja). Default: the Allomos app's language.
        #[arg(long, value_name = "CODE")]
        language: Option<String>,
    },
    /// When to write a handoff: the quota and context gates (with any override for you, --as).
    /// With options, change them.
    #[command(name = "handoff-rules")]
    HandoffRules {
        /// Quota gate: with this percentage of the 5-hour quota left, write the handoff and stop.
        #[arg(long, value_name = "PERCENT")]
        quota: Option<String>,
        /// Context gate: at this percentage of context used, write the handoff and hand over.
        #[arg(long, value_name = "PERCENT")]
        context: Option<String>,
        /// When else to write a handoff, in plain words.
        #[arg(long, value_name = "TEXT")]
        write_when: Option<String>,
        /// Change this agent's or account's own gates (for example codex) instead of the project's.
        #[arg(long, value_name = "NAME")]
        agent: Option<String>,
        /// Back to the defaults (with --agent: remove that agent's own gates).
        #[arg(long)]
        clear: bool,
    },
    /// Hand a task off (--as your session): its handoff note becomes this text (where it stopped,
    /// the one next step, who it waits on), --evidence is added under its description's Evidence,
    /// and its history records who handed it off. Several tasks: once for each.
    Handoff {
        id: String,
        /// The handoff note's text.
        #[arg(
            long,
            value_name = "TEXT",
            conflicts_with = "from",
            required_unless_present = "from"
        )]
        text: Option<String>,
        /// A Markdown file with the handoff note's text.
        #[arg(long, value_name = "FILE")]
        from: Option<PathBuf>,
        /// What was done that must not run twice (an outside effect), for the description's Evidence.
        #[arg(long, value_name = "TEXT")]
        evidence: Option<String>,
    },
}

#[derive(Subcommand)]
enum CardsWhat {
    /// Report card files you found in the project folder: <id>=<path> (relative to the project
    /// folder), several at once; <id>= withdraws a report.
    Found {
        #[arg(value_name = "ID=PATH", required = true)]
        items: Vec<String>,
    },
}

#[derive(Subcommand)]
enum Add {
    /// Add an area: `alm add area [<id>] <name>`. With only a name, the ID is the next free one,
    /// like A08 after A07.
    Area {
        /// The area's ID, then its name; or only its name.
        #[arg(value_name = "ID")]
        first: String,
        #[arg(value_name = "NAME")]
        name: Option<String>,
    },
    /// Add an iteration (when tasks are planned), ordered after the existing ones:
    /// `alm add iteration [<id>] <name>`. With only a name, the ID is the next free one, like I16
    /// after I15.
    Iteration {
        /// The iteration's ID, then its name; or only its name.
        #[arg(value_name = "ID")]
        first: String,
        #[arg(value_name = "NAME")]
        name: Option<String>,
        /// The release it produces, for example v0.2.
        #[arg(long)]
        release: Option<String>,
        /// First day, YYYY-MM-DD.
        #[arg(long, value_name = "DATE")]
        start: Option<String>,
        /// Last day, YYYY-MM-DD.
        #[arg(long, value_name = "DATE")]
        end: Option<String>,
    },
    /// Add a task in backlog.
    Task {
        /// The task's ID. Default: the next free one in the area, like A01-04.
        id: Option<String>,
        #[arg(long)]
        area: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        description: Option<String>,
        /// The branch you are on, as your version control tool shows it (in Git:
        /// `git branch --show-current`; alm reads no version control). When it is a
        /// task's working branch, the new task gets that task's base; otherwise this branch.
        #[arg(long, value_name = "NAME")]
        branch: Option<String>,
        /// The base branch to use instead of the one --branch chooses.
        #[arg(long, value_name = "NAME")]
        base: Option<String>,
        /// The iteration to plan it in. Default: the active iteration, if one is
        /// (Task-A01-root343); otherwise none.
        #[arg(long, value_name = "ID")]
        iteration: Option<String>,
    },
}

#[derive(Subcommand)]
enum Rename {
    /// Give an area a new name; its ID stays.
    Area { id: String, name: String },
    /// Give an iteration a new ID, for example `I-knit` (Task-A00-rope803). Every task planned in
    /// it, done and dropped ones too, moves along; its name, release, dates, state and order stay.
    Iteration { old: String, new: String },
    /// Rename a step of the label ladder, and the label of every task that carries it (done and
    /// dropped ones too). Its place on the ladder stays.
    Label { old: String, new: String },
}

#[derive(Subcommand)]
enum ChangeWhat {
    /// Change an iteration's name, release, dates or state; its ID stays. What is not given stays
    /// as it is; an empty release or date ("") clears it. `--state`: planned, active or closed; at
    /// most one iteration is active.
    Iteration {
        id: String,
        /// planned, active or closed (Task-A01-root343).
        #[arg(long, value_name = "STATE")]
        state: Option<String>,
        #[arg(long)]
        name: Option<String>,
        /// The release it produces, for example v0.2.
        #[arg(long)]
        release: Option<String>,
        /// First day, YYYY-MM-DD.
        #[arg(long, value_name = "DATE")]
        start: Option<String>,
        /// Last day, YYYY-MM-DD.
        #[arg(long, value_name = "DATE")]
        end: Option<String>,
    },
}

#[derive(Subcommand)]
enum MoveWhat {
    /// Put an iteration just before or just after another one (Task-A00-skate868). Its ID, name,
    /// release, dates and state stay; the iterations are numbered again in the new order.
    #[command(group(clap::ArgGroup::new("place").required(true).args(["before", "after"])))]
    Iteration {
        id: String,
        /// The iteration it goes just before.
        #[arg(long, value_name = "ID")]
        before: Option<String>,
        /// The iteration it goes just after.
        #[arg(long, value_name = "ID")]
        after: Option<String>,
    },
}

#[derive(Subcommand)]
enum Delete {
    /// Delete an area. Refused while any task is in it, done and dropped ones included.
    Area { id: String },
    /// Delete an iteration. Refused while any task is planned in it, done and dropped ones
    /// included: plan them elsewhere first.
    Iteration { id: String },
}

/// `alm add area|iteration [<id>] <name>`: the ID given, if any, and the name.
fn id_and_name<'a>(first: &'a str, name: &'a Option<String>) -> (Option<&'a str>, &'a str) {
    match name {
        Some(name) => (Some(first), name),
        None => (None, first),
    }
}

impl Command {
    /// The task or area a command acts on, for the commit message.
    fn target(&self) -> Option<&str> {
        match self {
            Command::Add(Add::Area { first, name } | Add::Iteration { first, name, .. }) => {
                id_and_name(first, name).0
            }
            Command::Rename(Rename::Label { old, .. } | Rename::Iteration { old, .. }) => Some(old),
            Command::Rename(Rename::Area { id, .. })
            | Command::Change(ChangeWhat::Iteration { id, .. })
            | Command::Move(MoveWhat::Iteration { id, .. })
            | Command::Delete(Delete::Area { id } | Delete::Iteration { id }) => Some(id),
            Command::Show { id }
            | Command::History { id }
            | Command::Claim { id, .. }
            | Command::Base { id, .. }
            | Command::Label { id, .. }
            | Command::Wait { id, .. }
            | Command::Block { id, .. }
            | Command::Unblock { id }
            | Command::Done { id, .. }
            | Command::Checked { id }
            | Command::Recheck { id, .. }
            | Command::AiDecided { id, .. }
            | Command::Reviewed { id }
            | Command::Release { id, .. }
            | Command::Needs { id, .. }
            | Command::Plan { id, .. }
            | Command::Ready { id }
            | Command::Title { id, .. }
            | Command::Describe { id, .. }
            | Command::Note { id, .. }
            | Command::Handoff { id, .. }
            | Command::Card { id, .. }
            | Command::Drop { id }
            | Command::Reopen { id, .. } => Some(id),
            Command::Add(Add::Task { id, .. }) => id.as_deref(),
            Command::Init { .. }
            | Command::Labels { .. }
            | Command::Project { .. }
            | Command::TaskLanguage { .. }
            | Command::Brief
            | Command::Quota { .. }
            | Command::Guide { .. }
            | Command::Confirm { .. }
            | Command::Progress { .. }
            | Command::List { .. }
            | Command::Human { .. }
            | Command::MoveIn { .. }
            | Command::HandoffRules { .. }
            | Command::Cards { .. }
            | Command::Next { .. } => None,
        }
    }

    /// Whether the command writes to the taskbag.
    fn changes(&self) -> bool {
        if let Command::Confirm { task, .. } = self {
            return task.is_some();
        }
        if let Command::Project { name, id } = self {
            return name.is_some() || id.is_some();
        }
        if let Command::TaskLanguage { value } = self {
            return value.is_some();
        }
        if let Command::HandoffRules {
            quota,
            context,
            write_when,
            clear,
            ..
        } = self
        {
            return quota.is_some() || context.is_some() || write_when.is_some() || *clear;
        }
        if let Command::Cards { what } = self {
            return what.is_some();
        }
        !matches!(
            self,
            Command::List { .. }
                | Command::Human { .. }
                | Command::Show { .. }
                | Command::History { .. }
                | Command::Next { .. }
                | Command::Confirm { .. }
                | Command::Progress { .. }
                | Command::Brief
                | Command::Quota { .. }
                | Command::Guide { .. }
                | Command::MoveIn { .. }
        )
    }

    fn name(&self) -> &'static str {
        match self {
            Command::Init { .. } => "init",
            Command::Confirm { .. } => "confirm",
            Command::Progress { .. } => "progress",
            Command::Labels { .. } => "labels",
            Command::TaskLanguage { .. } => "task-language",
            Command::Brief => "brief",
            Command::Quota { .. } => "quota",
            Command::Guide { .. } => "guide",
            Command::Add(Add::Area { .. }) => "add area",
            Command::Add(Add::Iteration { .. }) => "add iteration",
            Command::Add(Add::Task { .. }) => "add task",
            Command::Rename(Rename::Area { .. }) => "rename area",
            Command::Rename(Rename::Label { .. }) => "rename label",
            Command::Rename(Rename::Iteration { .. }) => "rename iteration",
            Command::Change(ChangeWhat::Iteration { .. }) => "change iteration",
            Command::Move(MoveWhat::Iteration { .. }) => "move iteration",
            Command::Delete(Delete::Area { .. }) => "delete area",
            Command::Delete(Delete::Iteration { .. }) => "delete iteration",
            Command::List { .. } => "list",
            Command::Show { .. } => "show",
            Command::History { .. } => "history",
            Command::Claim { .. } => "claim",
            Command::Base { .. } => "base",
            Command::Label { .. } => "label",
            Command::Wait { .. } => "wait",
            Command::Block { .. } => "block",
            Command::Unblock { .. } => "unblock",
            Command::Done { .. } => "done",
            Command::Checked { .. } => "checked",
            Command::Recheck { .. } => "recheck",
            Command::AiDecided { .. } => "ai-decided",
            Command::Reviewed { .. } => "reviewed",
            Command::Human { .. } => "human",
            Command::Release { .. } => "release",
            Command::Needs { .. } => "needs",
            Command::Plan { .. } => "plan",
            Command::Ready { .. } => "ready",
            Command::Title { .. } => "title",
            Command::Describe { .. } => "describe",
            Command::Note { .. } => "note",
            Command::Cards { what: None } => "cards",
            Command::Cards {
                what: Some(CardsWhat::Found { .. }),
            } => "cards found",
            Command::Card { .. } => "card",
            Command::Project { .. } => "project",
            Command::Drop { .. } => "drop",
            Command::Reopen { .. } => "reopen",
            Command::Next { .. } => "next",
            Command::MoveIn { .. } => "move-in",
            Command::HandoffRules { .. } => "handoff-rules",
            Command::Handoff { .. } => "handoff",
        }
    }
}

/// What a command produced: fields for the JSON result, the lines for people, and warnings.
struct Done {
    fields: Value,
    text: String,
    /// Printed after the Git commands: what to do next (Task-A03-33).
    after: String,
    warnings: Vec<Problem>,
    issues: Vec<Issue>,
}

impl Done {
    fn new(fields: Value, text: String) -> Self {
        Done {
            fields,
            text,
            after: String::new(),
            warnings: Vec::new(),
            issues: Vec::new(),
        }
    }
}

pub fn exit_code(problem: &Problem) -> i32 {
    match problem {
        Problem::TaskbagNewer { .. } => EXIT_NEWER,
        Problem::BadUsage { .. } => EXIT_USAGE,
        Problem::TaskbagBusy { .. }
        | Problem::WriteFailed { .. }
        | Problem::TaskbagUnreadable { .. }
        | Problem::CreateFailed { .. }
        | Problem::ReadFailed { .. } => EXIT_BUSY,
        _ => EXIT_REFUSED,
    }
}

fn problem_json(p: &Problem) -> Value {
    let mut v = serde_json::to_value(p).expect("problems serialize");
    v["message"] = Value::String(p.english());
    v
}

fn issue_json(i: &Issue) -> Value {
    json!({ "file": i.file, "field": i.field, "problem": problem_json(&i.problem) })
}

/// Run `alm` with these arguments from `cwd`; print the result and return the exit code.
pub fn run(args: Vec<OsString>, cwd: &Path) -> i32 {
    let reply = reply(args, cwd);
    print!("{}", reply.stdout);
    eprint!("{}", reply.stderr);
    reply.code
}

/// What one run of `alm` printed, and its exit code.
struct Reply {
    code: i32,
    stdout: String,
    stderr: String,
}

/// Run `alm` and collect what it would print. Help, version and usage errors without `--json` are
/// printed by `clap`.
/// The input a one-time number stands for (Task-A01-58): the command's words and who runs it,
/// without the number itself or `--json` (how the result is printed is not the change).
fn request_input(args: &[OsString], holder: Option<&str>) -> String {
    let mut words = Vec::new();
    let mut skip = false;
    for a in args.iter().skip(1) {
        let a = a.to_string_lossy();
        if skip {
            skip = false;
        } else if a == "--request-id" {
            skip = true;
        } else if !(a.starts_with("--request-id=") || a == "--json") {
            words.push(a.into_owned());
        }
    }
    allomos_core::request::fingerprint(&format!("{}\n{}", holder.unwrap_or(""), words.join("\n")))
}

/// A change run again under a one-time number it already used (Task-A01-58): the first run's
/// result, and nothing changed; or `request_reused` when the number went with another input.
/// `None` when the number is new (or the command changes nothing).
fn repeated(cli: &Cli, cwd: &Path, id: &str, input: &str) -> Option<Result<Done, Problem>> {
    if !cli.command.changes() {
        return None;
    }
    let root = locate(cli, cwd).ok()?;
    let (events, _) = allomos_store::events::list(&root);
    let first = allomos_core::request::made_under(&events, id);
    let (_, was) = allomos_core::request::of(first.first()?)?;
    if was != input {
        return Some(Err(Problem::RequestReused { id: id.to_string() }));
    }
    let what: Vec<String> = first
        .iter()
        .map(|e| match &e.id {
            Some(item) => format!("{} {item}", e.command),
            None => e.command.clone(),
        })
        .collect();
    let text = format!(
        "Done already under the one-time number {id} ({}): {}. Nothing changed.\n",
        first[0].at,
        what.join(", ")
    );
    Some(Ok(Done::new(
        json!({ "taskbag": root, "repeated": true, "request_id": id, "first": allomos_core::request::summary(&first) }),
        text,
    )))
}

fn reply(args: Vec<OsString>, cwd: &Path) -> Reply {
    let wants_json = args.iter().any(|a| a == "--json");
    let raw_args = args.clone();
    let mut stdout = String::new();
    let mut stderr = String::new();
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(e) => {
            if matches!(e.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) {
                let _ = e.print();
                return Reply {
                    code: EXIT_OK,
                    stdout,
                    stderr,
                };
            }
            let rendered = e.render().to_string();
            let detail = rendered
                .lines()
                .next()
                .unwrap_or("wrong usage")
                .trim_start_matches("error: ")
                .to_string();
            let problem = Problem::BadUsage { detail };
            if wants_json {
                stdout.push_str(&error_line(None, &problem));
            } else {
                let _ = e.print();
            }
            return Reply {
                code: EXIT_USAGE,
                stdout,
                stderr,
            };
        }
    };
    let command = cli.command.name();
    // Before the change: which changes made in the app has no agent been told to share yet
    // (decision 0017, row 9)?
    let before = Before::look(&cli, cwd);
    // A one-time number (Task-A01-58): a repeat returns the first result; a new number marks
    // every event this change writes.
    let request = cli
        .request_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(|id| {
            (
                id.to_string(),
                request_input(&raw_args, cli.holder.as_deref()),
            )
        });
    let (result, written) = with_written_files(|| {
        check_versions(&cli, cwd).and_then(|outdated| {
            if let Some((id, input)) = &request {
                if let Some(first) = repeated(&cli, cwd, id, input) {
                    return first;
                }
                allomos_store::events::set_request(Some((id, input)));
            }
            let done = execute(&cli, cwd);
            allomos_store::events::set_request(None);
            let mut done = done?;
            done.warnings.extend(outdated);
            pack_history(&cli, cwd, &mut done);
            Ok(done)
        })
    });
    let code = match result {
        Ok(mut done) => {
            share(&cli, &before, &written, &mut done);
            // A command that moves a task on asks for a fresh quota report when the agent's is
            // missing or old (Task-A03-flour054): alm tells; the agent reads its usage.
            let moves_on = matches!(
                cli.command,
                Command::Claim { .. }
                    | Command::Label { .. }
                    | Command::Release { .. }
                    | Command::Done { .. }
                    | Command::Handoff { .. }
            );
            if moves_on {
                if let Some(ask) =
                    quota_ask(cli.holder.as_deref(), &quota_reports(), &events::now_utc())
                {
                    done.text.push('\n');
                    done.text.push_str(&ask.text());
                    done.fields["quota_ask"] = json!(ask);
                }
            }
            if cli.json {
                let mut out = json!({ "schema": SCHEMA, "ok": true, "command": command });
                if let (Some(out), Some(fields)) = (out.as_object_mut(), done.fields.as_object()) {
                    out.extend(fields.clone());
                }
                out["warnings"] = done.warnings.iter().map(problem_json).collect();
                out["issues"] = done.issues.iter().map(issue_json).collect();
                stdout.push_str(&format!("{out}\n"));
            } else {
                stdout.push_str(&done.text);
                for w in &done.warnings {
                    stderr.push_str(&format!("warning [{}]: {}\n", w.code(), w.english()));
                }
                if !done.issues.is_empty() {
                    stderr.push_str(&format!(
                        "note: {} problem(s) in the taskbag's files; see `alm list --json` or the app.\n",
                        done.issues.len()
                    ));
                }
            }
            EXIT_OK
        }
        Err(problem) => {
            // A save from an out-of-date copy (Task-A01-57): the task as it is now goes with
            // the refusal, so the agent can compare and decide without another read.
            let current = match &problem {
                Problem::TaskChanged { id, .. } => locate(&cli, cwd)
                    .ok()
                    .and_then(|root| load(&root).taskbag)
                    .and_then(|bag| ops::find_task(&bag, id).ok()),
                _ => None,
            };
            if cli.json {
                let mut line: Value = serde_json::from_str(&error_line(Some(command), &problem))
                    .expect("error lines are JSON");
                if let Some(task) = &current {
                    line["current"] = task_json(task);
                }
                stdout.push_str(&format!("{line}\n"));
            } else {
                stderr.push_str(&format!(
                    "error [{}]: {}\n",
                    problem.code(),
                    problem.english()
                ));
                if let Some(task) = &current {
                    stderr.push_str(&format!("Now:\n  title: {}\n", task.title));
                    if task.description.is_empty() {
                        stderr.push_str("  description: (none)\n");
                    } else {
                        stderr.push_str(&format!("  description:\n{}", indent(&task.description)));
                    }
                }
            }
            exit_code(&problem)
        }
    };
    Reply {
        code,
        stdout,
        stderr,
    }
}

/// After a command that worked, from the 28th of a month (Task-A01-59, approver 2026-10-20: alm
/// packs by itself): pack the history of whole earlier months into one file each. What was
/// packed is said, and the Git commands that share the change include the packed and removed
/// files. A taskbag busy with another writer is left for the next command.
fn pack_history(cli: &Cli, cwd: &Path, done: &mut Done) {
    if matches!(
        cli.command,
        Command::Init { .. }
            | Command::Brief
            | Command::Quota { .. }
            | Command::Guide { .. }
            | Command::Confirm { .. }
            | Command::MoveIn { .. }
    ) {
        return;
    }
    let Ok(root) = locate(cli, cwd) else {
        return;
    };
    let Ok(lock) = Lock::acquire(&root, Duration::ZERO) else {
        return;
    };
    let packed = lock.pack_history().unwrap_or_default();
    drop(lock);
    if packed.is_empty() {
        return;
    }
    let mut text = String::new();
    for p in &packed {
        text.push_str(&format!(
            "Packed the history of {} into {} ({} changes; {} single files removed).\n",
            p.month, p.file, p.changes, p.removed
        ));
    }
    done.text.push_str(&text);
    if done.fields["taskbag"].is_null() {
        done.fields["taskbag"] = json!(root);
    }
    done.fields["packed"] = json!(packed);
}

/// Before a command: is the taskbag in the format this Allomos reads, and is Allomos's section in
/// the project's instruction files current (Task-A01-25)? A newer taskbag stops every command
/// (`taskbag_newer`), and so does one from an older Allomos (`taskbag_to_move_in`, Task-A01-45:
/// nothing is read or changed; its tasks are moved into a new taskbag). An out-of-date section
/// gives a warning telling the agent to renew it. Commands that do not work on a taskbag (`init`,
/// `brief`, `guide`, `confirm`) and `move-in`, which prints the request for an older one, are not
/// checked.
fn check_versions(cli: &Cli, cwd: &Path) -> Result<Vec<Problem>, Problem> {
    if matches!(
        cli.command,
        Command::Init { .. }
            | Command::Brief
            | Command::Quota { .. }
            | Command::Guide { .. }
            | Command::Confirm { .. }
            | Command::MoveIn { .. }
    ) {
        return Ok(Vec::new());
    }
    // No taskbag here: the command itself says so.
    let Ok(root) = locate(cli, cwd) else {
        return Ok(Vec::new());
    };
    versions::write_allowed(&root)?;
    Ok(versions::check_connected(&root)
        .and_then(|c| c.instructions_warning())
        .into_iter()
        .collect())
}

/// What `alm` knows before a change, for the Git commands it prints after it.
struct Before {
    /// Changes made in the app that no agent has been told to share yet.
    app: Option<events::AppChanges>,
}

impl Before {
    fn look(cli: &Cli, cwd: &Path) -> Before {
        let telling = (cli.command.changes() && !matches!(cli.command, Command::Init { .. }))
            || matches!(cli.command, Command::Next { .. });
        let app = telling
            .then(|| locate(cli, cwd).ok())
            .flatten()
            .map(|r| events::unshared_app_changes(&r))
            .filter(|a| a.count > 0);
        Before { app }
    }
}

/// One argument of a printed command: as it is when it is plain, otherwise in double quotes.
fn quoted(arg: &str) -> String {
    let plain = !arg.is_empty()
        && arg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-/@+=,:".contains(c));
    if plain {
        arg.to_string()
    } else {
        format!("\"{}\"", arg.replace('"', "\\\""))
    }
}

/// `git add`, `git commit` (with the two message lines) and, unless `push` is false, `git push`,
/// run in `taskbag` and naming exactly `files` (relative to it).
fn git_commands(
    taskbag: &Path,
    files: &[String],
    subject: &str,
    who: &str,
    push: bool,
) -> Vec<String> {
    let at = format!(
        "git -C {}",
        quoted(&plain_path(taskbag).display().to_string())
    );
    let list: Vec<String> = files.iter().map(|f| quoted(f)).collect();
    let list = list.join(" ");
    let mut out = vec![
        format!("{at} add -- {list}"),
        format!(
            "{at} commit -m {} -m {} -- {list}",
            quoted(subject),
            quoted(who)
        ),
    ];
    if push {
        out.push(format!("{at} push"));
    }
    out
}

/// After a change (decision 0017): `alm` runs no Git; it tells the agent the exact commands that
/// share this change, naming only the files it wrote (`changed_files`, `git_commands`), and
/// any changes made in the app that are not shared yet (`app_changes`), to commit as the
/// person's. `alm init` prints none: no agent is connected yet.
fn share(cli: &Cli, before: &Before, written: &[PathBuf], done: &mut Done) {
    let Some(root) = done.fields["taskbag"].as_str().map(PathBuf::from) else {
        return;
    };
    // A command that packed old history (Task-A01-59) shares that too, even when it only reads.
    let packed = done.fields["packed"]
        .as_array()
        .is_some_and(|p| !p.is_empty());
    let changes = (cli.command.changes() || packed) && !matches!(cli.command, Command::Init { .. });
    let mut text = String::new();
    let mut files: Vec<String> = written
        .iter()
        .filter(|p| p.file_name().is_some_and(|n| n != allomos_store::LOCK_FILE))
        .map(|p| match p.strip_prefix(&root) {
            Ok(rel) => allomos_store::portable_path(rel),
            Err(_) => p.display().to_string(),
        })
        .collect();
    files.sort();
    files.dedup();
    if changes {
        let command = cli.command.name();
        // Added without an ID: name the one it got.
        let made = ["task", "area", "iteration"]
            .iter()
            .find_map(|k| done.fields[*k]["id"].as_str())
            .map(String::from);
        let subject = match cli.command.target().map(String::from).or(made) {
            Some(id) => format!("alm: {command} {id}"),
            None => format!("alm: {command}"),
        };
        let subject = if packed && !cli.command.changes() {
            "alm: pack history".to_string()
        } else {
            subject
        };
        let who = format!(
            "Agent: {}",
            cli.holder.as_deref().unwrap_or("<your session name>")
        );
        // In general words, with the Git commands as the example (Task-A01-60, approver
        // 2026-10-20): Allomos does not assume Git.
        let (instruction, commands) = if files.is_empty() {
            (
                "Nothing changed, so there is nothing to save.".to_string(),
                Vec::new(),
            )
        } else if matches!(cli.command, Command::Claim { .. }) {
            (
                "Save this change in your version control tool and share it now, before you start the work. Until it is shared, other sessions do not see your claim and may take this task too. Changed files:".to_string(),
                git_commands(&root, &files, &subject, &who, true),
            )
        } else {
            (
                "Save this change in your version control tool and share it. Changed files:"
                    .to_string(),
                git_commands(&root, &files, &subject, &who, true),
            )
        };
        text.push('\n');
        text.push_str(&instruction);
        text.push('\n');
        if !commands.is_empty() {
            for f in &files {
                text.push_str(&format!("  {f}\n"));
            }
            text.push_str("For Git:\n");
        }
        for c in &commands {
            text.push_str(&format!("  {c}\n"));
        }
        if !commands.is_empty() {
            text.push_str("Save only these files; leave other changes as they are.\n");
        }
        done.fields["changed_files"] = json!(files);
        done.fields["git_commands"] = json!(commands);
        done.fields["git_instruction"] = json!(instruction);
    }
    if let Some(app) = &before.app {
        // Files this change also wrote are already in its own commit.
        let rest: Vec<String> = app
            .files
            .iter()
            .filter(|f| !(changes && files.contains(f)))
            .cloned()
            .collect();
        if !rest.is_empty() {
            let by = if app.by.is_empty() {
                "the person".to_string()
            } else {
                app.by.join(", ")
            };
            let commands = git_commands(
                &root,
                &rest,
                "app: changes made in the Allomos app",
                &format!("By: {by} (Allomos app)"),
                true,
            );
            text.push_str(&format!(
                "\n{} change(s) made in the Allomos app by {by} are not shared yet. Save and share them in your version control tool as the person's, on their own (if someone already did, there is nothing to save; that is fine). Changed files:\n",
                app.count
            ));
            for f in &rest {
                text.push_str(&format!("  {f}\n"));
            }
            text.push_str("For Git:\n");
            for c in &commands {
                text.push_str(&format!("  {c}\n"));
            }
            done.fields["app_changes"] = json!({
                "count": app.count,
                "by": app.by,
                "files": rest,
                "git_commands": commands,
            });
        }
    }
    done.text.push_str(&text);
    if !done.after.is_empty() {
        let after = std::mem::take(&mut done.after);
        done.text.push('\n');
        done.text.push_str(&after);
    }
}

fn error_line(command: Option<&str>, problem: &Problem) -> String {
    format!(
        "{}\n",
        json!({ "schema": SCHEMA, "ok": false, "command": command, "error": problem_json(problem) })
    )
}

/// The entry point of both programs.
pub fn main() -> i32 {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    run(std::env::args_os().collect(), &cwd)
}

fn lock_wait() -> Duration {
    let ms = std::env::var("ALLOMOS_LOCK_WAIT_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(LOCK_WAIT_MS);
    Duration::from_millis(ms)
}

/// The taskbag to work on: `--taskbag` / `ALLOMOS_TASKBAG` (the taskbag itself, or a folder
/// holding `.allomos/taskbag/`), else the nearest one at or above `cwd`.
fn locate(cli: &Cli, cwd: &Path) -> Result<PathBuf, Problem> {
    let not_found = |p: &Path| Problem::TaskbagNotFound {
        path: p.display().to_string(),
    };
    match &cli.taskbag {
        Some(path) => {
            // Without `.` and `..`, so every path alm prints is tidy (Task-A03-32); as given
            // when tidying it would miss the taskbag (a symbolic link followed by `..`).
            let given = cwd.join(path);
            let tidy = tidy(&given);
            let path = if tidy.join("project.json").is_file()
                || default_root(&tidy).join("project.json").is_file()
            {
                tidy
            } else {
                given
            };
            if path.join("project.json").is_file() {
                Ok(path)
            } else if default_root(&path).join("project.json").is_file() {
                Ok(default_root(&path))
            } else {
                Err(not_found(&path))
            }
        }
        None => find_taskbag(cwd).ok_or_else(|| not_found(cwd)),
    }
}

/// One line saying which language to write tasks in, and why.
fn language_line(l: &allomos_store::TaskLanguage) -> String {
    let why = match l.source {
        "taskbag" => "set in this taskbag".to_string(),
        "app" => "follows the Allomos app's language".to_string(),
        _ => "follows this computer's language; the Allomos app has not saved one yet".to_string(),
    };
    format!(
        "Write new and changed tasks in: {} ({why}). Keep IDs, commands, code and file names as they are.",
        l.language
    )
}

/// The project folder `alm confirm` asks about: the folder holding the taskbag given with
/// `--taskbag` (which may not exist yet), else the nearest taskbag's, else `cwd`.
fn confirm_project(cli: &Cli, cwd: &Path) -> PathBuf {
    let taskbag = match &cli.taskbag {
        Some(path) => cwd.join(path),
        None => find_taskbag(cwd).unwrap_or_else(|| default_root(cwd)),
    };
    picked_folder(&taskbag)
        .map(Path::to_path_buf)
        .unwrap_or(taskbag)
}

/// How often `alm confirm` looks for the answer, and how long it gives an open app to show the
/// question before opening the app itself.
const CONFIRM_POLL: Duration = Duration::from_millis(250);
const CONFIRM_LAUNCH_AFTER: Duration = Duration::from_secs(3);

/// Open the Allomos app on `project`, so it can show the question: on macOS by its bundle ID, on
/// Windows the app beside `alm.exe`, where the installer puts both (Task-A07-06).
/// `ALLOMOS_NO_LAUNCH` turns it off (tests).
fn open_app(project: &Path) {
    if std::env::var_os("ALLOMOS_NO_LAUNCH").is_some() {
        return;
    }
    if cfg!(target_os = "macos") {
        let _ = std::process::Command::new("open")
            .args(["-n", "-b", "io.allomos", "--args"])
            .arg(project)
            .status();
    } else if cfg!(windows) {
        if let Some(app) = windows_app() {
            // Not waited for: the app keeps running after the question is answered.
            let _ = std::process::Command::new(app).arg(project).spawn();
        }
    }
}

/// The Allomos app installed beside this `alm.exe`, if there is one.
fn windows_app() -> Option<PathBuf> {
    let dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    ["Allomos.exe", "allomos-desktop.exe"]
        .iter()
        .map(|name| dir.join(name))
        .find(|p| p.is_file())
}

/// `alm confirm`: leave the question, wait for the person's answer in the app, then tidy up.
fn confirm(
    cli: &Cli,
    cwd: &Path,
    title: &str,
    plan: &Path,
    timeout: u64,
    evidence: Option<&Path>,
    task: Option<&str>,
) -> Result<Done, Problem> {
    let project = confirm_project(cli, cwd);
    let read = |file: &Path| {
        let path = cwd.join(file);
        read_text(&path).map_err(|e| Problem::ReadFailed {
            detail: format!("{}: {e}", path.display()),
        })
    };
    let text = read(plan)?;
    let evidence = evidence.map(read).transpose()?;
    // The task must exist before the person is asked about it.
    let root = match task {
        Some(id) => {
            let root = locate(cli, cwd)?;
            allomos_core::ops::find_task(&taskbag_of(&root, load(&root))?.0, id)?;
            Some(root)
        }
        None => None,
    };
    if text.len() + evidence.as_ref().map_or(0, String::len) > MAX_PLAN_BYTES {
        return Err(Problem::BadUsage {
            detail: format!(
                "the plan is longer than {} KB; keep it to what the person must decide",
                MAX_PLAN_BYTES / 1024
            ),
        });
    }
    let asked = post_confirm(
        &project,
        title,
        &text,
        cli.holder.as_deref(),
        timeout,
        ConfirmExtra {
            evidence: evidence.as_deref(),
            task,
        },
    )
    .map_err(|e| Problem::WriteFailed {
        path: requests_dir(&project).display().to_string(),
        detail: e.to_string(),
    })?;
    if !cli.json {
        eprintln!(
            "Waiting for the person to answer in the Allomos app (project {})…",
            project.display()
        );
    }
    let start = std::time::Instant::now();
    let mut launched = false;
    let answer = loop {
        if let Some(answer) = read_answer(&project, &asked.id, &asked.fingerprint) {
            break Some(answer);
        }
        if start.elapsed() >= Duration::from_secs(timeout) {
            break None;
        }
        if !launched && start.elapsed() >= CONFIRM_LAUNCH_AFTER && !was_seen(&project, &asked.id) {
            launched = true;
            open_app(&project);
        }
        std::thread::sleep(CONFIRM_POLL);
    };
    withdraw_confirm(&project, &asked.id);
    match answer {
        Some(true) => {
            let mut out = format!(
                "The person chose Go ahead, for exactly this plan (fingerprint {}). If the plan changes, ask again.\n",
                asked.fingerprint
            );
            if let (Some(root), Some(id)) = (&root, task) {
                let task = allomos_store::record_approval(
                    root,
                    Actor::alm(cli.holder.as_deref()),
                    lock_wait(),
                    id,
                    title,
                    &asked.fingerprint,
                )?;
                out.push_str(&format!("Recorded in {}'s history.\n", task.id));
            }
            Ok(Done::new(
                json!({ "answer": "yes", "project": project, "fingerprint": asked.fingerprint, "task": task }),
                out,
            ))
        }
        Some(false) => Err(Problem::ConfirmDeclined),
        None => Err(Problem::ConfirmTimeout {
            seconds: timeout.to_string(),
        }),
    }
}

/// What `alm next` tells the agent when there are tasks to take (Task-A03-14, owner 2026-10-13).
const CHOOSING_RULE: &str = "Take the task the person named, if any. Otherwise take one from the active iteration (the one marked active, or else the first planned one) whose next dependency is met (the list shows its tasks first); with none there, from the earliest iteration, and read its handoff first (`alm show <id>`). Get the latest changes first, then claim it with `alm claim <id> --as <your session name> --branch <the branch you are on>` and save and share the claim right away with your version control tool (it prints the Git commands as the example): until your claim is shared, other sessions do not see it.";

fn task_json(t: &Task) -> Value {
    let mut v = serde_json::to_value(t).expect("tasks serialize");
    // The version an agent passes back with `--expected-version` (Task-A01-57).
    v["version"] = json!(ops::task_version(t));
    v
}

/// A task for `--json` reading commands: with each blocker's current state when it is blocked.
fn task_json_in(bag: &Taskbag, t: &Task) -> Value {
    let mut v = task_json(t);
    if !t.blocked_by.is_empty() {
        v["blockers"] = json!(ops::blockers(bag, t));
        v["all_blockers_done"] = json!(ops::all_blockers_done(bag, t));
    }
    v
}

/// What a stopped task waits on, in one line: "waiting for you: approve the card", "waiting for
/// Ann: …", or "blocked by A01-17 (in progress), A01-20 (done)" (decision 0016).
fn stop_line(bag: &Taskbag, t: &Task) -> Option<String> {
    match t.state {
        State::Waiting => Some(format!(
            "waiting for {}: {}",
            t.who.as_deref().unwrap_or("the approver"),
            t.waiting_on.as_deref().unwrap_or("-")
        )),
        State::Blocked => {
            let list: Vec<String> = ops::blockers(bag, t)
                .iter()
                .map(ops::Blocker::describe)
                .collect();
            let mut line = format!("blocked by {}", list.join(", "));
            if ops::all_blockers_done(bag, t) {
                line.push_str(&format!(" — all done: check, then `alm unblock {}`", t.id));
            }
            Some(line)
        }
        _ => None,
    }
}

/// An iteration's heading: ID and name, then its release and dates when it has them.
fn iteration_heading(it: &Iteration) -> String {
    let mut s = format!("{}  {}", it.id, it.name);
    let dates = match (&it.start, &it.end) {
        (Some(a), Some(b)) => Some(format!("{a} to {b}")),
        (Some(a), None) => Some(format!("from {a}")),
        (None, Some(b)) => Some(format!("until {b}")),
        (None, None) => None,
    };
    let extra: Vec<String> = it.release.iter().cloned().chain(dates).collect();
    if !extra.is_empty() {
        s.push_str(&format!("  ({})", extra.join(", ")));
    }
    // Its state, unless planned (Task-A01-root343).
    if !it.state.is_planned() {
        s.push_str(&format!("  [{}]", it.state.as_str()));
    }
    s
}

/// When to write a handoff, for the agent that asked.
/// The project notes for `alm next`: where they are, the day they last changed, then their
/// Running and Pitfalls sections as written (Task-A03-20).
fn notes_lines(n: &handoff::ProjectNotes) -> String {
    if !n.found {
        return format!(
            "Project notes: this taskbag has no {} yet ({}).\n",
            n.file,
            plain_path(&n.path).display()
        );
    }
    let mut text = format!(
        "Project notes ({}, last changed {}):\n",
        plain_path(&n.path).display(),
        n.changed.as_deref().unwrap_or("unknown")
    );
    for (name, section) in [("Running", &n.running), ("Pitfalls", &n.pitfalls)] {
        match section.as_deref() {
            None => text.push_str(&format!("{name}: no such section.\n")),
            Some("") => text.push_str(&format!("{name}: (empty)\n")),
            Some(body) => {
                text.push_str(&format!("{name}:\n"));
                for line in body.lines() {
                    if line.is_empty() {
                        text.push('\n');
                    } else {
                        text.push_str(&format!("  {line}\n"));
                    }
                }
            }
        }
    }
    // Where the code and the taskbag are kept online, for an agent working from a copy elsewhere
    // (Task-A03-30). alm reads no remote; the agent fills the section from its own Git remotes.
    match n.repositories.as_deref() {
        None => text.push_str(&format!(
            "Repositories: no such section. Add `## Repositories` to {} with where the code and this taskbag are kept online (each folder's remote repository, the address only; in Git: its remote), so an agent working from a copy elsewhere, in the cloud for example, finds both.\n",
            n.file
        )),
        Some([]) => text.push_str("Repositories: none named.\n"),
        Some(items) => {
            text.push_str("Repositories:\n");
            for item in items {
                text.push_str(&format!("  - {item}\n"));
            }
        }
    }
    text
}

/// The line `alm handoff` adds: keep the project notes true too (Task-A03-20).
fn notes_reminder(n: &handoff::ProjectNotes) -> String {
    let when = if n.found {
        format!("last changed {}", n.changed.as_deref().unwrap_or("unknown"))
    } else {
        "this taskbag has none yet".to_string()
    };
    format!(
        "If what is running or a pitfall changed, update `{}` too ({when}): {}\n",
        n.file,
        plain_path(&n.path).display()
    )
}

fn rules_line(r: &allomos_core::AppliedRules) -> String {
    let whose = r
        .override_for
        .as_ref()
        .map(|n| format!(" (override for {n})"))
        .unwrap_or_default();
    format!(
        "Handoff rules{whose}: with {}% of the 5-hour quota left, start nothing new, write your handoff (`alm handoff`) and stop; at {}% context used, finish the current step, write your handoff, commit, and give the person the exact prompt for a new session; also write it {}. Report your quota each time you read it: `alm quota --agent <your agent's id> --left <percent> [--resets <when>]` (the Allomos app shows it); `alm claim`, `alm label`, `alm release`, `alm done` and `alm handoff` ask for it when your last report is missing or {} minutes old or more.\n",
        r.quota_gate,
        r.context_gate,
        r.write_when,
        allomos_store::QUOTA_ASK_AFTER_MINUTES
    )
}

/// An event for people: when, who (and through which writer), the command, then one line per
/// changed field. Long or multi-line values are shortened to their first 80 characters.
fn event_lines(e: &event::Event) -> String {
    let value = |v: &Option<Value>| match v {
        None => "—".to_string(),
        Some(Value::String(s)) => {
            let first = s.lines().next().unwrap_or_default();
            let short: String = first.chars().take(80).collect();
            if short.len() < s.trim_end().len() {
                format!("{short}…")
            } else {
                short
            }
        }
        Some(other) => other.to_string(),
    };
    let mut out = format!(
        "{}  {} ({})  {}\n",
        e.at,
        e.by.as_deref().unwrap_or("someone"),
        e.via,
        e.command
    );
    for c in &e.changes {
        out.push_str(&format!(
            "  {}: {} → {}\n",
            c.field,
            value(&c.from),
            value(&c.to)
        ));
    }
    out
}

/// Text indented by four spaces, ending with a new line.
fn indent(text: &str) -> String {
    text.trim_end()
        .lines()
        .map(|l| {
            if l.is_empty() {
                "\n".to_string()
            } else {
                format!("    {l}\n")
            }
        })
        .collect()
}

/// `alm human` (Task-A03-18): the tasks that need a person. **To check**: finished without the
/// human check and not checked since, each with why (and with `--steps`, how). **To answer**:
/// waiting on a person, each with what and who. Backlog tasks are not listed. `--json` always
/// carries the steps.
fn human(
    root: &Path,
    bag: &Taskbag,
    iteration: Option<&str>,
    area: Option<&str>,
    steps: bool,
) -> Result<Done, Problem> {
    if let Some(a) = area {
        if !bag.areas.iter().any(|x| x.id == a) {
            return Err(Problem::UnknownArea { area: a.into() });
        }
    }
    if let Some(i) = iteration {
        if !bag.iterations.iter().any(|x| x.id == i) {
            return Err(Problem::UnknownIteration {
                iteration: i.into(),
            });
        }
    }
    let tasks: Vec<&Task> = bag
        .tasks
        .iter()
        .filter(|t| {
            area.is_none_or(|a| t.area == a)
                && iteration.is_none_or(|i| t.iteration.as_deref() == Some(i))
        })
        .collect();
    let to_check: Vec<&Task> = tasks.iter().copied().filter(|t| t.to_check()).collect();
    let to_answer: Vec<&Task> = tasks
        .iter()
        .copied()
        .filter(|t| t.state == State::Waiting)
        .collect();
    // Task-A03-19: tasks with AI decisions not reviewed yet, in any state.
    let decisions = allomos_store::ai_decisions(root);
    let to_review: Vec<(&Task, &Vec<Decision>)> = tasks
        .iter()
        .filter_map(|t| decisions.get(&t.id).map(|d| (*t, d)))
        .collect();
    let mut text = String::new();
    text.push_str(&format!("To check ({})\n", to_check.len()));
    if to_check.is_empty() {
        text.push_str("  Nothing to check.\n");
    }
    for t in &to_check {
        text.push_str(&format!("  {}\n", task_line(t)));
        let why = t.without_human_check.as_deref().unwrap_or("-");
        text.push_str(&format!("      not human-checked: {why}\n"));
        if steps {
            match &t.human_check_steps {
                Some(s) => {
                    text.push_str("      steps:\n");
                    for line in indent(s).lines() {
                        text.push_str(&format!("      {line}\n"));
                    }
                }
                None => text.push_str("      steps: none written\n"),
            }
        }
    }
    text.push_str(&format!("\nTo answer ({})\n", to_answer.len()));
    if to_answer.is_empty() {
        text.push_str("  Nothing to answer.\n");
    }
    for t in &to_answer {
        text.push_str(&format!("  {}\n", task_line(t)));
        if let Some(line) = stop_line(bag, t) {
            text.push_str(&format!("      {line}\n"));
        }
    }
    text.push_str(&format!(
        "\nTo review: AI decisions ({})\n",
        to_review.len()
    ));
    if to_review.is_empty() {
        text.push_str("  Nothing to review.\n");
    }
    for (t, d) in &to_review {
        text.push_str(&format!("  {}\n", task_line(t)));
        for line in decision_lines(d).lines() {
            text.push_str(&format!("    {line}\n"));
        }
    }
    if !to_check.is_empty() {
        text.push_str("\nOnce a person has checked a task: `alm checked <id>`.\n");
    }
    if !to_review.is_empty() {
        text.push_str("Once a person has reviewed a task's AI decisions: `alm reviewed <id>`.\n");
    }
    let list = |members: &[&Task]| {
        members
            .iter()
            .map(|t| task_json_in(bag, t))
            .collect::<Vec<_>>()
    };
    let review: Vec<Value> = to_review
        .iter()
        .map(|(t, d)| {
            let mut v = task_json_in(bag, t);
            v["ai_decisions"] = json!(d);
            v
        })
        .collect();
    Ok(Done::new(
        json!({ "taskbag": root, "to_check": list(&to_check), "to_answer": list(&to_answer), "to_review": review }),
        text,
    ))
}

/// A task's AI decisions to review, one per line: "  - <text> (<by>, <at>)" (Task-A03-19).
fn decision_lines(decisions: &[Decision]) -> String {
    decisions
        .iter()
        .map(|d| match &d.by {
            Some(by) => format!("  - {} ({by}, {})\n", d.text, d.at),
            None => format!("  - {} ({})\n", d.text, d.at),
        })
        .collect()
}

/// One line about a task: ID, state, label, holder, title.
fn task_line(t: &Task) -> String {
    format!(
        "{}  {}  {}  {}  {}",
        t.id,
        t.state.as_str(),
        t.label.as_deref().unwrap_or("-"),
        t.holder.as_deref().unwrap_or("-"),
        t.title
    )
}

fn execute(cli: &Cli, cwd: &Path) -> Result<Done, Problem> {
    let me = cli.holder.as_deref();
    match &cli.command {
        Command::Init {
            name,
            id,
            labels,
            write_agent_files,
        } => {
            let root = match &cli.taskbag {
                Some(p) => cwd.join(p),
                // Here, never the repository's top folder: `alm` reads no Git (decision 0017).
                None => default_root(cwd),
            };
            let folder = root
                .parent()
                .and_then(Path::parent)
                .and_then(Path::file_name)
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "project".to_string());
            let name = name.clone().unwrap_or_else(|| folder.clone());
            let id = id.clone().unwrap_or_else(|| id_from_name(&folder));
            let labels = match labels {
                Some(list) => list
                    .split(',')
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .map(String::from)
                    .collect(),
                None => allomos_core::default_labels(),
            };
            let root = init_with_labels(&root, &id, &name, labels, Actor::alm(me))?;
            let mut text = format!("Created an empty taskbag at {}\n", root.display());
            // Tell AI agents about Allomos in the project's instruction files: by default ask
            // the agent running `alm init` to merge the section in; with --write-agent-files, write it.
            let mut agent_files = Vec::new();
            let mut warnings = Vec::new();
            let project = picked_folder(&root).unwrap_or(&root).to_path_buf();
            let agent_setup = (!*write_agent_files).then(|| agent_setup_request(&project, &root));
            if let (true, Some(folder)) = (*write_agent_files, picked_folder(&root)) {
                match add_agent_instructions(folder) {
                    Ok(files) => agent_files = files,
                    Err(e) => warnings.push(Problem::WriteFailed {
                        path: folder.join("AGENTS.md").display().to_string(),
                        detail: e.to_string(),
                    }),
                }
            }
            if !agent_files.is_empty() {
                text.push_str(&format!(
                    "Added Allomos's section for AI agents to {}\n",
                    agent_files.join(" and ")
                ));
            }
            if let Some(request) = &agent_setup {
                text.push_str("\nNext step for the AI agent running this command:\n\n");
                text.push_str(request);
            }
            let mut done = Done::new(
                json!({ "taskbag": root, "agent_files": agent_files, "agent_setup": agent_setup }),
                text,
            );
            done.warnings = warnings;
            Ok(done)
        }
        Command::Confirm {
            title,
            plan,
            timeout,
            evidence,
            task,
        } => confirm(
            cli,
            cwd,
            title,
            plan,
            *timeout,
            evidence.as_deref(),
            task.as_deref(),
        ),
        Command::Progress {
            title,
            step,
            done,
            of,
            finished,
        } => {
            // Passed on to the app as it is (Task-A05-08); alm checks nothing.
            let project = confirm_project(cli, cwd);
            let p = allomos_store::progress::report(
                &project,
                cli.holder.as_deref(),
                &allomos_store::progress::Report {
                    title: title.as_deref(),
                    step: step.as_deref(),
                    done: *done,
                    of: *of,
                    finished: finished.as_deref(),
                },
            )?;
            let count = match (p.done, p.of) {
                (Some(d), Some(o)) => format!(" ({d} of {o})"),
                _ => String::new(),
            };
            let text = match &p.finished {
                Some(f) => format!("Shown in the Allomos app: {}: finished: {f}\n", p.title),
                None => format!(
                    "Shown in the Allomos app: {}: {}{count}\n",
                    p.title,
                    p.step.as_deref().unwrap_or("…")
                ),
            };
            Ok(Done::new(
                json!({ "progress": p, "project": project }),
                text,
            ))
        }
        Command::Quota {
            agent,
            account,
            left,
            resets,
            window,
        } => {
            let reports = match (agent, left) {
                (Some(agent), Some(left)) => {
                    let report = allomos_store::new_quota_report(
                        agent,
                        account.as_deref(),
                        window,
                        *left,
                        resets.as_deref(),
                    )?;
                    allomos_store::save_quota(report).map_err(|e| Problem::WriteFailed {
                        path: "quota.json".to_string(),
                        detail: e.to_string(),
                    })?
                }
                _ => allomos_store::quota_reports(),
            };
            let mut text = String::new();
            if agent.is_some() {
                text.push_str("Reported. Kept on this computer only: nothing to save or share.\n");
            }
            for r in &reports {
                text.push_str(&format!(
                    "{}{}  {}  {}% left{}  (reported {})\n",
                    r.agent,
                    r.account
                        .as_deref()
                        .map(|a| format!(" ({a})"))
                        .unwrap_or_default(),
                    r.window,
                    r.left,
                    r.resets
                        .as_deref()
                        .map(|t| format!(", resets {t}"))
                        .unwrap_or_default(),
                    r.at
                ));
            }
            if reports.is_empty() {
                text.push_str("No quota reported on this computer yet.\n");
            }
            Ok(Done::new(json!({ "reports": reports }), text))
        }
        Command::Brief => {
            // With a taskbag, the section names it when it is outside the project folder.
            let text = match locate(cli, cwd) {
                Ok(root) => {
                    let project = load(&root)
                        .taskbag
                        .and_then(|bag| project_folder_of(&root, &bag.project))
                        .unwrap_or_else(|| picked_folder(&root).unwrap_or(&root).to_path_buf());
                    agent_section_for(&project, &root)
                }
                Err(_) => agent_section(),
            };
            Ok(Done::new(json!({ "brief": text }), text))
        }
        Command::HandoffRules {
            quota,
            context,
            write_when,
            agent,
            clear,
        } => {
            let root = locate(cli, cwd)?;
            let changing = quota.is_some() || context.is_some() || write_when.is_some() || *clear;
            let (bag, issues) = if changing {
                let lock = Lock::acquire(&root, lock_wait())?;
                let (bag, issues) = taskbag_of(&root, lock.load())?;
                let change = ops::HandoffChange {
                    quota_gate: quota.as_deref(),
                    context_gate: context.as_deref(),
                    write_when: write_when.as_deref(),
                    agent: agent.as_deref(),
                    clear: *clear,
                };
                let project = ops::set_handoff_rules(&bag, &change)?;
                save_project(&lock, me, "handoff-rules", &bag.project, &project)?;
                (Taskbag { project, ..bag }, issues)
            } else {
                taskbag_of(&root, load(&root))?
            };
            let rules = bag.project.handoff.clone().unwrap_or_default();
            let applied = rules.applied(me.or(agent.as_deref()));
            let mut text = rules_line(&applied);
            for (name, g) in &rules.overrides {
                let part = |v: Option<u8>| {
                    v.map(|v| format!("{v}%"))
                        .unwrap_or_else(|| "project's".into())
                };
                text.push_str(&format!(
                    "  override for {name}: quota gate {}, context gate {}\n",
                    part(g.quota_gate),
                    part(g.context_gate)
                ));
            }
            let mut done = Done::new(
                json!({ "taskbag": root, "handoff_rules": rules, "applied": applied }),
                text,
            );
            done.issues = issues;
            Ok(done)
        }
        Command::Handoff {
            id,
            text,
            from,
            evidence,
        } => {
            let root = locate(cli, cwd)?;
            let note = match from {
                Some(file) => {
                    let file = cwd.join(file);
                    read_text(&file).map_err(|e| Problem::ReadFailed {
                        detail: format!("{}: {e}", file.display()),
                    })?
                }
                None => text.clone().unwrap_or_default(),
            };
            let now = handoff::utc_now();
            let day = now.get(..10).unwrap_or(&now);
            let changed = allomos_store::change_task(&root, Actor::alm(me), lock_wait(), |bag| {
                ops::hand_off(bag, id, me, &note, evidence.as_deref(), day).map(|o| (o, "handoff"))
            })?;
            let notes = handoff::project_notes(&root);
            let text = format!(
                "{}\nHanded off: its handoff note is set{}; `alm show {}` shows it.\n{}",
                task_line(&changed.task),
                if evidence.is_some() {
                    " and the evidence is under its description's Evidence"
                } else {
                    ""
                },
                changed.task.id,
                notes_reminder(&notes)
            );
            let mut done = Done::new(
                json!({ "taskbag": root, "task": task_json(&changed.task), "project_notes": notes }),
                text,
            );
            done.warnings = changed.warnings;
            done.issues = changed.issues;
            Ok(done)
        }
        Command::MoveIn { language } => {
            let root = locate(cli, cwd)?;
            if let versions::FormatState::Newer { found } = versions::format_state(&root) {
                return Err(Problem::TaskbagNewer {
                    found,
                    supported: FORMAT.to_string(),
                });
            }
            let language = language.clone().unwrap_or_else(|| {
                saved_ui_language().unwrap_or_else(|| {
                    let system: Vec<String> = sys_locale::get_locales().collect();
                    pick_ui_language(&system).to_string()
                })
            });
            // An older taskbag is not read: its project folder comes from its project.json as it is.
            let project = versions::project_folder(&root);
            // One request for Update the taskbag… (Task-A01-45): the move-in request for an older
            // taskbag, otherwise the move-cards request for card files kept outside the taskbag.
            let request = allomos_store::agent_move_tasks_in_request_in(&language, &project, &root);
            let Some(move_in) = versions::move_in(&root) else {
                let cards = allomos_store::cards::outside(&root);
                return Ok(Done::new(
                    json!({
                        "taskbag": root,
                        "move_in": { "needed": false },
                        "cards_outside": cards,
                        "language": language,
                        "request": request,
                    }),
                    request,
                ));
            };
            Ok(Done::new(
                json!({
                    "taskbag": root,
                    "move_in": {
                        "needed": true,
                        "from": move_in.found,
                        "to": FORMAT,
                        "old": move_in.old,
                        "prepared": move_in.prepared,
                    },
                    "language": language,
                    "request": request,
                }),
                request,
            ))
        }
        Command::Guide { words, language } => {
            // A topic may be written as separate words (`alm guide github release`,
            // Task-A03-planet530); words after it are the guide's, for the agent to read.
            let (topic, rest) = if words.is_empty() {
                (None, Vec::new())
            } else {
                match guide_topic(words) {
                    Some((topic, rest)) => (Some(topic), rest),
                    None => {
                        return Err(Problem::BadUsage {
                            detail: format!(
                                "There is no guide topic \"{}\". The topics: {}.",
                                words.join(" "),
                                GUIDE_TOPICS.join(", ")
                            ),
                        });
                    }
                }
            };
            let after = if rest.is_empty() {
                String::new()
            } else {
                format!(
                    "Words after the topic, for you to read as this guide says: {}\n\n",
                    rest.join(" ")
                )
            };
            let Some(name) = topic.filter(|t| LOCALIZED_TOPICS.contains(t)) else {
                let text = format!("{after}{}", guide(topic).unwrap_or_default());
                return Ok(Done::new(
                    json!({ "topic": topic, "words": rest, "guide": text }),
                    text,
                ));
            };
            // The person reads it (or reads along): in the task language (Task-A03-21,
            // Task-A03-24). Without a taskbag here, the language the app would use.
            let language = match language {
                Some(code) => code.trim().to_string(),
                None => {
                    let system: Vec<String> = sys_locale::get_locales().collect();
                    match locate(cli, cwd)
                        .ok()
                        .and_then(|root| taskbag_of(&root, load(&root)).ok())
                    {
                        Some((bag, _)) => task_language(&bag.project, &system).language,
                        None => saved_ui_language()
                            .unwrap_or_else(|| pick_ui_language(&system).to_string()),
                    }
                }
            };
            let (text, written) = match localized(name, &language) {
                Some(text) => (text.to_string(), true),
                // The goal prompt and the release procedure are the agent's to follow, not to
                // show (Task-A03-25-01, Task-A03-planet530).
                None if name == "goal" || name == "github-release" => (
                    format!(
                        "This text is not written in {language} yet: follow a faithful translation \
                         of it into {language}, keeping commands as they are{}.\n\n{}",
                        if name == "goal" {
                            format!(", and write the tasks in {language}")
                        } else {
                            String::new()
                        },
                        localized(name, "en").unwrap_or_default()
                    ),
                    false,
                ),
                None => (
                    format!(
                        "This text is not written in {language} yet: show the person a faithful \
                         translation into {language}, keeping commands as they are.\n\n{}",
                        localized(name, "en").unwrap_or_default()
                    ),
                    false,
                ),
            };
            let text = format!("{after}{text}");
            Ok(Done::new(
                json!({ "topic": topic, "words": rest, "language": language, "written_in_language": written, "guide": text }),
                text,
            ))
        }
        Command::TaskLanguage { value: Some(value) } => {
            let root = locate(cli, cwd)?;
            let lock = Lock::acquire(&root, lock_wait())?;
            let (bag, issues) = taskbag_of(&root, lock.load())?;
            let project = ops::set_task_language(&bag, value)?;
            save_project(&lock, me, "task-language", &bag.project, &project)?;
            let system: Vec<String> = sys_locale::get_locales().collect();
            let now = task_language(&project, &system);
            let mut done = Done::new(
                json!({ "taskbag": root, "task_language": now }),
                format!(
                    "Task language set to {}. {}\n",
                    now.setting,
                    language_line(&now)
                ),
            );
            done.issues = issues;
            Ok(done)
        }
        Command::TaskLanguage { value: None } => {
            let root = locate(cli, cwd)?;
            let (bag, issues) = taskbag_of(&root, load(&root))?;
            let system: Vec<String> = sys_locale::get_locales().collect();
            let now = task_language(&bag.project, &system);
            let mut done = Done::new(
                json!({ "taskbag": root, "task_language": now }),
                format!("{}\n", language_line(&now)),
            );
            done.issues = issues;
            Ok(done)
        }
        Command::Labels { ladder } => {
            let root = locate(cli, cwd)?;
            let lock = Lock::acquire(&root, lock_wait())?;
            let (bag, issues) = taskbag_of(&root, lock.load())?;
            let labels: Vec<String> = ladder.split(',').map(String::from).collect();
            let project = ops::set_ladder(&bag, &labels)?;
            save_project(&lock, me, "labels", &bag.project, &project)?;
            let mut done = Done::new(
                json!({ "taskbag": root, "labels": project.labels }),
                format!("Label ladder: {}\n", project.labels.join(" → ")),
            );
            done.issues = issues;
            Ok(done)
        }
        Command::Add(add) => {
            let root = locate(cli, cwd)?;
            let actor = Actor::alm(me);
            let (mut done, issues) = match add {
                Add::Area { first, name } => {
                    let (id, name) = id_and_name(first, name);
                    let added = allomos_store::add_area(&root, actor, lock_wait(), id, name)?;
                    let area = added.item;
                    let text = format!("Added area {} ({})\n", area.id, area.name);
                    let done = Done::new(json!({ "taskbag": root, "area": area }), text);
                    (done, added.issues)
                }
                Add::Iteration {
                    first,
                    name,
                    release,
                    start,
                    end,
                } => {
                    let (id, name) = id_and_name(first, name);
                    let details = ops::IterationDetails {
                        release: release.as_deref(),
                        start: start.as_deref(),
                        end: end.as_deref(),
                    };
                    let added =
                        allomos_store::add_iteration(&root, actor, lock_wait(), id, name, details)?;
                    let iteration = added.item;
                    let text = format!("Added iteration {}\n", iteration_heading(&iteration));
                    let done = Done::new(json!({ "taskbag": root, "iteration": iteration }), text);
                    (done, added.issues)
                }
                Add::Task {
                    id,
                    area,
                    title,
                    description,
                    branch,
                    base,
                    iteration,
                } => {
                    // A new task goes into the active iteration unless another is named
                    // (approver, 2026-10-22; Task-A01-root343).
                    let active = taskbag_of(&root, load(&root))
                        .ok()
                        .and_then(|(bag, _)| ops::active_iteration(&bag).map(|i| i.id.clone()));
                    let planned_in = iteration.clone().or(active.clone());
                    let new = allomos_store::NewTask {
                        id: id.clone(),
                        area: area.clone(),
                        title: title.clone(),
                        description: description.clone(),
                        iteration: planned_in.clone(),
                        branch: branch.clone(),
                        base: base.clone(),
                    };
                    let added = allomos_store::add_task_with_base(&root, actor, lock_wait(), &new)?;
                    let task = added.task;
                    let mut text = format!("Added {}\n", task_line(&task));
                    if let Some(c) = &added.base {
                        text.push_str(&format!("  base branch: {}\n", c.english()));
                    }
                    // What to do next, in the task language: plan it, then make it ready
                    // unless something is the person's to decide (Task-A03-33).
                    let system: Vec<String> = sys_locale::get_locales().collect();
                    let (language, iterations) = match taskbag_of(&root, load(&root)) {
                        Ok((bag, _)) => (
                            task_language(&bag.project, &system).language,
                            bag.iterations.iter().map(iteration_heading).collect(),
                        ),
                        Err(_) => ("en".to_string(), Vec::new()),
                    };
                    if let Some(i) = &task.iteration {
                        let why = if iteration.is_some() {
                            "as asked"
                        } else {
                            "the active iteration"
                        };
                        text.push_str(&format!("  iteration: {i} ({why})\n"));
                    }
                    let steps = allomos_store::added_task_steps(
                        &language,
                        &task.id,
                        &iterations,
                        task.iteration.as_deref(),
                    );
                    let mut done = Done::new(
                        json!({ "taskbag": root, "task": task_json(&task), "base": added.base, "next_steps": steps }),
                        text,
                    );
                    done.after = steps;
                    done.warnings = added.warnings;
                    (done, added.issues)
                }
            };
            done.issues = issues;
            Ok(done)
        }
        Command::Rename(Rename::Area { id, name }) => {
            let root = locate(cli, cwd)?;
            let renamed = allomos_store::rename_area(&root, Actor::alm(me), lock_wait(), id, name)?;
            let area = renamed.item;
            let text = format!("Renamed area {} ({})\n", area.id, area.name);
            let mut done = Done::new(json!({ "taskbag": root, "area": area }), text);
            done.issues = renamed.issues;
            Ok(done)
        }
        Command::Rename(Rename::Iteration { old, new }) => {
            let root = locate(cli, cwd)?;
            let renamed =
                allomos_store::rename_iteration(&root, Actor::alm(me), lock_wait(), old, new)?;
            let (iteration, tasks) = renamed.item;
            let mut text = if iteration.id == *old {
                format!("{old} already has that ID; nothing changed.\n")
            } else {
                format!(
                    "Renamed iteration {old} to {}\n",
                    iteration_heading(&iteration)
                )
            };
            if !tasks.is_empty() {
                let ids: Vec<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
                text.push_str(&format!(
                    "Moved {} task(s) along: {}\n",
                    ids.len(),
                    ids.join(", ")
                ));
            }
            let mut done = Done::new(
                json!({ "taskbag": root, "iteration": iteration, "tasks": tasks.iter().map(task_json).collect::<Vec<_>>() }),
                text,
            );
            done.issues = renamed.issues;
            Ok(done)
        }
        Command::Rename(Rename::Label { old, new }) => {
            let root = locate(cli, cwd)?;
            let renamed =
                allomos_store::rename_label(&root, Actor::alm(me), lock_wait(), old, new)?;
            let (labels, tasks) = renamed.item;
            let ids: Vec<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
            let mut text = format!("Label ladder: {}\n", labels.join(" → "));
            text.push_str(&if ids.is_empty() {
                "No task carried the label.\n".to_string()
            } else {
                format!("Relabelled {} task(s): {}\n", ids.len(), ids.join(", "))
            });
            let mut done = Done::new(
                json!({ "taskbag": root, "labels": labels, "tasks": tasks.iter().map(task_json).collect::<Vec<_>>() }),
                text,
            );
            done.issues = renamed.issues;
            Ok(done)
        }
        Command::Change(ChangeWhat::Iteration {
            id,
            name,
            release,
            start,
            end,
            state,
        }) => {
            let root = locate(cli, cwd)?;
            let state = match state {
                Some(s) => Some(
                    IterationState::parse(s)
                        .ok_or_else(|| Problem::InvalidIterationState { value: s.clone() })?,
                ),
                None => None,
            };
            let change = ops::IterationChange {
                name: name.clone(),
                release: release.clone(),
                start: start.clone(),
                end: end.clone(),
                state,
            };
            let changed =
                allomos_store::change_iteration(&root, Actor::alm(me), lock_wait(), id, &change)?;
            let iteration = changed.item;
            let mut text = format!("Changed iteration {}\n", iteration_heading(&iteration));
            // Closing an iteration with unfinished tasks moves none of them: alm lists them and
            // asks (approver, 2026-10-22; Task-A01-root343).
            let mut unfinished: Vec<Value> = Vec::new();
            if state == Some(IterationState::Closed) {
                if let Ok((bag, _)) = taskbag_of(&root, load(&root)) {
                    let left = ops::unfinished_in(&bag, &iteration.id);
                    if !left.is_empty() {
                        text.push_str(&format!(
                            "{} task(s) in {} are not finished; none was moved:\n",
                            left.len(),
                            iteration.id
                        ));
                        for t in &left {
                            text.push_str(&format!("  {}\n", task_line(t)));
                        }
                        text.push_str("Decide for each: plan it in another iteration (`alm plan <id> <iteration>`), finish or drop it, or leave it here. Ask the person when it is theirs to decide.\n");
                    }
                    unfinished = left.into_iter().map(task_json).collect();
                }
            }
            let mut done = Done::new(
                json!({ "taskbag": root, "iteration": iteration, "unfinished": unfinished }),
                text,
            );
            done.issues = changed.issues;
            Ok(done)
        }
        Command::Move(MoveWhat::Iteration { id, before, after }) => {
            let root = locate(cli, cwd)?;
            let place = match (before, after) {
                (Some(other), _) => ops::Place::Before(other),
                (None, Some(other)) => ops::Place::After(other),
                (None, None) => unreachable!("clap requires --before or --after"),
            };
            let moved =
                allomos_store::move_iteration(&root, Actor::alm(me), lock_wait(), id, place)?;
            let (word, other) = match place {
                ops::Place::Before(o) => ("before", o),
                ops::Place::After(o) => ("after", o),
            };
            let mut text = if moved.item.is_empty() {
                format!("{id} is already there; nothing changed.\n")
            } else {
                format!("Moved iteration {id} {word} {other}\n")
            };
            let mut order: Vec<Iteration> = Vec::new();
            if let Ok((bag, _)) = taskbag_of(&root, load(&root)) {
                order = bag.iterations;
                order.sort_by(|a, b| a.order.cmp(&b.order).then(a.id.cmp(&b.id)));
                text.push_str("The iterations, in order:\n");
                for it in &order {
                    text.push_str(&format!("  {}\n", iteration_heading(it)));
                }
            }
            let mut done = Done::new(
                json!({ "taskbag": root, "moved": moved.item, "iterations": order }),
                text,
            );
            done.issues = moved.issues;
            Ok(done)
        }
        Command::Delete(what) => {
            let root = locate(cli, cwd)?;
            let actor = Actor::alm(me);
            let done = match what {
                Delete::Area { id } => {
                    let gone = allomos_store::delete_area(&root, actor, lock_wait(), id)?;
                    let text = format!("Deleted area {} ({})\n", gone.item.id, gone.item.name);
                    let mut done = Done::new(
                        json!({ "taskbag": root, "deleted": { "area": gone.item } }),
                        text,
                    );
                    done.issues = gone.issues;
                    done
                }
                Delete::Iteration { id } => {
                    let gone = allomos_store::delete_iteration(&root, actor, lock_wait(), id)?;
                    let text = format!("Deleted iteration {}\n", iteration_heading(&gone.item));
                    let mut done = Done::new(
                        json!({ "taskbag": root, "deleted": { "iteration": gone.item } }),
                        text,
                    );
                    done.issues = gone.issues;
                    done
                }
            };
            Ok(done)
        }
        Command::List {
            area,
            iteration,
            by,
            state,
            mine,
            without_human_check,
        } => {
            let root = locate(cli, cwd)?;
            let (bag, issues) = taskbag_of(&root, load(&root))?;
            if *mine && me.is_none() {
                return Err(Problem::HolderMissing);
            }
            if let Some(a) = area {
                if !bag.areas.iter().any(|x| &x.id == a) {
                    return Err(Problem::UnknownArea { area: a.clone() });
                }
            }
            if let Some(i) = iteration {
                if !bag.iterations.iter().any(|x| &x.id == i) {
                    return Err(Problem::UnknownIteration {
                        iteration: i.clone(),
                    });
                }
            }
            let keep = |t: &&Task| {
                area.as_ref().is_none_or(|a| &t.area == a)
                    && iteration
                        .as_ref()
                        .is_none_or(|i| t.iteration.as_ref() == Some(i))
                    && state.as_ref().is_none_or(|s| t.state.as_str() == s)
                    && (!*mine || t.holder.as_deref() == me)
                    && (!*without_human_check || t.to_check())
            };
            // In the taskbag's order: numbered IDs, then word IDs as they were made (A01-54).
            let tasks: Vec<&Task> = bag.tasks.iter().filter(keep).collect();
            let mut text = String::new();
            let mut group = |heading: String, members: &[&&Task]| {
                if members.is_empty() {
                    return;
                }
                text.push_str(&heading);
                text.push('\n');
                for t in members {
                    text.push_str(&format!("  {}\n", task_line(t)));
                    if let Some(line) = stop_line(&bag, t) {
                        text.push_str(&format!("      {line}\n"));
                    }
                }
            };
            let tasks_json = |members: &[&&Task]| {
                members
                    .iter()
                    .map(|t| task_json_in(&bag, t))
                    .collect::<Vec<_>>()
            };
            let fields = if by == "iteration" {
                let active = ops::active_iteration(&bag).map(|i| i.id.clone());
                let mut groups = Vec::new();
                for it in &bag.iterations {
                    if iteration.as_ref().is_some_and(|x| x != &it.id) {
                        continue;
                    }
                    let members: Vec<&&Task> = tasks
                        .iter()
                        .filter(|t| t.iteration.as_ref() == Some(&it.id))
                        .collect();
                    let is_active = active.as_ref() == Some(&it.id);
                    let mut heading = iteration_heading(it);
                    // The first planned iteration, active because none is marked (Task-A01-root343).
                    if is_active && it.state.is_planned() {
                        heading.push_str("  [active]");
                    }
                    group(heading, &members);
                    let mut g = json!(it);
                    g["active"] = json!(is_active);
                    g["tasks"] = json!(tasks_json(&members));
                    groups.push(g);
                }
                // Tasks with no iteration, or one that does not exist, come last.
                let unplanned: Vec<&&Task> = tasks
                    .iter()
                    .filter(|t| {
                        t.iteration
                            .as_ref()
                            .is_none_or(|i| !bag.iterations.iter().any(|x| &x.id == i))
                    })
                    .collect();
                group("Unplanned".to_string(), &unplanned);
                json!({ "taskbag": root, "iterations": groups, "unplanned": tasks_json(&unplanned) })
            } else {
                let mut groups = Vec::new();
                for a in &bag.areas {
                    if area.as_ref().is_some_and(|x| x != &a.id) {
                        continue;
                    }
                    let members: Vec<&&Task> = tasks.iter().filter(|t| t.area == a.id).collect();
                    group(format!("{}  {}", a.id, a.name), &members);
                    groups.push(json!({
                        "id": a.id, "name": a.name, "order": a.order,
                        "tasks": tasks_json(&members),
                    }));
                }
                json!({ "taskbag": root, "areas": groups })
            };
            if text.is_empty() {
                text.push_str("No tasks.\n");
            }
            // The card files still outside the taskbag, which the app's notice counts
            // (Task-A01-51): the same facts as `alm cards`.
            let mut fields = fields;
            fields["cards_outside"] = json!(allomos_store::cards::outside_of(&root, &bag));
            let mut done = Done::new(fields, text);
            done.issues = issues;
            Ok(done)
        }
        Command::Show { id } => {
            let root = locate(cli, cwd)?;
            let (bag, issues) = taskbag_of(&root, load(&root))?;
            let task = ops::find_task(&bag, id)?;
            let mut text = String::new();
            let blocked = (!task.blocked_by.is_empty()).then(|| {
                ops::blockers(&bag, &task)
                    .iter()
                    .map(ops::Blocker::describe)
                    .collect::<Vec<_>>()
                    .join(", ")
            });
            let fields = [
                ("id", Some(task.id.as_str())),
                ("title", Some(task.title.as_str())),
                ("area", Some(task.area.as_str())),
                ("iteration", task.iteration.as_deref()),
                ("state", Some(task.state.as_str())),
                ("label", task.label.as_deref()),
                ("holder", task.holder.as_deref()),
                ("waiting on", task.waiting_on.as_deref()),
                ("waiting for", task.who.as_deref()),
                ("blocked by", blocked.as_deref()),
                ("next dependency", task.next_dependency.as_deref()),
                ("handoff note", task.handoff_note.as_deref()),
                ("not human-checked", task.without_human_check.as_deref()),
            ];
            // For `alm title` / `alm describe --expected-version` (Task-A01-57).
            let version = ops::task_version(&task);
            for (name, value) in fields {
                if let Some(v) = value {
                    text.push_str(&format!("{name}: {v}\n"));
                }
            }
            if let Some(steps) = &task.human_check_steps {
                text.push_str(&format!("human check steps:\n{}", indent(steps)));
            }
            text.push_str(&format!("version: {version}\n"));
            let branches = allomos_store::branches(&root)
                .remove(&task.id)
                .unwrap_or_default();
            for (name, value) in [
                ("base branch", &branches.base),
                ("working branch", &branches.working),
            ] {
                if let Some(v) = value {
                    text.push_str(&format!("{name}: {v}\n"));
                }
            }
            let decisions = allomos_store::ai_decisions(&root)
                .remove(&task.id)
                .unwrap_or_default();
            if !decisions.is_empty() {
                text.push_str("AI decided (to review):\n");
                text.push_str(&decision_lines(&decisions));
            }
            // Whoever takes over checks before trusting the handoff note and repeats nothing
            // (Task-A03-23).
            let taking_over = task.handoff_note.as_ref().map(|_| handoff::TAKING_OVER);
            if let Some(rules) = taking_over {
                text.push_str(rules);
            }
            if ops::all_blockers_done(&bag, &task) {
                text.push_str(&format!(
                    "All blockers are done: check, then `alm unblock {}`. alm moves nothing by itself.\n",
                    task.id
                ));
            }
            if !task.description.is_empty() {
                text.push('\n');
                text.push_str(&task.description);
            }
            let mut done = Done::new(
                json!({ "taskbag": root, "task": task_json_in(&bag, &task), "branches": branches, "ai_decisions": decisions, "taking_over": taking_over }),
                text,
            );
            done.issues = issues;
            Ok(done)
        }
        Command::History { id } => {
            let root = locate(cli, cwd)?;
            let (bag, mut issues) = taskbag_of(&root, load(&root))?;
            let task = ops::find_task(&bag, id)?;
            let history = events::history(&root, &task.id);
            let mut text = match &history.since {
                Some(since) => format!("History of {} (recorded since {since})\n", task.id),
                None => format!(
                    "No history yet: this taskbag has recorded no changes, so none are shown for {}.\n",
                    task.id
                ),
            };
            if history.since.is_some() && history.events.is_empty() {
                text.push_str("No changes recorded for this task yet.\n");
            }
            for e in &history.events {
                text.push_str(&event_lines(e));
            }
            issues.extend(history.issues.iter().cloned());
            let mut done = Done::new(
                json!({ "taskbag": root, "task": task.id, "since": history.since, "events": history.events }),
                text,
            );
            done.issues = issues;
            Ok(done)
        }
        Command::Next { iteration } => {
            let root = locate(cli, cwd)?;
            let (bag, issues) = taskbag_of(&root, load(&root))?;
            // `alm` lists; the agent chooses (Task-A03-14).
            let tasks = ops::next(&bag, iteration.as_deref())?;
            let instruction = if tasks.is_empty() {
                match iteration {
                    Some(i) => format!(
                        "Nothing to take: no ready task in iteration {i} is free. Ask the person what to do next."
                    ),
                    None => {
                        "Nothing to take: no ready task is free. Ask the person what to do next."
                            .to_string()
                    }
                }
            } else {
                CHOOSING_RULE.to_string()
            };
            let mut text = String::new();
            // The active iteration all done: suggest the next step; alm changes no state
            // (approver, 2026-10-22; Task-A01-root343).
            let finished = ops::finished_active(&bag).map(|(done_it, next_it)| {
                let words = match next_it {
                    // Active only because it is the first planned: closing it is enough, the next
                    // planned one is then active the same way.
                    Some(n) if done_it.state.is_planned() => format!(
                        "{} is all done (every task done or dropped). Close it? `alm change iteration {} --state closed` (ask the person first unless they set this up); {}, the next planned iteration, is then active.",
                        done_it.id, done_it.id, n.id
                    ),
                    Some(n) => format!(
                        "{} is all done (every task done or dropped). Close it and make {} active? `alm change iteration {} --state closed`, then `alm change iteration {} --state active` (ask the person first unless they set this up).",
                        done_it.id, n.id, done_it.id, n.id
                    ),
                    None => format!(
                        "{} is all done (every task done or dropped). Close it? `alm change iteration {} --state closed` (ask the person first unless they set this up); no planned iteration follows it.",
                        done_it.id, done_it.id
                    ),
                };
                (done_it.id.clone(), next_it.map(|n| n.id.clone()), words)
            });
            if let Some((_, _, words)) = &finished {
                text.push_str(words);
                text.push_str("\n\n");
            }
            let mut listed = Vec::new();
            for t in &tasks {
                text.push_str(&format!("{}\n", task_line(t)));
                if let Some(i) = &t.iteration {
                    text.push_str(&format!("  iteration: {i}\n"));
                }
                if let Some(n) = &t.next_dependency {
                    text.push_str(&format!("  next dependency: {n}\n"));
                }
                if let Some(note) = &t.handoff_note {
                    text.push_str(&format!("  handoff note: {note}\n"));
                }
                listed.push(task_json(t));
            }
            if !tasks.is_empty() {
                text.push('\n');
            }
            // Once, when a listed task has a handoff note: the rules for taking over (Task-A03-23).
            let taking_over = tasks
                .iter()
                .any(|t| t.handoff_note.is_some())
                .then_some(handoff::TAKING_OVER);
            if let Some(rules) = taking_over {
                text.push_str(rules);
                text.push('\n');
            }
            text.push_str(&instruction);
            text.push('\n');
            // Blocked tasks are skipped by their state; `alm` tells when one may go on, or when a
            // blocker was dropped, and moves nothing (decision 0016).
            let mut blocked = Vec::new();
            for t in bag.tasks.iter().filter(|t| {
                t.state == State::Blocked
                    && iteration
                        .as_ref()
                        .is_none_or(|i| t.iteration.as_ref() == Some(i))
            }) {
                let all_done = ops::all_blockers_done(&bag, t);
                let dropped = ops::blockers(&bag, t)
                    .iter()
                    .any(|b| b.state == Some(State::Dropped));
                if !(all_done || dropped) {
                    continue;
                }
                let list: Vec<String> = ops::blockers(&bag, t)
                    .iter()
                    .map(ops::Blocker::describe)
                    .collect();
                text.push_str(&if all_done {
                    format!(
                        "{}: all blockers are done ({}) — unblock it? Check, then `alm unblock {}`.\n",
                        t.id,
                        list.join(", "),
                        t.id
                    )
                } else {
                    format!(
                        "{}: blocked by {}: a dropped task is never done.\n",
                        t.id,
                        list.join(", ")
                    )
                });
                blocked.push(task_json_in(&bag, t));
            }
            // Card files (Task-A00-30): what the tasks' `Card:` links point to outside the
            // taskbag and what agents reported; nothing when there are none (Task-A03-34).
            let outside = allomos_store::cards::outside(&root);
            let cards = next_cards_line(outside.len());
            text.push_str(&cards);
            // Old handoff files (Task-A03-polar770): the taskbag needs updating; alm only says
            // they are there.
            let old_handoffs = handoff::legacy(&root);
            let handoffs_line = next_handoffs_line(&old_handoffs);
            text.push_str(&handoffs_line);
            // At the start of a session: when this agent must write its handoff (Task-A03-06).
            let rules = bag.project.handoff.clone().unwrap_or_default().applied(me);
            text.push_str(&rules_line(&rules));
            // What every session must know: the project notes' Running and Pitfalls, and when
            // they last changed (Task-A03-20). Facts only.
            let notes = handoff::project_notes(&root);
            text.push('\n');
            text.push_str(&notes_lines(&notes));
            let mut done = Done::new(
                json!({
                    "taskbag": root,
                    "tasks": listed,
                    "instruction": instruction,
                    "taking_over": taking_over,
                    "blocked": blocked,
                    "handoff_rules": rules,
                    "project_notes": notes,
                    "cards_outside": outside,
                    "cards": if cards.is_empty() { Value::Null } else { json!(cards.trim_end()) },
                    "old_handoffs": old_handoffs,
                    "handoffs": if handoffs_line.is_empty() { Value::Null } else { json!(handoffs_line.trim_end()) },
                    "active_iteration": ops::active_iteration(&bag).map(|i| i.id.clone()),
                    "finished_iteration": finished.as_ref().map(|(done_it, next_it, words)| json!({ "iteration": done_it, "next": next_it, "suggestion": words })),
                }),
                text,
            );
            done.issues = issues;
            Ok(done)
        }
        Command::Claim {
            id,
            take,
            branch,
            working,
        } => {
            let root = locate(cli, cwd)?;
            // The taskbag is updated before any task is worked (Task-A03-27, approver
            // 2026-10-18): card files kept outside it are moved in first.
            let outside = allomos_store::cards::outside(&root);
            if !outside.is_empty() {
                let tasks: Vec<&str> = outside.iter().map(|c| c.task.as_str()).collect();
                return Err(Problem::ClaimCardsOutside {
                    count: outside.len().to_string(),
                    tasks: tasks.join(", "),
                });
            }
            let claimed = allomos_store::claim_task(
                &root,
                Actor::alm(me),
                lock_wait(),
                id,
                *take,
                branch.as_deref(),
                working.as_deref(),
            )?;
            let at = &claimed.branches;
            let mut text = format!("{}\n", task_line(&claimed.changed.task));
            let steps = branch_steps(at);
            text.push_str(&steps.text);
            let mut done = Done::new(
                json!({
                    "taskbag": root,
                    "task": task_json(&claimed.changed.task),
                    "branches": at.branches,
                    "base_why": at.why,
                    "branch_commands": { "start": steps.start, "merge": steps.merge },
                    "branch_facts": steps.facts,
                }),
                text,
            );
            // Before planning: the size of the files to change, in the task language
            // (Task-A03-tide388). alm reminds; the agent checks.
            let system: Vec<String> = sys_locale::get_locales().collect();
            let language = taskbag_of(&root, load(&root))
                .map(|(bag, _)| task_language(&bag.project, &system).language)
                .unwrap_or_else(|_| "en".to_string());
            let reminder = allomos_store::claimed_task_steps(&language);
            done.fields["before_planning"] = json!(reminder);
            done.after = reminder;
            done.warnings = claimed.changed.warnings;
            done.issues = claimed.changed.issues;
            Ok(done)
        }
        Command::Base { id, branch } => {
            let root = locate(cli, cwd)?;
            let (task, after, issues) =
                allomos_store::set_base(&root, Actor::alm(me), lock_wait(), id, branch)?;
            let mut text = format!("{}\n", task_line(&task));
            if let Some(base) = &after.base {
                text.push_str(&format!("  base branch: {base}\n"));
            }
            let mut done = Done::new(
                json!({ "taskbag": root, "task": task_json(&task), "branches": after }),
                text,
            );
            done.issues = issues;
            Ok(done)
        }
        Command::Label { id, label } => change(cli, cwd, |bag| ops::set_label(bag, id, me, label)),
        Command::Wait { id, on, who } => {
            change(cli, cwd, |bag| ops::wait(bag, id, me, on, who.as_deref()))
        }
        Command::Block { id, by } => {
            let mut done = change(cli, cwd, |bag| ops::block(bag, id, me, by))?;
            let root = locate(cli, cwd)?;
            if let Some(bag) = load(&root).taskbag {
                if let Some(task) = bag.tasks.iter().find(|t| &t.id == id) {
                    if let Some(line) = stop_line(&bag, task) {
                        done.text.push_str(&format!("  {line}\n"));
                    }
                    done.fields["task"] = task_json_in(&bag, task);
                }
            }
            Ok(done)
        }
        Command::Unblock { id } => change(cli, cwd, |bag| ops::unblock(bag, id, me)),
        Command::Done {
            id,
            without_human_check,
            steps,
        } => {
            let steps = match steps {
                Some(file) => {
                    let path = cwd.join(file);
                    Some(
                        std::fs::read_to_string(&path).map_err(|e| Problem::ReadFailed {
                            detail: format!("{}: {e}", path.display()),
                        })?,
                    )
                }
                None => None,
            };
            change(cli, cwd, |bag| {
                ops::finish(
                    bag,
                    id,
                    me,
                    without_human_check.as_deref(),
                    steps.as_deref(),
                )
            })
        }
        Command::Checked { id } => change(cli, cwd, |bag| ops::checked(bag, id)),
        Command::Recheck { id, reason, steps } => {
            let steps = match steps {
                Some(file) => {
                    let path = cwd.join(file);
                    Some(
                        std::fs::read_to_string(&path).map_err(|e| Problem::ReadFailed {
                            detail: format!("{}: {e}", path.display()),
                        })?,
                    )
                }
                None => None,
            };
            change(cli, cwd, |bag| {
                ops::recheck(bag, id, reason, steps.as_deref())
            })
        }
        Command::AiDecided { id, text } => {
            let root = locate(cli, cwd)?;
            let d = allomos_store::record_ai_decision(
                &root,
                Actor::alm(cli.holder.as_deref()),
                lock_wait(),
                id,
                text,
            )?;
            let mut out = format!("{}\n", task_line(&d.task));
            for x in &d.decisions {
                out.push_str(&format!("  AI decided: {}\n", x.text));
            }
            out.push_str(&format!(
                "{} AI decision(s) to review; `alm human` lists them for the person.\n",
                d.pending.len()
            ));
            let mut done = Done::new(
                json!({ "taskbag": root, "task": task_json(&d.task), "decision": d.decisions.first(), "ai_decisions": d.pending }),
                out,
            );
            done.issues = d.issues;
            Ok(done)
        }
        Command::Reviewed { id } => {
            let root = locate(cli, cwd)?;
            let d = allomos_store::review_ai_decisions(
                &root,
                Actor::alm(cli.holder.as_deref()),
                lock_wait(),
                id,
            )?;
            let mut out = format!("{}\n", task_line(&d.task));
            out.push_str(&format!("Reviewed {} AI decision(s):\n", d.decisions.len()));
            for x in &d.decisions {
                out.push_str(&format!("  {}\n", x.text));
            }
            let mut done = Done::new(
                json!({ "taskbag": root, "task": task_json(&d.task), "reviewed": d.decisions }),
                out,
            );
            done.issues = d.issues;
            Ok(done)
        }
        Command::Human {
            iteration,
            area,
            steps,
        } => {
            let root = locate(cli, cwd)?;
            let (bag, issues) = taskbag_of(&root, load(&root))?;
            human(&root, &bag, iteration.as_deref(), area.as_deref(), *steps).map(|mut done| {
                done.issues = issues;
                done
            })
        }
        Command::Title {
            id,
            title,
            expected_version,
        } => change(cli, cwd, |bag| {
            ops::check_version(bag, id, expected_version.as_deref())?;
            ops::set_title(bag, id, me, title)
        }),
        Command::Describe {
            id,
            text,
            from,
            expected_version,
        } => {
            // `--text` and `--from` exclude each other, and one of them is required.
            let text = match from {
                Some(file) => {
                    let file = cwd.join(file);
                    read_text(&file).map_err(|e| Problem::ReadFailed {
                        detail: format!("{}: {e}", file.display()),
                    })?
                }
                None => text.clone().unwrap_or_default(),
            };
            change(cli, cwd, |bag| {
                ops::check_version(bag, id, expected_version.as_deref())?;
                ops::set_description(bag, id, me, &text)
            })
        }
        Command::Cards { what } => {
            let root = locate(cli, cwd)?;
            match what {
                None => {
                    let (_, issues) = taskbag_of(&root, load(&root))?;
                    let outside = allomos_store::cards::outside(&root);
                    let mut done = Done::new(
                        json!({ "taskbag": root, "cards_outside": outside }),
                        cards_text(&outside),
                    );
                    done.issues = issues;
                    Ok(done)
                }
                Some(CardsWhat::Found { items }) => {
                    let items = items
                        .iter()
                        .map(|item| match item.split_once('=') {
                            Some((id, path)) => Ok((id.trim().to_string(), path.to_string())),
                            None => Err(Problem::BadUsage {
                                detail: format!(
                                    "\"{item}\" is not <id>=<path>, for example A01-01=docs/tasks/A01-01.md."
                                ),
                            }),
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    let r =
                        allomos_store::cards::report(&root, Actor::alm(me), lock_wait(), &items)?;
                    let mut text = String::new();
                    for o in &r.found {
                        text.push_str(&format!("Reported: {}  {}\n", o.task, o.path));
                    }
                    for id in &r.withdrawn {
                        text.push_str(&format!("Withdrawn: {id}\n"));
                    }
                    if r.found.is_empty() && r.withdrawn.is_empty() {
                        text.push_str("Nothing new to report.\n");
                    }
                    text.push_str(&cards_text(&r.outside));
                    let mut done = Done::new(
                        json!({
                            "taskbag": root,
                            "found": r.found,
                            "withdrawn": r.withdrawn,
                            "cards_outside": r.outside,
                        }),
                        text,
                    );
                    done.issues = r.issues;
                    Ok(done)
                }
            }
        }
        Command::Card { id, from, replace } => {
            let root = locate(cli, cwd)?;
            let from = from.as_ref().map(|f| cwd.join(f));
            let m = allomos_store::cards::move_in(
                &root,
                Actor::alm(me),
                lock_wait(),
                id,
                from.as_deref(),
                *replace,
            )?;
            let mut text = format!("{}\n", task_line(&m.task));
            text.push_str(&format!(
                "  card file: {} (from {})\n  The description's Card: link points to it.\n",
                m.card,
                plain_path(&m.source).display()
            ));
            if let Some(path) = &m.reported {
                text.push_str(&format!(
                    "alm removes nothing outside the taskbag. When your confirmed plan says so, remove the original from the project folder, for example: git -C {} rm -- {}\n",
                    quoted(&plain_path(&m.project).display().to_string()),
                    quoted(path)
                ));
            } else if let Some(original) = &m.original {
                // Linked from the description, not reported (Task-A01-51).
                text.push_str(&format!(
                    "alm removes nothing outside the taskbag. When your confirmed plan says so, remove the original, for example: git rm -- {}\n",
                    quoted(&plain_path(original).display().to_string())
                ));
            }
            let mut done = Done::new(
                json!({
                    "taskbag": root,
                    "task": task_json(&m.task),
                    "card": m.card,
                    "source": m.source,
                    "reported": m.reported,
                    "original": m.original,
                }),
                text,
            );
            done.issues = m.issues;
            Ok(done)
        }
        Command::Note { id, text } => change(cli, cwd, |bag| {
            ops::set_handoff_note(bag, id, me, text.as_deref())
        }),
        Command::Project { name, id } => {
            let root = locate(cli, cwd)?;
            let lock = Lock::acquire(&root, lock_wait())?;
            let (bag, issues) = taskbag_of(&root, lock.load())?;
            let project = ops::rename_project(&bag, name.as_deref(), id.as_deref())?;
            if name.is_some() || id.is_some() {
                save_project(&lock, me, "project", &bag.project, &project)?;
            }
            let mut done = Done::new(
                json!({ "taskbag": root, "project": { "id": project.id, "name": project.name } }),
                format!("Project: {} ({})\n", project.name, project.id),
            );
            done.issues = issues;
            Ok(done)
        }
        Command::Needs { id, text } => {
            change(cli, cwd, |bag| ops::set_next(bag, id, me, text.as_deref()))
        }
        Command::Plan { id, iteration } => {
            change(cli, cwd, |bag| ops::plan(bag, id, iteration.as_deref()))
        }
        Command::Ready { id } => change(cli, cwd, |bag| ops::make_ready(bag, id, me)),
        Command::Drop { id } => change(cli, cwd, |bag| ops::drop_task(bag, id, me)),
        Command::Reopen { id, note } => change(cli, cwd, |bag| ops::reopen(bag, id, note)),
        Command::Release { id, note } => {
            change(cli, cwd, |bag| ops::release(bag, id, me, note.as_deref()))
        }
    }
}

/// What `alm claim` says about branches: the base and working branch, the Git commands to start
/// from the base and to merge back into it, and facts the agent should check (Task-A03-22). alm
/// runs none of them (decision 0017).
struct BranchSteps {
    text: String,
    start: Vec<String>,
    merge: Vec<String>,
    facts: Vec<String>,
}

fn branch_steps(at: &allomos_core::branch::AtClaim) -> BranchSteps {
    let mut steps = BranchSteps {
        text: String::new(),
        start: Vec::new(),
        merge: Vec::new(),
        facts: Vec::new(),
    };
    let Some(base) = at.branches.base.as_deref() else {
        steps.text.push_str(
            "This task has no base branch yet. Claim it again with --branch <the branch you are on> to record it.\n",
        );
        return steps;
    };
    let why = at.why.as_ref().map(|w| w.english(base)).unwrap_or_default();
    steps
        .text
        .push_str(&format!("  base branch: {base} ({why})\n"));
    let Some(working) = at.branches.working.as_deref() else {
        steps.text.push_str(&format!(
            "Start from {base} and merge back into {base}. Claim it again with --branch <the branch you are on> for the steps (with the Git commands as the example).\n"
        ));
        return steps;
    };
    steps
        .text
        .push_str(&format!("  working branch: {working}\n"));
    let working = Some(working);
    let current = at.current.as_deref();
    let b = quoted(base);
    match working {
        Some(w) if w != base => {
            let wq = quoted(w);
            // Always in a worktree of its own (Task-A03-38): the main checkout stays on the
            // base, so the merge runs there and the worktree is removed afterwards.
            if current != Some(w) {
                steps.start = vec![
                    format!("git switch {b}"),
                    "git pull --ff-only".to_string(),
                    format!("git worktree add <folder> -b {wq} {b}"),
                ];
            }
            steps.merge = vec![
                format!("git switch {b}"),
                "git pull --ff-only".to_string(),
                format!("git merge {wq}"),
                "git push".to_string(),
                "git worktree remove <folder>".to_string(),
                format!("git branch -d {wq}"),
            ];
        }
        _ => {
            if current != Some(base) {
                steps.start.push(format!("git switch {b}"));
            }
            steps.start.push("git pull --ff-only".to_string());
            steps.facts.push(format!(
                "Your working branch is {base}, the task's base. Work every task in a worktree of its own, on a branch of its own: claim again with --working <new branch>."
            ));
        }
    }
    if let Some(c) = current.filter(|c| *c != base) {
        steps
            .facts
            .push(format!("You are on {c}, not on {base}, the task's base."));
    }
    steps.text.push_str(&format!(
        "Start from {base} and merge back into {base} (alm runs no version control tool; for Git, these are the commands):\n"
    ));
    if steps.start.is_empty() {
        steps
            .text
            .push_str("  To start: you are on the working branch already.\n");
    } else {
        steps.text.push_str("  To start:\n");
        for c in &steps.start {
            steps.text.push_str(&format!("    {c}\n"));
        }
        if working.is_some_and(|w| w != base) {
            steps.text.push_str(
                "    (run these in the main checkout, which stays on the base; then work in <folder>)\n",
            );
        }
    }
    if steps.merge.is_empty() {
        steps.text.push_str(&format!(
            "  You work on {base} itself: save and share your changes there (in Git: commit and push); there is nothing to merge.\n"
        ));
    } else {
        steps
            .text
            .push_str("  When the task is done, merge it into its base, in the main checkout:\n");
        for c in &steps.merge {
            steps.text.push_str(&format!("    {c}\n"));
        }
    }
    for f in &steps.facts {
        steps.text.push_str(&format!("Fact: {f}\n"));
    }
    steps
}

/// Rewrite `project.json` under the held lock, recording what changed as `command`'s event.
fn save_project(
    lock: &Lock,
    me: Option<&str>,
    command: &str,
    before: &Project,
    after: &Project,
) -> Result<(), Problem> {
    let event = Actor::alm(me).event(
        command,
        Kind::Project,
        Some(&after.id),
        event::changes(Some(before), after, &[]),
    );
    lock.record(&event, |l| l.save_project(after))
}

/// One change to one task, under the taskbag's lock: load, apply the rule, write it and its event.
/// The app makes its changes through the same `allomos_store::change_task` (Task-A01-18).
fn change(
    cli: &Cli,
    cwd: &Path,
    rule: impl FnOnce(&Taskbag) -> Result<Outcome, Problem>,
) -> Result<Done, Problem> {
    let root = locate(cli, cwd)?;
    let command = cli.command.name();
    let changed = allomos_store::change_task(
        &root,
        Actor::alm(cli.holder.as_deref()),
        lock_wait(),
        |bag| rule(bag).map(|o| (o, command)),
    )?;
    let text = format!("{}\n", task_line(&changed.task));
    let mut done = Done::new(
        json!({ "taskbag": root, "task": task_json(&changed.task) }),
        text,
    );
    done.warnings = changed.warnings;
    done.issues = changed.issues;
    Ok(done)
}

/// What `alm cards` says: the card files agents reported as still in the project folder, and what
/// to do about them (Task-A00-30).
fn cards_text(outside: &[allomos_core::card::Outside]) -> String {
    if outside.is_empty() {
        return "No task's Card: link points outside the taskbag, and no card files are reported as \
kept in the project folder. If you find others, report them with `alm cards found <id>=<path>` \
(`alm guide move-cards`).\n"
            .to_string();
    }
    let mut text = format!(
        "{} task(s) keep their card files in the project folder:\n",
        outside.len()
    );
    for o in outside {
        let how = match o.source {
            allomos_core::card::Source::Link => "its Card: link".to_string(),
            allomos_core::card::Source::Report => format!(
                "reported by {}, {}",
                o.by.as_deref().unwrap_or("?"),
                o.at.as_deref().unwrap_or("?")
            ),
        };
        text.push_str(&format!("  {}  {}  ({how})\n", o.task, o.path));
    }
    text.push_str(
        "Propose moving them in beside their tasks, and do it only after the person's Go ahead: `alm guide move-cards`.\n",
    );
    text
}

/// `alm next`'s line about card files (Task-A00-30): the ones still outside the taskbag (linked
/// or reported). None are: no line at all (Task-A03-34, approver 2026-10-20), so `alm next`
/// prints only what is useful.
/// What `alm next` says when an older taskbag still keeps handoff files (Task-A03-polar770):
/// nothing when there are none.
fn next_handoffs_line(files: &[String]) -> String {
    if files.is_empty() {
        return String::new();
    }
    format!(
        "This taskbag needs updating: its `handoffs/` folder still holds {} old handoff file(s) ({}). A handoff is now written into the task (`alm handoff <id>`). Tell the person: they choose Update the taskbag… in the Allomos app (or you run `alm move-in` and follow the request it prints), which moves what is still useful into the tasks and removes the files after their Go ahead.\n",
        files.len(),
        files.join(", ")
    )
}

fn next_cards_line(outside: usize) -> String {
    if outside > 0 {
        format!(
            "Card files: {outside} task(s) keep their card files in the project folder (`alm cards`): propose moving them in beside their tasks, following `alm guide move-cards`.\n"
        )
    } else {
        String::new()
    }
}
