//! A task's change type and risk (Task-A01-palm011, decision 0021 § 1.1 "Risk class and change
//! type", approver 2026-11-08).
//!
//! A task may record:
//! - its **change type**: `standard` (a pre-approved routine change), `normal` (approved before it
//!   is made) or `emergency` (made first, approved afterwards, by a deadline);
//! - its **risk**: `low`, `medium` or `high`;
//! - what it **touches**: `production` and/or `others_data` (other people's data), or nothing.
//!
//! Like the traces (Task-A01-fennel617) they live only in the task's history, each as an event
//! change of its own (`change_type`, `risk`, `touches`, and for an emergency `approval_due`, the
//! last day to record its approval), so `task.json` gains no field and the format stays as it is.
//! The last value recorded wins; a change with only a `from` clears it.
//!
//! An emergency counts as **approved afterwards** once an approval (`alm approve`,
//! Task-A01-coach782) that covers the task and is not withdrawn was recorded at or after the time
//! it was marked emergency. Emergencies without one are listed (`alm human`, `alm show`), with
//! their deadline and whether it has passed: `alm` tells, it refuses nothing.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::approval::{Recorded as Approval, Status};
use crate::event::{Change, Event, Kind};
use crate::ops::{check_holder, find_task};
use crate::validate::valid_date;
use crate::{Problem, Taskbag};

/// The command that records them (`alm classify`).
pub const COMMAND: &str = "classify";
/// The fields, each a change of its own.
pub const CHANGE_TYPE: &str = "change_type";
pub const RISK: &str = "risk";
pub const TOUCHES: &str = "touches";
pub const APPROVAL_DUE: &str = "approval_due";

/// The change types, in their order.
pub const CHANGE_TYPES: [&str; 3] = ["standard", "normal", "emergency"];
/// The risk levels, lowest first.
pub const RISKS: [&str; 3] = ["low", "medium", "high"];
/// What a change can touch.
pub const TOUCHED: [&str; 2] = ["production", "others_data"];
/// The emergency change type.
pub const EMERGENCY: &str = "emergency";
/// Days after it is marked emergency until its approval is due, when no `--due` is given.
pub const DEFAULT_DUE_DAYS: i64 = 3;

/// A task's classification as recorded, from the history.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Classification {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub change_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk: Option<String>,
    /// Empty: none recorded. `["none"]` is never kept: touching nothing is recorded as an empty
    /// list and read as [`Classification::touches_recorded`].
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub touches: Vec<String>,
    /// Whether `touches` was recorded at all (an empty list then means "touches neither").
    #[serde(skip)]
    pub touches_recorded: bool,
    /// For an emergency: the last day (`YYYY-MM-DD`, UTC) to record its approval.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_due: Option<String>,
    /// For an emergency: when it was marked so (UTC).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emergency_since: Option<String>,
}

impl Classification {
    pub fn is_empty(&self) -> bool {
        self.change_type.is_none() && self.risk.is_none() && !self.touches_recorded
    }

    pub fn is_emergency(&self) -> bool {
        self.change_type.as_deref() == Some(EMERGENCY)
    }
}

/// What `alm classify` was given; `None` leaves that one as it is.
#[derive(Debug, Clone, Default)]
pub struct Given<'a> {
    pub change_type: Option<&'a str>,
    pub risk: Option<&'a str>,
    /// `Some(["none"])` or `Some([])`: touches neither.
    pub touches: Option<&'a [String]>,
    /// The approval's deadline, for an emergency.
    pub due: Option<&'a str>,
}

fn invalid(field: &str, value: &str, allowed: &[&str]) -> Problem {
    Problem::ClassifyInvalid {
        field: field.to_string(),
        value: value.to_string(),
        allowed: allowed.join(", "),
    }
}

/// One value in its written form (lowercase, `-` read as `_`), when it is one of `allowed`.
fn one_of(field: &str, given: &str, allowed: &[&str]) -> Result<String, Problem> {
    let v = given.trim().to_ascii_lowercase().replace('-', "_");
    if allowed.contains(&v.as_str()) {
        Ok(v)
    } else {
        Err(invalid(field, given, allowed))
    }
}

/// What a task touches, in its written form and order: `production` before `others_data`;
/// `none` (alone) is the empty list. Each may be given separately or comma-separated.
pub fn touches(given: &[String]) -> Result<Vec<String>, Problem> {
    let mut allowed: Vec<&str> = TOUCHED.to_vec();
    allowed.push("none");
    let mut out = Vec::new();
    let mut none = false;
    for part in given.iter().flat_map(|g| g.split(',')) {
        if part.trim().is_empty() {
            continue;
        }
        let v = one_of(TOUCHES, part, &allowed)?;
        if v == "none" {
            none = true;
        } else if !out.contains(&v) {
            out.push(v);
        }
    }
    if none && !out.is_empty() {
        return Err(Problem::BadUsage {
            detail: "--touches none cannot be given with production or others-data.".into(),
        });
    }
    out.sort_by_key(|v| TOUCHED.iter().position(|t| t == v));
    Ok(out)
}

/// Every task's classification, by task ID, from `events` (oldest first). Tasks with none are
/// left out.
pub fn recorded(events: &[Event]) -> BTreeMap<String, Classification> {
    let mut out: BTreeMap<String, Classification> = BTreeMap::new();
    for e in events {
        let (Kind::Task, Some(id)) = (e.kind, e.id.as_deref()) else {
            continue;
        };
        for c in &e.changes {
            let text = c.to.as_ref().and_then(Value::as_str).map(str::to_string);
            match c.field.as_str() {
                CHANGE_TYPE => {
                    let entry = out.entry(id.to_string()).or_default();
                    let was_emergency = entry.is_emergency();
                    entry.change_type = text;
                    if entry.is_emergency() {
                        if !was_emergency {
                            entry.emergency_since = Some(e.at.clone());
                        }
                    } else {
                        entry.emergency_since = None;
                    }
                }
                RISK => out.entry(id.to_string()).or_default().risk = text,
                APPROVAL_DUE => out.entry(id.to_string()).or_default().approval_due = text,
                TOUCHES => {
                    let entry = out.entry(id.to_string()).or_default();
                    match &c.to {
                        Some(Value::Array(items)) => {
                            entry.touches = items
                                .iter()
                                .filter_map(|v| v.as_str().map(str::to_string))
                                .collect();
                            entry.touches_recorded = true;
                        }
                        _ => {
                            entry.touches = Vec::new();
                            entry.touches_recorded = false;
                        }
                    }
                }
                _ => {}
            }
        }
    }
    out.retain(|_, c| !c.is_empty() || c.approval_due.is_some());
    out
}

/// What `alm classify` changes on task `id`, given `today` (`YYYY-MM-DD`, UTC). Refused for an
/// unknown task, one someone else holds (the holder rule), a value not in its list
/// (`classify_invalid`), a date that is not one, and `--due` on a task that is not (or not becoming)
/// an emergency. Done and dropped tasks may be classified too. Marking a task emergency without
/// `--due` sets the deadline to [`DEFAULT_DUE_DAYS`] days after today; leaving emergency clears it.
/// Returns the changes (none when nothing changes) and the classification after them (its
/// `emergency_since` left as it was; the caller reads it back from the history).
pub fn set(
    taskbag: &Taskbag,
    recorded: &BTreeMap<String, Classification>,
    id: &str,
    me: Option<&str>,
    given: &Given,
    today: &str,
) -> Result<(Vec<Change>, Classification), Problem> {
    let task = find_task(taskbag, id)?;
    check_holder(&task, me)?;
    let before = recorded.get(&task.id).cloned().unwrap_or_default();
    let mut after = before.clone();
    if let Some(t) = given.change_type {
        after.change_type = Some(one_of(CHANGE_TYPE, t, &CHANGE_TYPES)?);
    }
    if let Some(r) = given.risk {
        after.risk = Some(one_of(RISK, r, &RISKS)?);
    }
    if let Some(list) = given.touches {
        after.touches = touches(list)?;
        after.touches_recorded = true;
    }
    if let Some(due) = given.due {
        let due = due.trim();
        if !valid_date(due) {
            return Err(Problem::InvalidDate {
                value: due.to_string(),
            });
        }
        if !after.is_emergency() {
            return Err(Problem::BadUsage {
                detail: format!(
                    "--due is the deadline for an emergency change's approval, and {} is not one: add --type emergency.",
                    task.id
                ),
            });
        }
        after.approval_due = Some(due.to_string());
    } else if after.is_emergency() && after.approval_due.is_none() {
        after.approval_due = Some(add_days(today, DEFAULT_DUE_DAYS));
    }
    if !after.is_emergency() {
        after.approval_due = None;
    }

    let text = |v: &Option<String>| v.clone().map(Value::String);
    let list = |l: &[String]| Value::Array(l.iter().cloned().map(Value::String).collect());
    let mut changes = Vec::new();
    let mut push = |field: &str, from: Option<Value>, to: Option<Value>| {
        if from != to {
            changes.push(Change {
                extra: Default::default(),
                field: field.to_string(),
                from,
                to,
            });
        }
    };
    push(
        CHANGE_TYPE,
        text(&before.change_type),
        text(&after.change_type),
    );
    push(RISK, text(&before.risk), text(&after.risk));
    push(
        TOUCHES,
        before.touches_recorded.then(|| list(&before.touches)),
        after.touches_recorded.then(|| list(&after.touches)),
    );
    push(
        APPROVAL_DUE,
        text(&before.approval_due),
        text(&after.approval_due),
    );
    Ok((changes, after))
}

/// An emergency change still to be approved afterwards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Unapproved {
    pub task: String,
    /// When it was marked emergency (UTC).
    pub since: String,
    /// The last day to record its approval, when known.
    pub due: Option<String>,
    /// Past `due` on the day asked.
    pub overdue: bool,
}

/// The approval that approves emergency `c` afterwards, of `approvals` (the task's, oldest
/// first): the first one not withdrawn and recorded at or after it was marked emergency.
pub fn approved_afterwards<'a>(
    c: &Classification,
    approvals: &'a [Approval],
) -> Option<&'a Approval> {
    let since = c.emergency_since.as_deref()?;
    approvals
        .iter()
        .find(|a| a.status != Status::Withdrawn && a.approval.at.as_str() >= since)
}

/// Every emergency without an approval recorded afterwards, by task ID, on `today`.
pub fn unapproved(
    recorded: &BTreeMap<String, Classification>,
    approvals: &BTreeMap<String, Vec<Approval>>,
    today: &str,
) -> Vec<Unapproved> {
    let none = Vec::new();
    recorded
        .iter()
        .filter(|(_, c)| c.is_emergency())
        .filter(|(id, c)| approved_afterwards(c, approvals.get(*id).unwrap_or(&none)).is_none())
        .map(|(id, c)| Unapproved {
            task: id.clone(),
            since: c.emergency_since.clone().unwrap_or_default(),
            due: c.approval_due.clone(),
            overdue: c.approval_due.as_deref().is_some_and(|d| today > d),
        })
        .collect()
}

/// `date` (`YYYY-MM-DD`) plus `days`; `date` itself when it is not a date.
pub fn add_days(date: &str, days: i64) -> String {
    if !valid_date(date) {
        return date.to_string();
    }
    let num = |r: std::ops::Range<usize>| date[r].parse::<i64>().unwrap_or(0);
    let (y, m, d) = (num(0..4), num(5..7), num(8..10));
    // Howard Hinnant's days_from_civil and civil_from_days.
    let y0 = if m <= 2 { y - 1 } else { y };
    let era = y0.div_euclid(400);
    let yoe = y0 - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let z = era * 146_097 + doe - 719_468 + days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::approval::{Approval as Record, Scope};
    use serde_json::json;

    fn event(at: &str, id: &str, changes: Value) -> Event {
        serde_json::from_value(json!({
            "at": at,
            "via": "alm",
            "command": COMMAND,
            "kind": "task",
            "id": id,
            "changes": changes,
        }))
        .unwrap()
    }

    fn approval(at: &str, status: Status) -> Approval {
        Approval {
            approval: Record {
                id: "ap-abc123".into(),
                approver: None,
                scope: Scope {
                    tasks: vec!["A01-1".into()],
                    plan_sha256: "x".into(),
                    plan: "description".into(),
                    extra: Default::default(),
                },
                at: at.into(),
                channel: "chat".into(),
                words: Some("ok".into()),
                expires: None,
                extra: Default::default(),
            },
            recorded_by: None,
            withdrawn_at: None,
            withdrawn_by: None,
            status,
        }
    }

    #[test]
    fn values_have_one_written_form() {
        assert_eq!(
            one_of(CHANGE_TYPE, " Emergency ", &CHANGE_TYPES).unwrap(),
            "emergency"
        );
        assert!(matches!(
            one_of(RISK, "critical", &RISKS),
            Err(Problem::ClassifyInvalid { .. })
        ));
        assert_eq!(
            touches(&["others-data,production".into()]).unwrap(),
            vec!["production", "others_data"]
        );
        assert_eq!(touches(&["none".into()]).unwrap(), Vec::<String>::new());
        assert!(touches(&["none".into(), "production".into()]).is_err());
        assert!(touches(&["prod".into()]).is_err());
    }

    #[test]
    fn days_are_added_across_months_and_years() {
        assert_eq!(add_days("2026-11-08", 3), "2026-11-11");
        assert_eq!(add_days("2026-12-30", 3), "2027-01-02");
        assert_eq!(add_days("2028-02-27", 3), "2028-03-01");
        assert_eq!(add_days("2026-02-27", 2), "2026-03-01");
    }

    #[test]
    fn the_last_value_wins_and_an_emergency_is_dated() {
        let events = vec![
            event(
                "2026-11-08T01:00:00.000Z",
                "A01-1",
                json!([
                    { "field": CHANGE_TYPE, "to": "normal" },
                    { "field": RISK, "to": "low" },
                    { "field": TOUCHES, "to": [] },
                ]),
            ),
            event(
                "2026-11-08T02:00:00.000Z",
                "A01-1",
                json!([
                    { "field": CHANGE_TYPE, "from": "normal", "to": "emergency" },
                    { "field": APPROVAL_DUE, "to": "2026-11-11" },
                ]),
            ),
            event(
                "2026-11-08T03:00:00.000Z",
                "A01-1",
                json!([{ "field": RISK, "from": "low", "to": "high" }]),
            ),
        ];
        let rec = recorded(&events);
        let c = &rec["A01-1"];
        assert_eq!(c.change_type.as_deref(), Some("emergency"));
        assert_eq!(c.risk.as_deref(), Some("high"));
        assert!(c.touches_recorded && c.touches.is_empty());
        assert_eq!(
            c.emergency_since.as_deref(),
            Some("2026-11-08T02:00:00.000Z")
        );

        // Not approved afterwards: listed, and overdue after the 11th.
        let none = BTreeMap::new();
        let list = unapproved(&rec, &none, "2026-11-12");
        assert_eq!(list.len(), 1);
        assert!(list[0].overdue);
        assert!(!unapproved(&rec, &none, "2026-11-11")[0].overdue);

        // An approval recorded before it was marked emergency does not count; a withdrawn one
        // neither; one recorded afterwards does.
        let mut approvals = BTreeMap::new();
        approvals.insert(
            "A01-1".to_string(),
            vec![
                approval("2026-11-08T01:30:00.000Z", Status::Applies),
                approval("2026-11-08T04:00:00.000Z", Status::Withdrawn),
            ],
        );
        assert_eq!(unapproved(&rec, &approvals, "2026-11-09").len(), 1);
        approvals
            .get_mut("A01-1")
            .unwrap()
            .push(approval("2026-11-08T05:00:00.000Z", Status::PlanChanged));
        assert!(unapproved(&rec, &approvals, "2026-11-09").is_empty());
    }
}
