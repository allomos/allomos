//! Retention and redaction on disk (Task-A01-stem986): `alm redact` under the taskbag's lock, and
//! how many events are past the retention period. The rules are in [`allomos_core::retention`].

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Duration;

use allomos_core::canonical::fingerprint;
use allomos_core::event::Kind;
use allomos_core::retention::{self, RedactedFile, Redaction};
use allomos_core::{Issue, Problem, Task, ops};
use serde::Serialize;
use serde_json::Value;

use crate::events::{self, Actor, EVENTS_DIR, PACK_EXT};
use crate::lock_for_change;

/// What to remove: the task, its fields, optionally only some of its events.
#[derive(Debug, Default, Clone)]
pub struct Request<'a> {
    pub fields: &'a [String],
    /// `events/<name>.json` files; empty: every event of the task.
    pub events: &'a [String],
    pub reason: Option<&'a str>,
    pub approved_by: Option<&'a str>,
}

/// What [`redact`] did.
#[derive(Debug, Clone, PartialEq)]
pub struct Redacted {
    pub task: Task,
    /// `None`: nothing to remove (every past value is the current one or already removed), and
    /// no event was written.
    pub redaction: Option<Redaction>,
    pub issues: Vec<Issue>,
}

/// A new redaction's ID: `rd-` and six letters or digits.
pub fn new_id() -> String {
    format!("rd-{}", events::random_suffix())
}

/// Where an event is kept: its own file, or a line of a packed month.
enum Place {
    Single(std::path::PathBuf),
    Pack(std::path::PathBuf, usize),
}

/// Every event as written, with its file name (`events/<name>.json`) and where it is kept.
fn placed(root: &Path) -> Vec<(String, Value, Place)> {
    let mut out = Vec::new();
    for path in crate::sorted_entries(&root.join(EVENTS_DIR)) {
        let ext = path.extension().and_then(|e| e.to_str());
        let Ok(text) = crate::read_text(&path) else {
            continue;
        };
        if ext == Some(PACK_EXT) {
            for (n, line) in text.lines().enumerate() {
                let Some((file, event)) = serde_json::from_str::<Value>(line).ok().and_then(|v| {
                    Some((
                        v.get("file")?.as_str()?.to_string(),
                        v.get("event")?.clone(),
                    ))
                }) else {
                    continue;
                };
                out.push((
                    format!("{EVENTS_DIR}/{file}"),
                    event,
                    Place::Pack(path.clone(), n),
                ));
            }
        } else if ext == Some("json") {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if let Ok(v) = serde_json::from_str::<Value>(&text) {
                out.push((
                    format!("{EVENTS_DIR}/{name}"),
                    v,
                    Place::Single(path.clone()),
                ));
            }
        }
    }
    out
}

/// Remove the chosen fields' past values from task `id`'s history: refused while a legal hold
/// is on, without a field or a reason, for a field the task's history never changed, and for an
/// event file that is not the task's. One `redact` event records what was removed (field
/// names, files and their fingerprints before, who approved it, the reason), never the values.
pub fn redact(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    request: &Request,
) -> Result<Redacted, Problem> {
    let fields: Vec<String> = request
        .fields
        .iter()
        .map(|f| f.trim().to_string())
        .filter(|f| !f.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    if fields.is_empty() {
        return Err(Problem::RedactFieldMissing);
    }
    let reason = request
        .reason
        .map(str::trim)
        .filter(|r| !r.is_empty())
        .ok_or(Problem::RedactReasonMissing)?;
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    if retention::of(&bag.project).legal_hold {
        return Err(Problem::LegalHoldOn);
    }
    let task = ops::find_task(&bag, id)?;
    let current = serde_json::to_value(&task).expect("a task always serializes");
    let all = placed(root);
    let mine: Vec<usize> = (0..all.len())
        .filter(|&i| retention::about_task(&all[i].1, &task.id))
        .collect();
    let chosen: BTreeSet<&str> = request.events.iter().map(|e| e.trim()).collect();
    for file in &chosen {
        let full = if file.starts_with(&format!("{EVENTS_DIR}/")) {
            file.to_string()
        } else {
            format!("{EVENTS_DIR}/{file}")
        };
        if !mine.iter().any(|&i| all[i].0 == full) {
            return Err(Problem::RedactEventUnknown {
                id: task.id.clone(),
                file: file.to_string(),
            });
        }
    }
    let in_scope = |file: &str| {
        chosen.is_empty()
            || chosen
                .iter()
                .any(|c| file == *c || file == format!("{EVENTS_DIR}/{c}"))
    };
    let mut seen_fields = BTreeSet::new();
    for &i in &mine {
        if in_scope(&all[i].0) {
            seen_fields.extend(retention::fields_in(&all[i].1, &fields));
        }
    }
    if let Some(f) = fields.iter().find(|f| !seen_fields.contains(*f)) {
        return Err(Problem::RedactFieldUnknown {
            id: task.id.clone(),
            field: f.clone(),
        });
    }
    let rid = new_id();
    let mut files = Vec::new();
    let mut rewritten: Vec<(usize, Value)> = Vec::new();
    for &i in &mine {
        let (file, event, _) = &all[i];
        if !in_scope(file) {
            continue;
        }
        let mut after = event.clone();
        let removed = retention::redact_event(&mut after, &fields, &current, &rid);
        if removed.is_empty() {
            continue;
        }
        files.push(RedactedFile {
            file: file.clone(),
            fingerprint: fingerprint(event),
            fields: removed,
        });
        rewritten.push((i, after));
    }
    if files.is_empty() {
        return Ok(Redacted {
            task,
            redaction: None,
            issues,
        });
    }
    let redaction = Redaction {
        id: rid,
        fields,
        files,
        approved_by: request
            .approved_by
            .map(str::trim)
            .filter(|a| !a.is_empty())
            .map(String::from),
    };
    let event = actor.event(
        retention::COMMAND,
        Kind::Task,
        Some(&task.id),
        vec![
            retention::change(&redaction),
            allomos_core::event::reason_change(reason),
        ],
    );
    // The event is written first, so its own chain points to the fingerprints before.
    lock.record(&event, |_| write_back(&all, rewritten))?;
    Ok(Redacted {
        task,
        redaction: Some(redaction),
        issues,
    })
}

fn write_back(
    all: &[(String, Value, Place)],
    rewritten: Vec<(usize, Value)>,
) -> Result<(), Problem> {
    let mut packs: BTreeMap<std::path::PathBuf, BTreeMap<usize, Value>> = BTreeMap::new();
    for (i, after) in rewritten {
        match &all[i].2 {
            Place::Single(path) => crate::write_atomic(path, &crate::json_text(&after))?,
            Place::Pack(path, line) => {
                packs.entry(path.clone()).or_default().insert(*line, after);
            }
        }
    }
    for (path, lines) in packs {
        let text = crate::read_text(&path).map_err(|e| Problem::ReadFailed {
            detail: format!("{}: {e}", path.display()),
        })?;
        let mut out = String::new();
        for (n, line) in text.lines().enumerate() {
            match (lines.get(&n), serde_json::from_str::<Value>(line)) {
                (Some(event), Ok(mut v)) => {
                    v["event"] = event.clone();
                    out.push_str(&serde_json::to_string(&v).expect("events serialize"));
                }
                _ => out.push_str(line),
            }
            out.push('\n');
        }
        crate::write_atomic(&path, &out)?;
    }
    Ok(())
}

/// How much of the history is older than the retention period.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PastRetention {
    /// The first day still inside the period (`YYYY-MM-DD`).
    pub cutoff: String,
    /// Events written before it.
    pub events: usize,
    /// The oldest such event's day.
    pub oldest: Option<String>,
}

/// The events older than a retention of `years`, counted back from `today` (`YYYY-MM-DD`).
pub fn past_retention(root: &Path, years: u32, today: &str) -> Option<PastRetention> {
    let cutoff = retention::cutoff(years, today)?;
    let mut events = 0;
    let mut oldest: Option<String> = None;
    for (file, _) in crate::trail::raw_events(root).0 {
        // Event files are named by their UTC time: `events/2026-11-08-…`.
        let Some(day) = file
            .strip_prefix(&format!("{EVENTS_DIR}/"))
            .and_then(|n| n.get(..10))
        else {
            continue;
        };
        if day < cutoff.as_str() {
            events += 1;
            if oldest.as_deref().is_none_or(|o| day < o) {
                oldest = Some(day.to_string());
            }
        }
    }
    Some(PastRetention {
        cutoff,
        events,
        oldest,
    })
}
