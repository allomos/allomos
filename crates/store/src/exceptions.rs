//! Exceptions and waivers in the history (Task-A01-seed343, decision 0021 § 1.1): recording a
//! skipped control and who accepted the risk (`alm exception`), under the taskbag's lock. The
//! rules are in [`allomos_core::exception`].

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use allomos_core::event::Kind;
use allomos_core::exception::{self, Exception, Given, Recorded};
use allomos_core::{Issue, Problem, Task, ops};

use crate::events::{self, Actor};
use crate::lock_for_change;

/// What [`record`] did.
#[derive(Debug, Clone, PartialEq)]
pub struct Excepted {
    pub task: Task,
    /// The record as it is now.
    pub exception: Exception,
    /// Whether an event was written (false: it was already recorded so).
    pub changed: bool,
    /// Problems found in the taskbag's files while loading it.
    pub issues: Vec<Issue>,
}

/// A new exception's ID: `ex-` and six letters or digits.
pub fn new_id() -> String {
    format!("ex-{}", events::random_suffix())
}

/// Every exception in the history, in the order first recorded.
pub fn all(root: &Path) -> Vec<Recorded> {
    exception::all(&events::list(root).0)
}

/// Every task's exceptions, by task ID.
pub fn by_task(root: &Path) -> BTreeMap<String, Vec<Recorded>> {
    exception::by_task(&events::list(root).0)
}

/// Record an exception on task `id`, or complete one (`given.of`): one `exception` event, under
/// the lock, when the record changes; the task's files do not change.
pub fn record(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    given: &Given,
) -> Result<Excepted, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let recorded = exception::all(&events::list(root).0);
    let (change, after) = exception::record(&bag, &recorded, id, actor.by, &new_id(), given)?;
    let task = ops::find_task(&bag, id)?;
    let changed = change.is_some();
    if let Some(change) = change {
        let event = actor.event(exception::COMMAND, Kind::Task, Some(&task.id), vec![change]);
        lock.record(&event, |_| Ok(()))?;
    }
    Ok(Excepted {
        task,
        exception: after,
        changed,
        issues,
    })
}
