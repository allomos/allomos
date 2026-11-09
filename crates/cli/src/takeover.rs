//! A session coming back (decision 0022 § 8, Task-A01-castle658). With `--as`, every command
//! first looks for tasks this session held that another session has taken over since: it names
//! them at the very start of what it prints (read-only commands too), and refuses the session's
//! changes to them (`task_held`).

use std::path::Path;

use allomos_core::Problem;
use allomos_core::takeover::{self, TakenOver};
use serde_json::{Value, json};

use crate::{Cli, load, locate};

/// The tasks taken over from the `--as` session. Nothing without `--as`, or without a taskbag to
/// read (`alm init`, a wrong path: the command itself reports that).
pub(crate) fn look(cli: &Cli, cwd: &Path) -> Vec<TakenOver> {
    let Some(me) = cli
        .holder
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty())
    else {
        return Vec::new();
    };
    let Ok(root) = locate(cli, cwd) else {
        return Vec::new();
    };
    let Some(bag) = load(&root).taskbag else {
        return Vec::new();
    };
    let (events, _) = allomos_store::events::list(&root);
    takeover::taken_over(&bag.tasks, &events, me)
}

/// Refuse a change to a task taken over from this session: it no longer holds it, whatever the
/// command (claiming it back too, `--take` included). Reading it is allowed.
pub(crate) fn refuse(cli: &Cli, taken: &[TakenOver]) -> Result<(), Problem> {
    if !cli.command.changes() {
        return Ok(());
    }
    let Some(id) = cli.command.target() else {
        return Ok(());
    };
    match taken.iter().find(|t| t.id == id) {
        Some(t) => Err(Problem::TaskHeld {
            id: t.id.clone(),
            holder: t.holder.clone(),
        }),
        None => Ok(()),
    }
}

/// The lines printed first, one notice for each task.
pub(crate) fn text(taken: &[TakenOver]) -> String {
    let mut out = String::new();
    for t in taken {
        let n = t.notice();
        out.push_str(&format!("notice [{}]: {}", n.code(), n.english()));
        if t.holder != t.by {
            out.push_str(&format!(" It is held by {} now.", t.holder));
        }
        out.push('\n');
    }
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

/// The `taken_over` list of a `--json` result.
pub(crate) fn json(taken: &[TakenOver]) -> Value {
    json!(taken)
}
