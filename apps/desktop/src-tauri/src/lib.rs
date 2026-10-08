//! FlipHopper desktop app: the Tauri window plus the commands the interface may call.
//!
//! The interface never calls these directly; it goes through its messenger module
//! (`apps/web/src/messenger.ts`), so the same interface can later reach the desktop's
//! local server from a browser without changes.

mod agents;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use fliphopper_core::{Loaded, Problem};
use fliphopper_store::{
    AgentSetup, Inspection, add_agent_instructions, add_readme_if_empty, agent_setup,
    agent_setup_request_in, create_starter, git_root, inspect, load, picked_folder, stamp,
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
    /// A `README-fliphopper.md` was just written next to `.fliphopper/`.
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

/// The folder to open at start-up: `FLIPHOPPER_OPEN` if set, else the command line's folder.
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

/// A folder given at start-up, so `fliphopper ~/my-project` and
/// `open FlipHopper.app --args ~/my-project` open that folder directly.
#[tauri::command]
fn startup_folder() -> Result<Option<PathBuf>, Problem> {
    choose_startup_folder(std::env::var_os("FLIPHOPPER_OPEN"), std::env::args_os())
}

/// What is this folder? Changes nothing.
#[tauri::command]
fn inspect_folder(path: PathBuf) -> FolderInfo {
    FolderInfo {
        git_root: git_root(&path),
        inspection: inspect(&path),
    }
}

/// Load the seabag at `root`.
#[tauri::command]
fn open_seabag(root: PathBuf) -> Opened {
    Opened {
        loaded: load(&root),
        root,
        readme_added: false,
    }
}

/// A fingerprint of the seabag's files. The window asks for it about once a second and
/// reloads when it changes, so changes made by `fh` show without reopening the folder.
/// Sent as text: a 64-bit number does not fit a JavaScript number.
#[tauri::command]
fn seabag_stamp(root: PathBuf) -> String {
    stamp(&root).to_string()
}

/// Create a starter seabag at `root` (normally `<folder>/.fliphopper/seabag/`), then load it.
/// When the picked folder looks empty, also write `README-fliphopper.md` there, in the
/// interface's `language`; a README that cannot be written does not undo the seabag.
/// (Telling AI agents about the project is a separate step: `ask_agents`.)
#[tauri::command]
fn create_seabag(root: PathBuf, name: String, language: String) -> Result<Opened, Problem> {
    create_starter(&root, &name).map_err(|e| {
        let path = root.display().to_string();
        if e.kind() == std::io::ErrorKind::AlreadyExists {
            Problem::FolderNotEmpty { path }
        } else {
            Problem::CreateFailed {
                path,
                detail: e.to_string(),
            }
        }
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
/// desktop's file manager elsewhere. Only a seabag folder is accepted, so the window cannot
/// be used to open any other path.
fn file_manager_command(root: &Path) -> Result<Command, Problem> {
    if inspect(root)
        != (Inspection::Found {
            root: root.to_path_buf(),
        })
    {
        return Err(Problem::NotASeabag {
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
    let mut command = Command::new(program);
    command.arg(root);
    Ok(command)
}

/// Show the open seabag's folder in Finder or Explorer.
#[tauri::command]
fn show_in_file_manager(root: PathBuf) -> Result<(), Problem> {
    let mut command = file_manager_command(&root)?;
    // Not waited for: Explorer reports failure even when it opened the folder.
    command.spawn().map(drop).map_err(|e| Problem::OpenFailed {
        path: root.display().to_string(),
        detail: e.to_string(),
    })
}

/// The computer's preferred languages, most preferred first (for example `["ja-JP", "en-JP"]`).
/// Read from the operating system: inside the macOS app window, `navigator.language` is
/// not a reliable guide to the system's language.
#[tauri::command]
fn system_languages() -> Vec<String> {
    sys_locale::get_locales().collect()
}

/// The AI agents on this computer that can be asked to add FlipHopper's instructions.
#[tauri::command]
async fn find_agents() -> Vec<agents::Agent> {
    agents::find_agents()
}

/// Open each chosen agent in `project` with the request to add FlipHopper's instructions typed
/// in. Nothing is sent: the person sends it. Returns the agents that opened.
#[tauri::command]
async fn ask_agents(
    agents: Vec<String>,
    project: PathBuf,
    seabag: PathBuf,
    language: String,
) -> Result<Vec<String>, Problem> {
    if !project.is_dir() {
        return Err(Problem::StartupNotAFolder {
            path: project.display().to_string(),
        });
    }
    let found = agents::find_agents();
    let mut request = agent_setup_request_in(&language, &project, &seabag);
    if let Some(note) = agents::shared_note(&agents, &language) {
        request = format!("{note}\n\n{request}");
    }
    let mut opened = Vec::new();
    for id in &agents {
        let Some(agent) = found.iter().find(|a| &a.id == id) else {
            continue;
        };
        agents::ask(&agent.id, &project, &request, &agent.how)?;
        opened.push(agent.id.clone());
    }
    Ok(opened)
}

/// Do the AI agents know about this project? What the window's agent line shows (Task-A03-04).
#[derive(Serialize)]
struct AgentStatus {
    /// The project folder the line is about.
    project: PathBuf,
    /// Which agents' instruction files tell them about FlipHopper.
    setup: AgentSetup,
    /// The newest seabag change an agent made with `fh --commit` / `--push`.
    last: Option<agents::LastUse>,
    /// Who holds tasks in progress now.
    holders: Vec<String>,
}

#[tauri::command]
async fn agent_status(root: PathBuf) -> AgentStatus {
    let project = picked_folder(&root).unwrap_or(&root).to_path_buf();
    let mut holders: Vec<String> = load(&root)
        .seabag
        .map(|bag| {
            bag.tasks
                .into_iter()
                .filter(|t| t.state == fliphopper_core::State::InProgress)
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

/// Write FlipHopper's section into `project`'s AGENTS.md and CLAUDE.md directly, for when no
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
const AGENTS_ID: &str = "agents";
const AGENTS_EVENT: &str = "menu:agents";

/// The menu items whose words follow the interface's language, and the Open Recent list.
struct MenuTexts {
    file: Submenu<Wry>,
    open_folder: MenuItem<Wry>,
    agents: MenuItem<Wry>,
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
    let agents_item = MenuItem::with_id(
        app,
        AGENTS_ID,
        "Tell AI Agents About This Project…",
        true,
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
            file.prepend_items(&[&open_folder, &recent, &agents_item])?;
            file.insert(&PredefinedMenuItem::separator(app)?, 3)?;
            file
        }
        None => {
            let file =
                Submenu::with_items(app, "File", true, &[&open_folder, &recent, &agents_item])?;
            // After the app menu on macOS; first elsewhere.
            menu.insert(&file, usize::from(cfg!(target_os = "macos")))?;
            file
        }
    };
    app.manage(MenuTexts {
        file,
        open_folder,
        agents: agents_item,
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
    agents: String,
}

/// Put the menu's words in the interface's language.
#[tauri::command]
fn set_menu_texts(app: AppHandle, texts: tauri::State<MenuTexts>, words: MenuWords) {
    // A menu that cannot be renamed keeps its previous words; nothing else depends on them.
    let _ = texts.file.set_text(words.file);
    let _ = texts.open_folder.set_text(words.open_folder);
    let _ = texts.recent.set_text(words.open_recent);
    let _ = texts.agents.set_text(words.agents);
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
            } else if id == AGENTS_ID {
                let _ = app.emit(AGENTS_EVENT, ());
            } else if id == CLEAR_RECENT_ID {
                let _ = app.emit(CLEAR_RECENT_EVENT, ());
            } else if let Some(folder) = recent_folder(&app.state::<MenuTexts>(), id) {
                let _ = app.emit(OPEN_RECENT_EVENT, folder);
            }
        })
        .invoke_handler(tauri::generate_handler![
            startup_folder,
            inspect_folder,
            open_seabag,
            seabag_stamp,
            create_seabag,
            show_in_file_manager,
            system_languages,
            set_menu_texts,
            set_recent_folders,
            find_agents,
            ask_agents,
            write_agent_files,
            agent_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running FlipHopper");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<OsString> {
        std::iter::once("fliphopper")
            .chain(list.iter().copied())
            .map(OsString::from)
            .collect()
    }

    #[test]
    fn startup_folder_skips_options() {
        let dir = std::env::temp_dir().join(format!("fliphopper-start-{}", std::process::id()));
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
        // FLIPHOPPER_OPEN wins over arguments, and is checked the same way.
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
    fn file_manager_opens_only_a_seabag() {
        let dir = std::env::temp_dir().join(format!("fliphopper-reveal-{}", std::process::id()));
        let root = dir.join(fliphopper_store::DEFAULT_DIR);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // The project folder itself, a missing path and a plain file are refused.
        for path in [dir.clone(), dir.join("missing"), root.clone()] {
            assert!(matches!(
                file_manager_command(&path),
                Err(Problem::NotASeabag { .. })
            ));
        }
        fliphopper_store::init(&root, "p", "P").unwrap();
        let command = file_manager_command(&root).unwrap();
        assert_eq!(command.get_args().collect::<Vec<_>>(), [root.as_os_str()]);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
