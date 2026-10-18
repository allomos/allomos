//! Is a newer Allomos on GitHub (Task-A07-16)? The app asks GitHub at most once a day, and
//! whenever the person chooses **Check for Updates…**; it remembers the answer, so a notice
//! still shows after a restart without asking again.
//!
//! GitHub is asked with `curl`, which macOS and Windows 10 and later already have, so the app
//! needs no web library. Any failure (no network, GitHub busy, no `curl`) is quiet: the app
//! simply does not know of a newer version.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

/// GitHub's answer for the newest published release (drafts and pre-releases are left out).
const LATEST_RELEASE_API: &str = "https://api.github.com/repos/allomos/allomos/releases/latest";
/// How long one answer is trusted before GitHub is asked again (approver, 2026-10-15).
pub const CHECK_EVERY_SECS: u64 = 24 * 60 * 60;
/// How long to wait for GitHub before giving up.
const TIMEOUT_SECS: &str = "10";

/// What the window shows. `newer` is the version to offer, when there is one to offer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdateStatus {
    /// This Allomos's version.
    pub current: String,
    /// The newest version known, from this check or the last one that worked.
    pub latest: Option<String>,
    /// The version to offer: newer than this one, and not skipped (unless asked from the menu).
    pub newer: Option<String>,
    /// A newer version than this one, even a skipped one: the menu offers it (approver,
    /// 2026-10-15: like the Claude app's "Restart to Update" item).
    pub available: Option<String>,
    /// GitHub was asked now and did not answer.
    pub failed: bool,
}

/// `0.1.5` or `v0.1.5` as numbers; `None` for anything else (`0.2.0-beta`, `nightly`).
pub fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    let text = text.trim();
    let text = text.strip_prefix('v').unwrap_or(text);
    let mut parts = text.split('.').map(|p| p.parse::<u64>().ok());
    let version = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(version)
}

/// The newest release's version from GitHub's answer, without the leading `v`.
pub fn latest_from_json(json: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let tag = value.get("tag_name")?.as_str()?;
    parse_version(tag)?;
    Some(tag.trim().trim_start_matches('v').to_string())
}

/// Whether GitHub should be asked now: always from the menu, otherwise when the last answer is
/// missing or a day old. A clock set back makes the last answer count as old.
pub fn due(checked_at: Option<u64>, now: u64, asked: bool) -> bool {
    match checked_at {
        _ if asked => true,
        None => true,
        Some(at) => now < at || now - at >= CHECK_EVERY_SECS,
    }
}

/// What to offer, given what is known. A skipped version is not offered again unless the person
/// asked from the menu; a newer one than the skipped one is.
pub fn decide(
    current: &str,
    latest: Option<&str>,
    skipped: Option<&str>,
    asked: bool,
    failed: bool,
) -> UpdateStatus {
    let available = latest.filter(|l| {
        matches!(
            (parse_version(l), parse_version(current)),
            (Some(l), Some(c)) if l > c
        )
    });
    let newer = available.filter(|l| asked || skipped != Some(l));
    UpdateStatus {
        current: current.to_string(),
        latest: latest.map(String::from),
        newer: newer.map(String::from),
        available: available.map(String::from),
        failed,
    }
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Ask GitHub for the newest release's version.
pub fn fetch_latest(current: &str) -> Option<String> {
    let output = crate::agents::command("curl")
        .args([
            "--silent",
            "--fail",
            "--location",
            "--max-time",
            TIMEOUT_SECS,
        ])
        .args(["--header", "Accept: application/vnd.github+json"])
        .args(["--user-agent", &format!("Allomos/{current}")])
        .arg(LATEST_RELEASE_API)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    latest_from_json(&String::from_utf8_lossy(&output.stdout))
}

/// The whole check: ask GitHub if due, remember the answer, and say what to offer.
pub fn check(asked: bool) -> UpdateStatus {
    let current = env!("CARGO_PKG_VERSION");
    let mut memory = allomos_store::update_memory();
    let mut failed = false;
    let now = now();
    if due(memory.checked_at, now, asked) {
        match fetch_latest(current) {
            Some(latest) => {
                let _ = allomos_store::save_update_check(now, &latest);
                memory.latest = Some(latest);
            }
            None => failed = true,
        }
    }
    decide(
        current,
        memory.latest.as_deref(),
        memory.skipped.as_deref(),
        asked,
        failed,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_are_read_with_or_without_a_v() {
        assert_eq!(parse_version("0.1.5"), Some((0, 1, 5)));
        assert_eq!(parse_version("v1.20.3"), Some((1, 20, 3)));
        assert_eq!(parse_version("0.2.0-beta"), None);
        assert_eq!(parse_version("0.2"), None);
        assert_eq!(parse_version("0.2.0.1"), None);
        assert_eq!(parse_version("nightly"), None);
    }

    #[test]
    fn the_latest_version_comes_from_the_tag_name() {
        let json = r#"{"tag_name":"v0.1.6","html_url":"https://github.com/x","draft":false}"#;
        assert_eq!(latest_from_json(json).as_deref(), Some("0.1.6"));
        assert_eq!(latest_from_json(r#"{"tag_name":"latest-build"}"#), None);
        assert_eq!(latest_from_json(r#"{"message":"Not Found"}"#), None);
        assert_eq!(latest_from_json("<html>"), None);
    }

    #[test]
    fn github_is_asked_once_a_day_or_from_the_menu() {
        let day = CHECK_EVERY_SECS;
        assert!(due(None, 5, false));
        assert!(!due(Some(1_000), 1_000 + day - 1, false));
        assert!(due(Some(1_000), 1_000 + day, false));
        assert!(due(Some(1_000), 1_001, true));
        // The clock was set back.
        assert!(due(Some(1_000), 999, false));
    }

    #[test]
    fn only_a_newer_version_is_offered() {
        assert_eq!(
            decide("0.1.5", Some("0.1.6"), None, false, false)
                .newer
                .as_deref(),
            Some("0.1.6")
        );
        // Compared as numbers, not as text.
        assert_eq!(
            decide("0.1.9", Some("0.1.10"), None, false, false)
                .newer
                .as_deref(),
            Some("0.1.10")
        );
        assert_eq!(
            decide("0.1.5", Some("0.1.5"), None, false, false).newer,
            None
        );
        assert_eq!(
            decide("0.2.0", Some("0.1.9"), None, false, false).newer,
            None
        );
        assert_eq!(decide("0.1.5", None, None, false, true).newer, None);
    }

    #[test]
    fn a_skipped_version_is_offered_only_from_the_menu() {
        let quiet = decide("0.1.5", Some("0.1.6"), Some("0.1.6"), false, false);
        assert_eq!(quiet.newer, None);
        assert_eq!(quiet.latest.as_deref(), Some("0.1.6"));
        // The menu still offers it.
        assert_eq!(quiet.available.as_deref(), Some("0.1.6"));
        let asked = decide("0.1.5", Some("0.1.6"), Some("0.1.6"), true, false);
        assert_eq!(asked.newer.as_deref(), Some("0.1.6"));
        // A version newer than the skipped one is offered again.
        let next = decide("0.1.5", Some("0.1.7"), Some("0.1.6"), false, false);
        assert_eq!(next.newer.as_deref(), Some("0.1.7"));
    }
}
