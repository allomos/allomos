//! Two sessions claimed one ready task at once (Task-A01-lawn320). Each claimed in its own copy;
//! the one that shared second had its push refused, and getting the latest changes left the
//! task's `task.json` with both claims in conflict markers. Every command first says so: whose
//! the task is (with `--as`: not yours) and the next ready task, which the session chooses from
//! (`alm next`'s order). A claim refused because the task is held already points to the same.
//! alm tells; it resolves nothing.

use std::path::Path;

use allomos_core::Taskbag;
use allomos_core::conflict::ClaimRace;
use allomos_core::ops;
use serde_json::{Value, json};

use crate::{Cli, folders, load, locate};

/// What a session is told first: the races, and the next ready task.
#[derive(Default)]
pub(crate) struct Races {
    pub races: Vec<ClaimRace>,
    /// The first ready task nobody holds, in `alm next`'s order (and folder), if any.
    pub next: Option<(String, String)>,
}

pub(crate) fn look(cli: &Cli, cwd: &Path) -> Races {
    let Ok(root) = locate(cli, cwd) else {
        return Races::default();
    };
    let races = allomos_store::claim_races(&root, cli.holder.as_deref());
    if races.is_empty() {
        return Races::default();
    }
    let next = load(&root)
        .taskbag
        .and_then(|bag| next_ready(&root, &bag, cwd, &[]));
    Races { races, next }
}

/// The first task `alm next` would list (inside a project folder, that folder's), leaving out
/// `except`.
pub(crate) fn next_ready(
    root: &Path,
    bag: &Taskbag,
    cwd: &Path,
    except: &[&str],
) -> Option<(String, String)> {
    let mut tasks = ops::next(bag, None).ok()?;
    if let Some(name) = folders::folder_here(root, &bag.project, cwd) {
        tasks = allomos_core::folders::in_folder(tasks, &name);
    }
    tasks
        .into_iter()
        .find(|t| !except.contains(&t.id.as_str()))
        .map(|t| (t.id.clone(), t.title.clone()))
}

/// The line naming the next ready task (or saying there is none).
pub(crate) fn next_line(next: &Option<(String, String)>) -> String {
    match next {
        Some((id, title)) => format!(
            "Next ready task (alm next's order; you choose, see `alm next`): {id}  {title}\n"
        ),
        None => "No other ready task is free (`alm next` shows what is).\n".to_string(),
    }
}

/// The lines printed first.
pub(crate) fn text(r: &Races) -> String {
    if r.races.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    for race in &r.races {
        let n = race.notice();
        out.push_str(&format!("notice [{}]: {}\n", n.code(), n.english()));
    }
    out.push_str(&next_line(&r.next));
    out.push('\n');
    out
}

/// The `claim_races` and `next_ready` of a `--json` result.
pub(crate) fn json(r: &Races, out: &mut Value) {
    if r.races.is_empty() {
        return;
    }
    out["claim_races"] = json!(r.races);
    out["next_ready"] = next_json(&r.next);
}

pub(crate) fn next_json(next: &Option<(String, String)>) -> Value {
    match next {
        Some((id, title)) => json!({ "id": id, "title": title }),
        None => Value::Null,
    }
}
