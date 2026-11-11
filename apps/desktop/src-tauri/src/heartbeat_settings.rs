//! The project's heartbeat settings in the app (Task-A01-osprey500, decision 0022 § 3): the
//! Heartbeat section of the taskbag's settings shows and changes `project.json`'s `heartbeat`,
//! the same settings `alm heartbeat-settings` shows and changes (Task-A01-zero387): two numbers,
//! and whether work in progress is pushed at each heartbeat (Task-A01-butter895).

use std::path::{Path, PathBuf};

use allomos_core::Problem;
use allomos_core::heartbeat::{self, SettingsChange};
use serde::Serialize;

/// The settings as written in `project.json` (missing: the default applies) and as they apply, the
/// numbers in whole minutes.
#[derive(Debug, Serialize)]
pub(crate) struct HeartbeatSettings {
    /// As written; `None` where the default applies.
    interval_minutes: Option<u32>,
    lost_after_minutes: Option<u32>,
    push_work: Option<bool>,
    /// With the defaults filled in.
    applied_interval_minutes: i64,
    applied_lost_after_minutes: i64,
    applied_push_work: bool,
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
        push_work: written.push_work,
        applied_interval_minutes: applied.interval_secs / 60,
        applied_lost_after_minutes: applied.lost_after_secs / 60,
        applied_push_work: applied.push_work,
        default_interval_minutes: heartbeat::INTERVAL_SECS / 60,
        default_lost_after_minutes: heartbeat::LOST_AFTER_SECS / 60,
    }
}

#[tauri::command]
pub(crate) fn heartbeat_settings(root: PathBuf) -> Result<HeartbeatSettings, Problem> {
    settings_of(&root)
}

/// Change the heartbeat settings, as `alm heartbeat-settings --interval --lost-after --wip`
/// does, or put them all back to their defaults (`clear`, as `--clear`). Returns the settings as saved; nothing
/// is written when they already say that.
#[tauri::command]
pub(crate) fn set_heartbeat_settings(
    root: PathBuf,
    interval: Option<String>,
    lost_after: Option<String>,
    push_work: Option<bool>,
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
        push_work,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_settings_are_shown_changed_refused_and_put_back_like_alm_heartbeat_settings() {
        let dir =
            std::env::temp_dir().join(format!("allomos-desktop-heartbeat-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join(allomos_store::DEFAULT_DIR);
        allomos_store::init(&root, "p", "P").unwrap();

        // Nothing written: the defaults, 10 and 30 minutes.
        let s = heartbeat_settings(root.clone()).unwrap();
        assert_eq!((s.interval_minutes, s.lost_after_minutes), (None, None));
        assert_eq!(
            (s.applied_interval_minutes, s.applied_lost_after_minutes),
            (10, 30)
        );
        assert_eq!(
            (s.default_interval_minutes, s.default_lost_after_minutes),
            (10, 30)
        );

        // A change is saved in project.json and read back by the task list's rule.
        let s = set_heartbeat_settings(root.clone(), None, Some("15".into()), None, false).unwrap();
        assert_eq!((s.interval_minutes, s.lost_after_minutes), (None, Some(15)));
        let bag = allomos_store::load(&root).taskbag.unwrap();
        assert_eq!(heartbeat::applied(&bag.project).lost_after_secs, 15 * 60);

        // A value that is not whole minutes of at least 1 is refused; nothing changes.
        for bad in ["0", "1.5", "abc"] {
            let e = set_heartbeat_settings(root.clone(), Some(bad.into()), None, None, false)
                .unwrap_err();
            assert!(
                matches!(e, Problem::HeartbeatMinutesInvalid { .. }),
                "{bad}"
            );
        }
        assert_eq!(
            heartbeat_settings(root.clone()).unwrap().interval_minutes,
            None
        );

        // Pushing work in progress is on by default; turning it off is written, on again is not.
        let s = heartbeat_settings(root.clone()).unwrap();
        assert_eq!((s.push_work, s.applied_push_work), (None, true));
        let s = set_heartbeat_settings(root.clone(), None, None, Some(false), false).unwrap();
        assert_eq!((s.push_work, s.applied_push_work), (Some(false), false));
        let bag = allomos_store::load(&root).taskbag.unwrap();
        assert!(!heartbeat::applied(&bag.project).push_work);
        let s = set_heartbeat_settings(root.clone(), None, None, Some(true), false).unwrap();
        assert_eq!((s.push_work, s.applied_push_work), (None, true));
        set_heartbeat_settings(root.clone(), None, None, Some(false), false).unwrap();

        // Restore Defaults: all go, and `heartbeat` leaves project.json.
        let s = set_heartbeat_settings(root.clone(), None, None, None, true).unwrap();
        assert_eq!((s.interval_minutes, s.lost_after_minutes), (None, None));
        assert!(
            allomos_store::load(&root)
                .taskbag
                .unwrap()
                .project
                .heartbeat
                .is_none()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
