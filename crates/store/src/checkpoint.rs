//! Checkpoints (Task-A01-gusto404, decision 0021 § 3.2): a signed file that fixes a fingerprint
//! of the whole taskbag at one time.
//!
//! `checkpoints/<date>-<time>-<ms>-<random>.json` holds `{at, git_commit (when the agent gave
//! it), sha256, files, events, heads, receipts, sig}`. `sha256` is the taskbag's fingerprint:
//! SHA-256 over one line per file, `<file's SHA-256>  <path>`, sorted by path, of every file in
//! the taskbag except the checkpoints themselves, hidden files (the lock, a writer's temporary
//! files, `.DS_Store`) and `.git`. `heads` names each key's last event and its fingerprint, so a
//! later check finds an event removed from the end of a chain. `receipts` is where an outside
//! witness's time-stamp receipts go (decision 0021 § 3.3); none are made yet.
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

/// Make a checkpoint under the taskbag's lock: `alm checkpoint`.
pub fn checkpoint(
    root: &Path,
    wait: std::time::Duration,
    git_commit: Option<&str>,
) -> Result<Made, Problem> {
    crate::versions::write_allowed(root)?;
    let _lock = crate::Lock::acquire(root, wait)?;
    make(root, git_commit)
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
