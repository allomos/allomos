//! Links to code and releases in the history (Task-A01-mint242, decision 0021 § 1.1): recording
//! one (`alm link`), under the taskbag's lock. The rules are in [`allomos_core::link`].

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use allomos_core::event::Kind;
use allomos_core::link::{self, Link, Recorded};
use allomos_core::{Issue, Problem, Task, ops};

use crate::events::{self, Actor};
use crate::lock_for_change;

/// What [`record`] did.
#[derive(Debug, Clone, PartialEq)]
pub struct Linked {
    pub task: Task,
    /// The link as it is now (taken off: as it was).
    pub link: Link,
    /// Whether an event was written (false: it was already recorded so).
    pub changed: bool,
    /// The task's links after the change.
    pub links: Vec<Recorded>,
    /// Problems found in the taskbag's files while loading it.
    pub issues: Vec<Issue>,
}

/// Every link in the history, in the order first recorded.
pub fn all(root: &Path) -> Vec<Recorded> {
    link::all(&events::list(root).0)
}

/// Every task's links, by task ID.
pub fn by_task(root: &Path) -> BTreeMap<String, Vec<Recorded>> {
    link::by_task(&events::list(root).0)
}

/// Record a link on task `id`, or take it off (`remove`): one `link` event, under the lock, when
/// it changes; the task's files do not change.
pub fn record(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    given: &Link,
    remove: bool,
) -> Result<Linked, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let mut history = events::list(root).0;
    let recorded = link::all(&history);
    let (change, after) = link::record(&bag, &recorded, id, actor.by, given, remove)?;
    let task = ops::find_task(&bag, id)?;
    let changed = change.is_some();
    if let Some(change) = change {
        let event = actor.event(link::COMMAND, Kind::Task, Some(&task.id), vec![change]);
        lock.record(&event, |_| Ok(()))?;
        history.push(event);
    }
    let links = link::by_task(&history).remove(&task.id).unwrap_or_default();
    Ok(Linked {
        task,
        link: after,
        changed,
        links,
        issues,
    })
}
