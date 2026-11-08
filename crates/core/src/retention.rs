//! Retention, legal hold and redaction (Task-A01-stem986, decision 0021 § 1.2 and § 6 item 5).
//!
//! - **Retention** (`project.json`'s `audit.retention_years`): how long the history is meant to
//!   be kept, forever (the default, not written) or a number of years. Allomos removes nothing on
//!   its own when records pass it: `alm audit-settings` says how many events are older than the
//!   period ("`alm` tells; the agent checks").
//! - **Legal hold** (`audit.legal_hold`): while it is on, nothing in the history may be removed
//!   or redacted. Turning it on or off asks for a reason.
//! - **Redaction** (`alm redact`): a personal detail written into a task by mistake is removed
//!   from the task's history. Every past value of the chosen fields that differs from the
//!   field's current value is replaced by a marker, `{"redacted": "<redaction ID>"}`, and one
//!   `redact` event records which fields, which event files (with each file's fingerprint
//!   before), who approved it and why, never the old values. The current value stays: the
//!   person corrects it first (`alm describe`, `alm title`, …).
//!
//! A redacted event no longer matches its signature, and the next event of the same computer
//! points to its fingerprint before; `alm audit` lists such events as redacted, from the
//! recorded fingerprints, instead of as changed or as a gap. The signature and that fingerprint
//! remain, so someone who guesses the exact old value can still confirm the guess; nothing in
//! the files says what it was.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::event::{self, Change};
use crate::model::Project;
use crate::{Problem, audit::Audit};

/// `project.json`'s `audit` field for the retention period, in years.
pub const RETENTION: &str = "retention_years";
/// `project.json`'s `audit` field for the legal hold.
pub const LEGAL_HOLD: &str = "legal_hold";
/// The command of a redaction's event.
pub const COMMAND: &str = "redact";
/// The change a redaction's event records.
pub const FIELD: &str = "redaction";
/// The key of the marker that replaces a removed value.
pub const MARKER: &str = "redacted";
/// The longest retention period accepted, in years.
pub const MAX_YEARS: u32 = 100;

/// The retention settings as they apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Retention {
    /// `None`: forever.
    pub years: Option<u32>,
    pub legal_hold: bool,
}

/// The retention settings of `project`.
pub fn of(project: &Project) -> Retention {
    let audit = project.audit.as_ref();
    Retention {
        years: audit.and_then(|a| a.retention_years),
        legal_hold: audit.is_some_and(|a| a.legal_hold),
    }
}

/// Read a retention period as given to `alm`: `forever`, or a number of years (`7`, `7y`).
pub fn parse(value: &str) -> Result<Option<u32>, Problem> {
    let v = value.trim().to_lowercase();
    if v == "forever" {
        return Ok(None);
    }
    let digits = v
        .strip_suffix("years")
        .or_else(|| v.strip_suffix("year"))
        .or_else(|| v.strip_suffix('y'))
        .unwrap_or(&v)
        .trim();
    match digits.parse::<u32>() {
        Ok(n) if (1..=MAX_YEARS).contains(&n) => Ok(Some(n)),
        _ => Err(Problem::RetentionInvalid {
            value: value.to_string(),
        }),
    }
}

/// The first day still inside a retention of `years`, counted back from `today`
/// (`YYYY-MM-DD`): the same day `years` earlier (29 February becomes 28 February).
pub fn cutoff(years: u32, today: &str) -> Option<String> {
    let y: i64 = today.get(..4)?.parse().ok()?;
    let rest = today.get(4..10)?;
    let y = y - i64::from(years);
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let rest = if rest == "-02-29" && !leap {
        "-02-28"
    } else {
        rest
    };
    Some(format!("{y:04}{rest}"))
}

/// A change to the retention settings.
#[derive(Debug, Default, Clone, Copy)]
pub struct RetentionChange<'a> {
    /// `Some(None)`: forever.
    pub years: Option<Option<u32>>,
    pub legal_hold: Option<bool>,
    pub reason: Option<&'a str>,
}

/// `project` with its retention settings changed, and the event's changes (the `audit` field's
/// from and to, and the reason). A shorter retention needs a reason (turning a control down,
/// decision 0021 § 6); turning the legal hold on or off needs one too (§ 6 item 5).
pub fn set(project: &Project, change: &RetentionChange) -> Result<(Project, Vec<Change>), Problem> {
    let now = of(project);
    let years = change.years.unwrap_or(now.years);
    let hold = change.legal_hold.unwrap_or(now.legal_hold);
    let reason = change.reason.map(str::trim).filter(|r| !r.is_empty());
    let shorter = match (now.years, years) {
        (None, Some(_)) => true,
        (Some(a), Some(b)) => b < a,
        _ => false,
    };
    if reason.is_none() {
        if shorter {
            return Err(Problem::AuditReasonMissing {
                setting: "retention".into(),
            });
        }
        if hold != now.legal_hold {
            return Err(Problem::AuditReasonMissing {
                setting: "legal hold".into(),
            });
        }
    }
    let mut audit = project.audit.clone().unwrap_or_default();
    audit.retention_years = years;
    audit.legal_hold = hold;
    let empty = audit == Audit::default();
    let after = Project {
        audit: (!empty).then_some(audit),
        ..project.clone()
    };
    let mut changes: Vec<Change> = event::changes(Some(project), &after, &[])
        .into_iter()
        .filter(|c| c.field == "audit")
        .collect();
    if let (false, Some(why)) = (changes.is_empty(), reason) {
        changes.push(event::reason_change(why));
    }
    Ok((after, changes))
}

/// One event file a redaction changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactedFile {
    /// `events/<name>.json` (an event in a packed month keeps the name of its own file).
    pub file: String,
    /// The event's fingerprint before the redaction ([`crate::canonical::fingerprint`]).
    pub fingerprint: String,
    /// The fields removed from it.
    pub fields: Vec<String>,
}

/// What a `redact` event records: never the old values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Redaction {
    /// `rd-` and six letters or digits.
    pub id: String,
    pub fields: Vec<String>,
    pub files: Vec<RedactedFile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_by: Option<String>,
}

/// The marker that replaces a removed value.
pub fn marker(id: &str) -> Value {
    serde_json::json!({ MARKER: id })
}

/// Whether `value` is a redaction's marker.
pub fn is_marker(value: &Value) -> bool {
    value
        .as_object()
        .is_some_and(|m| m.len() == 1 && m.contains_key(MARKER))
}

/// Whether the raw event `event` is about task `id`.
pub fn about_task(event: &Value, id: &str) -> bool {
    event.get("kind").and_then(Value::as_str) == Some("task")
        && event.get("id").and_then(Value::as_str) == Some(id)
}

/// The fields of `fields` that the raw event `event` has a change of.
pub fn fields_in(event: &Value, fields: &[String]) -> Vec<String> {
    let mut out: Vec<String> = changes_of(event)
        .filter_map(|c| c.get("field").and_then(Value::as_str))
        .filter(|f| fields.iter().any(|x| x == f))
        .map(String::from)
        .collect();
    out.sort();
    out.dedup();
    out
}

fn changes_of(event: &Value) -> impl Iterator<Item = &Value> {
    event
        .get("changes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

/// Replace, in the raw event `event`, every `from` and `to` of a change of one of `fields` that
/// differs from the field's current value in `current` (the task as JSON) by the marker of
/// redaction `id`. Returns the fields removed (none: the event is unchanged).
pub fn redact_event(
    event: &mut Value,
    fields: &[String],
    current: &Value,
    id: &str,
) -> Vec<String> {
    let mut removed = Vec::new();
    let Some(changes) = event.get_mut("changes").and_then(Value::as_array_mut) else {
        return removed;
    };
    for change in changes {
        let Some(field) = change
            .get("field")
            .and_then(Value::as_str)
            .map(String::from)
        else {
            continue;
        };
        if !fields.contains(&field) {
            continue;
        }
        let now = current.get(&field).filter(|v| !v.is_null());
        for side in ["from", "to"] {
            let Some(v) = change.get_mut(side) else {
                continue;
            };
            if is_marker(v) || Some(&*v) == now {
                continue;
            }
            *v = marker(id);
            removed.push(field.clone());
        }
    }
    removed.sort();
    removed.dedup();
    removed
}

/// Every redaction recorded in the raw events `events`, with the file of its event.
pub fn all(events: &[(String, Value)]) -> Vec<(String, Redaction)> {
    events
        .iter()
        .filter(|(_, e)| e.get("command").and_then(Value::as_str) == Some(COMMAND))
        .flat_map(|(file, e)| {
            changes_of(e)
                .filter(|c| c.get("field").and_then(Value::as_str) == Some(FIELD))
                .filter_map(|c| serde_json::from_value::<Redaction>(c.get("to")?.clone()).ok())
                .map(|r| (file.clone(), r))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The `redaction` change of `redaction`.
pub fn change(redaction: &Redaction) -> Change {
    Change {
        extra: Default::default(),
        field: FIELD.to_string(),
        from: None,
        to: Some(serde_json::to_value(redaction).expect("a redaction always serializes")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn project(audit: Option<Audit>) -> Project {
        let p: Project = serde_json::from_value(json!({
            "format": crate::FORMAT, "id": "p", "name": "P"
        }))
        .unwrap();
        Project { audit, ..p }
    }

    #[test]
    fn periods_are_read_and_refused() {
        assert_eq!(parse("forever").unwrap(), None);
        assert_eq!(parse("7").unwrap(), Some(7));
        assert_eq!(parse("7y").unwrap(), Some(7));
        assert_eq!(parse(" 2 years ").unwrap(), Some(2));
        for bad in ["0", "-1", "half", "101", "6m"] {
            assert_eq!(
                parse(bad).unwrap_err(),
                Problem::RetentionInvalid { value: bad.into() }
            );
        }
        assert_eq!(cutoff(7, "2026-11-08").as_deref(), Some("2019-11-08"));
        assert_eq!(cutoff(1, "2028-02-29").as_deref(), Some("2027-02-28"));
    }

    #[test]
    fn the_default_is_forever_without_a_hold_and_is_not_written() {
        let p = project(None);
        assert_eq!(
            of(&p),
            Retention {
                years: None,
                legal_hold: false
            }
        );
        let (after, changes) = set(
            &p,
            &RetentionChange {
                years: Some(None),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(after.audit, None);
        assert!(changes.is_empty());
    }

    #[test]
    fn shortening_and_the_hold_need_a_reason() {
        let p = project(None);
        let seven = RetentionChange {
            years: Some(Some(7)),
            ..Default::default()
        };
        assert_eq!(
            set(&p, &seven).unwrap_err(),
            Problem::AuditReasonMissing {
                setting: "retention".into()
            }
        );
        let (p, changes) = set(
            &p,
            &RetentionChange {
                reason: Some("company policy"),
                ..seven
            },
        )
        .unwrap();
        assert_eq!(of(&p).years, Some(7));
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[1].field, event::REASON);
        // Longer needs none.
        let (p, changes) = set(
            &p,
            &RetentionChange {
                years: Some(Some(10)),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(of(&p).years, Some(10));
        assert_eq!(changes.len(), 1);
        let hold = RetentionChange {
            legal_hold: Some(true),
            ..Default::default()
        };
        assert_eq!(
            set(&p, &hold).unwrap_err(),
            Problem::AuditReasonMissing {
                setting: "legal hold".into()
            }
        );
        let (p, _) = set(
            &p,
            &RetentionChange {
                reason: Some("litigation"),
                ..hold
            },
        )
        .unwrap();
        assert!(of(&p).legal_hold);
        assert!(
            serde_json::to_string(&p)
                .unwrap()
                .contains("\"legal_hold\":true")
        );
    }

    #[test]
    fn redaction_replaces_past_values_and_keeps_the_current_one() {
        let mut e = json!({
            "at": "2026-11-08T00:00:00Z", "via": "alm", "command": "describe",
            "kind": "task", "id": "A01-01",
            "changes": [
                {"field": "description", "from": "call me at <PHONE>", "to": "fixed"},
                {"field": "title", "from": "a", "to": "b"}
            ]
        });
        assert!(about_task(&e, "A01-01"));
        let fields = vec!["description".to_string()];
        assert_eq!(fields_in(&e, &fields), fields);
        let current = json!({"description": "fixed", "title": "b"});
        assert_eq!(redact_event(&mut e, &fields, &current, "rd-abc123"), fields);
        assert_eq!(e["changes"][0]["from"], marker("rd-abc123"));
        assert_eq!(e["changes"][0]["to"], "fixed");
        assert_eq!(e["changes"][1]["from"], "a");
        assert!(!e.to_string().contains("PHONE"));
        // A second time changes nothing.
        assert!(redact_event(&mut e, &fields, &current, "rd-zzz999").is_empty());
        let r = Redaction {
            id: "rd-abc123".into(),
            fields: fields.clone(),
            files: vec![RedactedFile {
                file: "events/x.json".into(),
                fingerprint: "f".into(),
                fields,
            }],
            approved_by: Some("pat".into()),
        };
        let ev = json!({"command": COMMAND, "changes": [change(&r)]});
        assert_eq!(
            all(&[("events/y.json".into(), ev)]),
            vec![("events/y.json".into(), r)]
        );
    }
}
