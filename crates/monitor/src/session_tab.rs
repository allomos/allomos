//! The terminal tab a monitored session runs in (Task-A03-sierra014, approver 2026-11-08): Allomos
//! remembers the tab it opened for a task, and once the task is **done** shows a line in it saying
//! the tab closes in 30 seconds unless a key is pressed; nobody presses one, Allomos ends the
//! agent the way a person would (types its exit command) and closes that tab. Every other ending
//! (waiting, blocked, released, dropped, the agent quitting on its own) leaves the tab as it is.
//!
//! macOS Terminal only so far. A key press is seen as the tab's terminal device being read: its
//! access time moves only when input arrives (output moves the modification time), so the agent
//! keeps every key and Allomos needs no permission beyond the Terminal scripting it already uses.

#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

use crate::request;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{Duration, Instant, SystemTime};

/// The tab Allomos opened: its window and its terminal device (`/dev/ttys…`). Both must still
/// match before anything is done to it, so a tab moved to another window, or a device reused by
/// another tab, is left alone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTab {
    pub window: i64,
    pub tty: String,
}

/// What the tab shows, in the interface language.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloseTexts {
    /// "… closes in 30 seconds; press any key to keep it."
    pub prompt: String,
    /// "… stays open."
    pub kept: String,
}

/// How long the prompt waits for a key.
pub const WAIT: Duration = Duration::from_secs(30);
/// How long the agent gets to end after its exit command before the tab is left open.
const END_WAIT: Duration = Duration::from_secs(30);
const POLL: Duration = Duration::from_millis(250);

/// The tab's details from what the opening script returned (`30229, /dev/ttys032`).
pub fn parse_opened(out: &str) -> Option<SessionTab> {
    let (window, tty) = out.trim().split_once(',')?;
    let window = window.trim().parse().ok()?;
    let tty = tty.trim();
    plain_tty(tty).then(|| SessionTab {
        window,
        tty: tty.to_string(),
    })
}

/// A terminal device path, nothing else (it goes into a script and is opened for writing).
fn plain_tty(tty: &str) -> bool {
    tty.strip_prefix("/dev/tty")
        .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_alphanumeric()))
}

/// The AppleScript reference to the tab, when it is still the one Allomos opened.
fn tab_ref(tab: &SessionTab) -> String {
    format!(
        "(first tab of window id {} whose tty is \"{}\")",
        tab.window, tab.tty
    )
}

/// What the line looks like in the tab: on a line of its own, in reverse video.
fn banner(text: &str) -> String {
    format!("\r\n\x1b[7m {text} \x1b[0m\r\n")
}

/// The exit command each agent takes. Typed as a person would: the text, then Return on its own,
/// since Codex takes text and Return arriving together as a paste and does not send it.
pub fn exit_command(agent: &str) -> Option<&'static str> {
    match agent {
        "claude_cli" | "codex_cli" | "antigravity_cli" => Some("/exit"),
        _ => None,
    }
}

/// How it went, for the log.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Closed,
    Kept,
    /// The tab is gone, or is no longer the one Allomos opened.
    NotFound,
    /// The agent was still running after its exit command: the tab stays.
    StillRunning,
    NotAvailable,
}

/// Start the countdown for a done task's tab. It runs in a process of its own: `alm monitor
/// close-tab`, started detached as the monitor itself is (Task-A03-bride850), so the monitor may
/// end, be stopped or be started again and the countdown still runs to its end. Then nothing is
/// returned: the monitor has nothing to wait for or to resume. Only when that process cannot be
/// started does the countdown run in a thread, returned as its handle: the monitor waits for it
/// before it ends and keeps it in the run's file (Task-A03-pocket537).
pub fn close_when_done(
    alm: Option<&Path>,
    log: &Path,
    agent: &str,
    tab: &SessionTab,
    language: &str,
) -> Option<std::thread::JoinHandle<()>> {
    if let Some(alm) = alm {
        match crate::runner::spawn_detached(alm, &close_args(agent, tab, language), log) {
            Ok(_) => return None,
            Err(e) => eprintln!("allomos: the tab countdown could not start its process: {e}"),
        }
    }
    let (agent, tab) = (agent.to_string(), tab.clone());
    let texts = request::close_texts(language);
    Some(std::thread::spawn(move || {
        let outcome = run(&agent, &tab, &texts);
        eprintln!("allomos: monitored session tab {tab:?}: {outcome:?}");
    }))
}

/// The arguments of the countdown process: `alm monitor close-tab …` (see [`countdown`]).
pub fn close_args(agent: &str, tab: &SessionTab, language: &str) -> Vec<String> {
    vec![
        "monitor".to_string(),
        "close-tab".to_string(),
        "--agent".to_string(),
        agent.to_string(),
        "--window".to_string(),
        tab.window.to_string(),
        "--tty".to_string(),
        tab.tty.clone(),
        "--language".to_string(),
        language.to_string(),
    ]
}

/// The whole countdown on the tab, for `alm monitor close-tab` to run in its own process.
pub fn countdown(agent: &str, tab: &SessionTab, language: &str) -> Outcome {
    run(agent, tab, &request::close_texts(language))
}

#[cfg(target_os = "macos")]
fn run(agent: &str, tab: &SessionTab, texts: &CloseTexts) -> Outcome {
    // An extra account's session (`claude_cli:<account>`) exits like its agent's (Task-A03-oven548).
    let Some(exit) = exit_command(crate::accounts::base_agent(agent)) else {
        return Outcome::NotAvailable;
    };
    if !plain_tty(&tab.tty) || busy(tab).is_none() {
        return Outcome::NotFound;
    }
    let before = read_at(&tab.tty);
    if write_tty(&tab.tty, &banner(&texts.prompt)).is_err() {
        return Outcome::NotFound;
    }
    let start = Instant::now();
    while start.elapsed() < WAIT {
        std::thread::sleep(POLL);
        if read_at(&tab.tty) != before {
            let _ = write_tty(&tab.tty, &banner(&texts.kept));
            return Outcome::Kept;
        }
    }
    match busy(tab) {
        None => return Outcome::NotFound,
        Some(true) => {
            // The agent is still running: type its exit command, then Return.
            let _ = type_in(tab, exit);
            std::thread::sleep(Duration::from_secs(1));
            let _ = type_in(tab, "");
            let start = Instant::now();
            loop {
                std::thread::sleep(Duration::from_millis(500));
                match busy(tab) {
                    None => return Outcome::NotFound,
                    Some(false) => break,
                    Some(true) if start.elapsed() >= END_WAIT => return Outcome::StillRunning,
                    Some(true) => {}
                }
            }
        }
        Some(false) => {}
    }
    close(tab)
}

#[cfg(not(target_os = "macos"))]
fn run(_agent: &str, _tab: &SessionTab, _texts: &CloseTexts) -> Outcome {
    Outcome::NotAvailable
}

#[cfg(target_os = "macos")]
fn osascript(script: &str) -> Option<String> {
    let out = crate::terminal::command("/usr/bin/osascript")
        .args(["-e", script])
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Whether something besides the shell runs in the tab; none when the tab is gone.
#[cfg(target_os = "macos")]
fn busy(tab: &SessionTab) -> Option<bool> {
    let out = osascript(&format!(
        "tell application \"Terminal\" to get busy of {}",
        tab_ref(tab)
    ))?;
    match out.as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// Text typed into the tab, followed by Return (Terminal's `do script … in`).
#[cfg(target_os = "macos")]
fn type_in(tab: &SessionTab, text: &str) -> Option<String> {
    osascript(&format!(
        "tell application \"Terminal\" to do script {} in {}",
        crate::terminal::applescript_string(text),
        tab_ref(tab)
    ))
}

/// Close the tab: its window when it is the window's only tab (only the shell is left, so
/// Terminal asks nothing); else end its shell, and Terminal's own settings decide the rest.
#[cfg(target_os = "macos")]
fn close(tab: &SessionTab) -> Outcome {
    let script = format!(
        "tell application \"Terminal\"\nset t to {}\nset w to window id {}\nif (count of tabs of w) is 1 then\nclose w\nelse\ndo script \"exit\" in t\nend if\nend tell",
        tab_ref(tab),
        tab.window
    );
    match osascript(&script) {
        Some(_) => Outcome::Closed,
        None => Outcome::NotFound,
    }
}

/// When the terminal device was last read: it moves when input arrives.
fn read_at(tty: &str) -> Option<SystemTime> {
    std::fs::metadata(tty).and_then(|m| m.accessed()).ok()
}

fn write_tty(tty: &str, text: &str) -> std::io::Result<()> {
    use std::io::Write;
    std::fs::OpenOptions::new()
        .write(true)
        .open(tty)?
        .write_all(text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_opened_tab_is_read_from_the_script_output() {
        assert_eq!(
            parse_opened("30229, /dev/ttys032\n"),
            Some(SessionTab {
                window: 30229,
                tty: "/dev/ttys032".into()
            })
        );
        assert_eq!(parse_opened("30229, /dev/ttys0\"; x"), None);
        assert_eq!(parse_opened("missing value, /dev/ttys1"), None);
        assert_eq!(parse_opened(""), None);
    }

    #[test]
    fn only_the_tab_allomos_opened_is_named() {
        let tab = SessionTab {
            window: 7,
            tty: "/dev/ttys004".into(),
        };
        assert_eq!(
            tab_ref(&tab),
            "(first tab of window id 7 whose tty is \"/dev/ttys004\")"
        );
        assert_eq!(exit_command("claude_cli"), Some("/exit"));
        assert_eq!(exit_command("codex_cli"), Some("/exit"));
        assert_eq!(exit_command("antigravity_cli"), Some("/exit"));
        assert_eq!(exit_command("claude_app"), None);
    }

    #[test]
    fn the_countdown_process_gets_the_tab_and_the_language() {
        let tab = SessionTab {
            window: 30229,
            tty: "/dev/ttys032".into(),
        };
        assert_eq!(
            close_args("claude_cli", &tab, "zh-CN"),
            [
                "monitor",
                "close-tab",
                "--agent",
                "claude_cli",
                "--window",
                "30229",
                "--tty",
                "/dev/ttys032",
                "--language",
                "zh-CN",
            ]
        );
    }

    /// Runs the whole countdown on a real tab: `ALLOMOS_PROBE_TAB="<window>, <tty>"` and
    /// `ALLOMOS_PROBE_AGENT=claude_cli|codex_cli`. Run by hand.
    #[test]
    #[ignore = "acts on a Terminal tab; run by hand"]
    fn probe_close_a_tab() {
        let tab = parse_opened(&std::env::var("ALLOMOS_PROBE_TAB").unwrap()).unwrap();
        let agent = std::env::var("ALLOMOS_PROBE_AGENT").unwrap();
        let texts = CloseTexts {
            prompt: "Allomos: the task is done. This tab closes in 30 seconds; press any key to keep it.".into(),
            kept: "Allomos: this tab stays open.".into(),
        };
        let start = Instant::now();
        let outcome = run(&agent, &tab, &texts);
        println!(
            "outcome {outcome:?} after {} ms",
            start.elapsed().as_millis()
        );
    }
}
