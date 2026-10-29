//! The Window menu's list of open windows (Task-A07-mitten556, approver 2026-10-29). macOS fills
//! its Window menu itself; on Windows the app keeps the list: below Minimize, Maximize and Close
//! Window, after a separator line, one entry per open Allomos window (its project's name, or
//! "Allomos" with no project open), the window in front checked. The app has one menu for every
//! window, so the list is rebuilt when a window opens, closes, comes to the front or opens another
//! taskbag. Choosing an entry brings that window to the front.

use tauri::menu::{CheckMenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Manager, Wry};

/// An entry's menu ID is `window:<the window's label>`.
pub const PREFIX: &str = "window:";

/// What a window's title ends with when it shows a project (`set_window_project`).
const TITLE_END: &str = " — Allomos";

/// The Window menu and how many items it had before the list (Minimize, Maximize, Close Window).
pub struct WindowMenu {
    pub submenu: Submenu<Wry>,
    pub fixed: usize,
}

/// One entry of the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub id: String,
    pub text: String,
    pub checked: bool,
}

/// The name an entry shows for a window titled `title`: the project's name, or "Allomos".
fn name_of(title: &str) -> String {
    let name = title.strip_suffix(TITLE_END).unwrap_or(title).trim();
    if name.is_empty() {
        "Allomos".to_string()
    } else {
        name.to_string()
    }
}

/// The order windows opened in: the first window (`main`), then `window-<n>` by `n`.
pub(crate) fn opened_at(label: &str) -> u32 {
    match label {
        "main" => 0,
        _ => label
            .strip_prefix("window-")
            .and_then(|n| n.parse().ok())
            .unwrap_or(u32::MAX),
    }
}

/// The list for `windows` (label, title, in front), in the order they opened. `skip` names windows
/// that are not project windows (the About window) or are closing.
pub fn entries(windows: &[(String, String, bool)], skip: &[&str]) -> Vec<Entry> {
    let mut shown: Vec<&(String, String, bool)> = windows
        .iter()
        .filter(|(label, _, _)| !skip.contains(&label.as_str()))
        .collect();
    shown.sort_by_key(|(label, _, _)| (opened_at(label), label.clone()));
    shown
        .into_iter()
        .map(|(label, title, front)| Entry {
            id: format!("{PREFIX}{label}"),
            text: name_of(title),
            checked: *front,
        })
        .collect()
}

/// Rebuild the list from the open windows; `closing`, a window being closed, is left out. Nothing
/// happens on macOS, where no `WindowMenu` is kept.
pub fn refresh(app: &AppHandle, closing: Option<&str>) {
    let Some(menu) = app.try_state::<WindowMenu>() else {
        return;
    };
    let windows: Vec<(String, String, bool)> = app
        .webview_windows()
        .into_iter()
        .map(|(label, w)| {
            let title = w.title().unwrap_or_default();
            let front = w.is_focused().unwrap_or(false);
            (label, title, front)
        })
        .collect();
    let mut skip = vec![crate::ABOUT_WINDOW, crate::windows::SETTINGS_WINDOW];
    skip.extend(closing);
    // A menu that cannot be rebuilt keeps its previous list; nothing else depends on it.
    let _ = fill(app, &menu, &entries(&windows, &skip));
}

fn fill(app: &AppHandle, menu: &WindowMenu, list: &[Entry]) -> tauri::Result<()> {
    let items = menu.submenu.items()?;
    for item in items.iter().skip(menu.fixed) {
        menu.submenu.remove(item)?;
    }
    if list.is_empty() {
        return Ok(());
    }
    menu.submenu.append(&PredefinedMenuItem::separator(app)?)?;
    for entry in list {
        let item = CheckMenuItem::with_id(
            app,
            &entry.id,
            &entry.text,
            true,
            entry.checked,
            None::<&str>,
        )?;
        menu.submenu.append(&item)?;
    }
    Ok(())
}

/// The menu event `id` chose an entry: bring its window to the front.
pub fn choose(app: &AppHandle, id: &str) -> bool {
    let Some(label) = id.strip_prefix(PREFIX) else {
        return false;
    };
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
    // The check mark follows the window now in front, even if the system sends no focus event.
    refresh(app, None);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(label: &str, title: &str, front: bool) -> (String, String, bool) {
        (label.into(), title.into(), front)
    }

    #[test]
    fn each_window_is_listed_by_its_project_in_the_order_it_opened() {
        let list = entries(
            &[
                w("window-10", "Allomos", false),
                w("about", "About Allomos", false),
                w("window-2", "Jobs SSR — Allomos", true),
                w("main", "Allomos — Allomos", false),
            ],
            &["about"],
        );
        let shown: Vec<(&str, &str, bool)> = list
            .iter()
            .map(|e| (e.id.as_str(), e.text.as_str(), e.checked))
            .collect();
        assert_eq!(
            shown,
            [
                ("window:main", "Allomos", false),
                ("window:window-2", "Jobs SSR", true),
                ("window:window-10", "Allomos", false),
            ]
        );
    }

    #[test]
    fn a_closing_window_leaves_the_list_and_a_window_with_no_project_reads_allomos() {
        let list = entries(
            &[w("main", "", true), w("window-2", "Notes — Allomos", false)],
            &["about", "window-2"],
        );
        assert_eq!(
            list,
            [Entry {
                id: "window:main".into(),
                text: "Allomos".into(),
                checked: true,
            }]
        );
        assert_eq!(entries(&[], &[]), []);
    }
}
