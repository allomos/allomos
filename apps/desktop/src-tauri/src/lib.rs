//! FlipHopper desktop app: the Tauri window plus the commands the interface may call.
//!
//! The interface never calls these directly; it goes through its messenger module
//! (`apps/web/src/messenger.ts`), so the same interface can later reach the desktop's
//! local server from a browser without changes.

use std::path::PathBuf;

use fliphopper_core::{Loaded, Problem};
use fliphopper_store::{Inspection, create_starter, git_root, inspect, load};
use serde::Serialize;

#[derive(Serialize)]
struct FolderInfo {
    inspection: Inspection,
    /// The Git repository the folder belongs to, if any (information only).
    git_root: Option<PathBuf>,
}

#[derive(Serialize)]
struct Opened {
    root: PathBuf,
    loaded: Loaded,
}

/// A folder given at start-up: the first command-line argument, or `FLIPHOPPER_OPEN`.
/// Lets `fliphopper ~/my-project` open that folder directly.
#[tauri::command]
fn startup_folder() -> Option<PathBuf> {
    std::env::var_os("FLIPHOPPER_OPEN")
        .map(PathBuf::from)
        .or_else(|| std::env::args_os().nth(1).map(PathBuf::from))
        .filter(|p| p.is_dir())
}

/// What is this folder? Changes nothing.
#[tauri::command]
fn inspect_folder(path: PathBuf) -> FolderInfo {
    FolderInfo {
        git_root: git_root(&path),
        inspection: inspect(&path),
    }
}

/// Load the seabag at `root`.
#[tauri::command]
fn open_seabag(root: PathBuf) -> Opened {
    Opened {
        loaded: load(&root),
        root,
    }
}

/// Create a starter seabag at `root` (normally `<folder>/.fliphopper/seabag/`), then load it.
#[tauri::command]
fn create_seabag(root: PathBuf, name: String) -> Result<Opened, Problem> {
    create_starter(&root, &name).map_err(|e| {
        let path = root.display().to_string();
        if e.kind() == std::io::ErrorKind::AlreadyExists {
            Problem::FolderNotEmpty { path }
        } else {
            Problem::CreateFailed {
                path,
                detail: e.to_string(),
            }
        }
    })?;
    Ok(Opened {
        loaded: load(&root),
        root,
    })
}

/// The computer's preferred languages, most preferred first (for example `["ja-JP", "en-JP"]`).
/// Read from the operating system: inside the macOS app window, `navigator.language` is
/// not a reliable guide to the system's language.
#[tauri::command]
fn system_languages() -> Vec<String> {
    sys_locale::get_locales().collect()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            startup_folder,
            inspect_folder,
            open_seabag,
            create_seabag,
            system_languages
        ])
        .run(tauri::generate_context!())
        .expect("error while running FlipHopper");
}
