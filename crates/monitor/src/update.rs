//! After an update, runs move to the new `alm` (Task-A03-cookie729, approver 2026-11-16).
//!
//! A run is a process of its own (`alm monitor run`) that outlives the app, so updating the app
//! replaces the `alm` file on disk but not the program a running run has in memory: before this
//! task, such a run went on with the old program, and when the new one changed the taskbag's
//! format the old one could no longer read it. Now the app, when it starts, looks for runs of its
//! own `alm` (the same file) that an older version runs, ends each one and starts it again with
//! its own `alm` from the run's resume file (A03-pocket537): the same settings, and the sessions
//! it had at work, which go on undisturbed. The resumed run tells the person (a notification,
//! and its line in the status bar). It does not ask first (approver, 2026-11-16).
//!
//! Runs of another `alm` (a build from source, another install) are left alone, and so are runs
//! of a newer version.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::registry::{self, Entry, Record};

/// A version as numbers (`0.1.64` → `[0, 1, 64]`); `None` when it is not one.
fn numbers(version: &str) -> Option<Vec<u64>> {
    let core = version.split(['-', '+']).next()?;
    core.split('.').map(|n| n.parse().ok()).collect()
}

/// Whether `record` is run by `own` (the app's own `alm`, at version `version`) in an older
/// version. `command` is its process's command line, for a run started before its record named
/// its program.
pub fn outdated(record: &Record, command: Option<&str>, own: &Path, version: &str) -> bool {
    let same_program = match &record.program.path {
        Some(path) => same_file(path, own),
        None => command.is_some_and(|c| {
            let own = own.display().to_string();
            c == own || c.starts_with(&format!("{own} "))
        }),
    };
    if !same_program {
        return false;
    }
    match record.program.version.as_deref() {
        // Started before runs named their version: older than this one.
        None => true,
        Some(v) => match (numbers(v), numbers(version)) {
            (Some(theirs), Some(mine)) => theirs < mine,
            _ => v != version,
        },
    }
}

fn same_file(a: &Path, b: &Path) -> bool {
    let tidy = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    tidy(a) == tidy(b)
}

/// The runs on this computer that `own`, at `version`, should take over.
pub fn outdated_runs(dir: &Path, own: &Path, version: &str) -> Vec<Record> {
    let procs = registry::monitor_processes();
    registry::list(dir, Some(&procs))
        .into_iter()
        .filter_map(|e| match e {
            Entry::Running(r) => Some(*r),
            Entry::Missing { .. } => None,
        })
        .filter(|r| {
            let command = registry::command_of(r.pid);
            outdated(r, command.as_deref(), own, version)
        })
        .collect()
}

/// End run `record`'s process and wait until it is gone, keeping its resume file so it can be
/// started again. Its run file is removed (it names the old process). `Err` with what went wrong.
pub fn end_for_resume(dir: &Path, record: &Record) -> Result<(), String> {
    registry::end_process(record.pid)
        .map_err(|e| format!("process {} could not be ended: {e}", record.pid))?;
    let lock = registry::lock_path(dir, &record.id);
    let start = Instant::now();
    while registry::held(&lock) {
        if start.elapsed() > Duration::from_secs(10) {
            return Err(format!("process {} did not end", record.pid));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let _ = std::fs::remove_file(registry::record_path(dir, &record.id));
    if !registry::resume_path(dir, &record.id).exists() {
        return Err(format!(
            "{}: no resume file to start it again from",
            record.id
        ));
    }
    Ok(())
}

/// The taskbags of `records`, each once, in order: `alm monitor resume` starts every run of a
/// taskbag that stopped without ending.
pub fn taskbags(records: &[Record]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for r in records {
        if !out.iter().any(|t| registry::same_place(t, &r.taskbag)) {
            out.push(r.taskbag.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow::Run;
    use crate::registry::Program;

    fn record(version: Option<&str>, path: Option<&str>) -> Record {
        Record {
            format: registry::FORMAT,
            id: "m-1".into(),
            pid: 1,
            taskbag: "/tb".into(),
            project: "/p".into(),
            language: "en".into(),
            log: "/l".into(),
            alive: 0,
            run: Run::start(
                crate::flow::Scope::All,
                vec!["claude_cli".into()],
                vec![],
                true,
                30,
                0,
            ),
            program: Program {
                version: version.map(str::to_string),
                path: path.map(PathBuf::from),
                updated_from: None,
            },
        }
    }

    const OWN: &str = "/Applications/Allomos.app/Contents/MacOS/alm";

    #[test]
    fn a_run_from_before_versions_were_kept_is_known_by_its_command_line() {
        let r = record(None, None);
        let cmd = format!("{OWN} monitor run m-1 --taskbag /tb --mode quiet");
        assert!(outdated(&r, Some(&cmd), Path::new(OWN), "0.1.64"));
        // Another alm's run (a build from source) is left alone.
        let other = "/Users/x/allomos/target/debug/alm monitor run m-1 --taskbag /tb";
        assert!(!outdated(&r, Some(other), Path::new(OWN), "0.1.64"));
        // A prefix of another path is not this alm.
        let longer = format!("{OWN}-old monitor run m-1");
        assert!(!outdated(&r, Some(&longer), Path::new(OWN), "0.1.64"));
        assert!(!outdated(&r, None, Path::new(OWN), "0.1.64"));
    }

    #[test]
    fn only_an_older_version_of_this_alm_is_taken_over() {
        let own = Path::new(OWN);
        assert!(outdated(
            &record(Some("0.1.63"), Some(OWN)),
            None,
            own,
            "0.1.64"
        ));
        assert!(outdated(
            &record(Some("0.1.9"), Some(OWN)),
            None,
            own,
            "0.1.10"
        ));
        assert!(!outdated(
            &record(Some("0.1.64"), Some(OWN)),
            None,
            own,
            "0.1.64"
        ));
        assert!(!outdated(
            &record(Some("0.2.0"), Some(OWN)),
            None,
            own,
            "0.1.64"
        ));
        assert!(!outdated(
            &record(Some("0.1.63"), Some("/elsewhere/alm")),
            None,
            own,
            "0.1.64"
        ));
    }

    #[test]
    fn each_taskbag_is_resumed_once() {
        let a = record(None, None);
        let mut b = record(None, None);
        b.id = "m-2".into();
        let mut c = record(None, None);
        c.taskbag = "/other".into();
        assert_eq!(
            taskbags(&[a, b, c]),
            vec![PathBuf::from("/tb"), PathBuf::from("/other")]
        );
    }
}
