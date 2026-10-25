//! `alm`'s side of project folders (decision 0019, Task-A01-dime494): which folder a command
//! runs in, and `alm upgrade`, which only a person asks for (approver, 2026-10-25).

use std::path::Path;

use allomos_core::{FORMAT, Problem, Project};
use serde_json::json;

use crate::Done;

/// The project folder `cwd` is inside, by name: the deepest one that holds it. `None` inside the
/// taskbag itself, or outside every folder.
pub(crate) fn folder_here(root: &Path, project: &Project, cwd: &Path) -> Option<String> {
    let real = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let cwd = real(cwd);
    if cwd.starts_with(real(root)) {
        return None;
    }
    allomos_store::project_folder_paths(root, project)
        .into_iter()
        .map(|(name, path)| (name, real(&path)))
        .filter(|(_, path)| cwd.starts_with(path))
        .max_by_key(|(_, path)| path.components().count())
        .map(|(name, _)| name)
}

/// `alm upgrade`: without `confirm`, what it will do; with it, the upgrade.
pub(crate) fn upgrade(root: &Path, me: Option<&str>, confirm: bool) -> Result<Done, Problem> {
    let loaded = allomos_store::load(root);
    let (bag, issues) = allomos_store::taskbag_of(root, loaded)?;
    if bag.project.upgraded() {
        let mut done = Done::new(
            json!({ "taskbag": root, "format": bag.project.format, "upgraded": false }),
            format!(
                "This taskbag is already in {}: nothing to upgrade.\n",
                bag.project.format
            ),
        );
        done.issues = issues;
        return Ok(done);
    }
    if !confirm {
        let folder = match bag.project.main_folder() {
            Some(path) => format!(
                "Its project folder ({path}) becomes the first of its project folders, named after the folder."
            ),
            None => "It has no project folder recorded yet.".to_string(),
        };
        let text = format!(
            "This taskbag is in {}. Upgrading writes {FORMAT}, which task folders need. {folder}\nAfter the upgrade, Allomos versions older than this one (the app, alm, and every agent session using them) refuse the taskbag until the newest Allomos is installed. Install it first, then run `alm upgrade --confirm`.\n",
            bag.project.format
        );
        let mut done = Done::new(
            json!({ "taskbag": root, "format": bag.project.format, "upgraded": false, "to": FORMAT }),
            text,
        );
        done.issues = issues;
        return Ok(done);
    }
    let actor = allomos_store::Actor::alm(me);
    let up = allomos_store::upgrade(root, actor, crate::lock_wait())?;
    let text = match &up {
        Some(u) => format!(
            "Upgraded from {} to {}.{}\n",
            u.from,
            u.to,
            u.folder
                .as_ref()
                .map(|f| format!(" The project folder is named {f}."))
                .unwrap_or_default()
        ),
        None => "Already up to date.\n".to_string(),
    };
    let mut done = Done::new(
        json!({ "taskbag": root, "upgraded": up.is_some(), "upgrade": up }),
        text,
    );
    done.issues = issues;
    Ok(done)
}
