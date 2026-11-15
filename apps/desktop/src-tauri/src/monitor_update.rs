//! After an update, the runs move to the app's new `alm` (Task-A03-cookie729, approver
//! 2026-11-16): when the app starts, each run that an older version of its own `alm` runs is ended
//! and started again with the new one, from its saved settings, with the sessions it had at work.
//! It does not ask; the resumed run tells the person (a notification, and its status bar line).
//! What is kept and what is left alone: [`allomos_monitor::update`].

use allomos_monitor::{registry, update};

/// Moves the outdated runs to the app's own `alm`; what went wrong goes to the app's log. Runs off
/// the main thread (it waits for the old processes to end).
pub fn take_over_outdated() {
    let (Some(dir), Some(own)) = (registry::dir(), crate::alm_command::own()) else {
        return;
    };
    let runs = update::outdated_runs(&dir, &own, env!("CARGO_PKG_VERSION"));
    if runs.is_empty() {
        return;
    }
    let mut ended = Vec::new();
    for run in runs {
        match update::end_for_resume(&dir, &run) {
            Ok(()) => ended.push(run),
            Err(e) => eprintln!("monitor {}: not moved to the new alm: {e}", run.id),
        }
    }
    for taskbag in update::taskbags(&ended) {
        let out = crate::agents::command(&own)
            .args(["--json", "--taskbag"])
            .arg(&taskbag)
            .args(["monitor", "resume"])
            .env_remove("ALLOMOS_TASKBAG")
            .output();
        match out {
            Ok(o) if o.status.success() => {}
            Ok(o) => eprintln!(
                "monitors of {}: not started again: {}",
                taskbag.display(),
                String::from_utf8_lossy(&o.stdout).trim()
            ),
            Err(e) => eprintln!("monitors of {}: not started again: {e}", taskbag.display()),
        }
    }
}
