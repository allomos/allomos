//! Heartbeats in `alm` (decision 0022): `alm beat` (Task-A01-nurse164) and the line `alm show`
//! gives a task's last heartbeat.

use std::path::Path;

use allomos_core::heartbeat;
use allomos_core::{Problem, State, Task, Taskbag};
use allomos_store::Lock;
use allomos_store::events::Actor;
use serde_json::{Value, json};

use crate::{Cli, Done, load, locate, lock_wait, save_project, task_line, taskbag_of};

/// One heartbeat in plain words (decision 0022): how long ago, when, by whom, and the busy window
/// while it runs.
pub(crate) fn heartbeat_line(h: &allomos_core::heartbeat::Heartbeat, now: i64) -> String {
    use allomos_core::heartbeat::{parse_utc, plain_duration};
    let mut line = match parse_utc(&h.at) {
        Some(at) => format!("{} ago ({}, by {})", plain_duration(now - at), h.at, h.by),
        None => format!("{} (by {})", h.at, h.by),
    };
    if let Some(until) = &h.busy_until {
        if parse_utc(until).is_some_and(|u| u > now) {
            line.push_str(&format!("; busy until {until}"));
        }
    }
    line
}

/// `alm beat` (decision 0022, Task-A01-nurse164): write the heartbeat of one task, or of every
/// task the session holds, and say what was written and when the next one is due.
pub(crate) fn beat(
    cli: &Cli,
    cwd: &Path,
    id: Option<&str>,
    busy: Option<&str>,
) -> Result<Done, Problem> {
    use allomos_core::heartbeat::{self, Beat};
    let busy = busy
        .map(|b| {
            heartbeat::parse_duration(b).ok_or_else(|| Problem::BusyInvalid {
                value: b.to_string(),
            })
        })
        .transpose()?;
    let root = locate(cli, cwd)?;
    let me = cli.holder.as_deref();
    let beaten = allomos_store::beat(&root, Actor::alm(me), lock_wait(), id, busy)?;
    let mut text = String::new();
    let mut list = Vec::new();
    let mut written = false;
    for b in &beaten.beats {
        match b {
            Beat::Written(task) => {
                written = true;
                let h = task
                    .heartbeat
                    .as_ref()
                    .expect("a written beat has its heartbeat");
                let due = heartbeat::parse_utc(&h.at)
                    .map(|at| heartbeat::format_utc(at + beaten.interval_secs))
                    .unwrap_or_default();
                text.push_str(&format!("{}: heartbeat written at {}", task.id, h.at));
                if let Some(until) = &h.busy_until {
                    text.push_str(&format!(", busy until {until}"));
                }
                text.push_str(&format!("; the next one is due by {due}.\n"));
                list.push(json!({
                    "id": task.id, "written": true, "heartbeat": h, "due": due,
                }));
            }
            Beat::TooSoon { task, ago, due } => {
                text.push_str(&format!(
                    "{}: the last heartbeat was {} ago, under half the interval ({}), so nothing was written. The next one is due by {due}.\n",
                    task.id,
                    heartbeat::plain_duration(*ago),
                    heartbeat::plain_duration(beaten.interval_secs / 2),
                ));
                list.push(json!({
                    "id": task.id, "written": false, "heartbeat": task.heartbeat,
                    "ago_seconds": ago, "due": due,
                }));
            }
        }
    }
    if beaten.beats.is_empty() {
        text.push_str(&format!(
            "{} holds no task in progress, so nothing was written.\n",
            me.unwrap_or("?")
        ));
    }
    if written {
        text.push_str(
            "A heartbeat counts only once it is shared: save and share it now (other sessions judge from what they pulled).\n",
        );
    }
    let mut done = Done::new(json!({ "taskbag": root, "beats": list }), text);
    done.issues = beaten.issues;
    Ok(done)
}

/// `alm heartbeat-settings` (decision 0022 § 3, Task-A01-zero387): show the project's two
/// heartbeat numbers, or change them.
pub(crate) fn settings(
    cli: &Cli,
    cwd: &Path,
    change: &heartbeat::SettingsChange,
) -> Result<Done, Problem> {
    let root = locate(cli, cwd)?;
    let changing = change.interval.is_some() || change.lost_after.is_some() || change.clear;
    let (project, issues, changed) = if changing {
        let lock = Lock::acquire(&root, lock_wait())?;
        let (bag, issues) = taskbag_of(&root, lock.load())?;
        let project = heartbeat::set_settings(&bag.project, change)?;
        let changed = project != bag.project;
        if changed {
            save_project(
                &lock,
                cli.holder.as_deref(),
                "heartbeat-settings",
                &bag.project,
                &project,
            )?;
        }
        (project, issues, changed)
    } else {
        let (bag, issues) = taskbag_of(&root, load(&root))?;
        (bag.project, issues, false)
    };
    let applied = heartbeat::applied(&project);
    let written = project.heartbeat.clone().unwrap_or_default();
    let mark = |v: Option<u32>| if v.is_some() { "" } else { " (default)" };
    let mut text = String::new();
    if changing && !changed {
        text.push_str("Nothing changed: the heartbeat settings already say that.\n");
    }
    text.push_str(&format!(
        "Heartbeat interval: {}{}: a session holding a task beats at least this often (`alm beat`).\n",
        heartbeat::plain_duration(applied.interval_secs),
        mark(written.interval_minutes),
    ));
    text.push_str(&format!(
        "Lost after: {}{}: a held task with no heartbeat for longer, and no busy window running, is listed as lost in `alm next`.\n",
        heartbeat::plain_duration(applied.lost_after_secs),
        mark(written.lost_after_minutes),
    ));
    // alm tells; the agent checks: a limit no longer than the interval is allowed but said.
    if applied.lost_after_secs <= applied.interval_secs {
        text.push_str(
            "Fact: the lost limit is not longer than the interval, so a task can be listed as lost between two regular heartbeats.\n",
        );
    }
    let mut done = Done::new(
        json!({ "taskbag": root, "heartbeat": project.heartbeat, "applied": applied }),
        text,
    );
    done.issues = issues;
    Ok(done)
}

/// What `alm next` says about the held tasks (decision 0022 § 3 and § 4, Task-A01-zero387): the
/// lost ones, with who holds them, how long they have been silent, the limit, any busy window
/// and the command to take one over; and the ones in a busy window still running. `alm` only
/// lists them: the session decides (`alm` tells, the agent checks).
pub(crate) struct Held {
    pub text: String,
    pub lost: Vec<Value>,
    pub busy: Vec<Value>,
    /// Whether any task is listed as lost.
    pub any_lost: bool,
}

pub(crate) fn held(root: &Path, bag: &Taskbag, tasks: &[&Task], now: i64) -> Held {
    let applied = heartbeat::applied(&bag.project);
    // The last recorded change dates a task whose holder never beat; read only when needed.
    let needs_history = tasks.iter().any(|t| {
        t.state == State::InProgress
            && t.holder.is_some()
            && t.heartbeat.as_ref().map(|h| &h.by) != t.holder.as_ref()
    });
    let last = if needs_history {
        allomos_store::last_changed(root)
    } else {
        Default::default()
    };
    let lost: Vec<(&Task, heartbeat::Lost)> = tasks
        .iter()
        .filter_map(|t| {
            heartbeat::lost(
                t,
                now,
                applied.lost_after_secs,
                last.get(&t.id).map(String::as_str),
            )
            .map(|l| (*t, l))
        })
        .collect();
    let busy: Vec<(&Task, String)> = tasks
        .iter()
        .filter_map(|t| heartbeat::busy_until(t, now).map(|u| (*t, u)))
        .collect();
    let branches = if lost.is_empty() {
        Default::default()
    } else {
        allomos_store::branches(root)
    };
    let mut text = String::new();
    let mut lost_json = Vec::new();
    if !lost.is_empty() {
        text.push_str(&format!(
            "Lost, can be taken over (held, no heartbeat for over {}, no busy window running). alm only lists them; you decide, and take one over only when you mean to continue its work:\n",
            heartbeat::plain_duration(applied.lost_after_secs)
        ));
    }
    for (t, l) in &lost {
        let working = branches.get(&t.id).and_then(|b| b.working.clone());
        text.push_str(&format!("{}\n", task_line(t)));
        let silent = heartbeat::plain_duration(l.silent_seconds);
        let since = match (&l.heartbeat_at, &l.busy_until) {
            (Some(_), Some(until)) if l.since == *until => {
                format!("its busy window ended {silent} ago ({until})")
            }
            (Some(at), _) => format!("last heartbeat {silent} ago ({at})"),
            (None, _) => format!(
                "no heartbeat from it; the task last changed {silent} ago ({})",
                l.since
            ),
        };
        text.push_str(&format!(
            "  held by {}; {since}; lost after {}\n",
            l.holder,
            heartbeat::plain_duration(l.lost_after_seconds)
        ));
        let take = format!(
            "alm claim {} --take --as <your session name> --branch {}",
            t.id,
            working.as_deref().unwrap_or("<its working branch>")
        );
        text.push_str(&format!("  to take it over: {take}\n"));
        let mut v = serde_json::to_value(l).unwrap_or(Value::Null);
        v["working_branch"] = json!(working);
        v["take_over"] = json!(take);
        lost_json.push(v);
    }
    if !lost.is_empty() {
        text.push('\n');
    }
    let mut busy_json = Vec::new();
    for (t, until) in &busy {
        text.push_str(&format!(
            "Busy: {} is held by {}, busy until {until} (not lost while it runs).\n",
            t.id,
            t.holder.as_deref().unwrap_or("?")
        ));
        busy_json.push(json!({ "id": t.id, "holder": t.holder, "busy_until": until }));
    }
    if !busy.is_empty() {
        text.push('\n');
    }
    Held {
        any_lost: !lost_json.is_empty(),
        text,
        lost: lost_json,
        busy: busy_json,
    }
}
