//! The outside-effect ledger in the history (Task-A01-job650, decision 0021 § 1.1): recording an
//! effect before and after (`alm effect`), under the taskbag's lock. The rules are in
//! [`allomos_core::effect`].

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use allomos_core::approval::Recorded as Approval;
use allomos_core::effect::{self, Effect, Given, Recorded};
use allomos_core::event::Kind;
use allomos_core::{Issue, Problem, Task, ops};

use crate::events::{self, Actor};
use crate::lock_for_change;

/// What [`record`] did.
#[derive(Debug, Clone, PartialEq)]
pub struct Effected {
    pub task: Task,
    /// The record as it is now.
    pub effect: Effect,
    /// Whether an event was written (false: it was already recorded so).
    pub changed: bool,
    /// The approval it names, as the history records it, with where it stands now.
    pub approval: Option<Approval>,
    /// Problems found in the taskbag's files while loading it.
    pub issues: Vec<Issue>,
}

/// Every outside effect in the history, in the order first recorded.
pub fn all(root: &Path) -> Vec<Recorded> {
    effect::all(&events::list(root).0)
}

/// Every task's outside effects, by task ID.
pub fn by_task(root: &Path) -> BTreeMap<String, Vec<Recorded>> {
    effect::by_task(&events::list(root).0)
}

/// Record an outside effect on task `id`, or its result (`given.of`): one `effect` event, under
/// the lock, when the record changes; the task's files do not change. The event relies on the
/// effect's approval (its `approval`). Refused also for an approval ID no task records.
pub fn record(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    given: &Given,
) -> Result<Effected, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let history = events::list(root).0;
    let recorded = effect::all(&history);
    let new_id = format!("ef-{}", events::random_suffix());
    let (change, after) = effect::record(&bag, &recorded, id, actor.by, &new_id, given)?;
    let approval = match after.approval.as_deref() {
        Some(ap) => {
            let today: String = events::now_utc().chars().take(10).collect();
            let found = allomos_core::approval::all(&history, &bag, &today)
                .into_iter()
                .find(|r| r.approval.id == ap);
            match found {
                Some(r) => Some(r),
                None => return Err(Problem::ApprovalNotFound { id: ap.to_string() }),
            }
        }
        None => None,
    };
    let task = ops::find_task(&bag, id)?;
    let changed = change.is_some();
    if let Some(change) = change {
        let mut event = actor.event(effect::COMMAND, Kind::Task, Some(&task.id), vec![change]);
        event.approval = after.approval.clone();
        lock.record(&event, |_| Ok(()))?;
    }
    Ok(Effected {
        task,
        effect: after,
        changed,
        approval,
        issues,
    })
}
