//! `alm show <id> --evidence` (Task-A01-inlet632, decision 0029 point 4): the task's Evidence
//! screen as text, every part in the app's order (from [`allomos_core::evidence`]). A part the
//! task lacks says "none"; nothing is judged or refused.

use std::path::Path;

use allomos_core::Problem;
use allomos_core::evidence::{self, ORDER, Part};
use allomos_core::labels::AI_DECISIONS;
use allomos_core::ops;
use serde_json::json;

use crate::text::{decision_lines, indent};
use crate::{Cli, Done, load, locate, taskbag_of};

/// What a part with nothing in it says.
const NONE: &str = "    none\n";

pub(crate) fn show(cli: &Cli, cwd: &Path, id: &str) -> Result<Done, Problem> {
    let root = locate(cli, cwd)?;
    let (bag, issues) = taskbag_of(&root, load(&root))?;
    let task = ops::find_task(&bag, id)?;
    let ev = allomos_store::evidence::by_task(&root, &bag)
        .remove(&task.id)
        .unwrap_or_else(|| evidence::evidence(&task, &[], &[], &[], &[]));
    let links = allomos_store::links::by_task(&root)
        .remove(&task.id)
        .unwrap_or_default();
    let decisions = allomos_store::ai_decisions(&root)
        .remove(&task.id)
        .unwrap_or_default();
    let risk = allomos_store::classify::by_task(&root)
        .remove(&task.id)
        .and_then(|c| c.risk);

    let mut text = format!(
        "Evidence of {}: {} (the parts of the app's Evidence screen, in its order)\n",
        task.id, task.title
    );
    for part in ORDER {
        let has = ev.parts.contains(&part);
        let (heading, body) = match part {
            Part::AskRisk => {
                let ask = if task.to_check() {
                    let review = decisions.iter().filter(|d| d.to_review()).count();
                    if task.has_label(AI_DECISIONS) {
                        format!("check it by hand; review {review} AI decision(s)")
                    } else {
                        "check it by hand".to_string()
                    }
                } else if task.human_checked() {
                    "none: a person has checked it".to_string()
                } else {
                    format!("none: it is {}, not done", task.state_text())
                };
                let risk = match (&risk, &ev.risk_reason) {
                    (Some(r), Some(why)) => format!("{r} — {why}"),
                    (Some(r), None) => r.clone(),
                    (None, _) => "none recorded".to_string(),
                };
                (
                    "Asked of you and risk",
                    format!("    asked: {ask}\n    risk: {risk}\n"),
                )
            }
            Part::CheckSteps => {
                let mut body = String::new();
                if let Some(steps) = task
                    .human_check_steps
                    .as_deref()
                    .filter(|s| !s.trim().is_empty())
                {
                    body.push_str(&indent(steps));
                }
                if let Some(why) = task
                    .without_human_check
                    .as_deref()
                    .filter(|s| !s.trim().is_empty())
                {
                    body.push_str(&format!("    finished without the human check: {why}\n"));
                }
                ("Human check steps", body)
            }
            Part::Summary => {
                let mut body = String::new();
                if !ev.summary.is_empty() {
                    body.push_str(&indent(&ev.summary));
                }
                for l in evidence::code_links(&links) {
                    let url = l
                        .link
                        .url
                        .as_deref()
                        .map(|u| format!("  {u}"))
                        .unwrap_or_default();
                    body.push_str(&format!("    {}: {}{url}\n", l.link.kind, l.link.reference));
                }
                ("What changed", body)
            }
            Part::Tests => (
                "Test results",
                if ev.check_results.is_empty() {
                    "    none recorded by the agent: see the Evidence section of the description (`alm show <id>`)\n"
                        .replace("<id>", &task.id)
                } else {
                    crate::check_results::lines(&ev.check_results)
                        .lines()
                        .skip(1)
                        .map(|l| format!("  {l}\n"))
                        .collect()
                },
            ),
            Part::Decisions => {
                let (review, details): (Vec<_>, Vec<_>) =
                    decisions.iter().cloned().partition(|d| d.to_review());
                let mut body = String::new();
                for line in decision_lines(&review).lines() {
                    body.push_str(&format!("  {line}\n"));
                }
                if !details.is_empty() {
                    body.push_str(&format!("    details, not reviewed: {}\n", details.len()));
                }
                ("AI decisions to review", body)
            }
            Part::Handoff => (
                "Handoff note",
                task.handoff_note.as_deref().map(indent).unwrap_or_default(),
            ),
        };
        text.push_str(&format!("{heading}:\n"));
        text.push_str(if has && !body.is_empty() { &body } else { NONE });
    }
    let mut done = Done::new(
        json!({ "taskbag": root, "task": task.id, "order": ORDER, "evidence": ev, "risk": risk, "links": evidence::code_links(&links), "ai_decisions": decisions }),
        text,
    );
    done.issues = issues;
    Ok(done)
}
