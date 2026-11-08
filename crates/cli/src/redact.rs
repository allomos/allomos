//! `alm redact` (Task-A01-stem986, decision 0021 § 6 item 5): removing personal details written
//! into a task by mistake, leaving an event that says what was removed, without the old values.

use std::path::Path;

use allomos_core::Problem;
use allomos_store::events::Actor;
use allomos_store::redact::{self, Request};
use serde_json::json;

use crate::{Cli, Done, locate, lock_wait, task_line};

pub(crate) fn redact(cli: &Cli, cwd: &Path, id: &str, request: &Request) -> Result<Done, Problem> {
    let root = locate(cli, cwd)?;
    let r = redact::redact(
        &root,
        Actor::alm(cli.holder.as_deref()),
        lock_wait(),
        id,
        request,
    )?;
    let mut out = format!("{}\n", task_line(&r.task));
    match &r.redaction {
        None => out.push_str(
            "Nothing removed: every past value of these fields is the current one or was removed before. The current value stays; change it first (alm describe, alm title, alm note…), then run alm redact again.\n",
        ),
        Some(rd) => {
            out.push_str(&format!(
                "Redaction {} recorded: {} removed from {} event{}; the event says which fields and files, who approved it and why, never the old values.\n",
                rd.id,
                rd.fields.join(", "),
                rd.files.len(),
                if rd.files.len() == 1 { "" } else { "s" }
            ));
            for f in &rd.files {
                out.push_str(&format!("  {} ({})\n", f.file, f.fields.join(", ")));
            }
            out.push_str(
                "The current value of each field stays as it is. Older versions stay in Git history unless the history is rewritten, which alm audit will then report.\n",
            );
            match rd.approved_by.as_deref() {
                None => out.push_str(
                    "Fact: nobody is named as approving this removal. The record keeps who approved it when given: --approved-by \"<their pseudonym ID or role>\".\n",
                ),
                Some(who) if who.contains('@') => out.push_str(
                    "Fact: \"approved by\" looks like an email address; the history keeps it for good. Personal details stay out of the taskbag (AGENTS.md): name a pseudonym ID or a role instead.\n",
                ),
                Some(_) => {}
            }
        }
    }
    let mut done = Done::new(
        json!({ "taskbag": root, "task": r.task.id, "redaction": r.redaction, "changed": r.redaction.is_some() }),
        out,
    );
    done.issues = r.issues;
    Ok(done)
}
