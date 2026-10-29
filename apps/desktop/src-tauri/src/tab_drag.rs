//! Dragging a tab like Chrome (Task-A01-fresco512, approver 2026-10-29). Along its own tab bar the
//! window moves the tab itself; once the tab is pulled out of the bar it tears off, and from then
//! on the app follows the pointer: the tab's window moves with it (a new window showing that
//! taskbag, or the window itself when it was its only tab). Over an Allomos window's tab bar the
//! moving window gets out of the way and that window shows where the tab will land; let go there,
//! the tab joins it; let go elsewhere, the new window stays. Escape puts everything back.
//!
//! The window the drag started in keeps the pointer until it is let go (the system sends it every
//! move and the release), so it says when the drag ends; the app reads the pointer's place itself.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};

use crate::windows::{self, Windows};

/// The tab is over this window's tab bar, at `x` (logical pixels from the window's left edge).
const HOVER_EVENT: &str = "tab:hover";
/// It left this window's tab bar.
const HOVER_END_EVENT: &str = "tab:hover-end";
/// It was let go on this window's tab bar: the window adds it where it showed the gap.
const DROP_EVENT: &str = "tab:drop-here";
/// The drag was called off (Escape): the window it came from puts the tab back.
const CANCELLED_EVENT: &str = "tab:drag-cancelled";
/// The tab bar's height in logical pixels (`.tabbar` in styles.css), and a little more, so a tab
/// let go just under the bar still counts.
const BAR_HEIGHT: f64 = 48.0;
/// How often the moving window follows the pointer: about once a screen refresh.
const TICK: Duration = Duration::from_millis(12);
/// Where the moving window waits while the tab is over a tab bar: off every screen. It stays open
/// and shown, so the window the drag started in keeps the pointer, and nothing takes the keyboard.
const PARKED: PhysicalPosition<f64> = PhysicalPosition {
    x: -100_000.0,
    y: -100_000.0,
};

/// The tab being dragged after it tore off its tab bar, if any.
#[derive(Default)]
pub(crate) struct Drag {
    active: Mutex<Option<Active>>,
    /// Counts drags, so a follower from an earlier one stops.
    run: AtomicU64,
}

struct Active {
    /// The window the tab came from; it holds the pointer.
    source: String,
    root: Option<PathBuf>,
    name: Option<String>,
    /// The window that moves with the pointer: a new one (`created`), or the source itself.
    moving: String,
    created: bool,
    /// The pointer's place in the moving window (from its outer corner), physical pixels.
    offset: (f64, f64),
    /// Where the source window was, when it moves itself (to put it back on Escape).
    home: Option<PhysicalPosition<i32>>,
    /// The window whose tab bar the tab is over.
    over: Option<String>,
    last_x: Option<f64>,
}

impl Drag {
    fn lock(&self) -> MutexGuard<'_, Option<Active>> {
        self.active.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// End the drag: the follower stops, and what it was is returned.
    fn stop(&self) -> Option<Active> {
        self.run.fetch_add(1, Ordering::Relaxed);
        self.lock().take()
    }
}

#[derive(Clone, Serialize)]
struct Hover {
    x: f64,
    root: Option<PathBuf>,
    name: Option<String>,
}

#[derive(Clone, Serialize)]
struct DropHere {
    root: Option<PathBuf>,
}

/// A window's tab bar on the screen, in physical pixels, with the window's scale.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Bar {
    pub(crate) label: String,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) scale: f64,
}

/// The tab bar under the point (`x`, `y`), not window `not`'s: the first in `bars`, which lists the
/// window in front first.
pub(crate) fn bar_under<'a>(bars: &'a [Bar], not: &str, x: f64, y: f64) -> Option<&'a Bar> {
    bars.iter()
        .find(|b| b.label != not && x >= b.x && x < b.x + b.width && y >= b.y && y < b.y + b.height)
}

/// Where the moving window goes for the pointer at `at`: under it, as grabbed.
pub(crate) fn follow_at(at: PhysicalPosition<f64>, offset: (f64, f64)) -> PhysicalPosition<f64> {
    PhysicalPosition::new(at.x - offset.0, at.y - offset.1)
}

/// The project windows' tab bars on the screen, the window in front first.
fn bars(app: &AppHandle) -> Vec<Bar> {
    let mut list: Vec<(bool, Bar)> = app
        .state::<Windows>()
        .labels()
        .into_iter()
        .filter_map(|label| {
            let w = app.get_webview_window(&label)?;
            if !w.is_visible().unwrap_or(false) || w.is_minimized().unwrap_or(false) {
                return None;
            }
            let p = w.inner_position().ok()?;
            let s = w.inner_size().ok()?;
            let scale = w.scale_factor().ok()?;
            Some((
                w.is_focused().unwrap_or(false),
                Bar {
                    label,
                    x: p.x as f64,
                    y: p.y as f64,
                    width: s.width as f64,
                    height: BAR_HEIGHT * scale,
                    scale,
                },
            ))
        })
        .collect();
    list.sort_by_key(|(front, _)| !front);
    list.into_iter().map(|(_, b)| b).collect()
}

/// Follow the pointer until drag `run` ends: move the window under it, or, over a tab bar, park
/// the window and tell that bar's window where the tab is.
fn follow(app: AppHandle, run: u64) {
    loop {
        std::thread::sleep(TICK);
        let drag = app.state::<Drag>();
        if drag.run.load(Ordering::Relaxed) != run {
            return;
        }
        let Ok(at) = app.cursor_position() else {
            continue;
        };
        let bars = bars(&app);
        // The state stays locked while the windows change, so the drag cannot end half-way.
        let mut guard = drag.lock();
        if drag.run.load(Ordering::Relaxed) != run {
            return;
        }
        let Some(a) = guard.as_mut() else {
            return;
        };
        let moving = app.get_webview_window(&a.moving);
        let bar = bar_under(&bars, &a.moving, at.x, at.y);
        let over = bar.map(|b| b.label.clone());
        if over != a.over {
            if let Some(prev) = &a.over {
                let _ = app.emit_to(prev.as_str(), HOVER_END_EVENT, ());
            }
            if over.is_some() {
                if let Some(w) = &moving {
                    let _ = w.set_position(PARKED);
                }
            }
            a.over = over;
            a.last_x = None;
        }
        match bar {
            Some(bar) => {
                let x = ((at.x - bar.x) / bar.scale).round();
                if a.last_x != Some(x) {
                    a.last_x = Some(x);
                    let hover = Hover {
                        x,
                        root: a.root.clone(),
                        name: a.name.clone(),
                    };
                    let _ = app.emit_to(bar.label.as_str(), HOVER_EVENT, hover);
                }
            }
            None => {
                if let Some(w) = &moving {
                    let _ = w.set_position(follow_at(at, a.offset));
                }
            }
        }
    }
}

/// The tab showing `root` (none: the start page), named `name`, was pulled out of the calling
/// window's tab bar, held at `grab` (logical pixels from the window's content corner): it tears
/// off. A new window showing it follows the pointer, or the window itself when it was its only tab.
/// False when it could not start; the tab then stays.
#[tauri::command]
pub(crate) async fn drag_tear(
    app: AppHandle,
    window: tauri::WebviewWindow,
    root: Option<PathBuf>,
    name: Option<String>,
    grab_x: f64,
    grab_y: f64,
) -> bool {
    let Ok(at) = app.cursor_position() else {
        return false;
    };
    let source = window.label().to_string();
    let only = app.state::<Windows>().tab_count(&source) <= 1;
    let (Ok(outer), Ok(inner), Ok(size), Ok(scale)) = (
        window.outer_position(),
        window.inner_position(),
        window.inner_size(),
        window.scale_factor(),
    ) else {
        return false;
    };
    let (moving, created, offset, home) = if only {
        (
            source.clone(),
            false,
            (at.x - outer.x as f64, at.y - outer.y as f64),
            Some(outer),
        )
    } else {
        // The tab sits in the new window's bar where it was grabbed, under the pointer.
        let offset = (
            grab_x * scale + (inner.x - outer.x) as f64,
            grab_y * scale + (inner.y - outer.y) as f64,
        );
        let corner = follow_at(at, offset);
        let opened = windows::open_drag_window(
            &app,
            root.clone(),
            (size.width as f64 / scale, size.height as f64 / scale),
            (corner.x / scale, corner.y / scale),
        );
        match opened {
            Ok(label) => (label, true, offset, None),
            Err(_) => return false,
        }
    };
    let drag = app.state::<Drag>();
    let run = drag.run.fetch_add(1, Ordering::Relaxed) + 1;
    *drag.lock() = Some(Active {
        source,
        root,
        name,
        moving,
        created,
        offset,
        home,
        over: None,
        last_x: None,
    });
    let handle = app.clone();
    std::thread::spawn(move || follow(handle, run));
    true
}

/// The torn-off tab was let go. Over a tab bar it joins that window where the gap was shown
/// (`home` when that is the window it came from, which puts it back itself; `moved` otherwise);
/// elsewhere its new window stays where it is (`moved`), or the window that moved itself stays
/// (`home`). `none` when no tab was being dragged. The window it came from removes it on `moved`.
#[tauri::command]
pub(crate) async fn drag_end(app: AppHandle) -> &'static str {
    let Some(a) = app.state::<Drag>().stop() else {
        return "none";
    };
    let moving = app.get_webview_window(&a.moving);
    let close_moving = || {
        if a.created {
            if let Some(w) = &moving {
                let _ = w.close();
            }
        }
    };
    match &a.over {
        Some(target) if *target == a.source => {
            close_moving();
            "home"
        }
        Some(target) => {
            let _ = app.emit_to(
                target.as_str(),
                DROP_EVENT,
                DropHere {
                    root: a.root.clone(),
                },
            );
            if let Some(w) = app.get_webview_window(target) {
                let _ = w.set_focus();
            }
            close_moving();
            "moved"
        }
        None if a.created => {
            if let Some(w) = &moving {
                let _ = w.set_focus();
            }
            "moved"
        }
        None => "home",
    }
}

/// Escape while a torn-off tab is dragged: the new window closes (or the window that moved itself
/// goes back), and the window it came from puts the tab back where it was. False when no tab was
/// being dragged.
#[tauri::command]
pub(crate) async fn drag_cancel(app: AppHandle) -> bool {
    let Some(a) = app.state::<Drag>().stop() else {
        return false;
    };
    if let Some(over) = &a.over {
        let _ = app.emit_to(over.as_str(), HOVER_END_EVENT, ());
    }
    if let Some(w) = app.get_webview_window(&a.moving) {
        if a.created {
            let _ = w.close();
        } else if let Some(home) = a.home {
            let _ = w.set_position(home);
        }
    }
    let _ = app.emit_to(a.source.as_str(), CANCELLED_EVENT, ());
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tab_bar_under_the_pointer_is_found() {
        let bar = |label: &str, x: f64| Bar {
            label: label.into(),
            x,
            y: 100.0,
            width: 100.0,
            height: 48.0,
            scale: 1.0,
        };
        // `main` at 0–100, `window-2` at 200–300, `window-3` in front overlapping it at 250–350.
        let bars = [
            bar("window-3", 250.0),
            bar("main", 0.0),
            bar("window-2", 200.0),
        ];
        let at = |x: f64, y: f64| bar_under(&bars, "window-9", x, y).map(|b| b.label.as_str());
        assert_eq!(at(50.0, 120.0), Some("main"));
        assert_eq!(at(220.0, 120.0), Some("window-2"));
        // Where two overlap, the window in front.
        assert_eq!(at(260.0, 120.0), Some("window-3"));
        // Below a bar (over the window's content) and between windows: none.
        assert_eq!(at(50.0, 160.0), None);
        assert_eq!(at(150.0, 120.0), None);
        // The moving window's own bar never counts.
        assert_eq!(bar_under(&bars, "main", 50.0, 120.0), None);
    }

    #[test]
    fn the_moving_window_keeps_the_tab_under_the_pointer() {
        let at = follow_at(PhysicalPosition::new(500.0, 300.0), (60.0, 40.0));
        assert_eq!((at.x, at.y), (440.0, 260.0));
    }
}
