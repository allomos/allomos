//! Reopen the agent session a task records (Task-A03-hive049): **Open in Claude** for a session
//! in the Claude desktop app, **Resume in terminal** for one in a terminal. Allomos reads only
//! what the agent passed at `alm claim`; it never reads the Claude app's or Claude Code's own
//! files, so it cannot know whether the session still exists or opened (approver, 2026-10-26).

use std::path::{Path, PathBuf};

use allomos_core::{AgentSession, Problem};

use crate::agents::{
    applescript_string, command, new_console, open_link, powershell_quote, shell_quote,
};

/// How a recorded session is reopened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reopen {
    /// The Claude desktop app's link for its `local_…` session.
    App(String),
    /// `<resume> <id>` in a terminal, in this folder: `claude --resume`, or Antigravity's
    /// `agy --conversation` (Task-A03-queen969).
    Terminal {
        id: String,
        folder: PathBuf,
        resume: &'static str,
    },
}

/// The command that resumes a terminal session of `agent` (what `--session-agent` recorded):
/// Antigravity's `agy --conversation <id>`; Claude Code's `claude --resume <id>` for every other
/// agent, as before.
fn resume_command(agent: Option<&str>) -> &'static str {
    match agent.map(allomos_store::quota_agent_id).as_deref() {
        Some("antigravity") => "agy --conversation",
        _ => "claude --resume",
    }
}

/// An ID only ever has letters, digits, `_`, `-` and `.`: anything else is not put in a link or
/// a command line.
fn plain_id(id: &str) -> Option<&str> {
    let id = id.trim();
    (!id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')))
    .then_some(id)
}

/// What the button does for `session`: the app's link when an app session ID is recorded, else a
/// terminal when a session ID is; `None` when neither (no button). `base` is the folder holding
/// `.allomos/`; the terminal opens in the folder `alm claim` ran in when it is still there.
pub fn reopen(session: &AgentSession, base: &Path) -> Option<Reopen> {
    if let Some(app) = session.app_session_id.as_deref().and_then(plain_id) {
        return Some(Reopen::App(format!("claude://code/continue?session={app}")));
    }
    let id = plain_id(session.id.as_deref()?)?.to_string();
    let folder = session
        .folder
        .as_deref()
        .map(|f| base.join(f.replace('\\', "/")))
        .filter(|f| f.is_dir())
        .unwrap_or_else(|| base.to_path_buf());
    Some(Reopen::Terminal {
        id,
        folder,
        resume: resume_command(session.agent.as_deref()),
    })
}

fn failed(detail: impl ToString) -> Problem {
    Problem::SessionOpenFailed {
        detail: detail.to_string(),
    }
}

/// Do it. The app link is handed to the system; whether the Claude app opened it cannot be known.
/// A terminal shows any error of `claude --resume` itself.
pub fn open(how: &Reopen) -> Result<(), Problem> {
    match how {
        Reopen::App(url) => open_link(url).map_err(failed),
        Reopen::Terminal { id, folder, resume } => open_terminal(id, folder, resume),
    }
}

fn open_terminal(id: &str, folder: &Path, resume: &str) -> Result<(), Problem> {
    if cfg!(windows) {
        let script = format!(
            "Set-Location -LiteralPath {}; {resume} {}",
            powershell_quote(&folder.display().to_string()),
            powershell_quote(id)
        );
        return new_console("powershell.exe")
            .args(["-NoExit", "-NoLogo", "-Command", &script])
            .spawn()
            .map(drop)
            .map_err(failed);
    }
    if !cfg!(target_os = "macos") {
        return Err(failed("not available on this system yet"));
    }
    let script = format!(
        "tell application \"Terminal\"\nactivate\ndo script {}\nend tell",
        applescript_string(&terminal_line(id, folder, resume))
    );
    let status = command("/usr/bin/osascript")
        .args(["-e", &script])
        .status()
        .map_err(failed)?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| failed("Terminal did not open"))
}

/// What the new Terminal window types (macOS).
fn terminal_line(id: &str, folder: &Path, resume: &str) -> String {
    format!(
        "cd {} && {resume} {}",
        shell_quote(&folder.display().to_string()),
        shell_quote(id)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(id: Option<&str>, app: Option<&str>, folder: Option<&str>) -> AgentSession {
        AgentSession {
            id: id.map(str::to_string),
            app_session_id: app.map(str::to_string),
            folder: folder.map(str::to_string),
            ..Default::default()
        }
    }

    #[test]
    fn an_app_session_opens_the_claude_app_link() {
        let s = session(Some("abc"), Some("local_12-ab"), None);
        assert_eq!(
            reopen(&s, Path::new("/p")),
            Some(Reopen::App(
                "claude://code/continue?session=local_12-ab".into()
            ))
        );
    }

    #[test]
    fn a_terminal_session_resumes_in_the_claim_folder_or_the_project() {
        let base = std::env::temp_dir().join(format!("allomos-sessions-{}", std::process::id()));
        std::fs::create_dir_all(base.join("wt/a")).unwrap();
        let s = session(Some("9ad4-1e"), None, Some("wt/a"));
        assert_eq!(
            reopen(&s, &base),
            Some(Reopen::Terminal {
                id: "9ad4-1e".into(),
                folder: base.join("wt/a"),
                resume: "claude --resume",
            })
        );
        // The folder is gone (a removed worktree): the project folder.
        let gone = session(Some("9ad4-1e"), None, Some("wt/b"));
        assert_eq!(
            reopen(&gone, &base),
            Some(Reopen::Terminal {
                id: "9ad4-1e".into(),
                folder: base.clone(),
                resume: "claude --resume",
            })
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn no_session_or_an_odd_id_gives_no_button() {
        assert_eq!(reopen(&session(None, None, None), Path::new("/p")), None);
        assert_eq!(
            reopen(&session(Some("a; rm -rf ~"), None, None), Path::new("/p")),
            None
        );
        // An odd app ID falls back to the terminal when a plain session ID is recorded.
        assert!(matches!(
            reopen(
                &session(Some("ok"), Some("local_x&y=1"), None),
                Path::new("/p")
            ),
            Some(Reopen::Terminal { .. })
        ));
    }

    #[test]
    fn the_terminal_line_quotes_the_folder() {
        assert_eq!(
            terminal_line("ab-1", Path::new("/a b/it's"), "claude --resume"),
            "cd '/a b/it'\\''s' && claude --resume 'ab-1'"
        );
    }

    /// Task-A03-queen969: an Antigravity session resumes with `agy --conversation <id>`.
    #[test]
    fn an_antigravity_session_resumes_with_agy() {
        let mut s = session(Some("c0nv-42"), None, None);
        s.agent = Some("antigravity".into());
        assert_eq!(
            reopen(&s, Path::new("/p")),
            Some(Reopen::Terminal {
                id: "c0nv-42".into(),
                folder: PathBuf::from("/p"),
                resume: "agy --conversation",
            })
        );
        assert_eq!(
            terminal_line("c0nv-42", Path::new("/p"), "agy --conversation"),
            "cd '/p' && agy --conversation 'c0nv-42'"
        );
        // Recorded under another spelling: the same agent.
        assert_eq!(resume_command(Some("Antigravity")), "agy --conversation");
        assert_eq!(resume_command(Some("claude-code")), "claude --resume");
        assert_eq!(resume_command(None), "claude --resume");
    }
}
