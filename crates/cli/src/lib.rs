//! The `fliphopper` command, also installed as `fh`.
//!
//! It reads options, calls the rules in `fliphopper-core` and the writer in
//! `fliphopper-store`, and prints the result: plain lines for people, or one JSON object
//! for agents (`--json`). It never asks for typed input. The contract is in
//! `docs/workflow/CODING_RULES.md` § The `fh` command.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use clap::{Parser, Subcommand, error::ErrorKind};
use fliphopper_core::ops::{self, Outcome};
use fliphopper_core::{Issue, Iteration, Problem, Seabag, State, Task};
use fliphopper_store::{
    DEFAULT_DIR, Lock, MAX_PLAN_BYTES, add_agent_instructions, agent_setup_request, find_seabag,
    git_root, id_from_name, init_with_labels, load, picked_folder, post_confirm, read_answer,
    requests_dir, was_seen, withdraw_confirm,
};
use serde_json::{Value, json};

/// The version of the `--json` output. A new field is an addition; anything else changes it.
pub const SCHEMA: &str = "fh/1";

/// Exit codes.
pub const EXIT_OK: i32 = 0;
pub const EXIT_REFUSED: i32 = 1;
pub const EXIT_USAGE: i32 = 2;
pub const EXIT_BUSY: i32 = 3;

/// How long a writer waits for another one to finish, unless `FLIPHOPPER_LOCK_WAIT_MS` says otherwise.
const LOCK_WAIT_MS: u64 = 5000;

#[derive(Parser)]
#[command(
    name = "fh",
    version,
    about = "Work a FlipHopper seabag's tasks from a terminal.",
    after_help = "Exit codes: 0 done, 1 refused by a rule, 2 wrong usage, 3 seabag busy or not writable."
)]
struct Cli {
    /// The seabag folder, or a folder holding .fliphopper/seabag/. Default: search upward from here.
    #[arg(long, global = true, env = "FLIPHOPPER_SEABAG", value_name = "PATH")]
    seabag: Option<PathBuf>,
    /// Who you are, for example claude/a01-03-fh.
    #[arg(
        long = "as",
        global = true,
        env = "FLIPHOPPER_HOLDER",
        value_name = "NAME"
    )]
    holder: Option<String>,
    /// Print one JSON result on standard output, and nothing else.
    #[arg(long, global = true)]
    json: bool,
    /// After a change, commit the seabag's files (only those) with Git.
    #[arg(long, global = true)]
    commit: bool,
    /// After a change, commit the seabag's files and push the current branch.
    #[arg(long, global = true)]
    push: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create an empty seabag at the root of this Git repository (or in this folder).
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
        /// Write FlipHopper's section for AI agents into AGENTS.md and CLAUDE.md yourself,
        /// instead of printing a request for the agent running `fh init` to do it.
        #[arg(long)]
        write_agent_files: bool,
    },
    /// Add an area or a task.
    #[command(subcommand)]
    Add(Add),
    /// Show a plan to the person in the FlipHopper app and wait for their answer: exit 0 means
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
    /// Drop a task: it will not be done.
    Drop { id: String },
    /// Show the first ready task that no one holds.
    Next,
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
        id: String,
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
            Command::Add(
                Add::Area { id, .. } | Add::Iteration { id, .. } | Add::Task { id, .. },
            )
            | Command::Show { id }
            | Command::Claim { id, .. }
            | Command::Label { id, .. }
            | Command::Wait { id, .. }
            | Command::Done { id }
            | Command::Release { id, .. }
            | Command::Needs { id, .. }
            | Command::Plan { id, .. }
            | Command::Ready { id }
            | Command::Drop { id } => Some(id),
            Command::Init { .. }
            | Command::Confirm { .. }
            | Command::List { .. }
            | Command::Next => None,
        }
    }

    /// Whether the command writes to the seabag.
    fn changes(&self) -> bool {
        !matches!(
            self,
            Command::List { .. } | Command::Show { .. } | Command::Next | Command::Confirm { .. }
        )
    }

    fn name(&self) -> &'static str {
        match self {
            Command::Init { .. } => "init",
            Command::Confirm { .. } => "confirm",
            Command::Add(Add::Area { .. }) => "add area",
            Command::Add(Add::Iteration { .. }) => "add iteration",
            Command::Add(Add::Task { .. }) => "add task",
            Command::List { .. } => "list",
            Command::Show { .. } => "show",
            Command::Claim { .. } => "claim",
            Command::Label { .. } => "label",
            Command::Wait { .. } => "wait",
            Command::Done { .. } => "done",
            Command::Release { .. } => "release",
            Command::Needs { .. } => "needs",
            Command::Plan { .. } => "plan",
            Command::Ready { .. } => "ready",
            Command::Drop { .. } => "drop",
            Command::Next => "next",
        }
    }
}

/// What a command produced: fields for the JSON result, the lines for people, and warnings.
struct Done {
    fields: Value,
    text: String,
    warnings: Vec<Problem>,
    issues: Vec<Issue>,
    /// Files outside the seabag that `--commit` also commits, relative to the seabag.
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
        Problem::BadUsage { .. } => EXIT_USAGE,
        Problem::SeabagBusy { .. }
        | Problem::WriteFailed { .. }
        | Problem::SeabagUnreadable { .. }
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

/// Run `fh` with these arguments from `cwd`; print the result and return the exit code.
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
    match execute(&cli, cwd) {
        Ok(mut done) => {
            if (cli.commit || cli.push) && cli.command.changes() {
                if let Some(root) = done.fields["seabag"].as_str().map(PathBuf::from) {
                    done.warnings.extend(share(&cli, &root, &done.extra));
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
                        "note: {} problem(s) in the seabag's files; see `fh list --json` or the app.",
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

/// `--commit` / `--push`: commit only the seabag's files (other changes are left alone), then
/// push if asked. The change itself is already saved, so a Git failure is a warning, not an error.
fn share(cli: &Cli, seabag: &Path, extra: &[String]) -> Vec<Problem> {
    let command = cli.command.name();
    let subject = match cli.command.target() {
        Some(id) => format!("fh: {command} {id}"),
        None => format!("fh: {command}"),
    };
    let agent = format!("Agent: {}", cli.holder.as_deref().unwrap_or("unknown (fh)"));
    // The seabag ("."), plus any other files the command wrote (for `init`, AGENTS.md / CLAUDE.md).
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
        git(seabag, &args)
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
        Ok(_) => match git(seabag, &["push", "--quiet"]) {
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
    let ms = std::env::var("FLIPHOPPER_LOCK_WAIT_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(LOCK_WAIT_MS);
    Duration::from_millis(ms)
}

/// The seabag to work on: `--seabag` / `FLIPHOPPER_SEABAG` (the seabag itself, or a folder
/// holding `.fliphopper/seabag/`), else the nearest one at or above `cwd`.
fn locate(cli: &Cli, cwd: &Path) -> Result<PathBuf, Problem> {
    let not_found = |p: &Path| Problem::SeabagNotFound {
        path: p.display().to_string(),
    };
    match &cli.seabag {
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
        None => find_seabag(cwd).ok_or_else(|| not_found(cwd)),
    }
}

/// The project folder `fh confirm` asks about: the folder holding the seabag given with
/// `--seabag` (which may not exist yet), else the nearest seabag's, else the Git repository's
/// root, else `cwd`.
fn confirm_project(cli: &Cli, cwd: &Path) -> PathBuf {
    let seabag = match &cli.seabag {
        Some(path) => cwd.join(path),
        None => find_seabag(cwd).unwrap_or_else(|| {
            git_root(cwd)
                .unwrap_or_else(|| cwd.to_path_buf())
                .join(DEFAULT_DIR)
        }),
    };
    picked_folder(&seabag)
        .map(Path::to_path_buf)
        .unwrap_or(seabag)
}

/// How often `fh confirm` looks for the answer, and how long it gives an open app to show the
/// question before opening the app itself.
const CONFIRM_POLL: Duration = Duration::from_millis(250);
const CONFIRM_LAUNCH_AFTER: Duration = Duration::from_secs(3);

/// Open the FlipHopper app on `project`, so it can show the question. macOS only for now;
/// `FLIPHOPPER_NO_LAUNCH` turns it off (tests).
fn open_app(project: &Path) {
    if cfg!(target_os = "macos") && std::env::var_os("FLIPHOPPER_NO_LAUNCH").is_none() {
        let _ = std::process::Command::new("open")
            .args(["-n", "-b", "io.fliphopper", "--args"])
            .arg(project)
            .status();
    }
}

/// `fh confirm`: leave the question, wait for the person's answer in the app, then tidy up.
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
            "Waiting for the person to answer in the FlipHopper app (project {})…",
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

fn seabag_of(
    root: &Path,
    loaded: fliphopper_core::Loaded,
) -> Result<(Seabag, Vec<Issue>), Problem> {
    match loaded.seabag {
        Some(bag) => Ok((bag, loaded.issues)),
        None => Err(Problem::SeabagUnreadable {
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
            let root = match &cli.seabag {
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
                None => fliphopper_core::default_labels(),
            };
            let root = init_with_labels(&root, &id, &name, labels)?;
            let mut text = format!("Created an empty seabag at {}\n", root.display());
            // Tell AI agents about FlipHopper in the project's instruction files: by default ask
            // the agent running `fh init` to merge the section in; with --write-agent-files, write it.
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
                    "Added FlipHopper's section for AI agents to {}\n",
                    agent_files.join(" and ")
                ));
            }
            if let Some(request) = &agent_setup {
                text.push_str("\nNext step for the AI agent running this command:\n\n");
                text.push_str(request);
            }
            let mut done = Done::new(
                json!({ "seabag": root, "agent_files": agent_files, "agent_setup": agent_setup }),
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
        Command::Add(add) => {
            let root = locate(cli, cwd)?;
            let lock = Lock::acquire(&root, lock_wait())?;
            let (bag, issues) = seabag_of(&root, lock.load())?;
            let mut done = match add {
                Add::Area { id, name } => {
                    let area = ops::new_area(&bag, id, name)?;
                    lock.create_area(&area)?;
                    let text = format!("Added area {} ({})\n", area.id, area.name);
                    Done::new(json!({ "seabag": root, "area": area }), text)
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
                    lock.create_iteration(&iteration)?;
                    let text = format!("Added iteration {}\n", iteration_heading(&iteration));
                    Done::new(json!({ "seabag": root, "iteration": iteration }), text)
                }
                Add::Task {
                    id,
                    area,
                    title,
                    description,
                } => {
                    let task = ops::new_task(&bag, id, area, title, description.as_deref())?;
                    lock.create_task(&task)?;
                    let text = format!("Added {}\n", task_line(&task));
                    Done::new(json!({ "seabag": root, "task": task_json(&task) }), text)
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
            let (bag, issues) = seabag_of(&root, load(&root))?;
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
                json!({ "seabag": root, "iterations": groups, "unplanned": tasks_json(&unplanned) })
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
                json!({ "seabag": root, "areas": groups })
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
            let (bag, issues) = seabag_of(&root, load(&root))?;
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
            if !task.description.is_empty() {
                text.push('\n');
                text.push_str(&task.description);
            }
            let mut done = Done::new(json!({ "seabag": root, "task": task_json(&task) }), text);
            done.issues = issues;
            Ok(done)
        }
        Command::Next => {
            let root = locate(cli, cwd)?;
            let (bag, issues) = seabag_of(&root, load(&root))?;
            let task = ops::next(&bag);
            let text = match task {
                Some(t) => {
                    let mut s = format!("{}\n", task_line(t));
                    if let Some(note) = &t.handoff_note {
                        s.push_str(&format!("  handoff note: {note}\n"));
                    }
                    s
                }
                None => "No ready task is free.\n".to_string(),
            };
            let mut done = Done::new(json!({ "seabag": root, "task": task.map(task_json) }), text);
            done.issues = issues;
            Ok(done)
        }
        Command::Claim { id, take } => change(cli, cwd, |bag| ops::claim(bag, id, me, *take)),
        Command::Label { id, label } => change(cli, cwd, |bag| ops::set_label(bag, id, me, label)),
        Command::Wait { id, on } => change(cli, cwd, |bag| ops::wait(bag, id, me, on)),
        Command::Done { id } => change(cli, cwd, |bag| ops::finish(bag, id, me)),
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

/// One change to one task, under the seabag's lock: load, apply the rule, write.
fn change(
    cli: &Cli,
    cwd: &Path,
    rule: impl FnOnce(&Seabag) -> Result<Outcome, Problem>,
) -> Result<Done, Problem> {
    let root = locate(cli, cwd)?;
    let lock = Lock::acquire(&root, lock_wait())?;
    let (bag, issues) = seabag_of(&root, lock.load())?;
    let outcome = rule(&bag)?;
    lock.save_task(&outcome.task)?;
    drop(lock);
    let text = format!("{}\n", task_line(&outcome.task));
    let mut done = Done::new(
        json!({ "seabag": root, "task": task_json(&outcome.task) }),
        text,
    );
    done.warnings = outcome.warnings;
    done.issues = issues;
    Ok(done)
}
