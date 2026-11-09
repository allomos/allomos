//! Exceptions and waivers (Task-A01-seed343, decision 0021 § 1.1 "Exceptions", approver
//! 2026-11-08).
//!
//! When a control is skipped, the task records it: a check that was not run (a test reported
//! NOT RUN, a gate left untested), a task finished without the human check
//! (`alm done --without-human-check`), or another exception. Each record says **why**, and **who
//! accepted the risk** (`accepted_by`: a pseudonym ID or a role, never an email address).
//!
//! Each is an event change named `exception` whose `to` is the whole record (`from` is the record
//! before, when a later command completes it, for example with who accepted it), so, like the
//! outside effects, it lives in the history alone: `task.json` gains no field. A task's
//! exceptions are the last record of each ID.
//!
//! `alm` refuses only what it cannot record: an unknown task or exception, a task someone else
//! holds, a kind it does not know, an exception without why, and a check not run without naming
//! the check. That nobody is named as accepting the risk, it tells; the agent asks the person.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::event::{Change, Event, Kind};
use crate::ops::{check_holder, find_task};
use crate::{Problem, Taskbag};

/// The field that records an exception.
pub const FIELD: &str = "exception";
/// The command that records one (`alm exception`).
pub const COMMAND: &str = "exception";

/// A check that was not run (NOT RUN).
pub const CHECK_NOT_RUN: &str = "check_not_run";
/// A task finished without the human check.
pub const WITHOUT_HUMAN_CHECK: &str = "without_human_check";
/// Any other control skipped.
pub const OTHER: &str = "other";
/// The kinds `alm exception --kind` takes.
pub const KINDS: [&str; 3] = [CHECK_NOT_RUN, WITHOUT_HUMAN_CHECK, OTHER];

/// One exception as recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exception {
    /// `ex-` and six letters or digits.
    pub id: String,
    /// One of [`KINDS`] (an older or newer writer's other kinds are kept as they are).
    pub kind: String,
    /// Why the control was skipped.
    pub why: String,
    /// Who accepted the risk: a pseudonym ID or a role; `None` while nobody is named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted_by: Option<String>,
    /// The check that was not run (for `check_not_run`), for example `cargo test -p allomos-desktop`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check: Option<String>,
    #[serde(flatten)]
    pub extra: crate::model::Extra,
}

/// An exception with its task and when it was recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Recorded {
    #[serde(flatten)]
    pub exception: Exception,
    /// The task it belongs to.
    pub task: String,
    /// When it was first recorded (UTC).
    pub at: String,
    /// Who first recorded it (`--as`), when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
}

/// What `alm exception` (or `alm done --without-human-check`) was given.
#[derive(Debug, Clone, Copy, Default)]
pub struct Given<'a> {
    /// The exception to complete or correct, by ID; `None`: a new one.
    pub of: Option<&'a str>,
    pub kind: Option<&'a str>,
    pub why: Option<&'a str>,
    pub accepted_by: Option<&'a str>,
    pub check: Option<&'a str>,
}

fn clean(s: Option<&str>) -> Option<String> {
    s.map(str::trim).filter(|s| !s.is_empty()).map(String::from)
}

/// `--kind` as given (`check-not-run` and `check_not_run` alike) as a known kind.
fn kind_of(given: &str) -> Result<String, Problem> {
    let k = given.trim().replace('-', "_");
    if KINDS.contains(&k.as_str()) {
        Ok(k)
    } else {
        Err(Problem::ExceptionKindUnknown {
            kind: given.trim().to_string(),
        })
    }
}

/// Every exception in `events` (oldest first), in the order first recorded; the last record of
/// each ID wins. A change whose value is not a record is left out.
pub fn all(events: &[Event]) -> Vec<Recorded> {
    let mut out: Vec<Recorded> = Vec::new();
    for e in events {
        let (Kind::Task, Some(task)) = (e.kind, e.id.as_deref()) else {
            continue;
        };
        for c in e.changes.iter().filter(|c| c.field == FIELD) {
            let Some(Ok(exception)) = c.to.clone().map(serde_json::from_value::<Exception>) else {
                continue;
            };
            match out.iter_mut().find(|r| r.exception.id == exception.id) {
                Some(r) => r.exception = exception,
                None => out.push(Recorded {
                    exception,
                    task: task.to_string(),
                    at: e.at.clone(),
                    by: e.by.clone(),
                }),
            }
        }
    }
    out
}

/// [`all`], by task ID. Tasks with none are left out.
pub fn by_task(events: &[Event]) -> BTreeMap<String, Vec<Recorded>> {
    let mut out: BTreeMap<String, Vec<Recorded>> = BTreeMap::new();
    for r in all(events) {
        out.entry(r.task.clone()).or_default().push(r);
    }
    out
}

/// The record and its change for a new exception `new_id` (no task rules: `alm done` applies its
/// own). Refused: an unknown kind, no why, a check not run without its check.
pub fn new(new_id: &str, given: &Given) -> Result<(Change, Exception), Problem> {
    let kind = kind_of(given.kind.unwrap_or(OTHER))?;
    let why = clean(given.why).ok_or(Problem::ExceptionIncomplete {
        missing: "--why".into(),
    })?;
    let check = clean(given.check);
    if kind == CHECK_NOT_RUN && check.is_none() {
        return Err(Problem::ExceptionIncomplete {
            missing: "--check (which check was not run)".into(),
        });
    }
    let after = Exception {
        id: new_id.to_string(),
        kind,
        why,
        accepted_by: clean(given.accepted_by),
        check,
        extra: Default::default(),
    };
    Ok((change(None, &after), after))
}

fn change(before: Option<&Exception>, after: &Exception) -> Change {
    let value = |e: &Exception| serde_json::to_value(e).expect("exceptions serialize");
    Change {
        extra: Default::default(),
        field: FIELD.to_string(),
        from: before.map(value),
        to: Some(value(after)),
    }
}

/// The change `alm exception` makes on task `id`, and the record after it. `new_id` is the ID a
/// new exception gets. Refused: an unknown task, one someone else holds (the holder rule; done and
/// dropped tasks may be recorded on too, an exception is often written down afterwards), an
/// exception ID this task does not record (`exception_not_found`), a kind not in [`KINDS`]
/// (`exception_kind_unknown`), and a new exception without why, or a check not run without its
/// check (`exception_incomplete`). `None` as the change: nothing would change.
pub fn record(
    taskbag: &Taskbag,
    recorded: &[Recorded],
    id: &str,
    me: Option<&str>,
    new_id: &str,
    given: &Given,
) -> Result<(Option<Change>, Exception), Problem> {
    let task = find_task(taskbag, id)?;
    check_holder(&task, me)?;
    let Some(of) = given.of else {
        let (change, after) = new(new_id, given)?;
        return Ok((Some(change), after));
    };
    let of = of.trim();
    let Some(r) = recorded
        .iter()
        .find(|r| r.exception.id == of && r.task == task.id)
    else {
        return Err(Problem::ExceptionNotFound {
            id: of.to_string(),
            task: task.id,
        });
    };
    let mut after = r.exception.clone();
    if let Some(k) = given.kind {
        after.kind = kind_of(k)?;
    }
    if let Some(v) = clean(given.why) {
        after.why = v;
    }
    if let Some(v) = clean(given.accepted_by) {
        after.accepted_by = Some(v);
    }
    if let Some(v) = clean(given.check) {
        after.check = Some(v);
    }
    if after == r.exception {
        return Ok((None, after));
    }
    Ok((Some(change(Some(&r.exception), &after)), after))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Area, Project, State, Task};

    fn bag(holder: Option<&str>) -> Taskbag {
        let task = Task {
            folders: Default::default(),
            session: None,
            heartbeat: None,
            extra: Default::default(),
            id: "A01-1".into(),
            title: "Build".into(),
            area: "A01".into(),
            state: State::InProgress,
            labels: Vec::new(),
            holder: holder.map(String::from),
            waiting_on: None,
            who: None,
            blocked_by: Vec::new(),
            next_dependency: None,
            iteration: None,
            difficulty: None,
            handoff_note: None,
            without_human_check: None,
            human_check_steps: None,
            description: String::new(),
        };
        Taskbag {
            project: serde_json::from_value::<Project>(serde_json::json!({
                "format": crate::FORMAT, "id": "P", "name": "P", "labels": ["implemented"]
            }))
            .unwrap(),
            areas: vec![Area {
                extra: Default::default(),
                id: "A01".into(),
                name: "A".into(),
                order: 0,
            }],
            iterations: Vec::new(),
            tasks: vec![task],
        }
    }

    fn event(at: &str, change: Change) -> Event {
        Event {
            extra: Default::default(),
            at: at.into(),
            by: Some("me".into()),
            via: "alm".into(),
            command: COMMAND.into(),
            kind: Kind::Task,
            id: Some("A01-1".into()),
            agent: None,
            tool: None,
            actor: None,
            on_behalf_of: None,
            approval: None,
            changes: vec![change],
        }
    }

    #[test]
    fn recorded_then_completed_with_who_accepted_it() {
        let b = bag(Some("me"));
        let given = Given {
            kind: Some("check-not-run"),
            why: Some(" no signing key on this computer "),
            check: Some("cargo test -p allomos-desktop"),
            ..Default::default()
        };
        let (change, first) = record(&b, &[], "A01-1", Some("me"), "ex-111111", &given).unwrap();
        let change = change.unwrap();
        assert!(change.from.is_none());
        assert_eq!(first.kind, CHECK_NOT_RUN);
        assert_eq!(first.why, "no signing key on this computer");
        assert_eq!(first.accepted_by, None);
        let mut events = vec![event("2026-11-08T01:00:00Z", change)];
        let accept = Given {
            of: Some("ex-111111"),
            accepted_by: Some("approver"),
            ..Default::default()
        };
        let (change, after) =
            record(&b, &all(&events), "A01-1", Some("me"), "ex-x", &accept).unwrap();
        let change = change.unwrap();
        assert_eq!(
            change.from.as_ref().unwrap()["accepted_by"],
            serde_json::Value::Null
        );
        assert_eq!(after.accepted_by.as_deref(), Some("approver"));
        events.push(event("2026-11-08T01:05:00Z", change));
        let recorded = all(&events);
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].at, "2026-11-08T01:00:00Z");
        assert_eq!(
            recorded[0].exception.accepted_by.as_deref(),
            Some("approver")
        );
        assert_eq!(by_task(&events)["A01-1"].len(), 1);
        // The same again changes nothing.
        let (change, _) = record(&b, &recorded, "A01-1", Some("me"), "ex-x", &accept).unwrap();
        assert!(change.is_none());
    }

    #[test]
    fn refusals() {
        let b = bag(Some("other"));
        let given = Given {
            kind: Some("other"),
            why: Some("x"),
            ..Default::default()
        };
        assert!(matches!(
            record(&b, &[], "A01-1", Some("me"), "ex-1", &given),
            Err(Problem::TaskHeld { .. })
        ));
        let b = bag(None);
        let bad = Given {
            kind: Some("skipped"),
            why: Some("x"),
            ..Default::default()
        };
        assert!(matches!(
            record(&b, &[], "A01-1", None, "ex-1", &bad),
            Err(Problem::ExceptionKindUnknown { .. })
        ));
        let no_why = Given {
            kind: Some("other"),
            why: Some("  "),
            ..Default::default()
        };
        assert!(matches!(
            record(&b, &[], "A01-1", None, "ex-1", &no_why),
            Err(Problem::ExceptionIncomplete { .. })
        ));
        let no_check = Given {
            kind: Some("check_not_run"),
            why: Some("x"),
            ..Default::default()
        };
        assert!(matches!(
            record(&b, &[], "A01-1", None, "ex-1", &no_check),
            Err(Problem::ExceptionIncomplete { .. })
        ));
        let unknown = Given {
            of: Some("ex-nope00"),
            accepted_by: Some("approver"),
            ..Default::default()
        };
        assert!(matches!(
            record(&b, &[], "A01-1", None, "ex-1", &unknown),
            Err(Problem::ExceptionNotFound { .. })
        ));
    }

    #[test]
    fn a_new_one_without_a_kind_is_other() {
        let (change, e) = new(
            "ex-222222",
            &Given {
                why: Some("waived by the approver"),
                accepted_by: Some("p-7k3q2f"),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(e.kind, OTHER);
        assert_eq!(change.to.unwrap()["accepted_by"], "p-7k3q2f");
    }
}
