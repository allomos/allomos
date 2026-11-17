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
pub mod effects;
pub mod events;
pub mod exceptions;
pub mod folders;
mod guide;
pub mod handoff;
pub mod identity;
pub mod links;
mod loading;
pub mod missing;
pub mod progress;
pub mod queue;
mod quota;
mod readme;
pub mod redact;
mod settings;
mod setup;
mod signing;
pub mod trail;
mod translate;
pub mod tsa;
mod update_taskbag;
pub mod versions;
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
pub use events::Actor;
pub use guide::{
    GOAL_PROMPT_VERSION, GUIDE_TOPICS, LOCALIZED_TOPICS, RUN_MODES_LANGUAGES, added_task_steps,
    agent_files, claimed_task_steps, discuss_request_in, github_release, goal, goal_request_in,
    guide, guide_topic, localized, run_modes,
};
pub use loading::*;
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
pub use translate::{agent_translate_request_in, translation_steps_in};
pub use update_taskbag::{
    Target as TranslateTarget, agent_update_taskbag_request_in, agent_update_taskbag_request_with,
};

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

pub(crate) fn write_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut text = serde_json::to_string_pretty(value).map_err(io::Error::other)?;
    text.push('\n');
    fs::write(path, text)
}

/// The JSON stored in `task.json`: the task without its description (that lives in
/// `description.md`) and without empty optional fields.
pub(crate) fn task_file_json(task: &Task) -> serde_json::Value {
    let mut v = serde_json::to_value(task).expect("a task always serializes");
    if let Some(map) = v.as_object_mut() {
        map.remove("description");
        map.retain(|_, value| !value.is_null());
    }
    v
}

/// Create a small starter project at `root`. `root` must not exist yet, or be an empty folder.
/// Returns `root`.
pub fn create_starter(root: &Path, name: &str) -> io::Result<PathBuf> {
    if root.exists() && fs::read_dir(root)?.next().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} is not empty", root.display()),
        ));
    }
    fs::create_dir_all(root)?;
    fs::write(root.join(".gitignore"), GITIGNORE)?;
    fs::write(root.join(".gitattributes"), GITATTRIBUTES)?;
    let project = Project {
        project_folders: Default::default(),
        extra: Default::default(),
        format: FORMAT.to_string(),
        id: "project".to_string(),
        name: name.to_string(),
        labels: allomos_core::default_labels(),
        project_folder: None,
        task_language: None,
        handoff: None,
        audit: None,
        heartbeat: None,
        risk: None,
    };
    write_json(&root.join(PROJECT_FILE), &project)?;
    let areas = [("A01", "Getting started", 1), ("A02", "First feature", 2)];
    for (id, name, order) in areas {
        write_json(
            &root.join("areas").join(format!("{id}.json")),
            &Area {
                extra: Default::default(),
                id: id.into(),
                name: name.into(),
                order,
            },
        )?;
    }
    let tasks = [
        (
            "A01-01",
            "Open this taskbag in Allomos",
            "A01",
            allomos_core::State::Done,
            Some("human verified"),
            "You picked a folder and Allomos created this starter project in it.\n",
        ),
        (
            "A01-02",
            "Rename the areas and tasks to your project's",
            "A01",
            allomos_core::State::Ready,
            None,
            "Edit the files in this folder, or use the `alm` command once it is available.\n",
        ),
        (
            "A02-01",
            "Describe the first outcome you want",
            "A02",
            allomos_core::State::Backlog,
            None,
            "One sentence about what a person can do when this task is done.\n",
        ),
    ];
    for (id, title, area, state, label, description) in tasks {
        let dir = root.join("tasks").join(id);
        let task = Task {
            folders: Default::default(),
            session: None,
            heartbeat: None,
            handover_request: None,
            extra: Default::default(),
            id: id.into(),
            title: title.into(),
            area: area.into(),
            state,
            labels: label.map(String::from).into_iter().collect(),
            holder: None,
            waiting_on: None,
            who: None,
            blocked_by: Vec::new(),
            next_dependency: None,
            iteration: None,
            difficulty: None,
            handoff_note: None,
            without_human_check: None,
            human_check_steps: None,
            description: String::new(),
        };
        write_json(&dir.join("task.json"), &task_file_json(&task))?;
        fs::write(dir.join("description.md"), description)?;
    }
    Ok(root.to_path_buf())
}

/// Find the taskbag for `start`: the nearest `.allomos/taskbag/` at or above it,
/// the way Git finds its repository.
pub fn find_taskbag(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .map(default_root)
        .find(|p| p.join(PROJECT_FILE).is_file())
}

/// A project ID made from a folder name: letters, digits, `-` and `_` kept, anything else
/// turned into `-`, at most 64 characters; `project` when nothing usable is left.
pub fn id_from_name(name: &str) -> String {
    let id: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .take(64)
        .collect();
    let id = id.trim_matches('-').to_string();
    if id.is_empty() {
        "project".to_string()
    } else {
        id
    }
}

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

thread_local! {
    /// The files written or removed by the change running on this thread, while
    /// [`with_written_files`] is listening.
    static WRITTEN: RefCell<Option<BTreeSet<PathBuf>>> = const { RefCell::new(None) };
}

/// Note that `path` was written or removed, when someone is listening.
pub(crate) fn note_written(path: &Path) {
    WRITTEN.with(|w| {
        if let Some(set) = w.borrow_mut().as_mut() {
            set.insert(path.to_path_buf());
        }
    });
}

/// Whether [`with_written_files`] is listening on this thread.
fn listening() -> bool {
    WRITTEN.with(|w| w.borrow().is_some())
}

/// Run `f`, and return with its result every file it wrote or removed through this crate
/// (decision 0017): a file rewritten with the same text is not counted. `alm` prints the Git
/// commands for exactly these files; nothing here runs Git.
pub fn with_written_files<T>(f: impl FnOnce() -> T) -> (T, Vec<PathBuf>) {
    let outer = WRITTEN.with(|w| w.borrow_mut().replace(BTreeSet::new()));
    let out = f();
    let mine = WRITTEN.with(|w| {
        let mut slot = w.borrow_mut();
        let mine = slot.take().unwrap_or_default();
        // Listening inside another listener: the outer one sees these files too.
        *slot = outer.map(|mut o| {
            o.extend(mine.iter().cloned());
            o
        });
        mine
    });
    (out, mine.into_iter().collect())
}

/// A name for a temporary file or folder next to `dir`'s contents, unique to this process.
fn tmp_path(dir: &Path, what: &str) -> PathBuf {
    let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    dir.join(format!("{TMP_PREFIX}{}-{n}-{what}", std::process::id()))
}

pub(crate) fn write_failed(path: &Path, e: impl std::fmt::Display) -> Problem {
    Problem::WriteFailed {
        path: path.display().to_string(),
        detail: e.to_string(),
    }
}

pub(crate) fn json_text<T: Serialize>(value: &T) -> String {
    let mut text = serde_json::to_string_pretty(value).expect("taskbag values always serialize");
    text.push('\n');
    text
}

/// How long replacing a file keeps trying while another program has it open (Windows).
const REPLACE_RETRY: Duration = Duration::from_millis(1000);
const REPLACE_PAUSE: Duration = Duration::from_millis(50);

/// Whether `e` may be a file that another program has open, which Windows reports as "access
/// denied" (5) or a sharing or lock violation (32, 33) until that program lets go.
fn may_be_in_use(e: &io::Error) -> bool {
    e.kind() == io::ErrorKind::PermissionDenied
        || (cfg!(windows) && matches!(e.raw_os_error(), Some(5 | 32 | 33)))
}

/// Run `attempt` until it works, a failure is not [`may_be_in_use`], or `limit` has passed,
/// pausing `pause` in between. A writer holds the taskbag's lock meanwhile, so the wait stays
/// well within the 5 seconds another writer waits for the lock.
fn retry_in_use<T>(
    limit: Duration,
    pause: Duration,
    mut attempt: impl FnMut() -> io::Result<T>,
) -> io::Result<T> {
    let start = Instant::now();
    loop {
        match attempt() {
            Err(e) if may_be_in_use(&e) && start.elapsed() < limit => std::thread::sleep(pause),
            result => return result,
        }
    }
}

/// Rename `from` over `to`. On Windows this fails while another program (an editor, a virus
/// scanner, Git) has `to` open without sharing it, so it is tried again for up to a second.
pub(crate) fn replace_file(from: &Path, to: &Path) -> io::Result<()> {
    retry_in_use(REPLACE_RETRY, REPLACE_PAUSE, || fs::rename(from, to))
}

/// Write `path` whole: the text goes to a temporary file in the same folder, is flushed to
/// disk, and is then renamed over `path`. A reader sees either the old file or the new one,
/// never a part-written one. Line endings are always written as `\n`.
pub(crate) fn write_atomic(path: &Path, text: &str) -> Result<(), Problem> {
    if listening() && fs::read_to_string(path).is_ok_and(|old| old == lf(text)) {
        // The same text again: the file is rewritten, but nothing changed to commit.
    } else {
        note_written(path);
    }
    let dir = path.parent().expect("taskbag files are inside a folder");
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let tmp = tmp_path(dir, name);
    let result = (|| {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(lf(text).as_bytes())?;
        f.sync_all()?;
        drop(f);
        replace_file(&tmp, path)
    })();
    result.map_err(|e| {
        let _ = fs::remove_file(&tmp);
        write_failed(path, e)
    })
}

/// Fill a new folder with files, then rename it to `target` in one step, so a reader never
/// sees it half-made. Refused when `target` already exists.
pub(crate) fn create_dir_atomic(target: &Path, files: &[(&str, String)]) -> Result<(), Problem> {
    let parent = target.parent().expect("a target has a parent");
    let name = target.file_name().and_then(|n| n.to_str()).unwrap_or("dir");
    let tmp = tmp_path(parent, name);
    let result = (|| {
        fs::create_dir_all(&tmp)?;
        for (rel_path, text) in files {
            let path = tmp.join(rel_path);
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir)?;
            }
            let mut f = fs::File::create(&path)?;
            f.write_all(lf(text).as_bytes())?;
            f.sync_all()?;
        }
        if target.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "already exists",
            ));
        }
        fs::rename(&tmp, target)
    })();
    if result.is_ok() {
        for (rel_path, _) in files {
            note_written(&target.join(rel_path));
        }
    }
    result.map_err(|e| {
        let _ = fs::remove_dir_all(&tmp);
        write_failed(target, e)
    })
}

/// Create an empty taskbag at `root` (normally `<repository>/.allomos/taskbag/`): `project.json`,
/// empty `areas/` and `tasks/`, a `.gitignore` for the lock and a `.gitattributes` keeping `\n`
/// line endings. Refused when `root` exists.
/// Never creates a Git repository.
/// This is how the app creates one, so its first event says `via: app`.
pub fn init(root: &Path, id: &str, name: &str) -> Result<PathBuf, Problem> {
    init_with_labels(root, id, name, Vec::new(), Actor::APP)
}

/// [`init`] with the project's own labels besides the built-in ones (decision 0020), made by
/// `actor`. The taskbag's first event records its creation, which is when its history starts.
pub fn init_with_labels(
    root: &Path,
    id: &str,
    name: &str,
    labels: Vec<String>,
    actor: Actor,
) -> Result<PathBuf, Problem> {
    if root.exists() {
        return Err(Problem::TaskbagExists {
            path: root.display().to_string(),
        });
    }
    let project = Project {
        project_folders: Default::default(),
        extra: Default::default(),
        format: FORMAT.to_string(),
        id: id.to_string(),
        name: name.to_string(),
        labels: allomos_core::labels::with_built_ins(
            labels
                .iter()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .map(|l| allomos_core::LabelEntry::custom(l, None))
                .collect(),
        ),
        project_folder: None,
        // New taskbags follow the app's language until someone sets another (Task-A01-21).
        task_language: Some(allomos_core::TASK_LANGUAGE_UI.to_string()),
        handoff: None,
        audit: None,
        heartbeat: None,
        risk: None,
    };
    // Checked like any project file, so `init` never writes one that `load` refuses.
    let text = json_text(&project);
    if let Err(issues) = parse_project(PROJECT_FILE, &text) {
        return Err(issues[0].problem.clone());
    }
    let first = actor.event(
        "init",
        Kind::Project,
        Some(id),
        event::changes(None, &project, &[]),
    );
    let parent = root.parent().expect("a taskbag root has a parent");
    fs::create_dir_all(parent).map_err(|e| write_failed(parent, e))?;
    let first_file = events::new_file(root);
    create_dir_atomic(
        root,
        &[
            (PROJECT_FILE, text),
            (".gitignore", GITIGNORE.to_string()),
            (".gitattributes", GITATTRIBUTES.to_string()),
            ("areas/.keep", String::new()),
            ("tasks/.keep", String::new()),
            (handoff::NOTES_FILE, handoff::DEFAULT_NOTES.to_string()),
            (&first_file, json_text(&first)),
        ],
    )
    .map_err(|p| match p {
        Problem::WriteFailed { .. } if root.exists() => Problem::TaskbagExists {
            path: root.display().to_string(),
        },
        p => p,
    })?;
    // An `allomos/8` taskbag's history is signed from its first event (Task-A01-gusto404).
    if signing::wanted(root, &first) {
        let mut value = serde_json::to_value(&first).expect("events serialize");
        let new_key = signing::sign_event(root, &mut value, &first_file);
        let mut first = first;
        if let Some(sig) = value.get("sig") {
            first.extra.insert("sig".into(), sig.clone());
        }
        write_atomic(&root.join(&first_file), &json_text(&first))?;
        if let Some(key) = new_key {
            signing::add_key(root, Some(id), &key, &value)?;
        }
    }
    Ok(root.to_path_buf())
}

/// The taskbag's lock, held for one change: load, check the rules, write, then drop it.
/// Every write goes through a `Lock`, so two Allomos writers never change the taskbag at
/// the same moment. The operating system releases the lock if the process dies.
pub struct Lock {
    root: PathBuf,
    _file: fs::File,
}

impl Lock {
    /// Pack the history of whole earlier months from the 28th of a month (Task-A01-59):
    /// [`events::pack`].
    pub fn pack_history(&self) -> Result<Vec<events::Packed>, Problem> {
        events::pack(&self.root)
    }

    /// Take the lock, waiting up to `wait` for another writer to finish.
    pub fn acquire(root: &Path, wait: Duration) -> Result<Lock, Problem> {
        if !root.join(PROJECT_FILE).is_file() {
            return Err(Problem::TaskbagNotFound {
                path: root.display().to_string(),
            });
        }
        let path = root.join(LOCK_FILE);
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .map_err(|e| write_failed(&path, e))?;
        let start = Instant::now();
        loop {
            match FileExt::try_lock(&file) {
                Ok(()) => {
                    return Ok(Lock {
                        root: root.to_path_buf(),
                        _file: file,
                    });
                }
                Err(TryLockError::WouldBlock) if start.elapsed() < wait => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(TryLockError::WouldBlock) => {
                    return Err(Problem::TaskbagBusy {
                        path: root.display().to_string(),
                    });
                }
                Err(TryLockError::Error(e)) => return Err(write_failed(&path, e)),
            }
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Load the taskbag while holding the lock, so what is read is what the change builds on.
    pub fn load(&self) -> Loaded {
        load(&self.root)
    }

    /// Write a new area file. Refused when the file already exists.
    pub fn create_area(&self, area: &Area) -> Result<(), Problem> {
        let dir = self.root.join("areas");
        fs::create_dir_all(&dir).map_err(|e| write_failed(&dir, e))?;
        let path = dir.join(format!("{}.json", area.id));
        if path.exists() {
            return Err(Problem::DuplicateAreaId {
                id: area.id.clone(),
            });
        }
        write_atomic(&path, &json_text(area))
    }

    /// Write a new iteration file. Refused when the file already exists.
    pub fn create_iteration(&self, iteration: &Iteration) -> Result<(), Problem> {
        let dir = self.root.join("iterations");
        fs::create_dir_all(&dir).map_err(|e| write_failed(&dir, e))?;
        let path = dir.join(format!("{}.json", iteration.id));
        if path.exists() {
            return Err(Problem::DuplicateIterationId {
                id: iteration.id.clone(),
            });
        }
        write_atomic(&path, &json_text(iteration))
    }

    /// Write a new task folder: `task.json`, plus `description.md` when there is a description.
    pub fn create_task(&self, task: &Task) -> Result<(), Problem> {
        let dir = self.root.join("tasks");
        fs::create_dir_all(&dir).map_err(|e| write_failed(&dir, e))?;
        let target = dir.join(&task.id);
        if target.exists() {
            return Err(Problem::DuplicateTaskId {
                id: task.id.clone(),
            });
        }
        let mut files = vec![("task.json", json_text(&task_file_json(task)))];
        if !task.description.is_empty() {
            files.push(("description.md", task.description.clone()));
        }
        create_dir_atomic(&target, &files)
    }

    /// Rewrite an existing area's file (`alm rename area`, Task-A01-29).
    pub fn save_area(&self, area: &Area) -> Result<(), Problem> {
        let path = self.root.join("areas").join(format!("{}.json", area.id));
        write_atomic(&path, &json_text(area))
    }

    /// Rewrite an existing iteration's file (`alm change iteration`, Task-A01-29).
    pub fn save_iteration(&self, iteration: &Iteration) -> Result<(), Problem> {
        let path = self
            .root
            .join("iterations")
            .join(format!("{}.json", iteration.id));
        write_atomic(&path, &json_text(iteration))
    }

    /// Remove an area's or an iteration's file (`alm delete area|iteration`, Task-A01-29).
    /// `folder` is `areas` or `iterations`.
    pub fn remove_item(&self, folder: &str, id: &str) -> Result<(), Problem> {
        let path = self.root.join(folder).join(format!("{id}.json"));
        fs::remove_file(&path).map_err(|e| write_failed(&path, e))?;
        note_written(&path);
        Ok(())
    }

    /// Rewrite `project.json` (`alm labels`).
    pub fn save_project(&self, project: &Project) -> Result<(), Problem> {
        write_atomic(&self.root.join(PROJECT_FILE), &json_text(project))
    }

    /// Rewrite an existing task's `task.json`. Its description is left as it is.
    pub fn save_task(&self, task: &Task) -> Result<(), Problem> {
        let path = self.root.join("tasks").join(&task.id).join("task.json");
        write_atomic(&path, &json_text(&task_file_json(task)))
    }

    /// Rewrite an existing task's `description.md`, or remove it when the description is empty
    /// (the app's task pane, Task-A01-18).
    pub fn save_description(&self, task: &Task) -> Result<(), Problem> {
        let path = self
            .root
            .join("tasks")
            .join(&task.id)
            .join("description.md");
        if task.description.is_empty() {
            match fs::remove_file(&path) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => Err(write_failed(&path, e)),
                Err(_) => Ok(()),
                Ok(()) => {
                    note_written(&path);
                    Ok(())
                }
            }
        } else {
            write_atomic(&path, &task.description)
        }
    }

    /// Make one change and record it (Task-A01-17): write `event`, then run `write`. When `write`
    /// fails the event is taken back, so the history never shows a change that did not happen.
    /// An event with no changes is not written: nothing happened worth recording.
    pub fn record(
        &self,
        event: &Event,
        write: impl FnOnce(&Lock) -> Result<(), Problem>,
    ) -> Result<(), Problem> {
        if event.changes.is_empty() {
            return write(self);
        }
        let file = events::write(&self.root, event)?;
        write(self).inspect_err(|_| {
            let _ = fs::remove_file(&file);
        })
    }
}

/// A task as changed by [`change_task`], with what is worth telling the person.
#[derive(Debug, Clone, PartialEq)]
pub struct Changed {
    pub task: Task,
    /// Things worth telling that did not refuse the change (for example `took_over`).
    pub warnings: Vec<Problem>,
    /// Problems found in the taskbag's files while loading it.
    pub issues: Vec<Issue>,
    /// The tasks this change freed: it finished their last open blocker, so they are no longer
    /// blocked. They are not changed: each keeps its state (decision 0026).
    pub unblocked: Vec<Task>,
}

/// The taskbag loaded at `root`, or why it cannot be worked on.
pub fn taskbag_of(root: &Path, loaded: Loaded) -> Result<(Taskbag, Vec<Issue>), Problem> {
    match loaded.taskbag {
        Some(bag) => Ok((bag, loaded.issues)),
        None => Err(Problem::TaskbagUnreadable {
            path: root.display().to_string(),
            detail: loaded
                .issues
                .first()
                .map(|i| format!("{}: {}", i.file, i.problem.english()))
                .unwrap_or_default(),
        }),
    }
}

/// Before any change, for `alm` and the app alike: refuse a taskbag in a format this Allomos does
/// not write ([`versions::write_allowed`]: a newer one, or an older one whose tasks are to be moved
/// in, Task-A01-45); take the lock (waiting up to `wait` for another writer), then load what the
/// change builds on.
/// What [`upgrade`] did: the format before and after, the main folder's new name, and how the
/// labels moved (decision 0020).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Upgraded {
    pub from: String,
    pub to: String,
    pub folder: Option<String>,
    /// Tasks whose labels changed.
    pub tasks: usize,
    /// How many tasks carry each label after the upgrade, by label.
    pub labels: std::collections::BTreeMap<String, usize>,
    /// Tasks that lost a dropped label (`drafted`, `implemented`).
    pub dropped: usize,
    /// Tasks that were in the old `blocked` state, each with the state it has now (decision
    /// 0026): the one it had before it was blocked, from its history.
    pub unblocked: Vec<(String, String)>,
    /// The checkpoint made right after the upgrade (`checkpoints/<name>.json`), which fixes the
    /// history recorded before signing; `None` without a signing key.
    pub checkpoint: Option<String>,
}

/// Upgrade an older taskbag to `allomos/9`, which only a person asks for (approver,
/// 2026-10-25). From `allomos/5`: `project_folder` becomes the first of `project_folders`, named
/// after the folder (decision 0019). From `allomos/5` or `allomos/6`: the ladder becomes the label
/// table and each task's `label` its `labels` (decision 0020, In detail 8): `drafted` and
/// `implemented` are dropped, `locally verified` and `deployed and verified` become "… by agent",
/// `human verified` (or `owner verified`) becomes `deployed and verified by human` when the task's
/// history shows `deployed and verified` before it, else `locally verified by human`, and a task
/// whose history has unreviewed AI decisions gets `AI decisions to review`. From anything before
/// `allomos/9` (decision 0026): a task in the old `blocked` state gets back the state it had
/// before it was blocked ([`state_before_blocked`]); its blockers stay. `None` when it is already
/// up to date.
pub fn upgrade(root: &Path, actor: Actor, wait: Duration) -> Result<Option<Upgraded>, Problem> {
    versions::write_allowed(root)?;
    let lock = Lock::acquire(root, wait)?;
    let (bag, _) = taskbag_of(root, lock.load())?;
    if bag.project.current() {
        return Ok(None);
    }
    let base = setup::project_folder_of(root, &bag.project)
        .and_then(|f| f.file_name().map(|n| n.to_string_lossy().into_owned()));
    let mut project = allomos_core::folders::upgraded(&bag.project, base.as_deref())
        .unwrap_or(bag.project.clone());
    project.format = FORMAT.to_string();
    let (history, _) = events::list(root);
    let deployed: std::collections::BTreeSet<String> = history
        .iter()
        .filter(|e| e.kind == Kind::Task)
        .filter(|e| {
            e.changes.iter().any(|c| {
                matches!(c.field.as_str(), "label" | "labels")
                    && c.to.as_ref().is_some_and(|v| {
                        v.as_str() == Some("deployed and verified")
                            || v.as_array().is_some_and(|a| {
                                a.iter()
                                    .any(|x| x.as_str() == Some("deployed and verified"))
                            })
                    })
            })
        })
        .filter_map(|e| e.id.clone())
        .collect();
    let unreviewed = decision::pending(&history);
    let mut changed = Vec::new();
    let mut dropped = 0;
    // From `allomos/7` on, labels already have their shape: only signing starts.
    let labels_done = allomos_core::format_version(&bag.project.format)
        .is_some_and(|n| n >= allomos_core::LABELS_FROM);
    for task in bag.tasks.iter().filter(|_| !labels_done) {
        let path = root.join("tasks").join(&task.id).join("task.json");
        let old = fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .and_then(|v| v.get("label").and_then(|l| l.as_str()).map(String::from));
        if old
            .as_deref()
            .is_some_and(|o| allomos_core::labels::upgraded_label(o, false).is_none())
        {
            dropped += 1;
        }
        let labels = allomos_core::labels::upgraded_labels(
            old.as_deref(),
            deployed.contains(&task.id),
            unreviewed
                .get(&task.id)
                .is_some_and(|d| d.iter().any(|x| x.to_review())),
        );
        let mut next = task.clone();
        next.labels = labels;
        allomos_core::labels::add_found(&mut project, &next.labels);
        changed.push(next);
    }
    // Decision 0026: `blocked` is no longer a state. Each task still in it gets back the state it
    // had before it was blocked, with what it waited on then.
    let mut unblocked = Vec::new();
    let mut restored: std::collections::BTreeMap<String, Vec<allomos_core::event::Change>> =
        Default::default();
    for task in bag.tasks.iter().filter(|t| t.old_blocked()) {
        let mut next = match changed.iter().position(|c| c.id == task.id) {
            Some(i) => changed.remove(i),
            None => task.clone(),
        };
        let was = state_before_blocked(&history, &next);
        let text = |v: &Option<String>| v.clone().map(serde_json::Value::String);
        let mut changes = vec![allomos_core::event::Change {
            extra: Default::default(),
            field: "state".into(),
            from: Some(serde_json::json!("blocked")),
            to: Some(serde_json::json!(was.state.as_str())),
        }];
        for (field, from, to) in [
            ("waiting_on", &next.waiting_on, &was.waiting_on),
            ("who", &next.who, &was.who),
        ] {
            if from != to {
                changes.push(allomos_core::event::Change {
                    extra: Default::default(),
                    field: field.into(),
                    from: text(from),
                    to: text(to),
                });
            }
        }
        changes.push(event::reason_change(if was.found {
            "upgrade: blocked is no longer a state (decision 0026); this is the state it had before it was blocked, from its history"
        } else {
            "upgrade: blocked is no longer a state (decision 0026); its history does not say what it was before, so it is ready (in progress when held)"
        }));
        next.extra.remove("state");
        next.state = was.state;
        next.waiting_on = was.waiting_on;
        next.who = was.who;
        unblocked.push((next.id.clone(), next.state.as_str().to_string()));
        restored.insert(next.id.clone(), changes);
        changed.push(next);
    }
    let before = Project {
        format: bag.project.format.clone(),
        ..bag.project.clone()
    };
    let event = actor.event(
        "upgrade",
        Kind::Project,
        Some(&project.id),
        event::changes(Some(&before), &project, &[]),
    );
    lock.record(&event, |l| l.save_project(&project))?;
    let mut tasks = 0;
    let mut counts = std::collections::BTreeMap::new();
    for next in &changed {
        for l in &next.labels {
            *counts.entry(l.clone()).or_insert(0) += 1;
        }
        // Every file is rewritten in the new shape, so the old `label` field goes.
        let raw_label = fs::read_to_string(root.join("tasks").join(&next.id).join("task.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .and_then(|v| v.get("label").cloned())
            .filter(|v| !v.is_null());
        let to = (!next.labels.is_empty()).then(|| serde_json::json!(next.labels));
        let mut changes = Vec::new();
        if !labels_done && (raw_label.is_some() || to.is_some()) {
            changes.push(allomos_core::event::Change {
                extra: Default::default(),
                field: "labels".into(),
                from: raw_label,
                to,
            });
        }
        if !changes.is_empty() {
            tasks += 1;
        }
        changes.extend(restored.remove(&next.id).unwrap_or_default());
        let event = actor.event("upgrade", Kind::Task, Some(&next.id), changes);
        lock.record(&event, |l| l.save_task(next))?;
    }
    if labels_done {
        counts.clear();
        for task in &bag.tasks {
            for l in &task.labels {
                *counts.entry(l.clone()).or_insert(0) += 1;
            }
        }
    }
    // The `upgrade` event is the first signed one; a checkpoint fixes everything written before
    // it (decision 0021 § 4, events before signing: option A). Without a key store, none.
    let checkpoint = checkpoint::make(root, None).ok().map(|m| m.file);
    Ok(Some(Upgraded {
        from: bag.project.format.clone(),
        to: project.format.clone(),
        folder: project.project_folders.first().map(|f| f.name.clone()),
        tasks,
        labels: counts,
        dropped,
        unblocked,
        checkpoint,
    }))
}

/// What a task in the old `blocked` state goes back to on the upgrade to `allomos/9`.
struct BeforeBlocked {
    state: allomos_core::State,
    waiting_on: Option<String>,
    who: Option<String>,
    /// Its history said which state it had; otherwise the fallback.
    found: bool,
}

/// The state `task` had before it was blocked (decision 0026, detail l): from the last change in
/// its history that made it `blocked`, the state before that change, with what it waited on then.
/// A state that no longer fits is corrected: in progress needs a holder and a holder means in
/// progress, waiting needs what it waits on. With nothing in the history: in progress when
/// someone holds it, else ready.
fn state_before_blocked(history: &[allomos_core::event::Event], task: &Task) -> BeforeBlocked {
    use allomos_core::State;
    let held = task.holder.is_some();
    let fallback = if held {
        State::InProgress
    } else {
        State::Ready
    };
    let text = |v: &Option<serde_json::Value>| {
        v.as_ref()
            .and_then(|v| v.as_str())
            .filter(|t| !t.trim().is_empty())
            .map(String::from)
    };
    let found = history
        .iter()
        .filter(|e| e.kind == Kind::Task && e.id.as_deref() == Some(task.id.as_str()))
        .filter(|e| {
            e.changes.iter().any(|c| {
                c.field == "state" && c.to.as_ref().and_then(|v| v.as_str()) == Some("blocked")
            })
        })
        .max_by(|a, b| a.at.cmp(&b.at));
    let Some(event) = found else {
        return BeforeBlocked {
            state: fallback,
            waiting_on: None,
            who: None,
            found: false,
        };
    };
    let from = |field: &str| {
        event
            .changes
            .iter()
            .find(|c| c.field == field)
            .and_then(|c| text(&c.from))
    };
    let waiting_on = from("waiting_on");
    let state = match from("state").as_deref() {
        Some("backlog") if !held => Some(State::Backlog),
        Some("ready") if !held => Some(State::Ready),
        Some("waiting") if waiting_on.is_some() => Some(State::Waiting),
        Some("in_progress") if held => Some(State::InProgress),
        _ => None,
    };
    match state {
        Some(State::Waiting) => BeforeBlocked {
            state: State::Waiting,
            waiting_on,
            who: from("who"),
            found: true,
        },
        Some(state) => BeforeBlocked {
            state,
            waiting_on: None,
            who: None,
            found: true,
        },
        None => BeforeBlocked {
            state: fallback,
            waiting_on: None,
            who: None,
            found: false,
        },
    }
}

pub(crate) fn lock_for_change(
    root: &Path,
    wait: Duration,
) -> Result<(Lock, Taskbag, Vec<Issue>), Problem> {
    versions::write_allowed(root)?;
    versions::upgraded_for_writing(root)?;
    let lock = Lock::acquire(root, wait)?;
    let (bag, issues) = taskbag_of(root, lock.load())?;
    Ok((lock, bag, issues))
}

/// Something added by [`add_area`], [`add_iteration`] or [`add_task`] (or renamed, changed or
/// deleted by [`rename_area`], [`change_iteration`], [`delete_area`], [`delete_iteration`]), with
/// the problems found in the taskbag's files while loading it.
#[derive(Debug, Clone, PartialEq)]
pub struct Added<T> {
    pub item: T,
    pub issues: Vec<Issue>,
}

/// A new area, the same way for `alm add area` and the app (Task-A01-19): under the lock, by the
/// `crates/core` rule, recorded as one `add area` event. Without an ID it takes the next free one.
pub fn add_area(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: Option<&str>,
    name: &str,
) -> Result<Added<Area>, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let id = id.map_or_else(|| ops::next_area_id(&bag), String::from);
    let area = ops::new_area(&bag, &id, name)?;
    let event = actor.event(
        "add area",
        Kind::Area,
        Some(&area.id),
        event::changes(None, &area, &[]),
    );
    lock.record(&event, |l| l.create_area(&area))?;
    Ok(Added { item: area, issues })
}

/// A new iteration, the same way for `alm add iteration` and the app (Task-A01-19): under the lock,
/// by the `crates/core` rule, recorded as one `add iteration` event. Without an ID it takes the
/// next free one.
pub fn add_iteration(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: Option<&str>,
    name: &str,
    details: ops::IterationDetails,
) -> Result<Added<Iteration>, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let id = id.map_or_else(|| ops::next_iteration_id(&bag), String::from);
    let iteration = ops::new_iteration(&bag, &id, name, details)?;
    let event = actor.event(
        "add iteration",
        Kind::Iteration,
        Some(&iteration.id),
        event::changes(None, &iteration, &[]),
    );
    lock.record(&event, |l| l.create_iteration(&iteration))?;
    Ok(Added {
        item: iteration,
        issues,
    })
}

/// The task pane's "New…" iteration (Task-A01-39): a new iteration, by the rule `alm add
/// iteration` uses (without an ID, the next free one), then task `task` planned in it by the rule `alm plan` uses,
/// under one lock, recorded as two events (`add iteration`, `plan`): the same as typing both
/// commands. Both rules are checked before anything is written, so a refusal of either (an empty
/// name, a bad date, an unknown task) writes nothing.
pub fn add_iteration_and_plan(
    root: &Path,
    actor: Actor,
    wait: Duration,
    task: &str,
    id: Option<&str>,
    name: &str,
    details: ops::IterationDetails,
) -> Result<(Iteration, Changed), Problem> {
    let (lock, mut bag, issues) = lock_for_change(root, wait)?;
    let id = id.map_or_else(|| ops::next_iteration_id(&bag), String::from);
    let iteration = ops::new_iteration(&bag, &id, name, details)?;
    bag.iterations.push(iteration.clone());
    let planned = ops::plan(&bag, task, Some(&iteration.id))?;
    let before = bag.tasks.iter().find(|t| t.id == planned.task.id).cloned();
    let added = actor.event(
        "add iteration",
        Kind::Iteration,
        Some(&iteration.id),
        event::changes(None, &iteration, &[]),
    );
    lock.record(&added, |l| l.create_iteration(&iteration))?;
    let plan = actor.event(
        "plan",
        Kind::Task,
        Some(&planned.task.id),
        event::changes(before.as_ref(), &planned.task, event::TASK_FIELDS),
    );
    lock.record(&plan, |l| l.save_task(&planned.task))?;
    Ok((
        iteration,
        Changed {
            task: planned.task,
            warnings: planned.warnings,
            issues,
            unblocked: Vec::new(),
        },
    ))
}

/// An area renamed, the same way for `alm rename area` and the app (Task-A01-29): under the lock,
/// by the `crates/core` rule, recorded as one `rename area` event. Its ID stays.
pub fn rename_area(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    name: &str,
) -> Result<Added<Area>, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let area = ops::rename_area(&bag, id, name)?;
    let before = bag.areas.iter().find(|a| a.id == area.id);
    let event = actor.event(
        "rename area",
        Kind::Area,
        Some(&area.id),
        event::changes(before, &area, &[]),
    );
    lock.record(&event, |l| l.save_area(&area))?;
    Ok(Added { item: area, issues })
}

/// What [`rename_iteration`] changed: the iteration under its new ID, and the tasks moved along.
pub type RenamedIteration = (Iteration, Vec<Task>);

/// An iteration given a new ID (`alm rename iteration`, Task-A00-rope803): under the lock, by the
/// `crates/core` rule, its file written under the new ID and the old one removed, recorded as one
/// `rename iteration` event for it (its `id` from the old to the new) and one for each task
/// planned in it. Renaming to the same ID writes nothing.
pub fn rename_iteration(
    root: &Path,
    actor: Actor,
    wait: Duration,
    old: &str,
    new: &str,
) -> Result<Added<RenamedIteration>, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let (iteration, tasks) = ops::rename_iteration(&bag, old, new)?;
    if iteration.id != old {
        let before = bag.iterations.iter().find(|i| i.id == old);
        let event = actor.event(
            "rename iteration",
            Kind::Iteration,
            Some(&iteration.id),
            event::changes(before, &iteration, &[]),
        );
        lock.record(&event, |l| {
            l.save_iteration(&iteration)?;
            l.remove_item("iterations", old)
        })?;
        for task in &tasks {
            let before = bag.tasks.iter().find(|t| t.id == task.id);
            let event = actor.event(
                "rename iteration",
                Kind::Task,
                Some(&task.id),
                event::changes(before, task, event::TASK_FIELDS),
            );
            lock.record(&event, |l| l.save_task(task))?;
        }
    }
    Ok(Added {
        item: (iteration, tasks),
        issues,
    })
}

/// What [`rename_label`] changed: the new ladder, and the tasks whose label changed.
pub type RenamedLabel = (Vec<String>, Vec<Task>);

/// A label renamed on the ladder together with every task that carries it (`alm rename label`,
/// Task-A01-32): under the lock, by the `crates/core` rule, recorded as one `rename label` event
/// for the project and one for each task changed. Returns the new ladder and the tasks changed.
pub fn rename_label(
    root: &Path,
    actor: Actor,
    wait: Duration,
    old: &str,
    new: &str,
) -> Result<Added<RenamedLabel>, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let (project, tasks) = ops::rename_label(&bag, old, new)?;
    let event = actor.event(
        "rename label",
        Kind::Project,
        Some(&project.id),
        event::changes(Some(&bag.project), &project, &[]),
    );
    lock.record(&event, |l| l.save_project(&project))?;
    for task in &tasks {
        let before = bag.tasks.iter().find(|t| t.id == task.id);
        let event = actor.event(
            "rename label",
            Kind::Task,
            Some(&task.id),
            event::changes(before, task, event::TASK_FIELDS),
        );
        lock.record(&event, |l| l.save_task(task))?;
    }
    Ok(Added {
        item: (project.label_names(), tasks),
        issues,
    })
}

/// An iteration changed (name, release, dates), the same way for `alm change iteration` and the
/// app (Task-A01-29): under the lock, by the `crates/core` rule, recorded as one
/// `change iteration` event. A change that changes nothing writes no event.
pub fn change_iteration(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    change: &ops::IterationChange,
) -> Result<Added<Iteration>, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let iteration = ops::change_iteration(&bag, id, change)?;
    let before = bag.iterations.iter().find(|i| i.id == iteration.id);
    let event = actor.event(
        "change iteration",
        Kind::Iteration,
        Some(&iteration.id),
        event::changes(before, &iteration, &[]),
    );
    lock.record(&event, |l| l.save_iteration(&iteration))?;
    Ok(Added {
        item: iteration,
        issues,
    })
}

/// An iteration moved before or after another one (`alm move iteration`, Task-A00-skate868): under
/// the lock, by the `crates/core` rule, recorded as one `move iteration` event for each iteration
/// whose order changed. Returns the iterations changed; a move that changes nothing writes none.
pub fn move_iteration(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    place: ops::Place<'_>,
) -> Result<Added<Vec<Iteration>>, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let moved = ops::move_iteration(&bag, id, place)?;
    for iteration in &moved {
        let before = bag.iterations.iter().find(|i| i.id == iteration.id);
        let event = actor.event(
            "move iteration",
            Kind::Iteration,
            Some(&iteration.id),
            event::changes(before, iteration, &[]),
        );
        lock.record(&event, |l| l.save_iteration(iteration))?;
    }
    Ok(Added {
        item: moved,
        issues,
    })
}

/// An area deleted, the same way for `alm delete area` and the app (Task-A01-29): refused while
/// any task is in it (done and dropped ones too); otherwise its file is removed under the lock,
/// recorded as one `delete area` event listing what it had. Returns the area as it was.
pub fn delete_area(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
) -> Result<Added<Area>, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let area = ops::delete_area(&bag, id)?;
    let event = actor.event(
        "delete area",
        Kind::Area,
        Some(&area.id),
        event::removed(&area, &[]),
    );
    lock.record(&event, |l| l.remove_item("areas", &area.id))?;
    Ok(Added { item: area, issues })
}

/// An iteration deleted, the same way for `alm delete iteration` and the app (Task-A01-29):
/// refused while any task is planned in it (done and dropped ones too); otherwise its file is
/// removed under the lock, recorded as one `delete iteration` event. Returns it as it was.
pub fn delete_iteration(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
) -> Result<Added<Iteration>, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let iteration = ops::delete_iteration(&bag, id)?;
    let event = actor.event(
        "delete iteration",
        Kind::Iteration,
        Some(&iteration.id),
        event::removed(&iteration, &[]),
    );
    lock.record(&event, |l| l.remove_item("iterations", &iteration.id))?;
    Ok(Added {
        item: iteration,
        issues,
    })
}

/// What a new task is made of (`alm add task`, the app's New task form).
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewTask {
    /// Without one, the next free ID in the area.
    #[serde(default)]
    pub id: Option<String>,
    pub area: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    /// The app only: plan it there too, as `alm plan` would (owner, 2026-10-14).
    #[serde(default)]
    pub iteration: Option<String>,
    /// The branch the agent is on (`alm add task --branch`), which chooses the base (Task-A03-22).
    #[serde(default)]
    pub branch: Option<String>,
    /// The base given (`--base`, or the person's pick in the app): it wins over `branch`.
    #[serde(default)]
    pub base: Option<String>,
    /// The project folders it belongs to (`alm add task --folder`, decision 0019); empty: none in
    /// particular. Each must be one of the taskbag's folders.
    #[serde(default)]
    pub folders: Vec<String>,
    /// How hard it is, 1 to 5 (`alm add task --difficulty`, Task-A01-actor813); `None`: not given,
    /// it counts as 3.
    #[serde(default)]
    pub difficulty: Option<u8>,
}

/// What [`add_task_with_base`] did: the task, the base it got and why, and warnings.
#[derive(Debug, Clone, PartialEq)]
pub struct AddedTask {
    pub task: Task,
    /// The base the task got, and why; `None` when neither a branch nor a base was given.
    pub base: Option<branch::Choice>,
    /// `branch_missing`, or `base_differs` for each task the adding session holds on another base.
    pub warnings: Vec<Problem>,
    pub issues: Vec<Issue>,
}

/// A new task in backlog, the same way for `alm add task` and the app (Task-A01-19): under the
/// lock, by the `crates/core` rule, recorded as one `add task` event. With an iteration it is then
/// planned under the same lock, by the rule `alm plan` uses, as a second `plan` event: the same as
/// typing both commands (owner, 2026-10-14). Both rules are checked before anything is written, so
/// a refusal of either writes nothing.
pub fn add_task(
    root: &Path,
    actor: Actor,
    wait: Duration,
    new: &NewTask,
) -> Result<Added<Task>, Problem> {
    add_task_with_base(root, actor, wait, new).map(|a| Added {
        item: a.task,
        issues: a.issues,
    })
}

/// [`add_task`], telling also which base the task got and why (Task-A03-22). The base is chosen
/// by [`branch::choose_base`] under the same lock and recorded in the `add task` event as a
/// `base_branch` change; `task.json` does not change shape.
pub fn add_task_with_base(
    root: &Path,
    actor: Actor,
    wait: Duration,
    new: &NewTask,
) -> Result<AddedTask, Problem> {
    let (lock, mut bag, issues) = lock_for_change(root, wait)?;
    let id = match &new.id {
        Some(id) => id.clone(),
        None => ops::next_task_id(&bag, &new.area)?,
    };
    let mut task = ops::new_task(&bag, &id, &new.area, &new.title, new.description.as_deref())?;
    task.folders = allomos_core::folders::check(&bag.project, &new.folders)?;
    if let Some(n) = new.difficulty {
        task.difficulty = Some(i64::from(allomos_core::difficulty::level(&n.to_string())?));
    }
    let recorded = branches(root);
    let filled = |v: &Option<String>| v.clone().filter(|b| !b.trim().is_empty());
    let base = branch::choose_base(
        &bag,
        &recorded,
        filled(&new.branch).as_deref(),
        filled(&new.base).as_deref(),
    )?;
    let mut warnings = Vec::new();
    match &base {
        Some(c) => warnings.extend(branch::other_bases(
            &bag, &recorded, actor.by, &task.id, &c.base,
        )),
        // The app adds without a branch when no open task has a base: nothing to say.
        None if actor.via == "alm" => warnings.push(Problem::BranchMissing {
            id: task.id.clone(),
        }),
        None => {}
    }
    bag.tasks.push(task.clone());
    let planned = match new
        .iteration
        .as_deref()
        .map(str::trim)
        .filter(|i| !i.is_empty())
    {
        Some(iteration) => Some(ops::plan(&bag, &task.id, Some(iteration))?.task),
        None => None,
    };
    let mut changes = event::changes(None, &task, event::TASK_FIELDS);
    if let Some(c) = &base {
        changes.extend(branch::change(branch::BASE, None, &c.base));
    }
    let added = actor.event("add task", Kind::Task, Some(&task.id), changes);
    lock.record(&added, |l| l.create_task(&task))?;
    let Some(planned) = planned else {
        return Ok(AddedTask {
            task,
            base,
            warnings,
            issues,
        });
    };
    let plan = actor.event(
        "plan",
        Kind::Task,
        Some(&task.id),
        event::changes(Some(&task), &planned, event::TASK_FIELDS),
    );
    lock.record(&plan, |l| l.save_task(&planned))?;
    Ok(AddedTask {
        task: planned,
        base,
        warnings,
        issues,
    })
}

/// One change to one task, the same way for `alm` and the app (Task-A01-18): refuse a taskbag in
/// another format, take the lock (waiting up to `wait` for another writer), load, apply `rule`
/// (a `crates/core` operation, which also names its command), then write the task and its event.
/// A refused change writes nothing.
pub fn change_task(
    root: &Path,
    actor: Actor,
    wait: Duration,
    rule: impl FnOnce(&Taskbag) -> Result<(allomos_core::ops::Outcome, &'static str), Problem>,
) -> Result<Changed, Problem> {
    change_task_under_lock(root, actor, wait, rule, false).map(|alone| alone.changed)
}

/// What [`change_task_alone`] did: the change, and what a window needs to show it without reading
/// the whole taskbag again (Task-A01-parade485).
#[derive(Debug, Clone, PartialEq)]
pub struct ChangedAlone {
    pub changed: Changed,
    /// When the change was recorded (its event's `at`, UTC): the task's new "last changed". None
    /// when nothing changed, so no event was written.
    pub at: Option<String>,
    /// The change also wrote `project.json` (a label new to the label table): more than this one
    /// task changed.
    pub project_changed: bool,
    /// The taskbag's fingerprint ([`stamp`]) just before the change was written and just after,
    /// both taken under the lock, so no other writer's change falls between them: a window whose
    /// last fingerprint is `stamp_before` has seen everything but this change.
    pub stamp_before: u64,
    pub stamp_after: u64,
}

/// [`change_task`] for a window that shows the taskbag (Task-A01-parade485): the same change, the
/// same way, and with it what the window needs to update this one task in place.
pub fn change_task_alone(
    root: &Path,
    actor: Actor,
    wait: Duration,
    rule: impl FnOnce(&Taskbag) -> Result<(allomos_core::ops::Outcome, &'static str), Problem>,
) -> Result<ChangedAlone, Problem> {
    change_task_under_lock(root, actor, wait, rule, true)
}

fn change_task_under_lock(
    root: &Path,
    actor: Actor,
    wait: Duration,
    rule: impl FnOnce(&Taskbag) -> Result<(allomos_core::ops::Outcome, &'static str), Problem>,
    stamps: bool,
) -> Result<ChangedAlone, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let (mut outcome, command) = rule(&bag)?;
    // Only a window asks for the fingerprints: `alm` does not pay for them.
    let stamp_before = if stamps { stamp(root) } else { 0 };
    // A label the table does not list yet is added to it, and `alm` says so (decision 0020, 5).
    let mut project = bag.project.clone();
    let added = allomos_core::labels::add_found(&mut project, &outcome.task.labels);
    let project_changed = !added.is_empty();
    if !added.is_empty() {
        let event = actor.event(
            "labels",
            Kind::Project,
            Some(&project.id),
            event::changes(Some(&bag.project), &project, &[]),
        );
        lock.record(&event, |l| l.save_project(&project))?;
        outcome
            .warnings
            .extend(added.into_iter().map(|label| Problem::LabelAddedToTable {
                id: outcome.task.id.clone(),
                label,
            }));
    }
    let before = bag.tasks.iter().find(|t| t.id == outcome.task.id);
    let described = before.is_some_and(|b| b.description != outcome.task.description);
    let mut changes = event::changes(before, &outcome.task, event::TASK_FIELDS);
    changes.extend(outcome.recorded.iter().cloned());
    // The holder's change is a heartbeat too (decision 0022 § 5): no extra commit, no event. A
    // change that changes nothing writes none either.
    if !changes.is_empty() {
        heartbeat::along(&mut outcome.task, actor.by, now_secs());
    }
    let event = actor.event(command, Kind::Task, Some(&outcome.task.id), changes);
    lock.record(&event, |l| {
        l.save_task(&outcome.task)?;
        if described {
            l.save_description(&outcome.task)?;
        }
        Ok(())
    })?;
    let stamp_after = if stamps { stamp(root) } else { 0 };
    let unblocked = freed_by(&bag, &outcome.task);
    Ok(ChangedAlone {
        at: (!event.changes.is_empty()).then(|| event.at.clone()),
        project_changed,
        stamp_before,
        stamp_after,
        changed: Changed {
            task: outcome.task,
            warnings: outcome.warnings,
            issues,
            unblocked,
        },
    })
}

/// Now, in seconds since 1970 (UTC); `ALLOMOS_NOW` fixes it in tests, as for events.
pub fn now_secs() -> i64 {
    heartbeat::parse_utc(&events::now_utc()).unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    })
}

/// What `alm beat` did ([`beat`]).
#[derive(Debug, Clone, PartialEq)]
pub struct Beaten {
    /// One entry for each task beaten for, in ID order: written, or too soon to write again.
    pub beats: Vec<heartbeat::Beat>,
    /// The project's heartbeat interval that applied, in seconds (Task-A01-zero387).
    pub interval_secs: i64,
    /// Whether the project asks for work in progress to be pushed at each heartbeat
    /// (Task-A01-butter895).
    pub push_work: bool,
    pub issues: Vec<Issue>,
}

/// `alm beat` (decision 0022, Task-A01-nurse164): write the heartbeat of task `id`, or with no
/// `id` of every task the session `actor.by` holds (in progress), in one change under the lock.
/// `busy`: a busy window of that many seconds. No event is written, and a task whose heartbeat is
/// younger than half the interval is left as it is. Refused without a session name
/// (`holder_missing`), for a task the session does not hold (`task_held`, `not_held`).
pub fn beat(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: Option<&str>,
    busy: Option<i64>,
    doing: Option<&str>,
) -> Result<Beaten, Problem> {
    let me = actor.by.ok_or(Problem::HolderMissing)?;
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let now = now_secs();
    let applied = heartbeat::applied(&bag.project);
    let interval = applied.interval_secs;
    let tasks: Vec<&Task> = match id {
        Some(id) => vec![
            bag.tasks
                .iter()
                .find(|t| t.id == id)
                .ok_or_else(|| Problem::TaskNotFound { id: id.to_string() })?,
        ],
        None => {
            let mut held: Vec<&Task> = bag
                .tasks
                .iter()
                .filter(|t| {
                    t.state == allomos_core::State::InProgress && t.holder.as_deref() == Some(me)
                })
                .collect();
            held.sort_by(|a, b| a.id.cmp(&b.id));
            held
        }
    };
    let beats = tasks
        .into_iter()
        .map(|t| heartbeat::beat(t, me, now, busy, doing, interval))
        .collect::<Result<Vec<_>, _>>()?;
    for b in &beats {
        if let heartbeat::Beat::Written(task) = b {
            lock.save_task(task)?;
        }
    }
    Ok(Beaten {
        beats,
        interval_secs: interval,
        push_work: applied.push_work,
        issues,
    })
}

/// After a change that leaves `task` finished (done or dropped): the tasks it was the last open
/// blocker of, which are no longer blocked. Nothing is written: being blocked is worked out from
/// `blocked_by`, and each task keeps its state (decision 0026, replacing Task-A01-tuna668's
/// automatic move to ready). The caller only tells.
pub(crate) fn freed_by(bag: &Taskbag, task: &Task) -> Vec<Task> {
    if !matches!(
        task.state,
        allomos_core::State::Done | allomos_core::State::Dropped
    ) {
        return Vec::new();
    }
    let mut after = bag.clone();
    match after.tasks.iter_mut().find(|t| t.id == task.id) {
        Some(t) => *t = task.clone(),
        None => after.tasks.push(task.clone()),
    }
    bag.tasks
        .iter()
        .filter(|t| t.id != task.id && t.blocked_by.contains(&task.id))
        .filter(|t| ops::is_blocked(bag, t) && !ops::is_blocked(&after, t))
        .cloned()
        .collect()
}

/// Every task's branches (base and working), by task ID (Task-A03-22), from the history.
pub fn branches(root: &Path) -> std::collections::BTreeMap<String, branch::Branches> {
    branch::recorded(&events::list(root).0)
}

/// When each task last changed, by task ID (Task-A01-52): the newest `at` among its events, in
/// UTC. The Iterations tab sorts by it. A task with no recorded change is left out.
pub fn last_changed(root: &Path) -> std::collections::BTreeMap<String, String> {
    let mut out: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    for e in events::list(root).0 {
        let (Kind::Task, Some(id)) = (e.kind, e.id) else {
            continue;
        };
        let at = out.entry(id).or_default();
        if e.at > *at {
            *at = e.at;
        }
    }
    out
}

/// When each task was made, by task ID (Task-A01-54): the oldest `at` among its events, in UTC.
/// Lists order word IDs by it. A task with no recorded change is left out.
pub fn created(root: &Path) -> std::collections::BTreeMap<String, String> {
    let mut out: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    for e in events::list(root).0 {
        let (Kind::Task, Some(id)) = (e.kind, e.id) else {
            continue;
        };
        let at = out.entry(id).or_insert_with(|| e.at.clone());
        if e.at < *at {
            *at = e.at;
        }
    }
    out
}

/// What [`claim_task`] did: the change, and the task's branches after it.
#[derive(Debug, Clone, PartialEq)]
pub struct Claimed {
    pub changed: Changed,
    /// What the claim recorded, or found, about the task's branches.
    pub branches: branch::AtClaim,
    /// Set when the claim took the task over from another holder (Task-A01-sign613).
    pub took_over: Option<TookOver>,
}

/// A claim that took a task over from another holder (decision 0022 § 6, Task-A01-sign613): from
/// whom, whether it was lost (and since when), and where the former holder worked, when its claim
/// recorded that. The claim's event records the change of holder; this is what `alm claim` tells.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TookOver {
    /// The former holder.
    pub from: String,
    /// Set when the task was lost; `None` when it was taken with `--take` while not lost.
    pub lost: Option<heartbeat::Lost>,
    /// The folder the former holder's claim ran in, relative to the folder holding `.allomos/`,
    /// when it recorded one. It may still hold uncommitted changes.
    pub folder: Option<String>,
    /// The working branch the task had before the claim.
    pub working: Option<String>,
    /// What the former holder last said it was doing (`alm beat --doing`), and when, in UTC.
    pub doing: Option<String>,
    pub doing_at: Option<String>,
}

/// Claim task `id` (`alm claim`, Task-A03-22): the claim rule, plus its branches. `current` is the
/// branch the agent is on, `working` the branch the work is done on (default `current`). A task
/// without a base gets `current`. `session` is the agent's session as it passed it (Task-A03-hive049):
/// it replaces the task's last one, and nothing passed clears it. One `claim` event holds both the
/// task's changes and the branch changes.
#[allow(clippy::too_many_arguments)]
pub fn claim_task(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    take: bool,
    current: Option<&str>,
    working: Option<&str>,
    session: Option<allomos_core::AgentSession>,
) -> Result<Claimed, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    // A lost task may be claimed: claiming it takes it over (decision 0022 § 6, Task-A01-sign613).
    // One held by someone else and not lost is refused as before, unless `--take`.
    let now = now_secs();
    let found = bag.tasks.iter().find(|t| t.id == id);
    let former = found
        .filter(|t| !matches!(t.state, State::Done | State::Dropped))
        .and_then(|t| t.holder.clone())
        .filter(|h| Some(h.as_str()) != actor.by.map(str::trim));
    let lost = match (found, &former) {
        (Some(t), Some(_)) => {
            let applied = heartbeat::applied(&bag.project);
            let last = last_changed(root);
            heartbeat::lost(
                t,
                now,
                applied.lost_after_secs,
                last.get(&t.id).map(String::as_str),
            )
        }
        _ => None,
    };
    let mut outcome = ops::claim(&bag, id, actor.by, take || lost.is_some())?;
    if let Some(l) = &lost {
        for w in outcome.warnings.iter_mut() {
            if let Problem::TookOver { id, previous } = w {
                *w = Problem::TookOverLost {
                    id: id.clone(),
                    previous: previous.clone(),
                    since: l.since.clone(),
                };
            }
        }
    }
    let before = branches(root).remove(id).unwrap_or_default();
    let took_over = former.map(|from| TookOver {
        from,
        lost: lost.clone(),
        folder: found
            .and_then(|t| t.session.as_ref())
            .and_then(|s| s.folder.clone()),
        working: before.working.clone(),
        doing: found
            .and_then(heartbeat::doing)
            .map(|(words, _)| words.to_string()),
        doing_at: found
            .and_then(heartbeat::doing)
            .and_then(|(_, at)| at.map(str::to_string)),
    });
    // Work goes on on the task's working branch (decision 0022 § 6): a takeover keeps it unless
    // `--working` names another.
    let working = match (&took_over, working) {
        (_, Some(w)) => Some(w),
        (Some(t), None) => t.working.as_deref().or(current),
        (None, None) => None,
    };
    outcome.task.session = session
        .filter(|s| !s.is_empty())
        .map(|s| allomos_core::AgentSession {
            by: outcome.task.holder.clone(),
            ..s
        });
    let at = branch::at_claim(id, &before, current, working)?;
    let old = bag.tasks.iter().find(|t| t.id == outcome.task.id);
    let mut changes = event::changes(old, &outcome.task, event::TASK_FIELDS);
    changes.extend(at.changes.iter().cloned());
    if !changes.is_empty() {
        heartbeat::along(&mut outcome.task, actor.by, now);
    }
    let event = actor.event("claim", Kind::Task, Some(&outcome.task.id), changes);
    lock.record(&event, |l| l.save_task(&outcome.task))?;
    let mut warnings = outcome.warnings;
    warnings.extend(at.warnings.iter().cloned());
    Ok(Claimed {
        changed: Changed {
            task: outcome.task,
            warnings,
            issues,
            unblocked: Vec::new(),
        },
        branches: at,
        took_over,
    })
}

/// Change task `id`'s base to `name` (`alm base`, Task-A03-22): one `base` event, under the lock;
/// the task's files do not change. Returns the task and its branches after it.
pub fn set_base(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    name: &str,
) -> Result<(Task, branch::Branches, Vec<Issue>), Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let recorded = branches(root);
    let changes = branch::set_base(&bag, &recorded, id, actor.by, name)?;
    let task = ops::find_task(&bag, id)?;
    let event = actor.event(branch::SET_BASE, Kind::Task, Some(&task.id), changes);
    lock.record(&event, |_| Ok(()))?;
    let after = branches(root).remove(&task.id).unwrap_or_default();
    Ok((task, after, issues))
}

/// Every task's traces (the requirements and decisions it serves), by task ID
/// (Task-A01-fennel617), from the history.
pub fn traces(root: &Path) -> std::collections::BTreeMap<String, Vec<String>> {
    allomos_core::traces::recorded(&events::list(root).0)
}

/// Add and remove task `id`'s traces (`alm trace`, Task-A01-fennel617): one `trace` event, under
/// the lock, when the list changes; the task's files do not change. Returns the task and its
/// traces after it.
pub fn set_traces(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    add: &[String],
    remove: &[String],
) -> Result<(Task, Vec<String>, Vec<Issue>), Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let recorded = traces(root);
    let (changes, after) = allomos_core::traces::set(&bag, &recorded, id, actor.by, add, remove)?;
    let task = ops::find_task(&bag, id)?;
    if !changes.is_empty() {
        let event = actor.event(
            allomos_core::traces::TRACE,
            Kind::Task,
            Some(&task.id),
            changes,
        );
        lock.record(&event, |_| Ok(()))?;
    }
    Ok((task, after, issues))
}

/// What [`record_ai_decision`] or [`review_ai_decisions`] did (Task-A03-19).
#[derive(Debug, Clone, PartialEq)]
pub struct Decided {
    pub task: Task,
    /// The decisions the change recorded or marked reviewed.
    pub decisions: Vec<decision::Decision>,
    /// The task's decisions since its last review after the change, details included (see
    /// [`decision::Decision::to_review`] for the ones that wait for review).
    pub pending: Vec<decision::Decision>,
    /// Problems found in the taskbag's files while loading it.
    pub issues: Vec<Issue>,
}

/// Every task's AI decisions since its last review, by task ID (Task-A03-19), from the history:
/// details too, which are kept but do not wait for review (decision 0025 § 3).
pub fn ai_decisions(root: &Path) -> std::collections::BTreeMap<String, Vec<decision::Decision>> {
    decision::pending(&events::list(root).0)
}

/// Record a decision an agent made on its own in task `id` (`alm ai-decided`, Task-A03-19): one
/// `ai-decided` event, under the lock. The task's files do not change.
pub fn record_ai_decision(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    text: &str,
    kind: Option<decision::DecisionKind>,
) -> Result<Decided, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let mut changes = decision::decide(&bag, id, text, kind)?;
    let before = ops::find_task(&bag, id)?;
    // The task carries `AI decisions to review` until `alm reviewed` (decision 0020); a detail
    // is kept but does not wait for review (decision 0025 § 3).
    let mut task = before.clone();
    if kind != Some(decision::DecisionKind::Detail) {
        task.add_label(allomos_core::labels::AI_DECISIONS);
    }
    heartbeat::along(&mut task, actor.by, now_secs());
    changes.extend(event::changes(Some(&before), &task, event::TASK_FIELDS));
    let event = actor.event(decision::DECIDED, Kind::Task, Some(&task.id), changes);
    lock.record(&event, |l| {
        if task != before {
            l.save_task(&task)?;
        }
        Ok(())
    })?;
    let pending = ai_decisions(root).remove(&task.id).unwrap_or_default();
    let decisions = pending.last().cloned().into_iter().collect();
    Ok(Decided {
        task,
        decisions,
        pending,
        issues,
    })
}

/// Record in task `id`'s history that the person chose Go ahead on a plan shown through
/// `alm confirm --task` (Task-A03-37): since Task-A01-coach782 an approval record
/// ([`approvals::approve`], channel `confirm`) whose scope holds the SHA-256 of exactly the plan
/// text shown (`plan`, from the file `plan_file`). A changed plan is a new question.
pub fn record_approval(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    title: &str,
    plan_file: &str,
    plan: &str,
) -> Result<approvals::Approved, Problem> {
    let tasks = [id.to_string()];
    approvals::approve(
        root,
        actor,
        wait,
        &approvals::Input {
            tasks: &tasks,
            approver: None,
            channel: "confirm",
            words: Some(&format!("Go ahead: {title}")),
            expires: None,
            plan: Some((plan_file, plan)),
            shown: None,
        },
    )
}

/// Mark task `id`'s AI decisions reviewed (`alm reviewed`, the app's Reviewed button,
/// Task-A03-19): one `reviewed` event listing each, under the lock. Refused with
/// `nothing_to_review` when it has none. Overturning one is done by changing the task as usual.
pub fn review_ai_decisions(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
) -> Result<Decided, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let before = ops::find_task(&bag, id)?;
    let decisions = ai_decisions(root).remove(&before.id).unwrap_or_default();
    let mut task = before.clone();
    let had_label = task.remove_label(allomos_core::labels::AI_DECISIONS);
    heartbeat::along(&mut task, actor.by, now_secs());
    // A task carrying the label with nothing left in its history is cleared too (decision 0020).
    // Details alone are nothing to review (decision 0025 § 3); a review lists them with the rest.
    if !had_label && !decisions.iter().any(|d| d.to_review()) {
        return Err(Problem::NothingToReview { id: before.id });
    }
    let mut changes = if decisions.is_empty() {
        Vec::new()
    } else {
        decision::review(&bag, id, &decisions)?
    };
    changes.extend(event::changes(Some(&before), &task, event::TASK_FIELDS));
    let event = actor.event(decision::REVIEWED, Kind::Task, Some(&task.id), changes);
    lock.record(&event, |l| {
        if task != before {
            l.save_task(&task)?;
        }
        Ok(())
    })?;
    Ok(Decided {
        task,
        decisions,
        pending: Vec::new(),
        issues,
    })
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
    use allomos_core::ops;
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

    /// Decision 0026: finishing a task's last open blocker changes nothing in that task; the
    /// change only reports it as freed.
    #[test]
    fn finishing_the_last_blocker_frees_a_task_without_changing_it() {
        let dir = tmp("freed");
        let root = dir.join(DEFAULT_DIR);
        init(&root, "p", "P").unwrap();
        let lock = Lock::acquire(&root, Duration::ZERO).unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.create_area(&ops::new_area(&bag, "A01", "First").unwrap())
            .unwrap();
        for id in ["A01-01", "A01-02", "A01-03"] {
            let bag = lock.load().taskbag.unwrap();
            lock.create_task(&ops::new_task(&bag, id, "A01", id, None).unwrap())
                .unwrap();
        }
        drop(lock);
        let me = || Actor::alm(Some("s"));
        let by = |ids: &[&str]| ids.iter().map(|i| i.to_string()).collect::<Vec<_>>();
        // A01-03 (backlog) is blocked by A01-01 and A01-02.
        let c = change_task(&root, me(), Duration::ZERO, |bag| {
            ops::block(bag, "A01-03", Some("s"), &by(&["A01-01", "A01-02"])).map(|o| (o, "block"))
        })
        .unwrap();
        assert_eq!(c.task.state, allomos_core::State::Backlog);
        let done = |id: &'static str| {
            change_task(&root, me(), Duration::ZERO, move |bag| {
                ops::finish(bag, id, Some("s"), None, None).map(|o| (o, "done"))
            })
            .unwrap()
        };
        assert!(done("A01-01").unblocked.is_empty());
        let events = events::list(&root).0.len();
        let freed = done("A01-02").unblocked;
        assert_eq!(freed.len(), 1);
        assert_eq!(freed[0].id, "A01-03");
        // Only the `done` event: the freed task is as it was, blockers and all.
        assert_eq!(events::list(&root).0.len(), events + 1);
        let bag = load(&root).taskbag.unwrap();
        let t = bag.tasks.iter().find(|t| t.id == "A01-03").unwrap();
        assert_eq!(t.state, allomos_core::State::Backlog);
        assert_eq!(t.blocked_by, by(&["A01-01", "A01-02"]));
        assert!(!ops::is_blocked(&bag, t));
        fs::remove_dir_all(dir).unwrap();
    }

    /// Decision 0026, detail l: the upgrade to `allomos/9` gives every task in the old `blocked`
    /// state the state it had before it was blocked, from its history; with nothing there, ready
    /// (in progress when held). Blockers stay.
    #[test]
    fn the_upgrade_gives_blocked_tasks_their_earlier_state() {
        let dir = tmp("upgrade-blocked");
        let root = dir.join(DEFAULT_DIR);
        init(&root, "p", "P").unwrap();
        let lock = Lock::acquire(&root, Duration::ZERO).unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.create_area(&ops::new_area(&bag, "A01", "First").unwrap())
            .unwrap();
        let ids = ["A01-01", "A01-02", "A01-03", "A01-04", "A01-05", "A01-06"];
        for id in ids {
            let bag = lock.load().taskbag.unwrap();
            lock.create_task(&ops::new_task(&bag, id, "A01", id, None).unwrap())
                .unwrap();
        }
        // As an `allomos/8` taskbag had them: `blocked` as a state, with what the history said.
        // 02: backlog before. 03: waiting on Ann before. 04: no history. 05: held, no history.
        // 06: ready before, blocked twice (the later change counts).
        let actor = Actor::alm(Some("s"));
        let change_of =
            |field: &str, from: Option<&str>, to: Option<&str>| allomos_core::event::Change {
                extra: Default::default(),
                field: field.into(),
                from: from.map(|v| serde_json::json!(v)),
                to: to.map(|v| serde_json::json!(v)),
            };
        let mut bag = lock.load().taskbag.unwrap();
        for t in bag.tasks.iter_mut().filter(|t| t.id != "A01-01") {
            let changes = match t.id.as_str() {
                "A01-02" => vec![change_of("state", Some("backlog"), Some("blocked"))],
                "A01-03" => vec![
                    change_of("state", Some("waiting"), Some("blocked")),
                    change_of("waiting_on", Some("the licence"), None),
                    change_of("who", Some("Ann"), None),
                ],
                "A01-06" => vec![change_of("state", Some("ready"), Some("blocked"))],
                _ => vec![],
            };
            if t.id == "A01-05" {
                t.holder = Some("b".into());
            }
            t.state = allomos_core::State::Unknown;
            t.extra.insert("state".into(), serde_json::json!("blocked"));
            t.blocked_by = vec!["A01-01".into()];
            if changes.is_empty() {
                lock.save_task(t).unwrap();
            } else {
                let event = actor.event("block", Kind::Task, Some(&t.id), changes);
                lock.record(&event, |l| l.save_task(t)).unwrap();
            }
        }
        drop(lock);
        let project = root.join(PROJECT_FILE);
        let text = fs::read_to_string(&project).unwrap();
        fs::write(&project, text.replace(FORMAT, "allomos/8")).unwrap();
        let bag = load(&root).taskbag.unwrap();
        assert!(bag.tasks.iter().filter(|t| t.old_blocked()).count() == 5);
        // Not upgraded yet: such a task is not changed, and blockers are not either.
        let refused = change_task(&root, Actor::alm(None), Duration::ZERO, |bag| {
            ops::make_ready(bag, "A01-02", None).map(|o| (o, "ready"))
        });
        assert_eq!(refused.unwrap_err().code(), "blocked_state_old");

        let up = upgrade(&root, Actor::alm(Some("s")), Duration::ZERO)
            .unwrap()
            .unwrap();
        assert_eq!((up.from.as_str(), up.to.as_str()), ("allomos/8", FORMAT));
        let got: Vec<(&str, &str)> = up
            .unblocked
            .iter()
            .map(|(id, state)| (id.as_str(), state.as_str()))
            .collect();
        assert_eq!(
            got,
            [
                ("A01-02", "backlog"),
                ("A01-03", "waiting"),
                ("A01-04", "ready"),
                ("A01-05", "in_progress"),
                ("A01-06", "ready"),
            ]
        );
        let loaded = load(&root);
        assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
        let bag = loaded.taskbag.unwrap();
        let t = |id: &str| bag.tasks.iter().find(|t| t.id == id).unwrap().clone();
        assert_eq!(
            (
                t("A01-03").waiting_on.as_deref(),
                t("A01-03").who.as_deref()
            ),
            (Some("the licence"), Some("Ann"))
        );
        for id in &ids[1..] {
            assert_eq!(t(id).blocked_by, ["A01-01"], "{id}");
            assert!(ops::is_blocked(&bag, &t(id)), "{id}");
            assert!(!t(id).extra.contains_key("state"), "{id}");
        }
        let e = &events::history(&root, "A01-02").events[0];
        assert_eq!(e.command, "upgrade");
        assert!(
            allomos_core::event::reason(e)
                .unwrap()
                .starts_with("upgrade:")
        );
        // Up to date now: nothing more to upgrade.
        assert!(
            upgrade(&root, Actor::alm(Some("s")), Duration::ZERO)
                .unwrap()
                .is_none()
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn init_with_own_id_and_ladder() {
        let dir = tmp("init-ladder");
        let root = dir.join(DEFAULT_DIR);
        let ladder = vec!["shipped".to_string(), " ".to_string()];
        init_with_labels(&root, "allomos", "Allomos", ladder, Actor::APP).unwrap();
        let bag = load(&root).taskbag.unwrap();
        let names = bag.project.label_names();
        assert_eq!(bag.project.id, "allomos");
        assert_eq!(names.len(), 6, "{names:?}");
        assert_eq!(names[5], "shipped");
        let other = dir.join("other");
        init_with_labels(&other, "p", "P", vec![], Actor::APP).unwrap();
        assert_eq!(load(&other).taskbag.unwrap().project.labels.len(), 5);
        let other = dir.join("other-2");
        assert!(!other.exists());
        assert!(matches!(
            init_with_labels(&dir.join("x"), "bad id", "P", vec!["a".into()], Actor::APP),
            Err(Problem::InvalidId { .. })
        ));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn empty_folder_gets_starter_taskbag_in_default_dir() {
        let dir = tmp("empty");
        let root = dir.join(".allomos/taskbag");
        assert_eq!(
            inspect(&dir),
            Inspection::Empty {
                folder: dir.clone(),
                suggested_root: root.clone()
            }
        );
        create_starter(&root, "Demo").unwrap();
        assert_eq!(inspect(&dir), Inspection::Found { root: root.clone() });
        let loaded = load(&root);
        assert_eq!(
            loaded.taskbag.as_ref().unwrap().project.format,
            allomos_core::FORMAT
        );
        assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
        let tb = loaded.taskbag.unwrap();
        assert_eq!(tb.project.name, "Demo");
        assert_eq!(tb.areas.len(), 2);
        assert_eq!(tb.tasks.len(), 3);
        assert!(tb.tasks[0].description.contains("starter"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn repo_folder_gets_nested_taskbag_without_nested_git() {
        let repo = tmp("repo");
        fs::create_dir(repo.join(".git")).unwrap();
        fs::write(repo.join("README.md"), "code").unwrap();
        let Inspection::NoTaskbag { suggested_root, .. } = inspect(&repo) else {
            panic!("expected NoTaskbag")
        };
        assert_eq!(suggested_root, repo.join(".allomos/taskbag"));
        create_starter(&suggested_root, "Repo").unwrap();
        assert_eq!(
            inspect(&repo),
            Inspection::Found {
                root: repo.join(".allomos/taskbag")
            }
        );
        assert!(!suggested_root.join(".git").exists());
        fs::remove_dir_all(repo).unwrap();
    }

    #[test]
    fn refuses_to_overwrite_a_non_empty_folder() {
        let dir = tmp("full");
        fs::write(dir.join("notes.txt"), "keep me").unwrap();
        assert!(create_starter(&dir, "X").is_err());
        assert_eq!(
            fs::read_to_string(dir.join("notes.txt")).unwrap(),
            "keep me"
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn init_makes_an_empty_taskbag_found_from_below() {
        let repo = tmp("init");
        let root = repo.join(DEFAULT_DIR);
        init(&root, &id_from_name("my repo!"), "My repo").unwrap();
        assert_eq!(
            init(&root, "x", "X"),
            Err(Problem::TaskbagExists {
                path: root.display().to_string()
            })
        );
        let loaded = load(&root);
        assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
        let bag = loaded.taskbag.unwrap();
        assert_eq!(bag.project.id, "my-repo");
        assert!(bag.areas.is_empty() && bag.tasks.is_empty());
        assert!(
            fs::read_to_string(root.join(".gitignore"))
                .unwrap()
                .contains(".lock")
        );
        let deep = repo.join("src/a/b");
        fs::create_dir_all(&deep).unwrap();
        assert_eq!(find_taskbag(&deep), Some(root.clone()));
        assert_eq!(find_taskbag(&std::env::temp_dir()), None);
        // No temporary folder is left next to the taskbag.
        let left: Vec<_> = fs::read_dir(repo.join(".allomos")).unwrap().collect();
        assert_eq!(left.len(), 1);
        fs::remove_dir_all(repo).unwrap();
    }

    #[test]
    fn id_from_name_keeps_ids_valid() {
        assert_eq!(id_from_name("allomos"), "allomos");
        assert_eq!(id_from_name("我的"), "project");
        assert_eq!(id_from_name(&"a".repeat(80)).len(), 64);
    }

    #[test]
    fn writes_go_through_the_lock() {
        let dir = tmp("lock");
        let root = dir.join(DEFAULT_DIR);
        assert!(matches!(
            Lock::acquire(&root, Duration::ZERO),
            Err(Problem::TaskbagNotFound { .. })
        ));
        init(&root, "p", "P").unwrap();
        let before = stamp(&root);
        let lock = Lock::acquire(&root, Duration::ZERO).unwrap();
        // A second writer waits, then gives up while the first holds the lock.
        assert!(matches!(
            Lock::acquire(&root, Duration::from_millis(20)),
            Err(Problem::TaskbagBusy { .. })
        ));
        let bag = lock.load().taskbag.unwrap();
        let area = allomos_core::ops::new_area(&bag, "A01", "First").unwrap();
        lock.create_area(&area).unwrap();
        assert!(matches!(
            lock.create_area(&area),
            Err(Problem::DuplicateAreaId { .. })
        ));
        let bag = lock.load().taskbag.unwrap();
        let task =
            allomos_core::ops::new_task(&bag, "A01-01", "A01", "Do it", Some("Why")).unwrap();
        lock.create_task(&task).unwrap();
        assert!(matches!(
            lock.create_task(&task),
            Err(Problem::DuplicateTaskId { .. })
        ));
        let bag = lock.load().taskbag.unwrap();
        let claimed = allomos_core::ops::claim(&bag, "A01-01", Some("me"), false)
            .unwrap()
            .task;
        lock.save_task(&claimed).unwrap();
        drop(lock);
        assert!(Lock::acquire(&root, Duration::ZERO).is_ok());

        let loaded = load(&root);
        assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
        let t = &loaded.taskbag.unwrap().tasks[0];
        assert_eq!(t.holder.as_deref(), Some("me"));
        assert_eq!(t.description, "Why\n");
        let json = fs::read_to_string(root.join("tasks/A01-01/task.json")).unwrap();
        assert!(!json.contains("null") && !json.contains("description"));
        assert_ne!(stamp(&root), before);
        // Only the lock and the taskbag's own files: no temporary files left behind.
        let hidden: Vec<_> = fs::read_dir(&root)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|n| n.starts_with(TMP_PREFIX))
            .collect();
        assert!(hidden.is_empty(), "{hidden:?}");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn iterations_are_written_read_and_sorted() {
        use allomos_core::ops::{self, IterationDetails};
        let dir = tmp("iterations");
        let root = dir.join(DEFAULT_DIR);
        init(&root, "p", "P").unwrap();
        // A taskbag made before iterations existed has no `iterations/` folder: still valid.
        assert!(!root.join("iterations").exists());
        let loaded = load(&root);
        assert!(loaded.issues.is_empty() && loaded.taskbag.unwrap().iterations.is_empty());

        let before = stamp(&root);
        let lock = Lock::acquire(&root, Duration::ZERO).unwrap();
        let bag = lock.load().taskbag.unwrap();
        let two = ops::new_iteration(
            &bag,
            "I02",
            "v0.2",
            IterationDetails {
                release: Some("v0.2"),
                ..Default::default()
            },
        )
        .unwrap();
        lock.create_iteration(&two).unwrap();
        assert!(matches!(
            lock.create_iteration(&two),
            Err(Problem::DuplicateIterationId { .. })
        ));
        // Written by hand with a lower order: listed first.
        fs::write(
            root.join("iterations/I01.json"),
            r#"{"id":"I01","name":"v0.1","order":0}"#,
        )
        .unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.create_area(&ops::new_area(&bag, "A01", "First").unwrap())
            .unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.create_task(&ops::new_task(&bag, "A01-01", "A01", "Do it", None).unwrap())
            .unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.save_task(&ops::plan(&bag, "A01-01", Some("I02")).unwrap().task)
            .unwrap();
        drop(lock);

        let json = fs::read_to_string(root.join("iterations/I02.json")).unwrap();
        assert!(!json.contains("null") && !json.contains("start"), "{json}");
        let loaded = load(&root);
        assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
        let bag = loaded.taskbag.unwrap();
        let ids: Vec<_> = bag.iterations.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(ids, ["I01", "I02"]);
        assert_eq!(bag.tasks[0].iteration.as_deref(), Some("I02"));
        assert_ne!(stamp(&root), before);

        // A task planned in a missing iteration is reported, and the task still shows.
        fs::remove_file(root.join("iterations/I02.json")).unwrap();
        let loaded = load(&root);
        assert_eq!(loaded.issues[0].problem.code(), "unknown_iteration");
        assert_eq!(loaded.taskbag.unwrap().tasks.len(), 1);
        fs::remove_dir_all(dir).unwrap();
    }

    /// Every text file under `dir` rewritten with `\r\n` line endings, the way Git on Windows
    /// may check a taskbag out.
    fn to_crlf(dir: &Path) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                to_crlf(&path);
            } else if let Ok(text) = fs::read_to_string(&path) {
                fs::write(&path, text.replace('\n', "\r\n")).unwrap();
            }
        }
    }

    #[test]
    fn a_taskbag_checked_out_with_crlf_loads_and_is_written_back_with_lf() {
        let dir = tmp("crlf");
        let root = default_root(&dir);
        init(&root, "p", "P").unwrap();
        let lock = Lock::acquire(&root, Duration::ZERO).unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.create_area(&ops::new_area(&bag, "A01", "First").unwrap())
            .unwrap();
        let bag = lock.load().taskbag.unwrap();
        let mut task =
            ops::new_task(&bag, "A01-01", "A01", "Do it", Some("Why\nand how\n")).unwrap();
        lock.create_task(&task).unwrap();
        drop(lock);

        to_crlf(&root);
        assert!(
            fs::read_to_string(root.join("project.json"))
                .unwrap()
                .contains("\r\n")
        );
        let loaded = load(&root);
        assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
        let bag = loaded.taskbag.unwrap();
        assert_eq!(bag.tasks[0].description, "Why\nand how\n");

        // What Allomos writes back has `\n` only, even from text that came with `\r\n`.
        let lock = Lock::acquire(&root, Duration::ZERO).unwrap();
        task.holder = Some("me".into());
        lock.save_task(&task).unwrap();
        lock.create_task(&Task {
            id: "A01-02".into(),
            description: "One\r\nTwo\r\n".into(),
            ..task.clone()
        })
        .unwrap();
        drop(lock);
        for file in ["tasks/A01-01/task.json", "tasks/A01-02/description.md"] {
            let text = fs::read_to_string(root.join(file)).unwrap();
            assert!(!text.contains('\r'), "{file}: {text:?}");
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_new_taskbag_keeps_lf_line_endings_in_git() {
        let dir = tmp("attributes");
        let root = default_root(&dir);
        init(&root, "p", "P").unwrap();
        let text = fs::read_to_string(root.join(".gitattributes")).unwrap();
        assert!(text.contains("*.json text eol=lf"), "{text}");
        assert!(text.contains("*.md text eol=lf"), "{text}");
        create_starter(&dir.join("starter"), "S").unwrap();
        assert!(dir.join("starter/.gitattributes").is_file());
        fs::remove_dir_all(dir).unwrap();
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

    #[test]
    fn replacing_a_file_that_is_open_for_reading_succeeds() {
        let dir = tmp("open-reader");
        let root = default_root(&dir);
        init(&root, "p", "P").unwrap();
        let lock = Lock::acquire(&root, Duration::ZERO).unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.create_area(&ops::new_area(&bag, "A01", "First").unwrap())
            .unwrap();
        let bag = lock.load().taskbag.unwrap();
        let mut task = ops::new_task(&bag, "A01-01", "A01", "Do it", None).unwrap();
        lock.create_task(&task).unwrap();
        let path = root.join("tasks/A01-01/task.json");
        let before = fs::read_to_string(&path).unwrap();

        // Another reader has the file open while it is replaced.
        let mut reader = fs::File::open(&path).unwrap();
        task.holder = Some("me".into());
        lock.save_task(&task).unwrap();
        let mut seen = String::new();
        io::Read::read_to_string(&mut reader, &mut seen).unwrap();
        // The reader sees the old file whole, or (where the system shares it) the new one whole.
        let after = fs::read_to_string(&path).unwrap();
        assert!(seen == before || seen == after, "{seen}");
        assert!(after.contains("\"holder\": \"me\""), "{after}");
        drop(reader);
        drop(lock);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn replacing_retries_while_the_file_is_in_use_then_gives_up() {
        let in_use = || io::Error::from(io::ErrorKind::PermissionDenied);
        // In use twice, then free: it works.
        let mut tries = 0;
        let result = retry_in_use(Duration::from_secs(1), Duration::ZERO, || {
            tries += 1;
            if tries < 3 { Err(in_use()) } else { Ok(tries) }
        });
        assert_eq!(result.unwrap(), 3);
        // Another failure is not retried.
        let mut tries = 0;
        let result: io::Result<()> = retry_in_use(Duration::from_secs(1), Duration::ZERO, || {
            tries += 1;
            Err(io::Error::from(io::ErrorKind::NotFound))
        });
        assert!(result.is_err());
        assert_eq!(tries, 1);
        // Still in use when the time is up: the failure is returned.
        let result: io::Result<()> =
            retry_in_use(Duration::from_millis(20), Duration::from_millis(5), || {
                Err(in_use())
            });
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
    }
}
