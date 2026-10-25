//! A taskbag's project folders (decision 0019, Task-A05-taco639): binding a folder, renaming it,
//! locating it again, and unbinding it. Binding writes Allomos's section, which points back to the
//! taskbag from that folder, into its AGENTS.md (and a CLAUDE.md pointer); unbinding removes only
//! that section. The tasks are kept: unbinding takes the folder's name off them (approver,
//! 2026-10-25). Only an `allomos/6` taskbag has folders (`alm upgrade`).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use allomos_core::event::{self, Kind};
use allomos_core::{Problem, Project, ProjectFolder, Task, folder_name_from, valid_folder_name};
use serde::Serialize;

use crate::{Actor, Lock, allomos_folder, portable_path, relative_path};

/// One bound folder, as `alm folder list` shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FolderInfo {
    pub name: String,
    /// As `project.json` records it.
    pub path: String,
    /// Where that is on this computer.
    pub full: PathBuf,
    pub exists: bool,
    /// Its AGENTS.md tells agents about Allomos.
    pub section: bool,
    /// How many tasks name it, and how many of those are still open.
    pub tasks: usize,
    pub open: usize,
}

/// What a folder change did: the folder, and the instruction files it wrote or changed there
/// (for the person to save in that folder's own repository).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FolderChange {
    pub folder: FolderInfo,
    pub files: Vec<String>,
    /// Tasks whose folders changed (renamed or unbound).
    pub tasks: Vec<String>,
}

fn open(t: &Task) -> bool {
    !matches!(
        t.state,
        allomos_core::State::Done | allomos_core::State::Dropped
    )
}

fn tells(folder: &Path) -> bool {
    fs::read_to_string(folder.join("AGENTS.md")).is_ok_and(|t| crate::tells_agents(&t))
}

fn info(root: &Path, tasks: &[Task], f: &ProjectFolder) -> FolderInfo {
    let full = crate::setup::join_recorded(&allomos_folder(root), f.path.trim());
    let full = std::path::absolute(&full).unwrap_or(full);
    let naming: Vec<&Task> = tasks
        .iter()
        .filter(|t| t.folders.contains(&f.name))
        .collect();
    FolderInfo {
        name: f.name.clone(),
        path: f.path.clone(),
        exists: full.is_dir(),
        section: tells(&full),
        tasks: naming.len(),
        open: naming.iter().filter(|t| open(t)).count(),
        full,
    }
}

/// Every bound folder of the taskbag at `root`.
pub fn list(root: &Path) -> Result<Vec<FolderInfo>, Problem> {
    let loaded = crate::load(root);
    let (bag, _) = crate::taskbag_of(root, loaded)?;
    Ok(bag
        .project
        .project_folders
        .iter()
        .map(|f| info(root, &bag.tasks, f))
        .collect())
}

/// `path` as `project.json` records it: relative to the folder holding `.allomos/`, else absolute.
fn recorded(root: &Path, path: &Path) -> String {
    let path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let base = std::path::absolute(allomos_folder(root)).unwrap_or_else(|_| allomos_folder(root));
    match relative_path(&base, &path) {
        Some(r) => portable_path(&r),
        None => path.to_string_lossy().into_owned(),
    }
}

fn not_a_folder(path: &Path) -> Problem {
    Problem::StartupNotAFolder {
        path: path.display().to_string(),
    }
}

fn need_upgraded(project: &Project) -> Result<(), Problem> {
    if project.upgraded() {
        Ok(())
    } else {
        Err(Problem::UpgradeFirst {
            found: project.format.clone(),
            current: allomos_core::FORMAT.to_string(),
        })
    }
}

fn find<'a>(project: &'a Project, name: &str) -> Result<&'a ProjectFolder, Problem> {
    project
        .project_folders
        .iter()
        .find(|f| f.name == name)
        .ok_or_else(|| Problem::FolderUnknown {
            folder: name.to_string(),
            folders: allomos_core::folders::names(project),
        })
}

fn check_new_name(project: &Project, name: &str) -> Result<(), Problem> {
    if !valid_folder_name(name) {
        return Err(Problem::FolderNameInvalid { name: name.into() });
    }
    if project.project_folders.iter().any(|f| f.name == name) {
        return Err(Problem::FolderNameTaken { name: name.into() });
    }
    Ok(())
}

/// Write Allomos's section, pointing back to the taskbag at `root` from `folder`, into its
/// AGENTS.md (and CLAUDE.md, which only imports it when new). A file that already tells agents
/// about Allomos is left as it is. Returns the files written.
pub fn write_section(folder: &Path, root: &Path) -> io::Result<Vec<String>> {
    let section = crate::agent_section_for(folder, root);
    let mut changed = Vec::new();
    let agents = folder.join("AGENTS.md");
    match fs::read_to_string(&agents) {
        Ok(text) if crate::tells_agents(&text) => {}
        Ok(text) => {
            crate::append(&agents, &text, &section)?;
            changed.push("AGENTS.md".to_string());
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            fs::write(&agents, format!("# Agent instructions\n\n{section}"))?;
            changed.push("AGENTS.md".to_string());
        }
        Err(e) => return Err(e),
    }
    let claude = folder.join("CLAUDE.md");
    match fs::read_to_string(&claude) {
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            fs::write(&claude, "@AGENTS.md\n")?;
            changed.push("CLAUDE.md".to_string());
        }
        Err(e) => return Err(e),
    }
    Ok(changed)
}

/// `text` without Allomos's section: from its `## Tasks: Allomos` heading to the next heading of
/// that level or higher. `None` when it has no such heading.
pub fn without_section(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let start = lines
        .iter()
        .position(|l| l.starts_with("## Tasks: Allomos"))?;
    let end = lines[start + 1..]
        .iter()
        .position(|l| l.starts_with("## ") || l.starts_with("# "))
        .map_or(lines.len(), |n| start + 1 + n);
    let mut out: String = lines[..start].concat();
    let rest: String = lines[end..].concat();
    if rest.is_empty() {
        out = out.trim_end().to_string();
        if !out.is_empty() {
            out.push('\n');
        }
    } else {
        out.push_str(&rest);
    }
    Some(out)
}

/// Remove Allomos's section from `folder`'s AGENTS.md (and CLAUDE.md, if it holds one). Other
/// text, and the files themselves, stay. Returns the files changed.
pub fn remove_section(folder: &Path) -> io::Result<Vec<String>> {
    let mut changed = Vec::new();
    for name in ["AGENTS.md", "CLAUDE.md"] {
        let path = folder.join(name);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        if let Some(rest) = without_section(&text) {
            fs::write(&path, rest)?;
            changed.push(name.to_string());
        }
    }
    Ok(changed)
}

fn write_failed(path: &Path, e: io::Error) -> Problem {
    Problem::WriteFailed {
        path: path.display().to_string(),
        detail: e.to_string(),
    }
}

/// Save the project and the tasks that changed, each with its event.
fn save(
    lock: &Lock,
    actor: Actor,
    command: &str,
    before: &Project,
    after: &Project,
    tasks: &[(Task, Task)],
) -> Result<(), Problem> {
    let event = actor.event(
        command,
        Kind::Project,
        Some(&after.id),
        event::changes(Some(before), after, &[]),
    );
    lock.record(&event, |l| l.save_project(after))?;
    for (old, new) in tasks {
        let e = actor.event(
            command,
            Kind::Task,
            Some(&new.id),
            event::changes(Some(old), new, event::TASK_FIELDS),
        );
        lock.record(&e, |l| l.save_task(new))?;
    }
    Ok(())
}

/// Bind `path` as a project folder named `name` (default: from the folder's own name), and write
/// Allomos's section there.
pub fn add(
    root: &Path,
    actor: Actor,
    wait: Duration,
    path: &Path,
    name: Option<&str>,
) -> Result<FolderChange, Problem> {
    if !path.is_dir() {
        return Err(not_a_folder(path));
    }
    let (lock, bag, _) = crate::lock_for_change(root, wait)?;
    need_upgraded(&bag.project)?;
    let full = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let name = match name {
        Some(n) => n.trim().to_string(),
        None => folder_name_from(&full.file_name().unwrap_or_default().to_string_lossy()),
    };
    check_new_name(&bag.project, &name)?;
    let mut project = bag.project.clone();
    let folder = ProjectFolder {
        name,
        path: recorded(root, &full),
        extra: Default::default(),
    };
    project.project_folders.push(folder.clone());
    save(&lock, actor, "folder add", &bag.project, &project, &[])?;
    let files = write_section(&full, root).map_err(|e| write_failed(&full, e))?;
    Ok(FolderChange {
        folder: info(root, &bag.tasks, &folder),
        files,
        tasks: Vec::new(),
    })
}

/// Give a folder a new short name, in `project.json` and in every task that names it. The folder
/// on disk keeps its name.
pub fn rename(
    root: &Path,
    actor: Actor,
    wait: Duration,
    old: &str,
    new: &str,
) -> Result<FolderChange, Problem> {
    let (lock, bag, _) = crate::lock_for_change(root, wait)?;
    need_upgraded(&bag.project)?;
    find(&bag.project, old)?;
    let new = new.trim();
    check_new_name(&bag.project, new)?;
    let mut project = bag.project.clone();
    for f in project.project_folders.iter_mut().filter(|f| f.name == old) {
        f.name = new.to_string();
    }
    let tasks: Vec<(Task, Task)> = bag
        .tasks
        .iter()
        .filter(|t| t.folders.iter().any(|f| f == old))
        .map(|t| {
            let mut n = t.clone();
            for f in n.folders.iter_mut().filter(|f| *f == old) {
                *f = new.to_string();
            }
            (t.clone(), n)
        })
        .collect();
    save(
        &lock,
        actor,
        "folder rename",
        &bag.project,
        &project,
        &tasks,
    )?;
    let renamed: Vec<Task> = tasks.iter().map(|(_, n)| n.clone()).collect();
    let folder = find(&project, new)?.clone();
    Ok(FolderChange {
        folder: info(root, &renamed, &folder),
        files: Vec::new(),
        tasks: tasks.into_iter().map(|(_, n)| n.id).collect(),
    })
}

/// Record where folder `name` is now (it moved, or a teammate keeps it elsewhere). No files move;
/// when its AGENTS.md has no Allomos section, it is written.
pub fn locate(
    root: &Path,
    actor: Actor,
    wait: Duration,
    name: &str,
    path: &Path,
) -> Result<FolderChange, Problem> {
    if !path.is_dir() {
        return Err(not_a_folder(path));
    }
    let (lock, bag, _) = crate::lock_for_change(root, wait)?;
    need_upgraded(&bag.project)?;
    find(&bag.project, name)?;
    let full = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut project = bag.project.clone();
    for f in project
        .project_folders
        .iter_mut()
        .filter(|f| f.name == name)
    {
        f.path = recorded(root, &full);
    }
    save(&lock, actor, "folder locate", &bag.project, &project, &[])?;
    let files = write_section(&full, root).map_err(|e| write_failed(&full, e))?;
    let folder = find(&project, name)?.clone();
    Ok(FolderChange {
        folder: info(root, &bag.tasks, &folder),
        files,
        tasks: Vec::new(),
    })
}

/// What unbinding folder `name` would do, without doing it.
pub fn remove_preview(root: &Path, name: &str) -> Result<FolderInfo, Problem> {
    let loaded = crate::load(root);
    let (bag, _) = crate::taskbag_of(root, loaded)?;
    need_upgraded(&bag.project)?;
    let f = find(&bag.project, name)?;
    Ok(info(root, &bag.tasks, f))
}

/// Unbind folder `name`: off `project.json`, its name taken off every task (tasks are never
/// deleted or moved), and only Allomos's section removed from its AGENTS.md; its files stay.
pub fn remove(
    root: &Path,
    actor: Actor,
    wait: Duration,
    name: &str,
) -> Result<FolderChange, Problem> {
    let (lock, bag, _) = crate::lock_for_change(root, wait)?;
    need_upgraded(&bag.project)?;
    let folder = info(root, &bag.tasks, find(&bag.project, name)?);
    let mut project = bag.project.clone();
    project.project_folders.retain(|f| f.name != name);
    let tasks: Vec<(Task, Task)> = bag
        .tasks
        .iter()
        .filter(|t| t.folders.iter().any(|f| f == name))
        .map(|t| {
            let mut n = t.clone();
            n.folders.retain(|f| f != name);
            (t.clone(), n)
        })
        .collect();
    save(
        &lock,
        actor,
        "folder remove",
        &bag.project,
        &project,
        &tasks,
    )?;
    let files = if folder.exists {
        remove_section(&folder.full).map_err(|e| write_failed(&folder.full, e))?
    } else {
        Vec::new()
    };
    Ok(FolderChange {
        folder,
        files,
        tasks: tasks.into_iter().map(|(_, n)| n.id).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_section_leaves() {
        let text = "# Notes\n\nKeep this.\n\n## Tasks: Allomos (v26)\n\nalm next\n- rule\n\n## Build\n\nmake\n";
        assert_eq!(
            without_section(text).unwrap(),
            "# Notes\n\nKeep this.\n\n## Build\n\nmake\n"
        );
        let last = "# Agent instructions\n\n## Tasks: Allomos (v26)\n\nalm next\n";
        assert_eq!(without_section(last).unwrap(), "# Agent instructions\n");
        assert_eq!(without_section("# Notes\n"), None);
    }
}
