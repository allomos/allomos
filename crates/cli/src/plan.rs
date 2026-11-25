//! `alm add`, `alm rename`, `alm change`, `alm move` and `alm delete`: areas, iterations, labels
//! and tasks being added, renamed, moved and removed.

use std::path::Path;

use allomos_core::ops;
use allomos_core::{Iteration, IterationState, Problem};
use allomos_store::{Actor, load, task_language, taskbag_of};
use serde_json::{Value, json};

use crate::{
    Add, ChangeWhat, Cli, Command, Delete, Done, MoveWhat, Rename, id_and_name, iteration_heading,
    locate, lock_wait, task_json, task_line,
};

/// Runs the commands this module holds; `execute` in `lib.rs` sends only those here.
pub(crate) fn execute(cli: &Cli, cwd: &Path) -> Result<Done, Problem> {
    let me = cli.holder.as_deref();
    match &cli.command {
        Command::Add(add) => {
            let root = locate(cli, cwd)?;
            let actor = Actor::alm(me);
            let (mut done, issues) = match add {
                Add::Area { first, name } => {
                    let (id, name) = id_and_name(first, name);
                    let added = allomos_store::add_area(&root, actor, lock_wait(), id, name)?;
                    let area = added.item;
                    let text = format!("Added area {} ({})\n", area.id, area.name);
                    let done = Done::new(json!({ "taskbag": root, "area": area }), text);
                    (done, added.issues)
                }
                Add::Iteration {
                    first,
                    name,
                    release,
                    start,
                    end,
                } => {
                    let (id, name) = id_and_name(first, name);
                    let details = ops::IterationDetails {
                        release: release.as_deref(),
                        start: start.as_deref(),
                        end: end.as_deref(),
                    };
                    let added =
                        allomos_store::add_iteration(&root, actor, lock_wait(), id, name, details)?;
                    let iteration = added.item;
                    let text = format!("Added iteration {}\n", iteration_heading(&iteration));
                    let done = Done::new(json!({ "taskbag": root, "iteration": iteration }), text);
                    (done, added.issues)
                }
                Add::Task {
                    id,
                    area,
                    title,
                    description,
                    branch,
                    base,
                    iteration,
                    folders,
                    no_folder,
                    difficulty,
                } => {
                    let current = taskbag_of(&root, load(&root)).ok();
                    // While the taskbag has folders, the agent decides one, several or none
                    // (approver, 2026-10-25; Task-A01-dime494): alm does not guess.
                    if let Some((bag, _)) = &current {
                        if !bag.project.project_folders.is_empty()
                            && folders.is_empty()
                            && !no_folder
                        {
                            return Err(Problem::FolderChoiceMissing {
                                folders: allomos_core::folders::names(&bag.project),
                            });
                        }
                    }
                    // A new task goes into the active iteration unless another is named
                    // (approver, 2026-10-22; Task-A01-root343).
                    let active = current
                        .as_ref()
                        .and_then(|(bag, _)| ops::active_iteration(bag).map(|i| i.id.clone()));
                    let planned_in = iteration.clone().or(active.clone());
                    let new = allomos_store::NewTask {
                        id: id.clone(),
                        area: area.clone(),
                        title: title.clone(),
                        description: description.clone(),
                        iteration: planned_in.clone(),
                        branch: branch.clone(),
                        base: base.clone(),
                        folders: folders.clone(),
                        difficulty: *difficulty,
                    };
                    let added = allomos_store::add_task_with_base(&root, actor, lock_wait(), &new)?;
                    let task = added.task;
                    let mut text = format!("Added {}\n", task_line(&task));
                    if let Some(c) = &added.base {
                        text.push_str(&format!("  base branch: {}\n", c.english()));
                    }
                    // What to do next, in the task language: plan it, then make it ready
                    // unless something is the person's to decide (Task-A03-33).
                    let system: Vec<String> = sys_locale::get_locales().collect();
                    let (language, iterations, wait_for_approval) =
                        match taskbag_of(&root, load(&root)) {
                            Ok((bag, _)) => (
                                task_language(&bag.project, &system).language,
                                bag.iterations.iter().map(iteration_heading).collect(),
                                allomos_core::new_tasks::wait_for_approval(&bag.project),
                            ),
                            Err(_) => ("en".to_string(), Vec::new(), false),
                        };
                    if let Some(i) = &task.iteration {
                        let why = if iteration.is_some() {
                            "as asked"
                        } else {
                            "the active iteration"
                        };
                        text.push_str(&format!("  iteration: {i} ({why})\n"));
                    }
                    let steps = allomos_store::added_task_steps(
                        &language,
                        &task.id,
                        &iterations,
                        task.iteration.as_deref(),
                        wait_for_approval,
                    );
                    // No difficulty given (Task-A01-actor813): alm says so and how to give it; it
                    // refuses nothing (the task counts as 3).
                    let no_difficulty = task.difficulty.is_none().then(|| {
                        let fact = format!(
                            "Task {}: no difficulty is given, so it counts as 3 (an ordinary task). Give it, from 1 (very easy) to 5 (very hard); `alm guide tasks` says what each level means.",
                            task.id
                        );
                        let fix = format!("alm difficulty {} <1-5>", task.id);
                        text.push_str(&format!("\nFact: {fact} Fix: `{fix}`\n"));
                        json!({ "code": "difficulty", "fact": fact, "fix": fix })
                    });
                    let mut done = Done::new(
                        json!({ "taskbag": root, "task": task_json(&task), "base": added.base, "next_steps": steps }),
                        text,
                    );
                    if let Some(m) = &no_difficulty {
                        done.fields["missing"] = json!([m]);
                    }
                    done.after = steps;
                    done.warnings = added.warnings;
                    (done, added.issues)
                }
            };
            done.issues = issues;
            Ok(done)
        }
        Command::Rename(Rename::Area { id, name }) => {
            let root = locate(cli, cwd)?;
            let renamed = allomos_store::rename_area(&root, Actor::alm(me), lock_wait(), id, name)?;
            let area = renamed.item;
            let text = format!("Renamed area {} ({})\n", area.id, area.name);
            let mut done = Done::new(json!({ "taskbag": root, "area": area }), text);
            done.issues = renamed.issues;
            Ok(done)
        }
        Command::Rename(Rename::Iteration { old, new }) => {
            let root = locate(cli, cwd)?;
            let renamed =
                allomos_store::rename_iteration(&root, Actor::alm(me), lock_wait(), old, new)?;
            let (iteration, tasks) = renamed.item;
            let mut text = if iteration.id == *old {
                format!("{old} already has that ID; nothing changed.\n")
            } else {
                format!(
                    "Renamed iteration {old} to {}\n",
                    iteration_heading(&iteration)
                )
            };
            if !tasks.is_empty() {
                let ids: Vec<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
                text.push_str(&format!(
                    "Moved {} task(s) along: {}\n",
                    ids.len(),
                    ids.join(", ")
                ));
            }
            let mut done = Done::new(
                json!({ "taskbag": root, "iteration": iteration, "tasks": tasks.iter().map(task_json).collect::<Vec<_>>() }),
                text,
            );
            done.issues = renamed.issues;
            Ok(done)
        }
        Command::Rename(Rename::Label { old, new }) => {
            let root = locate(cli, cwd)?;
            let renamed =
                allomos_store::rename_label(&root, Actor::alm(me), lock_wait(), old, new)?;
            let (labels, tasks) = renamed.item;
            let ids: Vec<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
            let mut text = format!("Label ladder: {}\n", labels.join(" → "));
            text.push_str(&if ids.is_empty() {
                "No task carried the label.\n".to_string()
            } else {
                format!("Relabelled {} task(s): {}\n", ids.len(), ids.join(", "))
            });
            let mut done = Done::new(
                json!({ "taskbag": root, "labels": labels, "tasks": tasks.iter().map(task_json).collect::<Vec<_>>() }),
                text,
            );
            done.issues = renamed.issues;
            Ok(done)
        }
        Command::Change(ChangeWhat::Iteration {
            id,
            name,
            release,
            start,
            end,
            state,
        }) => {
            let root = locate(cli, cwd)?;
            let state = match state {
                Some(s) => Some(
                    IterationState::parse(s)
                        .ok_or_else(|| Problem::InvalidIterationState { value: s.clone() })?,
                ),
                None => None,
            };
            let change = ops::IterationChange {
                name: name.clone(),
                release: release.clone(),
                start: start.clone(),
                end: end.clone(),
                state,
            };
            let changed =
                allomos_store::change_iteration(&root, Actor::alm(me), lock_wait(), id, &change)?;
            let iteration = changed.item;
            let mut text = format!("Changed iteration {}\n", iteration_heading(&iteration));
            // Closing an iteration with unfinished tasks moves none of them: alm lists them and
            // asks (approver, 2026-10-22; Task-A01-root343).
            let mut unfinished: Vec<Value> = Vec::new();
            if state == Some(IterationState::Closed) {
                if let Ok((bag, _)) = taskbag_of(&root, load(&root)) {
                    let left = ops::unfinished_in(&bag, &iteration.id);
                    if !left.is_empty() {
                        text.push_str(&format!(
                            "{} task(s) in {} are not finished; none was moved:\n",
                            left.len(),
                            iteration.id
                        ));
                        for t in &left {
                            text.push_str(&format!("  {}\n", task_line(t)));
                        }
                        text.push_str("Decide for each: plan it in another iteration (`alm plan <id> <iteration>`), finish or drop it, or leave it here. Ask the person when it is theirs to decide.\n");
                    }
                    unfinished = left.into_iter().map(task_json).collect();
                }
            }
            let mut done = Done::new(
                json!({ "taskbag": root, "iteration": iteration, "unfinished": unfinished }),
                text,
            );
            done.issues = changed.issues;
            Ok(done)
        }
        Command::Move(MoveWhat::Iteration { id, before, after }) => {
            let root = locate(cli, cwd)?;
            let place = match (before, after) {
                (Some(other), _) => ops::Place::Before(other),
                (None, Some(other)) => ops::Place::After(other),
                (None, None) => unreachable!("clap requires --before or --after"),
            };
            let moved =
                allomos_store::move_iteration(&root, Actor::alm(me), lock_wait(), id, place)?;
            let (word, other) = match place {
                ops::Place::Before(o) => ("before", o),
                ops::Place::After(o) => ("after", o),
            };
            let mut text = if moved.item.is_empty() {
                format!("{id} is already there; nothing changed.\n")
            } else {
                format!("Moved iteration {id} {word} {other}\n")
            };
            let mut order: Vec<Iteration> = Vec::new();
            if let Ok((bag, _)) = taskbag_of(&root, load(&root)) {
                order = bag.iterations;
                order.sort_by(|a, b| a.order.cmp(&b.order).then(a.id.cmp(&b.id)));
                text.push_str("The iterations, in order:\n");
                for it in &order {
                    text.push_str(&format!("  {}\n", iteration_heading(it)));
                }
            }
            let mut done = Done::new(
                json!({ "taskbag": root, "moved": moved.item, "iterations": order }),
                text,
            );
            done.issues = moved.issues;
            Ok(done)
        }
        Command::Delete(what) => {
            let root = locate(cli, cwd)?;
            let actor = Actor::alm(me);
            let done = match what {
                Delete::Area { id } => {
                    let gone = allomos_store::delete_area(&root, actor, lock_wait(), id)?;
                    let text = format!("Deleted area {} ({})\n", gone.item.id, gone.item.name);
                    let mut done = Done::new(
                        json!({ "taskbag": root, "deleted": { "area": gone.item } }),
                        text,
                    );
                    done.issues = gone.issues;
                    done
                }
                Delete::Iteration { id } => {
                    let gone = allomos_store::delete_iteration(&root, actor, lock_wait(), id)?;
                    let text = format!("Deleted iteration {}\n", iteration_heading(&gone.item));
                    let mut done = Done::new(
                        json!({ "taskbag": root, "deleted": { "iteration": gone.item } }),
                        text,
                    );
                    done.issues = gone.issues;
                    done
                }
            };
            Ok(done)
        }
        _ => unreachable!("lib.rs sends only this module's commands here"),
    }
}
