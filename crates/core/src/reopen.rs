//! Reopen reasons read from task history and priority for reopened tasks (decision 0030 § 5, 6).
//!
//! When a finished task is reopened (`alm reopen <id> --note "<why>"`), the reason is recorded
//! only in the task's event history (`reopen_note`, Task-A01-50), never in `task.json`. While
//! the task has not been finished again, `alm next`, `alm show` and `alm claim` list its reopen
//! reasons (most recent first) ahead of its handoff note, and `alm next` puts reopened ready
//! tasks ahead of other ready tasks in the same iteration.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::event::{Event, Kind};
use crate::ops::{REOPEN_NOTE, active_iteration};
use crate::{State, Task, Taskbag};

/// One reason a task was reopened (`alm reopen --note`), read from its history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReopenReason {
    /// When the task was reopened, in UTC.
    pub at: String,
    /// Who reopened it (`--as`, or the person's pseudonym from `actor.person`), when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    /// The exact reason given.
    pub reason: String,
}

fn clean(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
}

/// Every task's reopen reasons from `events` (oldest first), returned most recent first per task.
pub fn by_task(events: &[Event]) -> BTreeMap<String, Vec<ReopenReason>> {
    let mut map: BTreeMap<String, Vec<ReopenReason>> = BTreeMap::new();
    for e in events {
        let (Kind::Task, Some(task)) = (e.kind, e.id.as_deref()) else {
            continue;
        };
        let by = clean(e.by.as_deref())
            .or_else(|| clean(e.actor.as_ref().and_then(|a| a.person.as_deref())));
        for c in e.changes.iter().filter(|c| c.field == REOPEN_NOTE) {
            let Some(Value::String(text)) = &c.to else {
                continue;
            };
            if text.trim().is_empty() {
                continue;
            }
            map.entry(task.to_string()).or_default().push(ReopenReason {
                at: e.at.clone(),
                by: by.clone(),
                reason: text.clone(),
            });
        }
    }
    for reasons in map.values_mut() {
        reasons.reverse();
    }
    map
}

/// The reopen reasons to show for `task`: empty once the task is finished again (`done` or
/// `dropped`), otherwise every reopen reason recorded in its history, most recent first.
pub fn active_for<'a>(
    task: &Task,
    all: &'a BTreeMap<String, Vec<ReopenReason>>,
) -> &'a [ReopenReason] {
    if matches!(task.state, State::Done | State::Dropped) {
        return &[];
    }
    all.get(&task.id).map(Vec::as_slice).unwrap_or(&[])
}

/// Whether `task` has been reopened and has not been finished again.
pub fn is_reopened(task: &Task, all: &BTreeMap<String, Vec<ReopenReason>>) -> bool {
    !active_for(task, all).is_empty()
}

/// Move reopened tasks ahead of other ready tasks within the same iteration (decision 0030 § 6).
///
/// Expects `tasks` in `ops::next` order (active iteration first, then planned iterations in order,
/// unplanned last; then area order and ID). The sort is stable, so iteration order and relative
/// order within reopened and non-reopened groups are kept.
pub fn prioritize_reopened<'a>(
    tasks: &mut [&'a Task],
    bag: &Taskbag,
    reopened: &BTreeMap<String, Vec<ReopenReason>>,
) {
    if reopened.is_empty() {
        return;
    }
    let active = active_iteration(bag).map(|a| a.id.as_str());
    let iteration_rank = |t: &Task| {
        if t.iteration.is_some() && t.iteration.as_deref() == active {
            return 0;
        }
        t.iteration
            .as_ref()
            .and_then(|i| bag.iterations.iter().position(|x| &x.id == i))
            .map_or(usize::MAX, |p| p + 1)
    };
    tasks.sort_by_key(|t| (iteration_rank(t), !is_reopened(t, reopened)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Change, EventActor};
    use crate::model::{IterationState, Project, default_labels};
    use crate::ops;

    fn reopen_event(
        id: &str,
        at: &str,
        by: Option<&str>,
        person: Option<&str>,
        note: &str,
    ) -> Event {
        Event {
            extra: Default::default(),
            at: at.to_string(),
            by: by.map(String::from),
            via: "alm".to_string(),
            command: "reopen".to_string(),
            kind: Kind::Task,
            id: Some(id.to_string()),
            agent: None,
            tool: None,
            actor: person.map(|p| EventActor {
                person: Some(p.to_string()),
                ..Default::default()
            }),
            on_behalf_of: None,
            approval: None,
            changes: vec![Change {
                extra: Default::default(),
                field: REOPEN_NOTE.to_string(),
                from: None,
                to: Some(Value::String(note.to_string())),
            }],
        }
    }

    fn bag(specs: &[(&str, &str, Option<&str>, State)]) -> Taskbag {
        let mut b = Taskbag {
            project: Project {
                project_folders: Default::default(),
                extra: Default::default(),
                format: crate::FORMAT.into(),
                id: "p".into(),
                name: "P".into(),
                labels: default_labels(),
                project_folder: None,
                task_language: None,
                handoff: None,
                audit: None,
                heartbeat: None,
                risk: None,
                file_size: None,
                new_tasks: None,
            },
            areas: vec![],
            iterations: vec![],
            tasks: vec![],
        };
        b.areas.push(ops::new_area(&b, "A01", "One").unwrap());
        b.areas.push(ops::new_area(&b, "A02", "Two").unwrap());
        let mut i1 = ops::new_iteration(&b, "I1", "One", ops::IterationDetails::default()).unwrap();
        i1.state = IterationState::Active;
        b.iterations.push(i1);
        b.iterations
            .push(ops::new_iteration(&b, "I2", "Two", ops::IterationDetails::default()).unwrap());
        for (id, area, it, state) in specs {
            let mut t = ops::new_task(&b, id, area, id, None).unwrap();
            t.iteration = it.map(String::from);
            t.state = *state;
            b.tasks.push(t);
        }
        b
    }

    #[test]
    fn collects_most_recent_first_and_falls_back_to_person() {
        let events = vec![
            reopen_event("A01-1", "2026-11-01T10:00:00Z", None, None, "old note"),
            reopen_event(
                "A01-1",
                "2026-11-02T10:00:00Z",
                None,
                Some("p-abc123"),
                "from app",
            ),
            reopen_event(
                "A01-1",
                "2026-11-03T10:00:00Z",
                Some("will"),
                Some("p-abc123"),
                "latest note",
            ),
        ];
        let map = by_task(&events);
        let b = bag(&[("A01-1", "A01", Some("I1"), State::Ready)]);
        let ready = &b.tasks[0];
        let got = active_for(ready, &map);
        assert_eq!(got.len(), 3);
        assert_eq!(got[0].reason, "latest note");
        assert_eq!(got[0].by.as_deref(), Some("will"));
        assert_eq!(got[1].reason, "from app");
        assert_eq!(got[1].by.as_deref(), Some("p-abc123"));
        assert_eq!(got[2].reason, "old note");
        assert_eq!(got[2].by, None);

        let mut done = ready.clone();
        done.state = State::Done;
        assert!(active_for(&done, &map).is_empty());
        let mut dropped = ready.clone();
        dropped.state = State::Dropped;
        assert!(active_for(&dropped, &map).is_empty());
    }

    #[test]
    fn prioritizes_reopened_within_same_iteration_only() {
        let b = bag(&[
            ("A01-1", "A01", Some("I1"), State::Ready),
            ("A02-2", "A02", Some("I1"), State::Ready),
            ("A01-3", "A01", Some("I2"), State::Ready),
            ("A02-4", "A02", Some("I2"), State::Ready),
            ("A01-5", "A01", None, State::Ready),
            ("A02-6", "A02", None, State::Ready),
        ]);
        let events = vec![
            reopen_event(
                "A02-2",
                "2026-11-20T01:00:00Z",
                Some("will"),
                None,
                "fix I1",
            ),
            reopen_event(
                "A02-4",
                "2026-11-20T02:00:00Z",
                Some("will"),
                None,
                "fix I2",
            ),
            reopen_event(
                "A02-6",
                "2026-11-20T03:00:00Z",
                Some("will"),
                None,
                "fix unplanned",
            ),
        ];
        let reopened = by_task(&events);
        let mut next = ops::next(&b, None).unwrap();
        prioritize_reopened(&mut next, &b, &reopened);
        let ids: Vec<&str> = next.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["A02-2", "A01-1", "A02-4", "A01-3", "A02-6", "A01-5"]
        );
    }
}
