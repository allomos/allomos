//! `alm effect` and `alm effects` (Task-A01-job650, decision 0021 § 1.1 "Outside effects"): the
//! outside-effect ledger, and the lines `alm show` lists a task's effects with.

use std::path::Path;

use allomos_core::Problem;
use allomos_core::approval::Status;
use allomos_core::effect::{Given, Recorded};
use allomos_store::effects;
use allomos_store::events::Actor;
use serde_json::json;

use crate::{Cli, Done, load, locate, lock_wait, task_line, taskbag_of};

/// What `alm effect` was given (its approval is the global `--approval`).
pub(crate) struct Args<'a> {
    pub id: &'a str,
    pub of: Option<&'a str>,
    pub target: Option<&'a str>,
    pub action: Option<&'a str>,
    pub rollback: Option<&'a str>,
    pub result: Option<&'a str>,
}

/// The approval the change relies on: `--approval`, else `ALLOMOS_APPROVAL`.
fn approval_given(cli: &Cli) -> Option<String> {
    cli.approval
        .clone()
        .or_else(|| std::env::var("ALLOMOS_APPROVAL").ok())
        .map(|a| a.trim().to_string())
        .filter(|a| !a.is_empty())
}

pub(crate) fn effect(cli: &Cli, cwd: &Path, args: &Args) -> Result<Done, Problem> {
    let root = locate(cli, cwd)?;
    let approval = approval_given(cli);
    let e = effects::record(
        &root,
        Actor::alm(cli.holder.as_deref()),
        lock_wait(),
        args.id,
        &Given {
            of: args.of,
            target: args.target,
            action: args.action,
            rollback: args.rollback,
            approval: approval.as_deref(),
            result: args.result,
        },
    )?;
    let ef = &e.effect;
    let mut out = format!("{}\n", task_line(&e.task));
    let what = match (e.changed, args.of.is_some()) {
        (false, _) => "already recorded so; nothing changed",
        (true, false) => "recorded",
        (true, true) => "updated",
    };
    out.push_str(&format!("Outside effect {} {what}:\n", ef.id));
    out.push_str(&effect_lines(ef, "  "));
    // alm tells; the agent checks (AGENTS.md): what the approval is, not whether it suffices.
    match (&ef.approval, &e.approval) {
        (None, _) => out.push_str(
            "Fact: it names no approval. An outside effect needs the approver's exact approval (AGENTS.md § Always-on boundaries): once they gave it, record it with `alm approve` and name it with `--approval <approval ID>` on this command.\n",
        ),
        (Some(_), Some(r)) => {
            if r.status != Status::Applies {
                out.push_str(&format!(
                    "Fact: approval {} {}.\n",
                    r.approval.id,
                    crate::approve::status_words(r)
                ));
            }
            if !r.approval.scope.tasks.contains(&e.task.id) {
                out.push_str(&format!(
                    "Fact: approval {} covers {}, not {}.\n",
                    r.approval.id,
                    r.approval.scope.tasks.join(", "),
                    e.task.id
                ));
            }
        }
        (Some(_), None) => {}
    }
    if ef.result.is_none() {
        out.push_str(&format!(
            "Under way. Once it is done, record what happened: alm effect {} --of {} --result \"<what happened>\"\n",
            e.task.id, ef.id
        ));
    }
    let mut done = Done::new(
        json!({ "taskbag": root, "task": e.task.id, "effect": ef, "changed": e.changed, "approval": e.approval }),
        out,
    );
    done.issues = e.issues;
    Ok(done)
}

/// `alm effects [<task>]`: one task's outside effects, or every one in the project.
pub(crate) fn list(cli: &Cli, cwd: &Path, id: Option<&str>) -> Result<Done, Problem> {
    let root = locate(cli, cwd)?;
    let (bag, issues) = taskbag_of(&root, load(&root))?;
    let mut all = effects::all(&root);
    let mut out = String::new();
    match id {
        Some(id) => {
            let task = allomos_core::ops::find_task(&bag, id)?;
            all.retain(|r| r.task == task.id);
            out.push_str(&format!("{}\n", task_line(&task)));
            if all.is_empty() {
                out.push_str("  No outside effect recorded.\n");
            }
            for r in &all {
                out.push_str(&recorded_lines(r, "  "));
            }
        }
        None => {
            if all.is_empty() {
                out.push_str("No outside effect recorded yet (`alm effect <id> --target … --action … --rollback …`).\n");
            }
            let mut last: Option<&str> = None;
            for r in &all {
                if last != Some(r.task.as_str()) {
                    match bag.tasks.iter().find(|t| t.id == r.task) {
                        Some(t) => out.push_str(&format!("{}\n", task_line(t))),
                        None => out.push_str(&format!("{}\n", r.task)),
                    }
                }
                last = Some(r.task.as_str());
                out.push_str(&recorded_lines(r, "  "));
            }
        }
    }
    let under_way = all.iter().filter(|r| r.effect.result.is_none()).count();
    if under_way > 0 {
        out.push_str(&format!(
            "{under_way} without a result yet: record each with `alm effect <task> --of <effect ID> --result \"…\"`.\n"
        ));
    }
    let mut done = Done::new(json!({ "taskbag": root, "effects": all }), out);
    done.issues = issues;
    Ok(done)
}

fn effect_lines(ef: &allomos_core::effect::Effect, indent: &str) -> String {
    format!(
        "{indent}target: {}\n{indent}action: {}\n{indent}rollback: {}\n{indent}approval: {}\n{indent}result: {}\n",
        ef.target,
        ef.action,
        ef.rollback,
        ef.approval.as_deref().unwrap_or("none named"),
        ef.result.as_deref().unwrap_or("under way (none yet)"),
    )
}

fn recorded_lines(r: &Recorded, indent: &str) -> String {
    let mut s = format!(
        "{indent}- {} ({}{})\n",
        r.effect.id,
        r.at,
        r.by.as_deref()
            .map(|b| format!(", {b}"))
            .unwrap_or_default()
    );
    s.push_str(&effect_lines(&r.effect, &format!("{indent}    ")));
    s
}

/// `alm show`'s lines for a task's outside effects.
pub(crate) fn lines(effects: &[Recorded]) -> String {
    if effects.is_empty() {
        return String::new();
    }
    let mut text = String::from("Outside effects:\n");
    for r in effects {
        text.push_str(&recorded_lines(r, "  "));
    }
    text
}
