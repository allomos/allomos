//! A notification for the person (Task-A00-eclair542, approver 2026-11-09): Monitor ready tasks
//! tells them when it is stuck, because a task parked a question or no task is ready. On macOS it
//! is a system notification through `osascript` (`display notification`), with no new dependency;
//! elsewhere nothing is shown yet and the call does nothing.

/// Shows `body` under `title` as a system notification. Never fails: a notification that cannot
/// be shown is left out (the Monitor's status bar says the same thing).
pub fn show(title: &str, body: &str) {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "display notification {} with title {}",
            crate::agents::applescript_string(body),
            crate::agents::applescript_string(title)
        );
        let _ = crate::agents::command("/usr/bin/osascript")
            .args(["-e", &script])
            .output();
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (title, body);
    }
}
