//! A person's request that this session hand a task over (Task-A01-minnow200). With `--as`, every
//! command first names the requests in force addressed to that session, as `takeover` does for
//! tasks taken over from it. It only tells: the session hands over by itself.

use std::path::Path;

use allomos_core::handover::{self, HandoverAsked};
use serde_json::{Value, json};

use crate::{Cli, load, locate};

/// The requests addressed to the `--as` session. Nothing without `--as`, or without a taskbag to
/// read (`alm init`, a wrong path: the command itself reports that).
pub(crate) fn look(cli: &Cli, cwd: &Path) -> Vec<HandoverAsked> {
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
    handover::asked_of(&bag.tasks, me)
}

/// The lines printed first, one notice for each task.
pub(crate) fn text(asked: &[HandoverAsked]) -> String {
    let mut out = String::new();
    for a in asked {
        let n = a.notice();
        out.push_str(&format!("notice [{}]: {}\n", n.code(), n.english()));
    }
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

/// The `handover_requested` list of a `--json` result.
pub(crate) fn json(asked: &[HandoverAsked]) -> Value {
    json!(asked)
}
