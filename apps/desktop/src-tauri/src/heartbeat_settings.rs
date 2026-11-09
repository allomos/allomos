//! The project's heartbeat settings in the app (Task-A01-osprey500, decision 0022 § 3): the
//! Heartbeat section of the taskbag's settings shows and changes `project.json`'s `heartbeat`,
//! the same two numbers `alm heartbeat-settings` shows and changes (Task-A01-zero387).

use std::path::{Path, PathBuf};

use allomos_core::Problem;
use allomos_core::heartbeat::{self, SettingsChange};
use serde::Serialize;

/// The two numbers as written in `project.json` (missing: the default applies) and as they apply,
/// in whole minutes.
#[derive(Debug, Serialize)]
pub(crate) struct HeartbeatSettings {
    /// As written; `None` where the default applies.
    interval_minutes: Option<u32>,
    lost_after_minutes: Option<u32>,
    /// With the defaults filled in.
    applied_interval_minutes: i64,
    applied_lost_after_minutes: i64,
    /// The defaults (10 and 30), so the app shows them without a copy of its own.
    default_interval_minutes: i64,
    default_lost_after_minutes: i64,
}

fn settings_of(root: &Path) -> Result<HeartbeatSettings, Problem> {
    let loaded = allomos_store::load(root);
    let Some(bag) = loaded.taskbag else {
        return Err(loaded.issues.first().map(|i| i.problem.clone()).unwrap_or(
            Problem::NotATaskbag {
                path: root.display().to_string(),
            },
        ));
    };
    Ok(of_project(&bag.project))
}

fn of_project(project: &allomos_core::Project) -> HeartbeatSettings {
    let written = project.heartbeat.clone().unwrap_or_default();
    let applied = heartbeat::applied(project);
    HeartbeatSettings {
        interval_minutes: written.interval_minutes.filter(|m| *m > 0),
        lost_after_minutes: written.lost_after_minutes.filter(|m| *m > 0),
        applied_interval_minutes: applied.interval_secs / 60,
        applied_lost_after_minutes: applied.lost_after_secs / 60,
        default_interval_minutes: heartbeat::INTERVAL_SECS / 60,
        default_lost_after_minutes: heartbeat::LOST_AFTER_SECS / 60,
    }
}

#[tauri::command]
pub(crate) fn heartbeat_settings(root: PathBuf) -> Result<HeartbeatSettings, Problem> {
    settings_of(&root)
}

/// Change the heartbeat settings, as `alm heartbeat-settings --interval --lost-after` does, or
/// put both back to their defaults (`clear`, as `--clear`). Returns the settings as saved; nothing
/// is written when they already say that.
#[tauri::command]
pub(crate) fn set_heartbeat_settings(
    root: PathBuf,
    interval: Option<String>,
    lost_after: Option<String>,
    clear: bool,
) -> Result<HeartbeatSettings, Problem> {
    let lock = allomos_store::Lock::acquire(&root, std::time::Duration::from_secs(5))?;
    let loaded = lock.load();
    let bag = loaded
        .taskbag
        .ok_or_else(|| loaded.issues[0].problem.clone())?;
    let change = SettingsChange {
        interval: interval.as_deref(),
        lost_after: lost_after.as_deref(),
        clear,
    };
    let project = heartbeat::set_settings(&bag.project, &change)?;
    if project != bag.project {
        let event = allomos_store::Actor::APP.event(
            "heartbeat-settings",
            allomos_core::event::Kind::Project,
            Some(&project.id),
            allomos_core::event::changes(Some(&bag.project), &project, &[]),
        );
        lock.record(&event, |l| l.save_project(&project))?;
    }
    Ok(of_project(&project))
}
