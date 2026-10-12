//! The checks the app makes after opening a taskbag (docs/product/OPEN-FOLDER-FLOW.md): which
//! project folder the taskbag belongs to, whether that folder's agent instructions mention
//! Allomos, and whether the taskbag has tasks yet.

use std::path::{Component, Path, PathBuf};

use allomos_core::{Problem, Project, parse_project};
use serde::Serialize;

use crate::{AgentSetup, Lock, PROJECT_FILE, agent_setup, json_text, picked_folder, write_atomic};

/// How long recording the project folder waits for another writer.
const LOCK_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

/// The folder holding `.allomos/` for a taskbag at `root`; the taskbag itself when it lives
/// somewhere else.
pub fn allomos_folder(root: &Path) -> PathBuf {
    picked_folder(root).unwrap_or(root).to_path_buf()
}

/// `to` written relative to the folder `from` (both absolute): `.` for the same folder,
/// `../app` for a sibling. `None` when there is no relative way (another disk on Windows).
pub fn relative_path(from: &Path, to: &Path) -> Option<PathBuf> {
    let from: Vec<Component> = from.components().collect();
    let to: Vec<Component> = to.components().collect();
    if from.first() != to.first() {
        return None;
    }
    let same = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut path = PathBuf::new();
    for _ in same..from.len() {
        path.push("..");
    }
    for part in &to[same..] {
        path.push(part);
    }
    Some(if path.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        path
    })
}

/// The project folder recorded in `project`, as an absolute path, for a taskbag at `root`.
pub fn project_folder_of(root: &Path, project: &Project) -> Option<PathBuf> {
    let recorded = project.project_folder.as_deref()?.trim();
    if recorded.is_empty() {
        return None;
    }
    let path = allomos_folder(root).join(recorded);
    // Tidy `.` and `..` without asking the disk, so a moved folder still gives a clear path.
    let mut clean = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                clean.pop();
            }
            other => clean.push(other),
        }
    }
    Some(clean)
}

/// Record `folder` as the project folder of the taskbag at `root`, relative to the Allomos
/// folder when it can be. Returns what was written.
pub fn record_project_folder(root: &Path, folder: &Path) -> Result<String, Problem> {
    if !folder.is_dir() {
        return Err(Problem::StartupNotAFolder {
            path: folder.display().to_string(),
        });
    }
    let folder = std::path::absolute(folder).unwrap_or_else(|_| folder.to_path_buf());
    let base = std::path::absolute(allomos_folder(root)).unwrap_or_else(|_| allomos_folder(root));
    let recorded = relative_path(&base, &folder)
        .unwrap_or(folder)
        .to_string_lossy()
        .into_owned();
    let lock = Lock::acquire(root, LOCK_WAIT)?;
    let path = lock.root().join(PROJECT_FILE);
    let text = std::fs::read_to_string(&path).map_err(|e| Problem::ReadFailed {
        detail: format!("{}: {e}", path.display()),
    })?;
    let mut project =
        parse_project(PROJECT_FILE, &text).map_err(|issues| issues[0].problem.clone())?;
    project.project_folder = Some(recorded.clone());
    write_atomic(&path, &json_text(&project))?;
    Ok(recorded)
}

/// Where the opened taskbag stands in the flow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SetupState {
    /// The Allomos folder (holding `.allomos/`).
    pub allomos_folder: PathBuf,
    /// The project folder, once known. `None`: the app asks the person.
    pub project_folder: Option<PathBuf>,
    /// Which agents' instruction files in the project folder mention Allomos.
    pub agents: AgentSetup,
    /// Whether the taskbag has any task.
    pub has_tasks: bool,
}

/// Step 2 to 4 of the flow for the taskbag at `root`. When no project folder is recorded and
/// `AGENTS.md` or `CLAUDE.md` sits beside `.allomos/`, that folder is recorded here.
pub fn setup_state(root: &Path) -> Result<SetupState, Problem> {
    let loaded = crate::load(root);
    let Some(bag) = loaded.taskbag else {
        // An older taskbag is still connected: its project folder is known without reading it, so
        // the Connect panel can hand the update to an agent (Task-A01-25). Tasks are assumed, so
        // the wizard does not open by itself to look for them.
        if let crate::versions::FormatState::Older { .. } = crate::versions::format_state(root) {
            if let Some(project) = crate::versions::connected_project(root) {
                return Ok(SetupState {
                    agents: agent_setup(&project),
                    project_folder: Some(project),
                    allomos_folder: allomos_folder(root),
                    has_tasks: true,
                });
            }
        }
        // A newer or older format is said as itself, so the app words it in the person's language
        // instead of quoting the English meant for agents.
        if let Some(problem @ (Problem::TaskbagNewer { .. } | Problem::TaskbagOlder { .. })) =
            loaded.issues.first().map(|i| &i.problem)
        {
            return Err(problem.clone());
        }
        return Err(Problem::TaskbagUnreadable {
            path: root.display().to_string(),
            detail: loaded
                .issues
                .first()
                .map(|i| format!("{}: {}", i.file, i.problem.english()))
                .unwrap_or_default(),
        });
    };
    let home = allomos_folder(root);
    let mut project_folder = project_folder_of(root, &bag.project);
    if project_folder.is_none()
        && (home.join("AGENTS.md").is_file() || home.join("CLAUDE.md").is_file())
    {
        record_project_folder(root, &home)?;
        project_folder = Some(home.clone());
    }
    Ok(SetupState {
        agents: project_folder
            .as_deref()
            .map(agent_setup)
            .unwrap_or(AgentSetup {
                codex: false,
                claude: false,
            }),
        project_folder,
        allomos_folder: home,
        has_tasks: !bag.tasks.is_empty(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DEFAULT_DIR, init};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp(name: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!("allomos-setup-{name}-{n}"));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn a_newer_taskbag_is_reported_as_itself() {
        let dir = std::env::temp_dir().join(format!("allomos-setup-newer-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join(crate::DEFAULT_DIR);
        crate::init(&root, "p", "P").unwrap();
        let path = root.join(PROJECT_FILE);
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, text.replace(allomos_core::FORMAT, "allomos/9")).unwrap();
        assert_eq!(setup_state(&root).unwrap_err().code(), "taskbag_newer");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn relative_paths() {
        let r = |a: &str, b: &str| relative_path(Path::new(a), Path::new(b)).unwrap();
        assert_eq!(r("/a/b", "/a/b"), PathBuf::from("."));
        assert_eq!(r("/a/b", "/a/b/c"), PathBuf::from("c"));
        assert_eq!(r("/a/b", "/a/app"), PathBuf::from("../app"));
        assert_eq!(r("/a/b/c", "/x"), PathBuf::from("../../../x"));
    }

    #[test]
    fn the_project_folder_is_found_beside_allomos_or_left_to_ask() {
        let home = tmp("home");
        let root = home.join(DEFAULT_DIR);
        init(&root, "p", "P").unwrap();
        // Nothing beside .allomos: the app asks.
        let s = setup_state(&root).unwrap();
        assert_eq!(s.project_folder, None);
        assert!(!s.has_tasks);
        assert_eq!(s.allomos_folder, home);

        // AGENTS.md beside .allomos: recorded as "." and used.
        fs::write(home.join("AGENTS.md"), "# Guide\n").unwrap();
        let s = setup_state(&root).unwrap();
        assert_eq!(s.project_folder.as_deref(), Some(home.as_path()));
        assert_eq!(
            s.agents,
            AgentSetup {
                codex: false,
                claude: false
            }
        );
        let text = fs::read_to_string(root.join(PROJECT_FILE)).unwrap();
        assert!(text.contains("\"project_folder\": \".\""), "{text}");

        // Allomos's section in AGENTS.md: Codex is told.
        fs::write(home.join("AGENTS.md"), crate::agent_section()).unwrap();
        assert!(setup_state(&root).unwrap().agents.codex);
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn a_chosen_project_folder_is_recorded_relative() {
        let base = tmp("sibling");
        let home = base.join("tasks-home");
        let app = base.join("app");
        fs::create_dir_all(&app).unwrap();
        let root = home.join(DEFAULT_DIR);
        init(&root, "p", "P").unwrap();
        assert_eq!(record_project_folder(&root, &app).unwrap(), "../app");
        assert_eq!(
            setup_state(&root).unwrap().project_folder,
            Some(app.clone())
        );
        // A folder that does not exist is refused, and nothing changes.
        assert!(record_project_folder(&root, &base.join("missing")).is_err());
        assert_eq!(setup_state(&root).unwrap().project_folder, Some(app));
        fs::remove_dir_all(&base).unwrap();
    }
}
