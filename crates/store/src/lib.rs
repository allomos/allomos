//! Reads and creates taskbags on disk.
//!
//! A *taskbag* is a folder holding `project.json`, `areas/<id>.json`, optionally
//! `iterations/<id>.json`, and `tasks/<id>/task.json` (+ `description.md`). See `docs/product/FILE-FORMAT.md`.
//! This crate never runs or reads Git (decision 0017): a taskbag inside a repository is
//! versioned by that repository, and the agent commits each change itself. [`with_written_files`]
//! tells which files a change wrote, so `alm` can print the Git commands for exactly those.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use fs4::{FileExt, TryLockError};

use allomos_core::branch;
use allomos_core::decision;
use allomos_core::event::{self, Event, Kind};
use allomos_core::heartbeat;
use allomos_core::ops;
use allomos_core::{
    Area, FORMAT, Issue, Iteration, Loaded, Problem, Project, State, Task, Taskbag, parse_area,
    parse_iteration, parse_project, parse_task, validate_taskbag,
};
use serde::Serialize;

mod agent_files;
pub mod approvals;
pub mod cadence;
pub mod cards;
pub mod checkpoint;
pub mod classify;
pub mod codex_rules;
mod confirm;
mod convert;
mod decisions;
pub mod effects;
pub mod events;
pub mod exceptions;
pub mod folders;
mod guide;
pub mod handoff;
mod holding;
pub mod identity;
pub mod links;
mod loading;
mod lock;
pub mod missing;
pub mod progress;
pub mod queue;
mod quota;
mod readme;
pub mod redact;
mod settings;
mod setup;
mod signing;
mod starter;
mod structure;
mod tasks;
pub mod trail;
mod translate;
pub mod tsa;
mod update_taskbag;
mod upgrade;
pub mod versions;
mod writing;
pub use agent_files::*;
pub use confirm::{
    ConfirmRequest, Extra as ConfirmExtra, MAX_PLAN_BYTES, REQUESTS_DIR, answer_confirm,
    pending_confirms, post_confirm, read_answer, requests_dir, shown_fingerprint, was_seen,
    withdraw_confirm,
};
pub use convert::{
    CONNECT_REQUEST_VERSION, CONVERT_REQUEST_VERSION, MOVE_IN_REQUEST_VERSION,
    agent_connect_request_in, agent_connect_request_with, agent_convert_request_in,
    agent_move_in_request_in, agent_move_tasks_in_request_in,
};
pub use decisions::*;
pub use events::Actor;
pub use guide::{
    GOAL_PROMPT_VERSION, GUIDE_TOPICS, LOCALIZED_TOPICS, RUN_MODES_LANGUAGES, added_task_steps,
    agent_files, claimed_task_steps, discuss_request_in, github_release, goal, goal_request_in,
    guide, guide_topic, localized, run_modes,
};
pub use holding::*;
pub use loading::*;
pub use lock::*;
pub use quota::{
    ASK_AFTER_MINUTES as QUOTA_ASK_AFTER_MINUTES, QuotaAsk, QuotaReport, WINDOWS as QUOTA_WINDOWS,
    agent_id as quota_agent_id, is_codex as quota_is_codex, new_report as new_quota_report,
    quota_ask, quota_reports, save_quota,
};
pub use readme::*;
pub use settings::{
    StartWith, TaskLanguage, UpdateMemory, alm_offer_dismissed_for, auto_check, auto_download,
    dismiss_alm_offer, lan_viewing, pick_ui_language, restore_update_check, save_auto_check,
    save_auto_download, save_lan_viewing, save_owner_name, save_start_with, save_ui_language,
    save_update_check, saved_owner_name, saved_ui_language, settings_dir, skip_update, start_with,
    task_language, update_memory,
};
pub use setup::{
    SetupState, allomos_folder, project_folder_of, project_folder_paths, record_project_folder,
    relative_path, setup_state,
};
pub use signing::{
    Replaced, ReplacedIn, ThisComputer, ThisKey, public_key_export, replace_key, this_computer,
};
pub use starter::*;
pub use structure::*;
pub use tasks::*;
pub use translate::{agent_translate_request_in, translation_steps_in};
pub use update_taskbag::{
    Target as TranslateTarget, agent_update_taskbag_request_in, agent_update_taskbag_request_with,
};
pub use upgrade::*;
pub use writing::*;

/// Where a project's taskbag lives inside the folder the user picked.
/// Always used for a new taskbag, also in an empty folder, so every tool looks in one place.
pub const DEFAULT_DIR: &str = ".allomos/taskbag";
const PROJECT_FILE: &str = "project.json";
/// The file every writer locks for the length of one change. Never committed.
pub const LOCK_FILE: &str = ".lock";
/// Prefix of the temporary files and folders a writer renames into place.
const TMP_PREFIX: &str = ".tmp-";
/// Written into every new taskbag, so Git never sees Allomos's working files.
const GITIGNORE: &str = "# Allomos's own working files. Never commit them.\n.lock\n.tmp-*\n";
/// Written into every new taskbag, so Git on Windows checks the files out with `\n` line endings,
/// as Allomos writes them (Task-A07-03). Reading accepts `\r\n` too.
const GITATTRIBUTES: &str = "# Allomos writes these files with \\n line endings; keep them so on every computer.\n*.json text eol=lf\n*.md text eol=lf\n.gitignore text eol=lf\n.gitattributes text eol=lf\n";

/// `<folder>/.allomos/taskbag`, built part by part so the path uses the system's own separator
/// (`C:\p\.allomos\taskbag` on Windows, not `C:\p\.allomos/taskbag`).
pub fn default_root(folder: &Path) -> PathBuf {
    DEFAULT_DIR
        .split('/')
        .fold(folder.to_path_buf(), |p, part| p.join(part))
}

/// `text` with every `\r\n` turned into `\n`: Git on Windows may check text files out with
/// `\r\n`, and Allomos always writes `\n` (Task-A07-03).
pub fn lf(text: &str) -> String {
    text.replace("\r\n", "\n")
}

/// Read a text file, with `\r\n` line endings turned into `\n`.
pub fn read_text(path: &Path) -> io::Result<String> {
    fs::read_to_string(path).map(|t| lf(&t))
}

/// A path as the system's own programs want it: on Windows with `\` throughout and without the
/// `\\?\` prefix that [`Path::canonicalize`] adds (Explorer does not understand it). Elsewhere
/// the path as it is.
pub fn plain_path(path: &Path) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(plain_windows(&path.to_string_lossy()))
    } else {
        path.to_path_buf()
    }
}

/// [`plain_path`]'s text rule, kept apart so it is tested on every system: `/` becomes `\`,
/// `\\?\C:\x` becomes `C:\x` and `\\?\UNC\server\share` becomes `\\server\share`.
fn plain_windows(path: &str) -> String {
    let path = path.replace('/', "\\");
    if let Some(rest) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = path.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        path
    }
}

/// A path written with `/` between its parts, for files shared between computers (a
/// relative path recorded on Windows as `..\app` is stored as `../app`).
pub fn portable_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// What a folder the user picked turns out to be.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Inspection {
    /// The folder is a taskbag, or contains one in `.allomos/taskbag/`.
    Found { root: PathBuf },
    /// The folder looks empty (nothing, or only hidden entries such as `.DS_Store` or `.git`):
    /// a starter taskbag can be created in its `.allomos/taskbag/` without asking.
    Empty {
        folder: PathBuf,
        suggested_root: PathBuf,
    },
    /// The folder has other content but no taskbag: one can be created in `.allomos/taskbag/`.
    NoTaskbag {
        folder: PathBuf,
        suggested_root: PathBuf,
    },
    /// The path does not exist or is not a folder.
    NotAFolder { path: PathBuf },
}

/// A fingerprint of the taskbag's files (names, sizes and change times). It changes when any
/// file is written, so a window can reload only when something changed.
pub fn stamp(root: &Path) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let mut add = |p: &Path| {
        if let Ok(m) = fs::metadata(p) {
            p.hash(&mut h);
            m.len().hash(&mut h);
            m.modified().ok().hash(&mut h);
        }
    };
    add(&root.join(PROJECT_FILE));
    add(&root.join(handoff::NOTES_FILE));
    // A new event changes its folder's time; reading every event file here would grow with history.
    add(&root.join(events::EVENTS_DIR));
    // An older taskbag's handoff files (Task-A03-polar770): the notice goes once they are moved.
    for folder in ["areas", "iterations", handoff::HANDOFFS_DIR] {
        for p in sorted_entries(&root.join(folder)) {
            add(&p);
        }
    }
    for dir in sorted_entries(&root.join("tasks")) {
        add(&dir);
        for p in sorted_entries(&dir) {
            add(&p);
        }
    }
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp(name: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!("allomos-test-{name}-{n}"));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn a_path_with_backslashes_finds_its_project() {
        let base = tmp("backslash");
        let home = base.join("tasks-home");
        let app = base.join("app");
        fs::create_dir_all(&app).unwrap();
        let root = default_root(&home);
        init(&root, "p", "P").unwrap();
        // Written on Windows by an older Allomos, or by hand: `\` between the parts.
        let mut bag = load(&root).taskbag.unwrap();
        bag.project.project_folder = Some(r"..\app".into());
        assert_eq!(project_folder_of(&root, &bag.project), Some(app.clone()));
        bag.project.project_folder = Some(r"..\app\".into());
        assert_eq!(project_folder_of(&root, &bag.project), Some(app.clone()));
        // Recorded with `/`, so the taskbag works on every computer.
        assert_eq!(record_project_folder(&root, &app).unwrap(), "../app");
        // The taskbag found from the project folder, and the project folder from the taskbag.
        assert_eq!(picked_folder(&root), Some(home.as_path()));
        assert_eq!(find_taskbag(&home.join("src")), Some(root.clone()));
        // An AGENTS.md beside a sibling taskbag names it with `/`.
        let section = agent_section_for(&app, &root);
        assert!(
            section.contains("`--taskbag \"../tasks-home/.allomos/taskbag\"`"),
            "{section}"
        );
        assert_eq!(portable_path(Path::new(r"..\a\b")), "../a/b");
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn windows_paths_are_given_to_explorer_plainly() {
        assert_eq!(
            plain_windows(r"\\?\C:\p\.allomos\taskbag"),
            r"C:\p\.allomos\taskbag"
        );
        assert_eq!(
            plain_windows(r"C:\p\.allomos/taskbag"),
            r"C:\p\.allomos\taskbag"
        );
        assert_eq!(
            plain_windows(r"\\?\UNC\server\share\p"),
            r"\\server\share\p"
        );
        assert_eq!(plain_windows(r"\\server\share\p"), r"\\server\share\p");
    }
}
