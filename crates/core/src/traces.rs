//! Which requirements and decisions a task serves (Task-A01-fennel617, decision 0021 § 1.1
//! "Traceability", approver 2026-11-08).
//!
//! A task records the requirement IDs (`R77`, as `REQUIREMENTS.md` numbers them) and decision
//! numbers (`0021`, as `docs/decisions/` numbers them) it serves. Like the branches (Task-A03-22)
//! they live only in the task's history, as an event change named `traces` whose `to` is the
//! whole list after the change, so `task.json` gains no field and the format stays `allomos/7`.
//! A task's traces are the last list recorded; a change with only a `from` empties it.
//!
//! `alm` checks only that each one is written as a requirement ID or a decision number; whether
//! that requirement or decision exists is the agent's to check ("`alm` tells; the agent checks").

use std::collections::BTreeMap;

use serde_json::Value;

use crate::event::{Change, Event, Kind};
use crate::ops::{check_holder, find_task};
use crate::{Problem, Taskbag};

/// The field that records a task's traces.
pub const TRACES: &str = "traces";
/// The command that changes them (`alm trace`).
pub const TRACE: &str = "trace";

/// One trace as given, in its one written form: `R77` for a requirement (an `R`, either case, and
/// its number; leading zeros dropped), `0021` for a decision (its number, padded to four digits;
/// a decision file's name, such as `0021-audit-trail` or `0021-audit-trail.md`, gives its number).
/// Anything else is refused (`trace_invalid`).
pub fn trace_ref(given: &str) -> Result<String, Problem> {
    let s = given.trim();
    let invalid = || Problem::TraceInvalid {
        reference: given.to_string(),
    };
    let digits = |d: &str| !d.is_empty() && d.len() <= 6 && d.bytes().all(|b| b.is_ascii_digit());
    if let Some(n) = s.strip_prefix(['R', 'r']) {
        if digits(n) {
            let n: u32 = n.parse().map_err(|_| invalid())?;
            if n > 0 {
                return Ok(format!("R{n}"));
            }
        }
        return Err(invalid());
    }
    let number = s.split_once('-').map_or(s, |(n, _)| n);
    if digits(number) && (number.len() == 4 || number == s) {
        let n: u32 = number.parse().map_err(|_| invalid())?;
        if n > 0 && n <= 9999 {
            return Ok(format!("{n:04}"));
        }
    }
    Err(invalid())
}

/// Traces in their order: requirements by number, then decisions by number; each once.
pub fn sorted(mut traces: Vec<String>) -> Vec<String> {
    traces.sort_by_key(|t| match t.strip_prefix('R') {
        Some(n) => (0, n.parse::<u32>().unwrap_or(u32::MAX)),
        None => (1, t.parse::<u32>().unwrap_or(u32::MAX)),
    });
    traces.dedup();
    traces
}

/// Every task's traces, by task ID, from `events` (oldest first). The last `traces` change wins;
/// one with only a `from`, or an empty list, empties it. Tasks with none are left out.
pub fn recorded(events: &[Event]) -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for e in events {
        let (Kind::Task, Some(id)) = (e.kind, e.id.as_deref()) else {
            continue;
        };
        for c in e.changes.iter().filter(|c| c.field == TRACES) {
            let list = match &c.to {
                Some(Value::Array(items)) => items
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect(),
                Some(Value::String(s)) => vec![s.clone()],
                _ => Vec::new(),
            };
            out.insert(id.to_string(), list);
        }
    }
    out.retain(|_, list| !list.is_empty());
    out
}

/// The tasks that serve `reference`, by task ID (the reverse lookup: from a requirement or a
/// decision to its tasks).
pub fn tasks_for(recorded: &BTreeMap<String, Vec<String>>, reference: &str) -> Vec<String> {
    recorded
        .iter()
        .filter(|(_, list)| list.iter().any(|t| t == reference))
        .map(|(id, _)| id.clone())
        .collect()
}

/// Every requirement and decision some task serves, with its tasks (the whole reverse index).
pub fn index(recorded: &BTreeMap<String, Vec<String>>) -> Vec<(String, Vec<String>)> {
    let refs = sorted(recorded.values().flatten().cloned().collect());
    refs.into_iter()
        .map(|r| {
            let tasks = tasks_for(recorded, &r);
            (r, tasks)
        })
        .collect()
}

/// What `alm trace` changes on task `id`: `add` and `remove` (each a requirement ID or decision
/// number, [`trace_ref`]). Refused for an unknown task, one someone else holds (the holder rule),
/// and a reference in neither form. Done and dropped tasks may be traced too: linking them
/// afterwards is what an audit needs. Returns the change (none when the list stays the same) and
/// the list after it.
pub fn set(
    taskbag: &Taskbag,
    recorded: &BTreeMap<String, Vec<String>>,
    id: &str,
    me: Option<&str>,
    add: &[String],
    remove: &[String],
) -> Result<(Vec<Change>, Vec<String>), Problem> {
    let task = find_task(taskbag, id)?;
    check_holder(&task, me)?;
    let add = add
        .iter()
        .map(|r| trace_ref(r))
        .collect::<Result<Vec<_>, _>>()?;
    let remove = remove
        .iter()
        .map(|r| trace_ref(r))
        .collect::<Result<Vec<_>, _>>()?;
    let before = recorded.get(&task.id).cloned().unwrap_or_default();
    let mut after = before.clone();
    after.extend(add);
    after.retain(|t| !remove.contains(t));
    let after = sorted(after);
    if after == before {
        return Ok((Vec::new(), after));
    }
    let list = |l: &[String]| Value::Array(l.iter().cloned().map(Value::String).collect());
    let change = Change {
        extra: Default::default(),
        field: TRACES.to_string(),
        from: (!before.is_empty()).then(|| list(&before)),
        to: (!after.is_empty()).then(|| list(&after)),
    };
    Ok((vec![change], after))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn event(id: &str, to: Option<Value>) -> Event {
        serde_json::from_value(json!({
            "at": "2026-11-08T00:00:00.000Z",
            "via": "alm",
            "command": TRACE,
            "kind": "task",
            "id": id,
            "changes": [ { "field": TRACES, "to": to } ],
        }))
        .unwrap()
    }

    #[test]
    fn a_trace_has_one_written_form() {
        for (given, want) in [
            ("R77", "R77"),
            ("r77", "R77"),
            (" R007 ", "R7"),
            ("0021", "0021"),
            ("21", "0021"),
            ("0021-audit-trail", "0021"),
            ("0021-audit-trail.md", "0021"),
        ] {
            assert_eq!(trace_ref(given).unwrap(), want, "{given}");
        }
        for given in [
            "", "R", "R0", "Rx", "0000", "21-audit", "audit", "A01-17", "12345",
        ] {
            assert!(
                matches!(trace_ref(given), Err(Problem::TraceInvalid { .. })),
                "{given}"
            );
        }
    }

    #[test]
    fn the_last_list_wins_and_the_reverse_lookup_finds_the_tasks() {
        let events = vec![
            event("A01-1", Some(json!(["R77"]))),
            event("A01-2", Some(json!(["R77", "0021"]))),
            event("A01-1", Some(json!(["R5", "0021"]))),
            event("A01-3", Some(json!(["R9"]))),
            event("A01-3", None),
        ];
        let rec = recorded(&events);
        assert_eq!(rec["A01-1"], vec!["R5", "0021"]);
        assert!(!rec.contains_key("A01-3"));
        assert_eq!(tasks_for(&rec, "0021"), vec!["A01-1", "A01-2"]);
        assert_eq!(tasks_for(&rec, "R77"), vec!["A01-2"]);
        assert_eq!(
            index(&rec),
            vec![
                ("R5".to_string(), vec!["A01-1".to_string()]),
                ("R77".to_string(), vec!["A01-2".to_string()]),
                (
                    "0021".to_string(),
                    vec!["A01-1".to_string(), "A01-2".to_string()]
                ),
            ]
        );
    }

    #[test]
    fn traces_sort_requirements_first_then_decisions() {
        let got = sorted(vec![
            "0021".into(),
            "R77".into(),
            "R5".into(),
            "0003".into(),
            "R5".into(),
        ]);
        assert_eq!(got, vec!["R5", "R77", "0003", "0021"]);
    }
}
