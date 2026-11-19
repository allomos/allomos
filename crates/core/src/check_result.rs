//! Test results as records (Task-A01-easel128, decision 0029 § 5, approver 2026-11-20).
//!
//! An agent records each check it ran, or did not run, with `alm check-result <id> --name
//! "<check>" --result pass|fail|not-run [--detail "<one sentence>"]`. Like the traces, a result
//! lives only in the task's history: an event change named `check_result` whose `to` is the whole
//! record (`{ "name", "result", "detail"? }`) and whose `from` is the record of the same name
//! before, when there was one. `task.json` gains no field and the format stays `allomos/7`. A
//! task's results are the last record of each name, in the order first recorded; the Evidence
//! screen and `alm show` list failed and not run first ([`ordered`]).
//!
//! `alm` refuses only what it cannot record: an unknown task, one someone else holds, a check
//! without a name, and a result that is none of the three. Whether a task has enough results, or
//! the right ones, it does not judge ("`alm` tells; the agent checks").

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::event::{Change, Event, Kind};
use crate::ops::{check_holder, find_task};
use crate::{Problem, Taskbag};

/// The field that records a test result.
pub const FIELD: &str = "check_result";
/// The command that records one (`alm check-result`).
pub const COMMAND: &str = "check-result";

/// The written results, in the order they are listed: failed, not run, passed.
pub const RESULTS: [&str; 3] = ["fail", "not-run", "pass"];

/// One check's result as recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckResult {
    /// The check, as the agent names it (for example "npm test").
    pub name: String,
    /// `pass`, `fail` or `not-run`.
    pub result: String,
    /// One sentence: what ran, or why it failed or did not run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(flatten)]
    pub extra: crate::model::Extra,
}

/// A result with when and by whom it was last recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Recorded {
    #[serde(flatten)]
    pub check: CheckResult,
    /// When this result was recorded (UTC).
    pub at: String,
    /// Who recorded it (`--as`), when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
}

/// The one written form of a result: `pass`, `fail` or `not-run` (any case; `not_run`, `notrun`
/// and `not run` are taken as `not-run`). Anything else is refused (`check_result_unknown`).
pub fn result_of(given: &str) -> Result<&'static str, Problem> {
    let s = given.trim().to_ascii_lowercase();
    match s.as_str() {
        "pass" | "passed" => Ok("pass"),
        "fail" | "failed" => Ok("fail"),
        "not-run" | "not_run" | "notrun" | "not run" => Ok("not-run"),
        _ => Err(Problem::CheckResultUnknown {
            result: given.to_string(),
        }),
    }
}

/// Every task's results, by task ID, from `events` (oldest first): the last record of each name,
/// in the order the names were first recorded. A change whose value is not a record is left out;
/// tasks with none are left out.
pub fn by_task(events: &[Event]) -> BTreeMap<String, Vec<Recorded>> {
    let mut out: BTreeMap<String, Vec<Recorded>> = BTreeMap::new();
    for e in events {
        let (Kind::Task, Some(task)) = (e.kind, e.id.as_deref()) else {
            continue;
        };
        for c in e.changes.iter().filter(|c| c.field == FIELD) {
            let Some(Ok(check)) = c.to.clone().map(serde_json::from_value::<CheckResult>) else {
                continue;
            };
            let rec = Recorded {
                check,
                at: e.at.clone(),
                by: e.by.clone(),
            };
            let list = out.entry(task.to_string()).or_default();
            match list.iter_mut().find(|r| r.check.name == rec.check.name) {
                Some(r) => *r = rec,
                None => list.push(rec),
            }
        }
    }
    out
}

/// `results` in the order they are shown: failed first, then not run, then passed (an unknown
/// result after them); within each, in the order first recorded.
pub fn ordered(results: &[Recorded]) -> Vec<Recorded> {
    let rank = |r: &Recorded| {
        RESULTS
            .iter()
            .position(|w| *w == r.check.result)
            .unwrap_or(RESULTS.len())
    };
    let mut out = results.to_vec();
    out.sort_by_key(rank);
    out
}

/// The change `alm check-result` makes on task `id`, and the record after it. Refused: an unknown
/// task, one someone else holds (the holder rule; done and dropped tasks may be recorded on too, a
/// result is often written down at the end), a check without a name (`check_name_missing`), and a
/// result that is none of the three (`check_result_unknown`). `None` as the change: the same
/// result and detail are recorded already.
pub fn record(
    taskbag: &Taskbag,
    recorded: &BTreeMap<String, Vec<Recorded>>,
    id: &str,
    me: Option<&str>,
    name: &str,
    result: &str,
    detail: Option<&str>,
) -> Result<(Option<Change>, CheckResult), Problem> {
    let task = find_task(taskbag, id)?;
    check_holder(&task, me)?;
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        return Err(Problem::CheckNameMissing);
    }
    let result = result_of(result)?;
    let detail = detail
        .map(|d| d.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|d| !d.is_empty());
    let after = CheckResult {
        name,
        result: result.to_string(),
        detail,
        extra: Default::default(),
    };
    let before = recorded
        .get(&task.id)
        .and_then(|list| list.iter().find(|r| r.check.name == after.name))
        .map(|r| r.check.clone());
    if before.as_ref().is_some_and(|b| b.result == after.result && b.detail == after.detail) {
        return Ok((None, after));
    }
    let value = |c: &CheckResult| serde_json::to_value(c).expect("a check result serializes");
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
    use serde_json::{Value, json};

    fn event(id: &str, at: &str, to: Value) -> Event {
        serde_json::from_value(json!({
            "at": at,
            "via": "alm",
            "command": COMMAND,
            "kind": "task",
            "id": id,
            "by": "s1",
            "changes": [ { "field": FIELD, "to": to } ],
        }))
        .unwrap()
    }

    #[test]
    fn a_result_has_one_written_form() {
        for (given, want) in [
            ("pass", "pass"),
            ("PASS", "pass"),
            (" Fail ", "fail"),
            ("not-run", "not-run"),
            ("NOT_RUN", "not-run"),
            ("not run", "not-run"),
        ] {
            assert_eq!(result_of(given).unwrap(), want, "{given}");
        }
        for given in ["", "ok", "skipped", "passes"] {
            assert!(
                matches!(result_of(given), Err(Problem::CheckResultUnknown { .. })),
                "{given}"
            );
        }
    }

    #[test]
    fn the_last_record_of_each_name_wins_in_first_order() {
        let events = vec![
            event("A01-1", "2026-11-20T00:00:01.000Z", json!({ "name": "npm test", "result": "fail", "detail": "2 failed" })),
            event("A01-1", "2026-11-20T00:00:02.000Z", json!({ "name": "cargo test", "result": "pass" })),
            event("A01-2", "2026-11-20T00:00:03.000Z", json!({ "name": "npm test", "result": "pass" })),
            event("A01-1", "2026-11-20T00:00:04.000Z", json!({ "name": "npm test", "result": "pass" })),
            event("A01-1", "2026-11-20T00:00:05.000Z", json!("not a record")),
        ];
        let rec = by_task(&events);
        let one = &rec["A01-1"];
        assert_eq!(one.len(), 2);
        assert_eq!(one[0].check.name, "npm test");
        assert_eq!(one[0].check.result, "pass");
        assert_eq!(one[0].check.detail, None);
        assert_eq!(one[0].at, "2026-11-20T00:00:04.000Z");
        assert_eq!(one[1].check.name, "cargo test");
        assert_eq!(rec["A01-2"].len(), 1);
    }

    #[test]
    fn failed_and_not_run_come_first() {
        let events = vec![
            event("A01-1", "t1", json!({ "name": "a", "result": "pass" })),
            event("A01-1", "t2", json!({ "name": "b", "result": "not-run" })),
            event("A01-1", "t3", json!({ "name": "c", "result": "pass" })),
            event("A01-1", "t4", json!({ "name": "d", "result": "fail" })),
        ];
        let names: Vec<String> = ordered(&by_task(&events)["A01-1"])
            .into_iter()
            .map(|r| r.check.name)
            .collect();
        assert_eq!(names, vec!["d", "b", "a", "c"]);
    }
}
