//! The `allomos` command, also installed as `alm`.
//!
//! It reads options, calls the rules in `allomos-core` and the writer in
//! `allomos-store`, and prints the result: plain lines for people, or one JSON object
//! for agents (`--json`). It never asks for typed input. The contract is in
//! `docs/workflow/CODING_RULES.md` § The `alm` command.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use allomos_core::event::{self, Kind, TASK_FIELDS};
use allomos_core::ops::{self, Outcome};
use allomos_core::{Issue, Iteration, Problem, Project, State, Task, Taskbag};
use allomos_store::handoff;
use allomos_store::{
    Actor, DEFAULT_DIR, GUIDE_TOPICS, Lock, MAX_PLAN_BYTES, add_agent_instructions, agent_section,
    agent_section_for, agent_setup_request, events, find_taskbag, git_root, guide, id_from_name,
    init_with_labels, load, picked_folder, post_confirm, project_folder_of, read_answer,
    requests_dir, task_language, versions, was_seen, withdraw_confirm,
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
    /// After a change, commit the taskbag's files (only those) with Git.
    #[arg(long, global = true)]
    commit: bool,
    /// After a change, commit the taskbag's files and push the current branch.
    #[arg(long, global = true)]
    push: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create an empty taskbag at the root of this Git repository (or in this folder).
    Init {
        /// The project's name. Default: the folder's name.
        #[arg(long)]
        name: Option<String>,
        /// The project's ID. Default: made from the folder's name.
        #[arg(long)]
        id: Option<String>,
        /// The label ladder, lowest step first, separated by commas.
        /// Default: implemented, locally verified, deployed and verified, owner verified.
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
    /// Print Allomos's section for this project's AGENTS.md: the short brief agents read first.
    Brief,
    /// Print the details the brief points to: terms, what a project may customize, commands,
    /// and how to work the tasks. All topics when none is named.
    Guide {
        #[arg(value_parser = GUIDE_TOPICS)]
        topic: Option<String>,
    },
    /// The language to write new and changed tasks in. Without a value: show it (run this before
    /// adding a task or changing its title or description). With one: set it, to a language code
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
    },
    /// Show one task in full.
    Show { id: String },
    /// Show a task's history: every recorded change, newest first, with who and when.
    History { id: String },
    /// Hold a task and start it.
    Claim {
        id: String,
        /// Take the task over from its current holder.
        #[arg(long)]
        take: bool,
    },
    /// Set a task's label (a step of the project's ladder).
    Label { id: String, label: String },
    /// Park a task as waiting, saying on what.
    Wait {
        id: String,
        #[arg(long, value_name = "TEXT")]
        on: String,
    },
    /// Finish a task.
    Done { id: String },
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
    /// Mark an unheld backlog or waiting task ready (for example, the owner approved it).
    Ready { id: String },
    /// Give a task a new title.
    Title { id: String, title: String },
    /// Change the project's name and/or ID; without either, show them.
    Project {
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        id: Option<String>,
    },
    /// Drop a task: it will not be done.
    Drop { id: String },
    /// Show the first ready task that no one holds.
    Next,
    /// Bring a taskbag written by an older Allomos up to this one's format. Run it when `alm`
    /// says the taskbag is older; it adds only what the newer format has.
    Upgrade,
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
    /// Show the project's handoff template, or replace it with --set.
    #[command(name = "handoff-template")]
    HandoffTemplate {
        #[arg(long, value_name = "FILE")]
        set: Option<PathBuf>,
    },
    /// Write this session's handoff (--as): made from the template the first time, the same file
    /// later. Prints its path; fill it in, or pass --from.
    Handoff {
        /// The tasks it covers, separated by commas.
        #[arg(long, value_name = "ID,ID")]
        tasks: Option<String>,
        /// A Markdown file with the handoff's text.
        #[arg(long, value_name = "FILE")]
        from: Option<PathBuf>,
    },
    /// List handoffs, newest first.
    Handoffs {
        /// Only those that name this task.
        #[arg(long, value_name = "ID")]
        task: Option<String>,
        /// Only the newest one.
        #[arg(long)]
        latest: bool,
    },
}

#[derive(Subcommand)]
enum Add {
    /// Add an area.
    Area { id: String, name: String },
    /// Add an iteration (when tasks are planned), ordered after the existing ones.
    Iteration {
        id: String,
        name: String,
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
    },
}

impl Command {
    /// The task or area a command acts on, for the commit message.
    fn target(&self) -> Option<&str> {
        match self {
            Command::Add(Add::Area { id, .. } | Add::Iteration { id, .. })
            | Command::Show { id }
            | Command::History { id }
            | Command::Claim { id, .. }
            | Command::Label { id, .. }
            | Command::Wait { id, .. }
            | Command::Done { id }
            | Command::Release { id, .. }
            | Command::Needs { id, .. }
            | Command::Plan { id, .. }
            | Command::Ready { id }
            | Command::Title { id, .. }
            | Command::Drop { id } => Some(id),
            Command::Add(Add::Task { id, .. }) => id.as_deref(),
            Command::Init { .. }
            | Command::Labels { .. }
            | Command::Project { .. }
            | Command::TaskLanguage { .. }
            | Command::Brief
            | Command::Guide { .. }
            | Command::Confirm { .. }
            | Command::List { .. }
            | Command::Upgrade
            | Command::HandoffRules { .. }
            | Command::HandoffTemplate { .. }
            | Command::Handoff { .. }
            | Command::Handoffs { .. }
            | Command::Next => None,
        }
    }

    /// Whether the command writes to the taskbag.
    fn changes(&self) -> bool {
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
        if let Command::HandoffTemplate { set } = self {
            return set.is_some();
        }
        !matches!(
            self,
            Command::List { .. }
                | Command::Show { .. }
                | Command::History { .. }
                | Command::Next
                | Command::Confirm { .. }
                | Command::Brief
                | Command::Guide { .. }
                | Command::Handoffs { .. }
        )
    }

    fn name(&self) -> &'static str {
        match self {
            Command::Init { .. } => "init",
            Command::Confirm { .. } => "confirm",
            Command::Labels { .. } => "labels",
            Command::TaskLanguage { .. } => "task-language",
            Command::Brief => "brief",
            Command::Guide { .. } => "guide",
            Command::Add(Add::Area { .. }) => "add area",
            Command::Add(Add::Iteration { .. }) => "add iteration",
            Command::Add(Add::Task { .. }) => "add task",
            Command::List { .. } => "list",
            Command::Show { .. } => "show",
            Command::History { .. } => "history",
            Command::Claim { .. } => "claim",
            Command::Label { .. } => "label",
            Command::Wait { .. } => "wait",
            Command::Done { .. } => "done",
            Command::Release { .. } => "release",
            Command::Needs { .. } => "needs",
            Command::Plan { .. } => "plan",
            Command::Ready { .. } => "ready",
            Command::Title { .. } => "title",
            Command::Project { .. } => "project",
            Command::Drop { .. } => "drop",
            Command::Next => "next",
            Command::Upgrade => "upgrade",
            Command::HandoffRules { .. } => "handoff-rules",
            Command::HandoffTemplate { .. } => "handoff-template",
            Command::Handoff { .. } => "handoff",
            Command::Handoffs { .. } => "handoffs",
        }
    }
}

/// What a command produced: fields for the JSON result, the lines for people, and warnings.
struct Done {
    fields: Value,
    text: String,
    warnings: Vec<Problem>,
    issues: Vec<Issue>,
    /// Files outside the taskbag that `--commit` also commits, relative to the taskbag.
    extra: Vec<String>,
}

impl Done {
    fn new(fields: Value, text: String) -> Self {
        Done {
            fields,
            text,
            warnings: Vec::new(),
            issues: Vec::new(),
            extra: Vec::new(),
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
    let wants_json = args.iter().any(|a| a == "--json");
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(e) => {
            if matches!(e.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) {
                let _ = e.print();
                return EXIT_OK;
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
                print_error(None, &problem);
            } else {
                let _ = e.print();
            }
            return EXIT_USAGE;
        }
    };
    let command = cli.command.name();
    match check_versions(&cli, cwd).and_then(|outdated| {
        let mut done = execute(&cli, cwd)?;
        done.warnings.extend(outdated);
        Ok(done)
    }) {
        Ok(mut done) => {
            if (cli.commit || cli.push) && cli.command.changes() {
                if let Some(root) = done.fields["taskbag"].as_str().map(PathBuf::from) {
                    // A task added without an ID: name the one it got.
                    let made = done.fields["task"]["id"].as_str().map(String::from);
                    done.warnings
                        .extend(share(&cli, &root, &done.extra, made.as_deref()));
                }
            }
            if cli.json {
                let mut out = json!({ "schema": SCHEMA, "ok": true, "command": command });
                if let (Some(out), Some(fields)) = (out.as_object_mut(), done.fields.as_object()) {
                    out.extend(fields.clone());
                }
                out["warnings"] = done.warnings.iter().map(problem_json).collect();
                out["issues"] = done.issues.iter().map(issue_json).collect();
                println!("{out}");
            } else {
                print!("{}", done.text);
                for w in &done.warnings {
                    eprintln!("warning [{}]: {}", w.code(), w.english());
                }
                if !done.issues.is_empty() {
                    eprintln!(
                        "note: {} problem(s) in the taskbag's files; see `alm list --json` or the app.",
                        done.issues.len()
                    );
                }
            }
            EXIT_OK
        }
        Err(problem) => {
            if cli.json {
                print_error(Some(command), &problem);
            } else {
                eprintln!("error [{}]: {}", problem.code(), problem.english());
            }
            exit_code(&problem)
        }
    }
}

/// Before a command: is the taskbag in this Allomos's format, and is Allomos's section in the
/// project's instruction files current (Task-A01-25)? A newer or older taskbag stops the command;
/// an out-of-date section gives a warning telling the agent to renew it. Commands that do not work
/// on a taskbag (`init`, `brief`, `guide`, `confirm`), and `upgrade` itself, skip the format check.
fn check_versions(cli: &Cli, cwd: &Path) -> Result<Vec<Problem>, Problem> {
    let skips_format = matches!(
        cli.command,
        Command::Init { .. }
            | Command::Brief
            | Command::Guide { .. }
            | Command::Confirm { .. }
            | Command::Upgrade
    );
    if matches!(
        cli.command,
        Command::Init { .. } | Command::Brief | Command::Guide { .. } | Command::Confirm { .. }
    ) {
        return Ok(Vec::new());
    }
    // No taskbag here: the command itself says so.
    let Ok(root) = locate(cli, cwd) else {
        return Ok(Vec::new());
    };
    if !skips_format {
        match versions::format_state(&root) {
            versions::FormatState::Newer { found } => {
                return Err(Problem::TaskbagNewer {
                    found,
                    supported: allomos_core::FORMAT.to_string(),
                });
            }
            versions::FormatState::Older { found } => {
                return Err(Problem::TaskbagOlder {
                    found,
                    current: allomos_core::FORMAT.to_string(),
                });
            }
            versions::FormatState::Current | versions::FormatState::Unknown => {}
        }
    }
    Ok(versions::check_connected(&root)
        .and_then(|c| c.instructions_warning())
        .into_iter()
        .collect())
}

/// Run `git` in `dir`; on failure, Git's own message.
fn git(dir: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(out)
    } else {
        let text = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if text.is_empty() {
            format!("git {} failed", args.join(" "))
        } else {
            text
        })
    }
}

/// `--commit` / `--push`: commit only the taskbag's files (other changes are left alone), then
/// push if asked. The change itself is already saved, so a Git failure is a warning, not an error.
fn share(cli: &Cli, taskbag: &Path, extra: &[String], made: Option<&str>) -> Vec<Problem> {
    let command = cli.command.name();
    let subject = match cli.command.target().or(made) {
        Some(id) => format!("alm: {command} {id}"),
        None => format!("alm: {command}"),
    };
    let agent = format!(
        "Agent: {}",
        cli.holder.as_deref().unwrap_or("unknown (alm)")
    );
    // The taskbag ("."), plus any other files the command wrote (for `init`, AGENTS.md / CLAUDE.md).
    let paths: Vec<&str> = std::iter::once(".")
        .chain(extra.iter().map(String::as_str))
        .collect();
    let run = |head: &[&str]| {
        let args: Vec<&str> = head
            .iter()
            .copied()
            .chain(["--"])
            .chain(paths.iter().copied())
            .collect();
        git(taskbag, &args)
    };
    let commit = (|| {
        run(&["add", "--all"])?;
        // Nothing staged: the change made no difference, so there is nothing to commit.
        if run(&["diff", "--cached", "--quiet"]).is_ok() {
            return Ok(false);
        }
        run(&["commit", "--quiet", "-m", &subject, "-m", &agent])?;
        Ok(true)
    })();
    match commit {
        Err(detail) => vec![Problem::CommitFailed { detail }],
        Ok(_) if !cli.push => vec![],
        Ok(_) => match git(taskbag, &["push", "--quiet"]) {
            Ok(_) => vec![],
            Err(detail) => vec![Problem::PushFailed { detail }],
        },
    }
}

fn print_error(command: Option<&str>, problem: &Problem) {
    println!(
        "{}",
        json!({ "schema": SCHEMA, "ok": false, "command": command, "error": problem_json(problem) })
    );
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
            let path = cwd.join(path);
            if path.join("project.json").is_file() {
                Ok(path)
            } else if path.join(DEFAULT_DIR).join("project.json").is_file() {
                Ok(path.join(DEFAULT_DIR))
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
/// `--taskbag` (which may not exist yet), else the nearest taskbag's, else the Git repository's
/// root, else `cwd`.
fn confirm_project(cli: &Cli, cwd: &Path) -> PathBuf {
    let taskbag = match &cli.taskbag {
        Some(path) => cwd.join(path),
        None => find_taskbag(cwd).unwrap_or_else(|| {
            git_root(cwd)
                .unwrap_or_else(|| cwd.to_path_buf())
                .join(DEFAULT_DIR)
        }),
    };
    picked_folder(&taskbag)
        .map(Path::to_path_buf)
        .unwrap_or(taskbag)
}

/// How often `alm confirm` looks for the answer, and how long it gives an open app to show the
/// question before opening the app itself.
const CONFIRM_POLL: Duration = Duration::from_millis(250);
const CONFIRM_LAUNCH_AFTER: Duration = Duration::from_secs(3);

/// Open the Allomos app on `project`, so it can show the question. macOS only for now;
/// `ALLOMOS_NO_LAUNCH` turns it off (tests).
fn open_app(project: &Path) {
    if cfg!(target_os = "macos") && std::env::var_os("ALLOMOS_NO_LAUNCH").is_none() {
        let _ = std::process::Command::new("open")
            .args(["-n", "-b", "io.allomos", "--args"])
            .arg(project)
            .status();
    }
}

/// `alm confirm`: leave the question, wait for the person's answer in the app, then tidy up.
fn confirm(cli: &Cli, cwd: &Path, title: &str, plan: &Path, timeout: u64) -> Result<Done, Problem> {
    let project = confirm_project(cli, cwd);
    let path = cwd.join(plan);
    let text = std::fs::read_to_string(&path).map_err(|e| Problem::ReadFailed {
        detail: format!("{}: {e}", path.display()),
    })?;
    if text.len() > MAX_PLAN_BYTES {
        return Err(Problem::BadUsage {
            detail: format!(
                "the plan is longer than {} KB; keep it to what the person must decide",
                MAX_PLAN_BYTES / 1024
            ),
        });
    }
    let asked =
        post_confirm(&project, title, &text, cli.holder.as_deref(), timeout).map_err(|e| {
            Problem::WriteFailed {
                path: requests_dir(&project).display().to_string(),
                detail: e.to_string(),
            }
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
        if let Some(answer) = read_answer(&project, &asked.id) {
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
        Some(true) => Ok(Done::new(
            json!({ "answer": "yes", "project": project }),
            "The person chose Go ahead.\n".to_string(),
        )),
        Some(false) => Err(Problem::ConfirmDeclined),
        None => Err(Problem::ConfirmTimeout {
            seconds: timeout.to_string(),
        }),
    }
}

fn taskbag_of(root: &Path, loaded: allomos_core::Loaded) -> Result<(Taskbag, Vec<Issue>), Problem> {
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

fn task_json(t: &Task) -> Value {
    serde_json::to_value(t).expect("tasks serialize")
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
    s
}

/// One line about a handoff: file, writer, when, tasks, and its first line.
fn handoff_line(h: &handoff::Handoff) -> String {
    let tasks = if h.tasks.is_empty() {
        "no task".to_string()
    } else {
        h.tasks.join(", ")
    };
    format!(
        "handoff: {} ({}, {}, {}): {}\n",
        h.file, h.writer, h.updated, tasks, h.summary
    )
}

/// A handoff for `--json` lists: everything but its text.
fn handoff_brief(h: &handoff::Handoff) -> Value {
    json!({ "file": h.file, "writer": h.writer, "tasks": h.tasks, "written": h.written, "updated": h.updated, "summary": h.summary })
}

/// When to write a handoff, for the agent that asked.
fn rules_line(r: &allomos_core::AppliedRules) -> String {
    let whose = r
        .override_for
        .as_ref()
        .map(|n| format!(" (override for {n})"))
        .unwrap_or_default();
    format!(
        "Handoff rules{whose}: with {}% of the 5-hour quota left, start nothing new, write your handoff (`alm handoff`) and stop; at {}% context used, finish the current step, write your handoff, commit, and give the person the exact prompt for a new session; also write it {}.\n",
        r.quota_gate, r.context_gate, r.write_when
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
                None => git_root(cwd)
                    .unwrap_or_else(|| cwd.to_path_buf())
                    .join(DEFAULT_DIR),
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
            done.extra = agent_files.iter().map(|f| format!("../../{f}")).collect();
            Ok(done)
        }
        Command::Confirm {
            title,
            plan,
            timeout,
        } => confirm(cli, cwd, title, plan, *timeout),
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
            text.push_str(&format!(
                "Template: {}\n",
                root.join(handoff::TEMPLATE_FILE).display()
            ));
            let mut done = Done::new(
                json!({ "taskbag": root, "handoff_rules": rules, "applied": applied }),
                text,
            );
            done.issues = issues;
            Ok(done)
        }
        Command::HandoffTemplate { set } => {
            let root = locate(cli, cwd)?;
            if let Some(file) = set {
                let file = cwd.join(file);
                let text = std::fs::read_to_string(&file).map_err(|e| Problem::ReadFailed {
                    detail: format!("{}: {e}", file.display()),
                })?;
                handoff::set_template(&root, &text)?;
            }
            let text = handoff::template(&root);
            Ok(Done::new(
                json!({ "taskbag": root, "template": text, "path": root.join(handoff::TEMPLATE_FILE) }),
                text,
            ))
        }
        Command::Handoff { tasks, from } => {
            let root = locate(cli, cwd)?;
            let me = me.ok_or(Problem::HolderMissing)?;
            let (bag, issues) = taskbag_of(&root, load(&root))?;
            let known: Vec<String> = bag.tasks.iter().map(|t| t.id.clone()).collect();
            let tasks: Option<Vec<String>> = tasks.as_ref().map(|list| {
                list.split(',')
                    .map(str::trim)
                    .filter(|t| !t.is_empty())
                    .map(String::from)
                    .collect()
            });
            let body = match from {
                Some(file) => {
                    let file = cwd.join(file);
                    Some(
                        std::fs::read_to_string(&file).map_err(|e| Problem::ReadFailed {
                            detail: format!("{}: {e}", file.display()),
                        })?,
                    )
                }
                None => None,
            };
            let w = handoff::write(&root, me, tasks.as_deref(), body.as_deref(), &known, false)?;
            let text = format!(
                "{} {}\n{}",
                if w.created {
                    "Started this session's handoff:"
                } else {
                    "Updated this session's handoff:"
                },
                w.path.display(),
                if body.is_none() {
                    "Fill it in, in plain words first, for any reader. Evidence goes on the task's card.\n"
                } else {
                    ""
                }
            );
            let mut done = Done::new(
                json!({ "taskbag": root, "handoff": w.handoff, "path": w.path, "created": w.created }),
                text,
            );
            done.issues = issues;
            Ok(done)
        }
        Command::Handoffs { task, latest } => {
            let root = locate(cli, cwd)?;
            let mut all = handoff::list(&root);
            if let Some(task) = task {
                all.retain(|h| h.tasks.iter().any(|t| t == task));
            }
            if *latest {
                all.truncate(1);
            }
            let text = if all.is_empty() {
                "No handoffs.\n".to_string()
            } else {
                all.iter().map(handoff_line).collect()
            };
            Ok(Done::new(
                json!({ "taskbag": root, "handoffs": all.iter().map(handoff_brief).collect::<Vec<_>>() }),
                text,
            ))
        }
        Command::Upgrade => {
            let root = locate(cli, cwd)?;
            let up = versions::upgrade(&root, Actor::alm(me))?;
            let text = if up.changed {
                format!("Upgraded the taskbag from {} to {}.\n", up.from, up.to)
            } else {
                format!("The taskbag is already in {}; nothing to upgrade.\n", up.to)
            };
            Ok(Done::new(json!({ "taskbag": root, "upgrade": up }), text))
        }
        Command::Guide { topic } => {
            let text = guide(topic.as_deref()).unwrap_or_default();
            Ok(Done::new(json!({ "topic": topic, "guide": text }), text))
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
            let lock = Lock::acquire(&root, lock_wait())?;
            let (bag, issues) = taskbag_of(&root, lock.load())?;
            let mut done = match add {
                Add::Area { id, name } => {
                    let area = ops::new_area(&bag, id, name)?;
                    let event = Actor::alm(me).event(
                        "add area",
                        Kind::Area,
                        Some(&area.id),
                        event::changes(None, &area, &[]),
                    );
                    lock.record(&event, |l| l.create_area(&area))?;
                    let text = format!("Added area {} ({})\n", area.id, area.name);
                    Done::new(json!({ "taskbag": root, "area": area }), text)
                }
                Add::Iteration {
                    id,
                    name,
                    release,
                    start,
                    end,
                } => {
                    let details = ops::IterationDetails {
                        release: release.as_deref(),
                        start: start.as_deref(),
                        end: end.as_deref(),
                    };
                    let iteration = ops::new_iteration(&bag, id, name, details)?;
                    let event = Actor::alm(me).event(
                        "add iteration",
                        Kind::Iteration,
                        Some(&iteration.id),
                        event::changes(None, &iteration, &[]),
                    );
                    lock.record(&event, |l| l.create_iteration(&iteration))?;
                    let text = format!("Added iteration {}\n", iteration_heading(&iteration));
                    Done::new(json!({ "taskbag": root, "iteration": iteration }), text)
                }
                Add::Task {
                    id,
                    area,
                    title,
                    description,
                } => {
                    let id = match id {
                        Some(id) => id.clone(),
                        None => ops::next_task_id(&bag, area)?,
                    };
                    let task = ops::new_task(&bag, &id, area, title, description.as_deref())?;
                    let event = Actor::alm(me).event(
                        "add task",
                        Kind::Task,
                        Some(&task.id),
                        event::changes(None, &task, TASK_FIELDS),
                    );
                    lock.record(&event, |l| l.create_task(&task))?;
                    let text = format!("Added {}\n", task_line(&task));
                    Done::new(json!({ "taskbag": root, "task": task_json(&task) }), text)
                }
            };
            done.issues = issues;
            Ok(done)
        }
        Command::List {
            area,
            iteration,
            by,
            state,
            mine,
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
            };
            let mut tasks: Vec<&Task> = bag.tasks.iter().filter(keep).collect();
            tasks.sort_by(|a, b| a.id.cmp(&b.id));
            let mut text = String::new();
            let mut group = |heading: String, members: &[&&Task]| {
                if members.is_empty() {
                    return;
                }
                text.push_str(&heading);
                text.push('\n');
                for t in members {
                    text.push_str(&format!("  {}\n", task_line(t)));
                }
            };
            let tasks_json =
                |members: &[&&Task]| members.iter().map(|t| task_json(t)).collect::<Vec<_>>();
            let fields = if by == "iteration" {
                let mut groups = Vec::new();
                for it in &bag.iterations {
                    if iteration.as_ref().is_some_and(|x| x != &it.id) {
                        continue;
                    }
                    let members: Vec<&&Task> = tasks
                        .iter()
                        .filter(|t| t.iteration.as_ref() == Some(&it.id))
                        .collect();
                    group(iteration_heading(it), &members);
                    let mut g = json!(it);
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
            let mut done = Done::new(fields, text);
            done.issues = issues;
            Ok(done)
        }
        Command::Show { id } => {
            let root = locate(cli, cwd)?;
            let (bag, issues) = taskbag_of(&root, load(&root))?;
            let task = ops::find_task(&bag, id)?;
            let mut text = String::new();
            let fields = [
                ("id", Some(task.id.as_str())),
                ("title", Some(task.title.as_str())),
                ("area", Some(task.area.as_str())),
                ("iteration", task.iteration.as_deref()),
                ("state", Some(task.state.as_str())),
                ("label", task.label.as_deref()),
                ("holder", task.holder.as_deref()),
                ("waiting on", task.waiting_on.as_deref()),
                ("next dependency", task.next_dependency.as_deref()),
                ("handoff note", task.handoff_note.as_deref()),
            ];
            for (name, value) in fields {
                if let Some(v) = value {
                    text.push_str(&format!("{name}: {v}\n"));
                }
            }
            let latest = handoff::latest_for(&root, &task.id);
            if let Some(h) = &latest {
                text.push_str(&handoff_line(h));
            }
            if !task.description.is_empty() {
                text.push('\n');
                text.push_str(&task.description);
            }
            let mut done = Done::new(
                json!({ "taskbag": root, "task": task_json(&task), "handoff": latest.as_ref().map(handoff_brief) }),
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
        Command::Next => {
            let root = locate(cli, cwd)?;
            let (bag, issues) = taskbag_of(&root, load(&root))?;
            let task = ops::next(&bag);
            let latest = task.and_then(|t| handoff::latest_for(&root, &t.id));
            let mut text = match task {
                Some(t) => {
                    let mut s = format!("{}\n", task_line(t));
                    if let Some(note) = &t.handoff_note {
                        s.push_str(&format!("  handoff note: {note}\n"));
                    }
                    if let Some(h) = &latest {
                        s.push_str(&format!("  {}", handoff_line(h)));
                    }
                    s
                }
                None => "No ready task is free.\n".to_string(),
            };
            // At the start of a session: when this agent must write its handoff (Task-A03-06).
            let rules = bag.project.handoff.clone().unwrap_or_default().applied(me);
            text.push_str(&rules_line(&rules));
            let mut done = Done::new(
                json!({
                    "taskbag": root,
                    "task": task.map(task_json),
                    "handoff": latest.as_ref().map(handoff_brief),
                    "handoff_rules": rules,
                }),
                text,
            );
            done.issues = issues;
            Ok(done)
        }
        Command::Claim { id, take } => change(cli, cwd, |bag| ops::claim(bag, id, me, *take)),
        Command::Label { id, label } => change(cli, cwd, |bag| ops::set_label(bag, id, me, label)),
        Command::Wait { id, on } => change(cli, cwd, |bag| ops::wait(bag, id, me, on)),
        Command::Done { id } => change(cli, cwd, |bag| ops::finish(bag, id, me)),
        Command::Title { id, title } => change(cli, cwd, |bag| ops::set_title(bag, id, me, title)),
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
        Command::Release { id, note } => {
            change(cli, cwd, |bag| ops::release(bag, id, me, note.as_deref()))
        }
    }
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
fn change(
    cli: &Cli,
    cwd: &Path,
    rule: impl FnOnce(&Taskbag) -> Result<Outcome, Problem>,
) -> Result<Done, Problem> {
    let root = locate(cli, cwd)?;
    let lock = Lock::acquire(&root, lock_wait())?;
    let (bag, issues) = taskbag_of(&root, lock.load())?;
    let outcome = rule(&bag)?;
    let before = bag.tasks.iter().find(|t| t.id == outcome.task.id);
    let event = Actor::alm(cli.holder.as_deref()).event(
        cli.command.name(),
        Kind::Task,
        Some(&outcome.task.id),
        event::changes(before, &outcome.task, TASK_FIELDS),
    );
    lock.record(&event, |l| l.save_task(&outcome.task))?;
    drop(lock);
    let text = format!("{}\n", task_line(&outcome.task));
    let mut done = Done::new(
        json!({ "taskbag": root, "task": task_json(&outcome.task) }),
        text,
    );
    done.warnings = outcome.warnings;
    done.issues = issues;
    Ok(done)
}
