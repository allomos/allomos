//! `alm claim` and `alm base`: taking a task, and the branch it starts from and merges back into.

use std::path::Path;

use allomos_core::Problem;
use allomos_store::{Actor, load, task_language, taskbag_of};
use serde_json::{Value, json};

use crate::{
    Cli, Command, Done, blocked_text, heartbeat, locate, lock_wait, non_blank, quoted,
    reopen_lines, task_json, task_line,
};

/// Whether `ALLOMOS_MONITORED` is set to a truthy value (`1`, `true`, `yes`).
fn env_monitored() -> bool {
    std::env::var("ALLOMOS_MONITORED")
        .ok()
        .is_some_and(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
}

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
            monitored,
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
                    monitored: *monitored || env_monitored(),
                    ..Default::default()
                }),
            )?;
            let at = &claimed.branches;
            let reopen_reasons = allomos_store::reopen::active_for(&root, &claimed.changed.task);
            let mut text = format!("{}\n", task_line(&claimed.changed.task));
            text.push_str(&reopen_lines(&reopen_reasons, "  "));
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
                    "reopen_reasons": reopen_reasons,
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
            // The project's line limit too (Task-A03-dapper008).
            let (language, limit) = taskbag_of(&root, load(&root))
                .map(|(bag, _)| {
                    (
                        task_language(&bag.project, &system).language,
                        allomos_core::file_size::limit(&bag.project),
                    )
                })
                .unwrap_or_else(|_| ("en".to_string(), allomos_core::file_size::DEFAULT_LIMIT));
            let reminder = allomos_store::claimed_task_steps(&language, limit);
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

/// What `alm claim` says about branches: the base and working branch, the Git commands to start
/// from the base and to merge back into it, and facts the agent should check (Task-A03-22). alm
/// runs none of them (decision 0017).
pub(crate) struct BranchSteps {
    text: String,
    start: Vec<String>,
    merge: Vec<String>,
    facts: Vec<String>,
}

pub(crate) fn branch_steps(at: &allomos_core::branch::AtClaim) -> BranchSteps {
    let mut steps = BranchSteps {
        text: String::new(),
        start: Vec::new(),
        merge: Vec::new(),
        facts: Vec::new(),
    };
    let Some(base) = at.branches.base.as_deref() else {
        steps.text.push_str(
            "This task has no base branch yet. Claim it again with --branch <the branch you are on> to record it.\n",
        );
        return steps;
    };
    let why = at.why.as_ref().map(|w| w.english(base)).unwrap_or_default();
    steps
        .text
        .push_str(&format!("  base branch: {base} ({why})\n"));
    let Some(working) = at.branches.working.as_deref() else {
        steps.text.push_str(&format!(
            "Start from {base} and merge back into {base}. Claim it again with --branch <the branch you are on> for the steps (with the Git commands as the example).\n"
        ));
        return steps;
    };
    steps
        .text
        .push_str(&format!("  working branch: {working}\n"));
    let working = Some(working);
    let current = at.current.as_deref();
    let b = quoted(base);
    match working {
        Some(w) if w != base => {
            let wq = quoted(w);
            // Always in a worktree of its own (Task-A03-38): the main checkout stays on the
            // base, so the merge runs there and the worktree is removed afterwards.
            if current != Some(w) {
                // A branch that exists already (a takeover, a claim after a release) is
                // continued, not started again from the base (Task-A01-sign613).
                let add = if at.continues {
                    format!("git worktree add .worktrees/<short-name> {wq}")
                } else {
                    format!("git worktree add .worktrees/<short-name> -b {wq} {b}")
                };
                steps.start = vec![
                    format!("git switch {b}"),
                    "git pull --ff-only".to_string(),
                    add,
                ];
                if at.continues {
                    steps.facts.push(format!(
                        "The work goes on on {w}, the task's working branch already: start your worktree from it (with what was shared on it), not as a new branch from {base}. If your version control tool says {w} is checked out in another worktree already, that worktree is not yours: leave it as it is and ask the person (in a quiet run, park the question with alm wait)."
                    ));
                }
            }
            steps.merge = vec![
                format!("git switch {b}"),
                "git pull --ff-only".to_string(),
                format!("git merge {wq}"),
                "git push".to_string(),
                "git worktree remove .worktrees/<short-name>".to_string(),
                format!("git branch -d {wq}"),
            ];
        }
        _ => {
            if current != Some(base) {
                steps.start.push(format!("git switch {b}"));
            }
            steps.start.push("git pull --ff-only".to_string());
            steps.facts.push(format!(
                "Your working branch is {base}, the task's base. Work every task in a worktree of its own, on a branch of its own: claim again with --working <new branch>."
            ));
        }
    }
    if let Some(c) = current.filter(|c| *c != base) {
        steps
            .facts
            .push(format!("You are on {c}, not on {base}, the task's base."));
    }
    steps.text.push_str(&format!(
        "Start from {base} and merge back into {base} (alm runs no version control tool; for Git, these are the commands):\n"
    ));
    if steps.start.is_empty() {
        steps
            .text
            .push_str("  To start: you are on the working branch already.\n");
    } else {
        steps.text.push_str("  To start:\n");
        for c in &steps.start {
            steps.text.push_str(&format!("    {c}\n"));
        }
        if working.is_some_and(|w| w != base) {
            steps.text.push_str(
                "    (run these in the main checkout, which stays on the base; then work in .worktrees/<short-name> under the current project directory)\n",
            );
        }
    }
    if steps.merge.is_empty() {
        steps.text.push_str(&format!(
            "  You work on {base} itself: save and share your changes there (in Git: commit and push); there is nothing to merge.\n"
        ));
    } else {
        steps
            .text
            .push_str("  When the task is done, merge it into its base, in the main checkout:\n");
        for c in &steps.merge {
            steps.text.push_str(&format!("    {c}\n"));
        }
    }
    for f in &steps.facts {
        steps.text.push_str(&format!("Fact: {f}\n"));
    }
    steps
}

/// The folder `alm claim` runs in, relative to the folder holding `.allomos/`
/// (Task-A03-hive049): never absolute, so no user name is written; `None` when there is no
/// relative way.
/// Where the former holder of a taken-over task worked, as a path: its claim's folder, read from
/// the folder holding `.allomos/` (Task-A01-sign613).
pub(crate) fn former_folder(root: &Path, t: &allomos_store::TookOver) -> Option<String> {
    let folder = t.folder.as_deref()?;
    let path = allomos_store::allomos_folder(root).join(folder);
    let path = std::fs::canonicalize(&path).unwrap_or(path);
    Some(path.to_string_lossy().into_owned())
}

/// What `alm claim` says after taking a task over (decision 0022 § 6, Task-A01-sign613): from
/// whom, and that the former holder's worktree may still hold uncommitted changes that are not
/// this session's to change. alm does not look inside it (`alm` tells, the agent checks).
pub(crate) fn took_over_text(root: &Path, t: &allomos_store::TookOver) -> String {
    let mut text = format!(
        "  taken over from: {}{}\n",
        t.from,
        if t.lost.is_some() {
            " (lost)"
        } else {
            " (with --take)"
        }
    );
    let place = match former_folder(root, t) {
        Some(f) => format!("its claim ran in {f}"),
        None => "alm does not know where".to_string(),
    };
    if let Some(words) = &t.doing {
        text.push_str(&format!(
            "  it was doing: {}\n",
            heartbeat::doing_line(words, t.doing_at.as_deref(), allomos_store::now_secs())
        ));
    }
    text.push_str(
        "Fact: only what was pushed counts: go on from the commits on the working branch and the handoff note.\n",
    );
    text.push_str(&format!(
        "Fact: {}'s worktree may still hold changes it did not save or share ({place}), but that folder is of use only on the computer it ran on; on another computer, go by the working branch and the handoff note. On the same computer, look before you redo anything, but do not change, commit or remove what is there: it is not yours.\n",
        t.from
    ));
    text
}

pub(crate) fn took_over_json(root: &Path, t: &allomos_store::TookOver) -> Value {
    json!({
        "from": t.from,
        "lost": t.lost,
        "folder": t.folder,
        "folder_path": former_folder(root, t),
        "working_branch": t.working,
        "doing": t.doing,
        "doing_at": t.doing_at,
    })
}

pub(crate) fn claim_folder(root: &Path, cwd: &Path) -> Option<String> {
    let from = std::fs::canonicalize(allomos_store::allomos_folder(root)).ok()?;
    let to = std::fs::canonicalize(cwd).ok()?;
    let path = allomos_store::relative_path(&from, &to)?;
    Some(path.to_string_lossy().replace('\\', "/"))
}
