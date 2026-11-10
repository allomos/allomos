//! The terminal tab a monitored session runs in (Task-A03-sierra014, approver 2026-11-08): Allomos
//! remembers the tab it opened for a task, and once the task is **done** and its agent is idle
//! (or has exited; never while it is still working, A03-birch179) shows a line in it saying the
//! tab closes in 30 seconds unless a key is pressed; nobody presses one, Allomos ends the agent the
//! way a person would (types its exit command) and closes that tab. Every other ending (waiting,
//! blocked, released, dropped, the agent quitting on its own) leaves the tab as it is.
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
/// How long a tab's output must stay still before its agent counts as idle. A working agent keeps
/// printing (a spinner, a timer); an idle one sits at its prompt and prints nothing (A03-birch179).
pub const QUIET: Duration = Duration::from_secs(10);

/// The timings of one countdown: the real ones ([`Timing::default`]), or shorter ones in tests.
#[derive(Debug, Clone, Copy)]
struct Timing {
    wait: Duration,
    end_wait: Duration,
    quiet: Duration,
    poll: Duration,
    /// The pause between the exit command and its Return.
    typing: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Timing {
            wait: WAIT,
            end_wait: END_WAIT,
            quiet: QUIET,
            poll: POLL,
            typing: Duration::from_secs(1),
        }
    }
}

/// What the tab's agent is doing, as the countdown reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Settled {
    /// The tab is gone (or no longer the one Allomos opened).
    Gone,
    /// The agent is working: the countdown waits.
    Working,
    /// The agent is at its prompt, or has exited: the countdown may run.
    Idle,
}

/// The agent's state from what the tab shows. `busy` says whether something besides the shell runs
/// in the tab (none: the tab is gone); `output` is when its output last moved. Busy and still
/// printing is working; busy and quiet is idle at the prompt; not busy is idle (exited). An unknown
/// output time counts as working, so a countdown never starts on a guess.
pub fn settled(
    busy: Option<bool>,
    output: Option<SystemTime>,
    now: SystemTime,
    quiet: Duration,
) -> Settled {
    match busy {
        None => Settled::Gone,
        Some(false) => Settled::Idle,
        Some(true) => {
            let printing = match output {
                None => true,
                Some(at) => match now.duration_since(at) {
                    Ok(age) => age < quiet,
                    // A time after now (the clock was set back) counts as printing.
                    Err(_) => true,
                },
            };
            if printing {
                Settled::Working
            } else {
                Settled::Idle
            }
        }
    }
}

/// The tab as the countdown sees it: the macOS calls ([`MacTab`]), or a fake in tests.
trait TabIo {
    /// Whether something besides the shell runs in the tab; none when the tab is gone.
    fn busy(&mut self) -> Option<bool>;
    /// When the tab's output last moved (its terminal device's modification time).
    fn output_at(&mut self) -> Option<SystemTime>;
    /// When the tab's input last arrived (its terminal device's access time).
    fn input_at(&mut self) -> Option<SystemTime>;
    /// Writes raw text into the tab; false when it cannot be written to.
    fn write_raw(&mut self, text: &str) -> bool;
    /// Types text into the tab, then Return.
    fn type_text(&mut self, text: &str);
    /// Closes the tab.
    fn close_tab(&mut self) -> Outcome;
}

/// Waits while the agent works. Returns the busy flag once it is idle; none when the tab is gone.
fn wait_until_idle(io: &mut impl TabIo, timing: &Timing) -> Option<bool> {
    loop {
        let busy = io.busy();
        match settled(busy, io.output_at(), SystemTime::now(), timing.quiet) {
            Settled::Gone => return None,
            Settled::Idle => return busy,
            Settled::Working => std::thread::sleep(timing.poll),
        }
    }
}

/// The countdown for a done task's tab. It starts only once the agent is idle, so a working agent
/// gets no line and no exit command. Then a key press keeps the tab; otherwise the exit command is
/// typed (when the agent is still running) and the tab closes.
fn countdown_on(io: &mut impl TabIo, exit: &str, texts: &CloseTexts, timing: &Timing) -> Outcome {
    if wait_until_idle(io, timing).is_none() {
        return Outcome::NotFound;
    }
    let before = io.input_at();
    if !io.write_raw(&banner(&texts.prompt)) {
        return Outcome::NotFound;
    }
    let start = Instant::now();
    while start.elapsed() < timing.wait {
        std::thread::sleep(timing.poll);
        if io.input_at() != before {
            io.write_raw(&banner(&texts.kept));
            return Outcome::Kept;
        }
    }
    // The agent may have started work during the countdown: wait for it again, so the exit command
    // never reaches a working agent.
    match wait_until_idle(io, timing) {
        None => return Outcome::NotFound,
        Some(true) => {
            io.type_text(exit);
            std::thread::sleep(timing.typing);
            io.type_text("");
            let start = Instant::now();
            loop {
                std::thread::sleep(timing.poll);
                match io.busy() {
                    None => return Outcome::NotFound,
                    Some(false) => break,
                    Some(true) if start.elapsed() >= timing.end_wait => {
                        return Outcome::StillRunning;
                    }
                    Some(true) => {}
                }
            }
        }
        Some(false) => {}
    }
    io.close_tab()
}

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
    if !plain_tty(&tab.tty) {
        return Outcome::NotFound;
    }
    countdown_on(&mut MacTab { tab }, exit, texts, &Timing::default())
}

/// The macOS tab: Terminal's busy flag, and the tab's terminal device for output and input.
#[cfg(target_os = "macos")]
struct MacTab<'a> {
    tab: &'a SessionTab,
}

#[cfg(target_os = "macos")]
impl TabIo for MacTab<'_> {
    fn busy(&mut self) -> Option<bool> {
        busy(self.tab)
    }

    fn output_at(&mut self) -> Option<SystemTime> {
        modified_at(&self.tab.tty)
    }

    fn input_at(&mut self) -> Option<SystemTime> {
        read_at(&self.tab.tty)
    }

    fn write_raw(&mut self, text: &str) -> bool {
        write_tty(&self.tab.tty, text).is_ok()
    }

    fn type_text(&mut self, text: &str) {
        let _ = type_in(self.tab, text);
    }

    fn close_tab(&mut self) -> Outcome {
        close(self.tab)
    }
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

/// When the terminal device was last written to: output moves it.
#[cfg(target_os = "macos")]
fn modified_at(tty: &str) -> Option<SystemTime> {
    std::fs::metadata(tty).and_then(|m| m.modified()).ok()
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

    /// A tab the countdown runs on in tests. `scene` gives, for the busy checks so far and whether
    /// the exit command was typed, the busy flag and whether output is moving. A key press lands
    /// once the prompt is on screen when `key` is set.
    struct FakeTab {
        scene: fn(u32, bool) -> (Option<bool>, bool),
        key: bool,
        calls: u32,
        shown: Vec<String>,
        first_shown_at: Option<u32>,
        typed: Vec<String>,
        closed: bool,
    }

    impl FakeTab {
        fn new(scene: fn(u32, bool) -> (Option<bool>, bool), key: bool) -> FakeTab {
            FakeTab {
                scene,
                key,
                calls: 0,
                shown: Vec::new(),
                first_shown_at: None,
                typed: Vec::new(),
                closed: false,
            }
        }

        fn now_state(&self) -> (Option<bool>, bool) {
            (self.scene)(self.calls, self.typed.iter().any(|t| t == "/exit"))
        }
    }

    impl TabIo for FakeTab {
        fn busy(&mut self) -> Option<bool> {
            self.calls += 1;
            self.now_state().0
        }

        fn output_at(&mut self) -> Option<SystemTime> {
            let now = SystemTime::now();
            if self.now_state().1 {
                Some(now)
            } else {
                Some(now.checked_sub(Duration::from_secs(3600)).unwrap_or(now))
            }
        }

        fn input_at(&mut self) -> Option<SystemTime> {
            let base = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
            if self.key && !self.shown.is_empty() {
                Some(base + Duration::from_secs(1))
            } else {
                Some(base)
            }
        }

        fn write_raw(&mut self, text: &str) -> bool {
            if self.first_shown_at.is_none() {
                self.first_shown_at = Some(self.calls);
            }
            self.shown.push(text.to_string());
            true
        }

        fn type_text(&mut self, text: &str) {
            self.typed.push(text.to_string());
        }

        fn close_tab(&mut self) -> Outcome {
            self.closed = true;
            Outcome::Closed
        }
    }

    /// Timings short enough for tests; output counts as moving for 50 ms after it moved.
    fn quick() -> Timing {
        Timing {
            wait: Duration::from_millis(30),
            end_wait: Duration::from_millis(300),
            quiet: Duration::from_millis(50),
            poll: Duration::from_millis(1),
            typing: Duration::from_millis(1),
        }
    }

    fn texts() -> CloseTexts {
        CloseTexts {
            prompt: "prompt".into(),
            kept: "kept".into(),
        }
    }

    /// Working (output moving) for five checks, then the tab is closed by the person.
    fn working_then_closed(calls: u32, _exited: bool) -> (Option<bool>, bool) {
        if calls < 5 {
            (Some(true), true)
        } else {
            (None, false)
        }
    }

    /// Working for three checks, then idle at its prompt until the exit command is typed.
    fn idle_at_prompt(calls: u32, exited: bool) -> (Option<bool>, bool) {
        match (calls < 3, exited) {
            (true, _) => (Some(true), true),
            (false, false) => (Some(true), false),
            (false, true) => (Some(false), false),
        }
    }

    /// Working for four checks, then the agent exits on its own.
    fn working_then_exits(calls: u32, _exited: bool) -> (Option<bool>, bool) {
        if calls < 4 {
            (Some(true), true)
        } else {
            (Some(false), false)
        }
    }

    /// Idle at its prompt for ever.
    fn idle_forever(_calls: u32, _exited: bool) -> (Option<bool>, bool) {
        (Some(true), false)
    }

    /// A done task whose agent is still working gets no countdown line and no exit command.
    #[test]
    fn a_working_agent_gets_no_countdown_line_or_exit_command() {
        let mut tab = FakeTab::new(working_then_closed, false);
        let outcome = countdown_on(&mut tab, "/exit", &texts(), &quick());
        assert_eq!(outcome, Outcome::NotFound);
        assert!(tab.shown.is_empty());
        assert!(tab.typed.is_empty());
        assert!(!tab.closed);
    }

    /// The countdown starts once the agent is idle; with nothing pressed, the exit command is
    /// typed and the tab closes. The line never shows while the agent was still working.
    #[test]
    fn the_countdown_starts_once_the_agent_is_idle() {
        let mut tab = FakeTab::new(idle_at_prompt, false);
        let outcome = countdown_on(&mut tab, "/exit", &texts(), &quick());
        assert_eq!(outcome, Outcome::Closed);
        assert_eq!(tab.shown, vec![banner("prompt")]);
        assert!(tab.first_shown_at.unwrap() >= 3);
        assert_eq!(tab.typed, ["/exit", ""]);
        assert!(tab.closed);
    }

    /// An agent that exits on its own while working: the line shows only after it stopped, and no
    /// exit command is typed.
    #[test]
    fn an_agent_that_exits_by_itself_gets_no_exit_command() {
        let mut tab = FakeTab::new(working_then_exits, false);
        let outcome = countdown_on(&mut tab, "/exit", &texts(), &quick());
        assert_eq!(outcome, Outcome::Closed);
        assert!(tab.typed.is_empty());
        assert!(tab.first_shown_at.unwrap() >= 4);
    }

    /// A key press during the countdown keeps the tab: the kept line shows and nothing is typed.
    #[test]
    fn a_key_press_keeps_the_tab() {
        let mut tab = FakeTab::new(idle_forever, true);
        let outcome = countdown_on(&mut tab, "/exit", &texts(), &quick());
        assert_eq!(outcome, Outcome::Kept);
        assert_eq!(tab.shown, vec![banner("prompt"), banner("kept")]);
        assert!(tab.typed.is_empty());
        assert!(!tab.closed);
    }

    #[test]
    fn a_tab_is_working_while_its_agent_prints_and_idle_when_quiet() {
        let now = SystemTime::now();
        let quiet = Duration::from_secs(10);
        let recent = Some(now - Duration::from_secs(2));
        let old = Some(now - Duration::from_secs(60));
        assert_eq!(settled(None, recent, now, quiet), Settled::Gone);
        assert_eq!(settled(Some(false), recent, now, quiet), Settled::Idle);
        assert_eq!(settled(Some(true), recent, now, quiet), Settled::Working);
        assert_eq!(settled(Some(true), old, now, quiet), Settled::Idle);
        assert_eq!(settled(Some(true), None, now, quiet), Settled::Working);
        let ahead = Some(now + Duration::from_secs(5));
        assert_eq!(settled(Some(true), ahead, now, quiet), Settled::Working);
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
