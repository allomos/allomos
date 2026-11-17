//! Allomos desktop app: the Tauri window plus the commands the interface may call.
//!
//! The interface never calls these directly; it goes through its messenger module
//! (`apps/web/src/messenger.ts`), so the same interface can later reach the desktop's
//! local server from a browser without changes.

mod agent_commands;
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
mod task_commands;
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
use allomos_store::{
    Inspection, LinkTarget, add_readme_if_empty, id_from_name, init, inspect, load, picked_folder,
    resolve_task_link, stamp, versions,
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

/// The computer's preferred languages, most preferred first (for example `["ja-JP", "en-JP"]`).
/// Read from the operating system: inside the macOS app window, `navigator.language` is
/// not a reliable guide to the system's language.
#[tauri::command]
fn system_languages() -> Vec<String> {
    sys_locale::get_locales().collect()
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
            task_commands::edit_task,
            task_commands::review_ai_decisions,
            task_commands::approve_task,
            task_commands::withdraw_approval,
            task_commands::next_ids,
            task_commands::add_area,
            task_commands::add_iteration,
            task_commands::add_iteration_and_plan,
            task_commands::add_task,
            task_commands::rename_area,
            task_commands::change_iteration,
            task_commands::delete_area,
            task_commands::delete_iteration,
            menu::set_menu_texts,
            menu::set_recent_folders,
            agent_commands::find_agents,
            agent_commands::connect_agents,
            agent_commands::manual_request,
            agent_commands::build_app,
            agent_commands::build_app_request,
            agent_commands::move_tasks_in,
            agent_commands::move_tasks_in_request,
            agent_commands::translate_taskbag,
            agent_commands::translate_request,
            agent_commands::proceed_taskbag,
            agent_commands::proceed_with_agent,
            agent_commands::proceed_manual_request,
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
            agent_commands::run_modes_text,
            codex_rules::codex_rules_status,
            codex_rules::codex_rules_apply,
            copy_text,
            agent_commands::instruction_files,
            agent_commands::agents_set_up,
            agent_commands::last_agent,
            agent_commands::write_agent_files,
            agent_commands::agent_status,
            agent_commands::agent_setup_of,
            agent_commands::confirm_requests,
            agent_commands::answer_confirm_request,
            agent_commands::progress_reports,
            agent_commands::dismiss_progress,
            agent_commands::project_setup,
            agent_commands::set_project_folder,
            menu::set_connect_enabled,
            agent_commands::alm_command_status,
            agent_commands::install_alm_command,
            agent_commands::dismiss_alm_offer
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
}
