//! Reads and creates seabags on disk.
//!
//! A *seabag* is a folder holding `project.json`, `areas/<id>.json` and
//! `tasks/<id>/task.json` (+ `description.md`). See `docs/product/FILE-FORMAT.md`.
//! This crate never runs `git`: a seabag inside a repository is versioned by that
//! repository, and nothing here creates a nested one.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use fliphopper_core::{
    Area, FORMAT, Issue, Loaded, Problem, Project, Seabag, Task, parse_area, parse_project,
    parse_task, validate_seabag,
};
use serde::Serialize;

/// Where a project's seabag lives inside the folder the user picked.
/// Always used for a new seabag, also in an empty folder, so every tool looks in one place.
pub const DEFAULT_DIR: &str = ".fliphopper/seabag";
const PROJECT_FILE: &str = "project.json";

/// What a folder the user picked turns out to be.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Inspection {
    /// The folder is a seabag, or contains one in `.fliphopper/seabag/`.
    Found { root: PathBuf },
    /// The folder is empty: a starter seabag can be created in its `.fliphopper/seabag/`.
    Empty {
        folder: PathBuf,
        suggested_root: PathBuf,
    },
    /// The folder has other content but no seabag: one can be created in `.fliphopper/seabag/`.
    NoSeabag {
        folder: PathBuf,
        suggested_root: PathBuf,
    },
    /// The path does not exist or is not a folder.
    NotAFolder { path: PathBuf },
}

/// Work out what `folder` is, without changing anything.
pub fn inspect(folder: &Path) -> Inspection {
    if !folder.is_dir() {
        return Inspection::NotAFolder {
            path: folder.to_path_buf(),
        };
    }
    if folder.join(PROJECT_FILE).is_file() {
        return Inspection::Found {
            root: folder.to_path_buf(),
        };
    }
    let nested = folder.join(DEFAULT_DIR);
    if nested.join(PROJECT_FILE).is_file() {
        return Inspection::Found { root: nested };
    }
    let is_empty = fs::read_dir(folder)
        .map(|mut it| it.next().is_none())
        .unwrap_or(false);
    if is_empty {
        Inspection::Empty {
            folder: folder.to_path_buf(),
            suggested_root: nested,
        }
    } else {
        Inspection::NoSeabag {
            folder: folder.to_path_buf(),
            suggested_root: nested,
        }
    }
}

/// The nearest folder at or above `path` that contains `.git`, if any (shown to the user;
/// never used to run Git here).
pub fn git_root(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find(|p| p.join(".git").exists())
        .map(Path::to_path_buf)
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn read(root: &Path, path: &Path, issues: &mut Vec<Issue>) -> Option<String> {
    match fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(e) => {
            issues.push(Issue::new(
                &rel(root, path),
                None,
                Problem::ReadFailed {
                    detail: e.to_string(),
                },
            ));
            None
        }
    }
}

/// Sorted entries of a folder; a missing folder is simply empty.
fn sorted_entries(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(dir)
        .map(|it| it.filter_map(|e| e.ok().map(|e| e.path())).collect())
        .unwrap_or_default();
    v.sort();
    v
}

/// Load a seabag. Valid parts are returned even when other files have problems,
/// so the user sees their tasks plus a list of what to fix.
pub fn load(root: &Path) -> Loaded {
    let mut issues = Vec::new();
    let project_path = root.join(PROJECT_FILE);
    let project = match read(root, &project_path, &mut issues) {
        Some(text) => match parse_project(PROJECT_FILE, &text) {
            Ok(p) => Some(p),
            Err(mut e) => {
                issues.append(&mut e);
                None
            }
        },
        None => None,
    };
    let Some(project) = project else {
        return Loaded {
            seabag: None,
            issues,
        };
    };

    let mut areas: Vec<(String, Area)> = Vec::new();
    for path in sorted_entries(&root.join("areas")) {
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let file = rel(root, &path);
        if let Some(text) = read(root, &path, &mut issues) {
            match parse_area(&file, &text) {
                Ok(a) => areas.push((file, a)),
                Err(mut e) => issues.append(&mut e),
            }
        }
    }

    let mut tasks: Vec<(String, Task)> = Vec::new();
    for dir in sorted_entries(&root.join("tasks")) {
        if !dir.is_dir() {
            continue;
        }
        let path = dir.join("task.json");
        let file = rel(root, &path);
        if !path.is_file() {
            issues.push(Issue::new(&file, None, Problem::TaskFileMissing));
            continue;
        }
        if let Some(text) = read(root, &path, &mut issues) {
            match parse_task(&file, &text) {
                Ok(mut t) => {
                    t.description =
                        fs::read_to_string(dir.join("description.md")).unwrap_or_default();
                    tasks.push((file, t));
                }
                Err(mut e) => issues.append(&mut e),
            }
        }
    }

    issues.extend(validate_seabag(&project, &areas, &tasks));
    let mut areas: Vec<Area> = areas.into_iter().map(|(_, a)| a).collect();
    areas.sort_by(|a, b| a.order.cmp(&b.order).then(a.id.cmp(&b.id)));
    let tasks = tasks.into_iter().map(|(_, t)| t).collect();
    Loaded {
        seabag: Some(Seabag {
            project,
            areas,
            tasks,
        }),
        issues,
    }
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut text = serde_json::to_string_pretty(value).map_err(io::Error::other)?;
    text.push('\n');
    fs::write(path, text)
}

/// The JSON stored in `task.json`: the task without its description (that lives in
/// `description.md`) and without empty optional fields.
fn task_file_json(task: &Task) -> serde_json::Value {
    let mut v = serde_json::to_value(task).expect("a task always serializes");
    if let Some(map) = v.as_object_mut() {
        map.remove("description");
        map.retain(|_, value| !value.is_null());
    }
    v
}

/// Create a small starter project at `root`. `root` must not exist yet, or be an empty folder.
/// Returns `root`.
pub fn create_starter(root: &Path, name: &str) -> io::Result<PathBuf> {
    if root.exists() && fs::read_dir(root)?.next().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} is not empty", root.display()),
        ));
    }
    fs::create_dir_all(root)?;
    let project = Project {
        format: FORMAT.to_string(),
        id: "project".to_string(),
        name: name.to_string(),
        labels: fliphopper_core::default_labels(),
    };
    write_json(&root.join(PROJECT_FILE), &project)?;
    let areas = [("A01", "Getting started", 1), ("A02", "First feature", 2)];
    for (id, name, order) in areas {
        write_json(
            &root.join("areas").join(format!("{id}.json")),
            &Area {
                id: id.into(),
                name: name.into(),
                order,
            },
        )?;
    }
    let tasks = [
        (
            "A01-01",
            "Open this seabag in FlipHopper",
            "A01",
            fliphopper_core::State::Done,
            Some("owner verified"),
            "You picked a folder and FlipHopper created this starter project in it.\n",
        ),
        (
            "A01-02",
            "Rename the areas and tasks to your project's",
            "A01",
            fliphopper_core::State::Ready,
            None,
            "Edit the files in this folder, or use the `fh` command once it is available.\n",
        ),
        (
            "A02-01",
            "Describe the first outcome you want",
            "A02",
            fliphopper_core::State::Backlog,
            None,
            "One sentence about what a person can do when this task is done.\n",
        ),
    ];
    for (id, title, area, state, label, description) in tasks {
        let dir = root.join("tasks").join(id);
        let task = Task {
            id: id.into(),
            title: title.into(),
            area: area.into(),
            state,
            label: label.map(String::from),
            holder: None,
            waiting_on: None,
            next_dependency: None,
            description: String::new(),
        };
        write_json(&dir.join("task.json"), &task_file_json(&task))?;
        fs::write(dir.join("description.md"), description)?;
    }
    Ok(root.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp(name: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!("fliphopper-test-{name}-{n}"));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn empty_folder_gets_starter_seabag_in_default_dir() {
        let dir = tmp("empty");
        let root = dir.join(".fliphopper/seabag");
        assert_eq!(
            inspect(&dir),
            Inspection::Empty {
                folder: dir.clone(),
                suggested_root: root.clone()
            }
        );
        create_starter(&root, "Demo").unwrap();
        assert_eq!(inspect(&dir), Inspection::Found { root: root.clone() });
        let loaded = load(&root);
        assert_eq!(
            loaded.seabag.as_ref().unwrap().project.format,
            "fliphopper/1"
        );
        assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
        let tb = loaded.seabag.unwrap();
        assert_eq!(tb.project.name, "Demo");
        assert_eq!(tb.areas.len(), 2);
        assert_eq!(tb.tasks.len(), 3);
        assert!(tb.tasks[0].description.contains("starter"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn repo_folder_gets_nested_seabag_without_nested_git() {
        let repo = tmp("repo");
        fs::create_dir(repo.join(".git")).unwrap();
        fs::write(repo.join("README.md"), "code").unwrap();
        let Inspection::NoSeabag { suggested_root, .. } = inspect(&repo) else {
            panic!("expected NoSeabag")
        };
        assert_eq!(suggested_root, repo.join(".fliphopper/seabag"));
        create_starter(&suggested_root, "Repo").unwrap();
        assert_eq!(
            inspect(&repo),
            Inspection::Found {
                root: repo.join(".fliphopper/seabag")
            }
        );
        assert!(!suggested_root.join(".git").exists());
        assert_eq!(git_root(&suggested_root), Some(repo.clone()));
        fs::remove_dir_all(repo).unwrap();
    }

    #[test]
    fn refuses_to_overwrite_a_non_empty_folder() {
        let dir = tmp("full");
        fs::write(dir.join("notes.txt"), "keep me").unwrap();
        assert!(create_starter(&dir, "X").is_err());
        assert_eq!(
            fs::read_to_string(dir.join("notes.txt")).unwrap(),
            "keep me"
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn invalid_file_is_reported_and_valid_tasks_still_load() {
        let dir = tmp("broken");
        create_starter(&dir, "Demo").unwrap();
        fs::write(
            dir.join("tasks/A01-02/task.json"),
            r#"{"id":"A01-02","area":"A01","state":"ready"}"#,
        )
        .unwrap();
        let loaded = load(&dir);
        assert_eq!(loaded.issues.len(), 1);
        assert_eq!(loaded.issues[0].file, "tasks/A01-02/task.json");
        assert_eq!(loaded.issues[0].field.as_deref(), Some("title"));
        assert_eq!(loaded.seabag.unwrap().tasks.len(), 2);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_project_file_is_an_issue_not_a_crash() {
        let dir = tmp("noproject");
        let loaded = load(&dir);
        assert!(loaded.seabag.is_none());
        assert_eq!(loaded.issues[0].file, "project.json");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn not_a_folder() {
        let p = tmp("nf").join("missing");
        assert!(matches!(inspect(&p), Inspection::NotAFolder { .. }));
    }
}
