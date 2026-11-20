//! The project's line limit for source files in the app (Task-A03-dapper008): the File size
//! section of the taskbag's settings shows and changes `project.json`'s `file_size`, the same
//! number `alm file-size-settings` shows and changes. The app keeps and shows it; it measures no
//! file (each project's own tests do, `alm guide file-size`).

use std::path::PathBuf;

use allomos_core::Problem;
use allomos_core::file_size;
use serde::Serialize;

/// The limit as written in `project.json` (`None`: the default applies) and as it applies.
#[derive(Debug, Serialize)]
pub(crate) struct FileSizeSettings {
    limit_lines: Option<u32>,
    applied_limit_lines: u32,
    /// The default (1000), so the app shows it without a copy of its own.
    default_limit_lines: u32,
}

fn of_project(project: &allomos_core::Project) -> FileSizeSettings {
    FileSizeSettings {
        limit_lines: project
            .file_size
            .as_ref()
            .and_then(|s| s.limit_lines)
            .filter(|n| *n > 0),
        applied_limit_lines: file_size::limit(project),
        default_limit_lines: file_size::DEFAULT_LIMIT,
    }
}

#[tauri::command(async)]
pub(crate) fn file_size_settings(root: PathBuf) -> Result<FileSizeSettings, Problem> {
    let loaded = allomos_store::load(&root);
    let Some(bag) = loaded.taskbag else {
        return Err(loaded.issues.first().map(|i| i.problem.clone()).unwrap_or(
            Problem::NotATaskbag {
                path: root.display().to_string(),
            },
        ));
    };
    Ok(of_project(&bag.project))
}

/// Change the line limit, as `alm file-size-settings --limit` does, or put it back to the
/// default (`clear`, as `--clear`). Returns the settings as saved; nothing is written when they
/// already say that.
#[tauri::command(async)]
pub(crate) fn set_file_size_settings(
    root: PathBuf,
    limit: Option<String>,
    clear: bool,
) -> Result<FileSizeSettings, Problem> {
    let lock = allomos_store::Lock::acquire(&root, std::time::Duration::from_secs(5))?;
    let loaded = lock.load();
    let bag = loaded
        .taskbag
        .ok_or_else(|| loaded.issues[0].problem.clone())?;
    let project = file_size::set_limit(&bag.project, limit.as_deref(), clear)?;
    if project != bag.project {
        let event = allomos_store::Actor::APP.event(
            "file-size-settings",
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
    fn the_limit_is_shown_changed_refused_and_put_back_like_alm_file_size_settings() {
        let dir =
            std::env::temp_dir().join(format!("allomos-desktop-file-size-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join(allomos_store::DEFAULT_DIR);
        allomos_store::init(&root, "p", "P").unwrap();

        let s = file_size_settings(root.clone()).unwrap();
        assert_eq!((s.limit_lines, s.applied_limit_lines), (None, 1000));
        assert_eq!(s.default_limit_lines, 1000);

        let s = set_file_size_settings(root.clone(), Some("1500".into()), false).unwrap();
        assert_eq!((s.limit_lines, s.applied_limit_lines), (Some(1500), 1500));
        assert_eq!(
            file_size_settings(root.clone())
                .unwrap()
                .applied_limit_lines,
            1500
        );

        for bad in ["0", "abc", "1.5"] {
            let e = set_file_size_settings(root.clone(), Some(bad.into()), false).unwrap_err();
            assert!(matches!(e, Problem::FileSizeLimitInvalid { .. }), "{bad}");
        }
        assert_eq!(
            file_size_settings(root.clone())
                .unwrap()
                .applied_limit_lines,
            1500
        );

        let s = set_file_size_settings(root.clone(), None, true).unwrap();
        assert_eq!((s.limit_lines, s.applied_limit_lines), (None, 1000));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
