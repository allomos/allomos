//! `alm list`, `alm show`, `alm history` and `alm next`: what the taskbag holds, read without changing it.

use std::path::Path;

use allomos_core::decision::Decision;
use allomos_core::ops::{self};
use allomos_core::{Problem, State, Task};
use allomos_store::handoff;
use allomos_store::{events, load, taskbag_of};
use serde_json::{Value, json};

use crate::{
    CHOOSING_RULE, Cli, Command, Done, READ_BLOCKERS_FIRST, approve, blocked_text, classify,
    decision_lines, effects, event_lines, exceptions, finished_blockers_text, folders, heartbeat,
    indent, iteration_heading, links, locate, next_cards_line, next_handoffs_line, notes_lines,
    rules_line, stop_lines, task_json_in, task_line, task_line_in, traces_text,
};

/// Runs the commands this module holds; `execute` in `lib.rs` sends only those here.
pub(crate) fn execute(cli: &Cli, cwd: &Path) -> Result<Done, Problem> {
    let me = cli.holder.as_deref();
    match &cli.command {
        Command::List {
            area,
            iteration,
            by,
            state,
            blocked,
            mine,
            without_human_check,
            folder,
            difficulty,
        } => {
            let root = locate(cli, cwd)?;
            let (bag, issues) = taskbag_of(&root, load(&root))?;
            if let Some(f) = folder {
                allomos_core::folders::check(&bag.project, std::slice::from_ref(f))?;
            }
            if *mine && me.is_none() {
                return Err(Problem::HolderMissing);
            }
            if let Some(a) = area {
                if !bag.areas.iter().any(|x| &x.id == a) {
                    return Err(Problem::UnknownArea { area: a.clone() });
                }
            }
            if let Some(i) = iteration {
                if !bag.iterations.iter().any(|x| &x.id == i) {
                    return Err(Problem::UnknownIteration {
                        iteration: i.clone(),
                    });
                }
            }
            let keep = |t: &&Task| {
                area.as_ref().is_none_or(|a| &t.area == a)
                    && iteration
                        .as_ref()
                        .is_none_or(|i| t.iteration.as_ref() == Some(i))
                    && state.as_ref().is_none_or(|s| t.state.as_str() == s)
                    && (!*blocked || ops::is_blocked(&bag, t))
                    && (!*mine || t.holder.as_deref() == me)
                    && (!*without_human_check || t.to_check())
                    && folder.as_ref().is_none_or(|f| t.folders.contains(f))
                    && difficulty.is_none_or(|r| r.holds(t))
            };
            // In the taskbag's order: numbered IDs, then word IDs as they were made (A01-54).
            let tasks: Vec<&Task> = bag.tasks.iter().filter(keep).collect();
            let mut text = String::new();
            // `always`: the heading is shown even with no tasks to list (each iteration,
            // Task-A00-storm072); other groups only when they list some.
            let mut group = |heading: String, members: &[&&Task], always: bool| {
                if members.is_empty() && !always {
                    return;
                }
                text.push_str(&heading);
                text.push('\n');
                if members.is_empty() {
                    text.push_str("  (none)\n");
                }
                for t in members {
                    text.push_str(&format!("  {}\n", task_line_in(&bag, t)));
                    for line in stop_lines(&bag, t) {
                        text.push_str(&format!("      {line}\n"));
                    }
                }
            };
            let tasks_json = |members: &[&&Task]| {
                members
                    .iter()
                    .map(|t| task_json_in(&bag, t))
                    .collect::<Vec<_>>()
            };
            let fields = if by == "iteration" {
                let active = ops::active_iteration(&bag).map(|i| i.id.clone());
                let mut groups = Vec::new();
                // Each iteration's place in the plan, counted from 1 in plan order (the taskbag
                // keeps them by `order`, then ID), as the app shows it (Task-A00-turkey877): shown,
                // never stored, so it follows `alm move iteration` (Task-A00-storm072).
                for (place, it) in (1..).zip(&bag.iterations) {
                    if iteration.as_ref().is_some_and(|x| x != &it.id) {
                        continue;
                    }
                    let members: Vec<&&Task> = tasks
                        .iter()
                        .filter(|t| t.iteration.as_ref() == Some(&it.id))
                        .collect();
                    let is_active = active.as_ref() == Some(&it.id);
                    let mut heading = format!("{place}  {}", iteration_heading(it));
                    // The first planned iteration, active because none is marked (Task-A01-root343).
                    if is_active && it.state.is_planned() {
                        heading.push_str("  [active]");
                    }
                    group(heading, &members, true);
                    let mut g = json!(it);
                    g["place"] = json!(place);
                    g["active"] = json!(is_active);
                    g["tasks"] = json!(tasks_json(&members));
                    groups.push(g);
                }
                // Tasks with no iteration, or one that does not exist, come last.
                let unplanned: Vec<&&Task> = tasks
                    .iter()
                    .filter(|t| {
                        t.iteration
                            .as_ref()
                            .is_none_or(|i| !bag.iterations.iter().any(|x| &x.id == i))
                    })
                    .collect();
                group("Unplanned".to_string(), &unplanned, false);
                json!({ "taskbag": root, "iterations": groups, "unplanned": tasks_json(&unplanned) })
            } else {
                let mut groups = Vec::new();
                for a in &bag.areas {
                    if area.as_ref().is_some_and(|x| x != &a.id) {
                        continue;
                    }
                    let members: Vec<&&Task> = tasks.iter().filter(|t| t.area == a.id).collect();
                    group(format!("{}  {}", a.id, a.name), &members, false);
                    groups.push(json!({
                        "id": a.id, "name": a.name, "order": a.order,
                        "tasks": tasks_json(&members),
                    }));
                }
                json!({ "taskbag": root, "areas": groups })
            };
            if text.is_empty() {
                text.push_str("No tasks.\n");
            }
            // The card files still outside the taskbag, which the app's notice counts
            // (Task-A01-51): the same facts as `alm cards`.
            let mut fields = fields;
            fields["cards_outside"] = json!(allomos_store::cards::outside_of(&root, &bag));
            let mut done = Done::new(fields, text);
            done.issues = issues;
            Ok(done)
        }
        Command::Show { id } => {
            let root = locate(cli, cwd)?;
            let (bag, issues) = taskbag_of(&root, load(&root))?;
            let task = ops::find_task(&bag, id)?;
            let mut text = String::new();
            let state_shown = match blocked_text(&bag, &task) {
                Some(by) => format!("{} · {by}", task.state_text()),
                None => task.state_text(),
            };
            let blocked = (!task.blocked_by.is_empty()).then(|| {
                ops::blockers(&bag, &task)
                    .iter()
                    .map(ops::Blocker::describe)
                    .collect::<Vec<_>>()
                    .join(", ")
            });
            let folders_text = (!task.folders.is_empty()).then(|| task.folders.join(", "));
            let difficulty_text = allomos_core::difficulty::text(&task);
            let labels_text = (!task.labels.is_empty()).then(|| task.labels.join(", "));
            let heartbeat_text = task
                .heartbeat
                .as_ref()
                .map(|h| heartbeat::heartbeat_line(h, allomos_store::now_secs()));
            let doing_text = allomos_core::heartbeat::doing(&task)
                .map(|(w, at)| heartbeat::doing_line(w, at, allomos_store::now_secs()));
            let fields = [
                ("id", Some(task.id.as_str())),
                ("title", Some(task.title.as_str())),
                ("area", Some(task.area.as_str())),
                ("iteration", task.iteration.as_deref()),
                ("folders", folders_text.as_deref()),
                ("difficulty", Some(difficulty_text.as_str())),
                ("state", Some(state_shown.as_str())),
                ("labels", labels_text.as_deref()),
                ("holder", task.holder.as_deref()),
                ("heartbeat", heartbeat_text.as_deref()),
                ("doing", doing_text.as_deref()),
                ("waiting on", task.waiting_on.as_deref()),
                ("waiting for", task.who.as_deref()),
                ("blockers", blocked.as_deref()),
                ("next dependency", task.next_dependency.as_deref()),
                ("handoff note", task.handoff_note.as_deref()),
                ("not human-checked", task.without_human_check.as_deref()),
            ];
            // For `alm title` / `alm describe --expected-version` (Task-A01-57).
            let version = ops::task_version(&task);
            for (name, value) in fields {
                if let Some(v) = value {
                    text.push_str(&format!("{name}: {v}\n"));
                }
            }
            if let Some(steps) = &task.human_check_steps {
                text.push_str(&format!("human check steps:\n{}", indent(steps)));
            }
            text.push_str(&format!("version: {version}\n"));
            let branches = allomos_store::branches(&root)
                .remove(&task.id)
                .unwrap_or_default();
            for (name, value) in [
                ("base branch", &branches.base),
                ("working branch", &branches.working),
            ] {
                if let Some(v) = value {
                    text.push_str(&format!("{name}: {v}\n"));
                }
            }
            let traces = allomos_store::traces(&root)
                .remove(&task.id)
                .unwrap_or_default();
            if !traces.is_empty() {
                text.push_str(&format!("traces: {}\n", traces_text(&traces)));
            }
            let decisions = allomos_store::ai_decisions(&root)
                .remove(&task.id)
                .unwrap_or_default();
            let (review, details): (Vec<Decision>, Vec<Decision>) =
                decisions.iter().cloned().partition(|d| d.to_review());
            if !review.is_empty() {
                text.push_str("AI decided (to review):\n");
                text.push_str(&decision_lines(&review));
            }
            if !details.is_empty() {
                text.push_str("AI decided (details, not reviewed):\n");
                text.push_str(&decision_lines(&details));
            }
            let approvals = allomos_store::approvals::approvals(&root)
                .remove(&task.id)
                .unwrap_or_default();
            text.push_str(&approve::lines(&approvals));
            let classification = allomos_store::classify::by_task(&root)
                .remove(&task.id)
                .unwrap_or_default();
            text.push_str(&classify::lines(&task.id, &classification, &approvals));
            let outside_effects = allomos_store::effects::by_task(&root)
                .remove(&task.id)
                .unwrap_or_default();
            text.push_str(&effects::lines(&outside_effects));
            let task_exceptions = allomos_store::exceptions::by_task(&root)
                .remove(&task.id)
                .unwrap_or_default();
            text.push_str(&exceptions::lines(&task_exceptions));
            let task_links = allomos_store::links::by_task(&root)
                .remove(&task.id)
                .unwrap_or_default();
            text.push_str(&links::lines(&task_links));
            let withdrawn = allomos_core::approval::withdrawn_notice(&task, &approvals);
            if let Some(r) = withdrawn {
                text.push_str(&approve::withdrawn_line(
                    &task.id,
                    task.holder.as_deref(),
                    r,
                ));
            }
            // Whoever takes over checks before trusting the handoff note and repeats nothing
            // (Task-A03-23).
            let taking_over = task.handoff_note.as_ref().map(|_| handoff::TAKING_OVER);
            if let Some(rules) = taking_over {
                text.push_str(rules);
            }
            if let Some(line) = finished_blockers_text(&bag, &task) {
                text.push_str(&format!("{line}: {READ_BLOCKERS_FIRST}.\n"));
            }
            if !task.description.is_empty() {
                text.push('\n');
                text.push_str(&task.description);
            }
            let mut done = Done::new(
                json!({ "taskbag": root, "task": task_json_in(&bag, &task), "branches": branches, "traces": traces, "ai_decisions": decisions, "approvals": approvals, "classification": classification, "effects": outside_effects, "exceptions": task_exceptions, "links": task_links, "approval_withdrawn": withdrawn.map(|r| &r.approval.id), "taking_over": taking_over }),
                text,
            );
            done.issues = issues;
            Ok(done)
        }
        Command::History { id } => {
            let root = locate(cli, cwd)?;
            let (bag, mut issues) = taskbag_of(&root, load(&root))?;
            let task = ops::find_task(&bag, id)?;
            let history = events::history(&root, &task.id);
            let mut text = match &history.since {
                Some(since) => format!("History of {} (recorded since {since})\n", task.id),
                None => format!(
                    "No history yet: this taskbag has recorded no changes, so none are shown for {}.\n",
                    task.id
                ),
            };
            if history.since.is_some() && history.events.is_empty() {
                text.push_str("No changes recorded for this task yet.\n");
            }
            for e in &history.events {
                text.push_str(&event_lines(e));
            }
            issues.extend(history.issues.iter().cloned());
            let mut done = Done::new(
                json!({ "taskbag": root, "task": task.id, "since": history.since, "events": history.events }),
                text,
            );
            done.issues = issues;
            Ok(done)
        }
        Command::Next {
            iteration,
            all_folders,
            difficulty,
        } => {
            let root = locate(cli, cwd)?;
            let (bag, issues) = taskbag_of(&root, load(&root))?;
            // `alm` lists; the agent chooses (Task-A03-14).
            let mut tasks = ops::next(&bag, iteration.as_deref())?;
            // Run inside a project folder: only its tasks, without those that have no folder
            // (approver, 2026-10-25; Task-A01-dime494).
            let here = (!*all_folders)
                .then(|| folders::folder_here(&root, &bag.project, cwd))
                .flatten();
            if let Some(name) = &here {
                tasks = allomos_core::folders::in_folder(tasks, name);
            }
            // Only the difficulties asked for (Task-A01-actor813), as a monitor chooses them.
            if let Some(range) = difficulty {
                tasks.retain(|t| range.holds(t));
            }
            // Held tasks (decision 0022, Task-A01-zero387): the lost ones and the busy ones, in
            // the same iteration and folder as the list.
            let in_scope: Vec<&Task> = bag
                .tasks
                .iter()
                .filter(|t| {
                    iteration
                        .as_ref()
                        .is_none_or(|i| t.iteration.as_ref() == Some(i))
                })
                .collect();
            let in_scope = match &here {
                Some(name) => allomos_core::folders::in_folder(in_scope, name),
                None => in_scope,
            };
            let held = heartbeat::held(&root, &bag, &in_scope, allomos_store::now_secs());
            let instruction = if tasks.is_empty() && held.any_lost {
                "No ready task is free. The tasks listed as lost could be taken over: check each one first (who holds it, its handoff note), then decide; or ask the person what to do next.".to_string()
            } else if tasks.is_empty() {
                match iteration {
                    Some(i) => format!(
                        "Nothing to take: no ready task in iteration {i} is free. Ask the person what to do next."
                    ),
                    None => {
                        "Nothing to take: no ready task is free. Ask the person what to do next."
                            .to_string()
                    }
                }
            } else {
                CHOOSING_RULE.to_string()
            };
            let mut text = String::new();
            if let Some(name) = &here {
                text.push_str(&format!(
                    "Inside project folder {name}: only its tasks are listed (tasks with no folder are left out). `alm next --all-folders` lists every task.\n\n"
                ));
            }
            // The active iteration all done: suggest the next step; alm changes no state
            // (approver, 2026-10-22; Task-A01-root343).
            let finished = ops::finished_active(&bag).map(|(done_it, next_it)| {
                let words = match next_it {
                    // Active only because it is the first planned: closing it is enough, the next
                    // planned one is then active the same way.
                    Some(n) if done_it.state.is_planned() => format!(
                        "{} is all done (every task done or dropped). Close it? `alm change iteration {} --state closed` (ask the person first unless they set this up); {}, the next planned iteration, is then active.",
                        done_it.id, done_it.id, n.id
                    ),
                    Some(n) => format!(
                        "{} is all done (every task done or dropped). Close it and make {} active? `alm change iteration {} --state closed`, then `alm change iteration {} --state active` (ask the person first unless they set this up).",
                        done_it.id, n.id, done_it.id, n.id
                    ),
                    None => format!(
                        "{} is all done (every task done or dropped). Close it? `alm change iteration {} --state closed` (ask the person first unless they set this up); no planned iteration follows it.",
                        done_it.id, done_it.id
                    ),
                };
                (done_it.id.clone(), next_it.map(|n| n.id.clone()), words)
            });
            if let Some((_, _, words)) = &finished {
                text.push_str(words);
                text.push_str("\n\n");
            }
            let mut listed = Vec::new();
            for t in &tasks {
                text.push_str(&format!("{}\n", task_line(t)));
                if let Some(i) = &t.iteration {
                    text.push_str(&format!("  iteration: {i}\n"));
                }
                if let Some(n) = &t.next_dependency {
                    text.push_str(&format!("  next dependency: {n}\n"));
                }
                if let Some(note) = &t.handoff_note {
                    text.push_str(&format!("  handoff note: {note}\n"));
                }
                if let Some(line) = finished_blockers_text(&bag, t) {
                    text.push_str(&format!("  {line}: {READ_BLOCKERS_FIRST}.\n"));
                }
                listed.push(task_json_in(&bag, t));
            }
            if !tasks.is_empty() {
                text.push('\n');
            }
            // Once, when a listed task has a handoff note: the rules for taking over (Task-A03-23).
            let taking_over = tasks
                .iter()
                .any(|t| t.handoff_note.is_some())
                .then_some(handoff::TAKING_OVER);
            if let Some(rules) = taking_over {
                text.push_str(rules);
                text.push('\n');
            }
            text.push_str(&held.text);
            text.push_str(&instruction);
            text.push('\n');
            // An approval withdrawn while a task is in progress (approver, 2026-11-08,
            // Task-A01-baker800): said here too; the holder decides whether to stop.
            let approvals = allomos_store::approvals::approvals(&root);
            let mut withdrawn = Vec::new();
            for t in bag.tasks.iter().filter(|t| {
                iteration
                    .as_ref()
                    .is_none_or(|i| t.iteration.as_ref() == Some(i))
            }) {
                let list = approvals.get(&t.id).map(Vec::as_slice).unwrap_or_default();
                if let Some(r) = allomos_core::approval::withdrawn_notice(t, list) {
                    text.push_str(&approve::withdrawn_line(&t.id, t.holder.as_deref(), r));
                    withdrawn.push(json!({ "task": t.id, "approval": r.approval.id }));
                }
            }
            // Ready tasks that are blocked are not listed above (decision 0026): said here, each
            // with its open blockers, so nobody wonders where they went. alm changes nothing;
            // each shows up above by itself once its blockers have finished.
            let mut blocked = Vec::new();
            for t in bag.tasks.iter().filter(|t| {
                t.state == State::Ready
                    && t.holder.is_none()
                    && iteration
                        .as_ref()
                        .is_none_or(|i| t.iteration.as_ref() == Some(i))
            }) {
                let Some(by) = blocked_text(&bag, t) else {
                    continue;
                };
                if blocked.is_empty() {
                    text.push_str("Ready but blocked (not listed above; each is listed once its blockers have finished):\n");
                }
                text.push_str(&format!("  {}  {by}  {}\n", t.id, t.title));
                blocked.push(task_json_in(&bag, t));
            }
            // Card files (Task-A00-30): what the tasks' `Card:` links point to outside the
            // taskbag and what agents reported; nothing when there are none (Task-A03-34).
            let outside = allomos_store::cards::outside(&root);
            let cards = next_cards_line(outside.len());
            text.push_str(&cards);
            // Old handoff files (Task-A03-polar770): the taskbag needs updating; alm only says
            // they are there.
            let old_handoffs = handoff::legacy(&root);
            let handoffs_line = next_handoffs_line(&old_handoffs);
            text.push_str(&handoffs_line);
            // At the start of a session: when this agent must write its handoff (Task-A03-06).
            let rules = bag.project.handoff.clone().unwrap_or_default().applied(me);
            text.push_str(&rules_line(&rules));
            // What every session must know: the project notes' Running and Pitfalls, and when
            // they last changed (Task-A03-20). Facts only.
            let notes = handoff::project_notes(&root);
            text.push('\n');
            text.push_str(&notes_lines(&notes));
            let mut done = Done::new(
                json!({
                    "taskbag": root,
                    "tasks": listed,
                    "lost": held.lost,
                    "busy": held.busy,
                    "doing": held.doing,
                    "heartbeat_settings": allomos_core::heartbeat::applied(&bag.project),
                    "instruction": instruction,
                    "taking_over": taking_over,
                    "blocked": blocked,
                    "approval_withdrawn": withdrawn,
                    "handoff_rules": rules,
                    "project_notes": notes,
                    "cards_outside": outside,
                    "cards": if cards.is_empty() { Value::Null } else { json!(cards.trim_end()) },
                    "old_handoffs": old_handoffs,
                    "handoffs": if handoffs_line.is_empty() { Value::Null } else { json!(handoffs_line.trim_end()) },
                    "active_iteration": ops::active_iteration(&bag).map(|i| i.id.clone()),
                    "folder": here,
                    "finished_iteration": finished.as_ref().map(|(done_it, next_it, words)| json!({ "iteration": done_it, "next": next_it, "suggestion": words })),
                }),
                text,
            );
            done.issues = issues;
            Ok(done)
        }
        _ => unreachable!("lib.rs sends only this module's commands here"),
    }
}
