//! Checking the history (Task-A01-gusto404, decision 0021 § 3.4): `alm audit`.
//!
//! It recomputes the taskbag's fingerprint, checks every signature and every computer's chain,
//! matches each checkpoint to what it covered, and lists what it finds: unsigned events, changed
//! events, broken chains, keys not in `keys/`, checkpoints whose events are gone. Following
//! "`alm` tells; the agent checks", it never says the history is "valid" or "tampered": it lists
//! facts, and the agent or person decides what they mean.
//!
//! Events are read as they are written, not through Allomos's own types, so nothing a newer
//! Allomos added is lost before the signature is checked. A packed month's events are read
//! under the name of the single file each came from, so packing changes nothing here.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use allomos_core::canonical::fingerprint;
use serde::Serialize;
use serde_json::Value;

use crate::events::{EVENTS_DIR, PACK_EXT};
use crate::signing::{self, PublicKey};

/// Every event as written, oldest first, each with its file (`events/<name>.json`), and the
/// files or lines that could not be read.
pub fn raw_events(root: &Path) -> (Vec<(String, Value)>, Vec<String>) {
    let mut out = Vec::new();
    let mut unreadable = Vec::new();
    let mut seen = BTreeSet::new();
    for path in crate::sorted_entries(&root.join(EVENTS_DIR)) {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let file = format!("{EVENTS_DIR}/{name}");
        let ext = path.extension().and_then(|e| e.to_str());
        if ext == Some(PACK_EXT) {
            let Ok(text) = crate::read_text(&path) else {
                unreadable.push(file);
                continue;
            };
            for (n, line) in text
                .lines()
                .enumerate()
                .filter(|(_, l)| !l.trim().is_empty())
            {
                let parsed = serde_json::from_str::<Value>(line).ok().and_then(|v| {
                    let f = v.get("file")?.as_str()?.to_string();
                    Some((format!("{EVENTS_DIR}/{f}"), v.get("event")?.clone()))
                });
                match parsed {
                    Some((f, e)) if seen.insert(f.clone()) => out.push((f, e)),
                    Some(_) => {}
                    None => unreadable.push(format!("{file}:{}", n + 1)),
                }
            }
        } else if ext == Some("json") {
            match crate::read_text(&path)
                .ok()
                .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            {
                Some(v) if seen.insert(file.clone()) => out.push((file, v)),
                Some(_) => {}
                None => unreadable.push(file),
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    (out, unreadable)
}

fn sig(event: &Value) -> Option<(&str, Option<&str>, &str)> {
    let s = event.get("sig")?;
    Some((
        s.get("key")?.as_str()?,
        s.get("prev").and_then(Value::as_str),
        s.get("value")?.as_str()?,
    ))
}

/// Each key's last event in the history: key → (file, fingerprint). When a key's chain forks,
/// the newest tip.
pub(crate) fn tips(root: &Path) -> BTreeMap<String, (String, String)> {
    let (events, _) = raw_events(root);
    let mut followed: BTreeSet<String> = BTreeSet::new();
    let mut by_key: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for (file, e) in &events {
        if let Some((key, prev, _)) = sig(e) {
            if let Some(p) = prev {
                followed.insert(p.to_string());
            }
            by_key
                .entry(key.to_string())
                .or_default()
                .push((file.clone(), fingerprint(e)));
        }
    }
    by_key
        .into_iter()
        .filter_map(|(key, list)| {
            let tip = list
                .into_iter()
                .filter(|(_, fp)| !followed.contains(fp))
                .max_by(|a, b| a.0.cmp(&b.0))?;
            Some((key, tip))
        })
        .collect()
}

/// One key's events, as `alm audit` lists them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Chain {
    pub key: String,
    /// Whose key, from `keys/` (a pseudonym), when it says.
    pub owner: Option<String>,
    pub events: usize,
    pub first: String,
    pub last: String,
}

/// A signed event whose `prev` names no event of its key: the one before it is missing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Break {
    pub key: String,
    /// The event after the gap.
    pub file: String,
    /// The fingerprint of the missing event.
    pub missing: String,
    /// The newest of the key's events before `file` that is still there, if any.
    pub after: Option<String>,
    /// `after` was changed after signing: then the missing event is most likely that one as it
    /// was, and the gap comes from the change, not from a removal.
    pub after_changed: bool,
}

/// Two or more events of one key that follow the same event (a computer wrote on two copies of
/// the taskbag, or an event was put back in a new place).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Fork {
    pub key: String,
    pub prev: Option<String>,
    pub files: Vec<String>,
}

/// A checkpoint, as `alm audit` finds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CheckpointFacts {
    pub file: String,
    pub at: String,
    /// Its signature checks against a key in `keys/`.
    pub signed: bool,
    /// The fingerprint it fixed.
    pub sha256: String,
    /// How many events the history had then.
    pub events: u64,
    /// The last events it fixed (one per key) that are no longer in the history, or changed.
    pub heads_missing: Vec<String>,
    /// Receipts from an outside witness it holds (none until the witness is built).
    pub receipts: usize,
}

/// What `alm audit` found. Facts only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Facts {
    /// The taskbag's fingerprint now (the same rule as a checkpoint's).
    pub sha256: String,
    pub files: usize,
    pub events: usize,
    pub signed: usize,
    /// Unsigned events before the first signed one: recorded before signing.
    pub before_signing: usize,
    /// Unsigned events after the first signed one.
    pub unsigned: Vec<String>,
    /// Events whose signature does not match what they say now: changed after signing.
    pub changed: Vec<String>,
    /// Events signed by a key that `keys/` does not have.
    pub unknown_key: Vec<String>,
    pub chains: Vec<Chain>,
    pub breaks: Vec<Break>,
    pub forks: Vec<Fork>,
    pub checkpoints: Vec<CheckpointFacts>,
    /// The taskbag has changed since its last checkpoint.
    pub changed_since_checkpoint: Option<bool>,
    /// Signed events newer than the last checkpoint.
    pub events_since_checkpoint: usize,
    /// The last checkpoint this computer saw before, when it is no longer in the history: the
    /// history was rewritten since then (or the checkpoint removed). `(file, seen at)`.
    pub rewritten: Option<(String, String)>,
    /// Events changed by a recorded redaction (Task-A01-stem986): not listed under `changed`,
    /// and a later event pointing to their fingerprint before is not a gap.
    pub redacted: Vec<RedactedFact>,
    /// Files and lines that could not be read (in `events/`, `keys/`, `checkpoints/`).
    pub unreadable: Vec<String>,
}

/// One event a redaction changed, as `alm audit` lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RedactedFact {
    pub file: String,
    /// The redaction's ID and the file of its event.
    pub redaction: String,
    pub by_event: String,
    pub fields: Vec<String>,
}

/// Check the history at `root` and list what was found. Remembers, on this computer, the last
/// checkpoint seen, for [`Facts::rewritten`] next time.
pub fn audit(root: &Path) -> Facts {
    let (events, mut unreadable) = raw_events(root);
    let (keys, bad_keys) = signing::public_keys(root);
    unreadable.extend(bad_keys);
    let (sha256, files) = crate::checkpoint::taskbag_fingerprint(root);

    let first_signed = events.iter().position(|(_, e)| sig(e).is_some());
    let mut facts = Facts {
        sha256,
        files,
        events: events.len(),
        signed: 0,
        before_signing: first_signed.unwrap_or(events.len()),
        unsigned: Vec::new(),
        changed: Vec::new(),
        unknown_key: Vec::new(),
        chains: Vec::new(),
        breaks: Vec::new(),
        forks: Vec::new(),
        checkpoints: Vec::new(),
        changed_since_checkpoint: None,
        events_since_checkpoint: 0,
        rewritten: None,
        redacted: Vec::new(),
        unreadable,
    };

    // Recorded redactions: file → (fingerprint before, fact).
    let mut before: BTreeMap<String, String> = BTreeMap::new();
    for (by_event, r) in allomos_core::retention::all(&events) {
        for f in r.files {
            before.insert(f.file.clone(), f.fingerprint.clone());
            facts.redacted.push(RedactedFact {
                file: f.file,
                redaction: r.id.clone(),
                by_event: by_event.clone(),
                fields: f.fields,
            });
        }
    }
    let redacted_with = |file: &str, fp: &str| before.get(file).is_some_and(|b| b == fp);

    // Per key: (file, fingerprint, prev), in file order.
    let mut by_key: BTreeMap<String, Vec<(String, String, Option<String>)>> = BTreeMap::new();
    for (i, (file, e)) in events.iter().enumerate() {
        let Some((key, prev, value)) = sig(e) else {
            if first_signed.is_some_and(|f| i > f) {
                facts.unsigned.push(file.clone());
            }
            continue;
        };
        facts.signed += 1;
        match keys.get(key) {
            Some(k) if signing::verify(&k.public_key, e, value) => {}
            Some(_) if before.contains_key(file) => {}
            Some(_) => facts.changed.push(file.clone()),
            None => facts.unknown_key.push(file.clone()),
        }
        by_key.entry(key.to_string()).or_default().push((
            file.clone(),
            fingerprint(e),
            prev.map(String::from),
        ));
    }

    for (key, list) in &by_key {
        let mut present: BTreeSet<&str> = list.iter().map(|(_, fp, _)| fp.as_str()).collect();
        present.extend(
            list.iter()
                .filter_map(|(f, _, _)| before.get(f).map(String::as_str)),
        );
        let mut after: BTreeMap<Option<&str>, Vec<String>> = BTreeMap::new();
        for (i, (file, _, prev)) in list.iter().enumerate() {
            after.entry(prev.as_deref()).or_default().push(file.clone());
            if let Some(p) = prev.as_deref().filter(|p| !present.contains(p)) {
                facts.breaks.push(Break {
                    key: key.clone(),
                    file: file.clone(),
                    missing: p.to_string(),
                    after: i.checked_sub(1).map(|j| list[j].0.clone()),
                    after_changed: false,
                });
            }
        }
        for (prev, files) in after.into_iter().filter(|(_, f)| f.len() > 1) {
            facts.forks.push(Fork {
                key: key.clone(),
                prev: prev.map(String::from),
                files,
            });
        }
        facts.chains.push(Chain {
            key: key.clone(),
            owner: keys.get(key).and_then(|k: &PublicKey| k.owner.clone()),
            events: list.len(),
            first: list.first().map(|l| l.0.clone()).unwrap_or_default(),
            last: list.last().map(|l| l.0.clone()).unwrap_or_default(),
        });
    }

    for b in &mut facts.breaks {
        b.after_changed = b.after.as_ref().is_some_and(|a| facts.changed.contains(a));
    }

    // Checkpoints.
    let fingerprints: BTreeMap<&str, String> = events
        .iter()
        .map(|(f, e)| (f.as_str(), fingerprint(e)))
        .collect();
    let (checkpoints, bad) = crate::checkpoint::list(root);
    facts.unreadable.extend(bad);
    for (file, c) in &checkpoints {
        let signed = sig(c).is_some_and(|(key, _, value)| {
            keys.get(key)
                .is_some_and(|k| signing::verify(&k.public_key, c, value))
        });
        let heads_missing = c
            .get("heads")
            .and_then(Value::as_object)
            .map(|heads| {
                heads
                    .values()
                    .filter_map(|h| {
                        let f = h.get("file")?.as_str()?;
                        let fp = h.get("fingerprint")?.as_str()?;
                        (fingerprints.get(f).map(String::as_str) != Some(fp)
                            && !redacted_with(f, fp))
                        .then(|| f.to_string())
                    })
                    .collect()
            })
            .unwrap_or_default();
        facts.checkpoints.push(CheckpointFacts {
            file: file.clone(),
            at: str_of(c, "at"),
            signed,
            sha256: str_of(c, "sha256"),
            events: c.get("events").and_then(Value::as_u64).unwrap_or(0),
            heads_missing,
            receipts: c
                .get("receipts")
                .and_then(Value::as_array)
                .map_or(0, Vec::len),
        });
    }
    if let Some(last) = facts.checkpoints.last() {
        facts.changed_since_checkpoint = Some(last.sha256 != facts.sha256);
        let last_at = last.file.clone();
        // Event files are named by time, as checkpoint files are: compare the names' dates.
        let stamp = |f: &str| {
            f.rsplit('/')
                .next()
                .unwrap_or(f)
                .get(..21)
                .map(String::from)
        };
        let since = stamp(&last_at);
        facts.events_since_checkpoint = events
            .iter()
            .filter(|(f, e)| sig(e).is_some() && stamp(f) > since)
            .count();
    }
    facts.rewritten = seen_check(root, &checkpoints);
    facts
}

fn str_of(v: &Value, field: &str) -> String {
    v.get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// Compare with the last checkpoint this computer saw for this project, then remember the newest.
fn seen_check(root: &Path, checkpoints: &[(String, Value)]) -> Option<(String, String)> {
    let project = crate::read_text(&root.join("project.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v.get("id")?.as_str().map(String::from))?;
    let path = crate::checkpoint::seen_file()?;
    let mut seen: BTreeMap<String, (String, String)> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    let names: BTreeSet<&str> = checkpoints.iter().map(|(f, _)| f.as_str()).collect();
    let rewritten = seen
        .get(&project)
        .filter(|(file, _)| !names.contains(file.as_str()))
        .cloned();
    // Once found, the fact stays: the old checkpoint is remembered until it is back.
    if let (None, Some((file, _))) = (&rewritten, checkpoints.last()) {
        seen.insert(project, (file.clone(), crate::events::now_utc()));
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
            let _ = std::fs::write(&path, crate::json_text(&seen));
        }
    }
    rewritten
}
