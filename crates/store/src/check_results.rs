//! Test results in the history (Task-A01-easel128, decision 0029 § 5): recording one
//! (`alm check-result`), under the taskbag's lock. The rules are in
//! [`allomos_core::check_result`].

use std::collections::BTreeMap;
use std::path::Path;
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

/// Record one check's result on task `id`: one `check-result` event, under the lock, when it
/// changes; the task's files do not change.
pub fn record(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    name: &str,
    result: &str,
    detail: Option<&str>,
) -> Result<Checked, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let recorded = by_task(root);
    let (change, check) =
        check_result::record(&bag, &recorded, id, actor.by, name, result, detail)?;
    let task = ops::find_task(&bag, id)?;
    let changed = change.is_some();
    if let Some(change) = change {
        let event = actor.event(
            check_result::COMMAND,
            Kind::Task,
            Some(&task.id),
            vec![change],
        );
        lock.record(&event, |_| Ok(()))?;
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
