//! Reopen reasons read from the taskbag's history (decision 0030 § 5, 6).

use std::collections::BTreeMap;
use std::path::Path;

use allomos_core::reopen::{self, ReopenReason};
use allomos_core::{State, Task};

use crate::events;

/// Every task's reopen reasons in the history, most recent first per task.
pub fn by_task(root: &Path) -> BTreeMap<String, Vec<ReopenReason>> {
    reopen::by_task(&events::list(root).0)
}

/// The reopen reasons to show for `task`, most recent first; empty once `task` is finished again.
pub fn active_for(root: &Path, task: &Task) -> Vec<ReopenReason> {
    if matches!(task.state, State::Done | State::Dropped) {
        return Vec::new();
    }
    by_task(root).remove(&task.id).unwrap_or_default()
}
