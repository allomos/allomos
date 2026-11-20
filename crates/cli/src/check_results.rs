//! `alm check-result` (Task-A01-easel128, decision 0029 § 5): one check's result on a task, as a
//! record in its history, and the lines `alm show` lists them with, failed and not run first. A
//! failed check may bring screenshots (Task-A01-fondue470, § 6), copied into the taskbag.

use std::path::{Path, PathBuf};

use allomos_core::Problem;
use allomos_core::check_result::Recorded;
use allomos_store::check_results;
use allomos_store::events::Actor;
use serde_json::json;

use crate::{Cli, Done, locate, lock_wait, task_line};

pub(crate) fn record(
    cli: &Cli,
    cwd: &Path,
    id: &str,
    name: &str,
    result: &str,
    detail: Option<&str>,
    screenshots: &[PathBuf],
) -> Result<Done, Problem> {
    let root = locate(cli, cwd)?;
    let screenshots: Vec<PathBuf> = screenshots.iter().map(|p| cwd.join(p)).collect();
    let c = check_results::record(
        &root,
        Actor::alm(cli.holder.as_deref()),
        lock_wait(),
        id,
        name,
        result,
        detail,
        &screenshots,
    )?;
    let mut out = format!("{}\n", task_line(&c.task));
    let what = if c.changed {
        "recorded"
    } else {
        "already recorded so; nothing changed"
    };
    out.push_str(&format!("{}: {} {what}.\n", c.check.name, c.check.result));
    out.push_str(&lines(&c.task.id, &c.results));
    // alm tells; the agent checks (AGENTS.md): it runs no check and judges none.
    out.push_str(
        "Fact: alm records what you report; it runs no check. Record each check you ran or did not run; the same name again replaces its result.\n",
    );
    let mut done = Done::new(
        json!({ "taskbag": root, "task": c.task.id, "check_result": c.check, "changed": c.changed, "check_results": c.results }),
        out,
    );
    done.issues = c.issues;
    Ok(done)
}

/// The lines `alm show` and `alm check-result` list a task's results with: failed, then not run,
/// then passed. Empty when it has none.
pub(crate) fn lines(task: &str, results: &[Recorded]) -> String {
    if results.is_empty() {
        return String::new();
    }
    let mut out = String::from("Check results:\n");
    for r in allomos_core::check_result::ordered(results) {
        let result = match r.check.result.as_str() {
            "fail" => "FAIL",
            "not-run" => "NOT RUN",
            "pass" => "PASS",
            other => other,
        };
        out.push_str(&format!("  {result:<7}  {}", r.check.name));
        if let Some(d) = &r.check.detail {
            out.push_str(&format!(": {d}"));
        }
        out.push('\n');
        for s in &r.check.screenshots {
            out.push_str(&format!(
                "           screenshot: tasks/{task}/{}/{s}\n",
                allomos_core::check_result::EVIDENCE_DIR
            ));
        }
    }
    out
}
