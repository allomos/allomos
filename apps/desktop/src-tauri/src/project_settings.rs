//! The taskbag's settings that are not their own section's module: the task language, the
//! progress ladder, the project folders, the project's name and the handoff rules.

use std::path::{Path, PathBuf};

use allomos_core::Problem;
use allomos_store::load;
use serde::Serialize;

use crate::agents;

/// The project's handoff rules (as written, and with the defaults filled in) (Task-A03-06), for
/// the Handoff section of the taskbag's settings (Settings → Handoff, Task-A05-yacht960).
#[derive(Serialize)]
pub(crate) struct HandoffSettings {
    rules: allomos_core::HandoffRules,
    applied: allomos_core::AppliedRules,
}

fn handoff_settings_of(root: &Path) -> Result<HandoffSettings, Problem> {
    let loaded = load(root);
    let Some(bag) = loaded.taskbag else {
        return Err(loaded.issues.first().map(|i| i.problem.clone()).unwrap_or(
            Problem::NotATaskbag {
                path: root.display().to_string(),
            },
        ));
    };
    let rules = bag.project.handoff.unwrap_or_default();
    Ok(HandoffSettings {
        applied: rules.applied(None),
        rules,
    })
}

/// Set the language tasks are written in, as `alm task-language` does (`ui` follows the interface;
/// Task-A05-yeti951).
#[tauri::command(async)]
pub(crate) fn set_task_language(root: PathBuf, value: String) -> Result<(), Problem> {
    let lock = allomos_store::Lock::acquire(&root, std::time::Duration::from_secs(5))?;
    let loaded = lock.load();
    let bag = loaded
        .taskbag
        .ok_or_else(|| loaded.issues[0].problem.clone())?;
    let project = allomos_core::ops::set_task_language(&bag, &value)?;
    let event = allomos_store::Actor::APP.event(
        "task-language",
        allomos_core::event::Kind::Project,
        Some(&project.id),
        allomos_core::event::changes(Some(&bag.project), &project, &[]),
    );
    lock.record(&event, |l| l.save_project(&project))
}

/// One step of the ladder as the Tasks section edits it: `from` is its name before (none for a new
/// step), `to` its name now.
#[derive(serde::Deserialize)]
pub(crate) struct LadderStep {
    from: Option<String>,
    to: String,
}

/// Change the project's own labels as the Tasks section edited them (Task-A05-yeti951; decision
/// 0020: the label table, whose five built-in labels stay as they are): each renamed label as
/// `alm rename label` (on every task that carries it), then the labels in their new order, added
/// and removed. Checked first, so nothing changes when any of it is refused: removing a label
/// tasks still carry names those tasks. Returns the whole table's names, built-in first.
#[tauri::command(async)]
pub(crate) fn change_ladder(root: PathBuf, steps: Vec<LadderStep>) -> Result<Vec<String>, Problem> {
    let loaded = allomos_store::load(&root);
    let bag = loaded
        .taskbag
        .ok_or_else(|| loaded.issues[0].problem.clone())?;
    let names: Vec<String> = steps.iter().map(|s| s.to.trim().to_string()).collect();
    if names.iter().any(|n| n.is_empty()) {
        return Err(Problem::LabelNameMissing);
    }
    if let Some(n) = names
        .iter()
        .enumerate()
        .find(|(i, n)| names[..*i].contains(n))
        .map(|(_, n)| n)
    {
        return Err(Problem::LabelExists { label: n.clone() });
    }
    let renames: Vec<(&str, &str)> = steps
        .iter()
        .filter_map(|s| {
            let from = s.from.as_deref()?;
            (from != s.to.trim()).then_some((from, s.to.trim()))
        })
        .collect();
    // A rename onto a name another step still has before its own rename would collide midway.
    if let Some((_, to)) = renames
        .iter()
        .find(|(_, to)| bag.project.labels.iter().any(|l| l.name == *to))
    {
        return Err(Problem::LabelExists {
            label: to.to_string(),
        });
    }
    let kept: Vec<&str> = steps.iter().filter_map(|s| s.from.as_deref()).collect();
    for label in bag
        .project
        .label_names()
        .iter()
        .filter(|l| !kept.contains(&l.as_str()) && !allomos_core::labels::is_built_in(l))
    {
        let tasks: Vec<&str> = bag
            .tasks
            .iter()
            .filter(|t| t.labels.contains(label))
            .map(|t| t.id.as_str())
            .collect();
        if !tasks.is_empty() {
            return Err(Problem::LabelInUse {
                label: label.clone(),
                tasks: tasks.join(", "),
            });
        }
    }
    let wait = std::time::Duration::from_secs(5);
    for (from, to) in renames {
        allomos_store::rename_label(&root, allomos_store::Actor::APP, wait, from, to)?;
    }
    let lock = allomos_store::Lock::acquire(&root, wait)?;
    let loaded = lock.load();
    let bag = loaded
        .taskbag
        .ok_or_else(|| loaded.issues[0].problem.clone())?;
    let project = allomos_core::labels::set_custom(&bag, &names)?;
    if project.label_names() != bag.project.label_names() {
        let event = allomos_store::Actor::APP.event(
            "labels",
            allomos_core::event::Kind::Project,
            Some(&project.id),
            allomos_core::event::changes(Some(&bag.project), &project, &[]),
        );
        lock.record(&event, |l| l.save_project(&project))?;
    }
    Ok(project.label_names())
}

/// The taskbag's project folders (decision 0019), each with whether it is found here and how many
/// tasks name it, for the task pane's Folders and the task list (Task-A01-sketch327).
#[tauri::command(async)]
pub(crate) fn project_folders(
    root: PathBuf,
) -> Result<Vec<allomos_store::folders::FolderInfo>, Problem> {
    allomos_store::folders::list(&root)
}

/// Bind a project folder, as `alm folder add` does: recorded in `project.json` and Allomos's
/// section written into its AGENTS.md (Task-A05-plaza078). Returns the folders after.
#[tauri::command(async)]
pub(crate) fn bind_folder(
    root: PathBuf,
    path: PathBuf,
) -> Result<Vec<allomos_store::folders::FolderInfo>, Problem> {
    let wait = std::time::Duration::from_secs(5);
    allomos_store::folders::add(&root, allomos_store::Actor::APP, wait, &path, None)?;
    allomos_store::folders::list(&root)
}

/// Give a folder a new short name, as `alm folder rename` does; the folder on disk keeps its name.
#[tauri::command(async)]
pub(crate) fn rename_folder(
    root: PathBuf,
    name: String,
    new_name: String,
) -> Result<Vec<allomos_store::folders::FolderInfo>, Problem> {
    let wait = std::time::Duration::from_secs(5);
    allomos_store::folders::rename(&root, allomos_store::Actor::APP, wait, &name, &new_name)?;
    allomos_store::folders::list(&root)
}

/// Record where a folder that is not found is now, as `alm folder locate` does; no files move.
#[tauri::command(async)]
pub(crate) fn locate_folder(
    root: PathBuf,
    name: String,
    path: PathBuf,
) -> Result<Vec<allomos_store::folders::FolderInfo>, Problem> {
    let wait = std::time::Duration::from_secs(5);
    allomos_store::folders::locate(&root, allomos_store::Actor::APP, wait, &name, &path)?;
    allomos_store::folders::list(&root)
}

/// Unbind a folder, as `alm folder remove --confirm` does: its tasks stay, its files stay, and only
/// Allomos's section leaves its AGENTS.md. The dialog asks first.
#[tauri::command(async)]
pub(crate) fn unbind_folder(
    root: PathBuf,
    name: String,
) -> Result<Vec<allomos_store::folders::FolderInfo>, Problem> {
    let wait = std::time::Duration::from_secs(5);
    allomos_store::folders::remove(&root, allomos_store::Actor::APP, wait, &name)?;
    allomos_store::folders::list(&root)
}

/// Show one of the taskbag's bound folders in the file manager: only a folder bound to this
/// taskbag, never any path a request names.
#[tauri::command]
pub(crate) fn show_folder_in_file_manager(root: PathBuf, name: String) -> Result<(), Problem> {
    let folder = allomos_store::folders::list(&root)?
        .into_iter()
        .find(|f| f.name == name && f.exists)
        .ok_or_else(|| Problem::FolderUnknown {
            folder: name.clone(),
            folders: String::new(),
        })?;
    let program = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    };
    let mut command = agents::command(program);
    command.arg(allomos_store::plain_path(&folder.full));
    command.spawn().map(drop).map_err(|e| Problem::OpenFailed {
        path: folder.full.display().to_string(),
        detail: e.to_string(),
    })
}

/// Rename the project, as `alm project --name` does (Task-A05-camera758's settings, General;
/// Task-A05-chalk937). The ID stays. Returns the name as saved.
#[tauri::command(async)]
pub(crate) fn rename_project(root: PathBuf, name: String) -> Result<String, Problem> {
    let lock = allomos_store::Lock::acquire(&root, std::time::Duration::from_secs(5))?;
    let loaded = lock.load();
    let bag = loaded
        .taskbag
        .ok_or_else(|| loaded.issues[0].problem.clone())?;
    let project = allomos_core::ops::rename_project(&bag, Some(&name), None)?;
    let event = allomos_store::Actor::APP.event(
        "project",
        allomos_core::event::Kind::Project,
        Some(&project.id),
        allomos_core::event::changes(Some(&bag.project), &project, &[]),
    );
    lock.record(&event, |l| l.save_project(&project))?;
    Ok(project.name)
}

#[tauri::command(async)]
pub(crate) fn handoff_settings(root: PathBuf) -> Result<HandoffSettings, Problem> {
    handoff_settings_of(&root)
}

/// Change the handoff rules, as `alm handoff-rules` does: the project's own (no `agent`), or one
/// agent's or account's override. Returns the settings as saved.
#[tauri::command(async)]
pub(crate) fn set_handoff_rules(
    root: PathBuf,
    quota: Option<String>,
    context: Option<String>,
    write_when: Option<String>,
    agent: Option<String>,
    clear: bool,
) -> Result<HandoffSettings, Problem> {
    let lock = allomos_store::Lock::acquire(&root, std::time::Duration::from_secs(5))?;
    let loaded = lock.load();
    let bag = loaded
        .taskbag
        .ok_or_else(|| loaded.issues[0].problem.clone())?;
    let change = allomos_core::ops::HandoffChange {
        quota_gate: quota.as_deref(),
        context_gate: context.as_deref(),
        write_when: write_when.as_deref(),
        agent: agent.as_deref(),
        clear,
    };
    let project = allomos_core::ops::set_handoff_rules(&bag, &change)?;
    let event = allomos_store::Actor::APP.event(
        "handoff-rules",
        allomos_core::event::Kind::Project,
        Some(&project.id),
        allomos_core::event::changes(Some(&bag.project), &project, &[]),
    );
    lock.record(&event, |l| l.save_project(&project))?;
    drop(lock);
    handoff_settings_of(&root)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Tasks section's ladder (Task-A05-yeti951): renames reach the tasks, order and new steps
    /// are saved, and removing a step tasks carry is refused, naming them, with nothing changed.
    #[test]
    fn the_ladder_changes_as_edited() {
        let dir =
            std::env::temp_dir().join(format!("allomos-desktop-ladder-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join(allomos_store::DEFAULT_DIR);
        allomos_store::init(&root, "p", "P").unwrap();
        let wait = std::time::Duration::from_secs(5);
        allomos_store::add_area(&root, allomos_store::Actor::APP, wait, None, "A").unwrap();
        let new: allomos_store::NewTask =
            serde_json::from_value(serde_json::json!({ "area": "A01", "title": "T" })).unwrap();
        let task = allomos_store::add_task(&root, allomos_store::Actor::APP, wait, &new)
            .unwrap()
            .item;
        let file = root.join("tasks").join(&task.id).join("task.json");
        let mut v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        v["labels"] = serde_json::json!(["implemented"]);
        std::fs::write(&file, v.to_string()).unwrap();
        let step = |from: Option<&str>, to: &str| LadderStep {
            from: from.map(String::from),
            to: to.into(),
        };
        let ladder = || {
            allomos_store::load(&root)
                .taskbag
                .unwrap()
                .project
                .label_names()
        };
        let before = ladder();
        // Removing "implemented", which the task carries: refused, naming the task.
        let err = change_ladder(root.clone(), vec![step(None, "spare")]).unwrap_err();
        assert_eq!(
            err,
            Problem::LabelInUse {
                label: "implemented".into(),
                tasks: task.id.clone()
            }
        );
        assert_eq!(ladder(), before);
        // Rename it and add one; the built-in labels stay first.
        let saved = change_ladder(
            root.clone(),
            vec![step(Some("implemented"), "built"), step(None, "reviewed")],
        )
        .unwrap();
        assert_eq!(saved.len(), 7, "{saved:?}");
        assert_eq!(saved[0], "locally verified by agent");
        assert_eq!(&saved[5..], ["built", "reviewed"]);
        assert_eq!(ladder(), saved);
        let bag = allomos_store::load(&root).taskbag.unwrap();
        assert_eq!(bag.tasks[0].labels, ["built"]);
        // Two steps with one name, and an empty name, are refused.
        assert!(change_ladder(root.clone(), vec![step(None, "x"), step(None, "x")]).is_err());
        assert!(change_ladder(root.clone(), vec![step(Some("built"), " ")]).is_err());
        // The task language.
        set_task_language(root.clone(), "ja".into()).unwrap();
        let bag = allomos_store::load(&root).taskbag.unwrap();
        assert_eq!(bag.project.task_language.as_deref(), Some("ja"));
        assert!(set_task_language(root.clone(), "not a language".into()).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
