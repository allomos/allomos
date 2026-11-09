//! Heartbeats in `alm` (decision 0022): `alm beat` (Task-A01-nurse164) and the line `alm show`
//! gives a task's last heartbeat.

use std::path::Path;

use allomos_core::Problem;
use allomos_store::events::Actor;
use serde_json::json;

use crate::{Cli, Done, locate, lock_wait};

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
                    .map(|at| heartbeat::format_utc(at + heartbeat::INTERVAL_SECS))
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
                    heartbeat::plain_duration(heartbeat::INTERVAL_SECS / 2),
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
