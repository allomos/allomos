//! Allomos desktop app: the Tauri window plus the commands the interface may call.
//!
//! The interface never calls these directly; it goes through its messenger module
//! (`apps/web/src/messenger.ts`), so the same interface can later reach the desktop's
//! local server from a browser without changes.

mod agents;
mod alm_command;
mod audit;
mod codex_rules;
mod for_you;
mod former_worktree;
mod heartbeat_settings;
mod lan;
mod lan_commands;
mod menu;
mod monitor_loop;
mod monitor_menu;
mod monitor_pause;
mod monitor_runs;
mod monitor_update;
mod pairing;
mod project_settings;
mod risk_settings;
mod sessions;
mod tab_drag;
mod task_change;
mod taskbag_watch;
mod update;
mod update_commands;
mod window_list;
mod windows;

use audit::{
    audit_settings, export_public_key, make_checkpoint, open_standards_map, redact_task,
    replace_key, set_identity, set_retention, set_separation, set_witness, show_identities_file,
    this_computer, witness_tick,
};
use windows::{Windows, fit_to_screen};

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use allomos_core::{Loaded, Problem};
use allomos_store::progress::{Job, with_progress_in};
use allomos_store::{
    AgentSetup, ConfirmRequest, GitStep, Inspection, LinkTarget, SetupState,
    add_agent_instructions, add_readme_if_empty, agent_connect_request_with, agent_setup,
    agent_setup_request_in, answer_confirm, id_from_name, init, inspect, load, pending_confirms,
    picked_folder, record_project_folder, resolve_task_link, setup_state, stamp, versions,
};
use serde::Serialize;
use tauri::Manager;

#[derive(Serialize)]
struct FolderInfo {
    inspection: Inspection,
}

#[derive(Serialize)]
struct Opened {
    root: PathBuf,
    loaded: Loaded,
    /// A `README-allomos.md` was just written next to `.allomos/`.
    readme_added: bool,
    /// Each task's AI decisions not reviewed yet, by task ID (Task-A03-19), from the history.
    ai_decisions: std::collections::BTreeMap<String, Vec<allomos_core::decision::Decision>>,
    /// Each task's approvals, by task ID, with where each stands now (Task-A01-coach782), from
    /// the history.
    approvals: std::collections::BTreeMap<String, Vec<allomos_core::approval::Recorded>>,
    /// Each task's change type, risk and what it touches as an agent recorded them, by task ID
    /// (Task-A01-garden385, decision 0027), from the history; a task with none is left out.
    classifications: std::collections::BTreeMap<String, allomos_core::classify::Classification>,
    /// Each task's outside effects, by task ID (Task-A01-job650), from the history.
    effects: std::collections::BTreeMap<String, Vec<allomos_core::effect::Recorded>>,
    /// Each task's links to code and releases, by task ID (Task-A01-mint242), from the history.
    links: std::collections::BTreeMap<String, Vec<allomos_core::link::Recorded>>,
    /// Each task's base and working branch, by task ID (Task-A03-22), from the history: the New
    /// task form chooses the new task's base from the open tasks' bases.
    branches: std::collections::BTreeMap<String, allomos_core::branch::Branches>,
    /// When each task last changed, by task ID (Task-A01-52), from the history: the Iterations
    /// tab's sort menu orders by it.
    last_changed: std::collections::BTreeMap<String, String>,
    /// The card files agents reported as still in the project folder (Task-A00-30): the notice
    /// "N tasks keep their card files in the project folder" with **Update the taskbag…**.
    cards_outside: Vec<allomos_core::card::Outside>,
    /// An older taskbag's handoff files in `handoffs/` (Task-A03-polar770): the taskbag needs
    /// updating, with **Update the taskbag…**, until they are moved into their tasks.
    old_handoffs: Vec<String>,
    /// Each task's version, by task ID (Task-A01-57): the task pane sends back the version it
    /// showed when it saves a title or description, so a newer change is never overwritten.
    versions: std::collections::BTreeMap<String, String>,
}

/// The folder named on the command line: the first argument that is not an option.
/// macOS options such as `-AppleLanguages (ja)` or `-NSDocumentRevisionsDebugMode YES` take
/// the next argument as their value, so it is skipped too. `-psn_…` (older macOS, from Finder)
/// and other options stand alone. `args` includes the program name, which is skipped.
fn folder_argument(args: impl IntoIterator<Item = OsString>) -> Option<PathBuf> {
    let mut args = args.into_iter().skip(1);
    while let Some(arg) = args.next() {
        let text = arg.to_string_lossy();
        let Some(option) = text.strip_prefix('-') else {
            return Some(PathBuf::from(arg));
        };
        let takes_value =
            !option.starts_with("psn_") && option.starts_with(|c: char| c.is_ascii_uppercase());
        if takes_value {
            args.next();
        }
    }
    None
}

/// The folder to open at start-up: `ALLOMOS_OPEN` if set, else the command line's folder.
/// A folder that was given but is not one is reported, never silently replaced.
fn choose_startup_folder(
    env: Option<OsString>,
    args: impl IntoIterator<Item = OsString>,
) -> Result<Option<PathBuf>, Problem> {
    let Some(given) = env.map(PathBuf::from).or_else(|| folder_argument(args)) else {
        return Ok(None);
    };
    if given.is_dir() {
        Ok(Some(std::path::absolute(&given).unwrap_or(given)))
    } else {
        Err(Problem::StartupNotAFolder {
            path: given.display().to_string(),
        })
    }
}

/// A folder given at start-up, so `allomos ~/my-project` and
/// `open Allomos.app --args ~/my-project` open that folder directly.
#[tauri::command]
fn startup_folder() -> Result<Option<PathBuf>, Problem> {
    choose_startup_folder(std::env::var_os("ALLOMOS_OPEN"), std::env::args_os())
}

/// What is this folder? Changes nothing.
#[tauri::command(async)]
fn inspect_folder(path: PathBuf) -> FolderInfo {
    FolderInfo {
        inspection: inspect(&path),
    }
}

/// Load the taskbag at `root`.
#[tauri::command(async)]
fn open_taskbag(root: PathBuf) -> Opened {
    let loaded = load(&root);
    // Read from the tasks' Card: links and agents' reports (Task-A01-51), so the notice shows
    // as soon as the taskbag opens and on every reload.
    let cards_outside = match &loaded.taskbag {
        Some(bag) => allomos_store::cards::outside_of(&root, bag),
        None => Vec::new(),
    };
    let versions = loaded
        .taskbag
        .iter()
        .flat_map(|bag| &bag.tasks)
        .map(|t| (t.id.clone(), allomos_core::ops::task_version(t)))
        .collect();
    Opened {
        loaded,
        versions,
        ai_decisions: allomos_store::ai_decisions(&root),
        approvals: allomos_store::approvals::approvals(&root),
        classifications: allomos_store::classify::by_task(&root),
        effects: allomos_store::effects::by_task(&root),
        links: allomos_store::links::by_task(&root),
        branches: allomos_store::branches(&root),
        last_changed: allomos_store::last_changed(&root),
        cards_outside,
        old_handoffs: allomos_store::handoff::legacy(&root),
        root,
        readme_added: false,
    }
}

/// A fingerprint of the taskbag's files. The window asks for it when the system says a file
/// changed (`taskbag_watch`), and once a minute in case a notice was missed, and reloads when it
/// changed, so changes made by `alm` show without reopening the folder.
/// Sent as text: a 64-bit number does not fit a JavaScript number.
#[tauri::command(async)]
fn taskbag_stamp(root: PathBuf) -> String {
    stamp(&root).to_string()
}

/// Create an empty taskbag at `root` (normally `<folder>/.allomos/taskbag/`), then load it.
/// Empty, with no example tasks, so the open-folder flow can tell whether the project has tasks
/// yet (docs/product/OPEN-FOLDER-FLOW.md).
/// When the picked folder looks empty, also write `README-allomos.md` there, in the
/// interface's `language`; a README that cannot be written does not undo the taskbag.
/// (Connecting AI agents with the project is a separate step: `connect_agents`.)
#[tauri::command(async)]
fn create_taskbag(root: PathBuf, name: String, language: String) -> Result<Opened, Problem> {
    init(&root, &id_from_name(&name), &name).map_err(|p| match p {
        Problem::TaskbagExists { path } => Problem::FolderNotEmpty { path },
        Problem::WriteFailed { path, detail } => Problem::CreateFailed { path, detail },
        other => other,
    })?;
    let readme_added = picked_folder(&root)
        .map(|f| add_readme_if_empty(f, &language).unwrap_or(false))
        .unwrap_or(false);
    Ok(Opened {
        loaded: load(&root),
        ai_decisions: Default::default(),
        approvals: Default::default(),
        classifications: Default::default(),
        effects: Default::default(),
        links: Default::default(),
        branches: Default::default(),
        last_changed: Default::default(),
        cards_outside: Vec::new(),
        old_handoffs: Vec::new(),
        versions: Default::default(),
        root,
        readme_added,
    })
}

/// The system program that shows a folder: Finder on macOS, Explorer on Windows, the
/// desktop's file manager elsewhere. Only a taskbag folder is accepted, so the window cannot
/// be used to open any other path.
fn file_manager_command(root: &Path) -> Result<Command, Problem> {
    if inspect(root)
        != (Inspection::Found {
            root: root.to_path_buf(),
        })
    {
        return Err(Problem::NotATaskbag {
            path: root.display().to_string(),
        });
    }
    let program = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    };
    let mut command = agents::command(program);
    // Explorer understands only `\` and no `\\?\` prefix.
    command.arg(allomos_store::plain_path(root));
    Ok(command)
}

/// The system program that opens a file or web address with the user's own app for it.
fn opener() -> Command {
    agents::command(if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    })
}

/// Open a link from a task's description: a web page in the browser, or a text file inside the
/// project (such as the task's card) in the user's editor. Anything else is refused by
/// `resolve_task_link` (Task-A01-16).
#[tauri::command(async)]
fn open_task_link(root: PathBuf, task: String, link: String) -> Result<(), Problem> {
    let mut command = opener();
    match resolve_task_link(&root, &task, &link)? {
        LinkTarget::Web(url) => command.arg(url),
        LinkTarget::File(path) => command.arg(path),
    };
    command.spawn().map(drop).map_err(|e| Problem::OpenFailed {
        path: link,
        detail: e.to_string(),
    })
}

/// Reopen the session task `task` records (Task-A03-hive049): the Claude app's link, or
/// `claude --resume` in a terminal. Read from the task itself, never from the window's copy.
#[tauri::command(async)]
fn open_agent_session(root: PathBuf, task: String) -> Result<(), Problem> {
    let (bag, _) = allomos_store::taskbag_of(&root, load(&root))?;
    let found = allomos_core::ops::find_task(&bag, &task)?;
    let how = found
        .session
        .as_ref()
        .and_then(|s| sessions::reopen(s, &allomos_store::allomos_folder(&root)))
        .ok_or(Problem::NoSession {
            id: found.id.clone(),
        })?;
    sessions::open(&how)
}

/// Show the open taskbag's folder in Finder or Explorer.
#[tauri::command]
fn show_in_file_manager(root: PathBuf) -> Result<(), Problem> {
    let mut command = file_manager_command(&root)?;
    // Not waited for: Explorer reports failure even when it opened the folder.
    command.spawn().map(drop).map_err(|e| Problem::OpenFailed {
        path: root.display().to_string(),
        detail: e.to_string(),
    })
}

/// Save the interface language the window shows, so `alm task-language` can tell agents which
/// language to write tasks in (Task-A01-21). A setting that cannot be saved changes nothing else.
#[tauri::command]
fn save_ui_language(language: String) {
    let _ = allomos_store::save_ui_language(&language);
}

/// The person's name the app records its task changes under (Task-A01-18); null until asked.
#[tauri::command(async)]
fn owner_name() -> Option<String> {
    allomos_store::saved_owner_name()
}

/// Keep the person's name in the app's settings, asked once (owner, 2026-10-14).
#[tauri::command(async)]
fn save_owner_name(name: String) -> Result<(), Problem> {
    if name.trim().is_empty() {
        return Err(Problem::HolderMissing);
    }
    allomos_store::save_owner_name(&name).map_err(|e| Problem::WriteFailed {
        path: allomos_store::settings_dir()
            .map(|d| d.display().to_string())
            .unwrap_or_default(),
        detail: e.to_string(),
    })
}

/// How long a change from the app waits for another writer (such as `alm`) to finish, as `alm`
/// waits by default.
const LOCK_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

/// Change one field of a task from the task pane (Task-A01-18), under the person's saved name.
/// The change goes through the same `crates/core` rule and the same locked, recorded write as its
/// `alm` command (`allomos_store::change_task`); a refused change writes nothing and comes back
/// as the same problem `alm` reports.
fn edit_task_as(
    root: &Path,
    name: Option<&str>,
    id: &str,
    edit: &allomos_core::ops::Edit,
) -> Result<allomos_core::Task, Problem> {
    change_task_as(root, name, id, edit).map(|change| change.task)
}

/// [`edit_task_as`], with what the window needs to show the change without reading the whole
/// taskbag again (Task-A01-parade485).
fn change_task_as(
    root: &Path,
    name: Option<&str>,
    id: &str,
    edit: &allomos_core::ops::Edit,
) -> Result<task_change::TaskChange, Problem> {
    task_change::edit(root, app_actor(name)?, LOCK_WAIT, id, edit)
}

/// The person has reviewed task `id`'s AI decisions (Task-A03-19): the same change as
/// `alm reviewed`, made as the person's saved name.
#[tauri::command(async)]
fn review_ai_decisions(root: PathBuf, id: String) -> Result<(), Problem> {
    review_as(&root, allomos_store::saved_owner_name().as_deref(), &id)
}

fn review_as(root: &Path, name: Option<&str>, id: &str) -> Result<(), Problem> {
    let actor = app_actor(name)?;
    allomos_store::review_ai_decisions(root, actor, LOCK_WAIT, id).map(|_| ())
}

/// The task pane's Approve (Task-A01-baker800, decision 0021 § 1.1): the same record as
/// `alm approve <id>`, channel `app`, approver this computer's person's pseudonym, made as the
/// person's saved name. `shown` is the description the confirmation showed: if the task's plan is
/// no longer exactly that, nothing is recorded (`task_changed`). Approving changes no state;
/// `ready` (Approve and make ready, only offered on a backlog task) then makes the change
/// `alm ready` makes, as a second step the person chose. The approver chose to record, not block,
/// an approval clicked by an agent operating the app (2026-11-08): the event shows it.
#[tauri::command(async)]
fn approve_task(
    root: PathBuf,
    id: String,
    shown: String,
    expires: Option<String>,
    ready: bool,
) -> Result<(), Problem> {
    approve_as(
        &root,
        allomos_store::saved_owner_name().as_deref(),
        &id,
        &shown,
        expires.as_deref(),
        ready,
    )
}

fn approve_as(
    root: &Path,
    name: Option<&str>,
    id: &str,
    shown: &str,
    expires: Option<&str>,
    ready: bool,
) -> Result<(), Problem> {
    let actor = app_actor(name)?;
    let tasks = [id.to_string()];
    allomos_store::approvals::approve(
        root,
        actor,
        LOCK_WAIT,
        &allomos_store::approvals::Input {
            tasks: &tasks,
            approver: None,
            channel: "app",
            words: None,
            expires,
            plan: None,
            shown: Some(shown),
        },
    )?;
    audit::after_approval(root);
    if ready {
        edit_task_as(
            root,
            name,
            id,
            &allomos_core::ops::Edit::State {
                state: allomos_core::State::Ready,
                waiting_on: None,
                who: None,
            },
        )?;
    }
    Ok(())
}

/// The task pane's Withdraw (Task-A01-baker800): the same change as `alm withdraw <approval ID>
/// --why "<reason>"`, made as the person's saved name. The reason is required here
/// (`withdraw_reason_missing`). Withdrawing stops nothing (approver, 2026-11-08).
#[tauri::command(async)]
fn withdraw_approval(root: PathBuf, id: String, reason: String) -> Result<(), Problem> {
    withdraw_as(
        &root,
        allomos_store::saved_owner_name().as_deref(),
        &id,
        &reason,
    )
}

fn withdraw_as(root: &Path, name: Option<&str>, id: &str, reason: &str) -> Result<(), Problem> {
    let actor = app_actor(name)?;
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(Problem::WithdrawReasonMissing);
    }
    allomos_store::approvals::withdraw(root, actor, LOCK_WAIT, id, Some(reason)).map(|_| ())
}

#[tauri::command(async)]
fn edit_task(
    root: PathBuf,
    id: String,
    edit: allomos_core::ops::Edit,
) -> Result<task_change::TaskChange, Problem> {
    change_task_as(
        &root,
        allomos_store::saved_owner_name().as_deref(),
        &id,
        &edit,
    )
}

/// A change sent by a paired device from the companion page (Task-A04-02-04, R77): the same
/// changes the desktop window makes, through the same functions, rules, lock and recorded events,
/// under `by` (the person's name and the device's, "will (iPhone)"). Only these operations; what
/// only the computer itself can do is not here (R68). Returns the changed item as JSON.
fn companion_change(
    root: &Path,
    by: &str,
    op: &str,
    args: serde_json::Value,
) -> Result<serde_json::Value, Problem> {
    fn arg<T: serde::de::DeserializeOwned>(
        args: &serde_json::Value,
        name: &str,
    ) -> Result<T, Problem> {
        serde_json::from_value(args.get(name).cloned().unwrap_or(serde_json::Value::Null)).map_err(
            |e| Problem::NotJson {
                detail: format!("{name}: {e}"),
            },
        )
    }
    let name = Some(by);
    let json = |v: serde_json::Result<serde_json::Value>| {
        v.map_err(|e| Problem::NotJson {
            detail: e.to_string(),
        })
    };
    match op {
        "edit_task" => {
            let edit: allomos_core::ops::Edit = arg(&args, "edit")?;
            let id: String = arg(&args, "id")?;
            json(serde_json::to_value(change_task_as(
                root, name, &id, &edit,
            )?))
        }
        "review_ai_decisions" => {
            review_as(root, name, &arg::<String>(&args, "id")?)?;
            Ok(serde_json::Value::Null)
        }
        "add_area" => json(serde_json::to_value(add_area_as(
            root,
            name,
            arg(&args, "id")?,
            &arg::<String>(&args, "name")?,
        )?)),
        "add_iteration" => json(serde_json::to_value(add_iteration_as(
            root,
            name,
            arg(&args, "iteration")?,
        )?)),
        "add_task" => json(serde_json::to_value(add_task_as(
            root,
            name,
            arg(&args, "task")?,
        )?)),
        "add_iteration_and_plan" => json(serde_json::to_value(add_iteration_and_plan_as(
            root,
            name,
            &arg::<String>(&args, "task")?,
            arg(&args, "iteration")?,
        )?)),
        "rename_area" => json(serde_json::to_value(rename_area_as(
            root,
            name,
            &arg::<String>(&args, "id")?,
            &arg::<String>(&args, "name")?,
        )?)),
        "change_iteration" => json(serde_json::to_value(change_iteration_as(
            root,
            name,
            &arg::<String>(&args, "id")?,
            &arg(&args, "change")?,
        )?)),
        "delete_area" => {
            delete_area_as(root, name, &arg::<String>(&args, "id")?)?;
            Ok(serde_json::Value::Null)
        }
        "delete_iteration" => {
            delete_iteration_as(root, name, &arg::<String>(&args, "id")?)?;
            Ok(serde_json::Value::Null)
        }
        _ => Err(Problem::CompanionDesktopOnly { op: op.to_string() }),
    }
}

/// The app's actor for a change: the person's saved name, via `app` (Task-A01-18). Refused
/// (`holder_missing`) until the name has been given.
fn app_actor(name: Option<&str>) -> Result<allomos_store::Actor<'_>, Problem> {
    let name = name
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .ok_or(Problem::HolderMissing)?;
    Ok(allomos_store::Actor {
        by: Some(name),
        via: "app",
    })
}

/// The IDs the New area, New iteration and New task forms suggest (Task-A01-19): each the next
/// free one, by the same rules `alm add` uses without an ID. The person may type another.
#[derive(Debug, Serialize, PartialEq)]
struct NextIds {
    area: String,
    iteration: String,
    /// For each area, the next free task ID in it.
    tasks: std::collections::BTreeMap<String, String>,
}

#[tauri::command(async)]
fn next_ids(root: PathBuf) -> Result<NextIds, Problem> {
    let (bag, _) = allomos_store::taskbag_of(&root, load(&root))?;
    let tasks = bag
        .areas
        .iter()
        .filter_map(|a| {
            allomos_core::ops::next_task_id(&bag, &a.id)
                .ok()
                .map(|id| (a.id.clone(), id))
        })
        .collect();
    Ok(NextIds {
        area: allomos_core::ops::next_area_id(&bag),
        iteration: allomos_core::ops::next_iteration_id(&bag),
        tasks,
    })
}

/// An empty ID box means "the next free one", as leaving the ID out of `alm add` does.
fn given(id: Option<String>) -> Option<String> {
    id.map(|i| i.trim().to_string()).filter(|i| !i.is_empty())
}

/// A new area from the list (Task-A01-19), under the person's saved name: the same rule, lock and
/// recorded event as `alm add area`.
fn add_area_as(
    root: &Path,
    name: Option<&str>,
    id: Option<String>,
    area_name: &str,
) -> Result<allomos_core::Area, Problem> {
    let id = given(id);
    allomos_store::add_area(root, app_actor(name)?, LOCK_WAIT, id.as_deref(), area_name)
        .map(|a| a.item)
}

#[tauri::command(async)]
fn add_area(
    root: PathBuf,
    id: Option<String>,
    name: String,
) -> Result<allomos_core::Area, Problem> {
    add_area_as(
        &root,
        allomos_store::saved_owner_name().as_deref(),
        id,
        &name,
    )
}

/// What the New iteration form sends.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct NewIteration {
    #[serde(default)]
    id: Option<String>,
    name: String,
    #[serde(default)]
    release: Option<String>,
    #[serde(default)]
    start: Option<String>,
    #[serde(default)]
    end: Option<String>,
}

/// A new iteration from the list (Task-A01-19): the same rule, lock and recorded event as
/// `alm add iteration`.
fn add_iteration_as(
    root: &Path,
    name: Option<&str>,
    new: NewIteration,
) -> Result<allomos_core::Iteration, Problem> {
    let id = given(new.id);
    let details = allomos_core::ops::IterationDetails {
        release: new.release.as_deref(),
        start: new.start.as_deref(),
        end: new.end.as_deref(),
    };
    allomos_store::add_iteration(
        root,
        app_actor(name)?,
        LOCK_WAIT,
        id.as_deref(),
        &new.name,
        details,
    )
    .map(|i| i.item)
}

#[tauri::command(async)]
fn add_iteration(
    root: PathBuf,
    iteration: NewIteration,
) -> Result<allomos_core::Iteration, Problem> {
    add_iteration_as(
        &root,
        allomos_store::saved_owner_name().as_deref(),
        iteration,
    )
}

/// The task pane's "New…" iteration (Task-A01-39): the same rule, lock and recorded event as
/// `alm add iteration`, then task `task` planned in it as `alm plan` would, under the same lock
/// (two events). A refusal of either adds and plans nothing. Resolves to the planned task.
fn add_iteration_and_plan_as(
    root: &Path,
    name: Option<&str>,
    task: &str,
    new: NewIteration,
) -> Result<allomos_core::Task, Problem> {
    let id = given(new.id);
    let details = allomos_core::ops::IterationDetails {
        release: new.release.as_deref(),
        start: new.start.as_deref(),
        end: new.end.as_deref(),
    };
    allomos_store::add_iteration_and_plan(
        root,
        app_actor(name)?,
        LOCK_WAIT,
        task,
        id.as_deref(),
        &new.name,
        details,
    )
    .map(|(_, changed)| changed.task)
}

#[tauri::command(async)]
fn add_iteration_and_plan(
    root: PathBuf,
    task: String,
    iteration: NewIteration,
) -> Result<allomos_core::Task, Problem> {
    add_iteration_and_plan_as(
        &root,
        allomos_store::saved_owner_name().as_deref(),
        &task,
        iteration,
    )
}

/// A new task from an area or iteration group (Task-A01-19): the same rule, lock and recorded event
/// as `alm add task`; with an iteration, then planned under the same lock as `alm plan` (two
/// events).
fn add_task_as(
    root: &Path,
    name: Option<&str>,
    mut new: allomos_store::NewTask,
) -> Result<allomos_core::Task, Problem> {
    new.id = given(new.id);
    // The person is on no branch: the window sends only the base it chose (Task-A03-22).
    new.branch = None;
    allomos_store::add_task(root, app_actor(name)?, LOCK_WAIT, &new).map(|t| t.item)
}

#[tauri::command(async)]
fn add_task(root: PathBuf, task: allomos_store::NewTask) -> Result<allomos_core::Task, Problem> {
    add_task_as(&root, allomos_store::saved_owner_name().as_deref(), task)
}

/// An area renamed from its group in the list (Task-A01-29), under the person's saved name: the
/// same rule, lock and recorded event as `alm rename area`.
fn rename_area_as(
    root: &Path,
    name: Option<&str>,
    id: &str,
    new_name: &str,
) -> Result<allomos_core::Area, Problem> {
    allomos_store::rename_area(root, app_actor(name)?, LOCK_WAIT, id, new_name).map(|a| a.item)
}

#[tauri::command(async)]
fn rename_area(root: PathBuf, id: String, name: String) -> Result<allomos_core::Area, Problem> {
    rename_area_as(
        &root,
        allomos_store::saved_owner_name().as_deref(),
        &id,
        &name,
    )
}

/// An iteration changed from its group (Task-A01-29): the same rule, lock and recorded event as
/// `alm change iteration`. The form sends every field; an empty release or date clears it.
fn change_iteration_as(
    root: &Path,
    name: Option<&str>,
    id: &str,
    change: &allomos_core::ops::IterationChange,
) -> Result<allomos_core::Iteration, Problem> {
    allomos_store::change_iteration(root, app_actor(name)?, LOCK_WAIT, id, change).map(|i| i.item)
}

#[tauri::command(async)]
fn change_iteration(
    root: PathBuf,
    id: String,
    change: allomos_core::ops::IterationChange,
) -> Result<allomos_core::Iteration, Problem> {
    change_iteration_as(
        &root,
        allomos_store::saved_owner_name().as_deref(),
        &id,
        &change,
    )
}

/// An area deleted from its group (Task-A01-29): the same rule (refused while any task is in it,
/// done and dropped ones too), lock and recorded event as `alm delete area`.
fn delete_area_as(root: &Path, name: Option<&str>, id: &str) -> Result<(), Problem> {
    allomos_store::delete_area(root, app_actor(name)?, LOCK_WAIT, id).map(|_| ())
}

#[tauri::command(async)]
fn delete_area(root: PathBuf, id: String) -> Result<(), Problem> {
    delete_area_as(&root, allomos_store::saved_owner_name().as_deref(), &id)
}

/// An iteration deleted from its group (Task-A01-29): the same rule (refused while any task is
/// planned in it, done and dropped ones too), lock and recorded event as `alm delete iteration`.
fn delete_iteration_as(root: &Path, name: Option<&str>, id: &str) -> Result<(), Problem> {
    allomos_store::delete_iteration(root, app_actor(name)?, LOCK_WAIT, id).map(|_| ())
}

#[tauri::command(async)]
fn delete_iteration(root: PathBuf, id: String) -> Result<(), Problem> {
    delete_iteration_as(&root, allomos_store::saved_owner_name().as_deref(), &id)
}

/// Whether to offer **Install the alm command** (Task-A07-12, A07-14): in a built app whose own
/// `alm` is not the one Terminal finds (a Mac: through `/usr/local/bin`; Windows: first on PATH),
/// unless the person chose "Don't ask again" for this same mismatch.
#[tauri::command]
async fn alm_command_status() -> alm_command::AlmStatus {
    let dismissed = allomos_store::alm_offer_dismissed_for();
    if cfg!(windows) {
        let Some(own) = alm_command::windows_own() else {
            return alm_command::AlmStatus::default();
        };
        let (terminal, system) = alm_command::windows_terminal();
        return alm_command::decide(
            Some(&own),
            &alm_command::Place::Missing,
            &terminal,
            alm_command::Fix::UserPath { system },
            dismissed.as_deref(),
        );
    }
    let Some(bundled) = alm_command::bundled() else {
        return alm_command::AlmStatus::default();
    };
    alm_command::decide(
        Some(&bundled),
        &alm_command::place(),
        &alm_command::terminal(),
        alm_command::Fix::Link,
        dismissed.as_deref(),
    )
}

/// Make the app's own `alm` the one Terminal finds, only when the person clicks: on a Mac, link
/// `/usr/local/bin/alm` to it (asking for the Mac password); on Windows, put the app's folder
/// first in the person's own PATH. Returns the status afterwards; `cancelled` when the password
/// dialog was closed.
#[derive(Serialize)]
struct AlmInstalled {
    cancelled: bool,
    status: alm_command::AlmStatus,
}

#[tauri::command]
async fn install_alm_command() -> Result<AlmInstalled, Problem> {
    if cfg!(windows) {
        let own = alm_command::windows_own().ok_or_else(|| Problem::WriteFailed {
            path: "PATH".to_string(),
            detail: "this Allomos carries no alm".to_string(),
        })?;
        alm_command::install_windows(&own).map_err(|detail| Problem::WriteFailed {
            path: "PATH".to_string(),
            detail,
        })?;
        return Ok(AlmInstalled {
            cancelled: false,
            status: alm_command_status().await,
        });
    }
    let bundled = alm_command::bundled().ok_or_else(|| Problem::WriteFailed {
        path: alm_command::LINK.to_string(),
        detail: "this Allomos carries no alm".to_string(),
    })?;
    let done = alm_command::install(&bundled).map_err(|detail| Problem::WriteFailed {
        path: alm_command::LINK.to_string(),
        detail,
    })?;
    Ok(AlmInstalled {
        cancelled: !done,
        status: alm_command_status().await,
    })
}

/// "Don't ask again" for **Install the alm command**, for this mismatch only (the status's
/// `mismatch`): when Terminal's `alm` changes, the app asks again (Task-A07-14).
#[tauri::command]
fn dismiss_alm_offer(mismatch: String) {
    let _ = allomos_store::dismiss_alm_offer(&mismatch);
}

/// The computer's preferred languages, most preferred first (for example `["ja-JP", "en-JP"]`).
/// Read from the operating system: inside the macOS app window, `navigator.language` is
/// not a reliable guide to the system's language.
#[tauri::command]
fn system_languages() -> Vec<String> {
    sys_locale::get_locales().collect()
}

/// The AI agents on this computer that can be asked to add Allomos's instructions.
#[tauri::command]
async fn find_agents() -> Vec<agents::Agent> {
    agents::find_agents()
}

/// Which request each chosen agent gets from **Connect with the agents** (Task-A05-03), in the
/// order chosen: every agent gets the instructions request only (Task-A03-radish933). Finding the
/// project's tasks and out-of-date files are **Update the Taskbag…**'s job. The first agent's
/// request is the connect request; with several agents, each request starts with a note naming
/// the others.
fn connect_plan(
    agents: &[String],
    language: &str,
    project: &Path,
    taskbag: &Path,
) -> Vec<(String, String)> {
    connect_plan_with(agents, language, project, taskbag, true, GitStep::Ask)
}

/// [`connect_plan`]; `progress`: the first agent's request asks it to report its progress to the
/// app (Task-A05-08), unless a larger job's request does (Build a new app…); `git`: whether the
/// first agent asks before `git init` or, in a quiet goal run, runs it itself (Task-A03-35).
fn connect_plan_with(
    agents: &[String],
    language: &str,
    project: &Path,
    taskbag: &Path,
    progress: bool,
    git: GitStep,
) -> Vec<(String, String)> {
    let job = |job: Job, request: String| {
        if progress {
            with_progress_in(language, job, &request)
        } else {
            request
        }
    };
    let note = agents::shared_note(agents, language);
    agents
        .iter()
        .enumerate()
        .map(|(i, id)| {
            // The first agent does the job and reports its progress to the app (Task-A05-08).
            let request = if i == 0 {
                job(
                    Job::Connect,
                    agent_connect_request_with(language, project, taskbag, git),
                )
            } else {
                agent_setup_request_in(language, project, taskbag)
            };
            let request = match &note {
                Some(note) => format!("{note}\n\n{request}"),
                None => request,
            };
            (id.clone(), request)
        })
        .collect()
}

/// Open each chosen agent in `project` with its request typed in ([`connect_plan`]). Nothing is
/// sent: the person sends it. Returns the agents that opened.
#[tauri::command]
async fn connect_agents(
    agents: Vec<String>,
    project: PathBuf,
    taskbag: PathBuf,
    language: String,
) -> Result<Vec<String>, Problem> {
    if !project.is_dir() {
        return Err(Problem::StartupNotAFolder {
            path: project.display().to_string(),
        });
    }
    let found = agents::find_agents();
    let mut opened = Vec::new();
    for (id, request) in connect_plan(&agents, &language, &project, &taskbag) {
        let Some(agent) = found.iter().find(|a| a.id == id) else {
            continue;
        };
        agents::ask(agent, &project, &request)?;
        opened.push(id);
    }
    Ok(opened)
}

/// What the **Manual** choice of Connect with the agents shows (Task-A05-06): exactly the request
/// the first chosen agent would get ([`connect_plan`]: connect, or update when something is out
/// of date), as an agent opened through a link gets it ([`agents::link_request`]).
fn manual_plan(language: &str, project: &Path, taskbag: &Path) -> String {
    connect_plan(&[MANUAL.to_string()], language, project, taskbag)
        .remove(0)
        .1
}

/// The id of the Manual choice: no agent is opened; the person copies the request.
const MANUAL: &str = "manual";

#[tauri::command]
async fn manual_request(
    project: PathBuf,
    taskbag: PathBuf,
    language: String,
) -> Result<String, Problem> {
    if !project.is_dir() {
        return Err(Problem::StartupNotAFolder {
            path: project.display().to_string(),
        });
    }
    let request = manual_plan(&language, &project, &taskbag);
    Ok(agents::link_request(
        &request,
        alm_command::own().as_deref(),
    ))
}

/// The one request **Build a new app…** types in (Task-A03-25-02): the connect request the setup
/// wizard's first agent gets ([`connect_plan`]), then the goal request with the person's
/// description and run mode ([`allomos_store::goal_request_in`]). Manual shows the same text.
/// With `discuss` (the Start menu's **Discuss**, Task-A03-tundra474) the discuss request takes the
/// goal request's place ([`allomos_store::discuss_request_in`]): the agent writes the goal task
/// only, and the person talks it over before sending the line that starts the build.
fn build_app_plan(
    agent: &str,
    language: &str,
    project: &Path,
    taskbag: &Path,
    description: &str,
    quiet: bool,
    discuss: bool,
) -> String {
    // A quiet run creates the Git repository itself, without stopping to ask (Task-A03-35,
    // approver 2026-10-19); a normal run asks, as connecting does.
    let git = if quiet { GitStep::Create } else { GitStep::Ask };
    let connect = connect_plan_with(&[agent.to_string()], language, project, taskbag, false, git)
        .remove(0)
        .1;
    let goal = if discuss {
        allomos_store::discuss_request_in(language, description, quiet)
    } else {
        allomos_store::goal_request_in(language, description, quiet)
    };
    // The whole job is building the app: one progress request, at the end.
    with_progress_in(
        language,
        Job::BuildApp,
        &format!("{}\n\n{goal}", connect.trim_end()),
    )
}

/// Check what **Build a new app…** was given: a project folder and a description.
fn build_app_checked(project: &Path, description: &str) -> Result<(), Problem> {
    if !project.is_dir() {
        return Err(Problem::StartupNotAFolder {
            path: project.display().to_string(),
        });
    }
    if description.trim().is_empty() {
        return Err(Problem::BadUsage {
            detail: "Describe the app first.".to_string(),
        });
    }
    Ok(())
}

/// **Build a new app…** with a found agent (Task-A03-25-02): open it in `project` with
/// [`build_app_plan`] typed in. Nothing is sent: the person presses Enter.
#[tauri::command]
async fn build_app(
    agent: String,
    project: PathBuf,
    taskbag: PathBuf,
    language: String,
    description: String,
    quiet: bool,
    discuss: bool,
) -> Result<(), Problem> {
    build_app_checked(&project, &description)?;
    let found = agents::find_agents();
    let Some(chosen) = found.iter().find(|a| a.id == agent) else {
        return Err(Problem::AgentNotFound { agent });
    };
    let request = build_app_plan(
        &agent,
        &language,
        &project,
        &taskbag,
        &description,
        quiet,
        discuss,
    );
    agents::ask(chosen, &project, &request)
}

/// **Build a new app…** with Manual: the same request, as an agent opened through a link gets it
/// ([`manual_request`]), for the person to copy.
#[tauri::command]
async fn build_app_request(
    project: PathBuf,
    taskbag: PathBuf,
    language: String,
    description: String,
    quiet: bool,
    discuss: bool,
) -> Result<String, Problem> {
    build_app_checked(&project, &description)?;
    let request = build_app_plan(
        MANUAL,
        &language,
        &project,
        &taskbag,
        &description,
        quiet,
        discuss,
    );
    Ok(agents::link_request(
        &request,
        alm_command::own().as_deref(),
    ))
}

/// The explanation of quiet and normal mode (Task-A03-21's `alm guide run-modes`, the person's
/// part from its first mode on), for the **Build a new app…** dialog.
#[tauri::command]
fn run_modes_text(language: String) -> String {
    run_modes_person(&language)
}

fn run_modes_person(language: &str) -> String {
    let text = allomos_store::run_modes(language)
        .or_else(|| allomos_store::run_modes("en"))
        .unwrap_or_default();
    match text.find("\n## ") {
        Some(at) => text[at + 1..].trim_end().to_string(),
        None => text.to_string(),
    }
}

/// Put `text` on the clipboard: the Manual choice's **Copy** button (Task-A05-06).
#[tauri::command]
async fn copy_text(text: String) -> Result<(), Problem> {
    agents::copy_text(&text)
}

/// Is the project up to date with this Allomos (Task-A01-25)? `None` until an agent is connected
/// with it. The window asks when a folder is opened and every few seconds while it is open, so
/// an agent's update shows without reopening.
#[tauri::command(async)]
fn version_check(root: PathBuf) -> Option<versions::VersionCheck> {
    versions::check_connected(&root)
}

/// **Update the taskbag…** on an older Allomos's taskbag (Task-A01-45): rename it aside
/// (`taskbag-allomos-<n>`) and create a new, empty taskbag in its place, then load that; the same
/// dialog then asks the chosen agent to move the tasks across ([`move_tasks_in`]). Nothing in the
/// old folder is changed.
#[tauri::command(async)]
fn move_taskbag_aside(root: PathBuf) -> Result<Opened, Problem> {
    versions::move_aside(&root)?;
    Ok(Opened {
        loaded: load(&root),
        ai_decisions: Default::default(),
        approvals: Default::default(),
        classifications: Default::default(),
        effects: Default::default(),
        links: Default::default(),
        branches: Default::default(),
        last_changed: Default::default(),
        cards_outside: Vec::new(),
        old_handoffs: Vec::new(),
        versions: Default::default(),
        root,
        readme_added: false,
    })
}

/// One task's history, newest first, and when the taskbag started recording (Task-A01-17).
#[tauri::command(async)]
fn task_history(root: PathBuf, id: String) -> allomos_store::events::History {
    allomos_store::events::history(&root, &id)
}

/// The taskbag's format compared with this Allomos's, whether or not an agent is connected: the
/// wizard's step 2 does not go on with a newer taskbag (owner, 2026-10-12).
#[derive(Serialize)]
struct TaskbagFormat {
    format: versions::FormatState,
    /// What this Allomos writes, for the message: `allomos/1`.
    current: &'static str,
}

#[tauri::command(async)]
fn taskbag_format(root: PathBuf) -> TaskbagFormat {
    TaskbagFormat {
        format: versions::format_state(&root),
        current: allomos_core::FORMAT,
    }
}

/// Whether `project` has an `AGENTS.md` and a `CLAUDE.md`. The window notes which were missing
/// before agents are connected, and says when one appears that running agent sessions have not
/// read it (Task-A05-03).
#[derive(Serialize)]
struct InstructionFiles {
    agents_md: bool,
    claude_md: bool,
}

#[tauri::command(async)]
fn instruction_files(project: PathBuf) -> InstructionFiles {
    InstructionFiles {
        agents_md: project.join("AGENTS.md").is_file(),
        claude_md: project.join("CLAUDE.md").is_file(),
    }
}

/// The entries under `## Agents set up` in the taskbag's project notes (Task-A03-24): whether the
/// agent used last is registered decides which agents **Update the taskbag…** offers
/// (Task-A03-27). Only that section is read; nothing else is checked.
#[tauri::command(async)]
fn agents_set_up(taskbag: PathBuf) -> Vec<String> {
    allomos_store::handoff::agents_set_up(&taskbag)
}

/// Steps 2 to 4 of the open-folder flow for the taskbag at `root`: its project folder (recorded
/// here when AGENTS.md or CLAUDE.md sits beside `.allomos/`), whether that folder's agent
/// instructions mention Allomos, and whether the taskbag has tasks.
#[tauri::command(async)]
fn project_setup(root: PathBuf) -> Result<SetupState, Problem> {
    setup_state(&root)
}

/// Record `folder` as the project folder of the taskbag at `root`; then the flow's new state.
#[tauri::command(async)]
fn set_project_folder(root: PathBuf, folder: PathBuf) -> Result<SetupState, Problem> {
    record_project_folder(&root, &folder)?;
    setup_state(&root)
}

/// The job **Update the taskbag…** asks an agent to do, for its progress popup (Task-A05-08):
/// moving an older Allomos's tasks in, renewing an out-of-date section (Task-A03-27), or card
/// files kept outside the taskbag.
fn tasks_in_job(taskbag: &Path) -> Job {
    if versions::move_in(taskbag).is_some() {
        Job::MoveIn
    } else if versions::check_connected(taskbag).is_some_and(|c| !c.outdated_files().is_empty()) {
        Job::Update
    } else {
        Job::Cards
    }
}

/// **Update the taskbag…** (Task-A01-45, approver 2026-10-16: one button for an older taskbag and
/// for card files kept outside the taskbag, Task-A00-30): open the chosen agent in the project
/// folder of the taskbag at `taskbag`, with the request for whichever applies typed in
/// ([`allomos_store::agent_update_taskbag_request_in`]). Nothing is sent: the person sends it;
/// the agent shows its plan in the app (`alm confirm`) before anything changes.
#[tauri::command]
async fn move_tasks_in(
    agent: String,
    taskbag: PathBuf,
    language: String,
    translate: Option<String>,
    name: Option<String>,
) -> Result<(), Problem> {
    let project = allomos_store::versions::project_folder(&taskbag);
    if !project.is_dir() {
        return Err(Problem::StartupNotAFolder {
            path: project.display().to_string(),
        });
    }
    let found = agents::find_agents();
    let Some(chosen) = found.iter().find(|a| a.id == agent) else {
        return Err(Problem::AgentNotFound { agent });
    };
    let request = tasks_in_plan(&taskbag, &language, translate.as_deref(), name.as_deref())?;
    agents::ask(chosen, &project, &request)
}

/// **Update the taskbag…** with Manual: the same request, as an agent opened through a link gets
/// it, for the person to copy.
#[tauri::command(async)]
fn move_tasks_in_request(
    taskbag: PathBuf,
    language: String,
    translate: Option<String>,
    name: Option<String>,
) -> Result<String, Problem> {
    let request = tasks_in_plan(&taskbag, &language, translate.as_deref(), name.as_deref())?;
    Ok(agents::link_request(
        &request,
        alm_command::own().as_deref(),
    ))
}

/// Update the taskbag…'s request with its progress line; `translate`, the language code the
/// person chose to translate into (Task-A05-cape996), none for Don't translate.
fn tasks_in_plan(
    taskbag: &Path,
    language: &str,
    translate: Option<&str>,
    name: Option<&str>,
) -> Result<String, Problem> {
    let translate = translate.map(str::trim).filter(|c| !c.is_empty());
    if let Some(code) = translate.filter(|c| !allomos_core::valid_language(c)) {
        return Err(Problem::InvalidLanguage {
            value: code.to_string(),
        });
    }
    let project = allomos_store::versions::project_folder(taskbag);
    Ok(with_progress_in(
        language,
        tasks_in_job(taskbag),
        &allomos_store::agent_update_taskbag_request_with(
            language,
            &project,
            taskbag,
            translate.map(|code| allomos_store::TranslateTarget { code, name }),
        ),
    ))
}

/// **Translate the taskbag…** (Task-A05-board831): the request for translating the taskbag at
/// `taskbag` into `target` (a language code; `name` as the menu showed it), with the progress
/// line, in the interface language `language`.
fn translate_plan(taskbag: &Path, language: &str, target: &str, name: Option<&str>) -> String {
    let project = allomos_store::versions::project_folder(taskbag);
    with_progress_in(
        language,
        Job::Translate,
        &allomos_store::agent_translate_request_in(language, &project, taskbag, target, name),
    )
}

/// Open the chosen agent in the project folder with [`translate_plan`] typed in. Nothing is sent:
/// the person sends it; the agent shows its plan (`alm confirm`) before anything changes.
#[tauri::command]
async fn translate_taskbag(
    agent: String,
    taskbag: PathBuf,
    language: String,
    target: String,
    name: Option<String>,
) -> Result<(), Problem> {
    if !allomos_core::valid_language(target.trim()) {
        return Err(Problem::InvalidLanguage { value: target });
    }
    let project = allomos_store::versions::project_folder(&taskbag);
    if !project.is_dir() {
        return Err(Problem::StartupNotAFolder {
            path: project.display().to_string(),
        });
    }
    let found = agents::find_agents();
    let Some(chosen) = found.iter().find(|a| a.id == agent) else {
        return Err(Problem::AgentNotFound { agent });
    };
    let request = translate_plan(&taskbag, &language, target.trim(), name.as_deref());
    agents::ask(chosen, &project, &request)
}

/// **Translate the taskbag…** with Manual: the same request, for the person to copy.
#[tauri::command(async)]
fn translate_request(
    taskbag: PathBuf,
    language: String,
    target: String,
    name: Option<String>,
) -> Result<String, Problem> {
    if !allomos_core::valid_language(target.trim()) {
        return Err(Problem::InvalidLanguage { value: target });
    }
    let request = translate_plan(&taskbag, &language, target.trim(), name.as_deref());
    Ok(agents::link_request(
        &request,
        alm_command::own().as_deref(),
    ))
}

/// **Work on it…** (Task-A03-galley051): the taskbag at `taskbag` as an agent working in its project
/// folder passes it to `--taskbag`: relative to that folder, written with `/` (absolute when there
/// is no relative way).
fn taskbag_arg(project: &Path, taskbag: &Path) -> String {
    let (project, taskbag) = (allomos_store::tidy(project), allomos_store::tidy(taskbag));
    allomos_store::relative_path(&project, &taskbag)
        .map(|p| allomos_store::portable_path(&p))
        .unwrap_or_else(|| taskbag.display().to_string())
}

/// **Work on it…**: the taskbag's path for the request the interface writes ([`taskbag_arg`]).
#[tauri::command]
fn proceed_taskbag(taskbag: PathBuf) -> String {
    taskbag_arg(&allomos_store::versions::project_folder(&taskbag), &taskbag)
}

/// **Work on it…**: open the chosen agent in the project folder with `request` (written by the
/// interface, perhaps edited by the person) typed in. Nothing is sent: the person sends it.
#[tauri::command]
async fn proceed_with_agent(
    agent: String,
    taskbag: PathBuf,
    request: String,
) -> Result<(), Problem> {
    let project = allomos_store::versions::project_folder(&taskbag);
    if !project.is_dir() {
        return Err(Problem::StartupNotAFolder {
            path: project.display().to_string(),
        });
    }
    let found = agents::find_agents();
    let Some(chosen) = found.iter().find(|a| a.id == agent) else {
        return Err(Problem::AgentNotFound { agent });
    };
    agents::ask(chosen, &project, &request)
}

/// **Work on it…** with Manual: `request` as the person copies it (with this Allomos's own `alm`, as
/// the other Manual requests are).
#[tauri::command]
fn proceed_manual_request(request: String) -> String {
    agents::link_request(&request, alm_command::own().as_deref())
}

/// The agent `alm` last ran under for a task in the taskbag at `root`, from its events
/// (Task-A03-27); none when no event records one.
#[tauri::command(async)]
fn last_agent(root: PathBuf) -> Option<allomos_store::events::LastAgent> {
    allomos_store::events::last_agent(&root)
}

/// What agents report about the jobs they do in `project` (`alm progress`, Task-A05-08), the
/// earliest started first.
#[tauri::command(async)]
fn progress_reports(project: PathBuf) -> Vec<allomos_store::progress::Progress> {
    allomos_store::progress::reports(&project)
}

/// The person closed a finished (or silent) report: remove it.
#[tauri::command(async)]
fn dismiss_progress(project: PathBuf, id: String) {
    allomos_store::progress::dismiss(&project, &id)
}

/// Questions an agent is asking about `project` with `alm confirm`, oldest first. Listing them
/// marks them seen, so `alm` knows this window shows them.
#[tauri::command(async)]
fn confirm_requests(project: PathBuf) -> Vec<ConfirmRequest> {
    pending_confirms(&project)
}

/// The person's answer to question `id`: go ahead (`yes`) or cancel.
#[tauri::command(async)]
fn answer_confirm_request(
    project: PathBuf,
    id: String,
    yes: bool,
    fingerprint: Option<String>,
) -> Result<(), Problem> {
    answer_confirm(&project, &id, yes, fingerprint.as_deref())
}

/// Do the AI agents know about this project? The project folder and which agents' instruction
/// files tell them about Allomos, for the taskbag settings' Agents section (Task-A05-bag963). The
/// window's agent line that showed it is gone (Task-A03-root435).
#[derive(Serialize)]
struct AgentStatus {
    /// The project folder the status is about.
    project: PathBuf,
    /// Which agents' instruction files tell them about Allomos.
    setup: AgentSetup,
}

/// Reads `project.json` and the instruction files; it does not load the tasks.
#[tauri::command]
async fn agent_status(root: PathBuf) -> AgentStatus {
    // The recorded project folder; before one is recorded, the folder holding `.allomos/`.
    let project = allomos_store::recorded_project_folder(&root)
        .unwrap_or_else(|| picked_folder(&root).unwrap_or(&root).to_path_buf());
    AgentStatus {
        setup: agent_setup(&project),
        project,
    }
}

/// Which agents a folder's instruction files tell about Allomos, for the Agents section of the
/// taskbag's settings (Task-A05-bag963): asked once for each project folder.
#[tauri::command(async)]
fn agent_setup_of(project: PathBuf) -> allomos_store::AgentSetup {
    agent_setup(&project)
}

/// Write Allomos's section into `project`'s AGENTS.md and CLAUDE.md directly, for when no
/// agent is available. Returns the files changed.
#[tauri::command(async)]
fn write_agent_files(project: PathBuf) -> Result<Vec<String>, Problem> {
    add_agent_instructions(&project).map_err(|e| Problem::WriteFailed {
        path: project.join("AGENTS.md").display().to_string(),
        detail: e.to_string(),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .menu(menu::build_menu)
        .manage(Windows::default())
        .manage(tab_drag::Drag::default())
        .manage(taskbag_watch::Following::default())
        .manage(monitor_loop::MonitorLoop::default())
        .manage(lan_commands::Lan::new())
        .manage(for_you::ForYou::default())
        // The first window starts hidden (tauri.conf.json) and opens sized to its screen.
        .setup(|app| {
            // Reopen windows and tabs (Task-A01-marble320): the other windows open again.
            windows::restore(app.handle());
            // One loop for the runs: the windows' status bars and Monitors bar, the View menu's
            // Running Monitors item (Task-A03-motor529) and the monitor's notifications
            // (Task-A03-stork103), Task-A03-spider528.
            monitor_loop::start(app.handle().clone());
            // After an update, runs still on the old alm move to this one (Task-A03-cookie729).
            std::thread::spawn(monitor_update::take_over_outdated);
            if let Some(window) = app.get_webview_window("main") {
                fit_to_screen(&window);
            }
            // Remembered on (Task-A04-02-01): listen again from the start.
            if allomos_store::lan_viewing() {
                let handle = app.handle().clone();
                let _ = lan_commands::start_lan(&handle, &handle.state::<lan_commands::Lan>());
            }
            Ok(())
        })
        .on_menu_event(menu::chosen)
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::Destroyed => {
                window.state::<Windows>().forget(window.label());
                window
                    .state::<taskbag_watch::Following>()
                    .forget(window.label());
                window
                    .state::<monitor_loop::MonitorLoop>()
                    .forget(window.label());
                window_list::refresh(window.app_handle(), Some(window.label()));
            }
            tauri::WindowEvent::Focused(focused) => {
                window
                    .state::<monitor_loop::MonitorLoop>()
                    .focus(window.label(), *focused);
                window_list::refresh(window.app_handle(), None)
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            startup_folder,
            menu::about_info,
            menu::open_website,
            menu::close_window,
            windows::window_start,
            windows::take_settings_request,
            windows::set_window_tabs,
            windows::show_project_elsewhere,
            tab_drag::drag_tear,
            tab_drag::drag_end,
            tab_drag::drag_cancel,
            windows::app_settings,
            windows::set_start_with,
            for_you::open_for_you,
            for_you::for_you_project,
            for_you::set_for_you_title,
            for_you::human_queue,
            update_commands::set_auto_check,
            update_commands::set_auto_download,
            inspect_folder,
            open_taskbag,
            taskbag_stamp,
            taskbag_watch::follow_taskbag,
            taskbag_watch::unfollow_taskbag,
            version_check,
            move_taskbag_aside,
            taskbag_format,
            update_commands::open_download_page,
            update_commands::update_status,
            update_commands::update_progress,
            update_commands::cancel_update_download,
            update_commands::skip_update,
            update_commands::restart_to_update_now,
            menu::set_update_item,
            project_settings::handoff_settings,
            project_settings::set_handoff_rules,
            heartbeat_settings::heartbeat_settings,
            heartbeat_settings::set_heartbeat_settings,
            risk_settings::risk_settings,
            risk_settings::set_risk_settings,
            audit_settings,
            set_separation,
            set_witness,
            set_retention,
            set_identity,
            show_identities_file,
            open_standards_map,
            make_checkpoint,
            witness_tick,
            redact_task,
            this_computer,
            export_public_key,
            replace_key,
            lan_commands::lan_status,
            lan_commands::set_lan_viewing,
            project_settings::rename_project,
            project_settings::project_folders,
            project_settings::bind_folder,
            project_settings::rename_folder,
            project_settings::locate_folder,
            project_settings::unbind_folder,
            project_settings::show_folder_in_file_manager,
            project_settings::set_task_language,
            project_settings::change_ladder,
            lan_commands::lan_pairing,
            lan_commands::answer_pairing,
            lan_commands::remove_paired_device,
            create_taskbag,
            task_history,
            show_in_file_manager,
            open_task_link,
            open_agent_session,
            system_languages,
            save_ui_language,
            owner_name,
            save_owner_name,
            edit_task,
            review_ai_decisions,
            approve_task,
            withdraw_approval,
            next_ids,
            add_area,
            add_iteration,
            add_iteration_and_plan,
            add_task,
            rename_area,
            change_iteration,
            delete_area,
            delete_iteration,
            menu::set_menu_texts,
            menu::set_recent_folders,
            find_agents,
            connect_agents,
            manual_request,
            build_app,
            build_app_request,
            move_tasks_in,
            move_tasks_in_request,
            translate_taskbag,
            translate_request,
            proceed_taskbag,
            proceed_with_agent,
            proceed_manual_request,
            former_worktree::former_worktree,
            monitor_runs::monitor_runs,
            monitor_runs::monitor_start,
            monitor_runs::monitor_resume,
            monitor_runs::monitor_last,
            monitor_runs::agent_accounts,
            monitor_runs::add_agent_account,
            monitor_runs::monitor_ask,
            monitor_runs::monitor_end_process,
            monitor_runs::monitor_all,
            monitor_runs::monitor_open,
            run_modes_text,
            codex_rules::codex_rules_status,
            codex_rules::codex_rules_apply,
            copy_text,
            instruction_files,
            agents_set_up,
            last_agent,
            write_agent_files,
            agent_status,
            agent_setup_of,
            confirm_requests,
            answer_confirm_request,
            progress_reports,
            dismiss_progress,
            project_setup,
            set_project_folder,
            menu::set_connect_enabled,
            alm_command_status,
            install_alm_command,
            dismiss_alm_offer
        ])
        .build(tauri::generate_context!())
        .expect("error while running Allomos")
        .run(|app, event| {
            // Task-A03-bowl641: quitting by the person's own request, while monitoring runs, first
            // says that it keeps running. The app quits only when the person confirms (which quits again,
            // programmatically, so this reminder does not come back).
            if let tauri::RunEvent::ExitRequested {
                code: None, api, ..
            } = &event
            {
                if let Some(sessions) = monitor_pause::running_sessions() {
                    api.prevent_exit();
                    let handle = app.clone();
                    monitor_pause::confirm_then(
                        app,
                        monitor_pause::Action::Quit,
                        sessions,
                        move || handle.exit(0),
                    );
                    return;
                }
            }
            // Quitting keeps every window and tab for the next start (Task-A01-marble320).
            if matches!(
                event,
                tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
            ) {
                app.state::<Windows>().quitting();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Task-A03-galley051: Work on it…'s request names the taskbag as the agent in the project folder
    /// passes it, beside the project or inside it.
    #[test]
    fn proceed_names_the_taskbag_from_the_project_folder() {
        assert_eq!(
            taskbag_arg(
                Path::new("/work/app"),
                Path::new("/work/app-taskbag/.allomos/taskbag")
            ),
            "../app-taskbag/.allomos/taskbag"
        );
        assert_eq!(
            taskbag_arg(
                Path::new("/work/app"),
                Path::new("/work/app/.allomos/taskbag")
            ),
            ".allomos/taskbag"
        );
    }

    /// Task-A04-02-04: a companion page's change goes through the window's own functions, given
    /// "name (device)" for the holder rule; what only the computer can do is refused.
    #[test]
    fn a_companion_change_is_made_and_recorded_like_the_windows() {
        let dir =
            std::env::temp_dir().join(format!("allomos-desktop-companion-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join(allomos_store::DEFAULT_DIR);
        allomos_store::init(&root, "p", "P").unwrap();
        let by = "Will (Phone)";
        let area = companion_change(
            &root,
            by,
            "add_area",
            serde_json::json!({ "id": null, "name": "Core" }),
        )
        .unwrap();
        let task = companion_change(
            &root,
            by,
            "add_task",
            serde_json::json!({ "task": { "area": area["id"], "title": "From the phone" } }),
        )
        .unwrap();
        let id = task["id"].as_str().unwrap().to_string();
        companion_change(
            &root,
            by,
            "edit_task",
            serde_json::json!({ "id": id, "edit": { "field": "state", "state": "ready" } }),
        )
        .unwrap();
        let bag = allomos_store::load(&root).taskbag.unwrap();
        assert_eq!(bag.tasks[0].state, allomos_core::State::Ready);
        let history = allomos_store::events::history(&root, &id);
        // Recorded through the app, under the person's pseudonym and never their name
        // (decision 0021 § 2, Task-A01-frolic587).
        assert_eq!(history.events.len(), 2);
        assert!(history.events.iter().all(|e| e.via == "app"));
        assert!(!serde_json::to_string(&history).unwrap().contains("Will"));
        let err =
            companion_change(&root, by, "show_in_file_manager", serde_json::json!({})).unwrap_err();
        assert_eq!(err.code(), "companion_desktop_only");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Task-A01-nacho373: the commands that read or write the taskbag run off the main thread, so
    /// two can run at the same moment. Changes sent together each take the lock in turn: every
    /// one lands, with one event each, while another thread keeps reading the taskbag.
    #[test]
    fn changes_sent_at_the_same_moment_all_land() {
        let dir =
            std::env::temp_dir().join(format!("allomos-desktop-together-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join(allomos_store::DEFAULT_DIR);
        allomos_store::init(&root, "p", "P").unwrap();
        let by = "Will (Phone)";
        let area = companion_change(
            &root,
            by,
            "add_area",
            serde_json::json!({ "id": null, "name": "Core" }),
        )
        .unwrap();
        let ids: Vec<String> = (0..8)
            .map(|i| {
                let task = companion_change(
                    &root,
                    by,
                    "add_task",
                    serde_json::json!({ "task": { "area": area["id"], "title": format!("Task {i}") } }),
                )
                .unwrap();
                task["id"].as_str().unwrap().to_string()
            })
            .collect();
        std::thread::scope(|scope| {
            for id in &ids {
                let root = &root;
                scope.spawn(move || {
                    companion_change(
                        root,
                        by,
                        "edit_task",
                        serde_json::json!({ "id": id, "edit": { "field": "state", "state": "ready" } }),
                    )
                    .unwrap();
                });
            }
            // A reader at the same time, as the window's reload is.
            let root = &root;
            scope.spawn(move || {
                for _ in 0..8 {
                    assert!(open_taskbag(root.clone()).loaded.taskbag.is_some());
                }
            });
        });
        let bag = allomos_store::load(&root).taskbag.unwrap();
        assert_eq!(bag.tasks.len(), 8);
        assert!(
            bag.tasks
                .iter()
                .all(|t| t.state == allomos_core::State::Ready)
        );
        for id in &ids {
            assert_eq!(allomos_store::events::history(&root, id).events.len(), 2);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Task-A01-nacho373: how long the window's commands take on a real taskbag, which is how
    /// long the window stood still while they ran on the main thread. Not run with the others:
    /// `ALLOMOS_TIMING_TASKBAG=<a copy of a taskbag> ALLOMOS_TIMING_TASK=<a backlog task's id>
    /// cargo test -p allomos-desktop timing -- --ignored --nocapture`. It changes that task, so
    /// give it a copy.
    #[test]
    #[ignore]
    fn timing_of_the_windows_commands_on_a_taskbag() {
        let root = PathBuf::from(std::env::var_os("ALLOMOS_TIMING_TASKBAG").unwrap());
        let id = std::env::var("ALLOMOS_TIMING_TASK").unwrap();
        let timed = |what: &str, run: &mut dyn FnMut()| {
            let start = std::time::Instant::now();
            run();
            println!("{what}: {} ms", start.elapsed().as_millis());
        };
        timed("open_taskbag", &mut || {
            open_taskbag(root.clone());
        });
        timed("taskbag_stamp", &mut || {
            taskbag_stamp(root.clone());
        });
        timed("task_history", &mut || {
            task_history(root.clone(), id.clone());
        });
        let state = |state: &str| -> allomos_core::ops::Edit {
            serde_json::from_value(serde_json::json!({ "field": "state", "state": state })).unwrap()
        };
        // Since Task-A01-parade485: the change and its answer are all the window waits for.
        timed(
            "edit_task with its answer (state: ready; all it takes now)",
            &mut || {
                change_task_as(&root, Some("Will"), &id, &state("ready")).unwrap();
            },
        );
        timed("edit_task, the change alone (state: backlog)", &mut || {
            edit_task_as(&root, Some("Will"), &id, &state("backlog")).unwrap();
        });
        // Until Task-A01-parade485 the window then asked for the fingerprint and read the whole
        // taskbag again; since then the change's answer is all it needs.
        timed(
            "taskbag_stamp (before A01-parade485: after the change)",
            &mut || {
                taskbag_stamp(root.clone());
            },
        );
        timed(
            "open_taskbag again (before A01-parade485: the reload after the change)",
            &mut || {
                open_taskbag(root.clone());
            },
        );
    }

    use allomos_store::agent_connect_request_in;

    fn args(list: &[&str]) -> Vec<OsString> {
        std::iter::once("allomos")
            .chain(list.iter().copied())
            .map(OsString::from)
            .collect()
    }

    #[test]
    fn startup_folder_skips_options() {
        let dir = std::env::temp_dir().join(format!("allomos-start-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let d = dir.to_str().unwrap();
        let missing = dir.join("no-such-folder");
        let m = missing.to_str().unwrap();
        let found = |a: &[&str]| choose_startup_folder(None, args(a));

        // The folder alone.
        assert_eq!(found(&[d]), Ok(Some(dir.clone())));
        // An option with a value, a lone option and an old Finder argument before the folder.
        assert_eq!(
            found(&["-AppleLanguages", "(ja)", d]),
            Ok(Some(dir.clone()))
        );
        assert_eq!(
            found(&[
                "-psn_0_12345",
                "-NSDocumentRevisionsDebugMode",
                "YES",
                "-v",
                d
            ]),
            Ok(Some(dir.clone()))
        );
        // An option before a folder that does not exist: reported, not replaced.
        assert_eq!(
            found(&["-AppleLanguages", "(ja)", m]),
            Err(Problem::StartupNotAFolder { path: m.into() })
        );
        // No arguments, or only options: nothing given.
        assert_eq!(found(&[]), Ok(None));
        assert_eq!(found(&["-AppleLanguages", "(ja)"]), Ok(None));
        // ALLOMOS_OPEN wins over arguments, and is checked the same way.
        assert_eq!(
            choose_startup_folder(Some(dir.clone().into()), args(&[m])),
            Ok(Some(dir.clone()))
        );
        assert_eq!(
            choose_startup_folder(Some(missing.clone().into()), args(&[d])),
            Err(Problem::StartupNotAFolder { path: m.into() })
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn file_manager_opens_only_a_taskbag() {
        let dir = std::env::temp_dir().join(format!("allomos-reveal-{}", std::process::id()));
        let root = allomos_store::default_root(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // The project folder itself, a missing path and a plain file are refused.
        for path in [dir.clone(), dir.join("missing"), root.clone()] {
            assert!(matches!(
                file_manager_command(&path),
                Err(Problem::NotATaskbag { .. })
            ));
        }
        allomos_store::init(&root, "p", "P").unwrap();
        let command = file_manager_command(&root).unwrap();
        // The path as Explorer wants it (on macOS and Linux, unchanged).
        let shown = allomos_store::plain_path(&root);
        assert_eq!(command.get_args().collect::<Vec<_>>(), [shown.as_os_str()]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_out_of_date_project_still_gets_the_connect_request() {
        let dir =
            std::env::temp_dir().join(format!("allomos-desktop-update-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let taskbag = dir.join(allomos_store::DEFAULT_DIR);
        allomos_store::init(&taskbag, "p", "P").unwrap();
        // A section from before versions: connected, but out of date.
        std::fs::write(dir.join("AGENTS.md"), "## Tasks: Allomos\n\nalm next\n").unwrap();
        // Connect only tells the agent about Allomos (Task-A03-radish933); the out-of-date
        // notice is Update the Taskbag…'s, so the connect request is the same.
        let plan = connect_plan(&["claude_app".into()], "en", &dir, &taskbag);
        assert_eq!(
            plan[0].1,
            with_progress_in(
                "en",
                Job::Connect,
                &agent_connect_request_in("en", &dir, &taskbag)
            )
        );
        assert!(!plan[0].1.contains("older Allomos"));
        assert!(version_check(taskbag.clone()).unwrap().needs_update());

        // Renewed: still the usual connect request.
        std::fs::write(dir.join("AGENTS.md"), allomos_store::agent_section()).unwrap();
        let plan = connect_plan(&["claude_app".into()], "en", &dir, &taskbag);
        assert_eq!(
            plan[0].1,
            with_progress_in(
                "en",
                Job::Connect,
                &agent_connect_request_in("en", &dir, &taskbag)
            )
        );
        assert!(!version_check(taskbag).unwrap().needs_update());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Task-A01-45: an older Allomos's taskbag gets the move-in request, before and after the app
    /// renames it aside; once the first task is moved in, the usual requests again.
    #[test]
    fn an_older_taskbag_gets_the_move_in_request() {
        let dir =
            std::env::temp_dir().join(format!("allomos-desktop-move-in-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let taskbag = dir.join(allomos_store::DEFAULT_DIR);
        allomos_store::init(&taskbag, "p", "P").unwrap();
        std::fs::write(dir.join("AGENTS.md"), allomos_store::agent_section()).unwrap();
        let project = taskbag.join("project.json");
        let text = std::fs::read_to_string(&project).unwrap();
        std::fs::write(&project, text.replace(allomos_core::FORMAT, "allomos/2")).unwrap();

        // Not renamed yet. Connect asks for the instructions only, not for the move-in.
        let check = version_check(taskbag.clone()).unwrap();
        assert!(!check.move_in.as_ref().unwrap().prepared);
        let plan = connect_plan(&["claude_app".into()], "en", &dir, &taskbag);
        assert!(!plan[0].1.starts_with("Please move the tasks of a taskbag"));
        assert!(!plan[0].1.contains(" init --id "));
        assert_eq!(manual_plan("en", &dir, &taskbag), plan[0].1);
        // The wizard can reach the Connect step: the project folder is known without reading.
        let setup = project_setup(taskbag.clone()).unwrap();
        assert_eq!(setup.project_folder.as_deref(), Some(dir.as_path()));

        // Move the tasks in…: the old folder aside, a new empty taskbag in its place.
        let opened = move_taskbag_aside(taskbag.clone()).unwrap();
        let bag = opened.loaded.taskbag.unwrap();
        assert!(bag.tasks.is_empty());
        let aside = dir.join(".allomos").join("taskbag-allomos-2");
        assert_eq!(
            std::fs::read_to_string(aside.join("project.json")).unwrap(),
            text.replace(allomos_core::FORMAT, "allomos/2")
        );
        let plan = connect_plan(&["claude_app".into()], "en", &dir, &taskbag);
        assert!(!plan[0].1.contains("Allomos renamed the old taskbag"));
        assert!(!plan[0].1.contains(" init --id "));
        assert!(version_check(taskbag.clone()).unwrap().needs_update());
        // Update the taskbag… asks for the move-in, reported as that job (Task-A01-45); once the
        // tasks are in, it is about card files (below).
        assert_eq!(tasks_in_job(&taskbag), Job::MoveIn);
        let request = move_tasks_in_request(taskbag.clone(), "en".into(), None, None).unwrap();
        assert!(request.contains("Allomos renamed the old taskbag (format allomos/2;"));
        // No agent setup in it (Task-A03-27): each agent registers itself.
        assert!(!request.contains("agent setup request"));
        // Asking twice changes nothing.
        assert!(move_taskbag_aside(taskbag.clone()).is_ok());
        assert!(!dir.join(".allomos").join("taskbag-allomos-4").exists());

        // The agent moved a task in: the usual request again.
        allomos_store::add_area(
            &taskbag,
            allomos_store::Actor::APP,
            std::time::Duration::from_secs(1),
            Some("A1"),
            "Area",
        )
        .unwrap();
        allomos_store::add_task(
            &taskbag,
            allomos_store::Actor::APP,
            std::time::Duration::from_secs(1),
            &serde_json::from_str(r#"{"area":"A1","title":"Bake"}"#).unwrap(),
        )
        .unwrap();
        let plan = connect_plan(&["claude_app".into()], "en", &dir, &taskbag);
        assert_eq!(
            plan[0].1,
            with_progress_in(
                "en",
                Job::Connect,
                &agent_connect_request_in("en", &dir, &taskbag)
            )
        );
        assert_eq!(tasks_in_job(&taskbag), Job::Cards);
        assert!(!version_check(taskbag).unwrap().needs_update());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn manual_shows_the_request_an_agent_would_get() {
        let dir =
            std::env::temp_dir().join(format!("allomos-desktop-manual-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let taskbag = dir.join(allomos_store::DEFAULT_DIR);
        allomos_store::init(&taskbag, "p", "P").unwrap();
        // Not connected yet: the connect request, in every language, as automatic sending.
        for language in ["en", "zh-CN", "ja"] {
            let auto = connect_plan(&["claude_app".into()], language, &dir, &taskbag);
            assert_eq!(manual_plan(language, &dir, &taskbag), auto[0].1);
        }
        assert_eq!(
            manual_plan("en", &dir, &taskbag),
            with_progress_in(
                "en",
                Job::Connect,
                &agent_connect_request_in("en", &dir, &taskbag)
            )
        );
        // Out of date: still the connect request, as automatic sending.
        std::fs::write(dir.join("AGENTS.md"), "## Tasks: Allomos\n\nalm next\n").unwrap();
        let auto = connect_plan(&["codex_cli".into()], "en", &dir, &taskbag);
        let manual = manual_plan("en", &dir, &taskbag);
        assert_eq!(manual, auto[0].1);
        assert!(!manual.starts_with("This project was set up by an older Allomos"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn build_a_new_app_types_the_connect_request_then_the_goal() {
        let dir =
            std::env::temp_dir().join(format!("allomos-desktop-build-app-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let taskbag = dir.join(allomos_store::DEFAULT_DIR);
        allomos_store::init(&taskbag, "p", "P").unwrap();
        let description = "A recipe box for two people.\nIt runs in the browser.";
        for language in ["en", "zh-CN", "ja"] {
            for quiet in [true, false] {
                let auto = build_app_plan(
                    "claude_app",
                    language,
                    &dir,
                    &taskbag,
                    description,
                    quiet,
                    false,
                );
                // One request: the wizard's connect request first, then the goal request. A quiet
                // run creates the Git repository itself; a normal run asks (Task-A03-35).
                let git = if quiet { GitStep::Create } else { GitStep::Ask };
                let connect = agent_connect_request_with(language, &dir, &taskbag, git);
                assert!(auto.starts_with(connect.trim_end()), "{language}");
                assert_eq!(
                    connect.contains("alm ai-decided <"),
                    quiet,
                    "{language}: only a quiet run records git init as an AI decision"
                );
                assert_eq!(
                    connect == agent_connect_request_in(language, &dir, &taskbag),
                    !quiet,
                    "{language}: a normal run asks, as connecting does"
                );
                let goal = allomos_store::goal_request_in(language, description, quiet);
                assert!(auto.contains(goal.trim_end()), "{language}");
                // One progress request, for the whole job (Task-A05-08).
                assert_eq!(
                    auto,
                    with_progress_in(
                        language,
                        Job::BuildApp,
                        &format!("{}\n\n{goal}", connect.trim_end())
                    )
                );
                assert_eq!(auto.matches("alm progress --finished").count(), 1);
                // The goal prompt in the task language, the description and the run mode.
                assert!(
                    auto.contains(allomos_store::goal(language).unwrap()),
                    "{language}"
                );
                assert!(
                    auto.contains("> A recipe box for two people.\n> It runs in the browser."),
                    "{language}"
                );
                let mode = if quiet {
                    "Run mode: quiet"
                } else {
                    "Run mode: normal"
                };
                assert!(auto.contains(mode), "{language}");
                // Manual shows the same request, for every agent.
                assert_eq!(
                    build_app_plan(MANUAL, language, &dir, &taskbag, description, quiet, false),
                    auto
                );
                assert_eq!(
                    build_app_plan(
                        "codex_cli",
                        language,
                        &dir,
                        &taskbag,
                        description,
                        quiet,
                        false
                    ),
                    auto
                );
                // Discuss (Task-A03-tundra474): the same connect request, then the discuss
                // request in place of the goal request, and one progress request.
                let talk = build_app_plan(
                    "claude_app",
                    language,
                    &dir,
                    &taskbag,
                    description,
                    quiet,
                    true,
                );
                let discuss = allomos_store::discuss_request_in(language, description, quiet);
                assert_eq!(
                    talk,
                    with_progress_in(
                        language,
                        Job::BuildApp,
                        &format!("{}\n\n{discuss}", connect.trim_end())
                    )
                );
                assert!(
                    !talk.contains(allomos_store::goal(language).unwrap()),
                    "{language}"
                );
                assert!(talk.contains(mode), "{language}");
            }
        }
        // Nothing to build without a description or a folder.
        assert!(matches!(
            build_app_checked(&dir, "  \n"),
            Err(Problem::BadUsage { .. })
        ));
        assert!(matches!(
            build_app_checked(&dir.join("missing"), description),
            Err(Problem::StartupNotAFolder { .. })
        ));
        assert_eq!(build_app_checked(&dir, description), Ok(()));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_dialog_explains_both_run_modes() {
        for language in ["en", "zh-CN", "ja"] {
            let text = run_modes_person(language);
            assert!(text.starts_with("## "), "{language}");
            assert_eq!(text.matches("\n## ").count(), 1, "{language}");
            assert!(!text.contains("For the agent"), "{language}");
            assert!(text.contains("`alm human`"), "{language}");
        }
    }

    #[test]
    fn connect_asks_only_for_the_instructions() {
        let project = PathBuf::from("/work/bakery");
        let taskbag = project.join(allomos_store::DEFAULT_DIR);
        // Only the first agent does the job, so only it reports progress (Task-A05-08).
        let connect = with_progress_in(
            "en",
            Job::Connect,
            &agent_connect_request_in("en", &project, &taskbag),
        );
        let setup = agent_setup_request_in("en", &project, &taskbag);

        // One agent: the connect request, with no note.
        let one = connect_plan(&["claude_app".into()], "en", &project, &taskbag);
        assert_eq!(one, [("claude_app".to_string(), connect.clone())]);

        // Two: the first chosen gets the connect request, the other the same instructions; both
        // are told about each other, and neither is asked to look for tasks (Task-A03-radish933).
        let two = connect_plan(
            &["claude_app".into(), "codex_cli".into()],
            "en",
            &project,
            &taskbag,
        );
        assert_eq!(two.len(), 2);
        assert_eq!(two[0].0, "claude_app");
        assert!(two[0].1.ends_with(&connect) && two[0].1.starts_with("Note: "));
        assert!(!two[0].1.contains("alm confirm"));
        assert_eq!(two[1].0, "codex_cli");
        assert!(two[1].1.ends_with(&setup) && two[1].1.starts_with("Note: "));
        assert!(!two[1].1.contains("alm confirm"));
        assert!(connect_plan(&[], "en", &project, &taskbag).is_empty());
    }

    /// Decision 0017: the wizard's steps 2 and 3 run no Git. A folder outside Git is set up
    /// without errors and stays outside Git; the agent connected in step 4 is asked to set Git up
    /// with the person and commit. The changes not shared yet come from the change history.
    #[test]
    fn the_wizard_sets_up_a_folder_outside_git_without_running_git() {
        let base = std::env::temp_dir().join(format!(
            "allomos-no-git-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let plain = base.join("plain");
        std::fs::create_dir_all(&plain).unwrap();
        let root = plain.join(allomos_store::DEFAULT_DIR);
        // Step 2 (Create) and step 3 (the project folder).
        let opened = create_taskbag(root.clone(), "Plain".into(), "en".into()).unwrap();
        assert!(opened.readme_added);
        let setup = set_project_folder(root.clone(), plain.clone()).unwrap();
        assert!(setup.project_folder.is_some());
        assert!(!plain.join(".git").exists() && !root.join(".git").exists());
        // Step 4's request asks the agent to set up Git with the person, then commit.
        let (project, taskbag) = (plain.as_path(), root.as_path());
        for language in ["en", "zh-CN", "ja"] {
            let request = agent_connect_request_in(language, project, taskbag);
            assert!(request.contains("`git init`"), "{language}");
            assert!(
                request.contains("commit -m \"Add Allomos taskbag\""),
                "{language}"
            );
        }
        // The app's own changes (create, project folder) are not shared yet.
        assert_eq!(allomos_store::events::unshared_app_changes(&root).count, 2);
        std::fs::remove_dir_all(&base).unwrap();
    }

    /// Task-A03-root435: `agent_status` names the recorded project folder (the Agents section
    /// asks it); it no longer counts the changes not shared yet.
    #[test]
    fn agent_status_names_the_project_folder() {
        let base = std::env::temp_dir().join(format!(
            "allomos-agent-status-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let plain = base.join("plain");
        std::fs::create_dir_all(&plain).unwrap();
        let root = plain.join(allomos_store::DEFAULT_DIR);
        create_taskbag(root.clone(), "Plain".into(), "en".into()).unwrap();
        set_project_folder(root.clone(), plain.clone()).unwrap();
        let status = tauri::async_runtime::block_on(agent_status(root.clone()));
        assert_eq!(
            status.project.canonicalize().unwrap(),
            plain.canonicalize().unwrap()
        );
        std::fs::remove_dir_all(&base).unwrap();
    }

    /// The task pane's Approve and Withdraw (Task-A01-baker800): an approval of the description
    /// shown, channel app, changing no state; Approve and make ready also makes it ready; a plan
    /// changed since it was shown is refused; Withdraw needs a reason and records it.
    #[test]
    fn the_task_pane_approves_and_withdraws() {
        let dir = std::env::temp_dir().join(format!(
            "allomos-approve-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = dir.join(allomos_store::DEFAULT_DIR);
        init(&root, "p", "P").unwrap();
        let lock = allomos_store::Lock::acquire(&root, std::time::Duration::ZERO).unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.create_area(&allomos_core::ops::new_area(&bag, "A01", "First").unwrap())
            .unwrap();
        let bag = lock.load().taskbag.unwrap();
        let mut task = allomos_core::ops::new_task(&bag, "A01-01", "A01", "Do it", None).unwrap();
        task.description = "## Outcome\n\nThe plan.".into();
        lock.create_task(&task).unwrap();
        drop(lock);
        let state =
            |root: &Path| allomos_store::taskbag_of(root, load(root)).unwrap().0.tasks[0].state;
        assert_eq!(state(&root), allomos_core::State::Backlog);

        assert_eq!(
            approve_as(&root, None, "A01-01", &task.description, None, false),
            Err(Problem::HolderMissing)
        );
        // Shown an older plan: nothing recorded.
        assert!(matches!(
            approve_as(&root, Some("Will"), "A01-01", "## Outcome", None, false),
            Err(Problem::TaskChanged { .. })
        ));
        assert!(open_taskbag(root.clone()).approvals.is_empty());

        approve_as(
            &root,
            Some("Will"),
            "A01-01",
            &task.description,
            Some("2099-01-01"),
            false,
        )
        .unwrap();
        assert_eq!(state(&root), allomos_core::State::Backlog);
        let opened = open_taskbag(root.clone());
        let a = &opened.approvals["A01-01"][0];
        assert_eq!(a.approval.channel, "app");
        assert_eq!(a.approval.expires.as_deref(), Some("2099-01-01"));
        assert_eq!(a.status, allomos_core::approval::Status::Applies);
        let id = a.approval.id.clone();

        // Approve and make ready: a second approval, then ready.
        approve_as(&root, Some("Will"), "A01-01", &task.description, None, true).unwrap();
        assert_eq!(state(&root), allomos_core::State::Ready);
        assert_eq!(open_taskbag(root.clone()).approvals["A01-01"].len(), 2);

        assert_eq!(
            withdraw_as(&root, Some("Will"), &id, "  "),
            Err(Problem::WithdrawReasonMissing)
        );
        withdraw_as(&root, Some("Will"), &id, "Plan is wrong").unwrap();
        let opened = open_taskbag(root.clone());
        assert_eq!(
            opened.approvals["A01-01"][0].status,
            allomos_core::approval::Status::Withdrawn
        );
        assert_eq!(state(&root), allomos_core::State::Ready);
        let history = allomos_store::events::history(&root, "A01-01");
        let e = history
            .events
            .iter()
            .find(|e| e.command == "withdraw")
            .unwrap();
        assert_eq!(e.via, "app");
        assert_eq!(allomos_core::event::reason(e), Some("Plan is wrong"));
        assert_eq!(
            withdraw_as(&root, Some("Will"), &id, "again"),
            Err(Problem::ApprovalAlreadyWithdrawn { id })
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Decision 0026: the task pane's Blocked by field sets a task's blockers without changing
    /// its state, and finishing the last blocker there (Done, as the interface sends it) leaves
    /// the task as it is: it is just no longer blocked.
    #[test]
    fn blockers_set_in_the_app_keep_the_state_when_set_and_when_they_finish() {
        let dir = std::env::temp_dir().join(format!(
            "allomos-unblock-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = dir.join(allomos_store::DEFAULT_DIR);
        init(&root, "p", "P").unwrap();
        let lock = allomos_store::Lock::acquire(&root, std::time::Duration::ZERO).unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.create_area(&allomos_core::ops::new_area(&bag, "A01", "First").unwrap())
            .unwrap();
        for id in ["A01-01", "A01-02"] {
            let bag = lock.load().taskbag.unwrap();
            lock.create_task(&allomos_core::ops::new_task(&bag, id, "A01", id, None).unwrap())
                .unwrap();
        }
        drop(lock);
        let block: allomos_core::ops::Edit = serde_json::from_value(serde_json::json!(
            { "field": "blocked_by", "ids": ["A01-01"] }
        ))
        .unwrap();
        edit_task_as(&root, Some("Will"), "A01-02", &block).unwrap();
        let bag = load(&root).taskbag.unwrap();
        let t = bag.tasks.iter().find(|t| t.id == "A01-02").unwrap();
        assert_eq!(t.state, allomos_core::State::Backlog);
        assert!(allomos_core::ops::is_blocked(&bag, t));
        let done: allomos_core::ops::Edit =
            serde_json::from_value(serde_json::json!({ "field": "state", "state": "done" }))
                .unwrap();
        edit_task_as(&root, Some("Will"), "A01-01", &done).unwrap();
        let bag = load(&root).taskbag.unwrap();
        let t = bag.tasks.iter().find(|t| t.id == "A01-02").unwrap();
        assert_eq!(t.state, allomos_core::State::Backlog);
        assert_eq!(t.blocked_by, ["A01-01"]);
        assert!(!allomos_core::ops::is_blocked(&bag, t));
        let e = &allomos_store::events::history(&root, "A01-02").events[0];
        assert_eq!((e.via.as_str(), e.command.as_str()), ("app", "block"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The task pane's change command (Task-A01-18): the edit arrives as the interface sends it,
    /// is refused without the person's name or by the holder rule, and is otherwise recorded
    /// under that name, through the app.
    #[test]
    fn the_task_pane_changes_a_task_under_the_persons_name() {
        let dir = std::env::temp_dir().join(format!(
            "allomos-edit-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = dir.join(allomos_store::DEFAULT_DIR);
        init(&root, "p", "P").unwrap();
        let lock = allomos_store::Lock::acquire(&root, std::time::Duration::ZERO).unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.create_area(&allomos_core::ops::new_area(&bag, "A01", "First").unwrap())
            .unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.create_task(
            &allomos_core::ops::new_task(&bag, "A01-01", "A01", "Do it", None).unwrap(),
        )
        .unwrap();
        drop(lock);

        // As the interface sends it: `{ field: "label", label: "implemented" }`.
        let edit: allomos_core::ops::Edit =
            serde_json::from_value(serde_json::json!({ "field": "label", "label": "implemented" }))
                .unwrap();
        assert_eq!(
            edit_task_as(&root, None, "A01-01", &edit),
            Err(Problem::HolderMissing)
        );
        assert_eq!(
            edit_task_as(&root, Some(" "), "A01-01", &edit),
            Err(Problem::HolderMissing)
        );
        assert!(load(&root).taskbag.unwrap().tasks[0].labels.is_empty());

        let task = edit_task_as(&root, Some("Will"), "A01-01", &edit).unwrap();
        assert_eq!(task.labels, ["implemented"]);
        let h = allomos_store::events::history(&root, "A01-01");
        let e = &h.events[0];
        // Under the person's pseudonym, never their name (decision 0021 § 2, Task-A01-frolic587);
        // these tests have no settings folder for one, so under nobody.
        assert_eq!(
            (e.by.as_deref(), e.via.as_str(), e.command.as_str()),
            (None, "app", "label")
        );

        // A task an agent holds is refused with the same problem `alm` gives.
        let lock = allomos_store::Lock::acquire(&root, std::time::Duration::ZERO).unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.save_task(
            &allomos_core::ops::claim(&bag, "A01-01", Some("claude/x"), false)
                .unwrap()
                .task,
        )
        .unwrap();
        drop(lock);
        assert_eq!(
            edit_task_as(&root, Some("Will"), "A01-01", &edit),
            Err(Problem::TaskHeld {
                id: "A01-01".into(),
                holder: "claude/x".into()
            })
        );

        // Task-A03-19: the agent's decision shows when the taskbag opens; the Reviewed button
        // needs the person's name, even on a task an agent holds, and takes it away.
        assert_eq!(
            review_as(&root, Some("Will"), "A01-01"),
            Err(Problem::NothingToReview {
                id: "A01-01".into()
            })
        );
        allomos_store::record_ai_decision(
            &root,
            allomos_store::Actor::alm(Some("claude/x")),
            LOCK_WAIT,
            "A01-01",
            "Names: alm note",
            None,
        )
        .unwrap();
        let opened = open_taskbag(root.clone());
        assert_eq!(opened.ai_decisions["A01-01"][0].text, "Names: alm note");
        // Task-A01-garden385: the risk an agent recorded comes along when the taskbag opens; a
        // task that is not classified is left out (the window shows it as "Not classified").
        assert!(opened.classifications.is_empty());
        allomos_store::classify::set(
            &root,
            allomos_store::Actor::alm(Some("claude/x")),
            LOCK_WAIT,
            "A01-01",
            &allomos_core::classify::Given {
                risk: Some("high"),
                ..Default::default()
            },
        )
        .unwrap();
        let opened = open_taskbag(root.clone());
        assert_eq!(
            opened.classifications["A01-01"].risk.as_deref(),
            Some("high")
        );
        assert_eq!(
            serde_json::to_value(&opened).unwrap()["classifications"]["A01-01"]["risk"],
            "high"
        );
        assert_eq!(
            review_as(&root, None, "A01-01"),
            Err(Problem::HolderMissing)
        );
        review_as(&root, Some("Will"), "A01-01").unwrap();
        assert!(open_taskbag(root.clone()).ai_decisions.is_empty());
        let e = &allomos_store::events::history(&root, "A01-01").events[0];
        assert_eq!(
            (e.by.as_deref(), e.via.as_str(), e.command.as_str()),
            (None, "app", "reviewed")
        );

        // Task-A01-50: Reopen… on a finished task, as the interface sends it; the reason goes in
        // the history under the person's name. An open task is refused as `alm reopen` refuses.
        let reopen: allomos_core::ops::Edit = serde_json::from_value(
            serde_json::json!({ "field": "reopen", "note": "The check failed" }),
        )
        .unwrap();
        assert_eq!(
            edit_task_as(&root, Some("Will"), "A01-01", &reopen)
                .unwrap_err()
                .code(),
            "cannot_reopen"
        );
        allomos_store::change_task(
            &root,
            allomos_store::Actor::alm(Some("claude/x")),
            LOCK_WAIT,
            |bag| {
                allomos_core::ops::finish(bag, "A01-01", Some("claude/x"), None, None)
                    .map(|o| (o, "done"))
            },
        )
        .unwrap();
        let task = edit_task_as(&root, Some("Will"), "A01-01", &reopen).unwrap();
        assert_eq!(
            (task.state, task.holder, task.labels),
            (allomos_core::State::Ready, None, vec![])
        );
        let e = &allomos_store::events::history(&root, "A01-01").events[0];
        assert_eq!(
            (e.by.as_deref(), e.via.as_str(), e.command.as_str()),
            (None, "app", "reopen")
        );
        assert!(e.changes.iter().any(
            |c| c.field == "reopen_note" && c.to == Some(serde_json::json!("The check failed"))
        ));
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// The New area, New iteration and New task forms (Task-A01-19), with the arguments as the
    /// interface sends them: suggested IDs, the same refusals as `alm add`, events under the name.
    #[test]
    fn the_list_adds_areas_iterations_and_tasks_under_the_persons_name() {
        let dir = std::env::temp_dir().join(format!(
            "allomos-add-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = dir.join(allomos_store::DEFAULT_DIR);
        init(&root, "p", "P").unwrap();
        let ids = next_ids(root.clone()).unwrap();
        assert_eq!(ids.area, "A01");
        // An unused `I-<labor word>` (Task-A00-rope803).
        assert!(ids.iteration.starts_with("I-"), "{}", ids.iteration);
        assert!(ids.tasks.is_empty());

        // Without the person's name, nothing is added.
        assert_eq!(
            add_area_as(&root, None, None, "Tracking"),
            Err(Problem::HolderMissing)
        );
        // New area: an empty ID box takes the suggested one.
        let area = add_area_as(&root, Some("Will"), Some(" ".into()), "Tracking").unwrap();
        assert_eq!(area.id, "A01");
        assert_eq!(
            add_area_as(&root, Some("Will"), Some("A01".into()), "Again"),
            Err(Problem::DuplicateAreaId { id: "A01".into() })
        );

        // New iteration, as `messenger.addIteration` sends it.
        let it: NewIteration = serde_json::from_value(serde_json::json!({
            "id": "I07", "name": "Seven", "release": "v0.7", "start": "2026-10-15", "end": null
        }))
        .unwrap();
        assert_eq!(add_iteration_as(&root, Some("Will"), it).unwrap().id, "I07");
        let bad: NewIteration = serde_json::from_value(
            serde_json::json!({ "id": null, "name": "Eight", "end": "31/10/2026" }),
        )
        .unwrap();
        assert_eq!(
            add_iteration_as(&root, Some("Will"), bad),
            Err(Problem::InvalidDate {
                value: "31/10/2026".into()
            })
        );
        let ids = next_ids(root.clone()).unwrap();
        assert!(ids.iteration.starts_with("I-"), "{}", ids.iteration);
        // A word ID in the area (Task-A01-54).
        let suggested = ids.tasks.get("A01").unwrap();
        assert!(
            suggested.starts_with("A01-") && !allomos_core::ids::numbered(suggested),
            "{suggested}"
        );

        // New task in an iteration group, as `messenger.addTask` sends it: added, then planned.
        let task: allomos_store::NewTask = serde_json::from_value(serde_json::json!({
            "id": "A01-01", "area": "A01", "title": "Plan the week", "description": "Why.", "iteration": "I07"
        }))
        .unwrap();
        let t = add_task_as(&root, Some("Will"), task).unwrap();
        assert_eq!(
            (t.id.as_str(), t.iteration.as_deref(), t.state),
            ("A01-01", Some("I07"), allomos_core::State::Backlog)
        );
        // Task-A03-22: with no base sent, none is recorded.
        assert!(open_taskbag(root.clone()).branches.is_empty());
        let h = allomos_store::events::history(&root, "A01-01");
        let seen: Vec<_> = h
            .events
            .iter()
            .map(|e| (e.command.as_str(), e.by.as_deref(), e.via.as_str()))
            .collect();
        // Task-A01-52: the task's last change is its newest event.
        assert_eq!(
            open_taskbag(root.clone()).last_changed.get("A01-01"),
            Some(&h.events[0].at)
        );
        // Newest first.
        assert_eq!(seen, [("plan", None, "app"), ("add task", None, "app")]);
        // The task pane's "New…" iteration, as `messenger.addIterationAndPlan` sends it: added
        // with the next free ID, then the task planned in it (two events, newest first).
        assert_eq!(
            add_iteration_and_plan_as(&root, None, "A01-01", NewIteration::default()),
            Err(Problem::HolderMissing)
        );
        let pane: NewIteration = serde_json::from_value(serde_json::json!({
            "id": null, "name": "Next", "release": null, "start": "2026-11-01", "end": null
        }))
        .unwrap();
        let t = add_iteration_and_plan_as(&root, Some("Will"), "A01-01", pane).unwrap();
        assert!(t.iteration.as_deref().is_some_and(|i| i.starts_with("I-")));
        let h = allomos_store::events::history(&root, "A01-01");
        assert_eq!(
            (h.events[0].command.as_str(), h.events[0].via.as_str()),
            ("plan", "app")
        );
        let nameless: NewIteration =
            serde_json::from_value(serde_json::json!({ "name": " " })).unwrap();
        assert_eq!(
            add_iteration_and_plan_as(&root, Some("Will"), "A01-01", nameless),
            Err(Problem::IterationNameMissing)
        );
        assert_eq!(load(&root).taskbag.unwrap().iterations.len(), 2);
        let unknown: allomos_store::NewTask =
            serde_json::from_value(serde_json::json!({ "area": "A09", "title": "x" })).unwrap();
        assert_eq!(
            add_task_as(&root, Some("Will"), unknown),
            Err(Problem::UnknownArea { area: "A09".into() })
        );
        assert_eq!(load(&root).taskbag.unwrap().tasks.len(), 1);

        // Task-A03-22: the base the person picked is recorded in the `add task` event and shows
        // when the taskbag opens; a `branch` sent by mistake chooses nothing.
        let picked: allomos_store::NewTask = serde_json::from_value(serde_json::json!({
            "area": "A01", "title": "On a branch", "base": "storage-v2", "branch": "claude/x"
        }))
        .unwrap();
        let t = add_task_as(&root, Some("Will"), picked).unwrap();
        let opened = open_taskbag(root.clone());
        assert_eq!(opened.branches[&t.id].base.as_deref(), Some("storage-v2"));
        assert_eq!(opened.branches[&t.id].working, None);
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// Task-A01-29: renaming an area, changing an iteration and deleting either, with the
    /// arguments as `messenger.renameArea`, `changeIteration`, `deleteArea` and `deleteIteration`
    /// send them; each under the person's name, each one event; a non-empty one is not deleted.
    #[test]
    fn the_list_renames_changes_and_deletes_under_the_persons_name() {
        let dir = std::env::temp_dir().join(format!(
            "allomos-rename-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = dir.join(allomos_store::DEFAULT_DIR);
        init(&root, "p", "P").unwrap();
        let will = Some("Will");
        add_area_as(&root, will, Some("A01".into()), "Tracking").unwrap();
        add_area_as(&root, will, Some("A02".into()), "Spare").unwrap();
        let it = |id: &str| -> NewIteration {
            serde_json::from_value(serde_json::json!({ "id": id, "name": id, "end": "2026-10-10" }))
                .unwrap()
        };
        add_iteration_as(&root, will, it("I01")).unwrap();
        add_iteration_as(&root, will, it("I02")).unwrap();
        let task: allomos_store::NewTask = serde_json::from_value(
            serde_json::json!({ "area": "A01", "title": "Do it", "iteration": "I01" }),
        )
        .unwrap();
        add_task_as(&root, will, task).unwrap();
        let count = || allomos_store::events::list(&root).0.len();
        let last = || {
            let e = allomos_store::events::list(&root).0.pop().unwrap();
            (e.command, e.id.unwrap_or_default(), e.by, e.via)
        };
        let by = |c: &str, id: &str| {
            (
                c.to_string(),
                id.to_string(),
                // The pseudonym, never the name (Task-A01-frolic587); no settings folder here.
                None,
                "app".to_string(),
            )
        };

        // Without the person's name, nothing changes.
        assert_eq!(
            rename_area_as(&root, None, "A01", "x"),
            Err(Problem::HolderMissing)
        );
        let start = count();
        assert_eq!(
            rename_area_as(&root, will, "A01", "Tracking work")
                .unwrap()
                .name,
            "Tracking work"
        );
        assert_eq!((count(), last()), (start + 1, by("rename area", "A01")));

        // The iteration form sends every field; an empty release clears it.
        let change: allomos_core::ops::IterationChange = serde_json::from_value(
            serde_json::json!({ "name": "I01", "release": "", "start": "", "end": "2026-10-20" }),
        )
        .unwrap();
        let changed = change_iteration_as(&root, will, "I01", &change).unwrap();
        assert_eq!(changed.end.as_deref(), Some("2026-10-20"));
        assert_eq!(
            (count(), last()),
            (start + 2, by("change iteration", "I01"))
        );
        let bad: allomos_core::ops::IterationChange =
            serde_json::from_value(serde_json::json!({ "end": "20/10/2026" })).unwrap();
        assert_eq!(
            change_iteration_as(&root, will, "I01", &bad),
            Err(Problem::InvalidDate {
                value: "20/10/2026".into()
            })
        );

        // Not empty: refused, nothing deleted.
        assert_eq!(
            delete_area_as(&root, will, "A01"),
            Err(Problem::AreaNotEmpty {
                id: "A01".into(),
                count: "1".into()
            })
        );
        assert_eq!(
            delete_iteration_as(&root, will, "I01"),
            Err(Problem::IterationNotEmpty {
                id: "I01".into(),
                count: "1".into()
            })
        );
        assert_eq!(count(), start + 2);
        delete_iteration_as(&root, will, "I02").unwrap();
        assert_eq!(
            (count(), last()),
            (start + 3, by("delete iteration", "I02"))
        );
        delete_area_as(&root, will, "A02").unwrap();
        assert_eq!((count(), last()), (start + 4, by("delete area", "A02")));
        let bag = load(&root).taskbag.unwrap();
        assert_eq!(
            (
                bag.areas.len(),
                bag.iterations.len(),
                bag.areas[0].name.as_str()
            ),
            (1, 1, "Tracking work")
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
