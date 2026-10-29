//! Dragging a tab like Chrome (Task-A01-fresco512, approver 2026-10-29). Along its own tab bar the
//! window moves the tab itself; once the tab is pulled out of the bar it tears off, and from then
//! on the app follows the pointer: the tab's window moves with it (a new window showing that
//! taskbag, or the window itself when it was its only tab). Anywhere over another Allomos window
//! (Task-A01-twig481), or over its own window's tab bar, the moving window gets out of the way and
//! that window shows where the tab will land; let go there, the tab joins it; let go elsewhere, the
//! new window stays. Escape puts everything back.
//!
//! The window the drag started in keeps the pointer until it is let go (the system sends it every
//! move and the release), so it says when the drag ends; the app reads the pointer's place itself.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::windows::{self, Windows};

/// The tab is over this window, at `x` (logical pixels from the window's left edge), held `grab`
/// logical pixels from a tab bar's left edge when its tab is first (Task-A01-bell953).
const HOVER_EVENT: &str = "tab:hover";
/// It left this window.
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

/// Places on the screen in one unit for every display (Task-A01-jackal747). On macOS, Tauri's
/// physical pixels are not one unit: the pointer's use the main display's scale, a window's its own
/// display's, so with a Retina and a standard display they disagree and the moving window jumped.
/// There the system's points are shared by every display; on Windows, physical pixels are.
mod screen {
    use tauri::{AppHandle, WebviewWindow};

    /// How many units one of the window's logical (CSS) pixels is.
    #[cfg(target_os = "macos")]
    pub(super) fn per_logical(_window: &WebviewWindow) -> Option<f64> {
        Some(1.0)
    }
    #[cfg(not(target_os = "macos"))]
    pub(super) fn per_logical(window: &WebviewWindow) -> Option<f64> {
        window.scale_factor().ok()
    }

    /// The pointer's place.
    pub(super) fn cursor(app: &AppHandle) -> Option<(f64, f64)> {
        let at = app.cursor_position().ok()?;
        #[cfg(target_os = "macos")]
        let scale = app.primary_monitor().ok().flatten()?.scale_factor();
        #[cfg(not(target_os = "macos"))]
        let scale = 1.0;
        Some((at.x / scale, at.y / scale))
    }

    /// The units for `window`'s physical pixels.
    fn of(window: &WebviewWindow) -> Option<f64> {
        Some(window.scale_factor().ok()? / per_logical(window)?)
    }

    /// The window's content: its corner and size.
    pub(super) fn inner(window: &WebviewWindow) -> Option<(f64, f64, f64, f64)> {
        let k = of(window)?;
        let p = window.inner_position().ok()?;
        let s = window.inner_size().ok()?;
        Some((
            p.x as f64 / k,
            p.y as f64 / k,
            s.width as f64 / k,
            s.height as f64 / k,
        ))
    }

    /// The whole window, title bar included: its outer corner and size.
    pub(super) fn outer(window: &WebviewWindow) -> Option<(f64, f64, f64, f64)> {
        let k = of(window)?;
        let p = window.outer_position().ok()?;
        let s = window.outer_size().ok()?;
        Some((
            p.x as f64 / k,
            p.y as f64 / k,
            s.width as f64 / k,
            s.height as f64 / k,
        ))
    }

    /// Get the moving window out of the way while the tab is over a tab bar (`parked`), or bring it
    /// back. It stays open and shown, so the window the drag started in keeps the pointer, and
    /// nothing takes the keyboard. macOS does not let a window leave every screen (it put it back in
    /// the top-left corner, Task-A01-jungle176), so there it turns see-through where it is;
    /// elsewhere it moves off every screen, and following the pointer brings it back.
    #[cfg(target_os = "macos")]
    pub(super) fn park(window: &WebviewWindow, parked: bool) {
        let w = window.clone();
        let _ = window.run_on_main_thread(move || {
            let Ok(ns) = w.ns_window() else {
                return;
            };
            let alpha: f64 = if parked { 0.0 } else { 1.0 };
            // SAFETY: `ns` is this window's live NSWindow, used on the main thread.
            unsafe {
                let ns = &*(ns as *const objc2::runtime::AnyObject);
                let _: () = objc2::msg_send![ns, setAlphaValue: alpha];
            }
        });
    }
    #[cfg(not(target_os = "macos"))]
    pub(super) fn park(window: &WebviewWindow, parked: bool) {
        if parked {
            place(window, (-100_000.0, -100_000.0));
        }
    }

    /// Bring the window to the top of the others without taking the keyboard, so the window the
    /// drag started in keeps it (Task-A01-skirt821): on macOS ordered to the front without becoming
    /// the key window; elsewhere kept on top for a moment.
    #[cfg(target_os = "macos")]
    pub(super) fn raise(window: &WebviewWindow) {
        let w = window.clone();
        let _ = window.run_on_main_thread(move || {
            let Ok(ns) = w.ns_window() else {
                return;
            };
            // SAFETY: `ns` is this window's live NSWindow, used on the main thread.
            unsafe {
                let ns = &*(ns as *const objc2::runtime::AnyObject);
                let none: *const objc2::runtime::AnyObject = std::ptr::null();
                let _: () = objc2::msg_send![ns, orderFront: none];
            }
        });
    }
    #[cfg(not(target_os = "macos"))]
    pub(super) fn raise(window: &WebviewWindow) {
        let _ = window.set_always_on_top(true);
        let _ = window.set_always_on_top(false);
    }

    /// Put the window's outer corner at `(x, y)`.
    pub(super) fn place(window: &WebviewWindow, (x, y): (f64, f64)) {
        #[cfg(target_os = "macos")]
        let _ = window.set_position(tauri::LogicalPosition::new(x, y));
        #[cfg(not(target_os = "macos"))]
        let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
    }
}

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
    /// Where the tab was grabbed: logical pixels from a tab bar's left edge when it is first.
    grab_x: f64,
    /// The pointer's place in the moving window (from its outer corner), in `screen` units.
    offset: (f64, f64),
    /// Where the source window was, when it moves itself (to put it back on Escape).
    home: Option<(f64, f64)>,
    /// The window the tab is over.
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
    grab: f64,
    root: Option<PathBuf>,
    name: Option<String>,
}

#[derive(Clone, Serialize)]
struct DropHere {
    root: Option<PathBuf>,
}

/// A window that takes a dragged tab, on the screen in `screen` units: the whole window (`x`, `y`,
/// `width`, `height`, title bar included), its content's left edge, the bottom of its tab bar, and
/// how many units one of the window's logical pixels is.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Bar {
    pub(crate) label: String,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) left: f64,
    pub(crate) bar_bottom: f64,
    pub(crate) scale: f64,
}

/// The window under the point (`x`, `y`) that takes the tab, not window `not`'s: the first in
/// `bars`, which lists the window in front first. Any window takes it anywhere over it
/// (Task-A01-twig481), but the window it came from, `home`, only over its tab bar, so a tab can
/// still be pulled down out of its own window.
pub(crate) fn bar_under<'a>(
    bars: &'a [Bar],
    not: &str,
    home: &str,
    x: f64,
    y: f64,
) -> Option<&'a Bar> {
    bars.iter().find(|b| {
        b.label != not
            && x >= b.x
            && x < b.x + b.width
            && y >= b.y
            && y < b.y + b.height
            && (b.label != home || y < b.bar_bottom)
    })
}

/// Where the moving window goes for the pointer at `at`: under it, as grabbed.
pub(crate) fn follow_at(at: (f64, f64), offset: (f64, f64)) -> (f64, f64) {
    (at.0 - offset.0, at.1 - offset.1)
}

/// The project windows on the screen, the window in front first.
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
            let (x, y, width, height) = screen::outer(&w)?;
            let (left, top, _, _) = screen::inner(&w)?;
            let scale = screen::per_logical(&w)?;
            Some((
                w.is_focused().unwrap_or(false),
                Bar {
                    label,
                    x,
                    y,
                    width,
                    height,
                    left,
                    bar_bottom: top + BAR_HEIGHT * scale,
                    scale,
                },
            ))
        })
        .collect();
    list.sort_by_key(|(front, _)| !front);
    list.into_iter().map(|(_, b)| b).collect()
}

/// Follow the pointer until drag `run` ends: move the window under it, or, over a window that takes
/// the tab, park the moving window and tell that window where the tab is.
fn follow(app: AppHandle, run: u64) {
    loop {
        std::thread::sleep(TICK);
        let drag = app.state::<Drag>();
        if drag.run.load(Ordering::Relaxed) != run {
            return;
        }
        let Some(at) = screen::cursor(&app) else {
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
        // The window it is over keeps it while the pointer stays there: that window was raised, so it
        // is on top even where the window in front overlaps it.
        let bar = a
            .over
            .as_deref()
            .and_then(|label| bars.iter().find(|b| b.label == label))
            .filter(|b| {
                bar_under(std::slice::from_ref(*b), &a.moving, &a.source, at.0, at.1).is_some()
            })
            .or_else(|| bar_under(&bars, &a.moving, &a.source, at.0, at.1));
        let over = bar.map(|b| b.label.clone());
        // Back from a tab bar: the window shows again once it is under the pointer.
        let back = over.is_none() && a.over.is_some();
        if over != a.over {
            if let Some(prev) = &a.over {
                let _ = app.emit_to(prev.as_str(), HOVER_END_EVENT, ());
            }
            if let Some(label) = &over {
                if let Some(w) = &moving {
                    screen::park(w, true);
                }
                if let Some(w) = app.get_webview_window(label) {
                    screen::raise(&w);
                }
            }
            a.over = over;
            a.last_x = None;
        }
        match bar {
            Some(bar) => {
                let x = ((at.0 - bar.left) / bar.scale).round();
                if a.last_x != Some(x) {
                    a.last_x = Some(x);
                    let hover = Hover {
                        x,
                        grab: a.grab_x,
                        root: a.root.clone(),
                        name: a.name.clone(),
                    };
                    let _ = app.emit_to(bar.label.as_str(), HOVER_EVENT, hover);
                }
            }
            None => {
                if let Some(w) = &moving {
                    screen::place(w, follow_at(at, a.offset));
                    if back {
                        screen::park(w, false);
                    }
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
    let Some(at) = screen::cursor(&app) else {
        return false;
    };
    let source = window.label().to_string();
    let only = app.state::<Windows>().tab_count(&source) <= 1;
    let (Some(outer), Some(inner), Some(k)) = (
        screen::outer(&window),
        screen::inner(&window),
        screen::per_logical(&window),
    ) else {
        return false;
    };
    let (moving, created, offset, home) = if only {
        (
            source.clone(),
            false,
            (at.0 - outer.0, at.1 - outer.1),
            Some((outer.0, outer.1)),
        )
    } else {
        // The tab sits in the new window's bar where it was grabbed, under the pointer.
        let offset = (
            grab_x * k + inner.0 - outer.0,
            grab_y * k + inner.1 - outer.1,
        );
        let corner = follow_at(at, offset);
        let opened = windows::open_drag_window(
            &app,
            root.clone(),
            (inner.2 / k, inner.3 / k),
            (corner.0 / k, corner.1 / k),
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
        grab_x,
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
            screen::place(&w, home);
            screen::park(&w, false);
        }
    }
    let _ = app.emit_to(a.source.as_str(), CANCELLED_EVENT, ());
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_under_the_pointer_takes_the_tab() {
        // Windows 100 tall from y = 100, their tab bars ending at y = 160.
        let bar = |label: &str, x: f64| Bar {
            label: label.into(),
            x,
            y: 100.0,
            width: 100.0,
            height: 100.0,
            left: x,
            bar_bottom: 160.0,
            scale: 1.0,
        };
        // `main` at 0–100, `window-2` at 200–300, `window-3` in front overlapping it at 250–350.
        let bars = [
            bar("window-3", 250.0),
            bar("main", 0.0),
            bar("window-2", 200.0),
        ];
        let at = |x: f64, y: f64| {
            bar_under(&bars, "window-9", "window-8", x, y).map(|b| b.label.as_str())
        };
        assert_eq!(at(50.0, 120.0), Some("main"));
        assert_eq!(at(220.0, 120.0), Some("window-2"));
        // Anywhere over a window, below its tab bar too.
        assert_eq!(at(50.0, 190.0), Some("main"));
        // Where two overlap, the window in front.
        assert_eq!(at(260.0, 120.0), Some("window-3"));
        // Between and below windows: none.
        assert_eq!(at(150.0, 120.0), None);
        assert_eq!(at(50.0, 210.0), None);
        // The moving window never counts.
        assert_eq!(bar_under(&bars, "main", "window-8", 50.0, 120.0), None);
        // The window it came from takes it back only over its tab bar.
        let home = |y: f64| bar_under(&bars, "window-9", "main", 50.0, y).map(|b| b.label.as_str());
        assert_eq!(home(120.0), Some("main"));
        assert_eq!(home(190.0), None);
    }

    #[test]
    fn the_moving_window_keeps_the_tab_under_the_pointer() {
        assert_eq!(follow_at((500.0, 300.0), (60.0, 40.0)), (440.0, 260.0));
    }
}
