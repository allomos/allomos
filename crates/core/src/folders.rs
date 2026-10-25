//! The project folders a task belongs to (decision 0019, Task-A01-dime494): checking the names
//! a command gives, changing a task's folders, and upgrading an `allomos/5` taskbag, which only a
//! person asks for (approver, 2026-10-25).

use crate::model::{Project, ProjectFolder, Task, Taskbag, folder_name_from, format_version};
use crate::ops::{Outcome, check_holder, find_task};
use crate::problem::Problem;

/// The taskbag's folders as a command lists them: `allomos (.), allomos-site (../site)`; `none`
/// when there are none.
pub fn names(project: &Project) -> String {
    if project.project_folders.is_empty() {
        return "none".into();
    }
    project
        .project_folders
        .iter()
        .map(|f| format!("{} ({})", f.name, f.path))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The folder names a command gives, checked: the taskbag must be upgraded, and each name one
/// of its folders. Duplicates are dropped; the order given is kept.
pub fn check(project: &Project, given: &[String]) -> Result<Vec<String>, Problem> {
    if given.is_empty() {
        return Ok(Vec::new());
    }
    if !project.upgraded() {
        return Err(Problem::UpgradeFirst {
            found: project.format.clone(),
            current: crate::model::FORMAT.to_string(),
        });
    }
    let mut out: Vec<String> = Vec::new();
    for name in given.iter().map(|n| n.trim()) {
        if !project.project_folders.iter().any(|f| f.name == name) {
            return Err(Problem::FolderUnknown {
                folder: name.to_string(),
                folders: names(project),
            });
        }
        if !out.iter().any(|n| n == name) {
            out.push(name.to_string());
        }
    }
    Ok(out)
}

/// Set the folders task `id` belongs to (`alm folders`); empty: none in particular.
pub fn set(
    taskbag: &Taskbag,
    id: &str,
    me: Option<&str>,
    given: &[String],
) -> Result<Outcome, Problem> {
    let mut task = find_task(taskbag, id)?;
    check_holder(&task, me)?;
    let folders = check(&taskbag.project, given)?;
    task.folders = folders;
    Ok(Outcome::plain(task))
}

/// The tasks run inside folder `name` lists: those that name it (tasks with no folder are left
/// out there; approver, 2026-10-25).
pub fn in_folder<'a>(tasks: impl IntoIterator<Item = &'a Task>, name: &str) -> Vec<&'a Task> {
    tasks
        .into_iter()
        .filter(|t| t.folders.iter().any(|f| f == name))
        .collect()
}

/// The project as `alm upgrade` writes it: `allomos/6`, with `project_folder` moved into
/// `project_folders` as the first entry, named after the folder (`base`: the folder's own name).
/// `None` when it is already `allomos/6` or later.
pub fn upgraded(project: &Project, base: Option<&str>) -> Option<Project> {
    if format_version(&project.format).is_none_or(|n| n >= 6) {
        return None;
    }
    let mut next = project.clone();
    next.format = crate::model::FORMAT.to_string();
    if let Some(path) = next.project_folder.take().filter(|p| !p.trim().is_empty()) {
        next.project_folders.insert(
            0,
            ProjectFolder {
                name: folder_name_from(base.unwrap_or("main")),
                path,
                extra: Default::default(),
            },
        );
    }
    Some(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(format: &str, folder: Option<&str>) -> Project {
        serde_json::from_value(serde_json::json!({
            "format": format, "id": "p", "name": "P", "project_folder": folder,
        }))
        .unwrap()
    }

    #[test]
    fn upgrading_moves_the_project_folder_into_the_list() {
        let old = project("allomos/5", Some("."));
        assert!(!old.upgraded());
        assert_eq!(old.main_folder(), Some("."));
        let new = upgraded(&old, Some("Allomos Site")).unwrap();
        assert!(new.upgraded());
        assert_eq!(new.format, "allomos/6");
        assert_eq!(new.project_folder, None);
        assert_eq!(new.project_folders[0].name, "allomos-site");
        assert_eq!(new.main_folder(), Some("."));
        assert!(upgraded(&new, None).is_none(), "once only");
        let bare = upgraded(&project("allomos/5", None), None).unwrap();
        assert!(bare.project_folders.is_empty());
    }

    #[test]
    fn folder_names_are_checked() {
        let old = project("allomos/5", Some("."));
        assert_eq!(check(&old, &[]), Ok(vec![]));
        assert_eq!(
            check(&old, &["main".into()]).unwrap_err().code(),
            "upgrade_first"
        );
        let new = upgraded(&old, Some("app")).unwrap();
        assert_eq!(
            check(&new, &["app".into(), "app".into()]),
            Ok(vec!["app".into()])
        );
        let err = check(&new, &["site".into()]).unwrap_err();
        assert_eq!(
            err,
            Problem::FolderUnknown {
                folder: "site".into(),
                folders: "app (.)".into()
            }
        );
        assert_eq!(names(&new), "app (.)");
        assert_eq!(folder_name_from("!!"), "main");
        assert_eq!(folder_name_from("My App_2"), "my-app-2");
    }
}
