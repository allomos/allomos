//! `alm monitor` (Task-A03-cow112, approver 2026-11-09): start, check now, stop and list
//! **Monitor ready tasks** without opening the app. `start` starts `alm monitor run <id>` in the
//! background, apart from the terminal (closing it does not stop the run), and returns; the run
//! ends with `stop`, or when the person logs out or the computer restarts (it is not started again
//! by itself). Each run has one small file in this computer's monitors folder
//! ([`allomos_monitor::registry`]); `status`, `list` and the app only read those files, and
//! `check` and `stop` leave a request beside it for the run to take.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use allomos_core::Problem;
use allomos_monitor::accounts;
use allomos_monitor::flow::{FIRST_FREQUENCY, FREQUENCIES, Run, Scope, TERMINAL_AGENTS};
use allomos_monitor::registry::{self, Entry, Request};
use allomos_monitor::runner;
use clap::{Args, Subcommand};
use serde_json::json;

use crate::{Cli, Done};

#[derive(Subcommand)]
pub(crate) enum MonitorWhat {
    /// Start monitoring this project's ready tasks in the background: each ready task is worked
    /// in a new terminal session of a chosen agent, which claims it first. Keeps running after
    /// this terminal is closed; ends with `alm monitor stop`. A log-out or restart ends it, and
    /// `alm monitor resume` (the app does it when the project opens) starts it again from its
    /// saved settings.
    Start(StartArgs),
    /// Run a monitor in this terminal, in the foreground, until it is stopped (what `start` runs
    /// in the background; for tests). With `--resume`, it carries on the run `id` that stopped
    /// without ending (its settings and sessions are those it saved).
    Run {
        /// The run's ID (`start` makes one). Default: a new one.
        id: Option<String>,
        #[command(flatten)]
        args: StartArgs,
        /// The language of the sessions' requests and the notifications. Default: the task
        /// language.
        #[arg(long, value_name = "CODE")]
        language: Option<String>,
        /// Carry on run `id` from its resume file, with the settings and sessions it saved.
        #[arg(long)]
        resume: bool,
    },
    /// Start again the runs of this project that stopped without ending (a log-out, a restart or
    /// a crash), from the settings they saved, with the sessions they had at work. Runs that are
    /// going on, and runs that were ended as asked, are left alone (A03-pocket537).
    Resume,
    /// Check now: let tasks whose blockers have finished go on, then give each free agent a ready
    /// task (as the app's Check now).
    Check {
        /// The run, by its ID (from `alm monitor status`); needed when the project has several.
        #[arg(long = "run", value_name = "ID")]
        run: Option<String>,
    },
    /// Stop: start no new session; the sessions at work finish their tasks undisturbed. A run
    /// whose record is missing is ended at once.
    Stop {
        /// The run, by its ID or process number; needed when the project has several.
        #[arg(long = "run", value_name = "ID|PID")]
        run: Option<String>,
    },
    /// This project's monitors: each run's ID, its tasks, agents, the tasks being worked, and the
    /// next check.
    Status,
    /// Every monitor running on this computer, whatever its project.
    List,
}

#[derive(Args, Clone)]
pub(crate) struct StartArgs {
    /// Only tasks planned in this iteration. Default: every ready task.
    #[arg(long)]
    iteration: Option<String>,
    /// An agent to give tasks to: claude-code, codex or antigravity (their terminal commands).
    /// Repeat for several; each works one task at a time. Default: the first one found.
    #[arg(long = "agent", value_name = "AGENT")]
    agents: Vec<String>,
    /// How many sessions an agent may have at work at once, as AGENT=N (for example
    /// claude-code=2). Repeat for several agents. Default: one each. No upper limit.
    #[arg(long = "sessions", value_name = "AGENT=N")]
    sessions: Vec<String>,
    /// How often to check when no task is ready, in minutes: 1, 5, 10, 30, 60, 90 or 120.
    #[arg(long, value_name = "MINUTES", default_value_t = FIRST_FREQUENCY)]
    every: u32,
    /// The run mode the sessions are given: quiet (never stop to wait for a person) or normal
    /// (ask as usual). The person chooses (`alm guide run-modes`).
    #[arg(long, value_name = "quiet|normal")]
    mode: String,
    /// The difficulties an agent takes tasks from: 1-3, or one level such as 4 (no difficulty
    /// counts as 3). Without an agent, every chosen agent; AGENT=RANGE for one, for example
    /// codex=1-2. Repeat for several. Default: every level, 1-5 (Task-A03-wave335).
    #[arg(long = "difficulty", value_name = "[AGENT=]RANGE")]
    difficulty: Vec<String>,
}

impl MonitorWhat {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            MonitorWhat::Start(_) => "monitor start",
            MonitorWhat::Run { .. } => "monitor run",
            MonitorWhat::Resume => "monitor resume",
            MonitorWhat::Check { .. } => "monitor check",
            MonitorWhat::Stop { .. } => "monitor stop",
            MonitorWhat::Status => "monitor status",
            MonitorWhat::List => "monitor list",
        }
    }
}

/// An agent as the person names it, as its ID; or why the monitor cannot start it.
fn agent_id(given: &str) -> Result<&'static str, Problem> {
    let g = given.trim().to_ascii_lowercase().replace('_', "-");
    let id = match g.as_str() {
        "claude-code" | "claude-cli" | "claude" => "claude_cli",
        "codex" | "codex-cli" => "codex_cli",
        "antigravity" | "antigravity-cli" | "agy" => "antigravity_cli",
        "claude-app" | "claude-desktop" | "codex-app" | "manual" => {
            return Err(Problem::BadUsage {
                detail: format!(
                    "--agent {given}: the monitor cannot start it. It starts only agents that run in a terminal (claude-code, codex, antigravity), with the request already sent; an app agent opens with the request typed in for a person to send, and Manual is a request a person copies"
                ),
            });
        }
        _ => {
            return Err(Problem::BadUsage {
                detail: format!(
                    "--agent {given}: not an agent the monitor knows; use claude-code, codex or antigravity"
                ),
            });
        }
    };
    Ok(id)
}

/// The agents of `run` as the command names them, with a count where it is more than one
/// (`claude-code ×2`).
fn agents_text(run: &Run) -> Vec<String> {
    run.agents
        .iter()
        .map(|a| {
            let count = match run.sessions_of(a) {
                1 => String::new(),
                n => format!(" ×{n}"),
            };
            let range = run.range_of(a);
            let levels = if range == allomos_core::difficulty::Range::all() {
                String::new()
            } else {
                format!(" ({})", range.text())
            };
            // An extra account is named as `--agent` takes it: `claude-code:<account ID>`.
            let word = match a.split_once(':') {
                Some((base, account)) => format!("{}:{account}", agent_word(base)),
                None => agent_word(a).to_string(),
            };
            format!("{word}{count}{levels}")
        })
        .collect()
}

/// An agent as the person names it, or an extra account of one (`claude-code:<account ID>`, the
/// account's ID as the app lists it in `agent-accounts.json`), as the monitor's ID. The account
/// must be listed (Task-A03-oven548).
fn agent_key(given: &str) -> Result<String, Problem> {
    let Some((name, account)) = given.split_once(':') else {
        return Ok(agent_id(given)?.to_string());
    };
    let base = agent_id(name)?;
    if accounts::load()
        .iter()
        .any(|a| a.agent == base && a.id == account)
    {
        return Ok(accounts::key(base, account));
    }
    Err(Problem::BadUsage {
        detail: format!(
            "--agent {given}: {} has no account {account}; the app's Monitor ready tasks dialog lists the accounts",
            agent_word(base)
        ),
    })
}

/// The command's name for an agent ID (an extra account's ID names its agent's command).
fn agent_word(id: &str) -> &'static str {
    match accounts::base_agent(id) {
        "claude_cli" => "claude-code",
        "codex_cli" => "codex",
        "antigravity_cli" => "antigravity",
        _ => "another agent",
    }
}

/// The run `args` describe, checked: known agents found on this computer, a frequency the app
/// offers, a run mode, an iteration that exists and a difficulty range alm reads.
fn new_run(root: &Path, args: &StartArgs, now: i64) -> Result<Run, Problem> {
    let found: Vec<String> = allomos_monitor::terminal::find_terminal_agents()
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    let mut agents: Vec<String> = Vec::new();
    for given in &args.agents {
        let id = agent_key(given)?;
        if !found.iter().any(|f| f == accounts::base_agent(&id)) {
            return Err(Problem::BadUsage {
                detail: format!(
                    "--agent {given}: {} was not found on this computer (its command answers no `--version` in PATH or the usual folders)",
                    allomos_monitor::terminal::name(&id)
                ),
            });
        }
        if !accounts::usable(&id, &accounts::load()) {
            return Err(Problem::BadUsage {
                detail: format!(
                    "--agent {given}: its account's folder is not there any more (add the account again in the app)"
                ),
            });
        }
        if !agents.iter().any(|a| *a == id) {
            agents.push(id);
        }
    }
    if agents.is_empty() {
        let first = found.first().ok_or_else(|| Problem::BadUsage {
            detail: "no agent the monitor can start was found on this computer (Claude Code, Codex or Antigravity in a terminal)".to_string(),
        })?;
        agents.push(first.clone());
    }
    if !FREQUENCIES.contains(&args.every) {
        return Err(Problem::BadUsage {
            detail: format!(
                "--every {}: use one of 1, 5, 10, 30, 60, 90 or 120 (minutes)",
                args.every
            ),
        });
    }
    let quiet = match args.mode.trim() {
        "quiet" => true,
        "normal" => false,
        other => {
            return Err(Problem::BadUsage {
                detail: format!("--mode {other}: use quiet or normal"),
            });
        }
    };
    let difficulties = difficulties_of(&args.difficulty, &agents)?;
    let sessions = sessions_of(&args.sessions, &agents)?;
    let (bag, _) = allomos_store::taskbag_of(root, allomos_store::load(root))?;
    let scope = match &args.iteration {
        Some(id) => {
            if !bag.iterations.iter().any(|i| &i.id == id) {
                return Err(Problem::UnknownIteration {
                    iteration: id.clone(),
                });
            }
            Scope::Iteration { id: id.clone() }
        }
        None => Scope::All,
    };
    // Each agent found, then its extra accounts that can be used (Task-A03-oven548).
    let listed = accounts::load();
    let offered: Vec<String> = TERMINAL_AGENTS
        .iter()
        .filter(|a| found.iter().any(|f| f == *a))
        .flat_map(|a| {
            std::iter::once(a.to_string()).chain(
                listed
                    .iter()
                    .filter(|x| x.agent == *a)
                    .map(|x| accounts::key(a, &x.id))
                    .filter(|k| accounts::usable(k, &listed)),
            )
        })
        .collect();
    let mut run = Run::start(scope, agents, offered, quiet, args.every, now);
    for (agent, count) in sessions {
        run.set_sessions(&agent, count);
    }
    // A range without an agent is every chosen agent's; one naming an agent is that agent's, so
    // it is set after them.
    let chosen = run.agents.clone();
    for (_, range) in difficulties.iter().filter(|(a, _)| a.is_none()) {
        for a in &chosen {
            run.set_difficulty(a, range);
        }
    }
    for (agent, range) in &difficulties {
        if let Some(a) = agent {
            run.set_difficulty(a, range);
        }
    }
    Ok(run)
}

/// The `--difficulty` values as (agent ID, or none for every chosen agent; range text). Each
/// range must read as `alm next --difficulty` does, and an agent must be one of `agents` (the
/// run's).
fn difficulties_of(
    given: &[String],
    agents: &[String],
) -> Result<Vec<(Option<String>, String)>, Problem> {
    let mut out = Vec::new();
    for g in given {
        let (agent, range) = match g.split_once('=') {
            Some((name, range)) => {
                let id = agent_key(name)?;
                if !agents.iter().any(|a| *a == id) {
                    return Err(Problem::BadUsage {
                        detail: format!(
                            "--difficulty {g}: {} is not one of the run's agents (give it with --agent)",
                            agent_word(&id)
                        ),
                    });
                }
                (Some(id), range)
            }
            None => (None, g.as_str()),
        };
        out.push((
            agent,
            allomos_core::difficulty::Range::parse(range.trim())?.text(),
        ));
    }
    Ok(out)
}

/// The `--sessions AGENT=N` values as (agent ID, count): each agent must be one of `agents` (the
/// run's), and the count at least one.
fn sessions_of(given: &[String], agents: &[String]) -> Result<Vec<(String, u32)>, Problem> {
    let mut out = Vec::new();
    for g in given {
        let bad = |why: &str| Problem::BadUsage {
            detail: format!(
                "--sessions {g}: {why}; write it as AGENT=N, for example claude-code=2"
            ),
        };
        let (name, count) = g.split_once('=').ok_or_else(|| bad("no count given"))?;
        let id = agent_key(name)?;
        let count: u32 = count
            .trim()
            .parse()
            .ok()
            .filter(|n| *n >= 1)
            .ok_or_else(|| bad("the count is a whole number, one or more"))?;
        if !agents.iter().any(|a| *a == id) {
            return Err(bad(&format!(
                "{} is not one of the run's agents (give it with --agent)",
                agent_word(&id)
            )));
        }
        out.push((id, count));
    }
    Ok(out)
}

/// The words that start the same run in a process of its own.
fn run_words(id: &str, root: &Path, args: &StartArgs, language: &str, run: &Run) -> Vec<String> {
    let mut w = vec![
        "monitor".to_string(),
        "run".to_string(),
        id.to_string(),
        "--taskbag".to_string(),
        root.display().to_string(),
        "--every".to_string(),
        args.every.to_string(),
        "--mode".to_string(),
        args.mode.trim().to_string(),
        "--language".to_string(),
        language.to_string(),
    ];
    for a in &run.agents {
        w.push("--agent".to_string());
        w.push(agent_word(a).to_string());
    }
    for (a, n) in &run.sessions {
        w.push("--sessions".to_string());
        w.push(format!("{}={n}", agent_word(a)));
    }
    if let Some(i) = &args.iteration {
        w.push("--iteration".to_string());
        w.push(i.clone());
    }
    for (a, range) in &run.difficulties {
        w.push("--difficulty".to_string());
        w.push(format!("{}={range}", agent_word(a)));
    }
    w
}

fn monitors_dir() -> Result<PathBuf, Problem> {
    registry::dir().ok_or_else(|| Problem::BadUsage {
        detail: "this computer has no settings folder for Allomos (HOME is not set)".to_string(),
    })
}

/// The task language for the taskbag at `root`.
fn language_of(root: &Path) -> Result<String, Problem> {
    let (bag, _) = allomos_store::taskbag_of(root, allomos_store::load(root))?;
    let system: Vec<String> = sys_locale::get_locales().collect();
    Ok(allomos_store::task_language(&bag.project, &system).language)
}

/// One run as a line of `status` and `list`.
fn entry_text(e: &Entry, with_project: bool) -> String {
    match e {
        Entry::Missing { pid, id, .. } => format!(
            "{}  running (record missing)  process {pid}: an `alm monitor run` with no run file; `alm monitor stop --run {pid}` ends it\n",
            id.as_deref().unwrap_or("(no ID)")
        ),
        Entry::Running(r) => {
            let run = &r.run;
            let scope = match &run.scope {
                Scope::All => "every ready task".to_string(),
                Scope::Iteration { id } => format!("iteration {id}"),
            };
            let agents = agents_text(run);
            let working: Vec<String> = run
                .working
                .iter()
                .map(|w| format!("{} ({})", w.id, agent_word(&w.agent)))
                .collect();
            let state = if run.stopping {
                "stopping: finishing the tasks at work".to_string()
            } else if run.free_agents().is_empty() {
                "every agent at work".to_string()
            } else {
                format!(
                    "next check {}",
                    runner::local_hhmm(run.next_check().max(r.alive))
                )
            };
            let mut text = format!(
                "{}  running  process {}{}\n  tasks: {scope}; agents: {}; every {} min; {} mode\n  working: {}; {state}\n",
                r.id,
                r.pid,
                if with_project {
                    format!("  {}", r.taskbag.display())
                } else {
                    String::new()
                },
                agents.join(", "),
                run.every,
                if run.quiet { "quiet" } else { "normal" },
                if working.is_empty() {
                    "none".to_string()
                } else {
                    working.join(", ")
                },
            );
            if !run.no_task.is_empty() {
                let words: Vec<&str> = run.no_task.iter().map(|a| agent_word(a)).collect();
                text.push_str(&format!(
                    "  no ready task in the levels of: {} (no session starts for them)\n",
                    words.join(", ")
                ));
            }
            if !run.taken.is_empty() {
                text.push_str(&format!(
                    "  sessions started for: {}\n",
                    run.taken.join(", ")
                ));
            }
            text
        }
    }
}

/// The one run `given` names among `entries` (by ID or process number); with none given, the
/// project's only run. Several and none named: refused, listing them.
fn choose<'a>(
    entries: &'a [Entry],
    given: Option<&str>,
    command: &str,
) -> Result<&'a Entry, Problem> {
    if let Some(g) = given {
        let g = g.trim();
        return entries
            .iter()
            .find(|e| e.id() == Some(g) || e.pid().to_string() == g)
            .ok_or_else(|| Problem::BadUsage {
                detail: format!("--run {g}: no monitor of this project has that ID or process number (see `alm monitor status`)"),
            });
    }
    let running: Vec<&Entry> = entries
        .iter()
        .filter(|e| matches!(e, Entry::Running(_)))
        .collect();
    match running.as_slice() {
        [] if entries.is_empty() => Err(Problem::BadUsage {
            detail: "no monitor is running for this project (`alm monitor start` starts one)"
                .to_string(),
        }),
        [] => Err(Problem::BadUsage {
            detail: format!(
                "no monitor of this project has a run file; name the process: {}",
                entries
                    .iter()
                    .map(|e| format!("alm monitor {command} --run {}", e.pid()))
                    .collect::<Vec<_>>()
                    .join(" or ")
            ),
        }),
        [one] => Ok(one),
        many => Err(Problem::BadUsage {
            detail: format!(
                "this project has {} monitors running; say which with --run: {}",
                many.len(),
                many.iter()
                    .filter_map(|e| e.id())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }),
    }
}

/// `alm monitor …`.
pub(crate) fn execute(cli: &Cli, cwd: &Path, what: &MonitorWhat) -> Result<Done, Problem> {
    let dir = monitors_dir()?;
    match what {
        MonitorWhat::Start(args) => {
            let root = crate::locate(cli, cwd)?;
            let now = runner::now_ms();
            let run = new_run(&root, args, now)?;
            let language = language_of(&root)?;
            std::fs::create_dir_all(&dir).map_err(|e| Problem::WriteFailed {
                path: dir.display().to_string(),
                detail: e.to_string(),
            })?;
            let id = registry::new_id(now);
            let log = registry::log_path(&dir, &id);
            let alm = std::env::current_exe().map_err(|e| Problem::BadUsage {
                detail: format!("cannot tell where this alm is: {e}"),
            })?;
            let words = run_words(&id, &root, args, &language, &run);
            let pid =
                runner::spawn_detached(&alm, &words, &log).map_err(|e| Problem::WriteFailed {
                    path: alm.display().to_string(),
                    detail: e.to_string(),
                })?;
            // Started once its run file is there; a process that ended first says why in its log.
            let start = Instant::now();
            let record = registry::record_path(&dir, &id);
            while !record.exists() {
                if start.elapsed() > Duration::from_secs(20) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
                if !registry::monitor_processes().iter().any(|p| p.pid == pid) {
                    break;
                }
            }
            if !record.exists() {
                let said = std::fs::read_to_string(&log).unwrap_or_default();
                return Err(Problem::BadUsage {
                    detail: format!(
                        "the monitor did not start: {}",
                        said.lines().last().unwrap_or("it said nothing")
                    ),
                });
            }
            let agents = agents_text(&run);
            let text = format!(
                "Monitor {id} started (process {pid}): {}, agents {}, every {} min, {} mode. It checks now, then at that frequency, and keeps running after this terminal is closed.\n  alm monitor status    what it is doing\n  alm monitor check --run {id}    check now\n  alm monitor stop --run {id}     start no new session; those at work finish\n  Log: {}\n",
                match &run.scope {
                    Scope::All => "every ready task".to_string(),
                    Scope::Iteration { id } => format!("iteration {id}"),
                },
                agents.join(", "),
                run.every,
                if run.quiet { "quiet" } else { "normal" },
                log.display()
            );
            Ok(Done::new(
                json!({ "taskbag": root, "run": id, "pid": pid, "log": log }),
                text,
            ))
        }
        MonitorWhat::Run {
            id,
            args,
            language,
            resume,
        } => {
            let root = crate::locate(cli, cwd)?;
            let now = runner::now_ms();
            let alm = std::env::current_exe().ok();
            if *resume {
                // The settings and sessions the run saved (A03-pocket537), not the flags given.
                let id = id.clone().ok_or_else(|| Problem::BadUsage {
                    detail: "--resume needs the run's ID".to_string(),
                })?;
                let saved = registry::read_resume(&dir, &id).ok_or_else(|| Problem::BadUsage {
                    detail: format!(
                        "{id}: no run of this computer has a resume file under that ID"
                    ),
                })?;
                runner::run(
                    &dir,
                    &id,
                    runner::Options {
                        taskbag: saved.taskbag.clone(),
                        language: saved.language.clone(),
                        run: saved.run,
                        alm,
                    },
                )?;
                return Ok(Done::new(
                    json!({ "taskbag": saved.taskbag, "run": id, "resumed": true }),
                    format!("Monitor {id} ended.\n"),
                ));
            }
            let run = new_run(&root, args, now)?;
            let language = match language {
                Some(l) => l.clone(),
                None => language_of(&root)?,
            };
            let id = id.clone().unwrap_or_else(|| registry::new_id(now));
            if !registry::plain_id(&id) {
                return Err(Problem::BadUsage {
                    detail: format!("{id}: a run ID has only letters, digits and -"),
                });
            }
            runner::run(
                &dir,
                &id,
                runner::Options {
                    taskbag: root.clone(),
                    language,
                    run,
                    alm,
                },
            )?;
            Ok(Done::new(
                json!({ "taskbag": root, "run": id }),
                format!("Monitor {id} ended.\n"),
            ))
        }
        MonitorWhat::Resume => {
            let root = crate::locate(cli, cwd)?;
            let procs = registry::monitor_processes();
            let alm = std::env::current_exe().map_err(|e| Problem::BadUsage {
                detail: format!("cannot tell where this alm is: {e}"),
            })?;
            let mut resumed: Vec<String> = Vec::new();
            for saved in registry::resumable(&dir, &root, &procs) {
                let id = saved.id.clone();
                let log = registry::log_path(&dir, &id);
                // `run` takes the flags `start` takes (its mode is required); the settings come from
                // the resume file, so these only satisfy the parser.
                let words = vec![
                    "monitor".to_string(),
                    "run".to_string(),
                    id.clone(),
                    "--resume".to_string(),
                    "--taskbag".to_string(),
                    root.display().to_string(),
                    "--mode".to_string(),
                    (if saved.run.quiet { "quiet" } else { "normal" }).to_string(),
                    "--every".to_string(),
                    saved.run.every.to_string(),
                ];
                let pid = runner::spawn_detached(&alm, &words, &log).map_err(|e| {
                    Problem::WriteFailed {
                        path: alm.display().to_string(),
                        detail: e.to_string(),
                    }
                })?;
                // Started once its run file is there (as `start` waits for it). A run that already
                // ended (its resume file is gone) counts as started too.
                let start = Instant::now();
                let record = registry::record_path(&dir, &id);
                while !record.exists() && start.elapsed() < Duration::from_secs(20) {
                    if !registry::monitor_processes().iter().any(|p| p.pid == pid) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
                if record.exists() || !registry::resume_path(&dir, &id).exists() {
                    resumed.push(id);
                }
            }
            let text = if resumed.is_empty() {
                "No monitor of this project needed starting again.\n".to_string()
            } else {
                format!(
                    "Started again {} monitor(s) of this project from their saved settings: {}.\n",
                    resumed.len(),
                    resumed.join(", ")
                )
            };
            Ok(Done::new(
                json!({ "taskbag": root, "resumed": resumed }),
                text,
            ))
        }
        MonitorWhat::Check { run } => {
            let root = crate::locate(cli, cwd)?;
            let entries = registry::of_project(&dir, &root);
            let chosen = choose(&entries, run.as_deref(), "check")?;
            let Entry::Running(r) = chosen else {
                return Err(Problem::BadUsage {
                    detail: format!(
                        "process {}: its run file is missing, so it cannot be asked to check; `alm monitor stop --run {}` ends it",
                        chosen.pid(),
                        chosen.pid()
                    ),
                });
            };
            ask(&dir, &r.id, &Request::Check)?;
            Ok(Done::new(
                json!({ "taskbag": root, "run": r.id, "asked": "check" }),
                format!(
                    "Asked monitor {} to check now: tasks whose blockers have finished go on first, then each free agent gets a ready task. `alm monitor status` shows what it did.\n",
                    r.id
                ),
            ))
        }
        MonitorWhat::Stop { run } => {
            let root = crate::locate(cli, cwd)?;
            let entries = registry::of_project(&dir, &root);
            let chosen = choose(&entries, run.as_deref(), "stop")?;
            match chosen {
                Entry::Missing { pid, id, .. } => {
                    registry::forget_process(&dir, *pid);
                    registry::end_process(*pid).map_err(|e| Problem::BadUsage {
                        detail: format!("process {pid} could not be ended: {e}"),
                    })?;
                    Ok(Done::new(
                        json!({ "taskbag": root, "run": id, "pid": pid, "ended": true }),
                        format!(
                            "Ended process {pid}, a monitor whose run file was missing. Sessions it had started go on by themselves.\n"
                        ),
                    ))
                }
                Entry::Running(r) => {
                    ask(&dir, &r.id, &Request::Stop)?;
                    let working: Vec<&str> = r.run.working.iter().map(|w| w.id.as_str()).collect();
                    let text = if working.is_empty() {
                        format!(
                            "Asked monitor {} to stop; it starts no new session and ends within a few seconds.\n",
                            r.id
                        )
                    } else {
                        format!(
                            "Asked monitor {} to stop: it starts no new session, and ends once the sessions at work have finished ({}); they are not interrupted.\n",
                            r.id,
                            working.join(", ")
                        )
                    };
                    Ok(Done::new(
                        json!({ "taskbag": root, "run": r.id, "asked": "stop", "working": working }),
                        text,
                    ))
                }
            }
        }
        MonitorWhat::Status => {
            let root = crate::locate(cli, cwd)?;
            let entries = registry::of_project(&dir, &root);
            let mut text = if entries.is_empty() {
                "No monitor is running for this project (`alm monitor start` starts one).\n"
                    .to_string()
            } else {
                format!("Monitors of {} on this computer:\n", root.display())
            };
            for e in &entries {
                text.push_str(&entry_text(e, false));
            }
            Ok(Done::new(
                json!({ "taskbag": root, "monitors": entries.iter().map(Entry::json).collect::<Vec<_>>() }),
                text,
            ))
        }
        MonitorWhat::List => {
            let procs = registry::monitor_processes();
            let entries = registry::list(&dir, Some(&procs));
            let mut text = if entries.is_empty() {
                "No monitor is running on this computer.\n".to_string()
            } else {
                "Monitors running on this computer:\n".to_string()
            };
            for e in &entries {
                text.push_str(&entry_text(e, true));
            }
            Ok(Done::new(
                json!({ "monitors": entries.iter().map(Entry::json).collect::<Vec<_>>() }),
                text,
            ))
        }
    }
}

fn ask(dir: &Path, id: &str, request: &Request) -> Result<(), Problem> {
    registry::ask(dir, id, request).map_err(|e| Problem::WriteFailed {
        path: dir.display().to_string(),
        detail: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sessions_are_counts_of_the_runs_agents() {
        let agents = vec!["claude_cli".to_string()];
        assert_eq!(
            sessions_of(&["claude-code=2".to_string()], &agents).unwrap(),
            vec![("claude_cli".to_string(), 2)]
        );
        assert!(sessions_of(&["claude-code=0".to_string()], &agents).is_err());
        assert!(sessions_of(&["claude-code".to_string()], &agents).is_err());
        assert!(sessions_of(&["codex=2".to_string()], &agents).is_err());
    }

    #[test]
    fn difficulties_are_ranges_of_the_runs_agents() {
        let agents = vec!["claude_cli".to_string(), "codex_cli".to_string()];
        assert_eq!(
            difficulties_of(&["3-5".to_string()], &agents).unwrap(),
            vec![(None, "3-5".to_string())]
        );
        assert_eq!(
            difficulties_of(&["codex=1-2".to_string(), "4".to_string()], &agents).unwrap(),
            vec![
                (Some("codex_cli".to_string()), "1-2".to_string()),
                (None, "4".to_string())
            ]
        );
        assert!(difficulties_of(&["0-3".to_string()], &agents).is_err());
        assert!(difficulties_of(&["4-2".to_string()], &agents).is_err());
        assert!(
            difficulties_of(&["claude-code=1-2".to_string()], &["codex_cli".to_string()]).is_err()
        );
    }

    #[test]
    fn app_agents_are_refused_with_the_reason() {
        assert_eq!(agent_id("claude-code").unwrap(), "claude_cli");
        assert_eq!(agent_id("codex_cli").unwrap(), "codex_cli");
        assert_eq!(agent_id("Antigravity").unwrap(), "antigravity_cli");
        let why = agent_id("claude-app").unwrap_err().english();
        assert!(why.contains("only agents that run in a terminal"), "{why}");
        assert!(
            agent_id("gemini")
                .unwrap_err()
                .english()
                .contains("not an agent")
        );
    }
}
