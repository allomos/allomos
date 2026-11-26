//! Whether new tasks wait for the approver, in the app (Task-A00-wheat560): the Tasks section of
//! the taskbag's settings shows and changes `project.json`'s `new_tasks`, the same setting
//! `alm new-task-settings` shows and changes. On, an agent leaves a task it added in backlog until
//! the approver agrees (except in a goal run); alm words its next steps by it and stops nothing.

use std::path::PathBuf;

use allomos_core::Problem;
use allomos_core::new_tasks;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct NewTaskSettings {
    wait_for_approval: bool,
}

fn of_project(project: &allomos_core::Project) -> NewTaskSettings {
    NewTaskSettings {
        wait_for_approval: new_tasks::wait_for_approval(project),
    }
}

#[tauri::command(async)]
pub(crate) fn new_task_settings(root: PathBuf) -> Result<NewTaskSettings, Problem> {
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

/// Turn it on or off, as `alm new-task-settings --wait-for-approval on|off` does. Returns the
/// settings as saved; nothing is written when they already say that.
#[tauri::command(async)]
pub(crate) fn set_new_task_settings(
    root: PathBuf,
    wait_for_approval: bool,
) -> Result<NewTaskSettings, Problem> {
    let lock = allomos_store::Lock::acquire(&root, std::time::Duration::from_secs(5))?;
    let loaded = lock.load();
    let bag = loaded
        .taskbag
        .ok_or_else(|| loaded.issues[0].problem.clone())?;
    let project = new_tasks::set_wait_for_approval(&bag.project, wait_for_approval);
    if project != bag.project {
        let event = allomos_store::Actor::APP.event(
            "new-task-settings",
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
    fn the_setting_is_shown_and_turned_on_and_off_like_alm_new_task_settings() {
        let dir =
            std::env::temp_dir().join(format!("allomos-desktop-new-tasks-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join(allomos_store::DEFAULT_DIR);
        allomos_store::init(&root, "p", "P").unwrap();

        assert!(new_task_settings(root.clone()).unwrap().wait_for_approval);
        assert!(
            !set_new_task_settings(root.clone(), false)
                .unwrap()
                .wait_for_approval
        );
        assert!(!new_task_settings(root.clone()).unwrap().wait_for_approval);
        assert!(
            set_new_task_settings(root.clone(), true)
                .unwrap()
                .wait_for_approval
        );
        assert!(new_task_settings(root.clone()).unwrap().wait_for_approval);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
