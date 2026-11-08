//! Checkpoints (Task-A01-gusto404, decision 0021 § 3.2): a signed file that fixes a fingerprint
//! of the whole taskbag at one time.
//!
//! `checkpoints/<date>-<time>-<ms>-<random>.json` holds `{at, git_commit (when the agent gave
//! it), sha256, files, events, heads, receipts, sig}`. `sha256` is the taskbag's fingerprint:
//! SHA-256 over one line per file, `<file's SHA-256>  <path>`, sorted by path, of every file in
//! the taskbag except the checkpoints themselves, hidden files (the lock, a writer's temporary
//! files, `.DS_Store`) and `.git`. `heads` names each key's last event and its fingerprint, so a
//! later check finds an event removed from the end of a chain. `receipts` holds the outside
//! witness's time-stamp receipts (Task-A01-wood982, decision 0021 § 3.3).
//!
//! The outside witness: while it is on, a checkpoint also names, in `witness`, the services its
//! fingerprint goes to, and it is signed with that list and `receipts: []`. Its fingerprint is
//! then sent (never the content), outside the lock; each receipt that comes back is added to
//! `receipts`, which the signature leaves out (a receipt carries its own signature, by the
//! service). A service that cannot be reached leaves the checkpoint waiting: the next
//! `alm checkpoint` (or **Make a checkpoint now**) sends it first, and its receipt carries that
//! later time, which is honest.
//!
//! `alm` runs no version control tool (decision 0017), so it does not read the Git commit itself:
//! the agent passes it (`--git-commit`).

use std::fs;
use std::path::{Path, PathBuf};

use allomos_core::Problem;
use allomos_core::canonical::sha256_hex;
use serde::Serialize;
use serde_json::{Value, json};

pub const CHECKPOINTS_DIR: &str = "checkpoints";
const SEEN_FILE: &str = "seen-checkpoints.json";

/// The taskbag's fingerprint and how many files it covers.
pub fn taskbag_fingerprint(root: &Path) -> (String, usize) {
    let mut lines = Vec::new();
    walk(root, root, &mut lines);
    lines.sort();
    let n = lines.len();
    let text = lines
        .into_iter()
        .fold(String::new(), |mut out, (path, sha)| {
            out.push_str(&sha);
            out.push_str("  ");
            out.push_str(&path);
            out.push('\n');
            out
        });
    (sha256_hex(text.as_bytes()), n)
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, String)>) {
    for path in crate::sorted_entries(dir) {
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        if rel == CHECKPOINTS_DIR {
            continue;
        }
        if path.is_dir() {
            walk(root, &path, out);
        } else if let Ok(bytes) = fs::read(&path) {
            out.push((rel, sha256_hex(&bytes)));
        }
    }
}

/// Every checkpoint, oldest first, as written, with the files that could not be read.
pub fn list(root: &Path) -> (Vec<(String, Value)>, Vec<String>) {
    let mut out = Vec::new();
    let mut bad = Vec::new();
    for path in crate::sorted_entries(&root.join(CHECKPOINTS_DIR)) {
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let file = format!(
            "{CHECKPOINTS_DIR}/{}",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        match crate::read_text(&path)
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        {
            Some(v) => out.push((file, v)),
            None => bad.push(file),
        }
    }
    (out, bad)
}

/// What [`make`] wrote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Made {
    /// `checkpoints/<name>.json`, relative to the taskbag.
    pub file: String,
    pub sha256: String,
    pub files: usize,
    pub events: usize,
    /// One per key: its last event.
    pub heads: usize,
}

/// Write a checkpoint of the taskbag at `root` now, signed with this computer's key. Refused
/// without one (`no_signing_key`). Callers hold the lock.
pub(crate) fn make(root: &Path, git_commit: Option<&str>) -> Result<Made, Problem> {
    let key = crate::signing::key().ok_or(Problem::NoSigningKey)?;
    let (sha256, files) = taskbag_fingerprint(root);
    let (events, _) = crate::trail::raw_events(root);
    let tips = crate::trail::tips(root);
    let heads: serde_json::Map<String, Value> = tips
        .iter()
        .map(|(k, (file, fp))| (k.clone(), json!({ "file": file, "fingerprint": fp })))
        .collect();
    let at = crate::events::now_utc();
    let witness = witness_services(root);
    let mut value = json!({
        "at": at,
        "sha256": sha256,
        "files": files,
        "events": events.len(),
        "heads": heads,
        "receipts": [],
    });
    if let Some(c) = git_commit.map(str::trim).filter(|c| !c.is_empty()) {
        value["git_commit"] = json!(c);
    }
    if !witness.is_empty() {
        value["witness"] = json!(witness);
    }
    let signature = key.sign(&value);
    value["sig"] = json!({ "key": key.id, "value": signature });
    let dir = root.join(CHECKPOINTS_DIR);
    fs::create_dir_all(&dir).map_err(|e| crate::write_failed(&dir, e))?;
    let name = crate::events::new_file(root)
        .trim_start_matches(&format!("{}/", crate::events::EVENTS_DIR))
        .to_string();
    let file = format!("{CHECKPOINTS_DIR}/{name}");
    crate::write_atomic(&root.join(&file), &crate::json_text(&value))?;
    Ok(Made {
        file,
        sha256,
        files,
        events: events.len(),
        heads: tips.len(),
    })
}

/// Make a checkpoint under the taskbag's lock: `alm checkpoint`. With the outside witness on,
/// every checkpoint still waiting for a receipt is then sent, this one last.
pub fn checkpoint(
    root: &Path,
    wait: std::time::Duration,
    git_commit: Option<&str>,
) -> Result<(Made, Witnessed), Problem> {
    crate::versions::write_allowed(root)?;
    let made = {
        let _lock = crate::Lock::acquire(root, wait)?;
        make(root, git_commit)?
    };
    let witnessed = send_waiting(root, wait)?;
    Ok((made, witnessed))
}

/// The services the project's outside witness sends to; empty while it is off.
fn witness_services(root: &Path) -> Vec<String> {
    crate::read_text(&root.join("project.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v.get("audit")?.get("witness").cloned())
        .and_then(|w| serde_json::from_value::<allomos_core::witness::Witness>(w).ok())
        .map(|w| w.services())
        .unwrap_or_default()
}

/// A checkpoint's fingerprint not yet countersigned by one of the services it names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Waiting {
    /// `checkpoints/<name>.json`.
    pub file: String,
    pub service: String,
    /// Why the last try gave no receipt, when there was a try.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
}

/// One receipt added.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Receipt {
    pub file: String,
    pub service: String,
    /// The time the service gives (UTC).
    pub time: String,
}

/// What sending the waiting fingerprints did.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Witnessed {
    pub receipts: Vec<Receipt>,
    /// Still waiting: the service was not reached (try again later) or did not countersign.
    pub waiting: Vec<Waiting>,
}

/// Every checkpoint (oldest first) and service it names whose receipt is missing.
pub fn waiting(root: &Path) -> Vec<Waiting> {
    let (all, _) = list(root);
    all.iter()
        .flat_map(|(file, c)| {
            missing_receipts(c).into_iter().map(|service| Waiting {
                file: file.clone(),
                service,
                why: None,
            })
        })
        .collect()
}

/// The services a checkpoint names in `witness` that none of its receipts comes from.
pub fn missing_receipts(checkpoint: &Value) -> Vec<String> {
    let have: Vec<&str> = checkpoint
        .get("receipts")
        .and_then(Value::as_array)
        .map(|r| {
            r.iter()
                .filter_map(|r| r.get("service")?.as_str())
                .collect()
        })
        .unwrap_or_default();
    checkpoint
        .get("witness")
        .and_then(Value::as_array)
        .map(|w| {
            w.iter()
                .filter_map(Value::as_str)
                .filter(|s| !have.contains(s))
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

/// Send every waiting fingerprint to its service, oldest first, and keep each receipt in its
/// checkpoint. Only to the services the project's outside witness has on now: once it is turned
/// off, or a service is replaced, nothing more goes to the old one (those stay listed as
/// waiting). The network is used without the lock; each receipt is written under it. Requests
/// to one service are a little over a second apart (FreeTSA asks for at most one a second).
pub fn send_waiting(root: &Path, wait: std::time::Duration) -> Result<Witnessed, Problem> {
    let mut done = Witnessed::default();
    let on = witness_services(root);
    let mut unreachable: Vec<String> = Vec::new();
    let mut last_sent: std::collections::BTreeMap<String, std::time::Instant> = Default::default();
    for w in waiting(root)
        .into_iter()
        .filter(|w| on.contains(&w.service))
    {
        if unreachable.contains(&w.service) {
            done.waiting.push(Waiting {
                why: Some("not reached".into()),
                ..w
            });
            continue;
        }
        let Some(sha256) = crate::read_text(&root.join(&w.file))
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .and_then(|c| c.get("sha256")?.as_str().map(String::from))
        else {
            continue;
        };
        if let Some(at) = last_sent.get(&w.service) {
            let gap = std::time::Duration::from_millis(1100);
            if let Some(rest) = gap.checked_sub(at.elapsed()) {
                std::thread::sleep(rest);
            }
        }
        last_sent.insert(w.service.clone(), std::time::Instant::now());
        match crate::tsa::countersign(&w.service, &sha256, &crate::events::now_utc()) {
            Ok(receipt) => {
                let time = receipt["time"].as_str().unwrap_or_default().to_string();
                let _lock = crate::Lock::acquire(root, wait)?;
                let path = root.join(&w.file);
                let mut c: Value = crate::read_text(&path)
                    .ok()
                    .and_then(|t| serde_json::from_str(&t).ok())
                    .ok_or_else(|| Problem::ReadFailed {
                        detail: w.file.clone(),
                    })?;
                if !c["receipts"].is_array() {
                    c["receipts"] = json!([]);
                }
                if let Some(r) = c["receipts"].as_array_mut() {
                    r.push(receipt);
                }
                crate::write_atomic(&path, &crate::json_text(&c))?;
                done.receipts.push(Receipt {
                    file: w.file,
                    service: w.service,
                    time,
                });
            }
            Err(crate::tsa::Refused::Unreachable(why)) => {
                unreachable.push(w.service.clone());
                done.waiting.push(Waiting {
                    why: Some(why),
                    ..w
                });
            }
            Err(crate::tsa::Refused::Answer(why)) => done.waiting.push(Waiting {
                why: Some(why),
                ..w
            }),
        }
    }
    Ok(done)
}

/// Where this computer remembers the last checkpoint it saw for each project (outside the
/// taskbag); `None` without a settings folder.
pub(crate) fn seen_file() -> Option<PathBuf> {
    if cfg!(any(test, feature = "test-settings"))
        && std::env::var_os("ALLOMOS_SETTINGS_DIR").is_none()
    {
        return None;
    }
    crate::settings::settings_dir().map(|d| d.join(SEEN_FILE))
}

/// The last checkpoint this computer saw for one project, outside the taskbag (Settings → This
/// computer → Last seen history, Task-A01-tail719).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Seen {
    pub project: String,
    /// `checkpoints/<name>.json`, relative to the taskbag.
    pub checkpoint: String,
    /// When this computer first saw it, in UTC.
    pub seen_at: String,
}

/// Every project's last checkpoint seen on this computer, by project ID; empty without a
/// settings folder or before the first `alm audit`.
pub fn seen() -> Vec<Seen> {
    let Some(path) = seen_file() else {
        return Vec::new();
    };
    let map: std::collections::BTreeMap<String, (String, String)> = fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    map.into_iter()
        .map(|(project, (checkpoint, seen_at))| Seen {
            project,
            checkpoint,
            seen_at,
        })
        .collect()
}
