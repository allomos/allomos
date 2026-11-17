//! Creating a taskbag (a starter project or an empty one) and finding it from a folder.

use super::*;

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
    fs::write(root.join(".gitattributes"), GITATTRIBUTES)?;
    let project = Project {
        project_folders: Default::default(),
        extra: Default::default(),
        format: FORMAT.to_string(),
        id: "project".to_string(),
        name: name.to_string(),
        labels: allomos_core::default_labels(),
        project_folder: None,
        task_language: None,
        handoff: None,
        audit: None,
        heartbeat: None,
        risk: None,
    };
    write_json(&root.join(PROJECT_FILE), &project)?;
    let areas = [("A01", "Getting started", 1), ("A02", "First feature", 2)];
    for (id, name, order) in areas {
        write_json(
            &root.join("areas").join(format!("{id}.json")),
            &Area {
                extra: Default::default(),
                id: id.into(),
                name: name.into(),
                order,
            },
        )?;
    }
    let tasks = [
        (
            "A01-01",
            "Open this taskbag in Allomos",
            "A01",
            allomos_core::State::Done,
            Some("human verified"),
            "You picked a folder and Allomos created this starter project in it.\n",
        ),
        (
            "A01-02",
            "Rename the areas and tasks to your project's",
            "A01",
            allomos_core::State::Ready,
            None,
            "Edit the files in this folder, or use the `alm` command once it is available.\n",
        ),
        (
            "A02-01",
            "Describe the first outcome you want",
            "A02",
            allomos_core::State::Backlog,
            None,
            "One sentence about what a person can do when this task is done.\n",
        ),
    ];
    for (id, title, area, state, label, description) in tasks {
        let dir = root.join("tasks").join(id);
        let task = Task {
            folders: Default::default(),
            session: None,
            heartbeat: None,
            handover_request: None,
            extra: Default::default(),
            id: id.into(),
            title: title.into(),
            area: area.into(),
            state,
            labels: label.map(String::from).into_iter().collect(),
            holder: None,
            waiting_on: None,
            who: None,
            blocked_by: Vec::new(),
            next_dependency: None,
            iteration: None,
            difficulty: None,
            handoff_note: None,
            without_human_check: None,
            human_check_steps: None,
            description: String::new(),
        };
        write_json(&dir.join("task.json"), &task_file_json(&task))?;
        fs::write(dir.join("description.md"), description)?;
    }
    Ok(root.to_path_buf())
}

/// Find the taskbag for `start`: the nearest `.allomos/taskbag/` at or above it,
/// the way Git finds its repository.
pub fn find_taskbag(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .map(default_root)
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

/// Create an empty taskbag at `root` (normally `<repository>/.allomos/taskbag/`): `project.json`,
/// empty `areas/` and `tasks/`, a `.gitignore` for the lock and a `.gitattributes` keeping `\n`
/// line endings. Refused when `root` exists.
/// Never creates a Git repository.
/// This is how the app creates one, so its first event says `via: app`.
pub fn init(root: &Path, id: &str, name: &str) -> Result<PathBuf, Problem> {
    init_with_labels(root, id, name, Vec::new(), Actor::APP)
}

/// [`init`] with the project's own labels besides the built-in ones (decision 0020), made by
/// `actor`. The taskbag's first event records its creation, which is when its history starts.
pub fn init_with_labels(
    root: &Path,
    id: &str,
    name: &str,
    labels: Vec<String>,
    actor: Actor,
) -> Result<PathBuf, Problem> {
    if root.exists() {
        return Err(Problem::TaskbagExists {
            path: root.display().to_string(),
        });
    }
    let project = Project {
        project_folders: Default::default(),
        extra: Default::default(),
        format: FORMAT.to_string(),
        id: id.to_string(),
        name: name.to_string(),
        labels: allomos_core::labels::with_built_ins(
            labels
                .iter()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .map(|l| allomos_core::LabelEntry::custom(l, None))
                .collect(),
        ),
        project_folder: None,
        // New taskbags follow the app's language until someone sets another (Task-A01-21).
        task_language: Some(allomos_core::TASK_LANGUAGE_UI.to_string()),
        handoff: None,
        audit: None,
        heartbeat: None,
        risk: None,
    };
    // Checked like any project file, so `init` never writes one that `load` refuses.
    let text = json_text(&project);
    if let Err(issues) = parse_project(PROJECT_FILE, &text) {
        return Err(issues[0].problem.clone());
    }
    let first = actor.event(
        "init",
        Kind::Project,
        Some(id),
        event::changes(None, &project, &[]),
    );
    let parent = root.parent().expect("a taskbag root has a parent");
    fs::create_dir_all(parent).map_err(|e| write_failed(parent, e))?;
    let first_file = events::new_file(root);
    create_dir_atomic(
        root,
        &[
            (PROJECT_FILE, text),
            (".gitignore", GITIGNORE.to_string()),
            (".gitattributes", GITATTRIBUTES.to_string()),
            ("areas/.keep", String::new()),
            ("tasks/.keep", String::new()),
            (handoff::NOTES_FILE, handoff::DEFAULT_NOTES.to_string()),
            (&first_file, json_text(&first)),
        ],
    )
    .map_err(|p| match p {
        Problem::WriteFailed { .. } if root.exists() => Problem::TaskbagExists {
            path: root.display().to_string(),
        },
        p => p,
    })?;
    // An `allomos/8` taskbag's history is signed from its first event (Task-A01-gusto404).
    if signing::wanted(root, &first) {
        let mut value = serde_json::to_value(&first).expect("events serialize");
        let new_key = signing::sign_event(root, &mut value, &first_file);
        let mut first = first;
        if let Some(sig) = value.get("sig") {
            first.extra.insert("sig".into(), sig.clone());
        }
        write_atomic(&root.join(&first_file), &json_text(&first))?;
        if let Some(key) = new_key {
            signing::add_key(root, Some(id), &key, &value)?;
        }
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
        let p = std::env::temp_dir().join(format!("allomos-test-{name}-{n}"));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn init_with_own_id_and_ladder() {
        let dir = tmp("init-ladder");
        let root = dir.join(DEFAULT_DIR);
        let ladder = vec!["shipped".to_string(), " ".to_string()];
        init_with_labels(&root, "allomos", "Allomos", ladder, Actor::APP).unwrap();
        let bag = load(&root).taskbag.unwrap();
        let names = bag.project.label_names();
        assert_eq!(bag.project.id, "allomos");
        assert_eq!(names.len(), 6, "{names:?}");
        assert_eq!(names[5], "shipped");
        let other = dir.join("other");
        init_with_labels(&other, "p", "P", vec![], Actor::APP).unwrap();
        assert_eq!(load(&other).taskbag.unwrap().project.labels.len(), 5);
        let other = dir.join("other-2");
        assert!(!other.exists());
        assert!(matches!(
            init_with_labels(&dir.join("x"), "bad id", "P", vec!["a".into()], Actor::APP),
            Err(Problem::InvalidId { .. })
        ));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn empty_folder_gets_starter_taskbag_in_default_dir() {
        let dir = tmp("empty");
        let root = dir.join(".allomos/taskbag");
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
            loaded.taskbag.as_ref().unwrap().project.format,
            allomos_core::FORMAT
        );
        assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
        let tb = loaded.taskbag.unwrap();
        assert_eq!(tb.project.name, "Demo");
        assert_eq!(tb.areas.len(), 2);
        assert_eq!(tb.tasks.len(), 3);
        assert!(tb.tasks[0].description.contains("starter"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn repo_folder_gets_nested_taskbag_without_nested_git() {
        let repo = tmp("repo");
        fs::create_dir(repo.join(".git")).unwrap();
        fs::write(repo.join("README.md"), "code").unwrap();
        let Inspection::NoTaskbag { suggested_root, .. } = inspect(&repo) else {
            panic!("expected NoTaskbag")
        };
        assert_eq!(suggested_root, repo.join(".allomos/taskbag"));
        create_starter(&suggested_root, "Repo").unwrap();
        assert_eq!(
            inspect(&repo),
            Inspection::Found {
                root: repo.join(".allomos/taskbag")
            }
        );
        assert!(!suggested_root.join(".git").exists());
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
    fn init_makes_an_empty_taskbag_found_from_below() {
        let repo = tmp("init");
        let root = repo.join(DEFAULT_DIR);
        init(&root, &id_from_name("my repo!"), "My repo").unwrap();
        assert_eq!(
            init(&root, "x", "X"),
            Err(Problem::TaskbagExists {
                path: root.display().to_string()
            })
        );
        let loaded = load(&root);
        assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
        let bag = loaded.taskbag.unwrap();
        assert_eq!(bag.project.id, "my-repo");
        assert!(bag.areas.is_empty() && bag.tasks.is_empty());
        assert!(
            fs::read_to_string(root.join(".gitignore"))
                .unwrap()
                .contains(".lock")
        );
        let deep = repo.join("src/a/b");
        fs::create_dir_all(&deep).unwrap();
        assert_eq!(find_taskbag(&deep), Some(root.clone()));
        assert_eq!(find_taskbag(&std::env::temp_dir()), None);
        // No temporary folder is left next to the taskbag.
        let left: Vec<_> = fs::read_dir(repo.join(".allomos")).unwrap().collect();
        assert_eq!(left.len(), 1);
        fs::remove_dir_all(repo).unwrap();
    }

    #[test]
    fn id_from_name_keeps_ids_valid() {
        assert_eq!(id_from_name("allomos"), "allomos");
        assert_eq!(id_from_name("我的"), "project");
        assert_eq!(id_from_name(&"a".repeat(80)).len(), 64);
    }

    #[test]
    fn a_new_taskbag_keeps_lf_line_endings_in_git() {
        let dir = tmp("attributes");
        let root = default_root(&dir);
        init(&root, "p", "P").unwrap();
        let text = fs::read_to_string(root.join(".gitattributes")).unwrap();
        assert!(text.contains("*.json text eol=lf"), "{text}");
        assert!(text.contains("*.md text eol=lf"), "{text}");
        create_starter(&dir.join("starter"), "S").unwrap();
        assert!(dir.join("starter/.gitattributes").is_file());
        fs::remove_dir_all(dir).unwrap();
    }
}
