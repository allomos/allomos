//! Separation of duties (Task-A01-grotto036, decision 0021 § 1.2 and Question 6): a switch in
//! `project.json`'s `audit`, off by default. When it is on, the same person or agent may not both
//! work on a task and check it (a human check), nor both work on it and approve it, in either
//! order. It is the one check `alm` makes on its own judgment; the approver approved it as a
//! switch the project turns on (decision 0021, Decided, 2026-11-08).
//!
//! Who is who comes from the events alone:
//! - **Working on a task** is an event of one of [`WORK`] (claim, start, label, done, handoff,
//!   release). Its people are the agent's session (`actor.session`), its `--as` name (`by`, for
//!   `alm`), the person in `actor.person`, and the person the agent works for (`for`): a person's
//!   agent doing the work counts as that person doing it.
//! - **Checking** is a `human_check` change; its person is the `checker`.
//! - **Approving** is an `approve` event's approval; its person is the `approver`. A withdrawn
//!   approval no longer counts.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::event::{Event, Kind};
use crate::model::Project;
use crate::{Problem, approval, labels};

/// The commands that work on a task.
pub const WORK: [&str; 6] = ["claim", "start", "label", "done", "handoff", "release"];

/// `project.json`'s `audit` field that holds the switch.
pub const FIELD: &str = "separation_of_duties";

/// Whether separation of duties is on in `project`.
pub fn on(project: &Project) -> bool {
    project
        .audit
        .as_ref()
        .is_some_and(|a| a.separation_of_duties)
}

/// One identity, as compared: `p:` a person's pseudonym, `s:` an agent's session ID, `n:` an
/// `--as` name given to `alm`. Shown to the person without its prefix.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Who(String);

impl Who {
    fn shown(&self) -> &str {
        &self.0[2..]
    }
}

fn clean(s: Option<&str>) -> Option<&str> {
    s.map(str::trim).filter(|s| !s.is_empty())
}

/// Everyone behind a change: its agent session, its `--as` name, its person, and whom it was for.
/// Ordered so the person shown in a refusal is the most telling one: a person first.
fn doers(e: &Event) -> Vec<Who> {
    let mut out = Vec::new();
    let actor = e.actor.as_ref();
    if let Some(p) = clean(actor.and_then(|a| a.person.as_deref())) {
        out.push(Who(format!("p:{p}")));
    }
    if let Some(p) = clean(e.on_behalf_of.as_deref()) {
        out.push(Who(format!("p:{p}")));
    }
    if let Some(by) = clean(e.by.as_deref()) {
        // The app records the person's pseudonym in `by` (Task-A01-frolic587).
        let prefix = if e.via == "app" { "p" } else { "n" };
        out.push(Who(format!("{prefix}:{by}")));
    }
    if let Some(s) = clean(actor.and_then(|a| a.session.as_deref())) {
        out.push(Who(format!("s:{s}")));
    }
    out
}

fn works(e: &Event) -> bool {
    WORK.contains(&e.command.as_str())
}

/// The checkers a change records: `Some(None)` for a human check whose checker is not known. A
/// "by human" label added without a `human_check` record (`alm label <id> "locally verified by
/// human"`) is a check too, by the person who made the change.
fn checkers(e: &Event) -> Vec<Option<Who>> {
    let recorded: Vec<Option<Who>> = e
        .changes
        .iter()
        .filter(|c| c.field == labels::HUMAN_CHECK)
        .map(|c| {
            let checker =
                c.to.as_ref()
                    .and_then(|v| v.get("checker"))
                    .and_then(Value::as_str);
            clean(checker).map(|p| Who(format!("p:{p}")))
        })
        .collect();
    if !recorded.is_empty() {
        return recorded;
    }
    let has = |v: &Option<Value>, l: &str| {
        v.as_ref()
            .and_then(Value::as_array)
            .is_some_and(|a| a.iter().any(|x| x.as_str() == Some(l)))
    };
    let adds_by_human = e.changes.iter().any(|c| {
        c.field == "labels"
            && labels::BY_HUMAN
                .iter()
                .any(|l| has(&c.to, l) && !has(&c.from, l))
    });
    if !adds_by_human {
        return Vec::new();
    }
    vec![doers(e).into_iter().find(|w| w.0.starts_with("p:"))]
}

/// The approval an `approve` change records: its ID and its approver.
fn approvals(e: &Event) -> Vec<(Option<String>, Option<Who>)> {
    if e.command != approval::APPROVE {
        return Vec::new();
    }
    e.changes
        .iter()
        .filter(|c| c.field == approval::FIELD)
        .filter_map(|c| c.to.as_ref())
        .map(|v| {
            let id = v.get("id").and_then(Value::as_str).map(String::from);
            let who =
                clean(v.get("approver").and_then(Value::as_str)).map(|p| Who(format!("p:{p}")));
            (id, who)
        })
        .collect()
}

/// The approval IDs withdrawn in `history`.
fn withdrawn(history: &[&Event]) -> BTreeSet<String> {
    history
        .iter()
        .filter(|e| e.command == approval::WITHDRAW)
        .flat_map(|e| e.changes.iter())
        .filter(|c| c.field == approval::FIELD && c.to.is_none())
        .filter_map(|c| c.from.as_ref()?.get("id")?.as_str().map(String::from))
        .collect()
}

/// Refuse `new` when separation of duties is on in `project` and it would make one person or
/// agent both do and check, or both do and approve, its task. `history` is every event recorded
/// before (only the same task's count). An event about anything but a task, and every event while
/// the switch is off, passes unchanged.
///
/// A human check whose checker is not known is refused too: Allomos cannot tell it is someone
/// else (`--checker <pseudonym>` names them).
pub fn refuse(project: &Project, history: &[Event], new: &Event) -> Result<(), Problem> {
    if !on(project) || new.kind != Kind::Task {
        return Ok(());
    }
    let Some(id) = new.id.as_deref() else {
        return Ok(());
    };
    let past: Vec<&Event> = history
        .iter()
        .filter(|e| e.kind == Kind::Task && e.id.as_deref() == Some(id))
        .collect();
    let did: BTreeSet<Who> = past
        .iter()
        .filter(|e| works(e))
        .flat_map(|e| doers(e))
        .collect();
    let is_doer = |w: &Who| did.contains(w);

    for checker in checkers(new) {
        let Some(checker) = checker else {
            return Err(Problem::CheckerUnknown { id: id.to_string() });
        };
        if is_doer(&checker) {
            return Err(Problem::SelfCheck {
                id: id.to_string(),
                who: checker.shown().to_string(),
            });
        }
    }
    for (_, approver) in approvals(new) {
        if let Some(approver) = approver.filter(is_doer) {
            return Err(Problem::SelfApprove {
                id: id.to_string(),
                who: approver.shown().to_string(),
            });
        }
    }
    if works(new) {
        let gone = withdrawn(&past);
        let mut signed: BTreeSet<Who> = past.iter().flat_map(|e| checkers(e)).flatten().collect();
        signed.extend(
            past.iter()
                .flat_map(|e| approvals(e))
                .filter(|(id, _)| id.as_ref().is_none_or(|i| !gone.contains(i)))
                .filter_map(|(_, who)| who),
        );
        if let Some(who) = doers(new).into_iter().find(|w| signed.contains(w)) {
            return Err(Problem::WorkAfterSigning {
                id: id.to_string(),
                who: who.shown().to_string(),
            });
        }
    }
    Ok(())
}

/// `project` with separation of duties turned `on` or off, and the event's changes: the `audit`
/// field's from and to, plus the reason when one was given. Turning it off needs a reason
/// (decision 0021 § 6); setting it to what it already is changes nothing.
pub fn set(
    project: &Project,
    turn_on: bool,
    reason: Option<&str>,
) -> Result<(Project, Vec<crate::event::Change>), Problem> {
    let reason = clean(reason);
    if on(project) == turn_on {
        return Ok((project.clone(), Vec::new()));
    }
    if !turn_on && reason.is_none() {
        return Err(Problem::AuditReasonMissing {
            setting: FIELD.into(),
        });
    }
    let mut audit = project.audit.clone().unwrap_or_default();
    audit.separation_of_duties = turn_on;
    let empty = audit == crate::audit::Audit::default();
    let after = Project {
        audit: (!empty).then_some(audit),
        ..project.clone()
    };
    let mut changes: Vec<crate::event::Change> = crate::event::changes(Some(project), &after, &[])
        .into_iter()
        .filter(|c| c.field == "audit")
        .collect();
    if let Some(why) = reason {
        changes.push(crate::event::reason_change(why));
    }
    Ok((after, changes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Change, EventActor};
    use serde_json::json;

    fn project(on: bool) -> Project {
        let p: Project = serde_json::from_value(json!({
            "format": crate::FORMAT, "id": "p", "name": "P"
        }))
        .unwrap();
        set(&p, on, None).map(|(p, _)| p).unwrap_or(p)
    }

    fn ev(
        command: &str,
        by: Option<&str>,
        session: Option<&str>,
        person_for: Option<&str>,
    ) -> Event {
        Event {
            at: "2026-11-08T00:00:00Z".into(),
            by: by.map(String::from),
            via: "alm".into(),
            command: command.into(),
            kind: Kind::Task,
            id: Some("A-1".into()),
            agent: session.map(|_| "claude".into()),
            tool: None,
            actor: session.map(|s| EventActor {
                session: Some(s.into()),
                ..Default::default()
            }),
            on_behalf_of: person_for.map(String::from),
            approval: None,
            changes: vec![Change {
                field: "state".into(),
                from: None,
                to: Some(json!("in_progress")),
                extra: Default::default(),
            }],
            extra: Default::default(),
        }
    }

    fn check(checker: Option<&str>) -> Event {
        let mut e = ev("checked", Some("rec"), Some("s-rec"), None);
        e.changes = vec![Change {
            field: labels::HUMAN_CHECK.into(),
            from: None,
            to: Some(json!({ "checker": checker, "result": "pass" })),
            extra: Default::default(),
        }];
        e
    }

    fn approve(id: &str, approver: &str) -> Event {
        let mut e = ev("approve", None, None, None);
        e.changes = vec![Change {
            field: approval::FIELD.into(),
            from: None,
            to: Some(json!({ "id": id, "approver": approver })),
            extra: Default::default(),
        }];
        e
    }

    #[test]
    fn off_by_default_changes_nothing() {
        let p = project(false);
        assert!(!on(&p));
        assert_eq!(p.audit, None);
        let history = vec![ev("claim", Some("w"), Some("s1"), Some("p-me"))];
        assert_eq!(refuse(&p, &history, &check(Some("p-me"))), Ok(()));
        assert_eq!(refuse(&p, &history, &check(None)), Ok(()));
    }

    #[test]
    fn the_doer_may_not_check() {
        let p = project(true);
        let history = vec![
            ev("add task", Some("planner"), None, Some("p-boss")),
            ev("claim", Some("w"), Some("s1"), Some("p-me")),
        ];
        assert_eq!(
            refuse(&p, &history, &check(Some("p-me"))),
            Err(Problem::SelfCheck {
                id: "A-1".into(),
                who: "p-me".into()
            })
        );
        // Someone who only planned it may check; so may anyone else.
        assert_eq!(refuse(&p, &history, &check(Some("p-boss"))), Ok(()));
        assert_eq!(refuse(&p, &history, &check(Some("p-other"))), Ok(()));
        // Who cannot be told apart is refused.
        assert_eq!(
            refuse(&p, &history, &check(None)),
            Err(Problem::CheckerUnknown { id: "A-1".into() })
        );
        // Another task's work does not count.
        let mut other = check(Some("p-me"));
        other.id = Some("A-2".into());
        assert_eq!(refuse(&p, &history, &other), Ok(()));
    }

    #[test]
    fn the_doer_may_not_approve_and_the_approver_may_not_do() {
        let p = project(true);
        let did = vec![ev("done", Some("w"), Some("s1"), Some("p-me"))];
        assert_eq!(
            refuse(&p, &did, &approve("ap-1", "p-me")),
            Err(Problem::SelfApprove {
                id: "A-1".into(),
                who: "p-me".into()
            })
        );
        let approved = vec![approve("ap-1", "p-boss")];
        // The approver's own agent may not then work on it.
        assert_eq!(
            refuse(
                &p,
                &approved,
                &ev("claim", Some("w"), Some("s1"), Some("p-boss"))
            ),
            Err(Problem::WorkAfterSigning {
                id: "A-1".into(),
                who: "p-boss".into()
            })
        );
        assert_eq!(
            refuse(
                &p,
                &approved,
                &ev("claim", Some("w"), Some("s1"), Some("p-me"))
            ),
            Ok(())
        );
        // Once withdrawn, the approval no longer counts.
        let mut gone = approve("ap-1", "p-boss");
        gone.command = approval::WITHDRAW.into();
        gone.changes[0].from = gone.changes[0].to.take();
        let history = vec![approve("ap-1", "p-boss"), gone];
        assert_eq!(
            refuse(
                &p,
                &history,
                &ev("claim", Some("w"), Some("s1"), Some("p-boss"))
            ),
            Ok(())
        );
    }

    /// A "by human" label set with `alm label` is a check by whoever set it.
    #[test]
    fn a_by_human_label_is_a_check() {
        let p = project(true);
        let history = vec![ev("claim", Some("w"), Some("s1"), Some("p-me"))];
        let mut label = ev("label", Some("w"), Some("s1"), Some("p-me"));
        label.changes = vec![Change {
            field: "labels".into(),
            from: Some(json!(["locally verified by agent"])),
            to: Some(json!([
                "locally verified by agent",
                labels::LOCALLY_BY_HUMAN
            ])),
            extra: Default::default(),
        }];
        assert_eq!(
            refuse(&p, &history, &label),
            Err(Problem::SelfCheck {
                id: "A-1".into(),
                who: "p-me".into()
            })
        );
        label.on_behalf_of = Some("p-other".into());
        label.by = Some("x".into());
        label.actor = None;
        assert_eq!(refuse(&p, &history, &label), Ok(()));
    }

    /// An agent session is one doer, whoever it works for; so is an `--as` name.
    #[test]
    fn agents_are_told_apart_by_session_and_name() {
        let p = project(true);
        let mut checked = check(Some("p-boss"));
        checked.by = Some("w".into());
        let history = vec![checked];
        assert_eq!(
            refuse(&p, &history, &ev("label", None, Some("s9"), Some("p-boss"))),
            Err(Problem::WorkAfterSigning {
                id: "A-1".into(),
                who: "p-boss".into()
            })
        );
        // The session that recorded the check (on the person's word) did not check.
        assert_eq!(
            refuse(
                &p,
                &history,
                &ev("label", Some("rec"), Some("s-rec"), Some("p-me"))
            ),
            Ok(())
        );
    }

    #[test]
    fn turning_it_off_needs_a_reason() {
        let p = project(false);
        let (on_p, changes) = set(&p, true, None).unwrap();
        assert!(on(&on_p));
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field, "audit");
        assert!(
            serde_json::to_string(&on_p)
                .unwrap()
                .contains("\"separation_of_duties\":true")
        );
        assert_eq!(
            set(&on_p, false, Some("  ")).unwrap_err(),
            Problem::AuditReasonMissing {
                setting: FIELD.into()
            }
        );
        let (off, changes) = set(&on_p, false, Some("one person left")).unwrap();
        assert!(!on(&off));
        assert_eq!(off.audit, None);
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[1].field, crate::event::REASON);
        // Already so: nothing changes.
        assert!(set(&off, false, None).unwrap().1.is_empty());
    }
}
