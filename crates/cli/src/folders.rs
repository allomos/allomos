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

/// One folder on one line: `site  ../site  there, Allomos section, 3 tasks (2 open)`.
fn folder_line(f: &allomos_store::folders::FolderInfo) -> String {
    format!(
        "{}  {}  {}, {}, {} task(s) ({} open)",
        f.name,
        f.path,
        if f.exists { "there" } else { "not found" },
        if f.section {
            "Allomos section"
        } else {
            "no Allomos section"
        },
        f.tasks,
        f.open
    )
}

/// The instruction files a change wrote in a folder, with the commands that save them in that
/// folder's own repository (alm runs no version control tool).
fn folder_files_text(
    f: &allomos_store::folders::FolderInfo,
    files: &[String],
    subject: &str,
) -> String {
    if files.is_empty() {
        return String::new();
    }
    let dir = f.full.display();
    let list = files.join(" ");
    format!(
        "\nAlso changed in folder {} ({dir}): {}. Save and share them in that folder's version control. For Git:\n  git -C {dir} add -- {list}\n  git -C {dir} commit -m \"{subject}\" -- {list}\n  git -C {dir} push\n",
        f.name,
        files.join(", ")
    )
}

/// `alm folder …` (Task-A05-taco639).
pub(crate) fn folder(
    root: &Path,
    me: Option<&str>,
    what: &crate::FolderWhat,
    cwd: &Path,
) -> Result<Done, Problem> {
    use crate::FolderWhat;
    use allomos_store::folders;
    let actor = allomos_store::Actor::alm(me);
    let wait = crate::lock_wait();
    let full = |p: &Path| {
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            cwd.join(p)
        }
    };
    let changed = |c: folders::FolderChange, done_text: String, command: &str| {
        let mut text = done_text;
        if !c.tasks.is_empty() {
            text.push_str(&format!("Tasks changed: {}\n", c.tasks.join(", ")));
        }
        text.push_str(&folder_files_text(
            &c.folder,
            &c.files,
            &format!("alm: {command}"),
        ));
        Done::new(
            json!({ "taskbag": root, "folder": c.folder, "folder_files": c.files, "tasks": c.tasks }),
            text,
        )
    };
    match what {
        FolderWhat::List => {
            let list = folders::list(root)?;
            let text = if list.is_empty() {
                "No project folders. Bind one with `alm folder add <path>` (the taskbag must be allomos/6: `alm upgrade`).\n".to_string()
            } else {
                list.iter().fold(String::new(), |mut out, f| {
                    out.push_str(&folder_line(f));
                    out.push('\n');
                    out
                })
            };
            Ok(Done::new(json!({ "taskbag": root, "folders": list }), text))
        }
        FolderWhat::Add { path, name } => {
            let c = folders::add(root, actor, wait, &full(path), name.as_deref())?;
            let text = format!("Bound {}\n", folder_line(&c.folder));
            Ok(changed(c, text, "folder add"))
        }
        FolderWhat::Rename { name, new_name } => {
            let c = folders::rename(root, actor, wait, name, new_name)?;
            let text = format!("Renamed {name} to {}\n", c.folder.name);
            Ok(changed(c, text, "folder rename"))
        }
        FolderWhat::Locate { name, path } => {
            let c = folders::locate(root, actor, wait, name, &full(path))?;
            let text = format!(
                "{} is now at {} ({}).\n",
                c.folder.name,
                c.folder.path,
                c.folder.full.display()
            );
            Ok(changed(c, text, "folder locate"))
        }
        FolderWhat::Remove {
            name,
            confirm: false,
        } => {
            let f = folders::remove_preview(root, name)?;
            let text = format!(
                "Unbinding {} will: keep its {} task(s) ({} still open), taking its name off them (a task that names only this folder then has no folder); leave the folder's files where they are ({}); remove only Allomos's section from its AGENTS.md. To do it: `alm folder remove {} --confirm`.\n",
                f.name,
                f.tasks,
                f.open,
                f.full.display(),
                f.name
            );
            Ok(Done::new(
                json!({ "taskbag": root, "folder": f, "removed": false }),
                text,
            ))
        }
        FolderWhat::Remove {
            name,
            confirm: true,
        } => {
            let c = folders::remove(root, actor, wait, name)?;
            let text = format!(
                "Unbound {}. Its files stay where they are.\n",
                c.folder.name
            );
            let mut done = changed(c, text, "folder remove");
            done.fields["removed"] = json!(true);
            Ok(done)
        }
    }
}
