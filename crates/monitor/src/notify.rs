//! A notification for the person (Task-A00-eclair542; Task-A03-stork103, approver 2026-11-09).
//!
//! The run does not show a system notification itself: `osascript` would make it the script
//! editor's, and a click would open the script editor. So the run leaves the notification as a note
//! in the monitors folder ([`registry::leave_note`]); the app, which shows it under its own name and
//! icon with the notification plugin, takes it from there. With no app running, the note waits
//! there until the app next runs. Never fails: a note that cannot be left is left out (the status
//! bar says the same thing).

use std::path::Path;

use crate::registry::{self, Note};

/// Leaves `body` under `title` for the app to show, from run `id` in `dir`.
pub fn show(dir: &Path, id: &str, title: &str, body: &str) {
    let _ = registry::leave_note(
        dir,
        id,
        &Note {
            title: title.to_string(),
            body: body.to_string(),
        },
    );
}
