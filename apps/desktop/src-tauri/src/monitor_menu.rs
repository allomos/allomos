//! The View menu's **Running Monitors** item (Task-A03-motor529, approver 2026-11-09; moved from
//! the Window menu by Task-A03-chair577): it opens
//! the list of this computer's running monitors in the window in front, and says how many run.
//! With none it stays in the menu but is greyed out. The count comes from the app's monitor loop
//! ([`crate::monitor_loop`], Task-A03-spider528), so the item is right whichever window is open
//! (and with none). The item's words are English like the app's other native menu items.

use tauri::menu::MenuItem;
use tauri::{AppHandle, Emitter, Manager, Wry};

/// The item's menu ID.
pub const ID: &str = "running_monitors";
/// The event sent to the window in front when the item is chosen; the window opens its list.
pub const EVENT: &str = "menu:monitors";
/// The item kept in the app's state, so its words and whether it is enabled can change.
pub struct MonitorMenu {
    pub item: MenuItem<Wry>,
}

/// The item's words for `count` running monitors.
pub fn text(count: usize) -> String {
    if count == 0 {
        "Running Monitors".to_string()
    } else {
        format!("Running Monitors ({count})…")
    }
}

/// Shows `count` running monitors (records missing included, as the list shows them).
pub fn show(app: &AppHandle, count: usize) {
    if let Some(menu) = app.try_state::<MonitorMenu>() {
        // A menu that cannot be changed keeps its words; nothing else depends on it.
        let _ = menu.item.set_text(text(count));
        let _ = menu.item.set_enabled(count > 0);
    }
}

/// The item was chosen: the window in front opens the list. With no window open, a new one opens
/// and the person chooses the item again.
pub fn chosen(app: &AppHandle, target: Option<&str>) {
    match target {
        Some(label) => {
            let _ = app.emit_to(label, EVENT, ());
        }
        None => {
            let _ = crate::windows::open_new_window(app);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_words_say_how_many_run() {
        assert_eq!(text(0), "Running Monitors");
        assert_eq!(text(2), "Running Monitors (2)…");
    }
}
