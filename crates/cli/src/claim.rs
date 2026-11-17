//! `alm claim` and `alm base`: taking a task, and the branch it starts from and merges back into.

use std::path::Path;

use allomos_core::Problem;
use allomos_store::{Actor, load, task_language, taskbag_of};
use serde_json::json;

use crate::{
    Cli, Command, Done, blocked_text, branch_steps, claim_folder, locate, lock_wait, non_blank,
    task_json, task_line, took_over_json, took_over_text,
};

/// Runs the commands this module holds; `execute` in `lib.rs` sends only those here.
pub(crate) fn execute(cli: &Cli, cwd: &Path) -> Result<Done, Problem> {
    let me = cli.holder.as_deref();
    match &cli.command {
        Command::Claim {
            id,
            take,
            branch,
            working,
            session_agent,
            session_id,
            session_kind,
            app_session_id,
        } => {
            let root = locate(cli, cwd)?;
            // The taskbag is updated before any task is worked (Task-A03-27, approver
            // 2026-10-18): card files kept outside it are moved in first.
            let outside = allomos_store::cards::outside(&root);
            if !outside.is_empty() {
                let tasks: Vec<&str> = outside.iter().map(|c| c.task.as_str()).collect();
                return Err(Problem::ClaimCardsOutside {
                    count: outside.len().to_string(),
                    tasks: tasks.join(", "),
                });
            }
            let claimed = allomos_store::claim_task(
                &root,
                Actor::alm(me),
                lock_wait(),
                id,
                *take,
                branch.as_deref(),
                working.as_deref(),
                Some(allomos_core::AgentSession {
                    agent: non_blank(session_agent),
                    id: non_blank(session_id),
                    kind: non_blank(session_kind),
                    app_session_id: non_blank(app_session_id),
                    folder: claim_folder(&root, cwd),
                    ..Default::default()
                }),
            )?;
            let at = &claimed.branches;
            let mut text = format!("{}\n", task_line(&claimed.changed.task));
            if let Some(t) = &claimed.took_over {
                text.push_str(&took_over_text(&root, t));
            }
            // A blocked task may be claimed: alm says so and the agent decides (decision 0026,
            // detail f).
            let blocked_fact = load(&root).taskbag.and_then(|bag| {
                let by = blocked_text(&bag, &claimed.changed.task)?;
                Some(format!(
                    "Fact: this task is {by}: tasks that were to finish first have not. You decide whether to go on, work another part, or release it (`alm release {}`).",
                    claimed.changed.task.id
                ))
            });
            if let Some(fact) = &blocked_fact {
                text.push_str(fact);
                text.push('\n');
            }
            let steps = branch_steps(at);
            text.push_str(&steps.text);
            let mut done = Done::new(
                json!({
                    "taskbag": root,
                    "blocked_fact": blocked_fact,
                    "task": task_json(&claimed.changed.task),
                    "branches": at.branches,
                    "base_why": at.why,
                    "branch_commands": { "start": steps.start, "merge": steps.merge },
                    "branch_facts": steps.facts,
                    "took_over": claimed.took_over.as_ref().map(|t| took_over_json(&root, t)),
                }),
                text,
            );
            // Before planning: the size of the files to change, in the task language
            // (Task-A03-tide388). alm reminds; the agent checks.
            let system: Vec<String> = sys_locale::get_locales().collect();
            let language = taskbag_of(&root, load(&root))
                .map(|(bag, _)| task_language(&bag.project, &system).language)
                .unwrap_or_else(|_| "en".to_string());
            let reminder = allomos_store::claimed_task_steps(&language);
            done.fields["before_planning"] = json!(reminder);
            done.after = reminder;
            done.warnings = claimed.changed.warnings;
            done.issues = claimed.changed.issues;
            Ok(done)
        }
        Command::Base { id, branch } => {
            let root = locate(cli, cwd)?;
            let (task, after, issues) =
                allomos_store::set_base(&root, Actor::alm(me), lock_wait(), id, branch)?;
            let mut text = format!("{}\n", task_line(&task));
            if let Some(base) = &after.base {
                text.push_str(&format!("  base branch: {base}\n"));
            }
            let mut done = Done::new(
                json!({ "taskbag": root, "task": task_json(&task), "branches": after }),
                text,
            );
            done.issues = issues;
            Ok(done)
        }
        _ => unreachable!("lib.rs sends only this module's commands here"),
    }
}
