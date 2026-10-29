//! The project windows (Task-A01-27): which taskbag each one shows, opening new ones, and sizing
//! them to their screen.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use allomos_core::Problem;
use allomos_store::{Inspection, inspect};
use tauri::{AppHandle, Manager};

use crate::window_list;

/// Which project each window shows (Task-A01-27), by window label: a project already open in
/// another window is brought forward instead of opening twice (owner, 2026-10-12).
#[derive(Default)]
pub(crate) struct Windows {
    pub(crate) projects: Mutex<std::collections::HashMap<String, PathBuf>>,
    /// The number in the next new window's label (`window-<n>`).
    pub(crate) next: std::sync::atomic::AtomicU32,
}

impl Windows {
    pub(crate) fn forget(&self, label: &str) {
        self.projects
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(label);
    }
}

/// A folder as it is on disk, so `/a/./b` and `/a/b` are the same project.
pub(crate) fn same_place(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// The other window that shows `root`, if any.
pub(crate) fn window_showing(
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
/// The About window is not a project window: menu commands skip it (Task-A07-28).
pub(crate) fn focused_window(app: &AppHandle) -> Option<String> {
    let windows = app.webview_windows();
    let projects = || {
        windows
            .iter()
            .filter(|(label, _)| label.as_str() != crate::ABOUT_WINDOW)
    };
    projects()
        .find(|(_, w)| w.is_focused().unwrap_or(false))
        .or_else(|| projects().next())
        .map(|(label, _)| label.clone())
}

/// Open another window on the start page. `?new` tells it not to reopen the last folder.
pub(crate) fn open_new_window(app: &AppHandle) -> tauri::Result<()> {
    open_window_at(app, "index.html?new")
}

/// File → Open Recent › a folder (Task-A01-deck174, approver 2026-10-25,
/// replacing "in the current window", 2026-10-12): the window already showing its taskbag comes to
/// the front; otherwise a new window opens it, and the window the menu was used from keeps what it
/// showed.
pub(crate) fn open_recent_in_window(app: &AppHandle, folder: &str) -> tauri::Result<()> {
    let path = PathBuf::from(folder);
    let root = match inspect(&path) {
        Inspection::Found { root } => root,
        _ => path.clone(),
    };
    let label = {
        let windows = app.state::<Windows>();
        let projects = windows.projects.lock().unwrap_or_else(|e| e.into_inner());
        window_showing(&projects, &root, "")
    };
    if let Some(window) = label.and_then(|l| app.get_webview_window(&l)) {
        let _ = window.unminimize();
        let _ = window.show();
        return window.set_focus();
    }
    open_window_at(app, &format!("index.html?open={}", url_encode(folder)))
}

/// `text` for a URL's query: letters, digits and `-_.~` as they are, every other byte as `%XX`.
pub(crate) fn url_encode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// A new project window at `url` (the interface's page and its query).
pub(crate) fn open_window_at(app: &AppHandle, url: &str) -> tauri::Result<()> {
    let n = app
        .state::<Windows>()
        .next
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        + 2;
    tauri::WebviewWindowBuilder::new(
        app,
        format!("window-{n}"),
        tauri::WebviewUrl::App(url.into()),
    )
    .title("Allomos")
    .inner_size(1100.0, 720.0)
    .min_inner_size(720.0, 480.0)
    .visible(false)
    .build()
    .map(|window| {
        fit_to_screen(&window);
        window_list::refresh(app, None);
    })
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

/// The top bar's **New window…** (Task-A01-27).
#[tauri::command]
pub(crate) fn new_window(app: AppHandle) -> Result<(), Problem> {
    open_new_window(&app).map_err(|e| Problem::OpenFailed {
        path: "New window".into(),
        detail: e.to_string(),
    })
}

/// The calling window now shows `root` (or nothing); its title names the project.
#[tauri::command]
pub(crate) fn set_window_project(
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
    window_list::refresh(window.app_handle(), None);
}

/// Another window already shows `root`: bring it forward and say so, so the calling window does
/// not open the project a second time.
#[tauri::command]
pub(crate) fn show_project_elsewhere(
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The folder in a new window's address (A01-deck174): everything but plain letters, digits
    /// and `-_.~` is escaped, so spaces, `&`, `#` and non-ASCII names arrive whole.
    #[test]
    fn a_recent_folder_goes_whole_into_the_address() {
        assert_eq!(url_encode("/Users/a b/proj"), "%2FUsers%2Fa%20b%2Fproj");
        assert_eq!(url_encode("x&y#z"), "x%26y%23z");
        assert_eq!(url_encode("项目"), "%E9%A1%B9%E7%9B%AE");
    }

    #[test]
    fn window_size_follows_the_screen() {
        // A 13-inch MacBook Air, a 1080p screen, a large screen, a small one.
        assert_eq!(window_size(1440.0, 875.0), (1152.0, 743.75));
        assert_eq!(window_size(1920.0, 1040.0), (1280.0, 884.0));
        assert_eq!(window_size(2560.0, 1415.0), (1280.0, 900.0));
        assert_eq!(window_size(800.0, 500.0), (720.0, 480.0));
        assert_eq!(window_size(700.0, 450.0), (700.0, 450.0));
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
}
