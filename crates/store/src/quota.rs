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

/// How old an agent's newest 5-hour report may be before a command that moves a task on asks for
/// a new one (Task-A03-flour054, approver 2026-10-22), in minutes: the app's chip turns stale at
/// the same age (`apps/web/src/quotaFlow.ts`).
pub const ASK_AFTER_MINUTES: i64 = 30;

/// The ask a command that moves a task on prints when the agent's quota report is missing or old.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QuotaAsk {
    /// The agent's id, from the session name (`claude/…` is `claude-code`); `None` without one.
    pub agent: Option<String>,
    /// How old the agent's newest 5-hour report is, in minutes; `None` when there is none.
    pub minutes: Option<i64>,
    /// The command to run, with the figure left for the agent to fill in.
    pub command: String,
}

impl QuotaAsk {
    /// The ask in plain words, with the command on a line of its own.
    pub fn text(&self) -> String {
        let who = match &self.agent {
            Some(a) => format!("Your agent ({a})"),
            None => "Your agent".to_string(),
        };
        let when = match self.minutes {
            Some(m) => format!("last reported its quota {m} min ago"),
            None => "has not reported its quota on this computer".to_string(),
        };
        format!(
            "Quota: {who} {when}. Read your usage now (in Claude Code: get_usage) and report it, so the \
             Allomos app shows a current figure; nothing to save or share:\n  {}\n",
            self.command
        )
    }
}

/// Seconds since 1970 for a UTC time `YYYY-MM-DDTHH:MM:SSZ`, or `None` for another shape.
fn utc_seconds(text: &str) -> Option<i64> {
    let b = text.as_bytes();
    if b.len() != 20 || b[19] != b'Z' || b[10] != b'T' {
        return None;
    }
    let n = |r: std::ops::Range<usize>| text.get(r)?.parse::<i64>().ok();
    let (y, m, d) = (n(0..4)?, n(5..7)?, n(8..10)?);
    let (hh, mm, ss) = (n(11..13)?, n(14..16)?, n(17..19)?);
    // Days from the civil date (Howard Hinnant's algorithm), the inverse of `civil_from_days`.
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days * 86_400 + hh * 3600 + mm * 60 + ss)
}

/// What a command that moves a task on asks of the agent `holder` (its session name, as `--as`
/// gives it), given every report and the time now (UTC): `None` when the agent's newest 5-hour
/// report is younger than [`ASK_AFTER_MINUTES`], otherwise the ask. `alm` only tells; the
/// command it asks after has already succeeded (AGENTS.md: "alm tells; the agent checks").
pub fn quota_ask(holder: Option<&str>, reports: &[QuotaReport], now: &str) -> Option<QuotaAsk> {
    let agent = holder.map(agent_id).filter(|a| !a.is_empty());
    let newest = reports
        .iter()
        .filter(|r| r.window == "5h")
        .filter(|r| agent.as_deref().is_none_or(|a| agent_id(&r.agent) == a))
        .filter_map(|r| utc_seconds(&r.at))
        .max();
    let minutes = match (newest, utc_seconds(now)) {
        (Some(at), Some(now)) => Some(((now - at) / 60).max(0)),
        _ => None,
    };
    if minutes.is_some_and(|m| m < ASK_AFTER_MINUTES) {
        return None;
    }
    let id = agent
        .clone()
        .unwrap_or_else(|| "<your agent's id>".to_string());
    Some(QuotaAsk {
        command: format!(
            "alm quota --agent {id} --left <percent> [--resets <local date and time>]"
        ),
        agent,
        minutes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(agent: &str, window: &str, at: &str) -> QuotaReport {
        QuotaReport {
            agent: agent.into(),
            account: None,
            window: window.into(),
            left: 50,
            resets: None,
            at: at.into(),
        }
    }

    #[test]
    fn a_missing_or_old_report_is_asked_for_and_a_recent_one_is_not() {
        let now = "2026-10-21T16:00:00Z";
        // Never reported: asked, with the agent's id from the session name.
        let ask = quota_ask(Some("claude/a03-flour054"), &[], now).unwrap();
        assert_eq!(ask.agent.as_deref(), Some("claude-code"));
        assert_eq!(ask.minutes, None);
        assert!(
            ask.command
                .starts_with("alm quota --agent claude-code --left <percent>")
        );
        assert!(
            ask.text()
                .contains("has not reported its quota on this computer")
        );
        // An old report, saved under a session name before Task-A03-39: still this agent's.
        let old = [report("claude/i03-loop", "5h", "2026-10-21T05:23:45Z")];
        let ask = quota_ask(Some("claude/x"), &old, now).unwrap();
        assert_eq!(ask.minutes, Some(636));
        assert!(ask.text().contains("last reported its quota 636 min ago"));
        // Recent enough: nothing asked. 29 minutes is recent; 30 is not.
        let recent = [report("claude-code", "5h", "2026-10-21T15:31:00Z")];
        assert_eq!(quota_ask(Some("claude/x"), &recent, now), None);
        let edge = [report("claude-code", "5h", "2026-10-21T15:30:00Z")];
        assert_eq!(
            quota_ask(Some("claude/x"), &edge, now).unwrap().minutes,
            Some(30)
        );
        // Another agent's or another window's report does not count.
        let others = [
            report("codex", "5h", "2026-10-21T15:59:00Z"),
            report("claude-code", "week", "2026-10-21T15:59:00Z"),
        ];
        assert_eq!(
            quota_ask(Some("claude/x"), &others, now).unwrap().minutes,
            None
        );
        assert_eq!(quota_ask(Some("codex/run"), &others, now), None);
        // Without a session name: any agent's recent report will do; otherwise a placeholder.
        assert_eq!(quota_ask(None, &others, now), None);
        let ask = quota_ask(None, &[], now).unwrap();
        assert_eq!(ask.agent, None);
        assert!(ask.command.contains("--agent <your agent's id>"));
    }

    #[test]
    fn utc_times_are_read_as_seconds() {
        assert_eq!(utc_seconds("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(utc_seconds("2026-10-21T16:00:00Z"), Some(1_792_598_400));
        assert_eq!(utc_seconds("2026-10-21T16:00"), None);
        assert_eq!(utc_seconds("tomorrow"), None);
    }

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
        assert_eq!(
            new_report("claude/a09-28", None, "5h", 86, None)
                .unwrap()
                .agent,
            "claude-code"
        );
        assert!(matches!(
            new_report("/x", None, "5h", 1, None),
            Err(Problem::BadUsage { .. })
        ));
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
