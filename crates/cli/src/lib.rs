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
use allomos_core::{Issue, Iteration, Problem, Project, State, Task, Taskbag};
use allomos_store::handoff;
use allomos_store::{
    Actor, ConfirmExtra, Lock, MAX_PLAN_BYTES, default_root, events, find_taskbag, load,
    picked_folder, plain_path, post_confirm, quota_ask, quota_is_codex, quota_reports, read_answer,
    read_text, requests_dir, taskbag_of, tidy, versions, was_seen, with_written_files,
    withdraw_confirm,
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

/// Said after every change's Git commands (Task-A01-flour637): what is not in the list stays
/// where it is, whoever made it.
const SAVE_AS_PRINTED: &str = "Run the commands as printed, never `git add -A`, `git add .` or `git commit -a` in the taskbag: those also take changes others have not shared yet (the person's changes in the Allomos app, other sessions' changes).\n";

/// Whether the session running this command is Codex for certain (Task-A01-flame635): then the
/// Git commands alm prints are plain (`git add …`, run in the folder it names), because
/// `git -C <folder> …` matches none of Allomos's rules for Codex. The agent is the session agent
/// of the task the command acts on (unless another session holds that task), else that of a task
/// this session holds, else what the session name says for certain. Unsure is not Codex.
pub(crate) fn for_codex(cli: &Cli, cwd: &Path) -> bool {
    let me = cli.holder.as_deref();
    let tasks = locate(cli, cwd)
        .ok()
        .and_then(|root| load(&root).taskbag)
        .map(|bag| bag.tasks)
        .unwrap_or_default();
    let agent_of = |t: &Task| {
        t.session
            .as_ref()
            .and_then(|s| s.agent.clone())
            .filter(|a| !a.trim().is_empty())
    };
    let others = |t: &&Task| t.holder.is_some() && me.is_some() && t.holder.as_deref() != me;
    let target = cli
        .command
        .target()
        .and_then(|id| tasks.iter().find(|t| t.id == id))
        .filter(|t| !others(t))
        .and_then(agent_of);
    let held = || {
        let me = me?;
        tasks
            .iter()
            .filter(|t| t.state == State::InProgress && t.holder.as_deref() == Some(me))
            .find_map(agent_of)
    };
    quota_is_codex(target.or_else(held).as_deref(), me)
}

/// Said to Codex after the commands of a change (Task-A01-flame635, the unattended trial): it
/// wrote `git status --short && git add … && git commit … && git push`, and a line with one
/// command the rules do not name runs in the sandbox as a whole, where Git cannot write.
const CODEX_ON_ITS_OWN: &str = "Codex: run every Git command on a line of its own, your own Git commands too (commit, push, merge, worktree): no `&&`, `;` or pipes, and nothing else on the line. Joined with a command that Allomos's rules for Codex do not name (`git status`, a test, `chmod`), the whole line runs in the sandbox, where Git cannot write.\n";

/// The line above printed Git commands: `For Git:`, or for Codex (`plain`) the folder to run
/// them in, since they do not name it themselves.
pub(crate) fn git_heading(folder: &Path, plain: bool) -> String {
    if plain {
        format!(
            "For Git, in {} (run each command on its own, with that folder as its working folder):\n",
            quoted(&plain_path(folder).display().to_string())
        )
    } else {
        "For Git:\n".to_string()
    }
}

/// `git add`, `git commit` (with the two message lines) and, unless `push` is false, `git push`,
/// run in `taskbag` and naming exactly `files` (relative to it). `plain` (for Codex,
/// [`for_codex`]): without `-C <taskbag>`; [`git_heading`] names the folder instead.
fn git_commands(
    taskbag: &Path,
    files: &[String],
    subject: &str,
    who: &str,
    push: bool,
    plain: bool,
) -> Vec<String> {
    let at = if plain {
        "git".to_string()
    } else {
        format!(
            "git -C {}",
            quoted(&plain_path(taskbag).display().to_string())
        )
    };
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
fn share(cli: &Cli, cwd: &Path, before: &Before, written: &[PathBuf], done: &mut Done) {
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
    // Read after the change, so `alm claim --session-agent codex` already prints the plain form.
    let plain = (changes || before.app.is_some()) && for_codex(cli, cwd);
    if changes || before.app.is_some() {
        done.fields["git_folder"] = json!(plain_path(&root).display().to_string());
    }
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
                git_commands(&root, &files, &subject, &who, true, plain),
            )
        } else {
            (
                "Save this change in your version control tool and share it. Changed files:"
                    .to_string(),
                git_commands(&root, &files, &subject, &who, true, plain),
            )
        };
        text.push('\n');
        text.push_str(&instruction);
        text.push('\n');
        if !commands.is_empty() {
            for f in &files {
                text.push_str(&format!("  {f}\n"));
            }
            text.push_str(&git_heading(&root, plain));
        }
        for c in &commands {
            text.push_str(&format!("  {c}\n"));
        }
        if !commands.is_empty() {
            text.push_str("Save only these files; leave other changes as they are.\n");
            // Task-A01-flour637: a beat saved with `git commit -a` took along changes the
            // person had made in the app, without their events.
            text.push_str(SAVE_AS_PRINTED);
            if plain {
                text.push_str(CODEX_ON_ITS_OWN);
            }
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
                plain,
            );
            text.push_str(&format!(
                "\n{} change(s) made in the Allomos app by {by} are not shared yet. Save and share them in your version control tool as the person's, on their own (if someone already did, there is nothing to save; that is fine). Changed files:\n",
                app.count
            ));
            for f in &rest {
                text.push_str(&format!("  {f}\n"));
            }
            text.push_str(&git_heading(&root, plain));
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

/// The identity settings in plain words (`alm audit-settings`).
fn identity_lines(id: &allomos_core::audit::Identity) -> String {
    let level = match id.level {
        allomos_core::audit::ACCOUNT => "Allomos account",
        allomos_core::audit::SSO => "Company sign-in (SSO)",
        _ => "Pseudonym",
    };
    let mut text = format!("Identity: {level}");
    if let Some(w) = &id.written {
        text.push_str(&format!(
            " (project.json says \"{w}\", which this Allomos does not know)"
        ));
    }
    text.push_str(&format!(
        ".\nSign in again to approve or check: {}.\n",
        if id.reauthenticate { "on" } else { "off" }
    ));
    text.push_str(
        "The history only ever holds a pseudonym ID; who is behind each one is kept on this computer (local project) or in the cloud (cloud project), never in the taskbag.\n",
    );
    if !id.sign_in_available {
        text.push_str(
            "Allomos account and Company sign-in (SSO) need a cloud project; this project is local-only.\n",
        );
    }
    text
}

/// Separation of duties in plain words (`alm audit-settings`, Task-A01-grotto036).
fn separation_lines(on: bool) -> String {
    if on {
        "Separation of duties: on. The same person or agent may not both do and check (or do and approve) a task; alm refuses it and says why. A project with one person cannot finish its checks while this is on.\n".into()
    } else {
        "Separation of duties: off. Anyone may check or approve a task, including who did it.\n"
            .into()
    }
}

/// The outside witness in plain words (`alm audit-settings`, Task-A01-wood982).
fn witness_lines(w: Option<&allomos_core::witness::Witness>) -> String {
    match w {
        None => "Outside witness: off. Checkpoints are signed on this computer only; no fingerprint leaves it.\n".into(),
        Some(w) => format!(
            "Outside witness: on. Each checkpoint's fingerprint (a hash, never the content) is sent to {}; the receipt is kept in the checkpoint.\n{}",
            w.services().join(" and "),
            match w.when() {
                allomos_core::witness::When::Daily => "How often: daily. The app while it is open, and any alm command that changes the taskbag, make a checkpoint when the last one is a day old.\n",
                allomos_core::witness::When::Approvals => "How often: at each approval and human check (alm approve, alm checked and the app's same).\n",
                allomos_core::witness::When::Asked => "How often: only when asked (alm checkpoint, Make a checkpoint now, and upgrading).\n",
            }
        ),
    }
}

/// Retention and the legal hold in plain words (`alm audit-settings`, Task-A01-stem986).
fn retention_lines(
    kept: &allomos_core::retention::Retention,
    past: Option<&allomos_store::redact::PastRetention>,
) -> String {
    let mut text = match kept.years {
        None => "Keep the history: forever.\n".to_string(),
        Some(y) => format!(
            "Keep the history: {y} year{}.\n",
            if y == 1 { "" } else { "s" }
        ),
    };
    if let Some(p) = past {
        if p.events == 0 {
            text.push_str(&format!(
                "No event is older than the retention period (before {}).\n",
                p.cutoff
            ));
        } else {
            text.push_str(&format!(
                "Fact: {} event{} older than the retention period (before {}; the oldest from {}). Allomos removes nothing on its own; what to do with them is the project's decision.\n",
                p.events,
                if p.events == 1 { " is" } else { "s are" },
                p.cutoff,
                p.oldest.as_deref().unwrap_or("?")
            ));
        }
    }
    text.push_str("China's Cybersecurity Law (Art. 23) asks for network logs to be kept at least six months.\n");
    text.push_str(if kept.legal_hold {
        "Legal hold: on. Nothing in the history may be removed or redacted while it is on.\n"
    } else {
        "Legal hold: off. Personal details written by mistake can be removed with alm redact.\n"
    });
    text
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
            let mut approval = Value::Null;
            if let (Some(root), Some(id)) = (&root, task) {
                let approved = allomos_store::record_approval(
                    root,
                    Actor::alm(cli.holder.as_deref()),
                    lock_wait(),
                    id,
                    title,
                    &plan.to_string_lossy(),
                    &text,
                )?;
                out.push_str(&format!(
                    "Recorded in {}'s history as approval {}: pass `--approval {}` on the changes that rely on it.\n",
                    id, approved.approval.id, approved.approval.id
                ));
                approval = json!(approved.approval);
            }
            Ok(Done::new(
                json!({ "answer": "yes", "project": project, "fingerprint": asked.fingerprint, "task": task, "approval": approval }),
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
const CHOOSING_RULE: &str = "Take the task the person named, if any. Otherwise take one from the active iteration (the one marked active, or else the first planned one) whose next dependency is met (the list shows its tasks first); with none there, from the earliest iteration, and read its handoff first (`alm show <id>`). Get the latest changes first, then claim it with `alm claim <id> --as <your session name> --branch <the branch you are on>` and save and share the claim right away with your version control tool (it prints the Git commands as the example): until your claim is shared, other sessions do not see it. When you claim, pass what the Allomos app needs to reopen your session: `--session-agent <your agent's id>`, `--session-id <your session's ID>`, and whatever else your agent needs to reopen it. In the Claude desktop app you must also pass `--session-kind desktop --app-session-id <its local_… ID>`: without them the Allomos app resumes your session in a terminal instead of opening it in the Claude app (in a terminal, pass `--session-kind terminal`).";

fn task_json(t: &Task) -> Value {
    let mut v = serde_json::to_value(t).expect("tasks serialize");
    // The version an agent passes back with `--expected-version` (Task-A01-57).
    v["version"] = json!(ops::task_version(t));
    // The level it counts as, and whether that is the default 3 (Task-A01-actor813). The file
    // keeps no difficulty when none was given.
    let (level, default) = allomos_core::difficulty::of(t);
    v["difficulty"] = json!(level);
    v["difficulty_default"] = json!(default);
    v
}

/// A task for `--json` reading commands: whether it is blocked (worked out, decision 0026), and
/// each blocker's current state when it has blockers.
pub(crate) fn task_json_in(bag: &Taskbag, t: &Task) -> Value {
    let mut v = task_json(t);
    v["blocked"] = json!(ops::is_blocked(bag, t));
    if !t.blocked_by.is_empty() {
        v["blockers"] = json!(ops::blockers(bag, t));
    }
    v
}

/// "blocked by A01-17 (in progress), A01-20 (backlog)": the blockers that have not finished;
/// none when the task is not blocked (decision 0026).
fn blocked_text(bag: &Taskbag, t: &Task) -> Option<String> {
    let open: Vec<String> = ops::open_blockers(bag, t)
        .iter()
        .map(ops::Blocker::describe)
        .collect();
    (!open.is_empty()).then(|| format!("blocked by {}", open.join(", ")))
}

/// "blockers finished: A01-17 (done)": the task has blockers and none is open any more. None for
/// a finished task.
fn finished_blockers_text(bag: &Taskbag, t: &Task) -> Option<String> {
    if matches!(t.state, State::Done | State::Dropped) || ops::is_blocked(bag, t) {
        return None;
    }
    let done: Vec<String> = ops::finished_blockers(bag, t)
        .iter()
        .map(ops::Blocker::describe)
        .collect();
    (!done.is_empty()).then(|| format!("blockers finished: {}", done.join(", ")))
}

/// What whoever starts a task with finished blockers does first (decision 0026, detail h).
const READ_BLOCKERS_FIRST: &str = "before starting, read their AI decisions and handoffs (`alm show <id>`), and change this task's plan if they changed what it needs";

/// What holds a task back, one line each: "waiting for you: approve the card", "waiting for
/// Ann: …", "blocked by A01-17 (in progress)" (a task can be both, decision 0026); or that its
/// blockers have finished.
pub(crate) fn stop_lines(bag: &Taskbag, t: &Task) -> Vec<String> {
    let mut lines = Vec::new();
    if t.state == State::Waiting {
        lines.push(format!(
            "waiting for {}: {}",
            t.who.as_deref().unwrap_or("the approver"),
            t.waiting_on.as_deref().unwrap_or("-")
        ));
    }
    lines.extend(blocked_text(bag, t));
    lines.extend(finished_blockers_text(bag, t));
    lines
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
        "Handoff rules{whose}, while you hold a task (from `alm claim` until `alm done` or `alm release`): with {}% of the 5-hour quota left, start nothing new, write your handoff into the task (`alm handoff`), release it (`alm release <id> --note \"<what the next one needs, and the working branch to continue on>\"`) and stop; at {}% context used, finish the current step, write your handoff, commit, release the task the same way, and give the person the exact prompt for a new session. A released task is ready again, so the next session or loop run picks it up and continues on that branch (`alm claim <id> --branch <that branch>`). While you hold no task, these gates do not apply: there is nothing to hand off. Also write a handoff {}. Report your quota each time you read it: `alm quota --agent <your agent's id> --left <percent> [--resets <when>]` (the Allomos app shows it); `alm claim`, `alm label`, `alm release`, `alm done` and `alm handoff` ask for it when your last report is missing or {} minutes old or more.\n",
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
pub(crate) fn indent(text: &str) -> String {
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

/// A task's AI decisions to review, one per line: "  - <text> (<by>, <at>)" (Task-A03-19).
pub(crate) fn decision_lines(decisions: &[Decision]) -> String {
    decisions
        .iter()
        .map(|d| match &d.by {
            Some(by) => format!("  - {} ({by}, {})\n", decision_text(d), d.at),
            None => format!("  - {} ({})\n", decision_text(d), d.at),
        })
        .collect()
}

/// A decision's text, after its kind in brackets when it has one (decision 0025 § 3).
fn decision_text(d: &Decision) -> String {
    match d.kind {
        Some(kind) => format!("[{}] {}", kind.as_str(), d.text),
        None => d.text.clone(),
    }
}

/// One line about a task: ID, state, label, holder, title.
mod approve;
mod claim;
mod command;
mod command_args;
mod command_meta;
mod plan;
mod project;
mod settings;
mod tasks;
mod views;
use command_meta::id_and_name;
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
mod takeover;
mod trail;
pub(crate) use command::Command;
pub(crate) use command_args::{
    Add, CardsWhat, ChangeWhat, Delete, FolderWhat, LabelsWhat, MoveWhat, Rename,
};

/// A task's traces in one line (Task-A01-fennel617): "R77, 0021", or "none".
fn traces_text(traces: &[String]) -> String {
    if traces.is_empty() {
        "none".to_string()
    } else {
        traces.join(", ")
    }
}

/// `--difficulty` and `alm difficulty`'s level, checked by the core rule (1 to 5).
fn difficulty_level(given: &str) -> Result<u8, String> {
    allomos_core::difficulty::level(given).map_err(|p| p.english())
}

/// `--difficulty <range>` for `alm next` and `alm list`.
fn difficulty_range(given: &str) -> Result<allomos_core::difficulty::Range, String> {
    allomos_core::difficulty::Range::parse(given).map_err(|p| p.english())
}

fn state_text(state: State, language: &str) -> &'static str {
    match (state, language) {
        (State::Backlog, "zh-CN") => "储备",
        (State::Backlog, "ja") => "バックログ",
        (State::Backlog, _) => "Backlog",
        (State::Ready, "zh-CN") => "就绪",
        (State::Ready, "ja") => "準備完了",
        (State::Ready, _) => "Ready",
        (State::InProgress, "zh-CN") => "进行中",
        (State::InProgress, "ja") => "進行中",
        (State::InProgress, _) => "In progress",
        (State::Waiting, "zh-CN") => "等待中",
        (State::Waiting, "ja") => "待機中",
        (State::Waiting, _) => "Waiting",
        (State::Done, "zh-CN") => "已完成",
        (State::Done, "ja") => "完了",
        (State::Done, _) => "Done",
        (State::Dropped, "zh-CN") => "已放弃",
        (State::Dropped, "ja") => "中止",
        (State::Dropped, _) => "Dropped",
        (State::Unknown, _) => state.as_str(),
    }
}

fn label_text<'a>(label: &'a str, language: &str) -> &'a str {
    use allomos_core::labels::{
        AI_DECISIONS, DEPLOYED_BY_AGENT, DEPLOYED_BY_HUMAN, LOCALLY_BY_AGENT, LOCALLY_BY_HUMAN,
    };
    match (label, language) {
        (LOCALLY_BY_AGENT, "zh-CN") => "智能体已在本地验证",
        (LOCALLY_BY_AGENT, "ja") => "エージェントがローカルで検証済み",
        (DEPLOYED_BY_AGENT, "zh-CN") => "智能体已部署并验证",
        (DEPLOYED_BY_AGENT, "ja") => "エージェントがデプロイして検証済み",
        (LOCALLY_BY_HUMAN, "zh-CN") => "人已在本地验证",
        (LOCALLY_BY_HUMAN, "ja") => "人がローカルで検証済み",
        (DEPLOYED_BY_HUMAN, "zh-CN") => "人已在部署后验证",
        (DEPLOYED_BY_HUMAN, "ja") => "人がデプロイ後に検証済み",
        (AI_DECISIONS, "zh-CN") => "待审阅 AI 决定",
        (AI_DECISIONS, "ja") => "レビュー待ちの AI の決定",
        _ => label,
    }
}

pub(crate) fn human_task_line(t: &Task, language: &str) -> String {
    format!(
        "{}  {}  {}  {}  {}",
        t.id,
        state_text(t.state, language),
        if t.labels.is_empty() {
            "-".to_string()
        } else {
            t.labels
                .iter()
                .map(|l| label_text(l, language))
                .collect::<Vec<_>>()
                .join(", ")
        },
        t.holder.as_deref().unwrap_or("-"),
        t.title
    )
}

fn task_line(t: &Task) -> String {
    format!(
        "{}  {}  {}  {}  {}",
        t.id,
        t.state.as_str(),
        if t.labels.is_empty() {
            "-".to_string()
        } else {
            t.labels.join(", ")
        },
        t.holder.as_deref().unwrap_or("-"),
        t.title
    )
}

/// A task's line where the taskbag is at hand: the state says "· blocked" when a blocker has not
/// finished (decision 0026: the state and being blocked are shown together).
fn task_line_in(bag: &Taskbag, t: &Task) -> String {
    if !ops::is_blocked(bag, t) {
        return task_line(t);
    }
    task_line(t).replacen(
        &format!("{}  {}  ", t.id, t.state.as_str()),
        &format!("{}  {} · blocked  ", t.id, t.state.as_str()),
        1,
    )
}

fn execute(cli: &Cli, cwd: &Path) -> Result<Done, Problem> {
    match &cli.command {
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
                // A branch that exists already (a takeover, a claim after a release) is
                // continued, not started again from the base (Task-A01-sign613).
                let add = if at.continues {
                    format!("git worktree add <folder> {wq}")
                } else {
                    format!("git worktree add <folder> -b {wq} {b}")
                };
                steps.start = vec![
                    format!("git switch {b}"),
                    "git pull --ff-only".to_string(),
                    add,
                ];
                if at.continues {
                    steps.facts.push(format!(
                        "The work goes on on {w}, the task's working branch already: start your worktree from it (with what was shared on it), not as a new branch from {base}. If your version control tool says {w} is checked out in another worktree already, that worktree is not yours: leave it as it is and ask the person (in a quiet run, park the question with alm wait)."
                    ));
                }
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

/// An option's value with its spaces trimmed; `None` when nothing is left.
fn non_blank(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

/// The folder `alm claim` runs in, relative to the folder holding `.allomos/`
/// (Task-A03-hive049): never absolute, so no user name is written; `None` when there is no
/// relative way.
/// Where the former holder of a taken-over task worked, as a path: its claim's folder, read from
/// the folder holding `.allomos/` (Task-A01-sign613).
fn former_folder(root: &Path, t: &allomos_store::TookOver) -> Option<String> {
    let folder = t.folder.as_deref()?;
    let path = allomos_store::allomos_folder(root).join(folder);
    let path = std::fs::canonicalize(&path).unwrap_or(path);
    Some(path.to_string_lossy().into_owned())
}

/// What `alm claim` says after taking a task over (decision 0022 § 6, Task-A01-sign613): from
/// whom, and that the former holder's worktree may still hold uncommitted changes that are not
/// this session's to change. alm does not look inside it (`alm` tells, the agent checks).
fn took_over_text(root: &Path, t: &allomos_store::TookOver) -> String {
    let mut text = format!(
        "  taken over from: {}{}\n",
        t.from,
        if t.lost.is_some() {
            " (lost)"
        } else {
            " (with --take)"
        }
    );
    let place = match former_folder(root, t) {
        Some(f) => format!("its claim ran in {f}"),
        None => "alm does not know where".to_string(),
    };
    if let Some(words) = &t.doing {
        text.push_str(&format!(
            "  it was doing: {}\n",
            heartbeat::doing_line(words, t.doing_at.as_deref(), allomos_store::now_secs())
        ));
    }
    text.push_str(
        "Fact: only what was pushed counts: go on from the commits on the working branch and the handoff note.\n",
    );
    text.push_str(&format!(
        "Fact: {}'s worktree may still hold changes it did not save or share ({place}), but that folder is of use only on the computer it ran on; on another computer, go by the working branch and the handoff note. On the same computer, look before you redo anything, but do not change, commit or remove what is there: it is not yours.\n",
        t.from
    ));
    text
}

fn took_over_json(root: &Path, t: &allomos_store::TookOver) -> Value {
    json!({
        "from": t.from,
        "lost": t.lost,
        "folder": t.folder,
        "folder_path": former_folder(root, t),
        "working_branch": t.working,
        "doing": t.doing,
        "doing_at": t.doing_at,
    })
}

fn claim_folder(root: &Path, cwd: &Path) -> Option<String> {
    let from = std::fs::canonicalize(allomos_store::allomos_folder(root)).ok()?;
    let to = std::fs::canonicalize(cwd).ok()?;
    let path = allomos_store::relative_path(&from, &to)?;
    Some(path.to_string_lossy().replace('\\', "/"))
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
