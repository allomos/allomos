//! `alm classify` (Task-A01-palm011, decision 0021 § 1.1 "Risk class and change type"): a task's
//! change type, risk and what it touches; the lines `alm show` lists them with, and the
//! emergencies `alm human` lists while they are not approved afterwards.

use std::path::Path;

use allomos_core::Problem;
use allomos_core::approval::Recorded as Approval;
use allomos_core::classify::{self, Classification, Given};
use allomos_store::events::Actor;
use serde_json::json;

use crate::{Cli, Done, load, locate, lock_wait, task_line, taskbag_of};

/// `alm classify <id> [--type …] [--risk …] [--touches …] [--due …]`; without options, what is
/// recorded.
pub(crate) fn classify(cli: &Cli, cwd: &Path, id: &str, given: &Given) -> Result<Done, Problem> {
    let root = locate(cli, cwd)?;
    let nothing_given = given.change_type.is_none()
        && given.risk.is_none()
        && given.touches.is_none()
        && given.due.is_none();
    let (task, c, changed, issues) = if nothing_given {
        let (bag, issues) = taskbag_of(&root, load(&root))?;
        let task = allomos_core::ops::find_task(&bag, id)?;
        let c = allomos_store::classify::by_task(&root)
            .remove(&task.id)
            .unwrap_or_default();
        (task, c, false, issues)
    } else {
        let r = allomos_store::classify::set(
            &root,
            Actor::alm(cli.holder.as_deref()),
            lock_wait(),
            id,
            given,
        )?;
        (r.task, r.classification, r.changed, r.issues)
    };
    let approvals = allomos_store::approvals::approvals(&root)
        .remove(&task.id)
        .unwrap_or_default();
    let mut out = format!("{}\n", task_line(&task));
    if !nothing_given && !changed {
        out.push_str("Already recorded so; nothing changed.\n");
    }
    if c.is_empty() {
        out.push_str(&format!(
            "No change type or risk recorded: alm classify {} --type standard|normal|emergency --risk low|medium|high --touches production|others-data|none\n",
            task.id
        ));
    } else {
        out.push_str(&lines(&task.id, &c, &approvals));
    }
    // alm tells; the agent checks (AGENTS.md): what is recorded and missing, never a verdict.
    if !c.is_empty() {
        let mut missing = Vec::new();
        if c.change_type.is_none() {
            missing.push("--type");
        }
        if c.risk.is_none() {
            missing.push("--risk");
        }
        if !c.touches_recorded {
            missing.push("--touches");
        }
        if !missing.is_empty() {
            out.push_str(&format!("Not recorded yet: {}.\n", missing.join(", ")));
        }
    }
    if !c.touches.is_empty() && c.change_type.as_deref() != Some(classify::EMERGENCY) {
        out.push_str(
            "Fact: it touches production or other people's data, an outside effect: it needs the approver's exact approval first (AGENTS.md § Always-on boundaries); record it with `alm approve`.\n",
        );
    }
    // Decision 0027 § 3 (Task-A01-steam844): without options, the rules to judge by, the fixed
    // ones and the project's own. alm lists them; the agent judges.
    let mut fields =
        json!({ "taskbag": root, "task": task.id, "classification": c, "changed": changed });
    if nothing_given {
        let (language, applied) = crate::risk_rules::of_taskbag(&root);
        out.push_str(&crate::risk_rules::text(&language, &applied));
        fields["risk_rules"] = crate::risk_rules::json(&applied);
    }
    let mut done = Done::new(fields, out);
    done.issues = issues;
    Ok(done)
}

fn touches_text(c: &Classification) -> String {
    if c.touches.is_empty() {
        return "neither production nor other people's data".to_string();
    }
    c.touches
        .iter()
        .map(|t| match t.as_str() {
            "others_data" => "other people's data",
            other => other,
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// `alm show`'s lines for a task's change type and risk; nothing when none is recorded.
pub(crate) fn lines(id: &str, c: &Classification, approvals: &[Approval]) -> String {
    if c.is_empty() {
        return String::new();
    }
    let mut text = String::new();
    if let Some(t) = &c.change_type {
        text.push_str(&format!("change type: {t}\n"));
    }
    if let Some(r) = &c.risk {
        text.push_str(&format!("risk: {r}\n"));
    }
    if c.touches_recorded {
        text.push_str(&format!("touches: {}\n", touches_text(c)));
    }
    if c.is_emergency() {
        match classify::approved_afterwards(c, approvals) {
            Some(a) => text.push_str(&format!(
                "  emergency: approved afterwards ({}, {})\n",
                a.approval.id, a.approval.at
            )),
            None => text.push_str(&format!(
                "  emergency: not approved afterwards yet; approval due by {} (once the approver gives it: `alm approve {id} --words \"<their words>\"`)\n",
                c.approval_due.as_deref().unwrap_or("no deadline recorded")
            )),
        }
    }
    text
}

/// A task's risk for `alm human` (Task-A01-garden385, decision 0027), in the task language like
/// the state and the labels of its line: "risk: high", or "risk: not classified" when no agent
/// recorded one. `alm` shows what is recorded and judges nothing.
pub(crate) fn human_risk_line(risk: Option<&str>, language: &str) -> String {
    let (word, sep) = match language {
        "zh-CN" => ("风险", "："),
        "ja" => ("リスク", "："),
        _ => ("risk", ": "),
    };
    let level = match (risk, language) {
        (Some("high"), "zh-CN") => "高",
        (Some("medium"), "zh-CN") => "中",
        (Some("low"), "zh-CN") => "低",
        (None, "zh-CN") => "未分级",
        (Some("high"), "ja") => "高",
        (Some("medium"), "ja") => "中",
        (Some("low"), "ja") => "低",
        (None, "ja") => "未分類",
        (None, _) => "not classified",
        (Some(other), _) => other,
    };
    format!("{word}{sep}{level}")
}
