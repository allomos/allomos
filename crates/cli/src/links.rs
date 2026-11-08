//! `alm link` and `alm links` (Task-A01-mint242, decision 0021 § 1.1 "Links to code and
//! releases"): the commits, branches, merge commits, PRs, CI runs and releases that go with a
//! task, and the lines `alm show` lists them with.

use std::path::Path;

use allomos_core::Problem;
use allomos_core::link::{self, Recorded};
use allomos_store::events::Actor;
use allomos_store::links;
use serde_json::json;

use crate::{Cli, Done, load, locate, lock_wait, task_line, taskbag_of};

pub(crate) fn link(
    cli: &Cli,
    cwd: &Path,
    id: &str,
    kind: &str,
    reference: &str,
    url: Option<&str>,
    remove: bool,
) -> Result<Done, Problem> {
    let root = locate(cli, cwd)?;
    let given = link::parse(kind, reference, url)?;
    let l = links::record(
        &root,
        Actor::alm(cli.holder.as_deref()),
        lock_wait(),
        id,
        &given,
        remove,
    )?;
    let mut out = format!("{}\n", task_line(&l.task));
    let what = match (l.changed, remove) {
        (false, _) => "already recorded so; nothing changed",
        (true, true) => "taken off",
        (true, false) => "recorded",
    };
    out.push_str(&format!("{} {} {what}.\n", l.link.kind, l.link.reference));
    out.push_str(&lines(&l.links));
    if !remove {
        // alm tells; the agent checks (AGENTS.md): it does not look the reference up.
        out.push_str(
            "Fact: alm runs no version control tool and checks only how it is written; check that it exists as given.\n",
        );
        if l.link.url.is_none() {
            out.push_str(&format!(
                "Fact: it has no web address, so the app cannot open it. Add one with: alm link {} {} {} --url <address>\n",
                l.task.id, l.link.kind, l.link.reference
            ));
        }
    }
    let mut done = Done::new(
        json!({ "taskbag": root, "task": l.task.id, "link": l.link, "changed": l.changed, "removed": remove && l.changed, "links": l.links }),
        out,
    );
    done.issues = l.issues;
    Ok(done)
}

/// `alm links [<ref>]`: the tasks a reference goes with, or every task's links.
pub(crate) fn list(cli: &Cli, cwd: &Path, reference: Option<&str>) -> Result<Done, Problem> {
    let root = locate(cli, cwd)?;
    let (bag, issues) = taskbag_of(&root, load(&root))?;
    let mut all = links::all(&root);
    let mut out = String::new();
    let head = |task: &str| match bag.tasks.iter().find(|t| t.id == task) {
        Some(t) => task_line(t),
        None => task.to_string(),
    };
    if let Some(given) = reference {
        all.retain(|r| link::matches(&r.link, given));
        if all.is_empty() {
            out.push_str(&format!("No task records a link to {}.\n", given.trim()));
        }
    } else if all.is_empty() {
        out.push_str(
            "No task records a link yet (`alm link <id> <kind> <ref> [--url <address>]`).\n",
        );
    }
    let mut tasks: Vec<String> = Vec::new();
    for r in &all {
        if !tasks.contains(&r.task) {
            tasks.push(r.task.clone());
        }
    }
    tasks.sort();
    for t in &tasks {
        out.push_str(&format!("{}\n", head(t)));
        let mut mine: Vec<&Recorded> = all.iter().filter(|r| &r.task == t).collect();
        mine.sort_by_key(|r| link::KINDS.iter().position(|k| *k == r.link.kind));
        for r in mine {
            out.push_str(&link_line(r, "  "));
        }
    }
    let mut done = Done::new(
        json!({ "taskbag": root, "reference": reference.map(str::trim), "tasks": tasks, "links": all }),
        out,
    );
    done.issues = issues;
    Ok(done)
}

fn link_line(r: &Recorded, indent: &str) -> String {
    let mut s = format!("{indent}- {}: {}", r.link.kind, r.link.reference);
    if let Some(u) = &r.link.url {
        s.push_str(&format!("  {u}"));
    }
    s.push_str(&format!(
        " ({}{})\n",
        r.at,
        r.by.as_deref()
            .map(|b| format!(", {b}"))
            .unwrap_or_default()
    ));
    s
}

/// `alm show`'s lines for a task's links.
pub(crate) fn lines(links: &[Recorded]) -> String {
    if links.is_empty() {
        return String::new();
    }
    let mut text = String::from("Links to code and releases:\n");
    for r in links {
        text.push_str(&link_line(r, "  "));
    }
    text
}
