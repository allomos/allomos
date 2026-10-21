//! How much of each AI agent's account quota is left, as the agents report it (`alm quota`,
//! Task-A03-02). The agent reads its own usage (for example Claude Code's `get_usage`) at the
//! handoff gate's quota check and passes the figure on; Allomos never reads a provider's quota.
//!
//! The reports are kept on this computer, in `quota.json` beside the app's settings
//! ([`crate::settings_dir`]), not in the taskbag: a quota changes every step, so keeping it in
//! the taskbag would mean a commit per report and conflicts between sessions. One report per
//! agent, account and window; a new one replaces it. `alm` stamps the time.

use std::fs;
use std::io;
use std::path::PathBuf;

use allomos_core::Problem;
use serde::{Deserialize, Serialize};

const QUOTA_FILE: &str = "quota.json";

/// The limits a figure can be for: the 5-hour window the handoff gate uses, or the week.
pub const WINDOWS: &[&str] = &["5h", "week"];

/// One agent account's report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuotaReport {
    /// The agent: `claude-code`, `codex`, or another agent's id.
    pub agent: String,
    /// A name for the account, only when the agent can tell one (never an email address).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    /// `5h` or `week`.
    pub window: String,
    /// The share left, 0–100.
    pub left: u8,
    /// When this window resets, local time as the agent gave it (`2026-10-20T17:00`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resets: Option<String>,
    /// When it was reported, UTC (`2026-10-20T10:31:05Z`).
    pub at: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct QuotaFile {
    #[serde(default)]
    reports: Vec<QuotaReport>,
}

fn path() -> Option<PathBuf> {
    crate::settings_dir().map(|d| d.join(QUOTA_FILE))
}

/// Every report on this computer, newest first.
pub fn quota_reports() -> Vec<QuotaReport> {
    let mut reports = path()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str::<QuotaFile>(&t).ok())
        .unwrap_or_default()
        .reports;
    reports.sort_by(|a, b| b.at.cmp(&a.at));
    reports
}

/// `YYYY-MM-DDTHH:MM`, optionally with `:SS`: a local date and time.
fn local_time(text: &str) -> bool {
    let b = text.as_bytes();
    let digits = |r: std::ops::Range<usize>| b[r].iter().all(u8::is_ascii_digit);
    let shape = (b.len() == 16 || b.len() == 19)
        && digits(0..4)
        && b[4] == b'-'
        && digits(5..7)
        && b[7] == b'-'
        && digits(8..10)
        && b[10] == b'T'
        && digits(11..13)
        && b[13] == b':'
        && digits(14..16)
        && (b.len() == 16 || (b[16] == b':' && digits(17..19)));
    if !shape {
        return false;
    }
    let n = |r: std::ops::Range<usize>| text[r].parse::<u32>().unwrap_or(99);
    (1..=12).contains(&n(5..7))
        && (1..=31).contains(&n(8..10))
        && n(11..13) < 24
        && n(14..16) < 60
        && (b.len() == 16 || n(17..19) < 60)
}

fn bad(detail: String) -> Problem {
    Problem::BadUsage { detail }
}

/// The agent's id from what a session reported. Sessions sometimes give their session name
/// (`claude/a09-28-kanto-picker`) or a short form (`Claude`, `codex-cli`) instead of the id, which
/// gave the app a chip per session (Task-A03-39): the part before any `/` is taken, and a word
/// that starts with `claude` is Claude Code (`claude-code`), one that starts with `codex` is Codex
/// (`codex`). Any other id is kept as given.
pub fn agent_id(reported: &str) -> String {
    let word = reported.trim().split('/').next().unwrap_or("").trim();
    let lower = word.to_ascii_lowercase();
    if lower.starts_with("claude") {
        "claude-code".into()
    } else if lower.starts_with("codex") {
        "codex".into()
    } else {
        word.to_string()
    }
}

/// The report an agent gives: refused when the input is invalid (an empty agent, an unknown
/// window, a share over 100, a bad date). Nothing else is judged.
pub fn new_report(
    agent: &str,
    account: Option<&str>,
    window: &str,
    left: u8,
    resets: Option<&str>,
) -> Result<QuotaReport, Problem> {
    let agent = agent_id(agent);
    if agent.is_empty() || agent.chars().any(char::is_whitespace) {
        return Err(bad(format!(
            "--agent needs the agent's id, one word (for example claude-code or codex), not \"{agent}\""
        )));
    }
    if !WINDOWS.contains(&window) {
        return Err(bad(format!("--window is 5h or week, not \"{window}\"")));
    }
    if left > 100 {
        return Err(bad(format!(
            "--left is the share left, 0 to 100, not {left}"
        )));
    }
    let resets = resets.map(str::trim).filter(|r| !r.is_empty());
    if let Some(r) = resets.filter(|r| !local_time(r)) {
        return Err(bad(format!(
            "--resets is a local date and time like 2026-10-20T17:00, not \"{r}\""
        )));
    }
    Ok(QuotaReport {
        agent,
        account: account
            .map(str::trim)
            .filter(|a| !a.is_empty())
            .map(String::from),
        window: window.to_string(),
        left,
        resets: resets.map(String::from),
        at: crate::events::now_utc(),
    })
}

/// Keep `report`, replacing the earlier one for the same agent, account and window. Returns
/// every report, newest first.
pub fn save_quota(report: QuotaReport) -> io::Result<Vec<QuotaReport>> {
    let path = path().ok_or_else(|| io::Error::other("no settings folder"))?;
    let mut reports = quota_reports();
    reports.retain(|r| {
        (&r.agent, &r.account, &r.window) != (&report.agent, &report.account, &report.window)
    });
    reports.insert(0, report);
    let dir = path.parent().expect("a file in the settings folder");
    fs::create_dir_all(dir)?;
    let text = serde_json::to_string_pretty(&QuotaFile {
        reports: reports.clone(),
    })
    .map_err(io::Error::other)?;
    let tmp = dir.join(format!("{QUOTA_FILE}.tmp-{}", std::process::id()));
    fs::write(&tmp, text + "\n")?;
    crate::replace_file(&tmp, &path)?;
    Ok(reports)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_session_name_or_short_form_is_folded_into_the_agent_id() {
        for (given, id) in [
            ("claude-code", "claude-code"),
            ("claude/a09-28-kanto-picker", "claude-code"),
            (" Claude ", "claude-code"),
            ("codex", "codex"),
            ("codex-cli/loop-2", "codex"),
            ("gemini", "gemini"),
            ("gemini/run-3", "gemini"),
        ] {
            assert_eq!(agent_id(given), id, "{given}");
        }
        assert_eq!(new_report("claude/a09-28", None, "5h", 86, None).unwrap().agent, "claude-code");
        assert!(matches!(new_report("/x", None, "5h", 1, None), Err(Problem::BadUsage { .. })));
    }

    #[test]
    fn invalid_input_is_refused_and_nothing_else_judged() {
        let ok = new_report(
            "claude-code",
            Some(" work "),
            "5h",
            52,
            Some("2026-10-20T17:00"),
        )
        .unwrap();
        assert_eq!(ok.account.as_deref(), Some("work"));
        assert_eq!(ok.resets.as_deref(), Some("2026-10-20T17:00"));
        assert!(ok.at.ends_with('Z'));
        assert!(new_report("codex", None, "week", 0, Some("2026-10-20T17:00:30")).is_ok());
        assert!(
            new_report("codex", Some(""), "5h", 100, Some(" "))
                .unwrap()
                .account
                .is_none()
        );
        for (agent, window, left, resets) in [
            ("", "5h", 1, None),
            ("some agent", "5h", 1, None),
            ("codex", "day", 1, None),
            ("codex", "5h", 101, None),
            ("codex", "5h", 1, Some("tomorrow")),
            ("codex", "5h", 1, Some("2026-13-20T17:00")),
            ("codex", "5h", 1, Some("2026-10-20 17:00")),
            ("codex", "5h", 1, Some("2026-10-20T24:00")),
        ] {
            assert!(
                matches!(
                    new_report(agent, None, window, left, resets),
                    Err(Problem::BadUsage { .. })
                ),
                "{agent} {window} {left} {resets:?}"
            );
        }
    }
}
