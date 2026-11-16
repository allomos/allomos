//! Whether an agent has quota left to start a session (A03-otter944). A monitor that starts a
//! session for an agent whose quota is used up gets a session that does nothing: its first reply
//! is "You've hit your 5-hour limit · resets 7:10pm". So before it starts one, the monitor reads
//! the quota (a terminal Claude Code answers `claude -p /usage` itself, without the model; for the
//! other agents, the last `alm quota` report), and after a session that never claimed its task, it
//! reads the session's tab for that line. Only reading and parsing here; the loop
//! ([`crate::runner`]) decides what to do.

use std::path::Path;

use allomos_store::QuotaReport;

/// A share used at or above this counts as used up: the handoff gate stops a session at 1% left.
pub const USED_UP_AT: u8 = 99;

/// The agent's quota is used up, and when it comes back, as the agent wrote it (`Nov 17 at
/// 12:10am (Asia/Tokyo)`, `7:10pm`, `2026-11-17T00:10`); none when it did not say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsedUp {
    pub resets: Option<String>,
}

/// What `claude -p /usage` printed, read: used up when its "Current session" or "Current week
/// (all models)" line is at [`USED_UP_AT`]% used or more; `Ok(None)` when it has quota left;
/// `Err(())` when neither line is there (it could not be read).
pub fn from_usage(text: &str) -> Result<Option<UsedUp>, ()> {
    let mut read = false;
    for line in text.lines().map(str::trim) {
        let Some(rest) = line
            .strip_prefix("Current session:")
            .or_else(|| line.strip_prefix("Current week (all models):"))
        else {
            continue;
        };
        let Some((used, after)) = rest.split_once("% used") else {
            continue;
        };
        let Ok(used) = used.trim().parse::<u8>() else {
            continue;
        };
        read = true;
        if used >= USED_UP_AT {
            let resets = after
                .split_once("resets")
                .map(|(_, r)| r.trim().to_string())
                .filter(|r| !r.is_empty());
            return Ok(Some(UsedUp { resets }));
        }
    }
    if read { Ok(None) } else { Err(()) }
}

/// The line an agent shows when its quota is used up, in what a session printed (its tab):
/// "You've hit your 5-hour limit · resets 7:10pm" (Claude Code), "You've hit your usage limit …
/// try again at 7:10 PM" (Codex). None when there is no such line.
pub fn from_session_text(text: &str) -> Option<UsedUp> {
    let line = text.lines().rev().find(|l| {
        let l = l.to_ascii_lowercase();
        (l.contains("hit your") && l.contains("limit"))
            || l.contains("usage limit reached")
            || l.contains("quota exceeded")
    })?;
    let lower = line.to_ascii_lowercase();
    let resets = ["resets ", "try again at ", "try again in "]
        .iter()
        .find_map(|key| lower.find(key).map(|i| i + key.len()))
        .map(|i| {
            line[i..]
                .trim()
                .trim_end_matches(['.', '"', '│', '|', '╯', '╰'])
                .trim()
                .to_string()
        })
        .filter(|r| !r.is_empty());
    Some(UsedUp { resets })
}

/// The agent's id in `alm quota` reports (`claude-code`, `codex`, `antigravity`).
pub fn report_agent(agent: &str) -> &'static str {
    crate::request::session_agent(agent)
}

/// From the agents' own `alm quota` reports: used up when the report for this agent's default
/// account (5-hour or week) has at most 1% left and its reset time, a local `YYYY-MM-DDTHH:MM`,
/// is still ahead of `now_local` (same form). A report with no reset time is not used: nothing
/// says when it ends. An extra account (`claude_cli:<id>`) is not judged from the reports.
pub fn from_reports(agent: &str, reports: &[QuotaReport], now_local: &str) -> Option<UsedUp> {
    if agent.contains(':') {
        return None;
    }
    let id = report_agent(agent);
    reports
        .iter()
        .filter(|r| r.agent == id && r.account.is_none())
        .filter(|r| 100 - r.left.min(100) >= USED_UP_AT)
        .filter_map(|r| r.resets.as_deref())
        .filter(|resets| *resets > now_local)
        .max()
        .map(|resets| UsedUp {
            resets: Some(resets.replace('T', " ")),
        })
}

/// Ask a terminal Claude Code for its usage (`claude -p /usage`, answered without the model), in
/// the account's settings folder when it is an extra account. None when it cannot be asked.
pub fn ask_usage(program: &Path, config: Option<(&str, &Path)>) -> Option<String> {
    let mut command = crate::terminal::command(program);
    command
        .args(["-p", "/usage"])
        .stdin(std::process::Stdio::null());
    if let Some((var, dir)) = config {
        command.env(var, dir);
    }
    let out = command.output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).to_string())
}

/// Now, local time, as `YYYY-MM-DDTHH:MM` (from the `date` command; UTC when it cannot be asked).
pub fn now_local() -> String {
    if let Ok(out) = crate::terminal::command("date")
        .arg("+%Y-%m-%dT%H:%M")
        .output()
    {
        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if out.status.success() && text.len() == 16 {
            return text;
        }
    }
    let at = allomos_store::events::now_utc();
    at.get(..16).unwrap_or(&at).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const USAGE: &str = "You are currently using your subscription to power your Claude Code usage

Current session: 73% used · resets Nov 17 at 12:10am (Asia/Tokyo)
Current week (all models): 77% used · resets Nov 19 at 4pm (Asia/Tokyo)
Current week (Fable): 100% used · resets Nov 19 at 4pm (Asia/Tokyo)
";

    #[test]
    fn usage_with_quota_left_is_not_used_up() {
        // One model's own weekly line does not count: the session may run on another model.
        assert_eq!(from_usage(USAGE), Ok(None));
    }

    #[test]
    fn a_used_up_session_says_when_it_comes_back() {
        let text = USAGE.replace("73% used", "100% used");
        assert_eq!(
            from_usage(&text),
            Ok(Some(UsedUp {
                resets: Some("Nov 17 at 12:10am (Asia/Tokyo)".into())
            }))
        );
        let week = USAGE.replace("77% used", "99% used");
        assert_eq!(
            from_usage(&week).unwrap().unwrap().resets.as_deref(),
            Some("Nov 19 at 4pm (Asia/Tokyo)")
        );
        assert_eq!(from_usage("Not logged in"), Err(()));
    }

    #[test]
    fn the_limit_line_in_a_session_is_read() {
        let tab = "╭───╮\n> 请做这个任务\n⎿  You've hit your 5-hour limit · resets 7:10pm\n\n> ";
        assert_eq!(
            from_session_text(tab),
            Some(UsedUp {
                resets: Some("7:10pm".into())
            })
        );
        let codex = "■ You've hit your usage limit. Upgrade to Pro, or try again at 7:10 PM.";
        assert_eq!(
            from_session_text(codex).unwrap().resets.as_deref(),
            Some("7:10 PM")
        );
        assert_eq!(from_session_text("Working… (esc to interrupt)"), None);
    }

    fn report(agent: &str, left: u8, resets: Option<&str>) -> QuotaReport {
        QuotaReport {
            agent: agent.into(),
            account: None,
            window: "5h".into(),
            left,
            resets: resets.map(String::from),
            at: "2026-11-16T07:06:00Z".into(),
        }
    }

    #[test]
    fn a_report_of_nothing_left_counts_until_its_reset_time() {
        let reports = [
            report("codex", 0, Some("2026-11-16T19:10")),
            report("claude-code", 37, Some("2026-11-16T19:10")),
        ];
        assert_eq!(
            from_reports("codex_cli", &reports, "2026-11-16T16:30"),
            Some(UsedUp {
                resets: Some("2026-11-16 19:10".into())
            })
        );
        // Past its reset time, or with quota left, or for an extra account: not used up.
        assert_eq!(
            from_reports("codex_cli", &reports, "2026-11-16T19:10"),
            None
        );
        assert_eq!(
            from_reports("claude_cli", &reports, "2026-11-16T16:30"),
            None
        );
        assert_eq!(
            from_reports("codex_cli:a1", &reports, "2026-11-16T16:30"),
            None
        );
        // No reset time: nothing says when it ends, so it is not used.
        assert_eq!(
            from_reports("codex_cli", &[report("codex", 0, None)], "2026-11-16T16:30"),
            None
        );
    }
}
