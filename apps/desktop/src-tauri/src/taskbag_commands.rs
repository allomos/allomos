//! Opening a taskbag in a window: looking at a folder, opening, creating and re-reading a
//! taskbag, its version and format, a task's history, and showing it or a task's link outside
//! the app.

use std::path::{Path, PathBuf};
use std::process::Command;

use allomos_core::{Loaded, Problem};
use allomos_store::{
    Inspection, LinkTarget, add_readme_if_empty, id_from_name, init, inspect, load, picked_folder,
    resolve_task_link, stamp, versions,
};
use serde::Serialize;

use crate::{agents, opener, sessions};

#[derive(Serialize)]
pub(crate) struct FolderInfo {
    inspection: Inspection,
}

#[derive(Serialize)]
pub(crate) struct Opened {
    pub(crate) root: PathBuf,
    pub(crate) loaded: Loaded,
    /// A `README-allomos.md` was just written next to `.allomos/`.
    pub(crate) readme_added: bool,
    /// Each task's AI decisions not reviewed yet, by task ID (Task-A03-19), from the history.
    pub(crate) ai_decisions:
        std::collections::BTreeMap<String, Vec<allomos_core::decision::Decision>>,
    /// Each task's approvals, by task ID, with where each stands now (Task-A01-coach782), from
    /// the history.
    pub(crate) approvals: std::collections::BTreeMap<String, Vec<allomos_core::approval::Recorded>>,
    /// Each task's change type, risk and what it touches as an agent recorded them, by task ID
    /// (Task-A01-garden385, decision 0027), from the history; a task with none is left out.
    pub(crate) classifications:
        std::collections::BTreeMap<String, allomos_core::classify::Classification>,
    /// Each task's outside effects, by task ID (Task-A01-job650), from the history.
    pub(crate) effects: std::collections::BTreeMap<String, Vec<allomos_core::effect::Recorded>>,
    /// Each task's links to code and releases, by task ID (Task-A01-mint242), from the history.
    pub(crate) links: std::collections::BTreeMap<String, Vec<allomos_core::link::Recorded>>,
    /// Each task's base and working branch, by task ID (Task-A03-22), from the history: the New
    /// task form chooses the new task's base from the open tasks' bases.
    pub(crate) branches: std::collections::BTreeMap<String, allomos_core::branch::Branches>,
    /// When each task last changed, by task ID (Task-A01-52), from the history: the Iterations
    /// tab's sort menu orders by it.
    pub(crate) last_changed: std::collections::BTreeMap<String, String>,
    /// The card files agents reported as still in the project folder (Task-A00-30): the notice
    /// "N tasks keep their card files in the project folder" with **Update the taskbag…**.
    pub(crate) cards_outside: Vec<allomos_core::card::Outside>,
    /// An older taskbag's handoff files in `handoffs/` (Task-A03-polar770): the taskbag needs
    /// updating, with **Update the taskbag…**, until they are moved into their tasks.
    pub(crate) old_handoffs: Vec<String>,
    /// Each task's version, by task ID (Task-A01-57): the task pane sends back the version it
    /// showed when it saves a title or description, so a newer change is never overwritten.
    pub(crate) versions: std::collections::BTreeMap<String, String>,
}

/// What is this folder? Changes nothing.
#[tauri::command(async)]
pub(crate) fn inspect_folder(path: PathBuf) -> FolderInfo {
    FolderInfo {
        inspection: inspect(&path),
    }
}

/// Load the taskbag at `root`.
#[tauri::command(async)]
pub(crate) fn open_taskbag(root: PathBuf) -> Opened {
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
pub(crate) fn taskbag_stamp(root: PathBuf) -> String {
    stamp(&root).to_string()
}

/// Create an empty taskbag at `root` (normally `<folder>/.allomos/taskbag/`), then load it.
/// Empty, with no example tasks, so the open-folder flow can tell whether the project has tasks
/// yet (docs/product/OPEN-FOLDER-FLOW.md).
/// When the picked folder looks empty, also write `README-allomos.md` there, in the
/// interface's `language`; a README that cannot be written does not undo the taskbag.
/// (Connecting AI agents with the project is a separate step: `connect_agents`.)
#[tauri::command(async)]
pub(crate) fn create_taskbag(
    root: PathBuf,
    name: String,
    language: String,
) -> Result<Opened, Problem> {
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

/// Open a link from a task's description: a web page in the browser, or a text file inside the
/// project (such as the task's card) in the user's editor. Anything else is refused by
/// `resolve_task_link` (Task-A01-16).
#[tauri::command(async)]
pub(crate) fn open_task_link(root: PathBuf, task: String, link: String) -> Result<(), Problem> {
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
pub(crate) fn open_agent_session(root: PathBuf, task: String) -> Result<(), Problem> {
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
pub(crate) fn show_in_file_manager(root: PathBuf) -> Result<(), Problem> {
    let mut command = file_manager_command(&root)?;
    // Not waited for: Explorer reports failure even when it opened the folder.
    command.spawn().map(drop).map_err(|e| Problem::OpenFailed {
        path: root.display().to_string(),
        detail: e.to_string(),
    })
}

/// Is the project up to date with this Allomos (Task-A01-25)? `None` until an agent is connected
/// with it. The window asks when a folder is opened and every few seconds while it is open, so
/// an agent's update shows without reopening.
#[tauri::command(async)]
pub(crate) fn version_check(root: PathBuf) -> Option<versions::VersionCheck> {
    versions::check_connected(&root)
}

/// **Update the taskbag…** on an older Allomos's taskbag (Task-A01-45): rename it aside
/// (`taskbag-allomos-<n>`) and create a new, empty taskbag in its place, then load that; the same
/// dialog then asks the chosen agent to move the tasks across ([`move_tasks_in`]). Nothing in the
/// old folder is changed.
#[tauri::command(async)]
pub(crate) fn move_taskbag_aside(root: PathBuf) -> Result<Opened, Problem> {
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
pub(crate) fn task_history(root: PathBuf, id: String) -> allomos_store::events::History {
    allomos_store::events::history(&root, &id)
}

/// The taskbag's format compared with this Allomos's, whether or not an agent is connected: the
/// wizard's step 2 does not go on with a newer taskbag (owner, 2026-10-12).
#[derive(Serialize)]
pub(crate) struct TaskbagFormat {
    format: versions::FormatState,
    /// What this Allomos writes, for the message: `allomos/1`.
    current: &'static str,
}

#[tauri::command(async)]
pub(crate) fn taskbag_format(root: PathBuf) -> TaskbagFormat {
    TaskbagFormat {
        format: versions::format_state(&root),
        current: allomos_core::FORMAT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
