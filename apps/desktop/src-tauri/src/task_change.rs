//! One task changed in the window (Task-A01-parade485): what the window gets back, so it shows the
//! change by replacing that one task in its list instead of reading the whole taskbag again.
//!
//! Reading the whole taskbag reads every task file and the history several times over (seconds on
//! a taskbag with thousands of events); the change itself already holds everything about the one
//! task it changed.

use std::path::Path;

use allomos_core::ops::Edit;
use allomos_core::{Problem, Task};
use serde::Serialize;

/// A task as an edit left it, with the parts of [`crate::Opened`] that follow one task.
#[derive(Debug, Serialize)]
pub(crate) struct TaskChange {
    pub(crate) task: Task,
    /// The task's new version (`Opened::versions`).
    version: String,
    /// When the change was recorded (`Opened::last_changed`); `None` when nothing changed.
    changed_at: Option<String>,
    /// The card file its `Card:` link points to outside the taskbag, if any
    /// (`Opened::cards_outside`, for a task no agent reported; a report stays as it is).
    card_outside: Option<allomos_core::card::Outside>,
    /// The change reached beyond this task (the label table, the outside witness's checkpoint):
    /// the window reads the whole taskbag again, as before.
    whole: bool,
    /// The taskbag's fingerprint just before the change was written and just after, taken under
    /// the lock; as text, like `taskbag_stamp`. A window whose last fingerprint is `stamp_before`
    /// has seen everything but this change, and goes on from `stamp_after`; any other window
    /// reads the whole taskbag again (another session changed it too).
    stamp_before: String,
    stamp_after: String,
}

/// Make `edit` to task `id` as the app (`actor`), the same change through the same rule, lock and
/// event as before, and tell the window what it changed.
pub(crate) fn edit(
    root: &Path,
    actor: allomos_store::Actor,
    wait: std::time::Duration,
    id: &str,
    edit: &Edit,
) -> Result<TaskChange, Problem> {
    let alone = allomos_store::change_task_alone(root, actor, wait, |bag| {
        allomos_core::ops::edit(bag, id, actor.by, edit)
    })?;
    let task = alone.changed.task;
    // The person's check: the outside witness may make a checkpoint (Task-A01-bake791), which
    // writes more files after the fingerprint was taken.
    let checked = matches!(edit, Edit::Checked { .. });
    if checked {
        crate::audit::after_approval(root);
    }
    Ok(TaskChange {
        version: allomos_core::ops::task_version(&task),
        changed_at: alone.at,
        card_outside: allomos_store::cards::linked_outside(root, &task),
        whole: alone.project_changed || checked,
        stamp_before: alone.stamp_before.to_string(),
        stamp_after: alone.stamp_after.to_string(),
        task,
    })
}
