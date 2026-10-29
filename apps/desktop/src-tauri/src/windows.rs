//! The project windows (Task-A01-27) and their tabs (Task-A01-marble320, approver 2026-10-29):
//! which taskbags each window's tabs show, opening new windows, moving a tab to another window,
//! remembering the windows and tabs for the next start, and sizing windows to their screen.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use allomos_core::Problem;
use allomos_store::{Inspection, StartWith, inspect};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::window_list;

/// The app's own Settings window (Task-A01-marble320): not a project window, so menu commands
/// never go to it.
pub(crate) const SETTINGS_WINDOW: &str = "settings";
/// The windows' and tabs' file in the settings folder, for **Reopen windows and tabs**.
const SESSION_FILE: &str = "windows.json";
/// The events a window hears about its tabs.
const TAB_OPEN_EVENT: &str = "tab:open";
const TAB_SELECT_EVENT: &str = "tab:select";

/// One tab as its window reports it: the taskbag it shows (its root), or none for the start page,
/// and the project's name for the window's title.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct Tab {
    pub(crate) root: Option<PathBuf>,
    #[serde(default)]
    pub(crate) name: Option<String>,
}

/// A window's tabs, in their order, and the one in front.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Shown {
    pub(crate) tabs: Vec<Tab>,
    pub(crate) active: usize,
}

/// How a tab starts: `open`, a folder from File → Open Recent (one that is gone is said so);
/// `restore`, a tab of the last session (one that is gone leaves quietly); `moved`, a tab dragged
/// from another window (already open there, so not brought forward elsewhere).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct StartTab {
    pub(crate) root: Option<PathBuf>,
    pub(crate) how: &'static str,
}

/// What a window opens with (`window_start`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct WindowStart {
    pub(crate) tabs: Vec<StartTab>,
    pub(crate) active: usize,
    /// The first window at start-up: what Allomos opens when it starts (`reopen`, `last` or
    /// `blank`), so it also opens a folder given on the command line, or with no tabs restored, the
    /// last taskbag or the start page.
    pub(crate) startup: Option<&'static str>,
}

/// The windows and tabs as kept for the next start.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Session {
    pub(crate) windows: Vec<SavedWindow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SavedWindow {
    pub(crate) tabs: Vec<Option<PathBuf>>,
    pub(crate) active: usize,
}

/// Which taskbags each window's tabs show (Task-A01-27, Task-A01-marble320), by window label: a
/// taskbag already open in a tab is brought forward instead of opening twice (owner, 2026-10-12).
#[derive(Default)]
pub(crate) struct Windows {
    shown: Mutex<HashMap<String, Shown>>,
    /// What each new window opens with, until it reports its own tabs.
    starts: Mutex<HashMap<String, WindowStart>>,
    /// The number in the next new window's label (`window-<n>`).
    next: AtomicU32,
    /// The app is quitting: its windows stay in the session for the next start.
    quitting: AtomicBool,
}

impl Windows {
    fn shown(&self) -> std::sync::MutexGuard<'_, HashMap<String, Shown>> {
        self.shown.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn starts(&self) -> std::sync::MutexGuard<'_, HashMap<String, WindowStart>> {
        self.starts.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The project windows, by label: the windows that report tabs.
    pub(crate) fn labels(&self) -> Vec<String> {
        self.shown().keys().cloned().collect()
    }

    /// How many tabs window `label` has.
    pub(crate) fn tab_count(&self, label: &str) -> usize {
        self.shown().get(label).map_or(0, |s| s.tabs.len())
    }

    /// Every taskbag a tab shows, for Devices on Your Network.
    pub(crate) fn open_taskbags(&self) -> Vec<PathBuf> {
        let mut roots: Vec<PathBuf> = self
            .shown()
            .values()
            .flat_map(|s| s.tabs.iter().filter_map(|t| t.root.clone()))
            .collect();
        roots.sort();
        roots.dedup();
        roots
    }

    /// The app is quitting: what the windows show is kept as it is.
    pub(crate) fn quitting(&self) {
        self.quitting.store(true, Ordering::Relaxed);
    }

    /// A window closed. While other windows stay open it leaves the session; the last window, or
    /// every window when the app quits, stays in it, so the next start reopens it.
    pub(crate) fn forget(&self, label: &str) {
        self.starts().remove(label);
        let mut shown = self.shown();
        if shown.remove(label).is_none() || self.quitting.load(Ordering::Relaxed) {
            return;
        }
        if !shown.is_empty() {
            save_session(&session_of(&shown));
        }
    }
}

/// A folder as it is on disk, so `/a/./b` and `/a/b` are the same project.
pub(crate) fn same_place(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// The window and tab, other than in window `not`, that shows `root`, if any.
pub(crate) fn window_showing(
    shown: &HashMap<String, Shown>,
    root: &Path,
    not: &str,
) -> Option<(String, usize)> {
    let root = same_place(root);
    let mut labels: Vec<&String> = shown.keys().filter(|l| l.as_str() != not).collect();
    labels.sort_by_key(|l| (window_list::opened_at(l), l.to_string()));
    labels.into_iter().find_map(|label| {
        shown[label]
            .tabs
            .iter()
            .position(|t| t.root.as_deref() == Some(root.as_path()))
            .map(|i| (label.clone(), i))
    })
}

/// The windows and tabs to keep, in the order the windows opened. A window with no tabs is left
/// out.
pub(crate) fn session_of(shown: &HashMap<String, Shown>) -> Session {
    let mut labels: Vec<&String> = shown.keys().collect();
    labels.sort_by_key(|l| (window_list::opened_at(l), l.to_string()));
    Session {
        windows: labels
            .into_iter()
            .map(|l| &shown[l])
            .filter(|s| !s.tabs.is_empty())
            .map(|s| SavedWindow {
                tabs: s.tabs.iter().map(|t| t.root.clone()).collect(),
                active: s.active.min(s.tabs.len() - 1),
            })
            .collect(),
    }
}

fn session_path() -> Option<PathBuf> {
    allomos_store::settings_dir().map(|d| d.join(SESSION_FILE))
}

/// The windows and tabs kept at the last change; none when there is no file or it cannot be read.
pub(crate) fn read_session() -> Session {
    session_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Keep the windows and tabs for the next start. A file that cannot be written leaves the last one:
/// nothing else depends on it.
fn save_session(session: &Session) {
    let Some(path) = session_path() else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string_pretty(session) {
        let _ = std::fs::write(path, text);
    }
}

/// What each window of a kept session opens with; the first is the window Allomos starts with.
pub(crate) fn restored(session: &Session) -> Vec<WindowStart> {
    session
        .windows
        .iter()
        .filter(|w| !w.tabs.is_empty())
        .map(|w| WindowStart {
            tabs: w
                .tabs
                .iter()
                .map(|root| StartTab {
                    root: root.clone(),
                    how: "restore",
                })
                .collect(),
            active: w.active.min(w.tabs.len() - 1),
            startup: None,
        })
        .collect()
}

/// At start, with **Reopen windows and tabs**: the first window gets the first kept window's tabs
/// and every other kept window opens again. With the other choices, or nothing kept, the first
/// window decides by itself (`window_start`).
pub(crate) fn restore(app: &AppHandle) {
    let start_with = allomos_store::start_with();
    let mut windows = if start_with == StartWith::Reopen {
        restored(&read_session())
    } else {
        Vec::new()
    };
    let rest = if windows.is_empty() {
        Vec::new()
    } else {
        windows.split_off(1)
    };
    let first = windows.pop().unwrap_or(WindowStart {
        tabs: Vec::new(),
        active: 0,
        startup: None,
    });
    app.state::<Windows>().starts().insert(
        "main".to_string(),
        WindowStart {
            startup: Some(start_with.as_str()),
            ..first
        },
    );
    for start in rest {
        let _ = open_window(app, start);
    }
}

/// The label of the project window in front, else the first one.
/// The About and Settings windows are not project windows: menu commands skip them (Task-A07-28).
pub(crate) fn focused_window(app: &AppHandle) -> Option<String> {
    let windows = app.webview_windows();
    let projects = || {
        windows.iter().filter(|(label, _)| {
            label.as_str() != crate::ABOUT_WINDOW && label.as_str() != SETTINGS_WINDOW
        })
    };
    projects()
        .find(|(_, w)| w.is_focused().unwrap_or(false))
        .or_else(|| projects().next())
        .map(|(label, _)| label.clone())
}

/// File → New Window… (⌘N): another window with one tab on the start page.
pub(crate) fn open_new_window(app: &AppHandle) -> tauri::Result<()> {
    open_window(
        app,
        WindowStart {
            tabs: vec![StartTab {
                root: None,
                how: "open",
            }],
            active: 0,
            startup: None,
        },
    )
}

/// File → New Tab (⌘T): a new tab on the start page in the window in front, or a new window when
/// none is open.
pub(crate) fn new_tab(app: &AppHandle) {
    match focused_window(app) {
        Some(label) => {
            let _ = app.emit_to(label.as_str(), TAB_OPEN_EVENT, None::<String>);
        }
        None => {
            let _ = open_new_window(app);
        }
    }
}

/// Bring window `label` to the front with its tab `index` in front.
fn bring_forward(app: &AppHandle, label: &str, index: usize) -> bool {
    let Some(window) = app.get_webview_window(label) else {
        return false;
    };
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
    let _ = app.emit_to(label, TAB_SELECT_EVENT, index);
    true
}

/// File → Open Recent › a folder (Task-A01-marble320, replacing A01-deck174's new window): a tab
/// already showing its taskbag comes to the front, with its window; otherwise it opens in a new tab
/// of the window in front, or in a new window when none is open.
pub(crate) fn open_recent_in_window(app: &AppHandle, folder: &str) -> tauri::Result<()> {
    let path = PathBuf::from(folder);
    let root = match inspect(&path) {
        Inspection::Found { root } => root,
        _ => path.clone(),
    };
    let found = window_showing(&app.state::<Windows>().shown(), &root, "");
    if let Some((label, index)) = found {
        if bring_forward(app, &label, index) {
            return Ok(());
        }
    }
    if let Some(label) = focused_window(app) {
        return app.emit_to(label.as_str(), TAB_OPEN_EVENT, Some(folder));
    }
    open_window(
        app,
        WindowStart {
            tabs: vec![StartTab {
                root: Some(path),
                how: "open",
            }],
            active: 0,
            startup: None,
        },
    )
}

/// The label for the next new window, which opens with `start`.
fn next_label(app: &AppHandle, start: WindowStart) -> String {
    let windows = app.state::<Windows>();
    let label = format!(
        "window-{}",
        windows.next.fetch_add(1, Ordering::Relaxed) + 2
    );
    windows.starts().insert(label.clone(), start);
    label
}

fn window_builder<'a>(
    app: &'a AppHandle,
    label: &'a str,
) -> tauri::WebviewWindowBuilder<'a, tauri::Wry, AppHandle> {
    tauri::WebviewWindowBuilder::new(app, label, tauri::WebviewUrl::App("index.html".into()))
        .title("Allomos")
        .inner_size(1100.0, 720.0)
        .min_inner_size(720.0, 480.0)
        .visible(false)
}

/// A new project window that opens with `start`, centred.
pub(crate) fn open_window(app: &AppHandle, start: WindowStart) -> tauri::Result<()> {
    let label = next_label(app, start);
    window_builder(app, &label).build().map(|window| {
        fit_to_screen(&window);
        window_list::refresh(app, None);
    })
}

/// The window a tab torn off its tab bar comes with (Task-A01-fresco512): it shows that taskbag
/// (`root`, none for the start page), as large as the window it came from (`width` × `height`), at
/// (`x`, `y`) on the screen (logical pixels), without taking the keyboard from that window, which
/// still follows the pointer. Its label.
pub(crate) fn open_drag_window(
    app: &AppHandle,
    root: Option<PathBuf>,
    (width, height): (f64, f64),
    (x, y): (f64, f64),
) -> tauri::Result<String> {
    let label = next_label(
        app,
        WindowStart {
            tabs: vec![StartTab { root, how: "moved" }],
            active: 0,
            startup: None,
        },
    );
    window_builder(app, &label)
        .inner_size(width, height)
        .position(x, y)
        .focused(false)
        .visible(true)
        .build()?;
    window_list::refresh(app, None);
    Ok(label)
}

/// A window's size for a screen whose usable area is `width` × `height` (logical pixels): about
/// 80 % of the width and 85 % of the height, between the minimum window and 1280 × 900, never larger
/// than the screen (Task-A05-gusto837).
pub(crate) fn window_size(width: f64, height: f64) -> (f64, f64) {
    (
        (width * 0.8).clamp(720.0, 1280.0).min(width),
        (height * 0.85).clamp(480.0, 900.0).min(height),
    )
}

/// Size a hidden window to the screen it is on, centre it and show it. Without a screen to measure,
/// it keeps the size it was built with.
pub(crate) fn fit_to_screen(window: &tauri::WebviewWindow) {
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten());
    if let Some(monitor) = monitor {
        let area = monitor
            .work_area()
            .size
            .to_logical::<f64>(monitor.scale_factor());
        let (width, height) = window_size(area.width, area.height);
        let _ = window.set_size(tauri::LogicalSize::new(width, height));
        let _ = window.center();
    }
    let _ = window.show();
}

/// What the calling window opens with: its own tabs again when it reloads; the tabs it was opened
/// with; for the first window, what Allomos opens at start.
#[tauri::command]
pub(crate) fn window_start(
    window: tauri::WebviewWindow,
    windows: tauri::State<Windows>,
) -> WindowStart {
    if let Some(shown) = windows.shown().get(window.label()) {
        return WindowStart {
            tabs: shown
                .tabs
                .iter()
                .map(|t| StartTab {
                    root: t.root.clone(),
                    how: "moved",
                })
                .collect(),
            active: shown.active,
            startup: None,
        };
    }
    windows
        .starts()
        .get(window.label())
        .cloned()
        .unwrap_or(WindowStart {
            tabs: Vec::new(),
            active: 0,
            startup: None,
        })
}

/// The calling window's tabs now, in their order, and the one in front: the window is named after
/// that tab's project, and the windows and tabs are kept for the next start.
#[tauri::command]
pub(crate) fn set_window_tabs(
    window: tauri::WebviewWindow,
    windows: tauri::State<Windows>,
    tabs: Vec<Tab>,
    active: usize,
) {
    let name = tabs.get(active).and_then(|t| t.name.clone());
    let tabs = tabs
        .into_iter()
        .map(|t| Tab {
            root: t.root.as_deref().map(same_place),
            name: t.name,
        })
        .collect();
    {
        let mut shown = windows.shown();
        shown.insert(window.label().to_string(), Shown { tabs, active });
        save_session(&session_of(&shown));
    }
    let title = match name {
        Some(name) if !name.trim().is_empty() => format!("{name} — Allomos"),
        _ => "Allomos".to_string(),
    };
    let _ = window.set_title(&title);
    window_list::refresh(window.app_handle(), None);
}

/// Another window's tab already shows `root`: bring it forward and say so, so the calling window
/// does not open the taskbag a second time. (The calling window looks in its own tabs itself.)
#[tauri::command]
pub(crate) fn show_project_elsewhere(
    app: AppHandle,
    window: tauri::WebviewWindow,
    windows: tauri::State<Windows>,
    root: PathBuf,
) -> bool {
    let found = window_showing(&windows.shown(), &root, window.label());
    match found {
        Some((label, index)) => bring_forward(&app, &label, index),
        None => false,
    }
}

/// The app's own **Settings…** window (Task-A01-marble320): settings for the whole app, apart from
/// each taskbag's own. One at a time: choosing it again brings it forward.
pub(crate) fn open_settings_window(app: &AppHandle, title: &str) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(SETTINGS_WINDOW) {
        let _ = window.unminimize();
        window.show()?;
        return window.set_focus();
    }
    tauri::WebviewWindowBuilder::new(
        app,
        SETTINGS_WINDOW,
        tauri::WebviewUrl::App("index.html?settings".into()),
    )
    .title(title)
    .inner_size(560.0, 380.0)
    .resizable(false)
    .maximizable(false)
    .minimizable(false)
    .center()
    .build()
    .map(drop)
}

/// The app's settings the Settings window shows.
#[derive(Serialize)]
pub(crate) struct AppSettings {
    start_with: &'static str,
}

#[tauri::command]
pub(crate) fn app_settings() -> AppSettings {
    AppSettings {
        start_with: allomos_store::start_with().as_str(),
    }
}

/// **When Allomos starts**: `reopen`, `last` or `blank`.
#[tauri::command]
pub(crate) fn set_start_with(choice: String) -> Result<(), Problem> {
    let Some(choice) = StartWith::parse(&choice) else {
        return Err(Problem::InvalidChoice { value: choice });
    };
    allomos_store::save_start_with(choice).map_err(|e| Problem::WriteFailed {
        path: "settings.json".into(),
        detail: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_size_follows_the_screen() {
        // A 13-inch MacBook Air, a 1080p screen, a large screen, a small one.
        assert_eq!(window_size(1440.0, 875.0), (1152.0, 743.75));
        assert_eq!(window_size(1920.0, 1040.0), (1280.0, 884.0));
        assert_eq!(window_size(2560.0, 1415.0), (1280.0, 900.0));
        assert_eq!(window_size(800.0, 500.0), (720.0, 480.0));
        assert_eq!(window_size(700.0, 450.0), (700.0, 450.0));
    }

    fn tab(root: Option<&Path>) -> Tab {
        Tab {
            root: root.map(same_place),
            name: None,
        }
    }

    #[test]
    fn a_taskbag_shown_in_another_window_is_found_with_its_tab() {
        let dir = std::env::temp_dir().join(format!("allomos-windows-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("a")).unwrap();
        let mut shown = HashMap::new();
        shown.insert(
            "main".to_string(),
            Shown {
                tabs: vec![tab(None), tab(Some(&dir.join("a")))],
                active: 0,
            },
        );
        // The same folder written another way is the same project; the tab is the second.
        let other = dir.join("a").join(".").join("..").join("a");
        assert_eq!(
            window_showing(&shown, &other, "window-2"),
            Some(("main".to_string(), 1))
        );
        // A window does not count itself, and another folder is not shown anywhere.
        assert_eq!(window_showing(&shown, &dir.join("a"), "main"), None);
        assert_eq!(window_showing(&shown, &dir.join("b"), "window-2"), None);
        // Open Recent looks in every window.
        assert_eq!(
            window_showing(&shown, &dir.join("a"), ""),
            Some(("main".to_string(), 1))
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_session_keeps_windows_in_the_order_they_opened() {
        let mut shown = HashMap::new();
        let bag = |p: &str| Tab {
            root: Some(PathBuf::from(p)),
            name: Some("x".into()),
        };
        shown.insert(
            "window-10".to_string(),
            Shown {
                tabs: vec![bag("/c")],
                active: 0,
            },
        );
        shown.insert(
            "main".to_string(),
            Shown {
                tabs: vec![bag("/a"), tab(None)],
                active: 1,
            },
        );
        shown.insert(
            "window-2".to_string(),
            Shown {
                tabs: vec![],
                active: 0,
            },
        );
        let session = session_of(&shown);
        assert_eq!(
            session,
            Session {
                windows: vec![
                    SavedWindow {
                        tabs: vec![Some(PathBuf::from("/a")), None],
                        active: 1
                    },
                    SavedWindow {
                        tabs: vec![Some(PathBuf::from("/c"))],
                        active: 0
                    },
                ]
            }
        );
        // Read back, each window opens again with its tabs, the one in front still in front.
        let text = serde_json::to_string(&session).unwrap();
        let back: Session = serde_json::from_str(&text).unwrap();
        let starts = restored(&back);
        assert_eq!(starts.len(), 2);
        assert_eq!(starts[0].active, 1);
        assert_eq!(
            starts[0].tabs,
            vec![
                StartTab {
                    root: Some(PathBuf::from("/a")),
                    how: "restore"
                },
                StartTab {
                    root: None,
                    how: "restore"
                }
            ]
        );
        // A front tab past the end (an edited file) falls back to the last tab.
        let odd = Session {
            windows: vec![SavedWindow {
                tabs: vec![None],
                active: 5,
            }],
        };
        assert_eq!(restored(&odd)[0].active, 0);
    }
}
