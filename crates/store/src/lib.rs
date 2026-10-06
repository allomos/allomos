//! Reads and creates seabags on disk.
//!
//! A *seabag* is a folder holding `project.json`, `areas/<id>.json` and
//! `tasks/<id>/task.json` (+ `description.md`). See `docs/product/FILE-FORMAT.md`.
//! This crate never runs `git`: a seabag inside a repository is versioned by that
//! repository, and nothing here creates a nested one.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use fs4::{FileExt, TryLockError};

use fliphopper_core::{
    Area, FORMAT, Issue, Loaded, Problem, Project, Seabag, Task, parse_area, parse_project,
    parse_task, validate_seabag,
};
use serde::Serialize;

/// Where a project's seabag lives inside the folder the user picked.
/// Always used for a new seabag, also in an empty folder, so every tool looks in one place.
pub const DEFAULT_DIR: &str = ".fliphopper/seabag";
const PROJECT_FILE: &str = "project.json";
/// The file every writer locks for the length of one change. Never committed.
pub const LOCK_FILE: &str = ".lock";
/// Prefix of the temporary files and folders a writer renames into place.
const TMP_PREFIX: &str = ".tmp-";
/// Written into every new seabag, so Git never sees FlipHopper's working files.
const GITIGNORE: &str = "# FlipHopper's own working files. Never commit them.\n.lock\n.tmp-*\n";

/// What a folder the user picked turns out to be.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Inspection {
    /// The folder is a seabag, or contains one in `.fliphopper/seabag/`.
    Found { root: PathBuf },
    /// The folder looks empty (nothing, or only hidden entries such as `.DS_Store` or `.git`):
    /// a starter seabag can be created in its `.fliphopper/seabag/` without asking.
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

/// Name of the short introduction FlipHopper leaves in a folder it fills from empty.
/// Never `README.md`, so it cannot clash with the user's own README (owner, 2026-10-06).
pub const README_FILE: &str = "README-fliphopper.md";

/// A folder looks empty when it has no entries, or only hidden ones (names starting with
/// `.`, such as `.DS_Store`, `.git` or `.fliphopper`). A folder that cannot be read does not.
pub fn looks_empty(folder: &Path) -> bool {
    fs::read_dir(folder)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .all(|e| e.file_name().to_string_lossy().starts_with('.'))
        })
        .unwrap_or(false)
}

/// The README's text in an interface language (`en`, `zh-CN`, `ja`); English otherwise.
pub fn readme_text(language: &str) -> &'static str {
    match language {
        "zh-CN" => README_ZH_CN,
        "ja" => README_JA,
        _ => README_EN,
    }
}

const README_EN: &str = "# FlipHopper project

This folder is a FlipHopper project. Its tasks, areas and progress are in
`.fliphopper/seabag/`, a hidden folder.

- Open this folder in the FlipHopper app to see the tasks by area.
- AI agents and people can also work the tasks from a terminal with the `fh` command
  (`fh list`, `fh next`, `fh claim <task>`).
- The files are plain JSON and Markdown, so Git can track every change.

FlipHopper wrote this file because the folder was empty. You may edit or delete it.
";

const README_ZH_CN: &str = "# FlipHopper 项目

此文件夹是一个 FlipHopper 项目。它的任务、领域和进度保存在隐藏文件夹
`.fliphopper/seabag/` 中。

- 在 FlipHopper 应用中打开此文件夹，即可按领域查看任务。
- AI 智能体和人也可以在终端中用 `fh` 命令处理任务
  （`fh list`、`fh next`、`fh claim <任务>`）。
- 这些文件是普通的 JSON 和 Markdown，Git 可以记录每一处更改。

此文件夹原本是空的，所以 FlipHopper 写了这个文件。你可以修改或删除它。
";

const README_JA: &str = "# FlipHopper プロジェクト

このフォルダーは FlipHopper のプロジェクトです。タスク、エリア、進捗は隠しフォルダー
`.fliphopper/seabag/` に入っています。

- FlipHopper アプリでこのフォルダーを開くと、タスクをエリアごとに見られます。
- AI エージェントや人は、ターミナルの `fh` コマンドでもタスクを扱えます
  （`fh list`、`fh next`、`fh claim <タスク>`）。
- ファイルはただの JSON と Markdown なので、Git ですべての変更を記録できます。

このフォルダーが空だったので、FlipHopper がこのファイルを書きました。編集や削除は自由です。
";

/// Write `README-fliphopper.md` into `folder` when the folder looks empty (see
/// [`looks_empty`]). Never overwrites a file. Returns whether the README was written.
pub fn add_readme_if_empty(folder: &Path, language: &str) -> io::Result<bool> {
    if !looks_empty(folder) {
        return Ok(false);
    }
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(folder.join(README_FILE))
    {
        Ok(mut file) => {
            file.write_all(readme_text(language).as_bytes())?;
            Ok(true)
        }
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(false),
        Err(e) => Err(e),
    }
}

/// The folder the user picked, for a seabag at `<folder>/.fliphopper/seabag`; `None` for a
/// seabag anywhere else.
pub fn picked_folder(root: &Path) -> Option<&Path> {
    if root.ends_with(DEFAULT_DIR) {
        root.parent()?.parent()
    } else {
        None
    }
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
    if looks_empty(folder) {
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

fn hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with('.'))
}

/// Sorted entries of a folder, without hidden ones (the lock file and a writer's
/// temporary files); a missing folder is simply empty.
fn sorted_entries(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(dir)
        .map(|it| {
            it.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| !hidden(p))
                .collect()
        })
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
    fs::write(root.join(".gitignore"), GITIGNORE)?;
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
            handoff_note: None,
            description: String::new(),
        };
        write_json(&dir.join("task.json"), &task_file_json(&task))?;
        fs::write(dir.join("description.md"), description)?;
    }
    Ok(root.to_path_buf())
}

/// Find the seabag for `start`: the nearest `.fliphopper/seabag/` at or above it,
/// the way Git finds its repository.
pub fn find_seabag(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .map(|p| p.join(DEFAULT_DIR))
        .find(|p| p.join(PROJECT_FILE).is_file())
}

/// A project ID made from a folder name: letters, digits, `-` and `_` kept, anything else
/// turned into `-`, at most 64 characters; `project` when nothing usable is left.
pub fn id_from_name(name: &str) -> String {
    let id: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .take(64)
        .collect();
    let id = id.trim_matches('-').to_string();
    if id.is_empty() {
        "project".to_string()
    } else {
        id
    }
}

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A name for a temporary file or folder next to `dir`'s contents, unique to this process.
fn tmp_path(dir: &Path, what: &str) -> PathBuf {
    let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    dir.join(format!("{TMP_PREFIX}{}-{n}-{what}", std::process::id()))
}

fn write_failed(path: &Path, e: impl std::fmt::Display) -> Problem {
    Problem::WriteFailed {
        path: path.display().to_string(),
        detail: e.to_string(),
    }
}

fn json_text<T: Serialize>(value: &T) -> String {
    let mut text = serde_json::to_string_pretty(value).expect("seabag values always serialize");
    text.push('\n');
    text
}

/// Write `path` whole: the text goes to a temporary file in the same folder, is flushed to
/// disk, and is then renamed over `path`. A reader sees either the old file or the new one,
/// never a part-written one.
fn write_atomic(path: &Path, text: &str) -> Result<(), Problem> {
    let dir = path.parent().expect("seabag files are inside a folder");
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let tmp = tmp_path(dir, name);
    let result = (|| {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(text.as_bytes())?;
        f.sync_all()?;
        fs::rename(&tmp, path)
    })();
    result.map_err(|e| {
        let _ = fs::remove_file(&tmp);
        write_failed(path, e)
    })
}

/// Fill a new folder with files, then rename it to `target` in one step, so a reader never
/// sees it half-made. Refused when `target` already exists.
fn create_dir_atomic(target: &Path, files: &[(&str, String)]) -> Result<(), Problem> {
    let parent = target.parent().expect("a target has a parent");
    let name = target.file_name().and_then(|n| n.to_str()).unwrap_or("dir");
    let tmp = tmp_path(parent, name);
    let result = (|| {
        fs::create_dir_all(&tmp)?;
        for (rel_path, text) in files {
            let path = tmp.join(rel_path);
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir)?;
            }
            let mut f = fs::File::create(&path)?;
            f.write_all(text.as_bytes())?;
            f.sync_all()?;
        }
        if target.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "already exists",
            ));
        }
        fs::rename(&tmp, target)
    })();
    result.map_err(|e| {
        let _ = fs::remove_dir_all(&tmp);
        write_failed(target, e)
    })
}

/// Create an empty seabag at `root` (normally `<repository>/.fliphopper/seabag/`): `project.json`,
/// empty `areas/` and `tasks/`, and a `.gitignore` for the lock. Refused when `root` exists.
/// Never creates a Git repository.
pub fn init(root: &Path, id: &str, name: &str) -> Result<PathBuf, Problem> {
    init_with_labels(root, id, name, fliphopper_core::default_labels())
}

/// [`init`] with the project's own label ladder, lowest step first. An empty ladder is
/// refused (`ladder_empty`), like an invalid ID.
pub fn init_with_labels(
    root: &Path,
    id: &str,
    name: &str,
    labels: Vec<String>,
) -> Result<PathBuf, Problem> {
    if root.exists() {
        return Err(Problem::SeabagExists {
            path: root.display().to_string(),
        });
    }
    let project = Project {
        format: FORMAT.to_string(),
        id: id.to_string(),
        name: name.to_string(),
        labels,
    };
    // Checked like any project file, so `init` never writes one that `load` refuses.
    let text = json_text(&project);
    if let Err(issues) = parse_project(PROJECT_FILE, &text) {
        return Err(issues[0].problem.clone());
    }
    let parent = root.parent().expect("a seabag root has a parent");
    fs::create_dir_all(parent).map_err(|e| write_failed(parent, e))?;
    create_dir_atomic(
        root,
        &[
            (PROJECT_FILE, text),
            (".gitignore", GITIGNORE.to_string()),
            ("areas/.keep", String::new()),
            ("tasks/.keep", String::new()),
        ],
    )
    .map_err(|p| match p {
        Problem::WriteFailed { .. } if root.exists() => Problem::SeabagExists {
            path: root.display().to_string(),
        },
        p => p,
    })?;
    Ok(root.to_path_buf())
}

/// The seabag's lock, held for one change: load, check the rules, write, then drop it.
/// Every write goes through a `Lock`, so two FlipHopper writers never change the seabag at
/// the same moment. The operating system releases the lock if the process dies.
pub struct Lock {
    root: PathBuf,
    _file: fs::File,
}

impl Lock {
    /// Take the lock, waiting up to `wait` for another writer to finish.
    pub fn acquire(root: &Path, wait: Duration) -> Result<Lock, Problem> {
        if !root.join(PROJECT_FILE).is_file() {
            return Err(Problem::SeabagNotFound {
                path: root.display().to_string(),
            });
        }
        let path = root.join(LOCK_FILE);
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .map_err(|e| write_failed(&path, e))?;
        let start = Instant::now();
        loop {
            match FileExt::try_lock(&file) {
                Ok(()) => {
                    return Ok(Lock {
                        root: root.to_path_buf(),
                        _file: file,
                    });
                }
                Err(TryLockError::WouldBlock) if start.elapsed() < wait => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(TryLockError::WouldBlock) => {
                    return Err(Problem::SeabagBusy {
                        path: root.display().to_string(),
                    });
                }
                Err(TryLockError::Error(e)) => return Err(write_failed(&path, e)),
            }
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Load the seabag while holding the lock, so what is read is what the change builds on.
    pub fn load(&self) -> Loaded {
        load(&self.root)
    }

    /// Write a new area file. Refused when the file already exists.
    pub fn create_area(&self, area: &Area) -> Result<(), Problem> {
        let dir = self.root.join("areas");
        fs::create_dir_all(&dir).map_err(|e| write_failed(&dir, e))?;
        let path = dir.join(format!("{}.json", area.id));
        if path.exists() {
            return Err(Problem::DuplicateAreaId {
                id: area.id.clone(),
            });
        }
        write_atomic(&path, &json_text(area))
    }

    /// Write a new task folder: `task.json`, plus `description.md` when there is a description.
    pub fn create_task(&self, task: &Task) -> Result<(), Problem> {
        let dir = self.root.join("tasks");
        fs::create_dir_all(&dir).map_err(|e| write_failed(&dir, e))?;
        let target = dir.join(&task.id);
        if target.exists() {
            return Err(Problem::DuplicateTaskId {
                id: task.id.clone(),
            });
        }
        let mut files = vec![("task.json", json_text(&task_file_json(task)))];
        if !task.description.is_empty() {
            files.push(("description.md", task.description.clone()));
        }
        create_dir_atomic(&target, &files)
    }

    /// Rewrite an existing task's `task.json`. Its description is left as it is.
    pub fn save_task(&self, task: &Task) -> Result<(), Problem> {
        let path = self.root.join("tasks").join(&task.id).join("task.json");
        write_atomic(&path, &json_text(&task_file_json(task)))
    }
}

/// A fingerprint of the seabag's files (names, sizes and change times). It changes when any
/// file is written, so a window can reload only when something changed.
pub fn stamp(root: &Path) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let mut add = |p: &Path| {
        if let Ok(m) = fs::metadata(p) {
            p.hash(&mut h);
            m.len().hash(&mut h);
            m.modified().ok().hash(&mut h);
        }
    };
    add(&root.join(PROJECT_FILE));
    for p in sorted_entries(&root.join("areas")) {
        add(&p);
    }
    for dir in sorted_entries(&root.join("tasks")) {
        add(&dir);
        for p in sorted_entries(&dir) {
            add(&p);
        }
    }
    h.finish()
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
    fn init_with_own_id_and_ladder() {
        let dir = tmp("init-ladder");
        let root = dir.join(DEFAULT_DIR);
        let ladder = vec!["drafted".to_string(), "owner verified".to_string()];
        init_with_labels(&root, "fliphopper", "FlipHopper", ladder.clone()).unwrap();
        let bag = load(&root).seabag.unwrap();
        assert_eq!(
            (bag.project.id.as_str(), bag.project.labels),
            ("fliphopper", ladder)
        );
        let other = dir.join("other");
        assert_eq!(
            init_with_labels(&other, "p", "P", vec![]),
            Err(Problem::LadderEmpty)
        );
        assert!(!other.exists());
        assert!(matches!(
            init_with_labels(&dir.join("x"), "bad id", "P", vec!["a".into()]),
            Err(Problem::InvalidId { .. })
        ));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn hidden_only_folder_looks_empty() {
        let dir = tmp("hidden-only");
        assert!(looks_empty(&dir));
        fs::write(dir.join(".DS_Store"), "x").unwrap();
        fs::create_dir(dir.join(".git")).unwrap();
        assert!(looks_empty(&dir));
        assert_eq!(
            inspect(&dir),
            Inspection::Empty {
                folder: dir.clone(),
                suggested_root: dir.join(DEFAULT_DIR)
            }
        );
        fs::write(dir.join("notes.txt"), "x").unwrap();
        assert!(!looks_empty(&dir));
        assert!(matches!(inspect(&dir), Inspection::NoSeabag { .. }));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn readme_goes_only_into_a_folder_that_looks_empty() {
        // Empty, and only `.DS_Store`: the seabag and the README both arrive.
        for extra in [None, Some(".DS_Store")] {
            let dir = tmp("readme-empty");
            if let Some(name) = extra {
                fs::write(dir.join(name), "x").unwrap();
            }
            let root = dir.join(DEFAULT_DIR);
            create_starter(&root, "Demo").unwrap();
            assert_eq!(picked_folder(&root), Some(dir.as_path()));
            assert!(add_readme_if_empty(&dir, "en").unwrap());
            let text = fs::read_to_string(dir.join(README_FILE)).unwrap();
            assert!(text.contains(".fliphopper/seabag/"));
            assert_eq!(inspect(&dir), Inspection::Found { root: root.clone() });
            // Now the folder has a visible file: a second call writes nothing.
            assert!(!add_readme_if_empty(&dir, "ja").unwrap());
            assert_eq!(fs::read_to_string(dir.join(README_FILE)).unwrap(), text);
            fs::remove_dir_all(dir).unwrap();
        }
        // A folder with a visible file gets no README.
        let dir = tmp("readme-code");
        fs::write(dir.join("main.rs"), "fn main() {}").unwrap();
        create_starter(&dir.join(DEFAULT_DIR), "Code").unwrap();
        assert!(!add_readme_if_empty(&dir, "en").unwrap());
        assert!(!dir.join(README_FILE).exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn readme_never_overwrites() {
        let dir = tmp("readme-keep");
        fs::create_dir(dir.join(".git")).unwrap();
        let path = dir.join(README_FILE);
        // An existing README is a visible file, so the folder no longer looks empty.
        fs::write(&path, "mine").unwrap();
        assert!(!add_readme_if_empty(&dir, "en").unwrap());
        assert_eq!(fs::read_to_string(&path).unwrap(), "mine");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn readme_speaks_three_languages() {
        assert!(readme_text("en").starts_with("# FlipHopper project"));
        assert!(readme_text("zh-CN").contains("此文件夹是一个 FlipHopper 项目"));
        assert!(readme_text("ja").contains("このフォルダーは FlipHopper のプロジェクトです"));
        assert_eq!(readme_text("fr"), readme_text("en"));
        for lang in ["en", "zh-CN", "ja"] {
            assert!(readme_text(lang).contains("`.fliphopper/seabag/`"));
            assert!(readme_text(lang).contains("`fh"));
        }
        assert_eq!(picked_folder(Path::new("/x/seabag")), None);
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

    #[test]
    fn init_makes_an_empty_seabag_found_from_below() {
        let repo = tmp("init");
        let root = repo.join(DEFAULT_DIR);
        init(&root, &id_from_name("my repo!"), "My repo").unwrap();
        assert_eq!(
            init(&root, "x", "X"),
            Err(Problem::SeabagExists {
                path: root.display().to_string()
            })
        );
        let loaded = load(&root);
        assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
        let bag = loaded.seabag.unwrap();
        assert_eq!(bag.project.id, "my-repo");
        assert!(bag.areas.is_empty() && bag.tasks.is_empty());
        assert!(
            fs::read_to_string(root.join(".gitignore"))
                .unwrap()
                .contains(".lock")
        );
        let deep = repo.join("src/a/b");
        fs::create_dir_all(&deep).unwrap();
        assert_eq!(find_seabag(&deep), Some(root.clone()));
        assert_eq!(find_seabag(&std::env::temp_dir()), None);
        // No temporary folder is left next to the seabag.
        let left: Vec<_> = fs::read_dir(repo.join(".fliphopper")).unwrap().collect();
        assert_eq!(left.len(), 1);
        fs::remove_dir_all(repo).unwrap();
    }

    #[test]
    fn id_from_name_keeps_ids_valid() {
        assert_eq!(id_from_name("fliphopper"), "fliphopper");
        assert_eq!(id_from_name("我的"), "project");
        assert_eq!(id_from_name(&"a".repeat(80)).len(), 64);
    }

    #[test]
    fn writes_go_through_the_lock() {
        let dir = tmp("lock");
        let root = dir.join(DEFAULT_DIR);
        assert!(matches!(
            Lock::acquire(&root, Duration::ZERO),
            Err(Problem::SeabagNotFound { .. })
        ));
        init(&root, "p", "P").unwrap();
        let before = stamp(&root);
        let lock = Lock::acquire(&root, Duration::ZERO).unwrap();
        // A second writer waits, then gives up while the first holds the lock.
        assert!(matches!(
            Lock::acquire(&root, Duration::from_millis(20)),
            Err(Problem::SeabagBusy { .. })
        ));
        let bag = lock.load().seabag.unwrap();
        let area = fliphopper_core::ops::new_area(&bag, "A01", "First").unwrap();
        lock.create_area(&area).unwrap();
        assert!(matches!(
            lock.create_area(&area),
            Err(Problem::DuplicateAreaId { .. })
        ));
        let bag = lock.load().seabag.unwrap();
        let task =
            fliphopper_core::ops::new_task(&bag, "A01-01", "A01", "Do it", Some("Why")).unwrap();
        lock.create_task(&task).unwrap();
        assert!(matches!(
            lock.create_task(&task),
            Err(Problem::DuplicateTaskId { .. })
        ));
        let bag = lock.load().seabag.unwrap();
        let claimed = fliphopper_core::ops::claim(&bag, "A01-01", Some("me"), false)
            .unwrap()
            .task;
        lock.save_task(&claimed).unwrap();
        drop(lock);
        assert!(Lock::acquire(&root, Duration::ZERO).is_ok());

        let loaded = load(&root);
        assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
        let t = &loaded.seabag.unwrap().tasks[0];
        assert_eq!(t.holder.as_deref(), Some("me"));
        assert_eq!(t.description, "Why\n");
        let json = fs::read_to_string(root.join("tasks/A01-01/task.json")).unwrap();
        assert!(!json.contains("null") && !json.contains("description"));
        assert_ne!(stamp(&root), before);
        // Only the lock and the seabag's own files: no temporary files left behind.
        let hidden: Vec<_> = fs::read_dir(&root)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|n| n.starts_with(TMP_PREFIX))
            .collect();
        assert!(hidden.is_empty(), "{hidden:?}");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn hidden_entries_are_not_tasks() {
        let dir = tmp("hidden");
        init(&dir.join("s"), "p", "P").unwrap();
        let root = dir.join("s");
        fs::create_dir_all(root.join("tasks/.tmp-1-0-X")).unwrap();
        fs::write(root.join("areas/.tmp-1-1-A.json"), "{").unwrap();
        let loaded = load(&root);
        assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
        fs::remove_dir_all(dir).unwrap();
    }
}
