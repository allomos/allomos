//! Where this computer keeps its monitoring runs (Task-A03-cow112, approver 2026-11-09): one small
//! file per run in one fixed folder, `monitors/` in Allomos's settings folder
//! (`~/Library/Application Support/io.allomos/monitors/` on macOS, `%APPDATA%\io.allomos\monitors\`
//! on Windows), never in a project or the taskbag. Beside it:
//!
//! - `<run>.json`: the run as its process last wrote it (rewritten on every change and about every
//!   30 seconds; removed when the run ends);
//! - `<run>.lock`: held with the system's file lock for as long as the process lives, so whether a
//!   run is alive is asked of the system (the lock is let go however the process ends), never
//!   guessed from how old its file is;
//! - `<run>.log`: what the process printed;
//! - `<run>.req-<n>.json`: a request (check now, stop…) left for the process, which takes it within
//!   about a second;
//! - `<run>.note-<n>.json`: a notification the process leaves for the app to show under its own
//!   name (A03-stork103); the app takes and removes each one ([`take_notes`]).
//! - `resume/<run>.json`: the run's last record, kept while the run goes on and after its process
//!   died without ending (a log-out, a restart, a crash); the app and `alm monitor resume` start
//!   such a run again from its saved settings when the project opens (A03-pocket537). Removed
//!   when the run ends as asked, or when it is ended by hand ([`forget_process`]).
//! - `last/<hash>.json`: the last run of one taskbag ([`LastSummary`]), kept when it ends, so the
//!   app's "last run" line counts a run that ended while the app was closed (A03-pocket537).
//!
//! `alm monitor status` and `list`, and the app, only read these files. As a last resort the
//! system's process list is searched for `alm monitor run` processes that no file names ("running,
//! record missing"); only those are ended directly, since they cannot see a request.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use fs4::{FileExt, TryLockError};
use serde::{Deserialize, Serialize};

use crate::flow::{Run, Scope};

/// The run file's format.
pub const FORMAT: u32 = 1;

/// The folder of this computer's runs: `monitors/` in Allomos's settings folder.
pub fn dir() -> Option<PathBuf> {
    allomos_store::settings_dir().map(|d| d.join("monitors"))
}

/// What a run's file holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub format: u32,
    /// The run's ID (the file's name).
    pub id: String,
    /// The process that runs it.
    pub pid: u32,
    /// The taskbag it watches, and its project folder.
    pub taskbag: PathBuf,
    pub project: PathBuf,
    /// The language its requests and notifications are written in.
    pub language: String,
    /// Where its process writes what it prints.
    pub log: PathBuf,
    /// When the process last said it is alive (ms since 1970).
    pub alive: i64,
    pub run: Run,
}

/// The paths of one run's files in `dir`.
pub fn record_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.json"))
}
pub fn lock_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.lock"))
}
pub fn log_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.log"))
}

/// A run ID: letters, digits and `-` only (it names files and goes on a command line).
pub fn plain_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// A new run ID: `m-` and the time, with a few random letters so two started in one second differ.
pub fn new_id(now_ms: i64) -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u32(std::process::id());
    format!("m-{:x}-{:04x}", now_ms / 1000, h.finish() & 0xffff)
}

/// Write `text` to `path` whole: to a file beside it first, then renamed over it, so a reader
/// never sees half a file.
pub fn write_whole(path: &Path, text: &str) -> io::Result<()> {
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    fs::write(&tmp, text)?;
    fs::rename(&tmp, path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

/// The lock a running process holds on its run.
pub struct Held {
    path: PathBuf,
    file: fs::File,
}

fn open_lock(path: &Path) -> io::Result<fs::File> {
    fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)
}

impl Held {
    /// Take the run's lock; `None` when another process holds it (the run is already running).
    pub fn take(path: &Path) -> io::Result<Option<Held>> {
        let file = open_lock(path)?;
        match FileExt::try_lock(&file) {
            Ok(()) => Ok(Some(Held {
                path: path.to_path_buf(),
                file,
            })),
            Err(TryLockError::WouldBlock) => Ok(None),
            Err(TryLockError::Error(e)) => Err(e),
        }
    }

    /// Make sure the lock is still on the file at its path: a lock file someone removed (on macOS
    /// and Linux the lock then guards nothing anyone can find) is made and locked again.
    pub fn keep(&mut self) {
        if same_file(&self.file, &self.path) {
            return;
        }
        if let Ok(Some(again)) = Held::take(&self.path) {
            *self = again;
        }
    }

    /// Let the lock go and remove its file.
    pub fn release(self) {
        let _ = fs::remove_file(&self.path);
        drop(self.file);
    }
}

#[cfg(unix)]
fn same_file(file: &fs::File, path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (file.metadata(), fs::metadata(path)) {
        (Ok(a), Ok(b)) => a.dev() == b.dev() && a.ino() == b.ino(),
        _ => false,
    }
}

#[cfg(not(unix))]
fn same_file(_file: &fs::File, path: &Path) -> bool {
    // Windows does not let a file that is open be removed: it is there while it is held.
    path.exists()
}

/// Whether some process holds the lock at `path`. Asked by trying to take it and letting it go
/// at once; a missing lock file is held by no one.
pub fn held(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }
    let Ok(file) = fs::OpenOptions::new().write(true).open(path) else {
        return false;
    };
    match FileExt::try_lock(&file) {
        Ok(()) => {
            let _ = FileExt::unlock(&file);
            false
        }
        Err(TryLockError::WouldBlock) => true,
        Err(TryLockError::Error(_)) => false,
    }
}

/// What can be asked of a running run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Request {
    /// Check now: let finished blockers' tasks go on, then give free agents the ready tasks.
    Check,
    /// Start no new session; the run ends once the sessions at work have finished.
    Stop,
    /// Take back a stop while sessions are still at work.
    UndoStop,
    /// A new frequency, in minutes.
    Every { minutes: u32 },
    /// These agents from now on.
    Agents { agents: Vec<String> },
    /// How many sessions one agent may have at work at once, from the next check on
    /// (Task-A03-weasel276): at least one.
    Sessions { agent: String, count: u32 },
    /// The difficulty range one agent takes tasks from, from the next check on, as `1-3` or `4`
    /// (Task-A03-wave335).
    Difficulty { agent: String, range: String },
    /// Which tasks the run takes, from the next check on (A03-yolk481): every ready task or one
    /// iteration's. The sessions at work finish their tasks.
    Scope { scope: Scope },
    /// The run mode the sessions started from now on are given (A03-yolk481): quiet or normal.
    /// The sessions at work keep the mode they were started with.
    Mode { quiet: bool },
}

/// Leave `request` for run `id` in `dir`; its process takes it within about a second.
pub fn ask(dir: &Path, id: &str, request: &Request) -> io::Result<()> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let path = dir.join(format!("{id}.req-{stamp:032}.json"));
    write_whole(&path, &serde_json::to_string(request).unwrap_or_default())
}

/// The requests left for run `id`, oldest first, each removed as it is taken. One that cannot be
/// read is removed and left out.
pub fn take_requests(dir: &Path, id: &str) -> Vec<Request> {
    let prefix = format!("{id}.req-");
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(&prefix) && n.ends_with(".json"))
        })
        .collect();
    paths.sort();
    paths
        .into_iter()
        .filter_map(|p| {
            let text = fs::read_to_string(&p).ok();
            let _ = fs::remove_file(&p);
            serde_json::from_str(&text?).ok()
        })
        .collect()
}

/// A notification for the person, left by a run for the app to show (A03-stork103).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    pub title: String,
    pub body: String,
}

/// Leave `note` for the app, from run `id` in `dir`. The app shows it and removes it; while no
/// app runs, it waits there.
pub fn leave_note(dir: &Path, id: &str, note: &Note) -> io::Result<()> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let path = dir.join(format!("{id}.note-{stamp:032}.json"));
    write_whole(&path, &serde_json::to_string(note).unwrap_or_default())
}

/// The notifications left in `dir` by any run, oldest first, each removed as it is taken. One
/// that cannot be read is removed and left out.
pub fn take_notes(dir: &Path) -> Vec<Note> {
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.contains(".note-") && n.ends_with(".json"))
        })
        .collect();
    paths.sort();
    paths
        .into_iter()
        .filter_map(|p| {
            let text = fs::read_to_string(&p).ok();
            let _ = fs::remove_file(&p);
            serde_json::from_str(&text?).ok()
        })
        .collect()
}

/// One run this computer is running.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Entry {
    /// A run with its file.
    Running(Box<Record>),
    /// An `alm monitor run` process no run file names: running, record missing.
    Missing {
        pid: u32,
        id: Option<String>,
        /// The taskbag its command line names, when it names one.
        taskbag: Option<PathBuf>,
    },
}

impl Entry {
    pub fn id(&self) -> Option<&str> {
        match self {
            Entry::Running(r) => Some(&r.id),
            Entry::Missing { id, .. } => id.as_deref(),
        }
    }
    /// As `alm monitor status --json` and the app read it: the run file's fields with
    /// `"record": "present"` and its `next_check` (ms), or `"record": "missing"` with the process.
    pub fn json(&self) -> serde_json::Value {
        match self {
            Entry::Missing { pid, id, taskbag } => {
                serde_json::json!({ "id": id, "pid": pid, "taskbag": taskbag, "record": "missing" })
            }
            Entry::Running(r) => {
                let mut v = serde_json::to_value(r.as_ref()).unwrap_or_default();
                v["record"] = serde_json::json!("present");
                v["next_check"] = serde_json::json!(r.run.next_check());
                v
            }
        }
    }

    pub fn pid(&self) -> u32 {
        match self {
            Entry::Running(r) => r.pid,
            Entry::Missing { pid, .. } => *pid,
        }
    }
}

/// Every run in `dir` that is alive, oldest first. A run whose lock nobody holds and whose
/// process is not to be found has ended without tidying up: its files are removed. With
/// `processes` (from [`monitor_processes`]), `alm monitor run` processes no file names are added
/// as [`Entry::Missing`].
pub fn list(dir: &Path, processes: Option<&[Process]>) -> Vec<Entry> {
    let mut records: Vec<Record> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension().is_some_and(|x| x == "json")
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| !n.contains(".req-") && !n.contains(".note-"))
        })
        .filter_map(|p| serde_json::from_str::<Record>(&fs::read_to_string(&p).ok()?).ok())
        .collect();
    records.sort_by(|a, b| a.run.started.cmp(&b.run.started).then(a.id.cmp(&b.id)));
    let mut out = Vec::new();
    let mut found_procs: Option<Vec<Process>> = processes.map(<[_]>::to_vec);
    for r in records {
        let alive = held(&lock_path(dir, &r.id)) || {
            // The lock is not held: perhaps its file was removed and the process has not made it
            // again yet. Its process, by number and run ID, settles it.
            let procs = found_procs.get_or_insert_with(monitor_processes);
            procs
                .iter()
                .any(|p| p.pid == r.pid && p.id.as_deref() == Some(r.id.as_str()))
        };
        if alive {
            out.push(Entry::Running(Box::new(r)));
        } else {
            let _ = fs::remove_file(record_path(dir, &r.id));
            let _ = fs::remove_file(lock_path(dir, &r.id));
            for leftover in take_requests(dir, &r.id) {
                drop(leftover);
            }
        }
    }
    // A lock file left by a run whose record is gone and whose lock nobody holds.
    for path in fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
    {
        let Some(id) = path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_suffix(".lock"))
        else {
            continue;
        };
        if !record_path(dir, id).exists() && !held(&path) {
            let _ = fs::remove_file(&path);
        }
    }
    if let Some(procs) = processes {
        for p in procs {
            let known = out.iter().any(|e| match e {
                Entry::Running(r) => r.pid == p.pid || p.id.as_deref() == Some(r.id.as_str()),
                Entry::Missing { .. } => false,
            });
            if !known && p.pid != std::process::id() {
                out.push(Entry::Missing {
                    pid: p.pid,
                    id: p.id.clone(),
                    taskbag: p.taskbag.clone(),
                });
            }
        }
    }
    out
}

/// The same folder, however it was written.
pub fn same_place(a: &Path, b: &Path) -> bool {
    let tidy = |p: &Path| fs::canonicalize(p).unwrap_or_else(|_| allomos_store::tidy(p));
    tidy(a) == tidy(b)
}

/// The runs of the taskbag at `root`, with the record-missing processes whose command line names
/// it (or names no taskbag).
pub fn of_project(dir: &Path, root: &Path) -> Vec<Entry> {
    of_project_with(dir, root, &monitor_processes())
}

/// [`of_project`] with a process list already read, for example by [`recent_processes`].
pub fn of_project_with(dir: &Path, root: &Path, procs: &[Process]) -> Vec<Entry> {
    list(dir, Some(procs))
        .into_iter()
        .filter(|e| match e {
            Entry::Running(r) => same_place(&r.taskbag, root),
            Entry::Missing { taskbag, .. } => taskbag.as_ref().is_none_or(|t| same_place(t, root)),
        })
        .collect()
}

/// The run ID in an `alm monitor run [<id>] …` command line, as its words; `None` when the words
/// are not such a command, `Some(None)` when it names no run ID.
pub fn run_words<'a>(words: &[&'a str]) -> Option<Option<&'a str>> {
    let program = Path::new(words.first()?)
        .file_stem()
        .and_then(|s| s.to_str())?;
    if program != "alm" && program != "allomos" {
        return None;
    }
    let at = words.windows(2).position(|w| w == ["monitor", "run"])?;
    Some(words.get(at + 2).copied().filter(|w| plain_id(w)))
}

/// An `alm monitor run` process on this computer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Process {
    pub pid: u32,
    /// The run ID its command line names.
    pub id: Option<String>,
    /// The taskbag its command line names (`--taskbag`).
    pub taskbag: Option<PathBuf>,
}

/// The `--taskbag` its words name: the words after it up to the next option (a path may hold
/// spaces).
pub fn taskbag_words(words: &[&str]) -> Option<PathBuf> {
    let at = words.iter().position(|w| *w == "--taskbag")?;
    let path: Vec<&str> = words[at + 1..]
        .iter()
        .take_while(|w| !w.starts_with("--"))
        .copied()
        .collect();
    (!path.is_empty()).then(|| PathBuf::from(path.join(" ")))
}

/// The `alm monitor run` processes on this computer.
pub fn monitor_processes() -> Vec<Process> {
    process_lines()
        .iter()
        .filter_map(|line| {
            let line = line.trim();
            let (pid, rest) = line.split_once(char::is_whitespace)?;
            let pid: u32 = pid.trim().parse().ok()?;
            let words: Vec<&str> = rest.split_whitespace().collect();
            let id = run_words(&words)?;
            Some(Process {
                pid,
                id: id.map(String::from),
                taskbag: taskbag_words(&words),
            })
        })
        .collect()
}

/// The `alm monitor run` processes, read again only when the last reading is older than
/// `max_age` (Task-A03-reef286). Reading the system's process list starts a program (`ps`), and
/// the app asks for it from several places at once (the menu, each window's runs, the Monitors
/// bar), so these share one reading. Use [`monitor_processes`] where a fresh answer matters (after
/// ending a process).
pub fn recent_processes(max_age: std::time::Duration) -> Vec<Process> {
    use std::sync::Mutex;
    use std::time::Instant;
    static LAST: Mutex<Option<(Instant, Vec<Process>)>> = Mutex::new(None);
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((at, procs)) = last.as_ref() {
        if at.elapsed() <= max_age {
            return procs.clone();
        }
    }
    let procs = monitor_processes();
    *last = Some((Instant::now(), procs.clone()));
    procs
}

/// Each process as `<number> <command line>`.
fn process_lines() -> Vec<String> {
    let out = if cfg!(windows) {
        crate::terminal::command("powershell.exe")
            .args([
                "-NoProfile",
                "-Command",
                "Get-CimInstance Win32_Process -Filter \"Name like 'alm%' or Name like 'allomos%'\" | ForEach-Object { \"$($_.ProcessId) $($_.CommandLine)\" }",
            ])
            .output()
    } else {
        crate::terminal::command("/bin/ps")
            .args(["-axww", "-o", "pid=,args="])
            .output()
    };
    out.map(|o| {
        String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(|l| l.replace('"', ""))
            .collect()
    })
    .unwrap_or_default()
}

/// The folder of the runs' resume files (A03-pocket537): each run's last record, kept while the
/// run is alive and after its process died without ending, so the app can start it again from
/// its saved settings when the project is opened. Removed when the run ends as asked.
fn resume_dir(dir: &Path) -> PathBuf {
    dir.join("resume")
}

/// The resume file of run `id`.
pub fn resume_path(dir: &Path, id: &str) -> PathBuf {
    resume_dir(dir).join(format!("{id}.json"))
}

/// Keep `record` as run `id`'s resume file.
pub fn write_resume(dir: &Path, record: &Record) -> io::Result<()> {
    fs::create_dir_all(resume_dir(dir))?;
    let text = serde_json::to_string_pretty(record).map_err(io::Error::other)?;
    write_whole(&resume_path(dir, &record.id), &text)
}

/// Run `id`'s resume file, when it has one.
pub fn read_resume(dir: &Path, id: &str) -> Option<Record> {
    if !plain_id(id) {
        return None;
    }
    serde_json::from_str(&fs::read_to_string(resume_path(dir, id)).ok()?).ok()
}

/// Forget run `id` for good: its resume file goes, so the app does not start it again.
pub fn remove_resume(dir: &Path, id: &str) {
    if plain_id(id) {
        let _ = fs::remove_file(resume_path(dir, id));
    }
}

/// Forget the run of process `pid` for good, when a run file names it (a run ended by hand).
pub fn forget_process(dir: &Path, pid: u32) {
    if let Some(id) = monitor_processes()
        .into_iter()
        .find(|p| p.pid == pid)
        .and_then(|p| p.id)
    {
        remove_resume(dir, &id);
    }
}

/// The runs of the taskbag at `root` that stopped without ending (their process was ended by a
/// log-out, a restart or a crash) and can be started again from their resume files. A run whose
/// lock is held, or whose process is alive, is not one of them.
pub fn resumable(dir: &Path, root: &Path, processes: &[Process]) -> Vec<Record> {
    let mut out: Vec<Record> = fs::read_dir(resume_dir(dir))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .filter_map(|p| serde_json::from_str::<Record>(&fs::read_to_string(&p).ok()?).ok())
        .filter(|r| {
            same_place(&r.taskbag, root)
                && plain_id(&r.id)
                && !held(&lock_path(dir, &r.id))
                && !processes
                    .iter()
                    .any(|p| p.id.as_deref() == Some(r.id.as_str()))
        })
        .collect();
    out.sort_by(|a, b| a.run.started.cmp(&b.run.started).then(a.id.cmp(&b.id)));
    out
}

/// What a finished run leaves for the app's "last run" line (A03-pocket537): kept per taskbag, so
/// a run that ended while the app was closed still counts. Field names as the app reads them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LastSummary {
    /// When it ended (ms since 1970).
    pub ended: i64,
    pub scope: crate::flow::Scope,
    pub taken: Vec<String>,
    #[serde(default)]
    pub taken_by: std::collections::BTreeMap<String, String>,
}

/// The file of the last run of the taskbag at `root`: a name made from its folder's path.
fn last_path(dir: &Path, root: &Path) -> PathBuf {
    let place = fs::canonicalize(root).unwrap_or_else(|_| allomos_store::tidy(root));
    // FNV-1a: a fixed hash, so the name is the same in every build of alm and the app.
    let hash = place
        .display()
        .to_string()
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
        });
    dir.join("last").join(format!("{hash:016x}.json"))
}

/// Keep `last` as the last run of the taskbag at `root`.
pub fn write_last(dir: &Path, root: &Path, last: &LastSummary) -> io::Result<()> {
    let path = last_path(dir, root);
    fs::create_dir_all(path.parent().unwrap_or(dir))?;
    let text = serde_json::to_string_pretty(last).map_err(io::Error::other)?;
    write_whole(&path, &text)
}

/// The last run of the taskbag at `root`, when one ended on this computer.
pub fn read_last(dir: &Path, root: &Path) -> Option<LastSummary> {
    serde_json::from_str(&fs::read_to_string(last_path(dir, root)).ok()?).ok()
}

/// End process `pid` (a run whose record is missing).
pub fn end_process(pid: u32) -> io::Result<()> {
    #[cfg(unix)]
    {
        // SAFETY: `kill` takes a process number and a signal; it touches no memory of ours.
        let r = unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) };
        // A paused process ends only once it goes on.
        // SAFETY: as above.
        unsafe { libc::kill(pid as libc::pid_t, libc::SIGCONT) };
        if r == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
    #[cfg(not(unix))]
    {
        let status = crate::terminal::command("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other("taskkill failed"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("allomos-monitors-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn a_run(id: &str, root: &Path, dir: &Path) -> Record {
        Record {
            format: FORMAT,
            id: id.to_string(),
            pid: 999_999_999,
            taskbag: root.to_path_buf(),
            project: dir.to_path_buf(),
            language: "en".to_string(),
            log: log_path(dir, id),
            alive: 1,
            run: Run::start(
                crate::flow::Scope::All,
                vec!["claude_cli".to_string()],
                vec!["claude_cli".to_string()],
                true,
                30,
                1_000,
            ),
        }
    }

    #[test]
    fn a_run_that_stopped_without_ending_resumes_until_it_is_forgotten() {
        let d = temp("resume");
        let root = d.join("tb");
        fs::create_dir_all(&root).unwrap();
        let record = a_run("m-1", &root, &d);
        write_resume(&d, &record).unwrap();
        let found = resumable(&d, &root, &[]);
        assert_eq!(found, [record.clone()]);
        // Another project's run is not this project's.
        assert!(resumable(&d, &d.join("other"), &[]).is_empty());
        // A run whose process is still alive is left alone.
        let alive = [Process {
            pid: 5,
            id: Some("m-1".to_string()),
            taskbag: None,
        }];
        assert!(resumable(&d, &root, &alive).is_empty());
        // Ended by hand: not started again.
        remove_resume(&d, "m-1");
        assert!(resumable(&d, &root, &[]).is_empty());
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn a_held_lock_keeps_a_run_out_of_the_resumable_ones() {
        let d = temp("resume-lock");
        let root = d.join("tb");
        fs::create_dir_all(&root).unwrap();
        write_resume(&d, &a_run("m-2", &root, &d)).unwrap();
        let h = Held::take(&lock_path(&d, "m-2")).unwrap().unwrap();
        assert!(resumable(&d, &root, &[]).is_empty());
        h.release();
        assert_eq!(resumable(&d, &root, &[]).len(), 1);
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn the_last_run_is_kept_per_taskbag_in_the_app_s_names() {
        let d = temp("last");
        let root = d.join("tb");
        fs::create_dir_all(&root).unwrap();
        assert!(read_last(&d, &root).is_none());
        let last = LastSummary {
            ended: 42,
            scope: crate::flow::Scope::All,
            taken: vec!["A1".to_string()],
            taken_by: std::collections::BTreeMap::from([(
                "A1".to_string(),
                "claude_cli".to_string(),
            )]),
        };
        write_last(&d, &root, &last).unwrap();
        assert_eq!(read_last(&d, &root), Some(last.clone()));
        assert_eq!(
            serde_json::to_value(&last).unwrap()["takenBy"]["A1"],
            "claude_cli"
        );
        assert!(read_last(&d, &d.join("other")).is_none());
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn a_held_lock_is_seen_and_a_let_go_one_is_not() {
        let d = temp("lock");
        let path = lock_path(&d, "m-1");
        assert!(!held(&path));
        let mut h = Held::take(&path).unwrap().unwrap();
        assert!(Held::take(&path).unwrap().is_none());
        // Its file removed: kept again at the same path.
        fs::remove_file(&path).unwrap();
        h.keep();
        assert!(path.exists());
        h.release();
        assert!(!path.exists());
        assert!(!held(&path));
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn requests_are_taken_in_order_once() {
        let d = temp("req");
        ask(&d, "m-1", &Request::Check).unwrap();
        ask(&d, "m-1", &Request::Every { minutes: 5 }).unwrap();
        ask(&d, "m-2", &Request::Stop).unwrap();
        assert_eq!(
            take_requests(&d, "m-1"),
            [Request::Check, Request::Every { minutes: 5 }]
        );
        assert!(take_requests(&d, "m-1").is_empty());
        assert_eq!(take_requests(&d, "m-2"), [Request::Stop]);
        fs::remove_dir_all(&d).unwrap();
    }

    /// A03-yolk481: what the app sends to change the scope and the run mode.
    #[test]
    fn scope_and_mode_requests_read_as_the_app_writes_them() {
        let scope: Request =
            serde_json::from_str(r#"{"action":"scope","scope":{"kind":"iteration","id":"I-2"}}"#)
                .unwrap();
        assert_eq!(
            scope,
            Request::Scope {
                scope: Scope::Iteration { id: "I-2".into() }
            }
        );
        let all: Request =
            serde_json::from_str(r#"{"action":"scope","scope":{"kind":"all"}}"#).unwrap();
        assert_eq!(all, Request::Scope { scope: Scope::All });
        let mode: Request = serde_json::from_str(r#"{"action":"mode","quiet":false}"#).unwrap();
        assert_eq!(mode, Request::Mode { quiet: false });
    }

    #[test]
    fn notes_are_taken_once_in_order_and_are_no_run() {
        let d = temp("note");
        let note = |body: &str| Note {
            title: "t".into(),
            body: body.into(),
        };
        leave_note(&d, "m-1", &note("first")).unwrap();
        leave_note(&d, "m-2", &note("second")).unwrap();
        ask(&d, "m-1", &Request::Check).unwrap();
        assert_eq!(take_notes(&d), [note("first"), note("second")]);
        assert!(take_notes(&d).is_empty());
        // A note is not a run, and a request is not one either.
        leave_note(&d, "m-1", &note("third")).unwrap();
        assert!(list(&d, Some(&[])).is_empty());
        assert_eq!(take_requests(&d, "m-1"), [Request::Check]);
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn a_recent_reading_of_the_processes_is_shared() {
        use std::time::Duration;
        // Fresh: read now. Within the age: the same reading again, even though the system's
        // list may have changed since.
        let first = recent_processes(Duration::ZERO);
        let again = recent_processes(Duration::from_secs(3600));
        assert_eq!(first, again);
    }

    #[test]
    fn monitor_processes_are_read_from_their_command_lines() {
        assert_eq!(
            run_words(&[
                "/Applications/Allomos.app/Contents/MacOS/alm",
                "monitor",
                "run",
                "m-1a-0f",
                "--taskbag",
                "x"
            ]),
            Some(Some("m-1a-0f"))
        );
        assert_eq!(
            run_words(&["alm", "--taskbag", "x", "monitor", "run"]),
            Some(None)
        );
        assert_eq!(run_words(&["alm", "monitor", "status"]), None);
        assert_eq!(run_words(&["grep", "monitor", "run", "m-1"]), None);
        assert_eq!(
            taskbag_words(&[
                "alm",
                "monitor",
                "run",
                "m-1",
                "--taskbag",
                "/a",
                "b/tb",
                "--every",
                "5"
            ]),
            Some(PathBuf::from("/a b/tb"))
        );
    }

    #[test]
    fn a_run_with_no_one_holding_its_lock_is_tidied_away() {
        let d = temp("list");
        let record = |id: &str| Record {
            format: FORMAT,
            id: id.into(),
            pid: 999_999,
            taskbag: "/tb".into(),
            project: "/p".into(),
            language: "en".into(),
            log: log_path(&d, id),
            alive: 0,
            run: Run::start(
                crate::flow::Scope::All,
                vec!["claude_cli".into()],
                vec![],
                true,
                30,
                0,
            ),
        };
        for id in ["m-alive", "m-gone"] {
            write_whole(
                &record_path(&d, id),
                &serde_json::to_string(&record(id)).unwrap(),
            )
            .unwrap();
        }
        let h = Held::take(&lock_path(&d, "m-alive")).unwrap().unwrap();
        fs::write(lock_path(&d, "m-gone"), "").unwrap();
        let procs = [
            Process {
                pid: 4242,
                id: Some("m-lost".into()),
                taskbag: Some("/tb".into()),
            },
            Process {
                pid: 999_999,
                id: Some("m-alive".into()),
                taskbag: None,
            },
        ];
        let entries = list(&d, Some(&procs));
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id(), Some("m-alive"));
        assert_eq!(
            entries[1],
            Entry::Missing {
                pid: 4242,
                id: Some("m-lost".into()),
                taskbag: Some("/tb".into())
            }
        );
        assert!(!record_path(&d, "m-gone").exists());
        assert!(!lock_path(&d, "m-gone").exists());
        h.release();
        fs::remove_dir_all(&d).unwrap();
    }
}
