//! Test results in the history (Task-A01-easel128, decision 0029 § 5): recording one
//! (`alm check-result`), under the taskbag's lock, with the screenshots of a failed check copied
//! into the task's `evidence/` folder (Task-A01-fondue470, § 6). The rules are in
//! [`allomos_core::check_result`].

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use allomos_core::check_result::{self, CheckResult, Recorded};
use allomos_core::event::Kind;
use allomos_core::{Issue, Problem, Task, ops};

use crate::events::{self, Actor};
use crate::lock_for_change;

/// What [`record`] did.
#[derive(Debug, Clone, PartialEq)]
pub struct Checked {
    pub task: Task,
    /// The result as it is now.
    pub check: CheckResult,
    /// Whether an event was written (false: it was already recorded so).
    pub changed: bool,
    /// The task's results after the change, failed and not run first.
    pub results: Vec<Recorded>,
    /// Problems found in the taskbag's files while loading it.
    pub issues: Vec<Issue>,
}

/// Every task's test results, by task ID: the last of each name, in the order first recorded.
pub fn by_task(root: &Path) -> BTreeMap<String, Vec<Recorded>> {
    check_result::by_task(&events::list(root).0)
}

/// The task's screenshot folder, `tasks/<id>/evidence/`.
pub fn evidence_dir(root: &Path, id: &str) -> PathBuf {
    root.join("tasks").join(id).join(check_result::EVIDENCE_DIR)
}

/// The files task `id`'s screenshot folder holds, by name (none when it has no folder).
pub fn kept(root: &Path, id: &str) -> Vec<String> {
    let mut out: Vec<String> = fs::read_dir(evidence_dir(root, id))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_file())
        .filter_map(|e| e.file_name().to_str().map(String::from))
        .filter(|n| !n.starts_with('.') && !n.starts_with(crate::TMP_PREFIX))
        .collect();
    out.sort();
    out
}

/// A screenshot as given, read and checked: its bytes and kind (`png`, `jpeg`, `webp`).
fn read_screenshot(path: &Path) -> Result<(Vec<u8>, &'static str), Problem> {
    let file = path.display().to_string();
    let unreadable = |e: std::io::Error| Problem::ScreenshotUnreadable {
        file: file.clone(),
        detail: e.to_string(),
    };
    let mut f = fs::File::open(path).map_err(unreadable)?;
    let size = f.metadata().map_err(unreadable)?.len();
    let mut head = [0u8; 16];
    let n = f.read(&mut head).map_err(unreadable)?;
    if let Some(p) = check_result::screenshot_problem(&file, size, &head[..n]) {
        return Err(p);
    }
    let mut bytes = head[..n].to_vec();
    f.read_to_end(&mut bytes).map_err(unreadable)?;
    let kind = check_result::image_kind(&bytes).expect("checked above");
    Ok((bytes, kind))
}

/// Record one check's result on task `id`: one `check-result` event, under the lock, when it
/// changes. `screenshots` (paths on this computer) go only with a failed check: each is checked
/// and copied into the task's `evidence/` folder, and the record names them (decision 0029 § 6).
/// When writing fails, the copies are taken back with the event.
#[allow(clippy::too_many_arguments)]
pub fn record(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    name: &str,
    result: &str,
    detail: Option<&str>,
    screenshots: &[PathBuf],
) -> Result<Checked, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let recorded = by_task(root);
    let task = ops::find_task(&bag, id)?;
    let kept = kept(root, &task.id);
    // The rules first (holder, result, how many), then the files: a refusal reads none.
    check_result::record(
        &bag,
        &recorded,
        id,
        actor.by,
        name,
        result,
        detail,
        &vec![String::new(); screenshots.len()],
        &kept,
    )?;
    let mut taken = kept.clone();
    let mut copies = Vec::new();
    for path in screenshots {
        let (bytes, kind) = read_screenshot(path)?;
        let name = check_result::screenshot_name(&path.to_string_lossy(), kind, &taken);
        taken.push(name.clone());
        copies.push((name, bytes));
    }
    let names: Vec<String> = copies.iter().map(|(n, _)| n.clone()).collect();
    let (change, check) = check_result::record(
        &bag, &recorded, id, actor.by, name, result, detail, &names, &kept,
    )?;
    let changed = change.is_some();
    if let Some(change) = change {
        let event = actor.event(
            check_result::COMMAND,
            Kind::Task,
            Some(&task.id),
            vec![change],
        );
        let dir = evidence_dir(root, &task.id);
        lock.record(&event, |_| {
            let mut written = Vec::new();
            for (name, bytes) in &copies {
                let path = dir.join(name);
                if let Err(e) = crate::writing::write_new_file(&path, bytes) {
                    for w in &written {
                        let _ = fs::remove_file(w);
                    }
                    return Err(e);
                }
                written.push(path);
            }
            Ok(())
        })?;
    }
    let results = check_result::ordered(&by_task(root).remove(&task.id).unwrap_or_default());
    Ok(Checked {
        task,
        check,
        changed,
        results,
        issues,
    })
}

/// Add one screenshot, the file at `path` on this computer, to task `id`'s `evidence/` folder,
/// under the lock, and give the name it is kept under; no event is written (Send back… names it
/// in the reopen reason, Task-A01-music456, decision 0030 § 4). Counted with the others: refused
/// (`return_screenshots_full`) when the folder holds [`check_result::MOST_SCREENSHOTS`] already.
pub fn add_screenshot(
    root: &Path,
    wait: Duration,
    id: &str,
    path: &Path,
) -> Result<String, Problem> {
    let (_lock, bag, _) = lock_for_change(root, wait)?;
    let task = ops::find_task(&bag, id)?;
    let kept = kept(root, &task.id);
    if kept.len() >= check_result::MOST_SCREENSHOTS {
        return Err(Problem::ReturnScreenshotsFull {
            id: task.id.clone(),
            have: kept.join(", "),
        });
    }
    let (bytes, kind) = read_screenshot(path)?;
    let name = check_result::screenshot_name(&path.to_string_lossy(), kind, &kept);
    crate::writing::write_new_file(&evidence_dir(root, &task.id).join(&name), &bytes)?;
    Ok(name)
}
