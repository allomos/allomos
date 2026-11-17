//! `alm confirm`: asking a person in the Allomos app and waiting for the answer.

use std::path::{Path, PathBuf};
use std::time::Duration;

use allomos_core::Problem;
use allomos_store::{
    Actor, ConfirmExtra, MAX_PLAN_BYTES, default_root, find_taskbag, load, picked_folder,
    post_confirm, read_answer, read_text, requests_dir, taskbag_of, was_seen, withdraw_confirm,
};
use serde_json::{Value, json};

use crate::{Cli, Done, locate, lock_wait};

/// The project folder `alm confirm` asks about: the folder holding the taskbag given with
/// `--taskbag` (which may not exist yet), else the nearest taskbag's, else `cwd`.
pub(crate) fn confirm_project(cli: &Cli, cwd: &Path) -> PathBuf {
    let taskbag = match &cli.taskbag {
        Some(path) => cwd.join(path),
        None => find_taskbag(cwd).unwrap_or_else(|| default_root(cwd)),
    };
    picked_folder(&taskbag)
        .map(Path::to_path_buf)
        .unwrap_or(taskbag)
}

/// How often `alm confirm` looks for the answer, and how long it gives an open app to show the
/// question before opening the app itself.
pub(crate) const CONFIRM_POLL: Duration = Duration::from_millis(250);

pub(crate) const CONFIRM_LAUNCH_AFTER: Duration = Duration::from_secs(3);

/// Open the Allomos app on `project`, so it can show the question: on macOS by its bundle ID, on
/// Windows the app beside `alm.exe`, where the installer puts both (Task-A07-06).
/// `ALLOMOS_NO_LAUNCH` turns it off (tests).
pub(crate) fn open_app(project: &Path) {
    if std::env::var_os("ALLOMOS_NO_LAUNCH").is_some() {
        return;
    }
    if cfg!(target_os = "macos") {
        let _ = std::process::Command::new("open")
            .args(["-n", "-b", "io.allomos", "--args"])
            .arg(project)
            .status();
    } else if cfg!(windows) {
        if let Some(app) = windows_app() {
            // Not waited for: the app keeps running after the question is answered.
            let _ = std::process::Command::new(app).arg(project).spawn();
        }
    }
}

/// The Allomos app installed beside this `alm.exe`, if there is one.
pub(crate) fn windows_app() -> Option<PathBuf> {
    let dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    ["Allomos.exe", "allomos-desktop.exe"]
        .iter()
        .map(|name| dir.join(name))
        .find(|p| p.is_file())
}

/// `alm confirm`: leave the question, wait for the person's answer in the app, then tidy up.
pub(crate) fn confirm(
    cli: &Cli,
    cwd: &Path,
    title: &str,
    plan: &Path,
    timeout: u64,
    evidence: Option<&Path>,
    task: Option<&str>,
) -> Result<Done, Problem> {
    let project = confirm_project(cli, cwd);
    let read = |file: &Path| {
        let path = cwd.join(file);
        read_text(&path).map_err(|e| Problem::ReadFailed {
            detail: format!("{}: {e}", path.display()),
        })
    };
    let text = read(plan)?;
    let evidence = evidence.map(read).transpose()?;
    // The task must exist before the person is asked about it.
    let root = match task {
        Some(id) => {
            let root = locate(cli, cwd)?;
            allomos_core::ops::find_task(&taskbag_of(&root, load(&root))?.0, id)?;
            Some(root)
        }
        None => None,
    };
    if text.len() + evidence.as_ref().map_or(0, String::len) > MAX_PLAN_BYTES {
        return Err(Problem::BadUsage {
            detail: format!(
                "the plan is longer than {} KB; keep it to what the person must decide",
                MAX_PLAN_BYTES / 1024
            ),
        });
    }
    let asked = post_confirm(
        &project,
        title,
        &text,
        cli.holder.as_deref(),
        timeout,
        ConfirmExtra {
            evidence: evidence.as_deref(),
            task,
        },
    )
    .map_err(|e| Problem::WriteFailed {
        path: requests_dir(&project).display().to_string(),
        detail: e.to_string(),
    })?;
    if !cli.json {
        eprintln!(
            "Waiting for the person to answer in the Allomos app (project {})…",
            project.display()
        );
    }
    let start = std::time::Instant::now();
    let mut launched = false;
    let answer = loop {
        if let Some(answer) = read_answer(&project, &asked.id, &asked.fingerprint) {
            break Some(answer);
        }
        if start.elapsed() >= Duration::from_secs(timeout) {
            break None;
        }
        if !launched && start.elapsed() >= CONFIRM_LAUNCH_AFTER && !was_seen(&project, &asked.id) {
            launched = true;
            open_app(&project);
        }
        std::thread::sleep(CONFIRM_POLL);
    };
    withdraw_confirm(&project, &asked.id);
    match answer {
        Some(true) => {
            let mut out = format!(
                "The person chose Go ahead, for exactly this plan (fingerprint {}). If the plan changes, ask again.\n",
                asked.fingerprint
            );
            let mut approval = Value::Null;
            if let (Some(root), Some(id)) = (&root, task) {
                let approved = allomos_store::record_approval(
                    root,
                    Actor::alm(cli.holder.as_deref()),
                    lock_wait(),
                    id,
                    title,
                    &plan.to_string_lossy(),
                    &text,
                )?;
                out.push_str(&format!(
                    "Recorded in {}'s history as approval {}: pass `--approval {}` on the changes that rely on it.\n",
                    id, approved.approval.id, approved.approval.id
                ));
                approval = json!(approved.approval);
            }
            Ok(Done::new(
                json!({ "answer": "yes", "project": project, "fingerprint": asked.fingerprint, "task": task, "approval": approval }),
                out,
            ))
        }
        Some(false) => Err(Problem::ConfirmDeclined),
        None => Err(Problem::ConfirmTimeout {
            seconds: timeout.to_string(),
        }),
    }
}
