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
    /// this terminal is closed; ends with `alm monitor stop`, or at log-out or restart.
    Start(StartArgs),
    /// Run a monitor in this terminal, in the foreground, until it is stopped (what `start` runs
    /// in the background; for tests).
    Run {
        /// The run's ID (`start` makes one). Default: a new one.
        id: Option<String>,
        #[command(flatten)]
        args: StartArgs,
        /// The language of the sessions' requests and the notifications. Default: the task
        /// language.
        #[arg(long, value_name = "CODE")]
        language: Option<String>,
    },
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
    /// The countdown after a done task's tab, run in its own process by a monitor (not for
    /// people): the tab closes unless a key is pressed.
    #[command(hide = true)]
    CloseTab {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        window: i64,
        #[arg(long)]
        tty: String,
        #[arg(long)]
        language: String,
    },
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
    /// How often to check when no task is ready, in minutes: 1, 5, 10, 30, 60, 90 or 120.
    #[arg(long, value_name = "MINUTES", default_value_t = FIRST_FREQUENCY)]
    every: u32,
    /// The run mode the sessions are given: quiet (never stop to wait for a person) or normal
    /// (ask as usual). The person chooses (`alm guide run-modes`).
    #[arg(long, value_name = "quiet|normal")]
    mode: String,
    /// Only tasks of these difficulties: 1-3, or one level such as 4 (no difficulty counts as 3).
    #[arg(long, value_name = "RANGE")]
    difficulty: Option<String>,
}

impl MonitorWhat {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            MonitorWhat::Start(_) => "monitor start",
            MonitorWhat::Run { .. } => "monitor run",
            MonitorWhat::Check { .. } => "monitor check",
            MonitorWhat::Stop { .. } => "monitor stop",
            MonitorWhat::Status => "monitor status",
            MonitorWhat::List => "monitor list",
            MonitorWhat::CloseTab { .. } => "monitor close-tab",
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

/// The command's name for an agent ID.
fn agent_word(id: &str) -> &'static str {
    match id {
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
        let id = agent_id(given)?;
        if !found.iter().any(|f| f == id) {
            return Err(Problem::BadUsage {
                detail: format!(
                    "--agent {given}: {} was not found on this computer (its command answers no `--version` in PATH or the usual folders)",
                    allomos_monitor::terminal::name(id)
                ),
            });
        }
        if !agents.iter().any(|a| a == id) {
            agents.push(id.to_string());
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
    if let Some(d) = &args.difficulty {
        allomos_core::difficulty::Range::parse(d)?;
    }
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
    let offered: Vec<String> = TERMINAL_AGENTS
        .iter()
        .filter(|a| found.iter().any(|f| f == *a))
        .map(|a| a.to_string())
        .collect();
    Ok(Run::start(
        scope,
        args.difficulty.clone(),
        agents,
        offered,
        quiet,
        args.every,
        now,
    ))
}

/// The words that start the same run in a process of its own.
fn run_words(
    id: &str,
    root: &Path,
    args: &StartArgs,
    language: &str,
    agents: &[String],
) -> Vec<String> {
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
    for a in agents {
        w.push("--agent".to_string());
        w.push(agent_word(a).to_string());
    }
    if let Some(i) = &args.iteration {
        w.push("--iteration".to_string());
        w.push(i.clone());
    }
    if let Some(d) = &args.difficulty {
        w.push("--difficulty".to_string());
        w.push(d.clone());
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
            let difficulty = run
                .difficulty
                .as_ref()
                .map(|d| format!(", difficulty {d}"))
                .unwrap_or_default();
            let agents: Vec<&str> = run.agents.iter().map(|a| agent_word(a)).collect();
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
                "{}  running  process {}{}\n  tasks: {scope}{difficulty}; agents: {}; every {} min; {} mode\n  working: {}; {state}\n",
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
            let words = run_words(&id, &root, args, &language, &run.agents);
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
            let agents: Vec<&str> = run.agents.iter().map(|a| agent_word(a)).collect();
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
        MonitorWhat::Run { id, args, language } => {
            let root = crate::locate(cli, cwd)?;
            let now = runner::now_ms();
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
            let alm = std::env::current_exe().ok();
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
        MonitorWhat::CloseTab {
            agent,
            window,
            tty,
            language,
        } => {
            let tab = allomos_monitor::session_tab::SessionTab {
                window: *window,
                tty: tty.clone(),
            };
            let outcome = allomos_monitor::session_tab::countdown(agent, &tab, language);
            Ok(Done::new(
                json!({ "window": window, "tty": tty, "outcome": format!("{outcome:?}") }),
                format!("Tab countdown: {outcome:?}\n"),
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
