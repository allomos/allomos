//! Allomos desktop app: the Tauri window plus the commands the interface may call.
//!
//! The interface never calls these directly; it goes through its messenger module
//! (`apps/web/src/messenger.ts`), so the same interface can later reach the desktop's
//! local server from a browser without changes.

mod agents;
mod alm_command;
mod update;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use allomos_core::{Loaded, Problem};
use allomos_store::progress::{Job, with_progress_in};
use allomos_store::{
    AgentSetup, ConfirmRequest, Inspection, LinkTarget, SetupState, add_agent_instructions,
    add_readme_if_empty, agent_connect_request_in, agent_setup, agent_setup_request_in,
    answer_confirm, id_from_name, init, inspect, load, pending_confirms, picked_folder,
    project_folder_of, record_project_folder, resolve_task_link, setup_state, stamp, versions,
};
use serde::Serialize;
use tauri::menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Manager, Wry};

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
    /// Each task's base and working branch, by task ID (Task-A03-22), from the history: the New
    /// task form chooses the new task's base from the open tasks' bases.
    branches: std::collections::BTreeMap<String, allomos_core::branch::Branches>,
    /// When each task last changed, by task ID (Task-A01-52), from the history: the Iterations
    /// tab's sort menu orders by it.
    last_changed: std::collections::BTreeMap<String, String>,
    /// The card files agents reported as still in the project folder (Task-A00-30): the notice
    /// "N tasks keep their card files in the project folder" with **Update the taskbag…**.
    cards_outside: Vec<allomos_core::card::Outside>,
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
#[tauri::command]
fn inspect_folder(path: PathBuf) -> FolderInfo {
    FolderInfo {
        inspection: inspect(&path),
    }
}

/// Load the taskbag at `root`.
#[tauri::command]
fn open_taskbag(root: PathBuf) -> Opened {
    let loaded = load(&root);
    // Read from the tasks' Card: links and agents' reports (Task-A01-51), so the notice shows
    // as soon as the taskbag opens and on every reload.
    let cards_outside = match &loaded.taskbag {
        Some(bag) => allomos_store::cards::outside_of(&root, bag),
        None => Vec::new(),
    };
    Opened {
        loaded,
        ai_decisions: allomos_store::ai_decisions(&root),
        branches: allomos_store::branches(&root),
        last_changed: allomos_store::last_changed(&root),
        cards_outside,
        root,
        readme_added: false,
    }
}

/// A fingerprint of the taskbag's files. The window asks for it about once a second and
/// reloads when it changes, so changes made by `alm` show without reopening the folder.
/// Sent as text: a 64-bit number does not fit a JavaScript number.
#[tauri::command]
fn taskbag_stamp(root: PathBuf) -> String {
    stamp(&root).to_string()
}

/// Create an empty taskbag at `root` (normally `<folder>/.allomos/taskbag/`), then load it.
/// Empty, with no example tasks, so the open-folder flow can tell whether the project has tasks
/// yet (docs/product/OPEN-FOLDER-FLOW.md).
/// When the picked folder looks empty, also write `README-allomos.md` there, in the
/// interface's `language`; a README that cannot be written does not undo the taskbag.
/// (Connecting AI agents with the project is a separate step: `connect_agents`.)
#[tauri::command]
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
        branches: Default::default(),
        last_changed: Default::default(),
        cards_outside: Vec::new(),
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
#[tauri::command]
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
#[tauri::command]
fn owner_name() -> Option<String> {
    allomos_store::saved_owner_name()
}

/// Keep the person's name in the app's settings, asked once (owner, 2026-10-14).
#[tauri::command]
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
    let actor = app_actor(name)?;
    allomos_store::change_task(root, actor, LOCK_WAIT, |bag| {
        allomos_core::ops::edit(bag, id, actor.by, edit)
    })
    .map(|c| c.task)
}

/// The person has reviewed task `id`'s AI decisions (Task-A03-19): the same change as
/// `alm reviewed`, made as the person's saved name.
#[tauri::command]
fn review_ai_decisions(root: PathBuf, id: String) -> Result<(), Problem> {
    review_as(&root, allomos_store::saved_owner_name().as_deref(), &id)
}

fn review_as(root: &Path, name: Option<&str>, id: &str) -> Result<(), Problem> {
    let actor = app_actor(name)?;
    allomos_store::review_ai_decisions(root, actor, LOCK_WAIT, id).map(|_| ())
}

#[tauri::command]
fn edit_task(
    root: PathBuf,
    id: String,
    edit: allomos_core::ops::Edit,
) -> Result<allomos_core::Task, Problem> {
    edit_task_as(
        &root,
        allomos_store::saved_owner_name().as_deref(),
        &id,
        &edit,
    )
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

#[tauri::command]
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

#[tauri::command]
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

#[tauri::command]
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

#[tauri::command]
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

#[tauri::command]
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

#[tauri::command]
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

#[tauri::command]
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

#[tauri::command]
fn delete_area(root: PathBuf, id: String) -> Result<(), Problem> {
    delete_area_as(&root, allomos_store::saved_owner_name().as_deref(), &id)
}

/// An iteration deleted from its group (Task-A01-29): the same rule (refused while any task is
/// planned in it, done and dropped ones too), lock and recorded event as `alm delete iteration`.
fn delete_iteration_as(root: &Path, name: Option<&str>, id: &str) -> Result<(), Problem> {
    allomos_store::delete_iteration(root, app_actor(name)?, LOCK_WAIT, id).map(|_| ())
}

#[tauri::command]
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
/// order chosen: the first gets the connect request (add Allomos's instructions, then find the
/// project's tasks and offer to move them), the others only the instructions request, because
/// two agents moving tasks would add every task twice. With several agents, each request starts
/// with a note naming the others.
fn connect_plan(
    agents: &[String],
    language: &str,
    project: &Path,
    taskbag: &Path,
) -> Vec<(String, String)> {
    connect_plan_with(agents, language, project, taskbag, true)
}

/// [`connect_plan`]; `progress`: the first agent's request asks it to report its progress to the
/// app (Task-A05-08), unless a larger job's request does (Build a new app…).
fn connect_plan_with(
    agents: &[String],
    language: &str,
    project: &Path,
    taskbag: &Path,
    progress: bool,
) -> Vec<(String, String)> {
    let job = |job: Job, request: String| {
        if progress {
            with_progress_in(language, job, &request)
        } else {
            request
        }
    };
    let note = agents::shared_note(agents, language);
    // Tasks from an older Allomos's taskbag to move in (Task-A01-45), or an out-of-date section
    // (Task-A01-25): the first agent is asked to do that instead.
    let check = versions::check_connected(taskbag);
    let move_in = check.as_ref().and_then(|c| c.move_in.clone());
    let update = check.filter(|c| !c.outdated_files().is_empty());
    agents
        .iter()
        .enumerate()
        .map(|(i, id)| {
            // The first agent does the job and reports its progress to the app (Task-A05-08).
            let request = if let (0, Some(m)) = (i, &move_in) {
                job(
                    Job::MoveIn,
                    allomos_store::agent_move_in_request_in(language, project, taskbag, m),
                )
            } else if let (0, Some(check)) = (i, &update) {
                job(
                    Job::Update,
                    versions::agent_update_request_in(language, project, taskbag, check),
                )
            } else if i == 0 {
                job(
                    Job::Connect,
                    agent_connect_request_in(language, project, taskbag),
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
fn build_app_plan(
    agent: &str,
    language: &str,
    project: &Path,
    taskbag: &Path,
    description: &str,
    quiet: bool,
) -> String {
    let connect = connect_plan_with(&[agent.to_string()], language, project, taskbag, false)
        .remove(0)
        .1;
    let goal = allomos_store::goal_request_in(language, description, quiet);
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
) -> Result<(), Problem> {
    build_app_checked(&project, &description)?;
    let found = agents::find_agents();
    let Some(chosen) = found.iter().find(|a| a.id == agent) else {
        return Err(Problem::AgentNotFound { agent });
    };
    let request = build_app_plan(&agent, &language, &project, &taskbag, &description, quiet);
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
) -> Result<String, Problem> {
    build_app_checked(&project, &description)?;
    let request = build_app_plan(MANUAL, &language, &project, &taskbag, &description, quiet);
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
#[tauri::command]
fn version_check(root: PathBuf) -> Option<versions::VersionCheck> {
    versions::check_connected(&root)
}

/// **Update the taskbag…** on an older Allomos's taskbag (Task-A01-45): rename it aside
/// (`taskbag-allomos-<n>`) and create a new, empty taskbag in its place, then load that; the same
/// dialog then asks the chosen agent to move the tasks across ([`move_tasks_in`]). Nothing in the
/// old folder is changed.
#[tauri::command]
fn move_taskbag_aside(root: PathBuf) -> Result<Opened, Problem> {
    versions::move_aside(&root)?;
    Ok(Opened {
        loaded: load(&root),
        ai_decisions: Default::default(),
        branches: Default::default(),
        last_changed: Default::default(),
        cards_outside: Vec::new(),
        root,
        readme_added: false,
    })
}

/// The project's handoff rules (as written, and with the defaults filled in) and its template
/// (Task-A03-06), for the Handoff settings dialog.
#[derive(Serialize)]
struct HandoffSettings {
    rules: allomos_core::HandoffRules,
    applied: allomos_core::AppliedRules,
    template: String,
}

fn handoff_settings_of(root: &Path) -> Result<HandoffSettings, Problem> {
    let loaded = load(root);
    let Some(bag) = loaded.taskbag else {
        return Err(loaded.issues.first().map(|i| i.problem.clone()).unwrap_or(
            Problem::NotATaskbag {
                path: root.display().to_string(),
            },
        ));
    };
    let rules = bag.project.handoff.unwrap_or_default();
    Ok(HandoffSettings {
        applied: rules.applied(None),
        rules,
        template: allomos_store::handoff::template(root),
    })
}

#[tauri::command]
fn handoff_settings(root: PathBuf) -> Result<HandoffSettings, Problem> {
    handoff_settings_of(&root)
}

/// Change the handoff rules, as `alm handoff-rules` does: the project's own (no `agent`), or one
/// agent's or account's override. Returns the settings as saved.
#[tauri::command]
fn set_handoff_rules(
    root: PathBuf,
    quota: Option<String>,
    context: Option<String>,
    write_when: Option<String>,
    agent: Option<String>,
    clear: bool,
) -> Result<HandoffSettings, Problem> {
    let lock = allomos_store::Lock::acquire(&root, std::time::Duration::from_secs(5))?;
    let loaded = lock.load();
    let bag = loaded
        .taskbag
        .ok_or_else(|| loaded.issues[0].problem.clone())?;
    let change = allomos_core::ops::HandoffChange {
        quota_gate: quota.as_deref(),
        context_gate: context.as_deref(),
        write_when: write_when.as_deref(),
        agent: agent.as_deref(),
        clear,
    };
    let project = allomos_core::ops::set_handoff_rules(&bag, &change)?;
    let event = allomos_store::Actor::APP.event(
        "handoff-rules",
        allomos_core::event::Kind::Project,
        Some(&project.id),
        allomos_core::event::changes(Some(&bag.project), &project, &[]),
    );
    lock.record(&event, |l| l.save_project(&project))?;
    drop(lock);
    handoff_settings_of(&root)
}

#[tauri::command]
fn set_handoff_template(root: PathBuf, text: String) -> Result<HandoffSettings, Problem> {
    allomos_store::handoff::set_template(&root, &text)?;
    handoff_settings_of(&root)
}

/// One task's history, newest first, and when the taskbag started recording (Task-A01-17).
#[tauri::command]
fn task_history(root: PathBuf, id: String) -> allomos_store::events::History {
    allomos_store::events::history(&root, &id)
}

/// Every handoff in the taskbag, newest first (the Handoffs tab and the task pane).
#[tauri::command]
fn list_handoffs(root: PathBuf) -> Vec<allomos_store::handoff::Handoff> {
    allomos_store::handoff::list(&root)
}

/// A handoff written by a person in the app: the same file an agent writes with `alm handoff`.
#[tauri::command]
fn write_handoff(
    root: PathBuf,
    writer: String,
    tasks: Vec<String>,
    body: String,
) -> Result<allomos_store::handoff::Handoff, Problem> {
    let known: Vec<String> = load(&root)
        .taskbag
        .map(|b| b.tasks.into_iter().map(|t| t.id).collect())
        .unwrap_or_default();
    allomos_store::handoff::write(&root, &writer, Some(&tasks), Some(&body), &known, true)
        .map(|w| w.handoff)
}

/// Where a newer Allomos is downloaded (owner, 2026-10-12: the GitHub releases page, even
/// while it is private). A fixed address: the window cannot ask to open any other.
const DOWNLOAD_URL: &str = "https://github.com/allomos/allomos/releases";

/// Open the download page in the web browser, for a taskbag that needs a newer Allomos.
#[tauri::command]
fn open_download_page() -> Result<(), Problem> {
    opener()
        .arg(DOWNLOAD_URL)
        .spawn()
        .map(drop)
        .map_err(|e| Problem::OpenFailed {
            path: DOWNLOAD_URL.to_string(),
            detail: e.to_string(),
        })
}

/// Whether a newer Allomos is on GitHub (Task-A07-16). `asked` when the person chose
/// **Check for Updates…**: GitHub is asked now, and a skipped version is offered again.
/// Otherwise GitHub is asked only when the last answer is a day old.
#[tauri::command]
async fn update_status(asked: bool) -> update::UpdateStatus {
    update::check(asked)
}

/// "Skip this version" on the update notice.
#[tauri::command]
fn skip_update(version: String) {
    let _ = allomos_store::skip_update(&version);
}

/// The taskbag's format compared with this Allomos's, whether or not an agent is connected: the
/// wizard's step 2 does not go on with a newer taskbag (owner, 2026-10-12).
#[derive(Serialize)]
struct TaskbagFormat {
    format: versions::FormatState,
    /// What this Allomos writes, for the message: `allomos/1`.
    current: &'static str,
}

#[tauri::command]
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

#[tauri::command]
fn instruction_files(project: PathBuf) -> InstructionFiles {
    InstructionFiles {
        agents_md: project.join("AGENTS.md").is_file(),
        claude_md: project.join("CLAUDE.md").is_file(),
    }
}

/// The entries under `## Agents set up` in the taskbag's project notes (Task-A03-24): the agents
/// panel shows "Not set up for this project yet." on an agent with no entry. Only that section is
/// read; nothing else is checked.
#[tauri::command]
fn agents_set_up(taskbag: PathBuf) -> Vec<String> {
    allomos_store::handoff::agents_set_up(&taskbag)
}

/// Steps 2 to 4 of the open-folder flow for the taskbag at `root`: its project folder (recorded
/// here when AGENTS.md or CLAUDE.md sits beside `.allomos/`), whether that folder's agent
/// instructions mention Allomos, and whether the taskbag has tasks.
#[tauri::command]
fn project_setup(root: PathBuf) -> Result<SetupState, Problem> {
    setup_state(&root)
}

/// Record `folder` as the project folder of the taskbag at `root`; then the flow's new state.
#[tauri::command]
fn set_project_folder(root: PathBuf, folder: PathBuf) -> Result<SetupState, Problem> {
    record_project_folder(&root, &folder)?;
    setup_state(&root)
}

/// File → Connect with the Agents… and File → Update the Taskbag… are offered only while a
/// taskbag is open.
#[tauri::command]
fn set_connect_enabled(texts: tauri::State<MenuTexts>, enabled: bool) {
    let _ = texts.connect.set_enabled(enabled);
    let _ = texts.move_tasks_in.set_enabled(enabled);
}

/// The job **Update the taskbag…** asks an agent to do, for its progress popup (Task-A05-08):
/// moving an older Allomos's tasks in, or card files kept outside the taskbag.
fn tasks_in_job(taskbag: &Path) -> Job {
    if versions::move_in(taskbag).is_some() {
        Job::MoveIn
    } else {
        Job::Cards
    }
}

/// **Update the taskbag…** (Task-A01-45, approver 2026-10-16: one button for an older taskbag and
/// for card files kept outside the taskbag, Task-A00-30): open the chosen agent in the project
/// folder of the taskbag at `taskbag`, with the request for whichever applies typed in
/// ([`allomos_store::agent_update_taskbag_request_in`]; `agents`: the agents found on this
/// computer with no line under `## Agents set up`, Task-A03-24). Nothing is sent: the person sends it; the
/// agent shows its plan in the app (`alm confirm`) before anything changes.
#[tauri::command]
async fn move_tasks_in(
    agent: String,
    taskbag: PathBuf,
    language: String,
    agents: Vec<String>,
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
    let request = with_progress_in(
        &language,
        tasks_in_job(&taskbag),
        &allomos_store::agent_update_taskbag_request_in(&language, &project, &taskbag, &agents),
    );
    agents::ask(chosen, &project, &request)
}

/// **Update the taskbag…** with Manual: the same request, as an agent opened through a link gets
/// it, for the person to copy.
#[tauri::command]
fn move_tasks_in_request(taskbag: PathBuf, language: String, agents: Vec<String>) -> String {
    let project = allomos_store::versions::project_folder(&taskbag);
    let request = with_progress_in(
        &language,
        tasks_in_job(&taskbag),
        &allomos_store::agent_update_taskbag_request_in(&language, &project, &taskbag, &agents),
    );
    agents::link_request(&request, alm_command::own().as_deref())
}

/// What agents report about the jobs they do in `project` (`alm progress`, Task-A05-08), the
/// earliest started first.
#[tauri::command]
fn progress_reports(project: PathBuf) -> Vec<allomos_store::progress::Progress> {
    allomos_store::progress::reports(&project)
}

/// The person closed a finished (or silent) report: remove it.
#[tauri::command]
fn dismiss_progress(project: PathBuf, id: String) {
    allomos_store::progress::dismiss(&project, &id)
}

/// Questions an agent is asking about `project` with `alm confirm`, oldest first. Listing them
/// marks them seen, so `alm` knows this window shows them.
#[tauri::command]
fn confirm_requests(project: PathBuf) -> Vec<ConfirmRequest> {
    pending_confirms(&project)
}

/// The person's answer to question `id`: go ahead (`yes`) or cancel.
#[tauri::command]
fn answer_confirm_request(project: PathBuf, id: String, yes: bool) -> Result<(), Problem> {
    answer_confirm(&project, &id, yes)
}

/// Do the AI agents know about this project? What the window's agent line shows (Task-A03-04).
#[derive(Serialize)]
struct AgentStatus {
    /// The project folder the line is about.
    project: PathBuf,
    /// Which agents' instruction files tell them about Allomos.
    setup: AgentSetup,
    /// The newest change an agent made through `alm` under a name, from the change history
    /// (decision 0017: not from Git).
    last: Option<allomos_store::events::LastUse>,
    /// Changes made in the app that no agent has been told to share yet (decision 0017, row 9).
    unshared: usize,
    /// Who holds tasks in progress now.
    holders: Vec<String>,
}

#[tauri::command]
async fn agent_status(root: PathBuf) -> AgentStatus {
    let loaded = load(&root).taskbag;
    // The recorded project folder; before one is recorded, the folder holding `.allomos/`.
    let project = loaded
        .as_ref()
        .and_then(|bag| project_folder_of(&root, &bag.project))
        .unwrap_or_else(|| picked_folder(&root).unwrap_or(&root).to_path_buf());
    let mut holders: Vec<String> = loaded
        .map(|bag| {
            bag.tasks
                .into_iter()
                .filter(|t| t.state == allomos_core::State::InProgress)
                .filter_map(|t| t.holder)
                .collect()
        })
        .unwrap_or_default();
    holders.sort();
    holders.dedup();
    AgentStatus {
        setup: agent_setup(&project),
        last: allomos_store::events::last_agent_use(&root),
        unshared: allomos_store::events::unshared_app_changes(&root).count,
        holders,
        project,
    }
}

/// Write Allomos's section into `project`'s AGENTS.md and CLAUDE.md directly, for when no
/// agent is available. Returns the files changed.
#[tauri::command]
fn write_agent_files(project: PathBuf) -> Result<Vec<String>, Problem> {
    add_agent_instructions(&project).map_err(|e| Problem::WriteFailed {
        path: project.join("AGENTS.md").display().to_string(),
        detail: e.to_string(),
    })
}

/// Which project each window shows (Task-A01-27), by window label: a project already open in
/// another window is brought forward instead of opening twice (owner, 2026-10-12).
#[derive(Default)]
struct Windows {
    projects: Mutex<std::collections::HashMap<String, PathBuf>>,
    /// The number in the next new window's label (`window-<n>`).
    next: std::sync::atomic::AtomicU32,
}

impl Windows {
    fn forget(&self, label: &str) {
        self.projects
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(label);
    }
}

/// A folder as it is on disk, so `/a/./b` and `/a/b` are the same project.
fn same_place(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// The other window that shows `root`, if any.
fn window_showing(
    projects: &std::collections::HashMap<String, PathBuf>,
    root: &Path,
    not: &str,
) -> Option<String> {
    let root = same_place(root);
    projects
        .iter()
        .find(|(label, shown)| label.as_str() != not && **shown == root)
        .map(|(label, _)| label.clone())
}

/// The label of the window in front, else the first one.
fn focused_window(app: &AppHandle) -> Option<String> {
    let windows = app.webview_windows();
    windows
        .iter()
        .find(|(_, w)| w.is_focused().unwrap_or(false))
        .or_else(|| windows.iter().next())
        .map(|(label, _)| label.clone())
}

/// Open another window on the start page. `?new` tells it not to reopen the last folder.
fn open_new_window(app: &AppHandle) -> tauri::Result<()> {
    let n = app
        .state::<Windows>()
        .next
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        + 2;
    tauri::WebviewWindowBuilder::new(
        app,
        format!("window-{n}"),
        tauri::WebviewUrl::App("index.html?new".into()),
    )
    .title("Allomos")
    .inner_size(1100.0, 720.0)
    .min_inner_size(720.0, 480.0)
    .build()
    .map(drop)
}

/// The top bar's **New window…** (Task-A01-27).
#[tauri::command]
fn new_window(app: AppHandle) -> Result<(), Problem> {
    open_new_window(&app).map_err(|e| Problem::OpenFailed {
        path: "New window".into(),
        detail: e.to_string(),
    })
}

/// The calling window now shows `root` (or nothing); its title names the project.
#[tauri::command]
fn set_window_project(
    window: tauri::WebviewWindow,
    windows: tauri::State<Windows>,
    root: Option<PathBuf>,
    name: Option<String>,
) {
    let mut projects = windows.projects.lock().unwrap_or_else(|e| e.into_inner());
    match root {
        Some(root) => {
            projects.insert(window.label().to_string(), same_place(&root));
        }
        None => {
            projects.remove(window.label());
        }
    }
    let title = match name {
        Some(name) if !name.trim().is_empty() => format!("{name} — Allomos"),
        _ => "Allomos".to_string(),
    };
    let _ = window.set_title(&title);
}

/// Another window already shows `root`: bring it forward and say so, so the calling window does
/// not open the project a second time.
#[tauri::command]
fn show_project_elsewhere(
    app: AppHandle,
    window: tauri::WebviewWindow,
    windows: tauri::State<Windows>,
    root: PathBuf,
) -> bool {
    let projects = windows.projects.lock().unwrap_or_else(|e| e.into_inner());
    let Some(label) = window_showing(&projects, &root, window.label()) else {
        return false;
    };
    drop(projects);
    match app.get_webview_window(&label) {
        Some(other) => {
            let _ = other.unminimize();
            let _ = other.show();
            let _ = other.set_focus();
            true
        }
        None => false,
    }
}

/// The menu items' IDs, and the events the interface hears when they are chosen.
/// File → New Window… (⌘N): another window on the start page (Task-A01-27). It replaces
/// File → Open Folder… (⌘O, removed; owner 2026-10-12).
const NEW_WINDOW_ID: &str = "new_window";
/// A recent folder's item is `recent:<position in the list>`.
const RECENT_PREFIX: &str = "recent:";
const OPEN_RECENT_EVENT: &str = "menu:open_recent";
const CLEAR_RECENT_ID: &str = "recent_clear";
const CLEAR_RECENT_EVENT: &str = "menu:clear_recent";
const CONNECT_ID: &str = "connect";
const CONNECT_EVENT: &str = "menu:connect";
const MOVE_TASKS_IN_ID: &str = "move_tasks_in";
const MOVE_TASKS_IN_EVENT: &str = "menu:move_tasks_in";
const CHECK_UPDATES_ID: &str = "check_updates";
/// The website the About window names (Task-A07-22).
const WEBSITE: &str = "https://www.allomos.com";
const CHECK_UPDATES_EVENT: &str = "menu:check_updates";

/// The menu items whose words follow the interface's language, and the Open Recent list.
struct MenuTexts {
    file: Submenu<Wry>,
    new_window: MenuItem<Wry>,
    connect: MenuItem<Wry>,
    move_tasks_in: MenuItem<Wry>,
    check_updates: MenuItem<Wry>,
    /// Check for Updates… has become **Download Allomos <version>…** (Task-A07-16).
    update_known: Mutex<bool>,
    recent: Submenu<Wry>,
    state: Mutex<Recent>,
}

/// What the Open Recent submenu shows: the folders, newest first, and its two fixed texts.
struct Recent {
    folders: Vec<String>,
    clear: String,
    none: String,
}

impl Default for Recent {
    fn default() -> Self {
        Recent {
            folders: Vec::new(),
            clear: "Clear Menu".into(),
            none: "No recent folders".into(),
        }
    }
}

/// Refill the Open Recent submenu: one item per folder, then Clear Menu; or a greyed
/// "No recent folders" when the list is empty.
fn fill_recent(app: &AppHandle, submenu: &Submenu<Wry>, recent: &Recent) -> tauri::Result<()> {
    for item in submenu.items()? {
        submenu.remove(&item)?;
    }
    if recent.folders.is_empty() {
        let none = MenuItem::with_id(app, "recent_none", &recent.none, false, None::<&str>)?;
        return submenu.append(&none);
    }
    for (i, folder) in recent.folders.iter().enumerate() {
        let item = MenuItem::with_id(
            app,
            format!("{RECENT_PREFIX}{i}"),
            folder,
            true,
            None::<&str>,
        )?;
        submenu.append(&item)?;
    }
    submenu.append(&PredefinedMenuItem::separator(app)?)?;
    submenu.append(&MenuItem::with_id(
        app,
        CLEAR_RECENT_ID,
        &recent.clear,
        true,
        None::<&str>,
    )?)
}

/// The system's usual menus, with **Open Folder…** (⌘O / Ctrl+O) at the top of the File menu.
/// Words start in English; the interface sends its own with `set_menu_texts`.
fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let menu = Menu::default(app)?;
    let new_window =
        MenuItem::with_id(app, NEW_WINDOW_ID, "New Window…", true, Some("CmdOrCtrl+N"))?;
    let recent = Submenu::new(app, "Open Recent", true)?;
    // One item for telling agents about Allomos and moving existing tasks in (owner, 2026-10-11).
    let connect_item = MenuItem::with_id(
        app,
        CONNECT_ID,
        "Connect with the Agents…",
        // Enabled by the window while a taskbag is open.
        false,
        None::<&str>,
    )?;
    // An older taskbag's tasks and card files kept in the project folder are moved in by an
    // agent: one item for both, Update the Taskbag… (Task-A01-45, Task-A00-30, A01-51).
    let move_tasks_in = MenuItem::with_id(
        app,
        MOVE_TASKS_IN_ID,
        "Update the Taskbag…",
        // Enabled by the window while a taskbag is open.
        false,
        None::<&str>,
    )?;
    fill_recent(app, &recent, &Recent::default())?;
    let existing = menu
        .items()?
        .into_iter()
        .filter_map(|item| item.as_submenu().cloned())
        .find(|s| s.text().is_ok_and(|t| t == "File"));
    let file = match existing {
        Some(file) => {
            file.prepend_items(&[&new_window, &recent, &connect_item, &move_tasks_in])?;
            file.insert(&PredefinedMenuItem::separator(app)?, 4)?;
            file
        }
        None => {
            let file = Submenu::with_items(
                app,
                "File",
                true,
                &[&new_window, &recent, &connect_item, &move_tasks_in],
            )?;
            // After the app menu on macOS; first elsewhere.
            menu.insert(&file, usize::from(cfg!(target_os = "macos")))?;
            file
        }
    };
    // Check for Updates…, where the Claude app has it: under About in the app menu on macOS,
    // in the Help menu elsewhere (approver, 2026-10-15).
    let check_updates = MenuItem::with_id(
        app,
        CHECK_UPDATES_ID,
        "Check for Updates…",
        true,
        None::<&str>,
    )?;
    let submenus: Vec<Submenu<Wry>> = menu
        .items()?
        .into_iter()
        .filter_map(|item| item.as_submenu().cloned())
        .collect();
    // The default About item, first in the app menu on macOS and in the Help menu elsewhere,
    // is replaced by one that also names the website (Task-A07-22).
    let about = about_item(app)?;
    if cfg!(target_os = "macos") {
        if let Some(app_menu) = submenus.first() {
            app_menu.remove_at(0)?;
            app_menu.insert(&about, 0)?;
            app_menu.insert(&check_updates, 1)?;
        }
    } else if let Some(help) = submenus
        .iter()
        .find(|s| s.text().is_ok_and(|t| t == "Help"))
    {
        help.remove_at(0)?;
        help.insert(&about, 0)?;
        help.append(&check_updates)?;
    } else {
        menu.append(&Submenu::with_items(app, "Help", true, &[&check_updates])?)?;
    }
    app.manage(MenuTexts {
        file,
        new_window,
        connect: connect_item,
        move_tasks_in,
        check_updates,
        update_known: Mutex::new(false),
        recent,
        state: Mutex::new(Recent::default()),
    });
    Ok(menu)
}

/// About Allomos, as the default menu has it, with the website added: macOS shows it under the
/// version (its About window has no website field), Windows and Linux in their website field.
fn about_item(app: &AppHandle) -> tauri::Result<PredefinedMenuItem<Wry>> {
    let info = app.package_info();
    let config = app.config();
    let metadata = AboutMetadata {
        name: Some(info.name.clone()),
        version: Some(info.version.to_string()),
        copyright: config.bundle.copyright.clone(),
        authors: config.bundle.publisher.clone().map(|p| vec![p]),
        website: Some(WEBSITE.to_string()),
        website_label: Some("www.allomos.com".to_string()),
        credits: Some("www.allomos.com".to_string()),
        ..Default::default()
    };
    PredefinedMenuItem::about(app, None, Some(metadata))
}

/// The menu's words in the interface's language, as the interface sends them.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MenuWords {
    file: String,
    new_window: String,
    open_recent: String,
    clear_menu: String,
    no_recent: String,
    connect: String,
    move_tasks_in: String,
}

/// Put the menu's words in the interface's language.
#[tauri::command]
fn set_menu_texts(app: AppHandle, texts: tauri::State<MenuTexts>, words: MenuWords) {
    // A menu that cannot be renamed keeps its previous words; nothing else depends on them.
    let _ = texts.file.set_text(words.file);
    let _ = texts.new_window.set_text(words.new_window);
    let _ = texts.recent.set_text(words.open_recent);
    let _ = texts.connect.set_text(words.connect);
    let _ = texts.move_tasks_in.set_text(words.move_tasks_in);
    let mut recent = texts.state.lock().unwrap_or_else(|e| e.into_inner());
    recent.clear = words.clear_menu;
    recent.none = words.no_recent;
    let _ = fill_recent(&app, &texts.recent, &recent);
}

/// The update item's words, as the window sends them: **Check for Updates…**, or **Download
/// Allomos <version>…** once a newer version is known, which then opens the download page, as
/// the Claude app's **Restart to Update** item does (approver, 2026-10-15).
#[tauri::command]
fn set_update_item(texts: tauri::State<MenuTexts>, text: String, known: bool) {
    let _ = texts.check_updates.set_text(text);
    *texts.update_known.lock().unwrap_or_else(|e| e.into_inner()) = known;
}

/// Show these folders, newest first, under File → Open Recent.
#[tauri::command]
fn set_recent_folders(app: AppHandle, texts: tauri::State<MenuTexts>, folders: Vec<String>) {
    let mut recent = texts.state.lock().unwrap_or_else(|e| e.into_inner());
    recent.folders = folders;
    let _ = fill_recent(&app, &texts.recent, &recent);
}

/// The folder behind a chosen Open Recent item, if `id` is one.
fn recent_folder(texts: &MenuTexts, id: &str) -> Option<String> {
    let i: usize = id.strip_prefix(RECENT_PREFIX)?.parse().ok()?;
    let recent = texts.state.lock().unwrap_or_else(|e| e.into_inner());
    recent.folders.get(i).cloned()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .menu(build_menu)
        .manage(Windows::default())
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            // Menu commands act on the window in front, not on every window (Task-A01-27).
            let target = focused_window(app);
            let send = |name: &str, payload: Option<String>| {
                if let Some(label) = &target {
                    let _ = match payload {
                        Some(p) => app.emit_to(label.as_str(), name, p),
                        None => app.emit_to(label.as_str(), name, ()),
                    };
                }
            };
            if id == NEW_WINDOW_ID {
                let _ = open_new_window(app);
            } else if id == CONNECT_ID {
                send(CONNECT_EVENT, None);
            } else if id == MOVE_TASKS_IN_ID {
                send(MOVE_TASKS_IN_EVENT, None);
            } else if id == CHECK_UPDATES_ID {
                let known = *app
                    .state::<MenuTexts>()
                    .update_known
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                if known {
                    let _ = open_download_page();
                } else {
                    send(CHECK_UPDATES_EVENT, None);
                }
            } else if id == CLEAR_RECENT_ID {
                send(CLEAR_RECENT_EVENT, None);
            } else if let Some(folder) = recent_folder(&app.state::<MenuTexts>(), id) {
                // In the current window (owner, 2026-10-12).
                send(OPEN_RECENT_EVENT, Some(folder));
            }
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                window.state::<Windows>().forget(window.label());
            }
        })
        .invoke_handler(tauri::generate_handler![
            startup_folder,
            new_window,
            set_window_project,
            show_project_elsewhere,
            inspect_folder,
            open_taskbag,
            taskbag_stamp,
            version_check,
            move_taskbag_aside,
            taskbag_format,
            open_download_page,
            update_status,
            skip_update,
            set_update_item,
            handoff_settings,
            set_handoff_rules,
            set_handoff_template,
            list_handoffs,
            write_handoff,
            create_taskbag,
            task_history,
            show_in_file_manager,
            open_task_link,
            system_languages,
            save_ui_language,
            owner_name,
            save_owner_name,
            edit_task,
            review_ai_decisions,
            next_ids,
            add_area,
            add_iteration,
            add_iteration_and_plan,
            add_task,
            rename_area,
            change_iteration,
            delete_area,
            delete_iteration,
            set_menu_texts,
            set_recent_folders,
            find_agents,
            connect_agents,
            manual_request,
            build_app,
            build_app_request,
            move_tasks_in,
            move_tasks_in_request,
            run_modes_text,
            copy_text,
            instruction_files,
            agents_set_up,
            write_agent_files,
            agent_status,
            confirm_requests,
            answer_confirm_request,
            progress_reports,
            dismiss_progress,
            project_setup,
            set_project_folder,
            set_connect_enabled,
            alm_command_status,
            install_alm_command,
            dismiss_alm_offer
        ])
        .run(tauri::generate_context!())
        .expect("error while running Allomos");
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn an_out_of_date_project_gets_the_update_request() {
        let dir =
            std::env::temp_dir().join(format!("allomos-desktop-update-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let taskbag = dir.join(allomos_store::DEFAULT_DIR);
        allomos_store::init(&taskbag, "p", "P").unwrap();
        // A section from before versions: connected, but out of date.
        std::fs::write(dir.join("AGENTS.md"), "## Tasks: Allomos\n\nalm next\n").unwrap();
        let plan = connect_plan(&["claude_app".into()], "en", &dir, &taskbag);
        assert!(
            plan[0]
                .1
                .starts_with("This project was set up by an older Allomos")
        );
        assert!(plan[0].1.contains("brief`"));
        assert!(version_check(taskbag.clone()).unwrap().needs_update());

        // Renewed: the usual connect request again.
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

        // Not renamed yet: the agent is asked to do it after the Go ahead.
        let check = version_check(taskbag.clone()).unwrap();
        assert!(!check.move_in.as_ref().unwrap().prepared);
        let plan = connect_plan(&["claude_app".into()], "en", &dir, &taskbag);
        assert!(plan[0].1.starts_with("Please move the tasks of a taskbag"));
        assert!(plan[0].1.contains(" init --id "));
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
        assert!(
            plan[0]
                .1
                .contains("Allomos renamed the old taskbag (format allomos/2;")
        );
        assert!(!plan[0].1.contains(" init --id "));
        assert!(version_check(taskbag.clone()).unwrap().needs_update());
        // Update the taskbag… asks for the move-in, reported as that job (Task-A01-45); once the
        // tasks are in, it is about card files (below).
        assert_eq!(tasks_in_job(&taskbag), Job::MoveIn);
        assert!(
            move_tasks_in_request(taskbag.clone(), "en".into(), Vec::new())
                .contains("Allomos renamed the old taskbag (format allomos/2;")
        );
        // An agent not yet set up (Task-A03-24): the same request also asks for its setup.
        let both = move_tasks_in_request(taskbag.clone(), "en".into(), vec!["Codex".into()]);
        assert!(
            both.contains("Allomos renamed the old taskbag")
                && both.contains("agent setup request")
        );
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
        // Out of date: the update request, as automatic sending.
        std::fs::write(dir.join("AGENTS.md"), "## Tasks: Allomos\n\nalm next\n").unwrap();
        let auto = connect_plan(&["codex_cli".into()], "en", &dir, &taskbag);
        let manual = manual_plan("en", &dir, &taskbag);
        assert_eq!(manual, auto[0].1);
        assert!(manual.starts_with("This project was set up by an older Allomos"));
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
                let auto =
                    build_app_plan("claude_app", language, &dir, &taskbag, description, quiet);
                // One request: the wizard's connect request first, then the goal request.
                let connect = agent_connect_request_in(language, &dir, &taskbag);
                assert!(auto.starts_with(connect.trim_end()), "{language}");
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
                    build_app_plan(MANUAL, language, &dir, &taskbag, description, quiet),
                    auto
                );
                assert_eq!(
                    build_app_plan("codex_cli", language, &dir, &taskbag, description, quiet),
                    auto
                );
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
    fn a_project_shown_in_another_window_is_found() {
        let dir = std::env::temp_dir().join(format!("allomos-windows-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("a")).unwrap();
        let mut projects = std::collections::HashMap::new();
        projects.insert("main".to_string(), same_place(&dir.join("a")));
        // The same folder written another way is the same project.
        let other = dir.join("a").join(".").join("..").join("a");
        assert_eq!(
            window_showing(&projects, &other, "window-2"),
            Some("main".to_string())
        );
        // A window does not count itself, and another folder is not shown anywhere.
        assert_eq!(window_showing(&projects, &dir.join("a"), "main"), None);
        assert_eq!(window_showing(&projects, &dir.join("b"), "window-2"), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_first_agent_also_looks_for_tasks() {
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

        // Two: the first chosen gets the connect request, the other only the instructions; both
        // are told about each other.
        let two = connect_plan(
            &["claude_app".into(), "codex_cli".into()],
            "en",
            &project,
            &taskbag,
        );
        assert_eq!(two.len(), 2);
        assert_eq!(two[0].0, "claude_app");
        assert!(two[0].1.ends_with(&connect) && two[0].1.starts_with("Note: "));
        assert_eq!(two[1].0, "codex_cli");
        assert!(two[1].1.ends_with(&setup) && two[1].1.starts_with("Note: "));
        assert!(!two[1].1.contains("alm confirm"));
        assert!(connect_plan(&[], "en", &project, &taskbag).is_empty());
    }

    /// Decision 0017: the wizard's steps 2 and 3 run no Git. A folder outside Git is set up
    /// without errors and stays outside Git; the agent connected in step 4 is asked to set Git up
    /// with the person and commit. "Last used by" and the changes not shared yet come from the
    /// change history.
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
        // No agent has used it yet; the app's own changes (create, project folder) are not
        // shared yet.
        assert_eq!(allomos_store::events::last_agent_use(&root), None);
        assert_eq!(allomos_store::events::unshared_app_changes(&root).count, 2);
        std::fs::remove_dir_all(&base).unwrap();
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
        assert!(load(&root).taskbag.unwrap().tasks[0].label.is_none());

        let task = edit_task_as(&root, Some("Will"), "A01-01", &edit).unwrap();
        assert_eq!(task.label.as_deref(), Some("implemented"));
        let h = allomos_store::events::history(&root, "A01-01");
        let e = &h.events[0];
        assert_eq!(
            (e.by.as_deref(), e.via.as_str(), e.command.as_str()),
            (Some("Will"), "app", "label")
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
        )
        .unwrap();
        let opened = open_taskbag(root.clone());
        assert_eq!(opened.ai_decisions["A01-01"][0].text, "Names: alm note");
        assert_eq!(
            review_as(&root, None, "A01-01"),
            Err(Problem::HolderMissing)
        );
        review_as(&root, Some("Will"), "A01-01").unwrap();
        assert!(open_taskbag(root.clone()).ai_decisions.is_empty());
        let e = &allomos_store::events::history(&root, "A01-01").events[0];
        assert_eq!(
            (e.by.as_deref(), e.via.as_str(), e.command.as_str()),
            (Some("Will"), "app", "reviewed")
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
            (task.state, task.holder, task.label),
            (allomos_core::State::Ready, None, None)
        );
        let e = &allomos_store::events::history(&root, "A01-01").events[0];
        assert_eq!(
            (e.by.as_deref(), e.via.as_str(), e.command.as_str()),
            (Some("Will"), "app", "reopen")
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
        assert_eq!((ids.area.as_str(), ids.iteration.as_str()), ("A01", "I01"));
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
        assert_eq!(ids.iteration, "I08");
        assert_eq!(ids.tasks.get("A01").map(String::as_str), Some("A01-01"));

        // New task in an iteration group, as `messenger.addTask` sends it: added, then planned.
        let task: allomos_store::NewTask = serde_json::from_value(serde_json::json!({
            "id": "", "area": "A01", "title": "Plan the week", "description": "Why.", "iteration": "I07"
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
        assert_eq!(
            seen,
            [
                ("plan", Some("Will"), "app"),
                ("add task", Some("Will"), "app")
            ]
        );
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
        assert_eq!(t.iteration.as_deref(), Some("I08"));
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
                Some("Will".to_string()),
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
