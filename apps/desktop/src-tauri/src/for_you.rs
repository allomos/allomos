//! The For You window (Task-A01-oak609, decision 0028): one window of its own, beside the project
//! windows, listing what needs the person in the order of `alm human` (the core's queue), with
//! the chosen task's pane on the right. It opens from a project window's For You button, or from
//! View → For You, and shows the project in front in that window; opened from a start page, it
//! keeps the project it showed before. Which project it follows as windows and tabs change
//! afterwards is a later task.

use std::path::PathBuf;
use std::sync::Mutex;

use allomos_core::Task;
use allomos_core::queue::{Asked, Row};
use allomos_store::load;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::windows::Windows;

/// The window's label. Not a project window: it reports no tabs and menu commands skip it.
pub(crate) const WINDOW: &str = "for-you";
/// View → For You.
pub(crate) const MENU_ID: &str = "for_you";
/// The menu item's shortcut (⇧⌘Y / Ctrl+Shift+Y).
pub(crate) const SHORTCUT: &str = "CmdOrCtrl+Shift+Y";
/// Tells the open window which project it shows now.
const PROJECT_EVENT: &str = "for_you:project";

/// The project the window shows: its taskbag, and its name for the title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Project {
    pub(crate) root: PathBuf,
    pub(crate) name: Option<String>,
}

/// The project the For You window shows; none until it was opened once.
#[derive(Default)]
pub(crate) struct ForYou {
    project: Mutex<Option<Project>>,
}

/// The project to show when the window is asked for: the one in front in the window it was asked
/// from (`front`), else the one it showed before (`kept`): a start page in front changes nothing.
pub(crate) fn following(front: Option<Project>, kept: Option<Project>) -> Option<Project> {
    front.or(kept)
}

/// Open the For You window, or bring it forward, on the project in front in window `from`. Asked
/// for while the For You window itself is in front, it stays as it is (decision 0028 § 3: working
/// in it never changes its project). With no project to show, nothing opens.
pub(crate) fn open(app: &AppHandle, from: Option<&str>) -> tauri::Result<()> {
    let existing = app.get_webview_window(WINDOW);
    if existing
        .as_ref()
        .is_some_and(|w| w.is_focused().unwrap_or(false))
    {
        return Ok(());
    }
    let front = from.and_then(|label| app.state::<Windows>().front_project(label));
    let project = {
        let state = app.state::<ForYou>();
        let mut kept = state.project.lock().unwrap_or_else(|e| e.into_inner());
        *kept = following(front, kept.take());
        kept.clone()
    };
    let Some(project) = project else {
        return Ok(());
    };
    match existing {
        Some(window) => {
            let _ = app.emit_to(WINDOW, PROJECT_EVENT, &project);
            let _ = window.unminimize();
            window.show()?;
            window.set_focus()
        }
        // The page asks for its project when it has loaded (`for_you_project`) and names the
        // window in the interface's language (`set_for_you_title`).
        None => tauri::WebviewWindowBuilder::new(
            app,
            WINDOW,
            tauri::WebviewUrl::App("index.html?for-you".into()),
        )
        .title("For You")
        .inner_size(1040.0, 700.0)
        .min_inner_size(640.0, 400.0)
        .build()
        .map(drop),
    }
}

/// The For You button of a project window: the queue of the project in front in that window.
#[tauri::command]
pub(crate) fn open_for_you(app: AppHandle, window: tauri::WebviewWindow) {
    let _ = open(&app, Some(window.label()));
}

/// The project the For You window shows, for the window itself when it loads.
#[tauri::command]
pub(crate) fn for_you_project(state: tauri::State<ForYou>) -> Option<Project> {
    state
        .project
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

/// The For You window names itself, in the interface's language: "For You · <project>". Any
/// other window's title is not changed here.
#[tauri::command]
pub(crate) fn set_for_you_title(window: tauri::WebviewWindow, title: String) {
    if window.label() == WINDOW {
        let _ = window.set_title(&title);
    }
}

/// A plan waiting for a Go ahead (`alm confirm --task`), as the window knows it: the task, and
/// when it was asked (UTC, as events write it).
#[derive(Debug, Deserialize)]
pub(crate) struct AskedNow {
    task: String,
    at: String,
}

/// What needs the person in the taskbag at `root`, one row per task, in the order of decision
/// 0028: the same order `alm human` lists (`allomos_core::queue`). `asked`: the plans waiting for
/// a Go ahead right now, which only the app knows. A taskbag that cannot be read has no rows.
#[tauri::command(async)]
pub(crate) fn human_queue(root: PathBuf, asked: Vec<AskedNow>) -> Vec<Row> {
    let Some(bag) = load(&root).taskbag else {
        return Vec::new();
    };
    let tasks: Vec<&Task> = bag.tasks.iter().collect();
    let asked: Vec<Asked> = asked
        .into_iter()
        .map(|a| Asked {
            task: a.task,
            at: a.at,
        })
        .collect();
    allomos_store::queue::read(&root, &bag).rows(&bag, &tasks, &asked)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(root: &str) -> Project {
        Project {
            root: PathBuf::from(root),
            name: Some(root.trim_start_matches('/').to_string()),
        }
    }

    #[test]
    fn it_shows_the_project_in_front_where_it_was_asked() {
        assert_eq!(
            following(Some(project("/a")), Some(project("/b"))),
            Some(project("/a"))
        );
        assert_eq!(following(Some(project("/a")), None), Some(project("/a")));
    }

    #[test]
    fn a_start_page_in_front_keeps_the_project_it_showed() {
        assert_eq!(following(None, Some(project("/b"))), Some(project("/b")));
        assert_eq!(following(None, None), None);
    }
}
