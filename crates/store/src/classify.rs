//! A task's change type, risk and what it touches, in the history (Task-A01-palm011, decision
//! 0021 § 1.1): recording them (`alm classify`) under the taskbag's lock, and the emergencies
//! still to be approved afterwards. The rules are in [`allomos_core::classify`].

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use allomos_core::classify::{self, Classification, Given, Unapproved};
use allomos_core::event::Kind;
use allomos_core::{Issue, Problem, Task, ops};

use crate::events::{self, Actor};
use crate::lock_for_change;

/// Today in UTC, `YYYY-MM-DD` (`ALLOMOS_NOW` fixes it in tests).
fn today() -> String {
    events::now_utc().chars().take(10).collect()
}

/// Every task's classification, by task ID.
pub fn by_task(root: &Path) -> BTreeMap<String, Classification> {
    classify::recorded(&events::list(root).0)
}

/// Every emergency change without an approval recorded afterwards, on today's date.
pub fn unapproved(root: &Path) -> Vec<Unapproved> {
    classify::unapproved(&by_task(root), &crate::approvals::approvals(root), &today())
}

/// What [`set`] did.
#[derive(Debug, Clone, PartialEq)]
pub struct Classified {
    pub task: Task,
    /// The classification after it, as the history now reads it.
    pub classification: Classification,
    /// Whether an event was written (false: nothing changed).
    pub changed: bool,
    /// Problems found in the taskbag's files while loading it.
    pub issues: Vec<Issue>,
}

/// Classify task `id`: one `classify` event, under the lock, when something changes; the task's
/// files do not change.
pub fn set(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    given: &Given,
) -> Result<Classified, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let recorded = by_task(root);
    let (changes, _) = classify::set(&bag, &recorded, id, actor.by, given, &today())?;
    let task = ops::find_task(&bag, id)?;
    let changed = !changes.is_empty();
    if changed {
        let event = actor.event(classify::COMMAND, Kind::Task, Some(&task.id), changes);
        lock.record(&event, |_| Ok(()))?;
    }
    let classification = by_task(root).remove(&task.id).unwrap_or_default();
    Ok(Classified {
        task,
        classification,
        changed,
        issues,
    })
}
