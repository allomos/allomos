//! Events: the history of changes (R7, Task-A01-17).
//!
//! Every change Allomos makes to a task, an area, an iteration or the project is also written
//! down as one event: what changed, from what to what, who did it and when. History, and later
//! the burndown and the timeline, are computed from events instead of guessed.
//!
//! This module only builds and checks events; `crates/store` names their files and writes them.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// What an event is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Task,
    Area,
    Iteration,
    Project,
}

/// One field's change. `from` is left out when the field was empty (or the item new), `to` when
/// it was emptied.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
    pub field: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<Value>,
}

/// One change to one item, as stored in `events/<name>.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    /// When, in UTC: `2026-10-13T14:02:05Z`.
    pub at: String,
    /// Who: the `--as` name of an agent session or person. Left out when nobody said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    /// Which writer made the change: `alm` or `app`.
    pub via: String,
    /// The command or action, for example `claim` or `add task`.
    pub command: String,
    pub kind: Kind,
    /// The item's ID; for the project, the project's ID (left out when unknown).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub changes: Vec<Change>,
}

/// The order a task's fields are listed in an event, as the task pane shows them.
pub const TASK_FIELDS: &[&str] = &[
    "title",
    "area",
    "iteration",
    "state",
    "label",
    "holder",
    "waiting_on",
    "next_dependency",
    "handoff_note",
    "description",
];

fn object<T: Serialize>(value: &T) -> Map<String, Value> {
    match serde_json::to_value(value) {
        Ok(Value::Object(map)) => map,
        _ => Map::new(),
    }
}

fn present(value: Option<&Value>) -> Option<Value> {
    match value {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) if s.is_empty() => None,
        Some(v) => Some(v.clone()),
    }
}

/// The fields that differ between `before` (`None`: the item is new) and `after`, listed in
/// `order` first and then by name. A new item's `id` is not listed: the event names the item.
/// A changed `id` (only the project's can change) is.
pub fn changes<T: Serialize>(before: Option<&T>, after: &T, order: &[&str]) -> Vec<Change> {
    let old = before.map(object).unwrap_or_default();
    let new = object(after);
    let mut fields: Vec<&str> = order.to_vec();
    let mut rest: Vec<&str> = old
        .keys()
        .chain(new.keys())
        .map(String::as_str)
        .filter(|k| !order.contains(k))
        .collect();
    rest.sort_unstable();
    rest.dedup();
    fields.extend(rest);
    fields
        .into_iter()
        .filter(|f| before.is_some() || *f != "id")
        .filter_map(|field| {
            let from = present(old.get(field));
            let to = present(new.get(field));
            (from != to).then(|| Change {
                field: field.to_string(),
                from,
                to,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{State, Task};

    fn task() -> Task {
        Task {
            id: "A01-01".into(),
            title: "First".into(),
            area: "A01".into(),
            state: State::Ready,
            label: None,
            holder: None,
            waiting_on: None,
            next_dependency: None,
            iteration: None,
            handoff_note: None,
            description: String::new(),
        }
    }

    #[test]
    fn lists_only_what_changed_in_task_order() {
        let before = task();
        let mut after = task();
        after.holder = Some("claude/x".into());
        after.state = State::InProgress;
        let c = changes(Some(&before), &after, TASK_FIELDS);
        assert_eq!(
            c,
            vec![
                Change {
                    field: "state".into(),
                    from: Some("ready".into()),
                    to: Some("in_progress".into()),
                },
                Change {
                    field: "holder".into(),
                    from: None,
                    to: Some("claude/x".into()),
                },
            ]
        );
        assert!(changes(Some(&before), &before, TASK_FIELDS).is_empty());
    }

    #[test]
    fn a_new_item_lists_its_fields_but_not_its_id() {
        let c = changes(None, &task(), TASK_FIELDS);
        let fields: Vec<&str> = c.iter().map(|c| c.field.as_str()).collect();
        assert_eq!(fields, ["title", "area", "state"]);
        assert!(c.iter().all(|c| c.from.is_none()));
    }

    #[test]
    fn stores_without_empty_parts() {
        let e = Event {
            at: "2026-10-13T14:02:05Z".into(),
            by: None,
            via: "alm".into(),
            command: "release".into(),
            kind: Kind::Task,
            id: Some("A01-01".into()),
            changes: vec![Change {
                field: "holder".into(),
                from: Some("claude/x".into()),
                to: None,
            }],
        };
        let v = serde_json::to_value(&e).unwrap();
        assert_eq!(
            v,
            serde_json::json!({
                "at": "2026-10-13T14:02:05Z", "via": "alm", "command": "release",
                "kind": "task", "id": "A01-01",
                "changes": [{ "field": "holder", "from": "claude/x" }]
            })
        );
        let back: Event = serde_json::from_value(v).unwrap();
        assert_eq!(back, e);
    }
}
