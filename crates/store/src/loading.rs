//! Reading a taskbag: what a picked folder is, loading its files, and the links a task's
//! description may open.

use super::*;

/// Where a link in a task's description leads, once Allomos has agreed to open it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkTarget {
    /// An `http` or `https` address, for the web browser.
    Web(String),
    /// A text file inside the project, for the user's editor.
    File(PathBuf),
}

/// File types a description link may open: text the user reads or edits, never a program.
const LINKABLE: &[&str] = &["md", "markdown", "txt"];

/// Check a link from task `task`'s `description.md` in the taskbag at `root`. Web addresses
/// (`http`, `https`) are allowed. A relative path is read from the task's folder, the way a
/// Markdown viewer would, and must lead to an existing `.md`, `.markdown` or `.txt` file inside
/// the project: the folder holding `.allomos/` (or the taskbag itself when it stands alone), or
/// the code folder recorded as `project_folder` in project.json, for a taskbag kept beside the
/// code it tracks. Everything else is refused, so a description cannot be used to start a
/// program.
pub fn resolve_task_link(root: &Path, task: &str, link: &str) -> Result<LinkTarget, Problem> {
    let refused = || Problem::LinkRefused {
        link: link.to_string(),
    };
    let lower = link.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return Ok(LinkTarget::Web(link.to_string()));
    }
    // Any other scheme (`file:`, `mailto:`, `javascript:` …) and absolute paths are refused.
    if link.is_empty() || link.contains(':') || link.starts_with('/') || link.starts_with('\\') {
        return Err(refused());
    }
    if !allomos_core::ops::valid_id(task) {
        return Err(refused());
    }
    // A link may point into a file (`card.md#approvals`); the part after `#` is not a file name.
    let path = link.split('#').next().unwrap_or_default();
    let target = root.join("tasks").join(task).join(path);
    let Ok(target) = target.canonicalize() else {
        return Err(Problem::LinkNotFound {
            link: link.to_string(),
        });
    };
    let recorded = recorded_project_folder(root);
    let inside = [
        Some(picked_folder(root).unwrap_or(root).to_path_buf()),
        recorded,
    ]
    .into_iter()
    .flatten()
    .filter_map(|folder| folder.canonicalize().ok())
    .any(|folder| target.starts_with(folder));
    let text = target
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| LINKABLE.contains(&e.to_ascii_lowercase().as_str()));
    if !inside || !target.is_file() || !text {
        return Err(refused());
    }
    Ok(LinkTarget::File(plain_path(&target)))
}

/// The project folder recorded in the taskbag's `project.json`, read from that one file, without
/// loading the tasks ([`project_folder_of`]). `None` when none is recorded or the file is unreadable.
pub fn recorded_project_folder(root: &Path) -> Option<PathBuf> {
    fs::read_to_string(root.join(PROJECT_FILE))
        .ok()
        .and_then(|text| parse_project(PROJECT_FILE, &text).ok())
        .and_then(|project| project_folder_of(root, &project))
}

/// The folder the user picked, for a taskbag at `<folder>/.allomos/taskbag`; `None` for a
/// taskbag anywhere else.
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
    let nested = default_root(folder);
    if nested.join(PROJECT_FILE).is_file() {
        return Inspection::Found { root: nested };
    }
    if looks_empty(folder) {
        Inspection::Empty {
            folder: folder.to_path_buf(),
            suggested_root: nested,
        }
    } else {
        Inspection::NoTaskbag {
            folder: folder.to_path_buf(),
            suggested_root: nested,
        }
    }
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn read(root: &Path, path: &Path, issues: &mut Vec<Issue>) -> Option<String> {
    match read_text(path) {
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
pub(crate) fn sorted_entries(dir: &Path) -> Vec<PathBuf> {
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

/// Load a taskbag. Valid parts are returned even when other files have problems,
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
            taskbag: None,
            issues,
            active_iteration: None,
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

    // Optional: a taskbag without an `iterations/` folder simply has none.
    let mut iterations: Vec<(String, Iteration)> = Vec::new();
    for path in sorted_entries(&root.join("iterations")) {
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let file = rel(root, &path);
        if let Some(text) = read(root, &path, &mut issues) {
            match parse_iteration(&file, &text) {
                Ok(i) => iterations.push((file, i)),
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
            // Two changes that met (Task-A01-lawn320): said as such, not as broken JSON.
            if allomos_core::conflict::has_markers(&text) {
                issues.push(Issue::new(&file, None, Problem::MergeConflict));
                continue;
            }
            match parse_task(&file, &text) {
                Ok(mut t) => {
                    t.description = read_text(&dir.join("description.md")).unwrap_or_default();
                    tasks.push((file, t));
                }
                Err(mut e) => issues.append(&mut e),
            }
        }
    }

    issues.extend(validate_taskbag(&project, &areas, &iterations, &tasks));
    // Labels the tasks carry that the table does not list yet are shown as table entries; they
    // are written with the next change to `project.json` (decision 0020, 5).
    let mut project = project;
    for (_, t) in &tasks {
        allomos_core::labels::add_found(&mut project, &t.labels);
    }
    let mut areas: Vec<Area> = areas.into_iter().map(|(_, a)| a).collect();
    areas.sort_by(|a, b| a.order.cmp(&b.order).then(a.id.cmp(&b.id)));
    let mut iterations: Vec<Iteration> = iterations.into_iter().map(|(_, i)| i).collect();
    iterations.sort_by(|a, b| a.order.cmp(&b.order).then(a.id.cmp(&b.id)));
    let mut tasks: Vec<Task> = tasks.into_iter().map(|(_, t)| t).collect();
    // Numbered IDs first as before, then word IDs in the order they were made (Task-A01-54). The
    // history is read only when there is a word ID.
    if tasks.iter().any(|t| !allomos_core::ids::numbered(&t.id)) {
        let made = created(root);
        tasks.sort_by(|a, b| allomos_core::ids::compare(&a.id, &b.id, &made));
    }
    let taskbag = Taskbag {
        project,
        areas,
        iterations,
        tasks,
    };
    let active_iteration = allomos_core::ops::active_iteration(&taskbag).map(|i| i.id.clone());
    Loaded {
        taskbag: Some(taskbag),
        issues,
        active_iteration,
    }
}

/// The tasks whose `task.json` holds two claims that met (Task-A01-lawn320), in ID order. `me`:
/// the `--as` session. Read without a lock, like everything else; nothing is written.
pub fn claim_races(root: &Path, me: Option<&str>) -> Vec<allomos_core::conflict::ClaimRace> {
    use allomos_core::conflict;
    let mut out = Vec::new();
    for dir in sorted_entries(&root.join("tasks")) {
        let path = dir.join("task.json");
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let Some((one, two)) = conflict::sides(&text) else {
            continue;
        };
        let file = rel(root, &path);
        if let (Ok(one), Ok(two)) = (parse_task(&file, &one), parse_task(&file, &two)) {
            out.extend(conflict::ClaimRace::of(&one, &two, me));
        }
    }
    out
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
        let p = std::env::temp_dir().join(format!("allomos-test-{name}-{n}"));
        fs::create_dir_all(&p).unwrap();
        p
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
        assert_eq!(loaded.taskbag.unwrap().tasks.len(), 2);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_project_file_is_an_issue_not_a_crash() {
        let dir = tmp("noproject");
        let loaded = load(&dir);
        assert!(loaded.taskbag.is_none());
        assert_eq!(loaded.issues[0].file, "project.json");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn not_a_folder() {
        let p = tmp("nf").join("missing");
        assert!(matches!(inspect(&p), Inspection::NotAFolder { .. }));
    }

    #[test]
    fn task_links_open_only_text_files_in_the_project_and_web_pages() {
        let dir = tmp("links");
        let root = dir.join(DEFAULT_DIR);
        init(&root, "p", "P").unwrap();
        fs::create_dir_all(dir.join("docs/tasks")).unwrap();
        fs::write(dir.join("docs/tasks/Card.md"), "# Card\n").unwrap();
        fs::write(dir.join("docs/run.sh"), "echo\n").unwrap();
        fs::create_dir_all(root.join("tasks/T1")).unwrap();
        let outside = tmp("links-outside");
        fs::write(outside.join("other.md"), "x").unwrap();

        // On Windows without the `\\?\` prefix canonicalize adds, so Explorer can open it.
        let card = plain_path(&dir.join("docs/tasks/Card.md").canonicalize().unwrap());
        let file = |link: &str| resolve_task_link(&root, "T1", link);
        assert_eq!(
            file("../../../../docs/tasks/Card.md"),
            Ok(LinkTarget::File(card.clone()))
        );
        assert_eq!(
            file("../../../../docs/tasks/Card.md#approvals"),
            Ok(LinkTarget::File(card))
        );
        assert_eq!(
            file("https://example.com/a"),
            Ok(LinkTarget::Web("https://example.com/a".into()))
        );
        let refused = |link: &str| Err(Problem::LinkRefused { link: link.into() });
        for link in [
            "../../../../docs/run.sh",
            "../../../../docs/tasks",
            "file:///etc/hosts",
            "mailto:a@b.c",
            "javascript:alert(1)",
            "/etc/hosts",
            "",
        ] {
            assert_eq!(file(link), refused(link), "{link}");
        }
        // Outside the project, even as a text file.
        let escape = format!(
            "../../../../../{}/other.md",
            outside.file_name().unwrap().to_str().unwrap()
        );
        assert_eq!(file(&escape), refused(&escape));
        assert_eq!(
            file("../../../../docs/tasks/Gone.md"),
            Err(Problem::LinkNotFound {
                link: "../../../../docs/tasks/Gone.md".into()
            })
        );
        assert!(matches!(
            resolve_task_link(&root, "../x", "a.md"),
            Err(Problem::LinkRefused { .. })
        ));
        // A taskbag kept beside its code: the recorded code folder counts as the project too.
        record_project_folder(&root, &outside).unwrap();
        let other = plain_path(&outside.join("other.md").canonicalize().unwrap());
        assert_eq!(file(&escape), Ok(LinkTarget::File(other)));
        fs::remove_dir_all(dir).unwrap();
        fs::remove_dir_all(outside).unwrap();
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
