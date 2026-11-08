//! `alm exception` and `alm exceptions` (Task-A01-seed343, decision 0021 § 1.1 "Exceptions"):
//! skipped controls with why and who accepted the risk, and the lines `alm show` lists a task's
//! exceptions with.

use std::path::Path;

use allomos_core::Problem;
use allomos_core::exception::{Exception, Given, Recorded};
use allomos_store::events::Actor;
use allomos_store::exceptions;
use serde_json::json;

use crate::{Cli, Done, load, locate, lock_wait, task_line, taskbag_of};

pub(crate) fn exception(cli: &Cli, cwd: &Path, id: &str, given: &Given) -> Result<Done, Problem> {
    let root = locate(cli, cwd)?;
    let e = exceptions::record(
        &root,
        Actor::alm(cli.holder.as_deref()),
        lock_wait(),
        id,
        given,
    )?;
    let ex = &e.exception;
    let mut out = format!("{}\n", task_line(&e.task));
    let what = match (e.changed, given.of.is_some()) {
        (false, _) => "already recorded so; nothing changed",
        (true, false) => "recorded",
        (true, true) => "updated",
    };
    out.push_str(&format!("Exception {} {what}:\n", ex.id));
    out.push_str(&exception_lines(ex, "  "));
    out.push_str(&facts(&e.task.id, ex));
    let mut done = Done::new(
        json!({ "taskbag": root, "task": e.task.id, "exception": ex, "changed": e.changed }),
        out,
    );
    done.issues = e.issues;
    Ok(done)
}

/// What alm tells about one exception (it refuses nothing for these): nobody named as accepting
/// the risk, or what looks like an email address where a pseudonym or role belongs.
pub(crate) fn facts(task: &str, ex: &Exception) -> String {
    match ex.accepted_by.as_deref() {
        None => format!(
            "Fact: nobody is named as accepting this risk. Once the person who accepted it says so, record it: alm exception {task} --of {} --accepted-by \"<their pseudonym ID or role>\"\n",
            ex.id
        ),
        Some(who) if who.contains('@') => format!(
            "Fact: \"accepted by\" looks like an email address; the history keeps it for good. Personal details stay out of the taskbag (AGENTS.md): name a pseudonym ID or a role instead: alm exception {task} --of {} --accepted-by \"<pseudonym ID or role>\"\n",
            ex.id
        ),
        Some(_) => String::new(),
    }
}

/// `alm exceptions [<task>]`: one task's exceptions, or every one in the project.
pub(crate) fn list(cli: &Cli, cwd: &Path, id: Option<&str>) -> Result<Done, Problem> {
    let root = locate(cli, cwd)?;
    let (bag, issues) = taskbag_of(&root, load(&root))?;
    let mut all = exceptions::all(&root);
    let mut out = String::new();
    match id {
        Some(id) => {
            let task = allomos_core::ops::find_task(&bag, id)?;
            all.retain(|r| r.task == task.id);
            out.push_str(&format!("{}\n", task_line(&task)));
            if all.is_empty() {
                out.push_str("  No exception recorded.\n");
            }
            for r in &all {
                out.push_str(&recorded_lines(r, "  "));
            }
        }
        None => {
            if all.is_empty() {
                out.push_str("No exception recorded yet (`alm exception <id> --kind … --why … --accepted-by …`).\n");
            }
            // By task, in the order each task's first exception was recorded.
            let mut order: Vec<&str> = Vec::new();
            for r in &all {
                if !order.contains(&r.task.as_str()) {
                    order.push(&r.task);
                }
            }
            for task_id in order {
                match bag.tasks.iter().find(|t| t.id == task_id) {
                    Some(t) => out.push_str(&format!("{}\n", task_line(t))),
                    None => out.push_str(&format!("{task_id}\n")),
                }
                for r in all.iter().filter(|r| r.task == task_id) {
                    out.push_str(&recorded_lines(r, "  "));
                }
            }
        }
    }
    let unaccepted = all
        .iter()
        .filter(|r| r.exception.accepted_by.is_none())
        .count();
    if unaccepted > 0 {
        out.push_str(&format!(
            "{unaccepted} with nobody named as accepting the risk: record each with `alm exception <task> --of <exception ID> --accepted-by \"…\"`.\n"
        ));
    }
    let mut done = Done::new(json!({ "taskbag": root, "exceptions": all }), out);
    done.issues = issues;
    Ok(done)
}

/// The kind in words.
fn kind_words(kind: &str) -> &str {
    match kind {
        allomos_core::exception::CHECK_NOT_RUN => "check not run",
        allomos_core::exception::WITHOUT_HUMAN_CHECK => "finished without the human check",
        allomos_core::exception::OTHER => "other",
        k => k,
    }
}

pub(crate) fn exception_lines(ex: &Exception, indent: &str) -> String {
    let mut s = format!("{indent}kind: {}\n", kind_words(&ex.kind));
    if let Some(check) = &ex.check {
        s.push_str(&format!("{indent}check: {check}\n"));
    }
    s.push_str(&format!(
        "{indent}why: {}\n{indent}accepted by: {}\n",
        ex.why,
        ex.accepted_by.as_deref().unwrap_or("nobody named yet")
    ));
    s
}

fn recorded_lines(r: &Recorded, indent: &str) -> String {
    let mut s = format!(
        "{indent}- {} ({}{})\n",
        r.exception.id,
        r.at,
        r.by.as_deref()
            .map(|b| format!(", {b}"))
            .unwrap_or_default()
    );
    s.push_str(&exception_lines(&r.exception, &format!("{indent}    ")));
    s
}

/// `alm show`'s lines for a task's exceptions.
pub(crate) fn lines(exceptions: &[Recorded]) -> String {
    if exceptions.is_empty() {
        return String::new();
    }
    let mut text = String::from("Exceptions:\n");
    for r in exceptions {
        text.push_str(&recorded_lines(r, "  "));
    }
    text
}
