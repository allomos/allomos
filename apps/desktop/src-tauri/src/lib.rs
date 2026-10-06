//! FlipHopper desktop app: the Tauri window plus the commands the interface may call.
//!
//! The interface never calls these directly; it goes through its messenger module
//! (`apps/web/src/messenger.ts`), so the same interface can later reach the desktop's
//! local server from a browser without changes.

use std::path::{Path, PathBuf};
use std::process::Command;

use fliphopper_core::{Loaded, Problem};
use fliphopper_store::{Inspection, create_starter, git_root, inspect, load, stamp};
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

/// A fingerprint of the seabag's files. The window asks for it about once a second and
/// reloads when it changes, so changes made by `fh` show without reopening the folder.
/// Sent as text: a 64-bit number does not fit a JavaScript number.
#[tauri::command]
fn seabag_stamp(root: PathBuf) -> String {
    stamp(&root).to_string()
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

/// The system program that shows a folder: Finder on macOS, Explorer on Windows, the
/// desktop's file manager elsewhere. Only a seabag folder is accepted, so the window cannot
/// be used to open any other path.
fn file_manager_command(root: &Path) -> Result<Command, Problem> {
    if inspect(root)
        != (Inspection::Found {
            root: root.to_path_buf(),
        })
    {
        return Err(Problem::NotASeabag {
            path: root.display().to_string(),
        });
    }
    let program = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    };
    let mut command = Command::new(program);
    command.arg(root);
    Ok(command)
}

/// Show the open seabag's folder in Finder or Explorer.
#[tauri::command]
fn show_in_file_manager(root: PathBuf) -> Result<(), Problem> {
    let mut command = file_manager_command(&root)?;
    // Not waited for: Explorer reports failure even when it opened the folder.
    command.spawn().map(drop).map_err(|e| Problem::OpenFailed {
        path: root.display().to_string(),
        detail: e.to_string(),
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
            seabag_stamp,
            create_seabag,
            show_in_file_manager,
            system_languages
        ])
        .run(tauri::generate_context!())
        .expect("error while running FlipHopper");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_manager_opens_only_a_seabag() {
        let dir = std::env::temp_dir().join(format!("fliphopper-reveal-{}", std::process::id()));
        let root = dir.join(fliphopper_store::DEFAULT_DIR);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // The project folder itself, a missing path and a plain file are refused.
        for path in [dir.clone(), dir.join("missing"), root.clone()] {
            assert!(matches!(
                file_manager_command(&path),
                Err(Problem::NotASeabag { .. })
            ));
        }
        fliphopper_store::init(&root, "p", "P").unwrap();
        let command = file_manager_command(&root).unwrap();
        assert_eq!(command.get_args().collect::<Vec<_>>(), [root.as_os_str()]);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
