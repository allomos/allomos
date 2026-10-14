//! The taskbag's `events/` folder: one small file per change (R7, Task-A01-17).
//!
//! Each file is named by the time of the change plus a random suffix,
//! `2026-10-13-140205-123-k7f3q2.json`, so two people's changes never get the same name and Git
//! merges them without a conflict. Events are written under the taskbag's lock, together with the
//! change they describe; they are read without a lock, like everything else.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use allomos_core::event::{Change, Event, Kind};
use allomos_core::{Issue, Problem, parse_event};
use serde::Serialize;

use crate::{json_text, sorted_entries, write_atomic, write_failed};

pub const EVENTS_DIR: &str = "events";

/// Who makes a change, and through which writer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Actor<'a> {
    /// The `--as` name, when one was given.
    pub by: Option<&'a str>,
    /// `alm` or `app`.
    pub via: &'static str,
}

impl<'a> Actor<'a> {
    /// The `alm` command, run as `by`.
    pub fn alm(by: Option<&'a str>) -> Self {
        Actor { by, via: "alm" }
    }

    /// The Allomos app. It does not know the person's name yet.
    pub const APP: Actor<'static> = Actor {
        by: None,
        via: "app",
    };

    /// An event made now by this actor.
    pub fn event(
        &self,
        command: &str,
        kind: Kind,
        id: Option<&str>,
        changes: Vec<Change>,
    ) -> Event {
        Event {
            at: now().0,
            by: self.by.map(String::from),
            via: self.via.to_string(),
            command: command.to_string(),
            kind,
            id: id.map(String::from),
            changes,
        }
    }
}

/// Now, in UTC: the event's `at` (`2026-10-13T14:02:05Z`) and the start of its file name
/// (`2026-10-13-140205-123`, with milliseconds, so files sort in the order they were written).
/// Tests may fix the time with `ALLOMOS_NOW` (`2026-10-13T14:02:05Z` or `2026-10-13T14:02Z`).
fn now() -> (String, String) {
    let since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let millis = since.subsec_millis();
    if let Ok(fixed) = std::env::var("ALLOMOS_NOW") {
        let stem = format!(
            "{}-{millis:03}",
            fixed.replace('T', "-").replace([':', 'Z'], "")
        );
        return (fixed, stem);
    }
    let secs = since.as_secs() as i64;
    let (days, rest) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let (y, m, d) = crate::handoff::civil_from_days(days);
    let (hh, mm, ss) = (rest / 3600, rest % 3600 / 60, rest % 60);
    (
        format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z"),
        format!("{y:04}-{m:02}-{d:02}-{hh:02}{mm:02}{ss:02}-{millis:03}"),
    )
}

/// Six random lowercase letters and digits, from the standard library's random hash keys.
fn suffix() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
    );
    h.write_u32(std::process::id());
    let mut n = h.finish();
    (0..6)
        .map(|_| {
            let c = b"abcdefghijklmnopqrstuvwxyz0123456789"[(n % 36) as usize];
            n /= 36;
            c as char
        })
        .collect()
}

/// A free file name for an event written now, relative to the taskbag: `events/<stem>-<suffix>.json`.
pub(crate) fn new_file(root: &Path) -> String {
    let stem = now().1;
    loop {
        let name = format!("{EVENTS_DIR}/{stem}-{}.json", suffix());
        if !root.join(&name).exists() {
            return name;
        }
    }
}

/// Write one event. Returns its file, so a change that then fails can take it back.
/// Callers hold the taskbag's lock.
pub(crate) fn write(root: &Path, event: &Event) -> Result<PathBuf, Problem> {
    let dir = root.join(EVENTS_DIR);
    fs::create_dir_all(&dir).map_err(|e| write_failed(&dir, e))?;
    let path = root.join(new_file(root));
    write_atomic(&path, &json_text(event))?;
    Ok(path)
}

/// Every event in the taskbag, oldest first, plus the files that could not be read.
pub fn list(root: &Path) -> (Vec<Event>, Vec<Issue>) {
    let mut events = Vec::new();
    let mut issues = Vec::new();
    for path in sorted_entries(&root.join(EVENTS_DIR)) {
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let file = format!("{EVENTS_DIR}/{name}");
        match fs::read_to_string(&path) {
            Ok(text) => match parse_event(&file, &text) {
                Ok(e) => events.push((file, e)),
                Err(i) => issues.push(i),
            },
            Err(e) => issues.push(Issue::new(
                &file,
                None,
                Problem::ReadFailed {
                    detail: e.to_string(),
                },
            )),
        }
    }
    // By time, then by file name (which has milliseconds and is unique).
    events.sort_by(|(fa, a), (fb, b)| a.at.cmp(&b.at).then(fa.cmp(fb)));
    (events.into_iter().map(|(_, e)| e).collect(), issues)
}

/// One task's history.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct History {
    /// When this taskbag started recording: its oldest event. `None` when it has none yet.
    pub since: Option<String>,
    /// The task's events, newest first.
    pub events: Vec<Event>,
    /// Event files that could not be read.
    pub issues: Vec<Issue>,
}

/// The history of the task `id`. Changes made before the taskbag started recording have no
/// events; none are invented (owner, 2026-10-13).
pub fn history(root: &Path, id: &str) -> History {
    let (all, issues) = list(root);
    let since = all.first().map(|e| e.at.clone());
    let mut events: Vec<Event> = all
        .into_iter()
        .filter(|e| e.kind == Kind::Task && e.id.as_deref() == Some(id))
        .collect();
    events.reverse();
    History {
        since,
        events,
        issues,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_are_unique_and_sort_by_time() {
        let dir = std::env::temp_dir().join(format!(
            "allomos-test-events-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let mut seen = std::collections::HashSet::new();
        for _ in 0..200 {
            let name = new_file(&dir);
            assert!(name.starts_with("events/20"), "{name}");
            assert!(name.ends_with(".json"));
            assert!(seen.insert(name));
        }
    }

    #[test]
    fn suffixes_are_six_letters_or_digits() {
        let s = suffix();
        assert_eq!(s.len(), 6);
        assert!(
            s.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        );
    }
}
