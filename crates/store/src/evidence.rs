//! Each task's Evidence screen (decision 0029; Task-A01-inlet632), read from the history once for
//! the whole taskbag: the app receives it with the taskbag and `alm show --evidence` prints it.
//! What it holds and its order are in [`allomos_core::evidence`].

use std::collections::BTreeMap;
use std::path::Path;

use allomos_core::evidence::{self, Evidence};
use allomos_core::{Task, Taskbag};

use crate::events;

/// One task's Evidence screen, as [`by_task`] has it: what the app gets back after an edit of
/// that task.
pub fn of(root: &Path, task: &Task) -> Evidence {
    let events = events::list(root).0;
    let links = allomos_core::link::by_task(&events).remove(&task.id);
    let decisions = allomos_core::decision::pending(&events).remove(&task.id);
    evidence::evidence(
        task,
        links.as_deref().unwrap_or_default(),
        decisions.as_deref().unwrap_or_default(),
        &events,
    )
}

/// Every task's Evidence screen, by task ID.
pub fn by_task(root: &Path, bag: &Taskbag) -> BTreeMap<String, Evidence> {
    let events = events::list(root).0;
    let links = allomos_core::link::by_task(&events);
    let decisions = allomos_core::decision::pending(&events);
    bag.tasks
        .iter()
        .map(|t| {
            let e = evidence::evidence(
                t,
                links.get(&t.id).map_or(&[][..], Vec::as_slice),
                decisions.get(&t.id).map_or(&[][..], Vec::as_slice),
                &events,
            );
            (t.id.clone(), e)
        })
        .collect()
}
