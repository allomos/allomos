//! The outside-effect ledger (Task-A01-job650, decision 0021 § 1.1 "Outside effects", approver
//! 2026-11-08).
//!
//! Every deployment, change to a hosted resource or SQL run on a real target is recorded in the
//! task it belongs to: the target (which environment), the action (what was done), how to roll it
//! back, the approval it relies on (an approval ID, `alm approve`), and the result. It is recorded
//! **before and after**: `alm effect <task> --target … --action … --rollback …` writes the record
//! with no result yet (it is under way), and `alm effect <task> --of <effect ID> --result …`
//! writes it again with its result. An effect recorded only afterwards gives everything at once.
//!
//! Each is an event change named `effect` whose `to` is the whole record (`from` is the record
//! before, when it changes one), so, like the traces, it lives in the history alone: `task.json`
//! gains no field and the format stays `allomos/7`. A task's effects are the last record of each
//! ID.
//!
//! `alm` refuses only what it cannot record: an unknown task or effect, a task someone else holds,
//! a new effect without its target, action or rollback, and an approval ID no task records.
//! Whether an approval was needed, still applies, or covers this task, it tells; the agent checks
//! ("`alm` tells; the agent checks").

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::event::{Change, Event, Kind};
use crate::ops::{check_holder, find_task};
use crate::{Problem, Taskbag};

/// The field that records an outside effect.
pub const FIELD: &str = "effect";
/// The command that records one (`alm effect`).
pub const COMMAND: &str = "effect";

/// One outside effect as recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Effect {
    /// `ef-` and six letters or digits.
    pub id: String,
    /// Where it acts: the environment or resource (for example "production database").
    pub target: String,
    /// What is done there.
    pub action: String,
    /// How it is undone.
    pub rollback: String,
    /// The approval ID it relies on (`alm approve`); `None` when none was named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval: Option<String>,
    /// What happened; `None` while it is under way (recorded before).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(flatten)]
    pub extra: crate::model::Extra,
}

/// An effect with its task and when it was recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Recorded {
    #[serde(flatten)]
    pub effect: Effect,
    /// The task it belongs to.
    pub task: String,
    /// When it was first recorded (UTC).
    pub at: String,
    /// Who first recorded it (`--as`), when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    /// When its result was recorded (UTC), once it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_at: Option<String>,
}

/// What `alm effect` was given.
#[derive(Debug, Clone, Copy, Default)]
pub struct Given<'a> {
    /// The effect to record the result of (or correct); `None`: a new one.
    pub of: Option<&'a str>,
    pub target: Option<&'a str>,
    pub action: Option<&'a str>,
    pub rollback: Option<&'a str>,
    pub approval: Option<&'a str>,
    pub result: Option<&'a str>,
}

fn clean(s: Option<&str>) -> Option<String> {
    s.map(str::trim).filter(|s| !s.is_empty()).map(String::from)
}

/// Every effect in `events` (oldest first), in the order first recorded; the last record of each
/// ID wins. A change whose value is not a record is left out.
pub fn all(events: &[Event]) -> Vec<Recorded> {
    let mut out: Vec<Recorded> = Vec::new();
    for e in events {
        let (Kind::Task, Some(task)) = (e.kind, e.id.as_deref()) else {
            continue;
        };
        for c in e.changes.iter().filter(|c| c.field == FIELD) {
            let Some(Ok(effect)) = c.to.clone().map(serde_json::from_value::<Effect>) else {
                continue;
            };
            let result_at = effect.result.as_ref().map(|_| e.at.clone());
            match out.iter_mut().find(|r| r.effect.id == effect.id) {
                Some(r) => {
                    if effect.result.is_some() && r.effect.result != effect.result {
                        r.result_at = result_at;
                    } else if effect.result.is_none() {
                        r.result_at = None;
                    }
                    r.effect = effect;
                }
                None => out.push(Recorded {
                    effect,
                    task: task.to_string(),
                    at: e.at.clone(),
                    by: e.by.clone(),
                    result_at,
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

/// The change `alm effect` makes on task `id`, and the record after it. `new_id` is the ID a new
/// effect gets. Refused: an unknown task, one someone else holds (the holder rule; done and dropped
/// tasks may be recorded on too, an effect is often written down afterwards), an effect ID this
/// task does not record (`effect_not_found`), and a new effect without its target, action or
/// rollback (`effect_incomplete`). `None` as the change: nothing would change.
pub fn record(
    taskbag: &Taskbag,
    recorded: &[Recorded],
    id: &str,
    me: Option<&str>,
    new_id: &str,
    given: &Given,
) -> Result<(Option<Change>, Effect), Problem> {
    let task = find_task(taskbag, id)?;
    check_holder(&task, me)?;
    let (before, after) = match given.of {
        Some(of) => {
            let of = of.trim();
            let Some(r) = recorded
                .iter()
                .find(|r| r.effect.id == of && r.task == task.id)
            else {
                return Err(Problem::EffectNotFound {
                    id: of.to_string(),
                    task: task.id,
                });
            };
            let mut after = r.effect.clone();
            if let Some(v) = clean(given.target) {
                after.target = v;
            }
            if let Some(v) = clean(given.action) {
                after.action = v;
            }
            if let Some(v) = clean(given.rollback) {
                after.rollback = v;
            }
            if let Some(v) = clean(given.approval) {
                after.approval = Some(v);
            }
            if let Some(v) = clean(given.result) {
                after.result = Some(v);
            }
            (Some(r.effect.clone()), after)
        }
        None => {
            let missing: Vec<&str> = [
                ("--target", given.target),
                ("--action", given.action),
                ("--rollback", given.rollback),
            ]
            .into_iter()
            .filter(|(_, v)| clean(*v).is_none())
            .map(|(name, _)| name)
            .collect();
            if !missing.is_empty() {
                return Err(Problem::EffectIncomplete {
                    missing: missing.join(", "),
                });
            }
            (
                None,
                Effect {
                    id: new_id.to_string(),
                    target: clean(given.target).unwrap_or_default(),
                    action: clean(given.action).unwrap_or_default(),
                    rollback: clean(given.rollback).unwrap_or_default(),
                    approval: clean(given.approval),
                    result: clean(given.result),
                    extra: Default::default(),
                },
            )
        }
    };
    if before.as_ref() == Some(&after) {
        return Ok((None, after));
    }
    let value = |e: &Effect| serde_json::to_value(e).expect("effects serialize");
    let change = Change {
        extra: Default::default(),
        field: FIELD.to_string(),
        from: before.as_ref().map(value),
        to: Some(value(&after)),
    };
    Ok((Some(change), after))
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
            handover_request: None,
            extra: Default::default(),
            id: "A01-1".into(),
            title: "Deploy".into(),
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

    fn new_effect() -> Given<'static> {
        Given {
            target: Some(" production "),
            action: Some("deploy 0.1.43"),
            rollback: Some("redeploy 0.1.42"),
            approval: Some("ap-abc123"),
            ..Default::default()
        }
    }

    #[test]
    fn before_and_after_fold_into_one_record() {
        let b = bag(Some("me"));
        let (change, first) =
            record(&b, &[], "A01-1", Some("me"), "ef-111111", &new_effect()).unwrap();
        let change = change.unwrap();
        assert!(change.from.is_none());
        assert_eq!(first.target, "production");
        assert_eq!(first.result, None);
        let events = vec![event("2026-11-08T01:00:00Z", change)];
        let recorded = all(&events);
        let given = Given {
            of: Some("ef-111111"),
            result: Some("deployed, health check green"),
            ..Default::default()
        };
        let (change, after) = record(&b, &recorded, "A01-1", Some("me"), "ef-x", &given).unwrap();
        let change = change.unwrap();
        assert_eq!(
            change.from.as_ref().unwrap()["result"],
            serde_json::Value::Null
        );
        assert_eq!(
            after.result.as_deref(),
            Some("deployed, health check green")
        );
        let mut events = events;
        events.push(event("2026-11-08T01:05:00Z", change));
        let recorded = all(&events);
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].at, "2026-11-08T01:00:00Z");
        assert_eq!(
            recorded[0].result_at.as_deref(),
            Some("2026-11-08T01:05:00Z")
        );
        assert_eq!(recorded[0].effect.approval.as_deref(), Some("ap-abc123"));
        assert_eq!(by_task(&events)["A01-1"].len(), 1);
    }

    #[test]
    fn refusals() {
        let b = bag(Some("other"));
        assert!(matches!(
            record(&b, &[], "A01-1", Some("me"), "ef-1", &new_effect()),
            Err(Problem::TaskHeld { .. })
        ));
        let b = bag(None);
        let given = Given {
            target: Some("prod"),
            ..Default::default()
        };
        match record(&b, &[], "A01-1", None, "ef-1", &given) {
            Err(Problem::EffectIncomplete { missing }) => {
                assert_eq!(missing, "--action, --rollback")
            }
            other => panic!("{other:?}"),
        }
        let given = Given {
            of: Some("ef-nope00"),
            result: Some("ok"),
            ..Default::default()
        };
        assert!(matches!(
            record(&b, &[], "A01-1", None, "ef-1", &given),
            Err(Problem::EffectNotFound { .. })
        ));
    }

    #[test]
    fn the_same_result_again_changes_nothing() {
        let b = bag(None);
        let mut g = new_effect();
        g.result = Some("ok");
        let (change, _) = record(&b, &[], "A01-1", None, "ef-222222", &g).unwrap();
        let events = vec![event("2026-11-08T01:00:00Z", change.unwrap())];
        let recorded = all(&events);
        assert_eq!(
            recorded[0].result_at.as_deref(),
            Some("2026-11-08T01:00:00Z")
        );
        let again = Given {
            of: Some("ef-222222"),
            result: Some("ok"),
            ..Default::default()
        };
        let (change, _) = record(&b, &recorded, "A01-1", None, "ef-x", &again).unwrap();
        assert!(change.is_none());
    }
}
