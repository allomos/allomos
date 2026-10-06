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
use fliphopper_core::{Issue, Problem, Seabag, State, Task};
use fliphopper_store::{DEFAULT_DIR, Lock, find_seabag, git_root, id_from_name, init, load};
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
    },
    /// Add an area or a task.
    #[command(subcommand)]
    Add(Add),
    /// List tasks, grouped by area.
    List {
        #[arg(long)]
        area: Option<String>,
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
    /// Show the first ready task that no one holds.
    Next,
}

#[derive(Subcommand)]
enum Add {
    /// Add an area.
    Area { id: String, name: String },
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
    fn name(&self) -> &'static str {
        match self {
            Command::Init { .. } => "init",
            Command::Add(Add::Area { .. }) => "add area",
            Command::Add(Add::Task { .. }) => "add task",
            Command::List { .. } => "list",
            Command::Show { .. } => "show",
            Command::Claim { .. } => "claim",
            Command::Label { .. } => "label",
            Command::Wait { .. } => "wait",
            Command::Done { .. } => "done",
            Command::Release { .. } => "release",
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
}

impl Done {
    fn new(fields: Value, text: String) -> Self {
        Done {
            fields,
            text,
            warnings: Vec::new(),
            issues: Vec::new(),
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
        Ok(done) => {
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
        Command::Init { name } => {
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
            let root = init(&root, &id_from_name(&folder), &name)?;
            let text = format!("Created an empty seabag at {}\n", root.display());
            Ok(Done::new(json!({ "seabag": root }), text))
        }
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
        Command::List { area, state, mine } => {
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
            let keep = |t: &&Task| {
                area.as_ref().is_none_or(|a| &t.area == a)
                    && state.as_ref().is_none_or(|s| t.state.as_str() == s)
                    && (!*mine || t.holder.as_deref() == me)
            };
            let mut tasks: Vec<&Task> = bag.tasks.iter().filter(keep).collect();
            tasks.sort_by(|a, b| a.id.cmp(&b.id));
            let mut text = String::new();
            let mut areas = Vec::new();
            for a in &bag.areas {
                let mine: Vec<&&Task> = tasks.iter().filter(|t| t.area == a.id).collect();
                if area.as_ref().is_some_and(|x| x != &a.id) {
                    continue;
                }
                areas.push(json!({
                    "id": a.id, "name": a.name, "order": a.order,
                    "tasks": mine.iter().map(|t| task_json(t)).collect::<Vec<_>>(),
                }));
                if mine.is_empty() {
                    continue;
                }
                text.push_str(&format!("{}  {}\n", a.id, a.name));
                for t in mine {
                    text.push_str(&format!("  {}\n", task_line(t)));
                }
            }
            if text.is_empty() {
                text.push_str("No tasks.\n");
            }
            let mut done = Done::new(json!({ "seabag": root, "areas": areas }), text);
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
