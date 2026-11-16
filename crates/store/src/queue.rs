//! What the queue of things needing a person is worked out from (Task-A01-muffin353, decision
//! 0028), read from the taskbag's history in one pass; the order itself is the core's
//! ([`allomos_core::queue`]), so `alm human` and the app list the same one.

use std::collections::BTreeMap;
use std::path::Path;

use allomos_core::classify::{self, Classification, Unapproved};
use allomos_core::decision::{self, Decision};
use allomos_core::queue::{self, Asked, Row, Times};
use allomos_core::{Task, Taskbag, approval};

use crate::events;

/// The history's part of the queue: nothing here is stored, all of it is read each time.
#[derive(Debug, Clone, Default)]
pub struct Read {
    /// Every task's classification (`alm classify`).
    pub classes: BTreeMap<String, Classification>,
    /// Every task's AI decisions to review.
    pub decisions: BTreeMap<String, Vec<Decision>>,
    /// The emergency changes not approved afterwards, on today's date.
    pub unapproved: Vec<Unapproved>,
    /// Since when each task needs a person.
    pub times: BTreeMap<String, Times>,
}

/// Read the history of the taskbag at `root` once.
pub fn read(root: &Path, bag: &Taskbag) -> Read {
    let events = events::list(root).0;
    let today = crate::classify::today();
    let classes = classify::recorded(&events);
    let unapproved = classify::unapproved(
        &classes,
        &approval::by_task(&events, bag, &today),
        &today,
    );
    Read {
        decisions: decision::pending(&events)
            .into_iter()
            .map(|(id, all)| (id, decision::to_review(&all)))
            .collect(),
        times: queue::times(&events),
        classes,
        unapproved,
    }
}

impl Read {
    /// The queue for `tasks` (all of `bag`'s, or the ones a filter kept), in the order of
    /// decision 0028. `asked`: the plans waiting for a Go ahead right now, when the caller knows
    /// them (the app does; they are not in the history).
    pub fn rows(&self, bag: &Taskbag, tasks: &[&Task], asked: &[Asked]) -> Vec<Row> {
        queue::rows(
            bag,
            tasks,
            queue::Facts {
                times: &self.times,
                classes: &self.classes,
                decisions: &self.decisions,
                unapproved: &self.unapproved,
                asked,
            },
        )
    }
}
