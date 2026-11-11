//! One loop for everything about **Monitor ready tasks** that the app keeps up to date
//! (Task-A03-spider528, approver 2026-11-12): each round it reads this computer's run files and
//! the system's process list once (`ps`), and
//! - when the runs changed, tells every window ([`EVENT`], with the runs as `monitor_all` lists
//!   them): the Monitors bar takes them as they are, and a project's status bars read their
//!   project's runs again (`monitor_runs`, which uses this round's process list);
//! - keeps the View menu's **Running Monitors** item's count ([`crate::monitor_menu`]);
//! - shows the notifications runs left (Task-A03-stork103): a task waiting on the person, shown
//!   under the app's own name and icon, and a click brings the app forward. A run
//!   (`alm monitor run`, which may outlive the app) leaves each one as a note in the monitors
//!   folder ([`registry::leave_note`]); notes left while the app is not running wait until it runs.
//!
//! The windows no longer ask on their own. A round comes every [`IN_USE`] while a window is in use
//! (shown and with the keyboard focus, as Task-A03-reef286 has it) and every [`IDLE`] otherwise;
//! a window coming into use starts a round at once.

use std::collections::HashSet;
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use allomos_monitor::registry::{self, Entry};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_notification::NotificationExt;

/// The event every window hears when the runs changed; the payload is the list of runs.
pub const EVENT: &str = "monitors:changed";
/// The pause between rounds while a window is in use.
pub const IN_USE: Duration = Duration::from_secs(3);
/// The pause between rounds while none is.
pub const IDLE: Duration = Duration::from_secs(15);

/// The pause before the next round.
pub fn pace(in_use: bool) -> Duration {
    if in_use { IN_USE } else { IDLE }
}

/// What the loop knows about the windows, kept in the app's state.
pub struct MonitorLoop {
    /// The windows that have the keyboard focus (in use).
    focused: Mutex<HashSet<String>>,
    /// Starts a round now.
    wake: Mutex<Sender<()>>,
    /// The loop's end of `wake`, taken when the loop starts.
    woken: Mutex<Option<Receiver<()>>>,
}

impl Default for MonitorLoop {
    fn default() -> Self {
        let (tx, rx) = channel();
        MonitorLoop {
            focused: Mutex::new(HashSet::new()),
            wake: Mutex::new(tx),
            woken: Mutex::new(Some(rx)),
        }
    }
}

impl MonitorLoop {
    /// Window `label` got (`true`) or lost the keyboard focus. Getting it starts a round now.
    pub fn focus(&self, label: &str, focused: bool) {
        let mut set = self.focused.lock().unwrap_or_else(|e| e.into_inner());
        let came = if focused {
            set.insert(label.to_string())
        } else {
            set.remove(label);
            false
        };
        drop(set);
        if came {
            self.wake_now();
        }
    }

    /// The window closed.
    pub fn forget(&self, label: &str) {
        self.focus(label, false);
    }

    /// Whether some window is in use.
    pub fn in_use(&self) -> bool {
        !self.focused.lock().unwrap_or_else(|e| e.into_inner()).is_empty()
    }

    /// Starts a round now (after the window asked a run for something, say).
    pub fn wake_now(&self) {
        let _ = self.wake.lock().unwrap_or_else(|e| e.into_inner()).send(());
    }
}

/// What a round found, and what it should tell.
#[derive(Default)]
pub struct Seen {
    runs: Option<Value>,
    count: Option<usize>,
}

/// What to tell after a round that found `runs`: the runs when they changed, the count when it did.
#[derive(Debug, PartialEq)]
pub struct Tell {
    pub runs: Option<Value>,
    pub count: Option<usize>,
}

impl Seen {
    /// Only what changed since the last round is told.
    pub fn round(&mut self, runs: Value) -> Tell {
        let count = runs.as_array().map_or(0, Vec::len);
        let tell = Tell {
            runs: (self.runs.as_ref() != Some(&runs)).then(|| runs.clone()),
            count: (self.count != Some(count)).then_some(count),
        };
        self.runs = Some(runs);
        self.count = Some(count);
        tell
    }
}

/// The runs on this computer now, as `monitor_all` lists them, read with a fresh process list
/// (which `monitor_runs` and `monitor_all` then share, [`registry::recent_processes`]).
fn read_runs() -> Value {
    let Some(dir) = registry::dir() else {
        return Value::Array(Vec::new());
    };
    let procs = registry::recent_processes(Duration::ZERO);
    Value::Array(registry::list(&dir, Some(&procs)).iter().map(Entry::json).collect())
}

/// Starts the loop; it runs until the app closes.
pub fn start(app: AppHandle) {
    let state = app.state::<MonitorLoop>();
    let Some(woken) = state.woken.lock().unwrap_or_else(|e| e.into_inner()).take() else {
        return;
    };
    std::thread::spawn(move || {
        let mut seen = Seen::default();
        loop {
            show_notes(&app);
            let tell = seen.round(read_runs());
            if let Some(runs) = tell.runs {
                let _ = app.emit(EVENT, runs);
            }
            if let Some(n) = tell.count {
                crate::monitor_menu::show(&app, n);
            }
            let wait = pace(app.state::<MonitorLoop>().in_use());
            // Woken early or the pause is over: either way, the next round. Several wakes at once
            // make one round.
            let _ = woken.recv_timeout(wait);
            while woken.try_recv().is_ok() {}
        }
    });
}

/// Shows each note left in the monitors folder, oldest first, and removes it.
fn show_notes<R: Runtime>(app: &AppHandle<R>) {
    let Some(dir) = registry::dir() else {
        return;
    };
    for note in registry::take_notes(&dir) {
        // A notification that cannot be shown is left out (the status bar says the same thing).
        let _ = app
            .notification()
            .builder()
            .title(note.title)
            .body(note.body)
            .show();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn quick_while_in_use_slow_otherwise() {
        assert_eq!(pace(true), Duration::from_secs(3));
        assert_eq!(pace(false), Duration::from_secs(15));
    }

    #[test]
    fn a_window_in_use_is_one_with_the_focus() {
        let l = MonitorLoop::default();
        assert!(!l.in_use());
        l.focus("main", true);
        assert!(l.in_use());
        l.focus("w2", true);
        l.focus("main", false);
        assert!(l.in_use(), "w2 still has it");
        l.forget("w2");
        assert!(!l.in_use());
    }

    #[test]
    fn coming_into_use_starts_a_round_at_once() {
        let l = MonitorLoop::default();
        let woken = l.woken.lock().unwrap().take().unwrap();
        l.focus("main", true);
        assert!(woken.try_recv().is_ok(), "getting the focus wakes the loop");
        l.focus("main", true);
        assert!(woken.try_recv().is_err(), "already in use: no extra round");
        l.focus("main", false);
        assert!(woken.try_recv().is_err(), "losing it does not");
    }

    #[test]
    fn only_a_change_is_told() {
        let mut seen = Seen::default();
        let none = json!([]);
        assert_eq!(seen.round(none.clone()), Tell { runs: Some(none.clone()), count: Some(0) }, "the first round tells");
        assert_eq!(seen.round(none.clone()), Tell { runs: None, count: None }, "nothing new");
        let one = json!([{ "id": "r1", "checked": "10:00" }]);
        assert_eq!(seen.round(one.clone()), Tell { runs: Some(one.clone()), count: Some(1) });
        let checked = json!([{ "id": "r1", "checked": "10:05" }]);
        assert_eq!(seen.round(checked.clone()), Tell { runs: Some(checked), count: None }, "a new check: the runs, not the count");
        assert_eq!(seen.round(none.clone()), Tell { runs: Some(none), count: Some(0) });
    }
}
