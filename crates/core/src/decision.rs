//! Decisions an AI agent made on its own inside a task (Task-A03-19, owner rule A00-29).
//!
//! An agent that decides a routine choice itself records it with `alm ai-decided <id> "<what and
//! why>"`. The decision lives only in the task's history, as an event (`ai-decided`) with one
//! change, `ai_decision`, whose `to` is the text. When the person has reviewed a task's decisions
//! (`alm reviewed <id>` or the app's **Reviewed** button), one `reviewed` event lists each of them
//! with only its `from`: the decision was "emptied". A task's decisions to review are therefore
//! the `ai_decision` changes since its last review, worked out from the events alone, so
//! `task.json` gains no field and the taskbag's format does not change.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::event::{Change, Event, Kind};
use crate::ops::find_task;
use crate::{Problem, Taskbag};

/// The field an `ai-decided` or `reviewed` event changes.
pub const FIELD: &str = "ai_decision";
/// The command that records a decision.
pub const DECIDED: &str = "ai-decided";
/// The command that marks a task's decisions reviewed.
pub const REVIEWED: &str = "reviewed";

/// One decision an agent made and recorded, not reviewed yet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Decision {
    /// When it was recorded, in UTC.
    pub at: String,
    /// Who recorded it (`--as`), when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    /// What was decided and why.
    pub text: String,
}

/// The change that records `text` as a decision on task `id`. Refused for an unknown task
/// (`task_not_found`) and for an empty text (`ai_decision_missing`). Any task, in any state,
/// held by anyone, may get one: recording a decision changes nothing in the task.
pub fn decide(taskbag: &Taskbag, id: &str, text: &str) -> Result<Vec<Change>, Problem> {
    find_task(taskbag, id)?;
    let text = text.trim();
    if text.is_empty() {
        return Err(Problem::AiDecisionMissing);
    }
    Ok(vec![Change {
        extra: Default::default(),
        field: FIELD.to_string(),
        from: None,
        to: Some(Value::String(text.to_string())),
    }])
}

/// The change that marks `pending` (task `id`'s decisions to review) reviewed: each decision
/// with only its `from`. Refused for an unknown task, and when there is nothing to review
/// (`nothing_to_review`).
pub fn review(taskbag: &Taskbag, id: &str, pending: &[Decision]) -> Result<Vec<Change>, Problem> {
    find_task(taskbag, id)?;
    if pending.is_empty() {
        return Err(Problem::NothingToReview { id: id.to_string() });
    }
    Ok(pending
        .iter()
        .map(|d| Change {
            extra: Default::default(),
            field: FIELD.to_string(),
            from: Some(Value::String(d.text.clone())),
            to: None,
        })
        .collect())
}

/// Every task's decisions to review, by task ID, oldest first, from `events` (oldest first).
/// A change to `ai_decision` with a `to` adds a decision; one with only a `from` (a review)
/// clears the task's list. Tasks with none are left out.
pub fn pending(events: &[Event]) -> BTreeMap<String, Vec<Decision>> {
    let mut out: BTreeMap<String, Vec<Decision>> = BTreeMap::new();
    for e in events {
        let (Kind::Task, Some(id)) = (e.kind, e.id.as_deref()) else {
            continue;
        };
        for c in e.changes.iter().filter(|c| c.field == FIELD) {
            match &c.to {
                Some(to) => out.entry(id.to_string()).or_default().push(Decision {
                    at: e.at.clone(),
                    by: e.by.clone(),
                    text: match to {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    },
                }),
                None => {
                    out.remove(id);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Area, Project, State, Task};

    fn bag() -> Taskbag {
        Taskbag {
            project: serde_json::from_value::<Project>(serde_json::json!({
                "format": crate::FORMAT, "id": "P", "name": "P", "labels": ["implemented", "human verified"]
            }))
            .unwrap(),
            areas: vec![Area {
                extra: Default::default(),
                id: "A01".into(),
                name: "A".into(),
                order: 0,
            }],
            iterations: Vec::new(),
            tasks: vec![Task {
                folders: Default::default(),
                extra: Default::default(),
                id: "A01-01".into(),
                title: "First".into(),
                area: "A01".into(),
                state: State::InProgress,
                label: None,
                holder: Some("claude/x".into()),
                waiting_on: None,
                who: None,
                blocked_by: Vec::new(),
                next_dependency: None,
                iteration: None,
                handoff_note: None,
                without_human_check: None,
                human_check_steps: None,
                description: String::new(),
            }],
        }
    }

    fn event(command: &str, id: &str, changes: Vec<Change>) -> Event {
        Event {
            extra: Default::default(),
            at: "2026-10-15T10:00:00Z".into(),
            by: Some("claude/x".into()),
            via: "alm".into(),
            command: command.into(),
            kind: Kind::Task,
            id: Some(id.into()),
            agent: None,
            tool: None,
            changes,
        }
    }

    #[test]
    fn a_decision_is_one_change_and_needs_a_text_and_a_task() {
        let c = decide(
            &bag(),
            "A01-01",
            "  Names: alm describe — match alm title  ",
        )
        .unwrap();
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].field, FIELD);
        assert_eq!(
            c[0].to,
            Some("Names: alm describe — match alm title".into())
        );
        assert_eq!(
            decide(&bag(), "A01-01", " "),
            Err(Problem::AiDecisionMissing)
        );
        assert!(matches!(
            decide(&bag(), "A09-09", "x"),
            Err(Problem::TaskNotFound { .. })
        ));
    }

    #[test]
    fn decisions_wait_until_reviewed() {
        let bag = bag();
        let mut events = vec![
            event(DECIDED, "A01-01", decide(&bag, "A01-01", "one").unwrap()),
            event(DECIDED, "A01-01", decide(&bag, "A01-01", "two").unwrap()),
            event("claim", "A01-01", vec![]),
        ];
        let p = pending(&events);
        let texts: Vec<&str> = p["A01-01"].iter().map(|d| d.text.as_str()).collect();
        assert_eq!(texts, ["one", "two"]);
        assert_eq!(p["A01-01"][0].by.as_deref(), Some("claude/x"));

        let reviewed = review(&bag, "A01-01", &p["A01-01"]).unwrap();
        assert_eq!(reviewed.len(), 2);
        assert!(reviewed.iter().all(|c| c.to.is_none() && c.from.is_some()));
        events.push(event(REVIEWED, "A01-01", reviewed));
        assert!(pending(&events).is_empty());
        assert_eq!(
            review(&bag, "A01-01", &[]),
            Err(Problem::NothingToReview {
                id: "A01-01".into()
            })
        );

        // A decision after the review waits again, alone.
        events.push(event(
            DECIDED,
            "A01-01",
            decide(&bag, "A01-01", "three").unwrap(),
        ));
        assert_eq!(pending(&events)["A01-01"].len(), 1);
    }
}
