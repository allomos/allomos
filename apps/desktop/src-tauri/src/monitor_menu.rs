//! The View menu's **Running Monitors** item (Task-A03-motor529, approver 2026-11-09; moved from
//! the Window menu by Task-A03-chair577): it opens
//! the list of this computer's running monitors in the window in front, and says how many run.
//! With none it stays in the menu but is greyed out. The count is read from the run files every
//! ten seconds, on a thread of its own, so the item is right whichever window is open (and
//! with none). The item's words are English like the app's other native menu items.

use std::time::Duration;

use allomos_monitor::registry;
use tauri::menu::MenuItem;
use tauri::{AppHandle, Emitter, Manager, Wry};

/// The item's menu ID.
pub const ID: &str = "running_monitors";
/// The event sent to the window in front when the item is chosen; the window opens its list.
pub const EVENT: &str = "menu:monitors";
/// How often the count is read again. Ten seconds (Task-A03-reef286): reading it starts `ps`, and
/// an idle app should cost next to nothing; the window's own reads share their reading with it.
const EVERY: Duration = Duration::from_secs(10);

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

/// How many monitors this computer runs now, records missing included (as the list shows them).
pub fn count() -> usize {
    let Some(dir) = registry::dir() else {
        return 0;
    };
    let procs = registry::recent_processes(crate::monitor_runs::PROCESSES_AGE);
    registry::list(&dir, Some(&procs)).len()
}

/// Keeps the item's words and enabled state up to date until the app closes.
pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let mut shown: Option<usize> = None;
        loop {
            let n = count();
            if shown != Some(n) {
                if let Some(menu) = app.try_state::<MonitorMenu>() {
                    // A menu that cannot be changed keeps its words; nothing else depends on it.
                    let _ = menu.item.set_text(text(n));
                    let _ = menu.item.set_enabled(n > 0);
                }
                shown = Some(n);
            }
            std::thread::sleep(EVERY);
        }
    });
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
