//! The notifications of **Monitor ready tasks** (Task-A03-stork103, approver 2026-11-09): a task
//! waiting on the person is shown under the app's own name and icon (「换肩」), and a click brings
//! the app forward. The app shows them, not the script editor (`osascript`), which made the
//! notification its own and opened it on a click.
//!
//! A run (`alm monitor run`, which may outlive the app) leaves each notification as a note in the
//! monitors folder ([`allomos_monitor::registry::leave_note`]); this thread takes the notes about
//! every two seconds and shows them. Notes left while the app is not running wait until it runs.

use std::time::Duration;

use allomos_monitor::registry;
use tauri::{AppHandle, Runtime};
use tauri_plugin_notification::NotificationExt;

/// How often the notes are looked for.
const EVERY: Duration = Duration::from_secs(2);

/// Starts the thread that shows the notes for as long as the app runs.
pub fn start<R: Runtime>(app: AppHandle<R>) {
    std::thread::spawn(move || {
        loop {
            show_waiting(&app);
            std::thread::sleep(EVERY);
        }
    });
}

/// Shows each note left in the monitors folder, oldest first, and removes it.
fn show_waiting<R: Runtime>(app: &AppHandle<R>) {
    let Some(dir) = registry::dir() else {
        return;
    };
    for note in registry::take_notes(&dir) {
        // A notification that cannot be shown is left out (the status bar says the same thing).
        let _ = app
            .notification()
            .builder()
            .title(note.title)
            .body(note.body)
            .show();
    }
}
