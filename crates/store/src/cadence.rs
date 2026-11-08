//! The outside witness's "how often" (Task-A01-bake791, decision 0021 § 6 row 4): checkpoints
//! made by themselves, daily or after each approval and human check, and their fingerprints sent.
//!
//! - **Daily:** the app while it is open (it asks every few minutes) and any `alm` command that
//!   changes the taskbag make one when the newest checkpoint is a day old or more. Both read the
//!   same time, the newest checkpoint's `at`, and decide again under the taskbag's lock, so two
//!   at once make one.
//! - **Approvals:** after `alm approve`, `alm checked` and the app's same.
//! - **Asked:** nothing by itself, as before (`alm checkpoint`, **Make a checkpoint now**).
//!
//! The checkpoint is always made on this computer first; sending comes after, outside the lock
//! (the store's [`crate::checkpoint::send_waiting`]). Offline, the fingerprint waits and the
//! command ends as usual; waiting fingerprints are tried again by the next checkpoint, or by a
//! change at most every [`RETRY_MINUTES`]. Requests to one service stay a little over a second
//! apart across processes on this computer too: the last time each was sent is kept in the
//! settings folder (`witness-sent.json`, outside the taskbag).

use std::path::{Path, PathBuf};
use std::time::Duration;

use allomos_core::Problem;
use allomos_core::witness::When;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::checkpoint::{CHECKPOINTS_DIR, Made, Witnessed};

/// A day, in seconds: how old the newest checkpoint is before the next daily one.
pub const DAY_SECONDS: i64 = 86_400;
/// How often, at most, a change tries waiting fingerprints again when no checkpoint is due.
pub const RETRY_MINUTES: i64 = 10;
/// The least gap between two requests to one service (FreeTSA asks for at most one a second).
pub const GAP: Duration = Duration::from_millis(1100);
const SENT_FILE: &str = "witness-sent.json";

/// What happened that may make a checkpoint by itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    /// An `alm` command changed the taskbag, or the app's clock asked.
    Change,
    /// An approval or a human check was recorded (it is a change too).
    Approval,
}

/// What [`auto`] did: the checkpoint made (if one was due) and what sending did.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Auto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub made: Option<Made>,
    /// The setting that made it: `daily` or `approvals`.
    pub when: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub witnessed: Option<Witnessed>,
}

/// The project's outside witness, when it is on.
fn witness(root: &Path) -> Option<allomos_core::witness::Witness> {
    crate::read_text(&root.join("project.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v.get("audit")?.get("witness").cloned())
        .and_then(|w| serde_json::from_value(w).ok())
}

/// The newest checkpoint's time (`at`), in seconds since 1970; `None` before the first.
pub fn last_checkpoint_at(root: &Path) -> Option<i64> {
    crate::sorted_entries(&root.join(CHECKPOINTS_DIR))
        .into_iter()
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json"))
        .filter_map(|p| {
            let v: Value = serde_json::from_str(&crate::read_text(&p).ok()?).ok()?;
            crate::quota::utc_seconds(v.get("at")?.as_str()?)
        })
        .max()
}

/// Now, in seconds since 1970 (`ALLOMOS_NOW` fixes it, as for events: the test clock).
pub fn now_seconds() -> i64 {
    crate::quota::utc_seconds(&crate::events::now_utc()).unwrap_or_default()
}

/// Whether a checkpoint is due now under `when`, for `trigger`.
fn due(root: &Path, when: When, trigger: Trigger, now: i64) -> bool {
    match when {
        When::Asked => false,
        When::Approvals => trigger == Trigger::Approval,
        When::Daily => last_checkpoint_at(root).is_none_or(|at| now - at >= DAY_SECONDS),
    }
}

/// Make a checkpoint if the outside witness's "how often" says one is due, then send what waits.
/// `None` when nothing was due and nothing was sent: the witness is off, set to `asked`, or the
/// checkpoint is not due and no waiting fingerprint is to be tried again yet. Callers call it
/// after their change, without the lock. Refused only as `alm checkpoint` is (no signing key, a
/// newer taskbag, the lock busy); callers show that and carry on: their change is made.
pub fn auto(root: &Path, wait: Duration, trigger: Trigger) -> Result<Option<Auto>, Problem> {
    let Some(w) = witness(root) else {
        return Ok(None);
    };
    let when = w.when();
    if when == When::Asked {
        return Ok(None);
    }
    let now = now_seconds();
    let mut made = None;
    if due(root, when, trigger, now) {
        crate::versions::write_allowed(root)?;
        let _lock = crate::Lock::acquire(root, wait)?;
        // Again under the lock: another process (the app, another `alm`) may have made it.
        if due(root, when, trigger, now) {
            made = Some(crate::checkpoint::make(root, None)?);
        }
    }
    let waiting = !crate::checkpoint::waiting(root).is_empty();
    let retry = waiting && (made.is_some() || retry_due(now));
    if !retry {
        return Ok(made.map(|m| Auto {
            made: Some(m),
            when: when.id().into(),
            witnessed: None,
        }));
    }
    mark_tried(now);
    let witnessed = crate::checkpoint::send_waiting(root, wait)?;
    Ok(Some(Auto {
        made,
        when: when.id().into(),
        witnessed: Some(witnessed),
    }))
}

// --- This computer's sending times, outside the taskbag. ---

#[derive(Debug, Default, Serialize, Deserialize)]
struct Sent {
    /// Each service's last request, in milliseconds since 1970.
    #[serde(default)]
    sent: std::collections::BTreeMap<String, u128>,
    /// The last time waiting fingerprints were tried by a change, in seconds since 1970.
    #[serde(default)]
    tried: i64,
}

fn sent_file() -> Option<PathBuf> {
    if cfg!(any(test, feature = "test-settings"))
        && std::env::var_os("ALLOMOS_SETTINGS_DIR").is_none()
    {
        return None;
    }
    crate::settings::settings_dir().map(|d| d.join(SENT_FILE))
}

fn read_sent() -> Sent {
    sent_file()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn write_sent(s: &Sent) {
    if let Some(p) = sent_file() {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        // Not `write_atomic`: this file is outside the taskbag, never listed to share.
        let _ = std::fs::write(&p, crate::json_text(s));
    }
}

fn retry_due(now: i64) -> bool {
    now - read_sent().tried >= RETRY_MINUTES * 60
}

fn mark_tried(now: i64) {
    let mut s = read_sent();
    s.tried = now;
    write_sent(&s);
}

fn millis_now() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

/// Wait until `service` was last sent to [`GAP`] ago or more (by any process on this computer),
/// then note that it is sent to now.
pub(crate) fn space_out(service: &str) {
    let last = read_sent().sent.get(service).copied().unwrap_or(0);
    let gap = GAP.as_millis();
    let now = millis_now();
    if now < last + gap {
        std::thread::sleep(Duration::from_millis((last + gap - now) as u64));
    }
    let mut s = read_sent();
    s.sent.insert(service.to_string(), millis_now());
    write_sent(&s);
}
