//! **A quiet Antigravity session showing a confirmation** (Task-A03-candle790): Antigravity runs a
//! title command whenever its state changes (its documented `title` setting), giving it the state
//! as JSON; `tool_confirmation_pending` is true while a confirmation prompt is showing. Monitored
//! quiet sessions get [`title_command`] in their session-only `--settings`: it marks the session's
//! terminal as waiting in [`dir`] (one empty file named after the terminal, such as `ttys011`)
//! while a prompt shows, and removes the mark once it is answered. The run reads the mark for the
//! tab it opened ([`pending_since`]): the session waits for the person, so it frees its slot and
//! the person is told. Nothing else of the state is kept: it also holds the account's details.

use std::path::{Path, PathBuf};

/// The folder of the marks and of the title script, beside the runs' files.
pub fn dir() -> Option<PathBuf> {
    crate::registry::dir().map(|d| d.join("antigravity-confirm"))
}

fn script_path(dir: &Path) -> PathBuf {
    dir.join("title.sh")
}

/// The tab's title while a prompt shows, in the task language; otherwise it reads `Antigravity`.
fn waiting_title(language: &str) -> &'static str {
    match crate::request::lang(language) {
        "zh-CN" => "⏸ Antigravity：等待批准",
        "ja" => "⏸ Antigravity：承認待ち",
        _ => "⏸ Antigravity: waiting for approval",
    }
}

/// The title script: it reads the state on stdin, marks or unmarks its terminal (the terminal of
/// its parent, the Antigravity process) and prints the tab's title.
fn script(marks: &Path, language: &str) -> String {
    let marks = crate::terminal::shell_quote(&marks.display().to_string());
    let waiting = crate::terminal::shell_quote(waiting_title(language));
    format!(
        "#!/bin/sh\n\
         # Allomos (Task-A03-candle790): marks this terminal while Antigravity shows a confirmation.\n\
         state=$(cat)\n\
         tty=$(ps -o tty= -p \"$PPID\" 2>/dev/null | tr -d ' ')\n\
         case \"$tty\" in ''|'?'|*/*) tty='' ;; esac\n\
         case \"$state\" in\n\
         *'\"tool_confirmation_pending\":true'*)\n\
         \x20 [ -n \"$tty\" ] && mkdir -p {marks} && : > {marks}/\"$tty\"\n\
         \x20 printf '%s\\n' {waiting} ;;\n\
         *)\n\
         \x20 [ -n \"$tty\" ] && rm -f {marks}/\"$tty\"\n\
         \x20 printf '%s\\n' 'Antigravity' ;;\n\
         esac\n"
    )
}

/// Write the title script for runs in `language` (at a run's start). Not on Windows: there the
/// sessions run in a console Allomos does not follow, so no mark would be read.
pub fn write_script(language: &str) -> std::io::Result<()> {
    if cfg!(windows) {
        return Ok(());
    }
    let Some(dir) = dir() else {
        return Ok(());
    };
    std::fs::create_dir_all(&dir)?;
    std::fs::write(script_path(&dir), script(&dir, language))
}

/// The `title` command a quiet Antigravity session gets, once the script is there.
pub fn title_command() -> Option<String> {
    let dir = dir()?;
    let path = script_path(&dir);
    path.exists().then(|| {
        format!(
            "sh {}",
            crate::terminal::shell_quote(&path.display().to_string())
        )
    })
}

/// The mark's file for terminal `tty` (`/dev/ttys011` or `ttys011`).
fn mark(dir: &Path, tty: &str) -> Option<PathBuf> {
    let name = tty
        .rsplit('/')
        .next()
        .filter(|n| !n.is_empty() && *n != "..")?;
    Some(dir.join(name))
}

/// Since when (ms) the session in terminal `tty` shows a confirmation; none when it shows none.
/// A mark older than `started` (ms) is a former session's in the same terminal and is ignored.
pub fn pending_since(tty: &str, started: i64) -> Option<i64> {
    pending_in(&dir()?, tty, started)
}

fn pending_in(dir: &Path, tty: &str, started: i64) -> Option<i64> {
    let modified = std::fs::metadata(mark(dir, tty)?).ok()?.modified().ok()?;
    let ms = modified
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_millis() as i64;
    (ms >= started).then_some(ms)
}

/// What a run makes of a session's terminal (A03-table574): a mark counts only while the tab the
/// run opened is still open (`tab_open`, asked only when there is a mark). Once that tab is
/// closed its device may be another window's, and a mark there is that session's, not this one's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Look {
    /// No confirmation shows.
    Clear,
    /// The session shows a confirmation, since then (ms).
    Waiting(i64),
    /// The tab is gone: the session no longer has a tab to follow.
    Gone,
}

/// The look from the mark's time (`since`, from [`pending_since`]) and whether the tab is open.
pub fn look(since: Option<i64>, tab_open: impl FnOnce() -> bool) -> Look {
    match since {
        None => Look::Clear,
        Some(_) if !tab_open() => Look::Gone,
        Some(at) => Look::Waiting(at),
    }
}

/// Remove the mark of terminal `tty` (its session has finished).
pub fn clear(tty: &str) {
    if let Some(path) = dir().and_then(|d| mark(&d, tty)) {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn run_script(dir: &Path, state: &str) -> String {
        let path = script_path(dir);
        let mut child = Command::new("sh")
            .arg(&path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        use std::io::Write;
        child
            .stdin
            .take()
            .unwrap()
            .write_all(state.as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    /// A mark counts only for the tab still open; the tab is asked only when there is a mark.
    #[test]
    fn a_mark_counts_only_while_the_session_tab_is_open() {
        // Really waiting: a mark, and the tab Allomos opened is open.
        assert_eq!(look(Some(5), || true), Look::Waiting(5));
        // The window was closed and its device reused by another session, which shows a prompt:
        // the mark is that session's, and this one's tab is gone.
        assert_eq!(look(Some(5), || false), Look::Gone);
        // No mark: the tab is not asked (no AppleScript each look).
        assert_eq!(look(None, || panic!("asked")), Look::Clear);
    }

    /// The mark follows `tool_confirmation_pending`; the title says so; a mark older than the
    /// session is not its own; no other part of the state is written anywhere.
    #[cfg(unix)]
    #[test]
    fn the_title_script_marks_its_terminal_while_a_prompt_shows() {
        let dir = std::env::temp_dir().join(format!("allomos-confirm-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(script_path(&dir), script(&dir, "zh-CN")).unwrap();
        // Run from a shell whose terminal is known: the test's own (or none, under a runner).
        let tty = Command::new("sh")
            .args(["-c", "ps -o tty= -p $$ | tr -d ' '"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();
        let before = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
            - 2_000;
        let shown = run_script(
            &dir,
            r#"{"agent_state":"tool_use","tool_confirmation_pending":true,"email":"x"}"#,
        );
        assert_eq!(shown, "⏸ Antigravity：等待批准");
        let files: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        if !tty.is_empty() && tty != "?" && tty != "??" {
            assert!(pending_in(&dir, &tty, before).is_some(), "{files:?}");
            assert!(pending_in(&dir, &format!("/dev/{tty}"), before).is_some());
            assert!(pending_in(&dir, &tty, i64::MAX).is_none());
            let mark_text = std::fs::read_to_string(dir.join(&tty)).unwrap();
            assert!(mark_text.is_empty());
        }
        assert_eq!(
            run_script(&dir, r#"{"agent_state":"working"}"#),
            "Antigravity"
        );
        if !tty.is_empty() {
            assert!(pending_in(&dir, &tty, before).is_none());
        }
        assert!(mark(&dir, "/dev/").is_none());
        assert!(mark(&dir, "..").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
