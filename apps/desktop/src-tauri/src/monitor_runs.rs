//! **Monitor ready tasks** from the app (Task-A03-cow112, approver 2026-11-09): the app only
//! shows and controls runs; it runs none itself. Start runs the app's own `alm monitor start`,
//! which starts the same background `alm monitor run` a terminal would, so a run started here
//! goes on after the app is closed. The status bar reads this computer's run files
//! ([`allomos_monitor::registry`]); Stop, Check now and the menus leave a request beside them.

use std::collections::BTreeMap;
use std::path::PathBuf;

use allomos_core::Problem;
use allomos_monitor::accounts::{self, Account};
use allomos_monitor::registry::{self, Entry, Request};
use tauri::AppHandle;

fn no_dir() -> Problem {
    Problem::BadUsage {
        detail: "this computer has no settings folder for Allomos".to_string(),
    }
}

/// The runs of the taskbag at `root` (several may run at once), oldest first, with the
/// record-missing `alm monitor run` processes that name it.
#[tauri::command]
pub fn monitor_runs(root: PathBuf) -> Vec<serde_json::Value> {
    let Some(dir) = registry::dir() else {
        return Vec::new();
    };
    registry::of_project(&dir, &root)
        .iter()
        .map(Entry::json)
        .collect()
}

/// Every run this computer runs, oldest first, for the Monitors list (Task-A03-motor529): all
/// projects', with the record-missing processes; `project` and `taskbag` name each one.
#[tauri::command]
pub fn monitor_all() -> Vec<serde_json::Value> {
    let Some(dir) = registry::dir() else {
        return Vec::new();
    };
    let procs = registry::monitor_processes();
    registry::list(&dir, Some(&procs))
        .iter()
        .map(Entry::json)
        .collect()
}

/// The list's Open: the project of the taskbag at `root` opens in a window (its tab comes to the
/// front when a window already shows it).
#[tauri::command]
pub fn monitor_open(app: AppHandle, root: PathBuf) -> Result<(), Problem> {
    crate::windows::open_recent_in_window(&app, &root.display().to_string()).map_err(|e| {
        Problem::BadUsage {
            detail: e.to_string(),
        }
    })
}

/// Start a run with the app's own `alm` (`monitor start`); resolves with its ID. `sessions`: how
/// many sessions each chosen agent may have at work at once (Task-A03-weasel276); one not listed
/// has one. `difficulties`: the difficulty range each chosen agent takes tasks from, as `1-3`
/// (Task-A03-wave335); one not listed takes every level.
#[tauri::command]
pub async fn monitor_start(
    root: PathBuf,
    agents: Vec<String>,
    every: u32,
    quiet: bool,
    iteration: Option<String>,
    sessions: BTreeMap<String, u32>,
    difficulties: BTreeMap<String, String>,
) -> Result<String, Problem> {
    let alm = crate::alm_command::own().ok_or_else(|| Problem::BadUsage {
        detail: "the app's own alm was not found beside it".to_string(),
    })?;
    let mut args: Vec<String> = vec![
        "--json".into(),
        "--taskbag".into(),
        root.display().to_string(),
        "monitor".into(),
        "start".into(),
        "--mode".into(),
        (if quiet { "quiet" } else { "normal" }).into(),
        "--every".into(),
        every.to_string(),
    ];
    for a in &agents {
        args.push("--agent".into());
        args.push(a.clone());
    }
    for (a, n) in sessions
        .iter()
        .filter(|(a, n)| agents.contains(a) && **n > 1)
    {
        args.push("--sessions".into());
        args.push(format!("{a}={n}"));
    }
    for (a, range) in difficulties.iter().filter(|(a, _)| agents.contains(a)) {
        args.push("--difficulty".into());
        args.push(format!("{a}={range}"));
    }
    if let Some(i) = iteration {
        args.push("--iteration".into());
        args.push(i);
    }
    let out = crate::agents::command(&alm)
        .args(&args)
        .env_remove("ALLOMOS_TASKBAG")
        .output()
        .map_err(|e| Problem::BadUsage {
            detail: format!("{}: {e}", alm.display()),
        })?;
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).map_err(|_| Problem::BadUsage {
            detail: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        })?;
    if v["ok"] == true {
        return Ok(v["run"].as_str().unwrap_or_default().to_string());
    }
    Err(Problem::BadUsage {
        detail: v["error"]["message"]
            .as_str()
            .unwrap_or("the monitor did not start")
            .to_string(),
    })
}

/// Leave `request` (check, stop, undo_stop, every, agents) for run `id`.
#[tauri::command]
pub fn monitor_ask(id: String, request: Request) -> Result<(), Problem> {
    if !registry::plain_id(&id) {
        return Err(Problem::BadUsage {
            detail: format!("{id}: not a run ID"),
        });
    }
    let dir = registry::dir().ok_or_else(no_dir)?;
    registry::ask(&dir, &id, &request).map_err(|e| Problem::WriteFailed {
        path: dir.display().to_string(),
        detail: e.to_string(),
    })
}

/// End process `pid`, an `alm monitor run` whose record is missing (it cannot see a request).
/// Refused for any other process.
#[tauri::command]
pub fn monitor_end_process(pid: u32) -> Result<(), Problem> {
    if !registry::monitor_processes().iter().any(|p| p.pid == pid) {
        return Err(Problem::BadUsage {
            detail: format!("process {pid} is not a monitor"),
        });
    }
    registry::end_process(pid).map_err(|e| Problem::BadUsage {
        detail: format!("process {pid} could not be ended: {e}"),
    })
}

/// The extra accounts of the terminal agents on this computer (Task-A03-oven548), in the order
/// they were added, and each agent's default folder (where the folder picker starts). The default
/// account of an agent is not listed: it is always there.
#[derive(serde::Serialize)]
pub struct AgentAccounts {
    accounts: Vec<Account>,
    defaults: BTreeMap<String, String>,
}

#[tauri::command]
pub fn agent_accounts() -> AgentAccounts {
    let home = allomos_monitor::terminal::home();
    let defaults = ["claude_cli", "codex_cli"]
        .into_iter()
        .filter_map(|agent| {
            accounts::default_dir(agent, &home)
                .map(|d| (agent.to_string(), d.display().to_string()))
        })
        .collect();
    AgentAccounts {
        accounts: accounts::load(),
        defaults,
    }
}

/// Add an account of an agent: the folder the person picked (its settings and login folder) and
/// the name they gave it. The folder is checked first (`accounts::check_new`); only the path and
/// the name are saved, in `agent-accounts.json`, never the login files.
#[tauri::command]
pub fn add_agent_account(agent: String, path: PathBuf, name: String) -> Result<Account, Problem> {
    let file = accounts::file_path().ok_or_else(no_dir)?;
    let mut listed = accounts::load_at(&file);
    let home = allomos_monitor::terminal::home();
    let account = accounts::check_new(
        &listed,
        &agent,
        &path,
        &name,
        &home,
        allomos_monitor::runner::now_ms(),
    )
    .map_err(|detail| Problem::BadUsage { detail })?;
    listed.push(account.clone());
    accounts::save_at(&file, &listed).map_err(|e| Problem::WriteFailed {
        path: file.display().to_string(),
        detail: e.to_string(),
    })?;
    Ok(account)
}
