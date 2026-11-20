//! Send back… with this screen attached (Task-A01-music456, decision 0030 § 4): the window the
//! person pressed it in is captured as a picture and kept in the task's `evidence/` folder in the
//! taskbag, counted with the failed checks' screenshots (at most 3, decision 0029 § 6). The page
//! then names it in the reopen reason. On macOS only (the system's `screencapture`); elsewhere it
//! is refused, saying so.

use std::path::{Path, PathBuf};

use allomos_core::Problem;

/// How long the store waits for the taskbag's lock.
const LOCK_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

/// The name the capture is given before it is kept (the kept name comes from it).
const SHOT_FILE: &str = "send-back.jpg";

/// A folder of its own for one capture, in the system's temporary folder.
fn capture_dir() -> PathBuf {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!("allomos-send-back-{}-{n}", std::process::id()))
}

fn failed(detail: impl ToString) -> Problem {
    Problem::ScreenCaptureFailed {
        detail: detail.to_string(),
    }
}

/// The window's number for the system (its CGWindowID), read on the main thread.
#[cfg(target_os = "macos")]
fn window_number(window: &tauri::WebviewWindow) -> Result<i64, Problem> {
    let (tx, rx) = std::sync::mpsc::channel();
    let w = window.clone();
    window
        .run_on_main_thread(move || {
            let number = w.ns_window().ok().map(|ns| {
                // SAFETY: `ns` is this window's live NSWindow, used on the main thread.
                unsafe {
                    let ns = &*(ns as *const objc2::runtime::AnyObject);
                    let n: isize = objc2::msg_send![ns, windowNumber];
                    n as i64
                }
            });
            let _ = tx.send(number);
        })
        .map_err(failed)?;
    rx.recv_timeout(std::time::Duration::from_secs(5))
        .ok()
        .flatten()
        .filter(|n| *n > 0)
        .ok_or_else(|| failed("the window has no number"))
}

/// Capture the window into `file` as a JPEG (without its shadow), made smaller when it is over
/// the screenshots' size limit.
#[cfg(target_os = "macos")]
fn capture(window: &tauri::WebviewWindow, file: &Path) -> Result<(), Problem> {
    use std::process::Command;
    let number = window_number(window)?;
    let out = Command::new("/usr/sbin/screencapture")
        .args(["-x", "-o", "-t", "jpg", &format!("-l{number}")])
        .arg(file)
        .output()
        .map_err(failed)?;
    let size = std::fs::metadata(file).map(|m| m.len()).unwrap_or(0);
    if !out.status.success() || size == 0 {
        let said = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(failed(if said.is_empty() {
            "the system made no picture".to_string()
        } else {
            said
        }));
    }
    if size > allomos_core::check_result::MOST_BYTES {
        let _ = Command::new("/usr/bin/sips")
            .args(["-Z", "1600", "-s", "formatOptions", "70"])
            .arg(file)
            .output();
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn capture(_window: &tauri::WebviewWindow, _file: &Path) -> Result<(), Problem> {
    Err(failed("capturing a window works only on macOS for now"))
}

/// Capture the window this was asked from and keep it in task `task`'s `evidence/` folder in the
/// taskbag at `taskbag`; answers the name it is kept under. Refused when the folder holds 3
/// already (`return_screenshots_full`) or the capture fails (`screen_capture_failed`).
#[tauri::command(async)]
pub fn capture_evidence_screen(
    window: tauri::WebviewWindow,
    taskbag: PathBuf,
    task: String,
) -> Result<String, Problem> {
    if !allomos_core::ops::valid_id(&task) {
        return Err(Problem::TaskNotFound { id: task });
    }
    // The count first: a full folder captures nothing.
    let kept = allomos_store::check_results::kept(&taskbag, &task);
    if kept.len() >= allomos_core::check_result::MOST_SCREENSHOTS {
        return Err(Problem::ReturnScreenshotsFull {
            id: task,
            have: kept.join(", "),
        });
    }
    let dir = capture_dir();
    std::fs::create_dir_all(&dir).map_err(failed)?;
    let file = dir.join(SHOT_FILE);
    let kept = capture(&window, &file).and_then(|()| {
        allomos_store::check_results::add_screenshot(&taskbag, LOCK_WAIT, &task, &file)
    });
    let _ = std::fs::remove_dir_all(&dir);
    kept
}
