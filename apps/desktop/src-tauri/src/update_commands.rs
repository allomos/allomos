//! App update commands (Task-A07-16, Task-A07-17, Task-A07-fact210, Task-A07-meter647,
//! Task-A07-trunk220).

use allomos_core::Problem;
use tauri::{AppHandle, Emitter};

use crate::{monitor_pause, opener, update};

const UPDATE_READY_EVENT: &str = "update:ready";
const UPDATE_FAILED_EVENT: &str = "update:failed";

/// Where a newer Allomos is downloaded (owner, 2026-10-12: the GitHub releases page, even
/// while it is private). A fixed address: the window cannot ask to open any other.
const DOWNLOAD_URL: &str = "https://github.com/allomos/allomos/releases";

/// Open the download page in the web browser, for a taskbag that needs a newer Allomos.
#[tauri::command]
pub(crate) fn open_download_page() -> Result<(), Problem> {
    opener()
        .arg(DOWNLOAD_URL)
        .spawn()
        .map(drop)
        .map_err(|e| Problem::OpenFailed {
            path: DOWNLOAD_URL.to_string(),
            detail: e.to_string(),
        })
}

/// Whether a newer Allomos is on GitHub (Task-A07-16). `asked` when the person chose
/// **Check for Updates…**: GitHub is asked now, and a skipped version is offered again.
/// Otherwise GitHub is asked only when the last answer is a day old.
///
/// When a newer version is known, its installer is downloaded and checked in the background
/// (Task-A07-17); when the download has ended, kept or not, every window hears `update:ready`
/// and asks again. While it downloads, the menu's **Download Allomos <version>…** waits
/// (A07-glove300); once it has failed, that item opens the release page.
///
/// With **Check for updates automatically** off (Task-A07-meter647), GitHub is asked only from
/// the menu. With **Download updates automatically** off (Task-A07-trunk220), a check does not
/// download in the background; the menu's **Check for Updates…** still does, because the person asked.
#[tauri::command]
pub(crate) async fn update_status(app: AppHandle, asked: bool) -> update::UpdateStatus {
    // What the check remembers before it runs: put back if the person cancels its download
    // (Task-A07-fact210).
    let before = allomos_store::update_memory();
    let mut status = update::check(asked, allomos_store::auto_check());
    if update::should_download(
        asked,
        allomos_store::auto_download(),
        status.available.as_deref(),
        status.ready.as_deref(),
    ) {
        if let Some(version) = status.available.clone() {
            if update::prepare_in_background(version.clone(), before, move || {
                let _ = app.emit(UPDATE_READY_EVENT, ());
            }) {
                status.preparing = Some(version);
            }
        }
    }
    status
}

/// How much of the newer Allomos's installer is downloaded, in percent, while it downloads
/// (Task-A07-fact210): the banner and the menu item show it. `None` when it is not known.
#[tauri::command]
pub(crate) fn update_progress() -> Option<u8> {
    update::download_percent()
}

/// **Cancel** on the update notice while the installer downloads (Task-A07-fact210): the download
/// stops, the check that found it is undone, and every window looks again. `false` when nothing
/// was downloading.
#[tauri::command]
pub(crate) fn cancel_update_download() -> bool {
    update::cancel_download()
}

/// **Settings… › General › Check for updates automatically** on or off (Task-A07-meter647). Every
/// window looks at updates again at once (the `update:ready` event), so the change shows without
/// waiting for the hourly look: turned off, a notice goes away; turned on, a day-old check runs now.
#[tauri::command]
pub(crate) fn set_auto_check(app: AppHandle, on: bool) -> Result<(), Problem> {
    allomos_store::save_auto_check(on).map_err(|e| Problem::WriteFailed {
        path: "settings.json".into(),
        detail: e.to_string(),
    })?;
    let _ = app.emit(UPDATE_READY_EVENT, ());
    Ok(())
}

/// **Settings… › General › Download updates automatically** on or off (Task-A07-trunk220). The
/// window looks again at once (the `update:ready` event): turned on, a newer version the last
/// check found is downloaded now.
#[tauri::command]
pub(crate) fn set_auto_download(app: AppHandle, on: bool) -> Result<(), Problem> {
    allomos_store::save_auto_download(on).map_err(|e| Problem::WriteFailed {
        path: "settings.json".into(),
        detail: e.to_string(),
    })?;
    let _ = app.emit(UPDATE_READY_EVENT, ());
    Ok(())
}

/// **Restart to Update to Allomos <version>** (Task-A07-17): run the checked installer and
/// quit; the new version starts by itself. If the person cancels macOS's password prompt,
/// nothing changes. A failure is told to every window, and this version keeps running.
pub(crate) fn restart_to_update(app: AppHandle) {
    tauri::async_runtime::spawn_blocking(move || {
        // Task-A03-bowl641: while monitoring runs, say first that it keeps running; Cancel changes nothing.
        if let Some(sessions) = monitor_pause::running_sessions() {
            if !monitor_pause::confirm_blocking(&app, monitor_pause::Action::Restart, sessions) {
                return;
            }
        }
        match update::restart_to_update() {
            Ok(update::Restart::Quit) => app.exit(0),
            Ok(update::Restart::Cancelled) => {}
            Err(detail) => {
                let _ = app.emit(UPDATE_FAILED_EVENT, Problem::UpdateFailed { detail });
            }
        }
    });
}

/// **Restart to Update** on the update notice: the same as the menu item, so the two never offer
/// different steps.
#[tauri::command]
pub(crate) fn restart_to_update_now(app: AppHandle) {
    restart_to_update(app);
}

/// "Skip this version" on the update notice.
#[tauri::command]
pub(crate) fn skip_update(version: String) {
    let _ = allomos_store::skip_update(&version);
}
