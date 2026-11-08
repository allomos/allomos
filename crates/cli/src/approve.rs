//! `alm approve` and `alm withdraw` (Task-A01-coach782, decision 0021 § 1.1): a person's approval
//! as a record, and the lines `alm show` lists a task's approvals with.

use std::path::Path;

use allomos_core::Problem;
use allomos_core::approval::{Recorded, Status};
use allomos_store::approvals;
use allomos_store::events::Actor;
use serde_json::json;

use crate::{Cli, Done, locate, lock_wait, task_line};

/// What `alm approve` was given.
pub(crate) struct Args<'a> {
    pub ids: &'a [String],
    pub words: Option<&'a str>,
    pub channel: &'a str,
    pub plan: Option<&'a Path>,
    pub expires: Option<&'a str>,
    pub approver: Option<&'a str>,
}

pub(crate) fn approve(cli: &Cli, cwd: &Path, args: &Args) -> Result<Done, Problem> {
    let root = locate(cli, cwd)?;
    let plan = match args.plan {
        Some(file) => {
            let path = cwd.join(file);
            let text = std::fs::read_to_string(&path).map_err(|e| Problem::ReadFailed {
                detail: format!("{}: {e}", path.display()),
            })?;
            Some((file.to_string_lossy().into_owned(), text))
        }
        None => None,
    };
    let a = approvals::approve(
        &root,
        Actor::alm(cli.holder.as_deref()),
        lock_wait(),
        &approvals::Input {
            tasks: args.ids,
            approver: args.approver,
            channel: args.channel,
            words: args.words,
            expires: args.expires,
            plan: plan.as_ref().map(|(n, t)| (n.as_str(), t.as_str())),
            shown: None,
        },
    )?;
    let mut out = String::new();
    for t in &a.tasks {
        out.push_str(&format!("{}\n", task_line(t)));
    }
    let ap = &a.approval;
    out.push_str(&format!(
        "Approval {} recorded: by {}, {} ({}), plan {} (SHA-256 {}).\n",
        ap.id,
        ap.approver.as_deref().unwrap_or("an unknown person"),
        ap.at,
        ap.channel,
        ap.scope.plan,
        &ap.scope.plan_sha256[..12]
    ));
    if let Some(e) = &ap.expires {
        out.push_str(&format!("It holds through {e} (UTC).\n"));
    }
    if ap.scope.plan == allomos_core::approval::PLAN_DESCRIPTION {
        out.push_str(
            "If the task's description changes, this approval no longer applies: ask again.\n",
        );
    } else {
        out.push_str("alm keeps no copy of that file: if the plan changes, ask again.\n");
    }
    out.push_str(&format!(
        "Pass `--approval {}` on the changes that rely on it.\n",
        ap.id
    ));
    let mut done = Done::new(
        json!({ "taskbag": root, "approval": ap, "tasks": a.tasks.iter().map(|t| &t.id).collect::<Vec<_>>() }),
        out,
    );
    done.issues = a.issues;
    Ok(done)
}

pub(crate) fn withdraw(cli: &Cli, cwd: &Path, id: &str) -> Result<Done, Problem> {
    let root = locate(cli, cwd)?;
    let a = approvals::withdraw(
        &root,
        Actor::alm(cli.holder.as_deref()),
        lock_wait(),
        id,
        None,
    )?;
    let mut out = String::new();
    for t in &a.tasks {
        out.push_str(&format!("{}\n", task_line(t)));
    }
    out.push_str(&format!(
        "Approval {} withdrawn: it no longer applies. The history keeps it.\n",
        a.approval.id
    ));
    let mut done = Done::new(
        json!({ "taskbag": root, "approval": a.approval, "tasks": a.tasks.iter().map(|t| &t.id).collect::<Vec<_>>() }),
        out,
    );
    done.issues = a.issues;
    Ok(done)
}

/// The line pointing out an approval withdrawn while task `id` is in progress (approver,
/// 2026-11-08, Task-A01-baker800): `alm` says so; the holder decides whether to stop.
pub(crate) fn withdrawn_line(id: &str, holder: Option<&str>, r: &Recorded) -> String {
    format!(
        "Approval withdrawn: {} on {} was withdrawn {} while {} is in progress. Withdrawing stops nothing by itself: {} decides whether to stop (`alm history {id}` says why).\n",
        r.approval.id,
        id,
        r.withdrawn_at.as_deref().unwrap_or(""),
        id,
        holder.unwrap_or("whoever holds it"),
    )
}

/// Where an approval stands, in words.
pub(crate) fn status_words(r: &Recorded) -> String {
    match r.status {
        Status::Applies => "applies".into(),
        Status::PlanChanged => "no longer applies: the plan changed since".into(),
        Status::Expired => format!(
            "no longer applies: expired after {}",
            r.approval.expires.as_deref().unwrap_or("")
        ),
        Status::Withdrawn => format!("withdrawn {}", r.withdrawn_at.as_deref().unwrap_or("")),
    }
}

/// `alm show`'s lines for a task's approvals.
pub(crate) fn lines(approvals: &[Recorded]) -> String {
    if approvals.is_empty() {
        return String::new();
    }
    let mut text = String::from("Approvals:\n");
    for r in approvals {
        let a = &r.approval;
        text.push_str(&format!(
            "  - {} by {} ({}, {}){}: {}\n",
            a.id,
            a.approver.as_deref().unwrap_or("an unknown person"),
            a.channel,
            a.at,
            a.words
                .as_deref()
                .map(|w| format!(", \"{w}\""))
                .unwrap_or_default(),
            status_words(r)
        ));
    }
    text
}
