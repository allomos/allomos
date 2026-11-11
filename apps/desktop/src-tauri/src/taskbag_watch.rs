//! Following the taskbag's files (Task-A03-spider528, approver 2026-11-12): the system tells the
//! app when a file in an open taskbag changes (FSEvents on macOS, ReadDirectoryChangesW on
//! Windows, through the `notify` crate), instead of each window asking every second. A change
//! reaches every window that shows that taskbag as the event [`EVENT`], with the taskbag's path as
//! the window gave it; the window then compares the taskbag's fingerprint (`taskbag_stamp`) and
//! reads it again when it changed. One write often comes as several notices (an atomic write
//! makes a `.tmp` file and renames it), so notices within [`GATHER`] make one event.
//!
//! The window still checks the fingerprint once a minute, in case a notice is missed (a network
//! drive, a cloud-synced folder, or a watch that could not start).

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::{Duration, Instant};

use ::notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

/// The event a window hears when its taskbag's files changed; the payload is the taskbag's path.
pub const EVENT: &str = "taskbag:changed";
/// Notices this close together make one event.
pub const GATHER: Duration = Duration::from_millis(300);

/// A watch on one folder and the folders inside it, calling `changed` (on a thread of its own)
/// once for each group of notices that come within [`GATHER`] of each other. Dropping it ends the
/// watch. `None` when the system could not start one (the once-a-minute check covers it).
pub struct Watch {
    _watcher: RecommendedWatcher,
}

impl Watch {
    pub fn start(folder: &Path, changed: impl Fn() + Send + 'static) -> Option<Watch> {
        let (tx, rx) = channel::<()>();
        let mut watcher = ::notify::recommended_watcher(move |res: ::notify::Result<::notify::Event>| {
            if let Ok(event) = res {
                if counts(&event.kind) {
                    let _ = tx.send(());
                }
            }
        })
        .ok()?;
        watcher.watch(folder, RecursiveMode::Recursive).ok()?;
        std::thread::spawn(move || {
            while gather(&rx, GATHER) {
                changed();
            }
        });
        Some(Watch { _watcher: watcher })
    }
}

/// Whether a notice can mean a change: reading a file is not one.
fn counts(kind: &EventKind) -> bool {
    !matches!(kind, EventKind::Access(_))
}

/// Waits for a notice, then for the notices that follow it within `quiet` of each other (at most
/// a few times `quiet` in all, so a folder that keeps changing still refreshes). False when the
/// watch has ended.
pub fn gather(rx: &Receiver<()>, quiet: Duration) -> bool {
    if rx.recv().is_err() {
        return false;
    }
    let until = Instant::now() + quiet * 4;
    loop {
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return true;
        }
        match rx.recv_timeout(quiet.min(left)) {
            Ok(()) => continue,
            Err(RecvTimeoutError::Timeout) => return true,
            // The watch ended after a notice: that notice still counts.
            Err(RecvTimeoutError::Disconnected) => return true,
        }
    }
}

/// Which windows show which taskbags, and the watch on each taskbag (one however many windows
/// and tabs show it). Kept in the app's state.
#[derive(Default)]
pub struct Following {
    inner: Mutex<State>,
}

#[derive(Default)]
struct State {
    /// Per window: the taskbags its tabs show, as the window gave them, and how many tabs.
    windows: HashMap<String, HashMap<String, usize>>,
    /// Per taskbag (as given): its watch, or `None` when it could not start.
    watches: HashMap<String, Option<Watch>>,
}

impl State {
    fn shown(&self, root: &str) -> bool {
        self.windows.values().any(|w| w.contains_key(root))
    }

    /// The windows showing `root`.
    fn windows_of(&self, root: &str) -> BTreeSet<String> {
        self.windows
            .iter()
            .filter(|(_, w)| w.contains_key(root))
            .map(|(label, _)| label.clone())
            .collect()
    }

    /// The watches no window needs any more.
    fn unneeded(&mut self) {
        let shown: Vec<String> = self.watches.keys().filter(|r| !self.shown(r)).cloned().collect();
        for root in shown {
            self.watches.remove(&root);
        }
    }
}

impl Following {
    /// A tab of `window` shows `root`; true when a watch on it was started now.
    fn follow(&self, window: &str, root: &str) -> bool {
        let mut s = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        *s.windows
            .entry(window.to_string())
            .or_default()
            .entry(root.to_string())
            .or_default() += 1;
        !s.watches.contains_key(root)
    }

    fn set_watch(&self, root: &str, watch: Option<Watch>) {
        let mut s = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        // Let go meanwhile: no watch.
        if s.shown(root) {
            s.watches.insert(root.to_string(), watch);
        }
    }

    /// A tab of `window` no longer shows `root`.
    fn unfollow(&self, window: &str, root: &str) {
        let mut s = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(w) = s.windows.get_mut(window) {
            if let Some(n) = w.get_mut(root) {
                *n = n.saturating_sub(1);
                if *n == 0 {
                    w.remove(root);
                }
            }
            if w.is_empty() {
                s.windows.remove(window);
            }
        }
        s.unneeded();
    }

    /// The window closed: it shows nothing.
    pub fn forget(&self, window: &str) {
        let mut s = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        s.windows.remove(window);
        s.unneeded();
    }

    /// The windows that show `root` now.
    fn windows_of(&self, root: &str) -> BTreeSet<String> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).windows_of(root)
    }
}

/// A tab of this window shows the taskbag at `root`: its files are watched, and the window hears
/// [`EVENT`] when they change, until [`unfollow_taskbag`] (or the window closes).
#[tauri::command]
pub fn follow_taskbag(window: WebviewWindow, root: String) {
    let app = window.app_handle().clone();
    let following = app.state::<Following>();
    if !following.follow(window.label(), &root) {
        return;
    }
    let told = root.clone();
    let handle = app.clone();
    let watch = Watch::start(&PathBuf::from(&root), move || tell(&handle, &told));
    following.set_watch(&root, watch);
}

/// The tab no longer shows the taskbag at `root`.
#[tauri::command]
pub fn unfollow_taskbag(window: WebviewWindow, root: String) {
    window.state::<Following>().unfollow(window.label(), &root);
}

/// Tells each window showing `root` that its files changed.
fn tell(app: &AppHandle, root: &str) {
    for label in app.state::<Following>().windows_of(root) {
        let _ = app.emit_to(label.as_str(), EVENT, root.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn notices_close_together_make_one_change() {
        let (tx, rx) = channel::<()>();
        for _ in 0..5 {
            tx.send(()).unwrap();
        }
        let start = Instant::now();
        assert!(gather(&rx, Duration::from_millis(50)));
        // All five were taken in one go, and it waited only for the quiet pause after them.
        assert!(rx.try_recv().is_err());
        assert!(start.elapsed() < Duration::from_millis(500));
        drop(tx);
        assert!(!gather(&rx, Duration::from_millis(50)), "an ended watch ends the thread");
    }

    #[test]
    fn a_folder_that_keeps_changing_still_refreshes() {
        let (tx, rx) = channel::<()>();
        let busy = std::thread::spawn(move || {
            for _ in 0..40 {
                if tx.send(()).is_err() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        });
        let start = Instant::now();
        assert!(gather(&rx, Duration::from_millis(50)));
        assert!(start.elapsed() < Duration::from_millis(350), "at most four quiet pauses");
        drop(rx);
        busy.join().unwrap();
    }

    #[test]
    fn reading_a_file_is_not_a_change() {
        assert!(!counts(&EventKind::Access(::notify::event::AccessKind::Any)));
        assert!(counts(&EventKind::Modify(::notify::event::ModifyKind::Any)));
        assert!(counts(&EventKind::Create(::notify::event::CreateKind::File)));
    }

    #[test]
    fn a_change_to_a_file_inside_is_told_within_a_second() {
        let dir = std::env::temp_dir().join(format!("allomos-watch-{}", std::process::id()));
        let tasks = dir.join("tasks").join("A01-x");
        std::fs::create_dir_all(&tasks).unwrap();
        // Canonical, as FSEvents reports /private/var for /var on macOS.
        let dir = std::fs::canonicalize(&dir).unwrap();
        let told = Arc::new(AtomicUsize::new(0));
        let seen = told.clone();
        let watch = Watch::start(&dir, move || {
            seen.fetch_add(1, Ordering::SeqCst);
        })
        .expect("a watch starts");
        // FSEvents may report what happened just before the watch began: let that pass first.
        std::thread::sleep(Duration::from_millis(500));
        let before = told.load(Ordering::SeqCst);
        let file = dir.join("tasks").join("A01-x").join("task.json");
        let start = Instant::now();
        std::fs::write(dir.join("tasks").join("A01-x").join("task.json.tmp"), "{}").unwrap();
        std::fs::rename(dir.join("tasks").join("A01-x").join("task.json.tmp"), &file).unwrap();
        while told.load(Ordering::SeqCst) == before && start.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(20));
        }
        let took = start.elapsed();
        assert!(told.load(Ordering::SeqCst) > before, "the change was told");
        assert!(took < Duration::from_millis(1500), "told after {took:?}");
        drop(watch);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn one_watch_per_taskbag_while_a_tab_shows_it() {
        let f = Following::default();
        assert!(f.follow("main", "/a"), "the first tab starts the watch");
        f.set_watch("/a", None);
        assert!(!f.follow("w2", "/a"), "another window showing it shares the watch");
        assert!(!f.follow("main", "/a"), "so does another tab");
        assert_eq!(f.windows_of("/a").into_iter().collect::<Vec<_>>(), ["main", "w2"]);
        f.unfollow("main", "/a");
        assert_eq!(f.windows_of("/a").len(), 2, "main still has a tab showing it");
        f.forget("w2");
        f.unfollow("main", "/a");
        assert!(f.windows_of("/a").is_empty());
        assert!(f.follow("main", "/a"), "shown again: a new watch");
    }
}
