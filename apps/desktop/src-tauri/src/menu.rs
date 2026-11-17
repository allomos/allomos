//! The app's menu bar: its items and their words in the interface's language, Open Recent, the
//! update item, and the About window.

use std::sync::Mutex;

use allomos_core::Problem;
use serde::Serialize;
#[cfg(not(any(target_os = "macos", windows)))]
use tauri::menu::AboutMetadata;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Manager, Wry};

use crate::windows::{focused_window, open_new_window, open_recent_in_window};
use crate::{for_you, monitor_menu, opener, update_commands, window_list, windows};

/// File → Connect with the Agents… and File → Update the Taskbag… are offered only while a
/// taskbag is open.
#[tauri::command]
pub(crate) fn set_connect_enabled(texts: tauri::State<MenuTexts>, enabled: bool) {
    let _ = texts.connect.set_enabled(enabled);
    let _ = texts.move_tasks_in.set_enabled(enabled);
}

/// The menu items' IDs, and the events the interface hears when they are chosen.
/// File → New Window… (⌘N): another window on the start page (Task-A01-27). It replaces
/// File → Open Folder… (⌘O, removed; owner 2026-10-12).
const NEW_WINDOW_ID: &str = "new_window";
/// File → New Tab (⌘T / Ctrl+T): a new tab on the start page (Task-A01-marble320).
const NEW_TAB_ID: &str = "new_tab";
/// **Settings…** (⌘, / Ctrl+,): the settings dialog of the window in front, at its Allomos group
/// (Task-A01-marble320, Task-A05-cookie962); in the Allomos menu on macOS, in the File menu
/// elsewhere.
const SETTINGS_ID: &str = "app_settings";
const SETTINGS_EVENT: &str = "menu:settings";
/// A recent folder's item is `recent:<position in the list>`.
const RECENT_PREFIX: &str = "recent:";
const CLEAR_RECENT_ID: &str = "recent_clear";
const CLEAR_RECENT_EVENT: &str = "menu:clear_recent";
const CONNECT_ID: &str = "connect";
const CONNECT_EVENT: &str = "menu:connect";
const MOVE_TASKS_IN_ID: &str = "move_tasks_in";
const MOVE_TASKS_IN_EVENT: &str = "menu:move_tasks_in";
/// File → Devices on Your Network…: "Allow devices on your network to view" (Task-A04-02-01).
const LAN_ID: &str = "lan_viewing";
const LAN_EVENT: &str = "menu:lan_viewing";
const CHECK_UPDATES_ID: &str = "check_updates";
/// The website the About window links to, and the words it shows (Task-A07-22).
const WEBSITE: &str = "https://www.allomos.com";
#[cfg(not(any(target_os = "macos", windows)))]
const WEBSITE_LABEL: &str = "www.allomos.com";
/// About Allomos on macOS and Windows: the menu item, and the label of the app's own About
/// window it opens (Task-A07-28).
#[cfg(any(target_os = "macos", windows))]
const ABOUT_ID: &str = "about";
/// The About window's label: not a project window, so menu commands never go to it.
pub(crate) const ABOUT_WINDOW: &str = "about";
const CHECK_UPDATES_EVENT: &str = "menu:check_updates";

/// What the update menu item does when chosen (Task-A07-16, Task-A07-17).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum UpdateAction {
    /// **Check for Updates…**: the window asks GitHub now.
    #[default]
    Check,
    /// **Download Allomos <version>…**: open the download page.
    Download,
    /// **Restart to Update to Allomos <version>**: install the checked download.
    Restart,
}

/// The menu items whose words follow the interface's language, and the Open Recent list.
pub(crate) struct MenuTexts {
    file: Submenu<Wry>,
    new_tab: MenuItem<Wry>,
    new_window: MenuItem<Wry>,
    settings: MenuItem<Wry>,
    connect: MenuItem<Wry>,
    move_tasks_in: MenuItem<Wry>,
    lan: MenuItem<Wry>,
    for_you: MenuItem<Wry>,
    check_updates: MenuItem<Wry>,
    /// Check for Updates… has become **Download Allomos <version>…** (Task-A07-16) or
    /// **Restart to Update to Allomos <version>** (Task-A07-17).
    update_action: Mutex<UpdateAction>,
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
pub(crate) fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let menu = Menu::default(app)?;
    let new_tab = MenuItem::with_id(app, NEW_TAB_ID, "New Tab", true, Some("CmdOrCtrl+T"))?;
    let new_window =
        MenuItem::with_id(app, NEW_WINDOW_ID, "New Window…", true, Some("CmdOrCtrl+N"))?;
    let settings = MenuItem::with_id(app, SETTINGS_ID, "Settings…", true, Some("CmdOrCtrl+,"))?;
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
    // An app setting, not a project's: always enabled.
    let lan_item = MenuItem::with_id(app, LAN_ID, "Devices on Your Network…", true, None::<&str>)?;
    fill_recent(app, &recent, &Recent::default())?;
    let existing = menu
        .items()?
        .into_iter()
        .filter_map(|item| item.as_submenu().cloned())
        .find(|s| s.text().is_ok_and(|t| t == "File"));
    let mut items: Vec<&dyn tauri::menu::IsMenuItem<Wry>> = vec![
        &new_tab,
        &new_window,
        &recent,
        &connect_item,
        &move_tasks_in,
        &lan_item,
    ];
    // Settings… is in the Allomos menu on macOS (below), in the File menu elsewhere.
    if !cfg!(target_os = "macos") {
        items.push(&settings);
    }
    let file = match existing {
        Some(file) => {
            file.prepend_items(&items)?;
            file.insert(&PredefinedMenuItem::separator(app)?, items.len())?;
            file
        }
        None => {
            let file = Submenu::with_items(app, "File", true, &items)?;
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
    // is replaced by one that also gives the website (Task-A07-22).
    let about = about_item(app)?;
    if cfg!(target_os = "macos") {
        if let Some(app_menu) = submenus.first() {
            app_menu.remove_at(0)?;
            app_menu.insert(&about, 0)?;
            app_menu.insert(&check_updates, 1)?;
            app_menu.insert(&PredefinedMenuItem::separator(app)?, 2)?;
            app_menu.insert(&settings, 3)?;
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
    // Task-A03-chair577 (moved from the Window menu, where Task-A03-motor529 put it): the Running
    // Monitors item is in the View menu, after the View menu's own items (on macOS, Enter Full
    // Screen). Windows and Linux have no View menu by default, so one is made before the Window
    // menu.
    let monitors = MenuItem::with_id(
        app,
        monitor_menu::ID,
        monitor_menu::text(0),
        false,
        None::<&str>,
    )?;
    let view = match menu
        .items()?
        .into_iter()
        .filter_map(|item| item.as_submenu().cloned())
        .find(|s| s.text().is_ok_and(|t| t == "View"))
    {
        Some(view) => {
            view.append(&PredefinedMenuItem::separator(app)?)?;
            view
        }
        None => {
            let view = Submenu::new(app, "View", true)?;
            let window_at = menu
                .items()?
                .iter()
                .position(|item| item.id() == tauri::menu::WINDOW_SUBMENU_ID)
                .unwrap_or(0);
            menu.insert(&view, window_at)?;
            view
        }
    };
    // Task-A01-oak609 (decision 0028): the For You window, before Running Monitors.
    let for_you = MenuItem::with_id(
        app,
        for_you::MENU_ID,
        "For You",
        true,
        Some(for_you::SHORTCUT),
    )?;
    view.append(&for_you)?;
    view.append(&monitors)?;
    app.manage(monitor_menu::MonitorMenu { item: monitors });
    // Task-A07-mitten556: on Windows the app lists the open windows in the Window menu itself.
    if !cfg!(target_os = "macos") {
        if let Some(submenu) = menu
            .get(tauri::menu::WINDOW_SUBMENU_ID)
            .and_then(|item| item.as_submenu().cloned())
        {
            let fixed = submenu.items()?.len();
            app.manage(window_list::WindowMenu { submenu, fixed });
        }
    }
    app.manage(MenuTexts {
        file,
        new_tab,
        new_window,
        settings,
        connect: connect_item,
        move_tasks_in,
        lan: lan_item,
        for_you,
        check_updates,
        update_action: Mutex::new(UpdateAction::Check),
        recent,
        state: Mutex::new(Recent::default()),
    });
    Ok(menu)
}

/// About Allomos. On macOS and Windows the item opens the app's own About window
/// ([`open_about_window`], Task-A07-28): the same on both, in the interface's language, with the
/// website as a real link. Linux, not shipped (decision 0011), keeps the default item.
#[cfg(any(target_os = "macos", windows))]
fn about_item(app: &AppHandle) -> tauri::Result<MenuItem<Wry>> {
    let name = &app.package_info().name;
    MenuItem::with_id(app, ABOUT_ID, format!("About {name}"), true, None::<&str>)
}

#[cfg(not(any(target_os = "macos", windows)))]
fn about_item(app: &AppHandle) -> tauri::Result<PredefinedMenuItem<Wry>> {
    let info = app.package_info();
    let config = app.config();
    let metadata = AboutMetadata {
        name: Some(info.name.clone()),
        version: Some(info.version.to_string()),
        copyright: config.bundle.copyright.clone(),
        authors: config.bundle.publisher.clone().map(|p| vec![p]),
        website: Some(WEBSITE.to_string()),
        website_label: Some(WEBSITE_LABEL.to_string()),
        ..Default::default()
    };
    PredefinedMenuItem::about(app, None, Some(metadata))
}

/// The app's own About window (Task-A07-28): a small window showing the web interface's About
/// page (`index.html?about`): the icon, name, version, slogan, the website link and the licence.
/// One at a time: choosing About again brings it forward.
#[cfg(any(target_os = "macos", windows))]
fn open_about_window(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(ABOUT_WINDOW) {
        let _ = window.unminimize();
        window.show()?;
        return window.set_focus();
    }
    let name = &app.package_info().name;
    tauri::WebviewWindowBuilder::new(
        app,
        ABOUT_WINDOW,
        tauri::WebviewUrl::App("index.html?about".into()),
    )
    .title(format!("About {name}"))
    .inner_size(360.0, 400.0)
    .resizable(false)
    .maximizable(false)
    .minimizable(false)
    .center()
    .build()
    .map(drop)
}

/// What the About window shows of the app itself: its name and version (Task-A07-28).
#[derive(Serialize)]
pub(crate) struct AboutInfo {
    name: String,
    version: String,
}

#[tauri::command]
pub(crate) fn about_info(app: AppHandle) -> AboutInfo {
    let info = app.package_info();
    AboutInfo {
        name: info.name.clone(),
        version: info.version.to_string(),
    }
}

/// The About window's link: the website in the default browser. A fixed address: the window
/// cannot ask to open any other.
#[tauri::command]
pub(crate) fn open_website() -> Result<(), Problem> {
    opener()
        .arg(WEBSITE)
        .spawn()
        .map(drop)
        .map_err(|e| Problem::OpenFailed {
            path: WEBSITE.to_string(),
            detail: e.to_string(),
        })
}

/// Close the calling window (the About window's Escape key).
#[tauri::command]
pub(crate) fn close_window(window: tauri::WebviewWindow) {
    let _ = window.close();
}

/// The menu's words in the interface's language, as the interface sends them.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MenuWords {
    file: String,
    new_tab: String,
    new_window: String,
    settings: String,
    open_recent: String,
    clear_menu: String,
    no_recent: String,
    connect: String,
    move_tasks_in: String,
    lan: String,
    for_you: String,
}

/// Put the menu's words in the interface's language.
#[tauri::command]
pub(crate) fn set_menu_texts(app: AppHandle, texts: tauri::State<MenuTexts>, words: MenuWords) {
    // A menu that cannot be renamed keeps its previous words; nothing else depends on them.
    let _ = texts.file.set_text(words.file);
    let _ = texts.new_tab.set_text(words.new_tab);
    let _ = texts.new_window.set_text(words.new_window);
    let _ = texts.settings.set_text(words.settings);
    let _ = texts.recent.set_text(words.open_recent);
    let _ = texts.connect.set_text(words.connect);
    let _ = texts.move_tasks_in.set_text(words.move_tasks_in);
    let _ = texts.lan.set_text(words.lan);
    let _ = texts.for_you.set_text(words.for_you);
    let mut recent = texts.state.lock().unwrap_or_else(|e| e.into_inner());
    recent.clear = words.clear_menu;
    recent.none = words.no_recent;
    let _ = fill_recent(&app, &texts.recent, &recent);
}

/// The update item's words and what it does, as the window sends them: **Check for Updates…**
/// (`check`); **Download Allomos <version>…** once a newer version is known, which opens the
/// download page (`download`, Task-A07-16); **Restart to Update to Allomos <version>** once its
/// installer is downloaded and checked, as the Claude app has it (`restart`, Task-A07-17).
/// `enabled` is false while the installer downloads (A07-glove300): the item waits, greyed out.
#[tauri::command]
pub(crate) fn set_update_item(
    texts: tauri::State<MenuTexts>,
    text: String,
    action: String,
    enabled: bool,
) {
    let _ = texts.check_updates.set_text(text);
    let _ = texts.check_updates.set_enabled(enabled);
    let action = match action.as_str() {
        "download" => UpdateAction::Download,
        "restart" => UpdateAction::Restart,
        _ => UpdateAction::Check,
    };
    *texts
        .update_action
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = action;
}

/// Show these folders, newest first, under File → Open Recent.
#[tauri::command]
pub(crate) fn set_recent_folders(
    app: AppHandle,
    texts: tauri::State<MenuTexts>,
    folders: Vec<String>,
) {
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

/// A menu item was chosen.
pub(crate) fn chosen(app: &AppHandle, event: tauri::menu::MenuEvent) {
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
    #[cfg(any(target_os = "macos", windows))]
    if id == ABOUT_ID {
        let _ = open_about_window(app);
        return;
    }
    if window_list::choose(app, id) {
        return;
    }
    if id == for_you::MENU_ID {
        // The queue of the project in front in the window in front (Task-A01-oak609).
        let _ = for_you::open(app, target.as_deref());
        return;
    }
    if id == monitor_menu::ID {
        monitor_menu::chosen(app, target.as_deref());
        return;
    }
    if id == NEW_WINDOW_ID {
        let _ = open_new_window(app);
    } else if id == NEW_TAB_ID {
        windows::new_tab(app);
    } else if id == SETTINGS_ID {
        // One settings dialog, in the window in front (Task-A05-cookie962); with no window
        // open, a new one shows it once it has loaded.
        match &target {
            Some(_) => send(SETTINGS_EVENT, None),
            None => {
                windows::ask_for_settings();
                let _ = open_new_window(app);
            }
        }
    } else if id == CONNECT_ID {
        send(CONNECT_EVENT, None);
    } else if id == MOVE_TASKS_IN_ID {
        send(MOVE_TASKS_IN_EVENT, None);
    } else if id == LAN_ID {
        send(LAN_EVENT, None);
    } else if id == CHECK_UPDATES_ID {
        let action = *app
            .state::<MenuTexts>()
            .update_action
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        match action {
            UpdateAction::Check => send(CHECK_UPDATES_EVENT, None),
            UpdateAction::Download => {
                let _ = update_commands::open_download_page();
            }
            UpdateAction::Restart => update_commands::restart_to_update(app.clone()),
        }
    } else if id == CLEAR_RECENT_ID {
        send(CLEAR_RECENT_EVENT, None);
    } else if let Some(folder) = recent_folder(&app.state::<MenuTexts>(), id) {
        // In a new window, or the one already showing it (A01-deck174).
        let _ = open_recent_in_window(app, &folder);
    }
}
