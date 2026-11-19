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

use allomos_core::event::{self, Kind};
use allomos_core::ops::{self, Outcome};
use allomos_core::{Issue, Problem, Project, Task, Taskbag};
use allomos_store::{
    Actor, Lock, default_root, events, find_taskbag, load, quota_ask, quota_reports, taskbag_of,
    tidy, versions, with_written_files,
};
use clap::{Parser, error::ErrorKind};
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
    /// The exact model ID you, the agent, run on (for example claude-opus-5-5), recorded in each
    /// event this command writes (Task-A01-black004). Once is enough per session on this computer:
    /// alm remembers it for your session ID.
    #[arg(long, global = true, env = "ALLOMOS_MODEL", value_name = "ID")]
    model: Option<String>,
    /// Your reasoning effort (for example medium), recorded in each event (Task-A01-urchin867).
    /// Claude Code tells alm by itself (CLAUDE_EFFORT); once per session is enough otherwise.
    #[arg(long, global = true, env = "ALLOMOS_EFFORT", value_name = "LEVEL")]
    effort: Option<String>,
    /// Your permission mode (for example auto, default, plan), recorded in each event. Once per
    /// session on this computer is enough.
    #[arg(
        long,
        global = true,
        env = "ALLOMOS_PERMISSION_MODE",
        value_name = "MODE"
    )]
    permission_mode: Option<String>,
    /// The commit hash of the AGENTS.md you work under (in Git: `git log -1 --format=%H --
    /// AGENTS.md`), recorded in each event. Once per session on this computer is enough.
    #[arg(long, global = true, env = "ALLOMOS_RULES", value_name = "COMMIT")]
    rules: Option<String>,
    /// The pseudonym ID of the person you work for, when it is not this computer's person
    /// (whose pseudonym alm records by itself).
    #[arg(
        long = "for",
        global = true,
        env = "ALLOMOS_FOR",
        value_name = "PSEUDONYM"
    )]
    on_behalf_of: Option<String>,
    /// The approval ID this change relies on, when it needs one.
    #[arg(long, global = true, env = "ALLOMOS_APPROVAL", value_name = "ID")]
    approval: Option<String>,
    /// Why you make this change, in one line (Task-A01-air289), recorded as a `reason` in its
    /// history. State changes (drop, release, backlog, ready, wait, block, unblock…), taking a
    /// label off and plan changes (plan, block, base) ask for one; alm records it and refuses
    /// nothing without it. reopen --note, recheck --reason, wait --on and done
    /// --without-human-check already say why.
    #[arg(long, global = true, value_name = "TEXT")]
    why: Option<String>,
    #[command(subcommand)]
    command: Command,
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
    // A session coming back (decision 0022 § 8, Task-A01-castle658): the tasks taken over from
    // it are named first, and its changes to them refused.
    let taken = takeover::look(&cli, cwd);
    // Two claims of one task that met (Task-A01-lawn320): said first, with the next ready task.
    let races = race::look(&cli, cwd);
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
            takeover::refuse(&cli, &taken)?;
            if let Some((id, input)) = &request {
                if let Some(first) = repeated(&cli, cwd, id, input) {
                    return first;
                }
                allomos_store::events::set_request(Some((id, input)));
            }
            allomos_store::events::set_who(Some(allomos_store::events::Who {
                model: cli.model.clone(),
                effort: cli.effort.clone(),
                permission_mode: cli.permission_mode.clone(),
                rules: cli.rules.clone(),
                alm: Some(env!("CARGO_PKG_VERSION").to_string()),
                on_behalf_of: cli.on_behalf_of.clone(),
                approval: cli.approval.clone(),
                reason: command_reason(&cli),
            }));
            let _ = allomos_store::events::take_gaps();
            let done = execute(&cli, cwd);
            allomos_store::events::set_request(None);
            allomos_store::events::set_who(None);
            let mut done = done?;
            done.warnings.extend(outdated);
            pack_history(&cli, cwd, &mut done);
            trail::auto_checkpoint(&cli, cwd, &mut done);
            Ok(done)
        })
    });
    let code = match result {
        Ok(mut done) => {
            share(&cli, cwd, &before, &written, &mut done);
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
            // An agent that changes the taskbag is still at work: its open progress report counts
            // as fresh, so the app does not say it went quiet (Task-A03-elixir705, approved).
            if cli.command.changes() {
                allomos_store::progress::touch(&confirm_project(&cli, cwd), cli.holder.as_deref());
            }
            if let Some(lines) = missing_facts(&cli, cwd, &mut done) {
                done.text.push_str(&lines);
            }
            if let Some(lines) = who_facts(&mut done) {
                done.text.push_str(&lines);
            }
            if moves_on {
                // The agent's id comes from the task's session (`--session-agent`), not from the
                // session name alone (Task-A03-possum352).
                let session_agent = cli.command.target().and_then(|id| {
                    let bag = load(&locate(&cli, cwd).ok()?).taskbag?;
                    let task = bag.tasks.into_iter().find(|t| t.id == id)?;
                    task.session?.agent
                });
                if let Some(ask) = quota_ask(
                    session_agent.as_deref(),
                    cli.holder.as_deref(),
                    &quota_reports(),
                    &events::now_utc(),
                ) {
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
                out["warnings"] = races
                    .races
                    .iter()
                    .map(|r| r.notice())
                    .chain(taken.iter().map(|t| t.notice()))
                    .chain(done.warnings.iter().cloned())
                    .map(|w| problem_json(&w))
                    .collect();
                out["issues"] = done.issues.iter().map(issue_json).collect();
                if !taken.is_empty() {
                    out["taken_over"] = takeover::json(&taken);
                }
                race::json(&races, &mut out);
                stdout.push_str(&format!("{out}\n"));
            } else {
                stdout.push_str(&race::text(&races));
                stdout.push_str(&takeover::text(&taken));
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
            // A claim refused because another session holds the task already (Task-A01-lawn320):
            // the next ready task, for the session to choose from.
            let next_ready = match (&cli.command, &problem) {
                (Command::Claim { .. }, Problem::TaskHeld { id, .. }) => {
                    locate(&cli, cwd).ok().and_then(|root| {
                        let bag = load(&root).taskbag?;
                        Some(race::next_ready(&root, &bag, cwd, &[id.as_str()]))
                    })
                }
                _ => None,
            };
            if cli.json {
                let mut line: Value = serde_json::from_str(&error_line(Some(command), &problem))
                    .expect("error lines are JSON");
                race::json(&races, &mut line);
                if let Some(next) = &next_ready {
                    line["next_ready"] = race::next_json(next);
                }
                if let Some(task) = &current {
                    line["current"] = task_json(task);
                }
                if !taken.is_empty() {
                    line["taken_over"] = takeover::json(&taken);
                }
                stdout.push_str(&format!("{line}\n"));
            } else {
                stderr.push_str(&race::text(&races));
                stderr.push_str(&takeover::text(&taken));
                stderr.push_str(&format!(
                    "error [{}]: {}\n",
                    problem.code(),
                    problem.english()
                ));
                if let Some(next) = &next_ready {
                    stderr.push_str(&race::next_line(next));
                }
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
/// checked; nor is `monitor list`, which lists this computer's runs in every project (Task-A03-motor529).
fn check_versions(cli: &Cli, cwd: &Path) -> Result<Vec<Problem>, Problem> {
    if matches!(
        cli.command,
        Command::Init { .. }
            | Command::Brief
            | Command::Quota { .. }
            | Command::Guide { .. }
            | Command::Confirm { .. }
            | Command::MoveIn { .. }
            | Command::Monitor {
                what: monitor::MonitorWhat::List
            }
            | Command::Monitor {
                what: monitor::MonitorWhat::CloseTab { .. },
            }
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

/// One line about a task: ID, state, label, holder, title.
mod approve;
mod claim;
mod command;
mod command_args;
mod command_meta;
mod confirm;
mod plan;
mod project;
mod settings;
mod share;
mod tasks;
mod text;
mod views;
use command_meta::id_and_name;
use confirm::{confirm, confirm_project};
pub(crate) use share::{Before, for_codex, git_heading, quoted, share};
pub(crate) use text::{
    blocked_text, decision_lines, decision_text, difficulty_level, difficulty_range,
    finished_blockers_text, human_task_line, indent, iteration_heading, notes_reminder, rules_line,
    stop_lines, task_json, task_json_in, task_line, traces_text,
};
mod check_results;
mod classify;
mod effects;
mod exceptions;
mod folders;
mod heartbeat;
mod human;
mod links;
mod monitor;
mod race;
mod redact;
mod risk_rules;
mod takeover;
mod trail;
pub(crate) use command::Command;
pub(crate) use command_args::{
    Add, CardsWhat, ChangeWhat, Delete, FolderWhat, LabelsWhat, MoveWhat, Rename,
};

fn execute(cli: &Cli, cwd: &Path) -> Result<Done, Problem> {
    match &cli.command {
        Command::Shared { through } => {
            let root = locate(cli, cwd)?;
            let changed = allomos_store::app_sharing::confirm(
                &root,
                Actor::alm(cli.holder.as_deref()),
                lock_wait(),
                through,
            )?;
            Ok(Done::new(
                json!({"taskbag": root, "changed": changed, "through": through}),
                format!(
                    "App changes shared through {through}. Confirmation recorded; alm did not inspect Git.\n"
                ),
            ))
        }
        Command::Monitor { what } => monitor::execute(cli, cwd, what),
        Command::Init { .. }
        | Command::Confirm { .. }
        | Command::Progress { .. }
        | Command::Brief
        | Command::Handoff { .. }
        | Command::MoveIn { .. }
        | Command::Translate { .. }
        | Command::Guide { .. } => project::execute(cli, cwd),
        Command::Quota { .. }
        | Command::HandoffRules { .. }
        | Command::HeartbeatSettings { .. }
        | Command::FileSizeSettings { .. }
        | Command::AuditSettings { .. }
        | Command::TaskLanguage { .. }
        | Command::Labels { .. } => settings::execute(cli, cwd),
        Command::Add(_)
        | Command::Rename(_)
        | Command::Change(_)
        | Command::Move(_)
        | Command::Delete(_) => plan::execute(cli, cwd),
        Command::List { .. }
        | Command::Show { .. }
        | Command::History { .. }
        | Command::Next { .. } => views::execute(cli, cwd),
        Command::Claim { .. } | Command::Base { .. } => claim::execute(cli, cwd),
        Command::Trace { .. }
        | Command::Traces { .. }
        | Command::Label { .. }
        | Command::Session { .. }
        | Command::Wait { .. }
        | Command::Block { .. }
        | Command::Unblock { .. }
        | Command::Done { .. }
        | Command::Checked { .. }
        | Command::Recheck { .. }
        | Command::AiDecided { .. }
        | Command::Approve { .. }
        | Command::Withdraw { .. }
        | Command::Effect { .. }
        | Command::Effects { .. }
        | Command::Exception { .. }
        | Command::Exceptions { .. }
        | Command::Redact { .. }
        | Command::Classify { .. }
        | Command::Link { .. }
        | Command::CheckResult { .. }
        | Command::Links { .. }
        | Command::Reviewed { .. }
        | Command::Human { .. }
        | Command::Title { .. }
        | Command::Describe { .. }
        | Command::Cards { .. }
        | Command::Card { .. }
        | Command::Beat { .. }
        | Command::Note { .. }
        | Command::Folders { .. }
        | Command::Difficulty { .. }
        | Command::Upgrade { .. }
        | Command::Audit
        | Command::Checkpoint { .. }
        | Command::Folder { .. }
        | Command::Project { .. }
        | Command::Needs { .. }
        | Command::Plan { .. }
        | Command::Ready { .. }
        | Command::Backlog { .. }
        | Command::Drop { .. }
        | Command::Reopen { .. }
        | Command::Release { .. } => tasks::execute(cli, cwd),
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
    let mut text = format!("{}\n", task_line(&changed.task));
    text.push_str(&unblocked_text(&changed.unblocked));
    let mut fields = json!({ "taskbag": root, "task": task_json(&changed.task) });
    if !changed.unblocked.is_empty() {
        fields["unblocked"] = json!(changed.unblocked.iter().map(task_json).collect::<Vec<_>>());
    }
    let mut done = Done::new(fields, text);
    done.warnings = changed.warnings;
    done.issues = changed.issues;
    Ok(done)
}

/// What a change that finished a task says about the tasks it freed (decision 0026): it was
/// their last open blocker. They are not changed; nothing when there are none.
fn unblocked_text(unblocked: &[Task]) -> String {
    let mut text = String::new();
    for t in unblocked {
        text.push_str(&format!(
            "{} ({}) is no longer blocked: all its blockers have finished. It keeps its state (a ready one is now listed by `alm next`); whoever takes it reads its blockers' AI decisions and handoffs first.\n",
            t.id,
            t.state.as_str().replace('_', " ")
        ));
    }
    text
}

/// An option's value with its spaces trimmed; `None` when nothing is left.
fn non_blank(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

/// Why this command makes its change (Task-A01-air289): `--why`, else what the command itself
/// says why with (`reopen --note`, `recheck --reason`, `wait --on`, `done
/// --without-human-check`).
fn command_reason(cli: &Cli) -> Option<String> {
    non_blank(&cli.why).or_else(|| match &cli.command {
        Command::Reopen { note, .. } => non_blank(&Some(note.clone())),
        Command::Recheck { reason, .. } => non_blank(&Some(reason.clone())),
        Command::Wait { on, .. } => non_blank(&Some(on.clone())),
        Command::Done {
            without_human_check,
            ..
        } => non_blank(without_human_check),
        _ => None,
    })
}

/// What the events this command wrote could not say about who made them (Task-A01-black004,
/// decision 0021 § 1.1): Fact lines with what to pass, and `event_missing` in `--json`. It
/// refuses nothing: `alm` tells, the agent passes it next time.
fn who_facts(done: &mut Done) -> Option<String> {
    let gaps = allomos_store::events::take_gaps();
    if gaps.is_empty() {
        return None;
    }
    let found: Vec<Value> = gaps
        .iter()
        .map(|gap| {
            let (fact, fix) = match *gap {
                "reason" => (
                    "This change's history does not say why: a state change, a label taken off or a change of plan asks for a reason.",
                    "alm --why \"<why, in one line>\" … (next time; the change itself is made)",
                ),
                "model" => (
                    "This change's history does not say which model ran it (\"unknown\"): alm cannot learn it from the environment.",
                    "alm --model <your exact model ID> … (once per session on this computer; alm remembers it)",
                ),
                "effort" => (
                    "This change's history says the effort is \"unknown\": alm cannot learn it from the environment.",
                    "alm --effort <your effort level> … (once per session on this computer; alm remembers it)",
                ),
                "permission_mode" => (
                    "This change's history says the permission mode is \"unknown\": alm cannot learn it from the environment.",
                    "alm --permission-mode <your permission mode, e.g. auto> … (once per session on this computer; alm remembers it)",
                ),
                "rules" => (
                    "This change's history says the rules are \"unknown\": which AGENTS.md commit you work under.",
                    "alm --rules <in Git: git log -1 --format=%H -- AGENTS.md> … (once per session on this computer; alm remembers it)",
                ),
                "session" => (
                    "This change's history does not say which agent session ran it: the environment gave no session ID.",
                    "run alm from inside the agent's session (Claude Code sets CLAUDE_CODE_SESSION_ID; Codex sets CODEX_THREAD_ID)",
                ),
                _ => (
                    "This change's history does not say whom the agent works for: there is no settings folder to keep this computer's pseudonym in.",
                    "alm --for <the person's pseudonym ID> …",
                ),
            };
            json!({ "code": gap, "fact": fact, "fix": fix })
        })
        .collect();
    let mut text = String::from("\n");
    for m in &found {
        text.push_str(&format!(
            "Fact: {} Fix: `{}`\n",
            m["fact"].as_str().unwrap_or(""),
            m["fix"].as_str().unwrap_or("")
        ));
    }
    done.fields["event_missing"] = json!(found);
    Some(text)
}

/// After a command that changes a task (Task-A03-ham538, approver 2026-10-27: a check inside
/// `alm`): what the task is missing that the app or the next session relies on, as Fact lines with
/// the command that fixes each, and as `missing` in `--json`. It refuses nothing.
fn missing_facts(cli: &Cli, cwd: &Path, done: &mut Done) -> Option<String> {
    let watched = matches!(
        cli.command,
        Command::Claim { .. }
            | Command::Session { .. }
            | Command::Label { .. }
            | Command::Wait { .. }
            | Command::Block { .. }
            | Command::Release { .. }
            | Command::Handoff { .. }
            | Command::Done { .. }
    );
    if !watched {
        return None;
    }
    let id = cli.command.target()?;
    let root = locate(cli, cwd).ok()?;
    let bag = load(&root).taskbag?;
    let task = bag.tasks.iter().find(|t| t.id == id)?;
    let claimed = task.holder.is_some()
        || allomos_store::events::list(&root)
            .0
            .iter()
            .any(|e| e.command == "claim" && e.id.as_deref() == Some(id));
    let branches = allomos_store::branches(&root);
    let found = allomos_store::missing::missing(task, branches.get(id), claimed);
    done.fields["missing"] = json!(found);
    if found.is_empty() {
        return None;
    }
    let mut text = String::from("\n");
    for m in &found {
        text.push_str(&format!("Fact: {} Fix: `{}`\n", m.fact, m.fix));
    }
    Some(text)
}
