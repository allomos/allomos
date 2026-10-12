//! Allomos desktop app: the Tauri window plus the commands the interface may call.
//!
//! The interface never calls these directly; it goes through its messenger module
//! (`apps/web/src/messenger.ts`), so the same interface can later reach the desktop's
//! local server from a browser without changes.

mod agents;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use allomos_core::{Loaded, Problem};
use allomos_store::{
    AgentSetup, ConfirmRequest, Inspection, LinkTarget, SetupState, add_agent_instructions,
    add_readme_if_empty, agent_connect_request_in, agent_setup, agent_setup_request_in,
    answer_confirm, git_root, id_from_name, init, inspect, load, pending_confirms, picked_folder,
    project_folder_of, record_project_folder, resolve_task_link, setup_state, stamp, versions,
};
use serde::Serialize;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Manager, Wry};

#[derive(Serialize)]
struct FolderInfo {
    inspection: Inspection,
    /// The Git repository the folder belongs to, if any (information only).
    git_root: Option<PathBuf>,
}

#[derive(Serialize)]
struct Opened {
    root: PathBuf,
    loaded: Loaded,
    /// A `README-allomos.md` was just written next to `.allomos/`.
    readme_added: bool,
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
        git_root: git_root(&path),
        inspection: inspect(&path),
    }
}

/// Load the taskbag at `root`.
#[tauri::command]
fn open_taskbag(root: PathBuf) -> Opened {
    Opened {
        loaded: load(&root),
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
    command.arg(root);
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
    let note = agents::shared_note(agents, language);
    // Something is out of date (Task-A01-25): the first agent is asked to update it instead.
    let update = versions::check_connected(taskbag).filter(|c| c.needs_update());
    agents
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let request = if let (0, Some(check)) = (i, &update) {
                versions::agent_update_request_in(language, project, taskbag, check)
            } else if i == 0 {
                agent_connect_request_in(language, project, taskbag)
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

/// Is the project up to date with this Allomos (Task-A01-25)? `None` until an agent is connected
/// with it. The window asks when a folder is opened and every few seconds while it is open, so
/// an agent's update shows without reopening.
#[tauri::command]
fn version_check(root: PathBuf) -> Option<versions::VersionCheck> {
    versions::check_connected(&root)
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

/// Where the setup wizard keeps the taskbag in Git (Task-A05-03).
#[derive(Debug, Serialize, PartialEq, Eq)]
struct GitKept {
    /// The repository that holds the taskbag.
    repo: PathBuf,
    /// The folder was not in a repository, so the wizard made it one (`git init`).
    initialized: bool,
    /// A commit was made; false when the taskbag was already committed as it is.
    committed: bool,
}

/// The commit message for the taskbag's first commit, the same as the agents' setup request uses;
/// later commits by the wizard (the project folder, step 3) say "Update".
const TASKBAG_COMMIT: &str = "Add Allomos taskbag";
const TASKBAG_UPDATE: &str = "Update Allomos taskbag";

/// Run `git -C dir …` with extra environment (tests set an author): its output, or Git's own
/// message on failure.
fn run_git(dir: &Path, args: &[&str], envs: &[(&str, &str)]) -> Result<String, String> {
    let out = agents::command("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .envs(envs.iter().copied())
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).into_owned());
    }
    let text = String::from_utf8_lossy(&out.stderr).trim().to_string();
    Err(if text.is_empty() {
        format!("git {} failed", args.join(" "))
    } else {
        text
    })
}

/// Whether a `git --version` run shows that Git is missing: the program is not found, or it is the
/// macOS stand-in that only offers to install the developer tools.
fn git_missing(started: bool, succeeded: bool, stderr: &str) -> bool {
    let stub = stderr.contains("xcrun: error") || stderr.contains("developer tools");
    !started || stub || !succeeded
}

/// Is there a working `git` program?
fn git_works(envs: &[(&str, &str)]) -> bool {
    match agents::command("git")
        .arg("--version")
        .envs(envs.iter().copied())
        .output()
    {
        Ok(out) => !git_missing(
            true,
            out.status.success(),
            &String::from_utf8_lossy(&out.stderr),
        ),
        Err(_) => !git_missing(false, false, ""),
    }
}

/// After the wizard's step 2 (owner, 2026-10-12): make sure the folder holding `.allomos/` is in a
/// Git repository, making it one with `git init` when it is not, without asking; then commit the
/// taskbag (and `README-allomos.md`, when Allomos left one) to that repository, and nothing else.
/// Never pushes. The person's own Git settings and hooks apply (decision 0010).
fn keep_taskbag_in_git(root: &Path, envs: &[(&str, &str)]) -> Result<GitKept, Problem> {
    if !git_works(envs) {
        return Err(Problem::GitMissing);
    }
    let folder = std::path::absolute(picked_folder(root).unwrap_or(root))
        .unwrap_or_else(|_| picked_folder(root).unwrap_or(root).to_path_buf());
    let (repo, initialized) = match git_root(&folder) {
        Some(repo) => (repo, false),
        None => {
            run_git(&folder, &["init", "--quiet"], envs).map_err(|detail| {
                Problem::GitInitFailed {
                    path: folder.display().to_string(),
                    detail,
                }
            })?;
            (folder.clone(), true)
        }
    };
    let root = std::path::absolute(root).unwrap_or_else(|_| root.to_path_buf());
    let mut paths = vec![root.display().to_string()];
    let readme = folder.join(allomos_store::README_FILE);
    if readme.is_file() {
        paths.push(readme.display().to_string());
    }
    let with_paths = |head: &[&str]| -> Vec<String> {
        head.iter()
            .map(|s| s.to_string())
            .chain(std::iter::once("--".to_string()))
            .chain(paths.iter().cloned())
            .collect()
    };
    let git = |args: Vec<String>| {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        run_git(&repo, &args, envs)
    };
    let commit = (|| {
        git(with_paths(&["add", "--all"]))?;
        // Nothing new to commit: the taskbag is already in the repository as it is.
        if git(with_paths(&["diff", "--cached", "--quiet"])).is_ok() {
            return Ok(false);
        }
        // Already in the history (an earlier step committed it): this one updates it.
        let before = git(vec![
            "log".into(),
            "-1".into(),
            "--format=%H".into(),
            "--".into(),
            paths[0].clone(),
        ]);
        let known = before.as_ref().is_ok_and(|out| !out.trim().is_empty());
        let message = if known {
            TASKBAG_UPDATE
        } else {
            TASKBAG_COMMIT
        };
        git(with_paths(&["commit", "--quiet", "-m", message]))?;
        Ok(true)
    })();
    match commit {
        Ok(committed) => Ok(GitKept {
            repo,
            initialized,
            committed,
        }),
        Err(detail) => Err(Problem::CommitFailed { detail }),
    }
}

/// One commit at a time: two `git add`/`git commit` runs at once fail on Git's `index.lock`. The
/// wizard can ask twice for one opening (React runs a starting effect twice in development, and a
/// quick second opening does too); the second then waits and finds nothing new to commit.
static KEEP_IN_GIT: Mutex<()> = Mutex::new(());

fn keep_one_at_a_time(root: &Path, envs: &[(&str, &str)]) -> Result<GitKept, Problem> {
    let _one = KEEP_IN_GIT.lock().unwrap_or_else(|e| e.into_inner());
    keep_taskbag_in_git(root, envs)
}

#[tauri::command]
async fn keep_in_git(root: PathBuf) -> Result<GitKept, Problem> {
    keep_one_at_a_time(&root, &[])
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

/// File → Connect with the Agents… is offered only while a taskbag is open.
#[tauri::command]
fn set_connect_enabled(texts: tauri::State<MenuTexts>, enabled: bool) {
    let _ = texts.connect.set_enabled(enabled);
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
    /// The newest taskbag change an agent made with `alm --commit` / `--push`.
    last: Option<agents::LastUse>,
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
        last: agents::last_agent_use(&root),
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

/// The menu items' IDs, and the events the interface hears when they are chosen.
const OPEN_FOLDER_ID: &str = "open_folder";
const OPEN_FOLDER_EVENT: &str = "menu:open_folder";
/// A recent folder's item is `recent:<position in the list>`.
const RECENT_PREFIX: &str = "recent:";
const OPEN_RECENT_EVENT: &str = "menu:open_recent";
const CLEAR_RECENT_ID: &str = "recent_clear";
const CLEAR_RECENT_EVENT: &str = "menu:clear_recent";
const CONNECT_ID: &str = "connect";
const CONNECT_EVENT: &str = "menu:connect";

/// The menu items whose words follow the interface's language, and the Open Recent list.
struct MenuTexts {
    file: Submenu<Wry>,
    open_folder: MenuItem<Wry>,
    connect: MenuItem<Wry>,
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
    let open_folder = MenuItem::with_id(
        app,
        OPEN_FOLDER_ID,
        "Open Folder…",
        true,
        Some("CmdOrCtrl+O"),
    )?;
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
    fill_recent(app, &recent, &Recent::default())?;
    let existing = menu
        .items()?
        .into_iter()
        .filter_map(|item| item.as_submenu().cloned())
        .find(|s| s.text().is_ok_and(|t| t == "File"));
    let file = match existing {
        Some(file) => {
            file.prepend_items(&[&open_folder, &recent, &connect_item])?;
            file.insert(&PredefinedMenuItem::separator(app)?, 3)?;
            file
        }
        None => {
            let file =
                Submenu::with_items(app, "File", true, &[&open_folder, &recent, &connect_item])?;
            // After the app menu on macOS; first elsewhere.
            menu.insert(&file, usize::from(cfg!(target_os = "macos")))?;
            file
        }
    };
    app.manage(MenuTexts {
        file,
        open_folder,
        connect: connect_item,
        recent,
        state: Mutex::new(Recent::default()),
    });
    Ok(menu)
}

/// The menu's words in the interface's language, as the interface sends them.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MenuWords {
    file: String,
    open_folder: String,
    open_recent: String,
    clear_menu: String,
    no_recent: String,
    connect: String,
}

/// Put the menu's words in the interface's language.
#[tauri::command]
fn set_menu_texts(app: AppHandle, texts: tauri::State<MenuTexts>, words: MenuWords) {
    // A menu that cannot be renamed keeps its previous words; nothing else depends on them.
    let _ = texts.file.set_text(words.file);
    let _ = texts.open_folder.set_text(words.open_folder);
    let _ = texts.recent.set_text(words.open_recent);
    let _ = texts.connect.set_text(words.connect);
    let mut recent = texts.state.lock().unwrap_or_else(|e| e.into_inner());
    recent.clear = words.clear_menu;
    recent.none = words.no_recent;
    let _ = fill_recent(&app, &texts.recent, &recent);
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
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            if id == OPEN_FOLDER_ID {
                let _ = app.emit(OPEN_FOLDER_EVENT, ());
            } else if id == CONNECT_ID {
                let _ = app.emit(CONNECT_EVENT, ());
            } else if id == CLEAR_RECENT_ID {
                let _ = app.emit(CLEAR_RECENT_EVENT, ());
            } else if let Some(folder) = recent_folder(&app.state::<MenuTexts>(), id) {
                let _ = app.emit(OPEN_RECENT_EVENT, folder);
            }
        })
        .invoke_handler(tauri::generate_handler![
            startup_folder,
            inspect_folder,
            open_taskbag,
            taskbag_stamp,
            version_check,
            create_taskbag,
            show_in_file_manager,
            open_task_link,
            system_languages,
            save_ui_language,
            set_menu_texts,
            set_recent_folders,
            find_agents,
            connect_agents,
            instruction_files,
            keep_in_git,
            write_agent_files,
            agent_status,
            confirm_requests,
            answer_confirm_request,
            project_setup,
            set_project_folder,
            set_connect_enabled
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
        let root = dir.join(allomos_store::DEFAULT_DIR);
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
        assert_eq!(command.get_args().collect::<Vec<_>>(), [root.as_os_str()]);
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
        assert_eq!(plan[0].1, agent_connect_request_in("en", &dir, &taskbag));
        assert!(!version_check(taskbag).unwrap().needs_update());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_first_agent_also_looks_for_tasks() {
        let project = PathBuf::from("/work/bakery");
        let taskbag = project.join(allomos_store::DEFAULT_DIR);
        let connect = agent_connect_request_in("en", &project, &taskbag);
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

    #[test]
    fn the_taskbag_is_put_under_git_and_committed_alone() {
        let me = [
            ("GIT_AUTHOR_NAME", "Test"),
            ("GIT_AUTHOR_EMAIL", "test@example.com"),
            ("GIT_COMMITTER_NAME", "Test"),
            ("GIT_COMMITTER_EMAIL", "test@example.com"),
        ];
        let base = std::env::temp_dir().join(format!("allomos-keep-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let log = |repo: &Path| {
            let out = agents::command("git")
                .arg("-C")
                .arg(repo)
                .args(["log", "--format=%s", "--name-only"])
                .output()
                .unwrap();
            String::from_utf8_lossy(&out.stdout).into_owned()
        };

        // A folder outside Git: made a repository, and only the taskbag is committed.
        let plain = base.join("plain");
        std::fs::create_dir_all(&plain).unwrap();
        std::fs::write(plain.join("notes.txt"), "mine").unwrap();
        let root = plain.join(allomos_store::DEFAULT_DIR);
        allomos_store::init(&root, "p", "P").unwrap();
        let kept = keep_taskbag_in_git(&root, &me).unwrap();
        assert!(kept.initialized && kept.committed);
        assert_eq!(kept.repo, plain);
        let history = log(&plain);
        assert!(history.starts_with(TASKBAG_COMMIT), "{history}");
        assert!(
            history.contains(".allomos/taskbag/project.json"),
            "{history}"
        );
        assert!(!history.contains("notes.txt"), "{history}");
        // Again: nothing new, no second commit.
        let again = keep_taskbag_in_git(&root, &me).unwrap();
        assert!(!again.initialized && !again.committed);
        // Step 3 records the project folder: committed before the agent is connected, as an update.
        allomos_store::record_project_folder(&root, &plain).unwrap();
        assert!(keep_taskbag_in_git(&root, &me).unwrap().committed);
        assert!(log(&plain).starts_with(TASKBAG_UPDATE), "{}", log(&plain));

        // Inside an existing repository: no new repository; the person's staged work stays staged.
        let repo = base.join("repo");
        let sub = repo.join("app");
        std::fs::create_dir_all(&sub).unwrap();
        run_git(&repo, &["init", "--quiet"], &me).unwrap();
        std::fs::write(repo.join("work.txt"), "staged").unwrap();
        run_git(&repo, &["add", "work.txt"], &me).unwrap();
        let root = sub.join(allomos_store::DEFAULT_DIR);
        allomos_store::init(&root, "q", "Q").unwrap();
        let kept = keep_taskbag_in_git(&root, &me).unwrap();
        assert_eq!(kept.repo, repo);
        assert!(!kept.initialized && kept.committed);
        assert!(!sub.join(".git").exists(), "no nested repository");
        assert!(!log(&repo).contains("work.txt"));
        assert!(
            run_git(
                &repo,
                &["diff", "--cached", "--quiet", "--", "work.txt"],
                &me
            )
            .is_err()
        );

        // Two openings at once (React's double start in development): one commits, the other
        // waits and finds nothing new, instead of failing on Git's index.lock.
        let twice = base.join("twice");
        std::fs::create_dir_all(&twice).unwrap();
        run_git(&twice, &["init", "--quiet"], &me).unwrap();
        let root = twice.join(allomos_store::DEFAULT_DIR);
        allomos_store::init(&root, "t", "T").unwrap();
        let results: Vec<_> = std::thread::scope(|s| {
            let a = s.spawn(|| keep_one_at_a_time(&root, &me));
            let b = s.spawn(|| keep_one_at_a_time(&root, &me));
            vec![a.join().unwrap(), b.join().unwrap()]
        });
        let committed: Vec<bool> = results.into_iter().map(|r| r.unwrap().committed).collect();
        assert_eq!(committed.iter().filter(|c| **c).count(), 1, "{committed:?}");
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn missing_git_is_recognized() {
        assert!(git_missing(false, false, ""), "no program");
        assert!(git_missing(
            true,
            false,
            "xcrun: error: invalid active developer path (/Library/Developer/CommandLineTools)"
        ));
        assert!(git_missing(
            true,
            true,
            "No developer tools were found, requesting install."
        ));
        assert!(!git_missing(true, true, ""), "git version 2.50.1");
    }
}
