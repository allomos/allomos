//! Changing tasks from the app under the person's saved name: the task pane's edits, reviews and
//! approvals, the list's areas, iterations and new tasks, and a companion page's changes.

use std::path::{Path, PathBuf};

use allomos_core::Problem;
use allomos_store::load;
use serde::Serialize;

use crate::{LOCK_WAIT, audit, task_change};

/// Change one field of a task from the task pane (Task-A01-18), under the person's saved name.
/// The change goes through the same `crates/core` rule and the same locked, recorded write as its
/// `alm` command (`allomos_store::change_task`); a refused change writes nothing and comes back
/// as the same problem `alm` reports.
pub(crate) fn edit_task_as(
    root: &Path,
    name: Option<&str>,
    id: &str,
    edit: &allomos_core::ops::Edit,
) -> Result<allomos_core::Task, Problem> {
    change_task_as(root, name, id, edit).map(|change| change.task)
}

/// [`edit_task_as`], with what the window needs to show the change without reading the whole
/// taskbag again (Task-A01-parade485).
pub(crate) fn change_task_as(
    root: &Path,
    name: Option<&str>,
    id: &str,
    edit: &allomos_core::ops::Edit,
) -> Result<task_change::TaskChange, Problem> {
    task_change::edit(root, app_actor(name)?, LOCK_WAIT, id, edit)
}

/// The person has reviewed task `id`'s AI decisions (Task-A03-19): the same change as
/// `alm reviewed`, made as the person's saved name.
#[tauri::command(async)]
pub(crate) fn review_ai_decisions(root: PathBuf, id: String) -> Result<(), Problem> {
    review_as(&root, allomos_store::saved_owner_name().as_deref(), &id)
}

fn review_as(root: &Path, name: Option<&str>, id: &str) -> Result<(), Problem> {
    let actor = app_actor(name)?;
    allomos_store::review_ai_decisions(root, actor, LOCK_WAIT, id).map(|_| ())
}

/// Approve multiple tasks in batch (decision 0030 § 1, 2, 3):
/// Records `checked` for tasks in `check_ids`, and `reviewed` for tasks in `review_ids`.
#[tauri::command(async)]
pub(crate) fn approve_selected(
    root: PathBuf,
    check_ids: Vec<String>,
    review_ids: Vec<String>,
) -> Result<(), Problem> {
    approve_selected_as(
        &root,
        allomos_store::saved_owner_name().as_deref(),
        &check_ids,
        &review_ids,
    )
}

pub(crate) fn approve_selected_as(
    root: &Path,
    name: Option<&str>,
    check_ids: &[String],
    review_ids: &[String],
) -> Result<(), Problem> {
    let actor = app_actor(name)?;
    if !check_ids.is_empty() {
        let given = allomos_core::labels::CheckGiven {
            checker: actor.by,
            result: allomos_core::labels::CheckResult::Pass,
            words: None,
        };
        allomos_store::change_tasks(root, actor, LOCK_WAIT, |bag| {
            allomos_core::labels::checked_many(bag, check_ids, None, &given)
                .map(|o| (o, "checked"))
        })?;
        crate::audit::after_approval(root);
    }
    if !review_ids.is_empty() {
        allomos_store::review_ai_decisions_many(root, actor, LOCK_WAIT, review_ids)?;
    }
    Ok(())
}

/// The task pane's Approve (Task-A01-baker800, decision 0021 § 1.1): the same record as
/// `alm approve <id>`, channel `app`, approver this computer's person's pseudonym, made as the
/// person's saved name. `shown` is the description the confirmation showed: if the task's plan is
/// no longer exactly that, nothing is recorded (`task_changed`). Approving changes no state;
/// `ready` (Approve and make ready, only offered on a backlog task) then makes the change
/// `alm ready` makes, as a second step the person chose. The approver chose to record, not block,
/// an approval clicked by an agent operating the app (2026-11-08): the event shows it.
#[tauri::command(async)]
pub(crate) fn approve_task(
    root: PathBuf,
    id: String,
    shown: String,
    expires: Option<String>,
    ready: bool,
) -> Result<(), Problem> {
    approve_as(
        &root,
        allomos_store::saved_owner_name().as_deref(),
        &id,
        &shown,
        expires.as_deref(),
        ready,
    )
}

fn approve_as(
    root: &Path,
    name: Option<&str>,
    id: &str,
    shown: &str,
    expires: Option<&str>,
    ready: bool,
) -> Result<(), Problem> {
    let actor = app_actor(name)?;
    let tasks = [id.to_string()];
    allomos_store::approvals::approve(
        root,
        actor,
        LOCK_WAIT,
        &allomos_store::approvals::Input {
            tasks: &tasks,
            approver: None,
            channel: "app",
            words: None,
            expires,
            plan: None,
            shown: Some(shown),
        },
    )?;
    audit::after_approval(root);
    if ready {
        edit_task_as(
            root,
            name,
            id,
            &allomos_core::ops::Edit::State {
                state: allomos_core::State::Ready,
                waiting_on: None,
                who: None,
            },
        )?;
    }
    Ok(())
}

/// The task pane's Withdraw (Task-A01-baker800): the same change as `alm withdraw <approval ID>
/// --why "<reason>"`, made as the person's saved name. The reason is required here
/// (`withdraw_reason_missing`). Withdrawing stops nothing (approver, 2026-11-08).
#[tauri::command(async)]
pub(crate) fn withdraw_approval(root: PathBuf, id: String, reason: String) -> Result<(), Problem> {
    withdraw_as(
        &root,
        allomos_store::saved_owner_name().as_deref(),
        &id,
        &reason,
    )
}

fn withdraw_as(root: &Path, name: Option<&str>, id: &str, reason: &str) -> Result<(), Problem> {
    let actor = app_actor(name)?;
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(Problem::WithdrawReasonMissing);
    }
    allomos_store::approvals::withdraw(root, actor, LOCK_WAIT, id, Some(reason)).map(|_| ())
}

#[tauri::command(async)]
pub(crate) fn edit_task(
    root: PathBuf,
    id: String,
    edit: allomos_core::ops::Edit,
) -> Result<task_change::TaskChange, Problem> {
    change_task_as(
        &root,
        allomos_store::saved_owner_name().as_deref(),
        &id,
        &edit,
    )
}

/// A change sent by a paired device from the companion page (Task-A04-02-04, R77): the same
/// changes the desktop window makes, through the same functions, rules, lock and recorded events,
/// under `by` (the person's name and the device's, "will (iPhone)"). Only these operations; what
/// only the computer itself can do is not here (R68). Returns the changed item as JSON.
pub(crate) fn companion_change(
    root: &Path,
    by: &str,
    op: &str,
    args: serde_json::Value,
) -> Result<serde_json::Value, Problem> {
    fn arg<T: serde::de::DeserializeOwned>(
        args: &serde_json::Value,
        name: &str,
    ) -> Result<T, Problem> {
        serde_json::from_value(args.get(name).cloned().unwrap_or(serde_json::Value::Null)).map_err(
            |e| Problem::NotJson {
                detail: format!("{name}: {e}"),
            },
        )
    }
    let name = Some(by);
    let json = |v: serde_json::Result<serde_json::Value>| {
        v.map_err(|e| Problem::NotJson {
            detail: e.to_string(),
        })
    };
    match op {
        "edit_task" => {
            let edit: allomos_core::ops::Edit = arg(&args, "edit")?;
            let id: String = arg(&args, "id")?;
            json(serde_json::to_value(change_task_as(
                root, name, &id, &edit,
            )?))
        }
        "review_ai_decisions" => {
            review_as(root, name, &arg::<String>(&args, "id")?)?;
            Ok(serde_json::Value::Null)
        }
        "approve_selected" => {
            approve_selected_as(
                root,
                name,
                &arg::<Vec<String>>(&args, "check_ids").unwrap_or_default(),
                &arg::<Vec<String>>(&args, "review_ids").unwrap_or_default(),
            )?;
            Ok(serde_json::Value::Null)
        }
        "add_area" => json(serde_json::to_value(add_area_as(
            root,
            name,
            arg(&args, "id")?,
            &arg::<String>(&args, "name")?,
        )?)),
        "add_iteration" => json(serde_json::to_value(add_iteration_as(
            root,
            name,
            arg(&args, "iteration")?,
        )?)),
        "add_task" => json(serde_json::to_value(add_task_as(
            root,
            name,
            arg(&args, "task")?,
        )?)),
        "add_iteration_and_plan" => json(serde_json::to_value(add_iteration_and_plan_as(
            root,
            name,
            &arg::<String>(&args, "task")?,
            arg(&args, "iteration")?,
        )?)),
        "rename_area" => json(serde_json::to_value(rename_area_as(
            root,
            name,
            &arg::<String>(&args, "id")?,
            &arg::<String>(&args, "name")?,
        )?)),
        "change_iteration" => json(serde_json::to_value(change_iteration_as(
            root,
            name,
            &arg::<String>(&args, "id")?,
            &arg(&args, "change")?,
        )?)),
        "delete_area" => {
            delete_area_as(root, name, &arg::<String>(&args, "id")?)?;
            Ok(serde_json::Value::Null)
        }
        "delete_iteration" => {
            delete_iteration_as(root, name, &arg::<String>(&args, "id")?)?;
            Ok(serde_json::Value::Null)
        }
        _ => Err(Problem::CompanionDesktopOnly { op: op.to_string() }),
    }
}

/// The app's actor for a change: the person's saved name, via `app` (Task-A01-18). Refused
/// (`holder_missing`) until the name has been given.
pub(crate) fn app_actor(name: Option<&str>) -> Result<allomos_store::Actor<'_>, Problem> {
    let name = name
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .ok_or(Problem::HolderMissing)?;
    Ok(allomos_store::Actor {
        by: Some(name),
        via: "app",
    })
}

/// The IDs the New area, New iteration and New task forms suggest (Task-A01-19): each the next
/// free one, by the same rules `alm add` uses without an ID. The person may type another.
#[derive(Debug, Serialize, PartialEq)]
pub(crate) struct NextIds {
    area: String,
    iteration: String,
    /// For each area, the next free task ID in it.
    tasks: std::collections::BTreeMap<String, String>,
}

#[tauri::command(async)]
pub(crate) fn next_ids(root: PathBuf) -> Result<NextIds, Problem> {
    let (bag, _) = allomos_store::taskbag_of(&root, load(&root))?;
    let tasks = bag
        .areas
        .iter()
        .filter_map(|a| {
            allomos_core::ops::next_task_id(&bag, &a.id)
                .ok()
                .map(|id| (a.id.clone(), id))
        })
        .collect();
    Ok(NextIds {
        area: allomos_core::ops::next_area_id(&bag),
        iteration: allomos_core::ops::next_iteration_id(&bag),
        tasks,
    })
}

/// An empty ID box means "the next free one", as leaving the ID out of `alm add` does.
fn given(id: Option<String>) -> Option<String> {
    id.map(|i| i.trim().to_string()).filter(|i| !i.is_empty())
}

/// A new area from the list (Task-A01-19), under the person's saved name: the same rule, lock and
/// recorded event as `alm add area`.
fn add_area_as(
    root: &Path,
    name: Option<&str>,
    id: Option<String>,
    area_name: &str,
) -> Result<allomos_core::Area, Problem> {
    let id = given(id);
    allomos_store::add_area(root, app_actor(name)?, LOCK_WAIT, id.as_deref(), area_name)
        .map(|a| a.item)
}

#[tauri::command(async)]
pub(crate) fn add_area(
    root: PathBuf,
    id: Option<String>,
    name: String,
) -> Result<allomos_core::Area, Problem> {
    add_area_as(
        &root,
        allomos_store::saved_owner_name().as_deref(),
        id,
        &name,
    )
}

/// What the New iteration form sends.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NewIteration {
    #[serde(default)]
    id: Option<String>,
    name: String,
    #[serde(default)]
    release: Option<String>,
    #[serde(default)]
    start: Option<String>,
    #[serde(default)]
    end: Option<String>,
}

/// A new iteration from the list (Task-A01-19): the same rule, lock and recorded event as
/// `alm add iteration`.
pub(crate) fn add_iteration_as(
    root: &Path,
    name: Option<&str>,
    new: NewIteration,
) -> Result<allomos_core::Iteration, Problem> {
    let id = given(new.id);
    let details = allomos_core::ops::IterationDetails {
        release: new.release.as_deref(),
        start: new.start.as_deref(),
        end: new.end.as_deref(),
    };
    allomos_store::add_iteration(
        root,
        app_actor(name)?,
        LOCK_WAIT,
        id.as_deref(),
        &new.name,
        details,
    )
    .map(|i| i.item)
}

#[tauri::command(async)]
pub(crate) fn add_iteration(
    root: PathBuf,
    iteration: NewIteration,
) -> Result<allomos_core::Iteration, Problem> {
    add_iteration_as(
        &root,
        allomos_store::saved_owner_name().as_deref(),
        iteration,
    )
}

/// The task pane's "New…" iteration (Task-A01-39): the same rule, lock and recorded event as
/// `alm add iteration`, then task `task` planned in it as `alm plan` would, under the same lock
/// (two events). A refusal of either adds and plans nothing. Resolves to the planned task.
fn add_iteration_and_plan_as(
    root: &Path,
    name: Option<&str>,
    task: &str,
    new: NewIteration,
) -> Result<allomos_core::Task, Problem> {
    let id = given(new.id);
    let details = allomos_core::ops::IterationDetails {
        release: new.release.as_deref(),
        start: new.start.as_deref(),
        end: new.end.as_deref(),
    };
    allomos_store::add_iteration_and_plan(
        root,
        app_actor(name)?,
        LOCK_WAIT,
        task,
        id.as_deref(),
        &new.name,
        details,
    )
    .map(|(_, changed)| changed.task)
}

#[tauri::command(async)]
pub(crate) fn add_iteration_and_plan(
    root: PathBuf,
    task: String,
    iteration: NewIteration,
) -> Result<allomos_core::Task, Problem> {
    add_iteration_and_plan_as(
        &root,
        allomos_store::saved_owner_name().as_deref(),
        &task,
        iteration,
    )
}

/// A new task from an area or iteration group (Task-A01-19): the same rule, lock and recorded event
/// as `alm add task`; with an iteration, then planned under the same lock as `alm plan` (two
/// events).
fn add_task_as(
    root: &Path,
    name: Option<&str>,
    mut new: allomos_store::NewTask,
) -> Result<allomos_core::Task, Problem> {
    new.id = given(new.id);
    // The person is on no branch: the window sends only the base it chose (Task-A03-22).
    new.branch = None;
    allomos_store::add_task(root, app_actor(name)?, LOCK_WAIT, &new).map(|t| t.item)
}

#[tauri::command(async)]
pub(crate) fn add_task(
    root: PathBuf,
    task: allomos_store::NewTask,
) -> Result<allomos_core::Task, Problem> {
    add_task_as(&root, allomos_store::saved_owner_name().as_deref(), task)
}

/// An area renamed from its group in the list (Task-A01-29), under the person's saved name: the
/// same rule, lock and recorded event as `alm rename area`.
fn rename_area_as(
    root: &Path,
    name: Option<&str>,
    id: &str,
    new_name: &str,
) -> Result<allomos_core::Area, Problem> {
    allomos_store::rename_area(root, app_actor(name)?, LOCK_WAIT, id, new_name).map(|a| a.item)
}

#[tauri::command(async)]
pub(crate) fn rename_area(
    root: PathBuf,
    id: String,
    name: String,
) -> Result<allomos_core::Area, Problem> {
    rename_area_as(
        &root,
        allomos_store::saved_owner_name().as_deref(),
        &id,
        &name,
    )
}

/// An iteration changed from its group (Task-A01-29): the same rule, lock and recorded event as
/// `alm change iteration`. The form sends every field; an empty release or date clears it.
fn change_iteration_as(
    root: &Path,
    name: Option<&str>,
    id: &str,
    change: &allomos_core::ops::IterationChange,
) -> Result<allomos_core::Iteration, Problem> {
    allomos_store::change_iteration(root, app_actor(name)?, LOCK_WAIT, id, change).map(|i| i.item)
}

#[tauri::command(async)]
pub(crate) fn change_iteration(
    root: PathBuf,
    id: String,
    change: allomos_core::ops::IterationChange,
) -> Result<allomos_core::Iteration, Problem> {
    change_iteration_as(
        &root,
        allomos_store::saved_owner_name().as_deref(),
        &id,
        &change,
    )
}

/// An area deleted from its group (Task-A01-29): the same rule (refused while any task is in it,
/// done and dropped ones too), lock and recorded event as `alm delete area`.
fn delete_area_as(root: &Path, name: Option<&str>, id: &str) -> Result<(), Problem> {
    allomos_store::delete_area(root, app_actor(name)?, LOCK_WAIT, id).map(|_| ())
}

#[tauri::command(async)]
pub(crate) fn delete_area(root: PathBuf, id: String) -> Result<(), Problem> {
    delete_area_as(&root, allomos_store::saved_owner_name().as_deref(), &id)
}

/// An iteration deleted from its group (Task-A01-29): the same rule (refused while any task is
/// planned in it, done and dropped ones too), lock and recorded event as `alm delete iteration`.
fn delete_iteration_as(root: &Path, name: Option<&str>, id: &str) -> Result<(), Problem> {
    allomos_store::delete_iteration(root, app_actor(name)?, LOCK_WAIT, id).map(|_| ())
}

#[tauri::command(async)]
pub(crate) fn delete_iteration(root: PathBuf, id: String) -> Result<(), Problem> {
    delete_iteration_as(&root, allomos_store::saved_owner_name().as_deref(), &id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taskbag_commands::{open_taskbag, task_history, taskbag_stamp};
    use allomos_store::init;

    /// Task-A04-02-04: a companion page's change goes through the window's own functions, given
    /// "name (device)" for the holder rule; what only the computer can do is refused.
    #[test]
    fn a_companion_change_is_made_and_recorded_like_the_windows() {
        let dir =
            std::env::temp_dir().join(format!("allomos-desktop-companion-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join(allomos_store::DEFAULT_DIR);
        allomos_store::init(&root, "p", "P").unwrap();
        let by = "Will (Phone)";
        let area = companion_change(
            &root,
            by,
            "add_area",
            serde_json::json!({ "id": null, "name": "Core" }),
        )
        .unwrap();
        let task = companion_change(
            &root,
            by,
            "add_task",
            serde_json::json!({ "task": { "area": area["id"], "title": "From the phone" } }),
        )
        .unwrap();
        let id = task["id"].as_str().unwrap().to_string();
        companion_change(
            &root,
            by,
            "edit_task",
            serde_json::json!({ "id": id, "edit": { "field": "state", "state": "ready" } }),
        )
        .unwrap();
        let bag = allomos_store::load(&root).taskbag.unwrap();
        assert_eq!(bag.tasks[0].state, allomos_core::State::Ready);
        let history = allomos_store::events::history(&root, &id);
        // Recorded through the app, under the person's pseudonym and never their name
        // (decision 0021 § 2, Task-A01-frolic587).
        assert_eq!(history.events.len(), 2);
        assert!(history.events.iter().all(|e| e.via == "app"));
        assert!(!serde_json::to_string(&history).unwrap().contains("Will"));
        let err =
            companion_change(&root, by, "show_in_file_manager", serde_json::json!({})).unwrap_err();
        assert_eq!(err.code(), "companion_desktop_only");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Task-A01-nacho373: the commands that read or write the taskbag run off the main thread, so
    /// two can run at the same moment. Changes sent together each take the lock in turn: every
    /// one lands, with one event each, while another thread keeps reading the taskbag.
    #[test]
    fn changes_sent_at_the_same_moment_all_land() {
        let dir =
            std::env::temp_dir().join(format!("allomos-desktop-together-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join(allomos_store::DEFAULT_DIR);
        allomos_store::init(&root, "p", "P").unwrap();
        let by = "Will (Phone)";
        let area = companion_change(
            &root,
            by,
            "add_area",
            serde_json::json!({ "id": null, "name": "Core" }),
        )
        .unwrap();
        let ids: Vec<String> = (0..8)
            .map(|i| {
                let task = companion_change(
                    &root,
                    by,
                    "add_task",
                    serde_json::json!({ "task": { "area": area["id"], "title": format!("Task {i}") } }),
                )
                .unwrap();
                task["id"].as_str().unwrap().to_string()
            })
            .collect();
        std::thread::scope(|scope| {
            for id in &ids {
                let root = &root;
                scope.spawn(move || {
                    companion_change(
                        root,
                        by,
                        "edit_task",
                        serde_json::json!({ "id": id, "edit": { "field": "state", "state": "ready" } }),
                    )
                    .unwrap();
                });
            }
            // A reader at the same time, as the window's reload is.
            let root = &root;
            scope.spawn(move || {
                for _ in 0..8 {
                    assert!(open_taskbag(root.clone()).loaded.taskbag.is_some());
                }
            });
        });
        let bag = allomos_store::load(&root).taskbag.unwrap();
        assert_eq!(bag.tasks.len(), 8);
        assert!(
            bag.tasks
                .iter()
                .all(|t| t.state == allomos_core::State::Ready)
        );
        for id in &ids {
            assert_eq!(allomos_store::events::history(&root, id).events.len(), 2);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Task-A01-nacho373: how long the window's commands take on a real taskbag, which is how
    /// long the window stood still while they ran on the main thread. Not run with the others:
    /// `ALLOMOS_TIMING_TASKBAG=<a copy of a taskbag> ALLOMOS_TIMING_TASK=<a backlog task's id>
    /// cargo test -p allomos-desktop timing -- --ignored --nocapture`. It changes that task, so
    /// give it a copy.
    #[test]
    #[ignore]
    fn timing_of_the_windows_commands_on_a_taskbag() {
        let root = PathBuf::from(std::env::var_os("ALLOMOS_TIMING_TASKBAG").unwrap());
        let id = std::env::var("ALLOMOS_TIMING_TASK").unwrap();
        let timed = |what: &str, run: &mut dyn FnMut()| {
            let start = std::time::Instant::now();
            run();
            println!("{what}: {} ms", start.elapsed().as_millis());
        };
        timed("open_taskbag", &mut || {
            open_taskbag(root.clone());
        });
        timed("taskbag_stamp", &mut || {
            taskbag_stamp(root.clone());
        });
        timed("task_history", &mut || {
            task_history(root.clone(), id.clone());
        });
        let state = |state: &str| -> allomos_core::ops::Edit {
            serde_json::from_value(serde_json::json!({ "field": "state", "state": state })).unwrap()
        };
        // Since Task-A01-parade485: the change and its answer are all the window waits for.
        timed(
            "edit_task with its answer (state: ready; all it takes now)",
            &mut || {
                change_task_as(&root, Some("Will"), &id, &state("ready")).unwrap();
            },
        );
        timed("edit_task, the change alone (state: backlog)", &mut || {
            edit_task_as(&root, Some("Will"), &id, &state("backlog")).unwrap();
        });
        // Until Task-A01-parade485 the window then asked for the fingerprint and read the whole
        // taskbag again; since then the change's answer is all it needs.
        timed(
            "taskbag_stamp (before A01-parade485: after the change)",
            &mut || {
                taskbag_stamp(root.clone());
            },
        );
        timed(
            "open_taskbag again (before A01-parade485: the reload after the change)",
            &mut || {
                open_taskbag(root.clone());
            },
        );
    }

    /// The task pane's Approve and Withdraw (Task-A01-baker800): an approval of the description
    /// shown, channel app, changing no state; Approve and make ready also makes it ready; a plan
    /// changed since it was shown is refused; Withdraw needs a reason and records it.
    #[test]
    fn the_task_pane_approves_and_withdraws() {
        let dir = std::env::temp_dir().join(format!(
            "allomos-approve-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = dir.join(allomos_store::DEFAULT_DIR);
        init(&root, "p", "P").unwrap();
        let lock = allomos_store::Lock::acquire(&root, std::time::Duration::ZERO).unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.create_area(&allomos_core::ops::new_area(&bag, "A01", "First").unwrap())
            .unwrap();
        let bag = lock.load().taskbag.unwrap();
        let mut task = allomos_core::ops::new_task(&bag, "A01-01", "A01", "Do it", None).unwrap();
        task.description = "## Outcome\n\nThe plan.".into();
        lock.create_task(&task).unwrap();
        drop(lock);
        let state =
            |root: &Path| allomos_store::taskbag_of(root, load(root)).unwrap().0.tasks[0].state;
        assert_eq!(state(&root), allomos_core::State::Backlog);

        assert_eq!(
            approve_as(&root, None, "A01-01", &task.description, None, false),
            Err(Problem::HolderMissing)
        );
        // Shown an older plan: nothing recorded.
        assert!(matches!(
            approve_as(&root, Some("Will"), "A01-01", "## Outcome", None, false),
            Err(Problem::TaskChanged { .. })
        ));
        assert!(open_taskbag(root.clone()).approvals.is_empty());

        approve_as(
            &root,
            Some("Will"),
            "A01-01",
            &task.description,
            Some("2099-01-01"),
            false,
        )
        .unwrap();
        assert_eq!(state(&root), allomos_core::State::Backlog);
        let opened = open_taskbag(root.clone());
        let a = &opened.approvals["A01-01"][0];
        assert_eq!(a.approval.channel, "app");
        assert_eq!(a.approval.expires.as_deref(), Some("2099-01-01"));
        assert_eq!(a.status, allomos_core::approval::Status::Applies);
        let id = a.approval.id.clone();

        // Approve and make ready: a second approval, then ready.
        approve_as(&root, Some("Will"), "A01-01", &task.description, None, true).unwrap();
        assert_eq!(state(&root), allomos_core::State::Ready);
        assert_eq!(open_taskbag(root.clone()).approvals["A01-01"].len(), 2);

        assert_eq!(
            withdraw_as(&root, Some("Will"), &id, "  "),
            Err(Problem::WithdrawReasonMissing)
        );
        withdraw_as(&root, Some("Will"), &id, "Plan is wrong").unwrap();
        let opened = open_taskbag(root.clone());
        assert_eq!(
            opened.approvals["A01-01"][0].status,
            allomos_core::approval::Status::Withdrawn
        );
        assert_eq!(state(&root), allomos_core::State::Ready);
        let history = allomos_store::events::history(&root, "A01-01");
        let e = history
            .events
            .iter()
            .find(|e| e.command == "withdraw")
            .unwrap();
        assert_eq!(e.via, "app");
        assert_eq!(allomos_core::event::reason(e), Some("Plan is wrong"));
        assert_eq!(
            withdraw_as(&root, Some("Will"), &id, "again"),
            Err(Problem::ApprovalAlreadyWithdrawn { id })
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Decision 0026: the task pane's Blocked by field sets a task's blockers without changing
    /// its state, and finishing the last blocker there (Done, as the interface sends it) leaves
    /// the task as it is: it is just no longer blocked.
    #[test]
    fn blockers_set_in_the_app_keep_the_state_when_set_and_when_they_finish() {
        let dir = std::env::temp_dir().join(format!(
            "allomos-unblock-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = dir.join(allomos_store::DEFAULT_DIR);
        init(&root, "p", "P").unwrap();
        let lock = allomos_store::Lock::acquire(&root, std::time::Duration::ZERO).unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.create_area(&allomos_core::ops::new_area(&bag, "A01", "First").unwrap())
            .unwrap();
        for id in ["A01-01", "A01-02"] {
            let bag = lock.load().taskbag.unwrap();
            lock.create_task(&allomos_core::ops::new_task(&bag, id, "A01", id, None).unwrap())
                .unwrap();
        }
        drop(lock);
        let block: allomos_core::ops::Edit = serde_json::from_value(serde_json::json!(
            { "field": "blocked_by", "ids": ["A01-01"] }
        ))
        .unwrap();
        edit_task_as(&root, Some("Will"), "A01-02", &block).unwrap();
        let bag = load(&root).taskbag.unwrap();
        let t = bag.tasks.iter().find(|t| t.id == "A01-02").unwrap();
        assert_eq!(t.state, allomos_core::State::Backlog);
        assert!(allomos_core::ops::is_blocked(&bag, t));
        let done: allomos_core::ops::Edit =
            serde_json::from_value(serde_json::json!({ "field": "state", "state": "done" }))
                .unwrap();
        edit_task_as(&root, Some("Will"), "A01-01", &done).unwrap();
        let bag = load(&root).taskbag.unwrap();
        let t = bag.tasks.iter().find(|t| t.id == "A01-02").unwrap();
        assert_eq!(t.state, allomos_core::State::Backlog);
        assert_eq!(t.blocked_by, ["A01-01"]);
        assert!(!allomos_core::ops::is_blocked(&bag, t));
        let e = &allomos_store::events::history(&root, "A01-02").events[0];
        assert_eq!((e.via.as_str(), e.command.as_str()), ("app", "block"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The task pane's change command (Task-A01-18): the edit arrives as the interface sends it,
    /// is refused without the person's name or by the holder rule, and is otherwise recorded
    /// under that name, through the app.
    #[test]
    fn the_task_pane_changes_a_task_under_the_persons_name() {
        let dir = std::env::temp_dir().join(format!(
            "allomos-edit-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = dir.join(allomos_store::DEFAULT_DIR);
        init(&root, "p", "P").unwrap();
        let lock = allomos_store::Lock::acquire(&root, std::time::Duration::ZERO).unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.create_area(&allomos_core::ops::new_area(&bag, "A01", "First").unwrap())
            .unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.create_task(
            &allomos_core::ops::new_task(&bag, "A01-01", "A01", "Do it", None).unwrap(),
        )
        .unwrap();
        drop(lock);

        // As the interface sends it: `{ field: "label", label: "implemented" }`.
        let edit: allomos_core::ops::Edit =
            serde_json::from_value(serde_json::json!({ "field": "label", "label": "implemented" }))
                .unwrap();
        assert_eq!(
            edit_task_as(&root, None, "A01-01", &edit),
            Err(Problem::HolderMissing)
        );
        assert_eq!(
            edit_task_as(&root, Some(" "), "A01-01", &edit),
            Err(Problem::HolderMissing)
        );
        assert!(load(&root).taskbag.unwrap().tasks[0].labels.is_empty());

        let task = edit_task_as(&root, Some("Will"), "A01-01", &edit).unwrap();
        assert_eq!(task.labels, ["implemented"]);
        let h = allomos_store::events::history(&root, "A01-01");
        let e = &h.events[0];
        // Under the person's pseudonym, never their name (decision 0021 § 2, Task-A01-frolic587);
        // these tests have no settings folder for one, so under nobody.
        assert_eq!(
            (e.by.as_deref(), e.via.as_str(), e.command.as_str()),
            (None, "app", "label")
        );

        // A task an agent holds is refused with the same problem `alm` gives.
        let lock = allomos_store::Lock::acquire(&root, std::time::Duration::ZERO).unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.save_task(
            &allomos_core::ops::claim(&bag, "A01-01", Some("claude/x"), false)
                .unwrap()
                .task,
        )
        .unwrap();
        drop(lock);
        assert_eq!(
            edit_task_as(&root, Some("Will"), "A01-01", &edit),
            Err(Problem::TaskHeld {
                id: "A01-01".into(),
                holder: "claude/x".into()
            })
        );

        // Task-A03-19: the agent's decision shows when the taskbag opens; the Reviewed button
        // needs the person's name, even on a task an agent holds, and takes it away.
        assert_eq!(
            review_as(&root, Some("Will"), "A01-01"),
            Err(Problem::NothingToReview {
                id: "A01-01".into()
            })
        );
        allomos_store::record_ai_decision(
            &root,
            allomos_store::Actor::alm(Some("claude/x")),
            LOCK_WAIT,
            "A01-01",
            "Names: alm note",
            None,
        )
        .unwrap();
        let opened = open_taskbag(root.clone());
        assert_eq!(opened.ai_decisions["A01-01"][0].text, "Names: alm note");
        // Task-A01-garden385: the risk an agent recorded comes along when the taskbag opens; a
        // task that is not classified is left out (the window shows it as "Not classified").
        assert!(opened.classifications.is_empty());
        allomos_store::classify::set(
            &root,
            allomos_store::Actor::alm(Some("claude/x")),
            LOCK_WAIT,
            "A01-01",
            &allomos_core::classify::Given {
                risk: Some("high"),
                ..Default::default()
            },
        )
        .unwrap();
        let opened = open_taskbag(root.clone());
        assert_eq!(
            opened.classifications["A01-01"].risk.as_deref(),
            Some("high")
        );
        assert_eq!(
            serde_json::to_value(&opened).unwrap()["classifications"]["A01-01"]["risk"],
            "high"
        );
        assert_eq!(
            review_as(&root, None, "A01-01"),
            Err(Problem::HolderMissing)
        );
        review_as(&root, Some("Will"), "A01-01").unwrap();
        assert!(open_taskbag(root.clone()).ai_decisions.is_empty());
        let e = &allomos_store::events::history(&root, "A01-01").events[0];
        assert_eq!(
            (e.by.as_deref(), e.via.as_str(), e.command.as_str()),
            (None, "app", "reviewed")
        );

        // Task-A01-50: Reopen… on a finished task, as the interface sends it; the reason goes in
        // the history under the person's name. An open task is refused as `alm reopen` refuses.
        let reopen: allomos_core::ops::Edit = serde_json::from_value(
            serde_json::json!({ "field": "reopen", "note": "The check failed" }),
        )
        .unwrap();
        assert_eq!(
            edit_task_as(&root, Some("Will"), "A01-01", &reopen)
                .unwrap_err()
                .code(),
            "cannot_reopen"
        );
        allomos_store::change_task(
            &root,
            allomos_store::Actor::alm(Some("claude/x")),
            LOCK_WAIT,
            |bag| {
                allomos_core::ops::finish(bag, "A01-01", Some("claude/x"), None, None)
                    .map(|o| (o, "done"))
            },
        )
        .unwrap();
        let task = edit_task_as(&root, Some("Will"), "A01-01", &reopen).unwrap();
        assert_eq!(
            (task.state, task.holder, task.labels),
            (allomos_core::State::Ready, None, vec![])
        );
        let e = &allomos_store::events::history(&root, "A01-01").events[0];
        assert_eq!(
            (e.by.as_deref(), e.via.as_str(), e.command.as_str()),
            (None, "app", "reopen")
        );
        assert!(e.changes.iter().any(
            |c| c.field == "reopen_note" && c.to == Some(serde_json::json!("The check failed"))
        ));
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// The New area, New iteration and New task forms (Task-A01-19), with the arguments as the
    /// interface sends them: suggested IDs, the same refusals as `alm add`, events under the name.
    #[test]
    fn the_list_adds_areas_iterations_and_tasks_under_the_persons_name() {
        let dir = std::env::temp_dir().join(format!(
            "allomos-add-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = dir.join(allomos_store::DEFAULT_DIR);
        init(&root, "p", "P").unwrap();
        let ids = next_ids(root.clone()).unwrap();
        assert_eq!(ids.area, "A01");
        // An unused `I-<labor word>` (Task-A00-rope803).
        assert!(ids.iteration.starts_with("I-"), "{}", ids.iteration);
        assert!(ids.tasks.is_empty());

        // Without the person's name, nothing is added.
        assert_eq!(
            add_area_as(&root, None, None, "Tracking"),
            Err(Problem::HolderMissing)
        );
        // New area: an empty ID box takes the suggested one.
        let area = add_area_as(&root, Some("Will"), Some(" ".into()), "Tracking").unwrap();
        assert_eq!(area.id, "A01");
        assert_eq!(
            add_area_as(&root, Some("Will"), Some("A01".into()), "Again"),
            Err(Problem::DuplicateAreaId { id: "A01".into() })
        );

        // New iteration, as `messenger.addIteration` sends it.
        let it: NewIteration = serde_json::from_value(serde_json::json!({
            "id": "I07", "name": "Seven", "release": "v0.7", "start": "2026-10-15", "end": null
        }))
        .unwrap();
        assert_eq!(add_iteration_as(&root, Some("Will"), it).unwrap().id, "I07");
        let bad: NewIteration = serde_json::from_value(
            serde_json::json!({ "id": null, "name": "Eight", "end": "31/10/2026" }),
        )
        .unwrap();
        assert_eq!(
            add_iteration_as(&root, Some("Will"), bad),
            Err(Problem::InvalidDate {
                value: "31/10/2026".into()
            })
        );
        let ids = next_ids(root.clone()).unwrap();
        assert!(ids.iteration.starts_with("I-"), "{}", ids.iteration);
        // A word ID in the area (Task-A01-54).
        let suggested = ids.tasks.get("A01").unwrap();
        assert!(
            suggested.starts_with("A01-") && !allomos_core::ids::numbered(suggested),
            "{suggested}"
        );

        // New task in an iteration group, as `messenger.addTask` sends it: added, then planned.
        let task: allomos_store::NewTask = serde_json::from_value(serde_json::json!({
            "id": "A01-01", "area": "A01", "title": "Plan the week", "description": "Why.", "iteration": "I07"
        }))
        .unwrap();
        let t = add_task_as(&root, Some("Will"), task).unwrap();
        assert_eq!(
            (t.id.as_str(), t.iteration.as_deref(), t.state),
            ("A01-01", Some("I07"), allomos_core::State::Backlog)
        );
        // Task-A03-22: with no base sent, none is recorded.
        assert!(open_taskbag(root.clone()).branches.is_empty());
        let h = allomos_store::events::history(&root, "A01-01");
        let seen: Vec<_> = h
            .events
            .iter()
            .map(|e| (e.command.as_str(), e.by.as_deref(), e.via.as_str()))
            .collect();
        // Task-A01-52: the task's last change is its newest event.
        assert_eq!(
            open_taskbag(root.clone()).last_changed.get("A01-01"),
            Some(&h.events[0].at)
        );
        // Creation time is the oldest event for the task.
        assert_eq!(
            open_taskbag(root.clone()).created.get("A01-01"),
            Some(&h.events.last().unwrap().at)
        );
        // Newest first.
        assert_eq!(seen, [("plan", None, "app"), ("add task", None, "app")]);
        // The task pane's "New…" iteration, as `messenger.addIterationAndPlan` sends it: added
        // with the next free ID, then the task planned in it (two events, newest first).
        assert_eq!(
            add_iteration_and_plan_as(&root, None, "A01-01", NewIteration::default()),
            Err(Problem::HolderMissing)
        );
        let pane: NewIteration = serde_json::from_value(serde_json::json!({
            "id": null, "name": "Next", "release": null, "start": "2026-11-01", "end": null
        }))
        .unwrap();
        let t = add_iteration_and_plan_as(&root, Some("Will"), "A01-01", pane).unwrap();
        assert!(t.iteration.as_deref().is_some_and(|i| i.starts_with("I-")));
        let h = allomos_store::events::history(&root, "A01-01");
        assert_eq!(
            (h.events[0].command.as_str(), h.events[0].via.as_str()),
            ("plan", "app")
        );
        let nameless: NewIteration =
            serde_json::from_value(serde_json::json!({ "name": " " })).unwrap();
        assert_eq!(
            add_iteration_and_plan_as(&root, Some("Will"), "A01-01", nameless),
            Err(Problem::IterationNameMissing)
        );
        assert_eq!(load(&root).taskbag.unwrap().iterations.len(), 2);
        let unknown: allomos_store::NewTask =
            serde_json::from_value(serde_json::json!({ "area": "A09", "title": "x" })).unwrap();
        assert_eq!(
            add_task_as(&root, Some("Will"), unknown),
            Err(Problem::UnknownArea { area: "A09".into() })
        );
        assert_eq!(load(&root).taskbag.unwrap().tasks.len(), 1);

        // Task-A03-22: the base the person picked is recorded in the `add task` event and shows
        // when the taskbag opens; a `branch` sent by mistake chooses nothing.
        let picked: allomos_store::NewTask = serde_json::from_value(serde_json::json!({
            "area": "A01", "title": "On a branch", "base": "storage-v2", "branch": "claude/x"
        }))
        .unwrap();
        let t = add_task_as(&root, Some("Will"), picked).unwrap();
        let opened = open_taskbag(root.clone());
        assert_eq!(opened.branches[&t.id].base.as_deref(), Some("storage-v2"));
        assert_eq!(opened.branches[&t.id].working, None);
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// Task-A01-29: renaming an area, changing an iteration and deleting either, with the
    /// arguments as `messenger.renameArea`, `changeIteration`, `deleteArea` and `deleteIteration`
    /// send them; each under the person's name, each one event; a non-empty one is not deleted.
    #[test]
    fn the_list_renames_changes_and_deletes_under_the_persons_name() {
        let dir = std::env::temp_dir().join(format!(
            "allomos-rename-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = dir.join(allomos_store::DEFAULT_DIR);
        init(&root, "p", "P").unwrap();
        let will = Some("Will");
        add_area_as(&root, will, Some("A01".into()), "Tracking").unwrap();
        add_area_as(&root, will, Some("A02".into()), "Spare").unwrap();
        let it = |id: &str| -> NewIteration {
            serde_json::from_value(serde_json::json!({ "id": id, "name": id, "end": "2026-10-10" }))
                .unwrap()
        };
        add_iteration_as(&root, will, it("I01")).unwrap();
        add_iteration_as(&root, will, it("I02")).unwrap();
        let task: allomos_store::NewTask = serde_json::from_value(
            serde_json::json!({ "area": "A01", "title": "Do it", "iteration": "I01" }),
        )
        .unwrap();
        add_task_as(&root, will, task).unwrap();
        let count = || allomos_store::events::list(&root).0.len();
        let last = || {
            let e = allomos_store::events::list(&root).0.pop().unwrap();
            (e.command, e.id.unwrap_or_default(), e.by, e.via)
        };
        let by = |c: &str, id: &str| {
            (
                c.to_string(),
                id.to_string(),
                // The pseudonym, never the name (Task-A01-frolic587); no settings folder here.
                None,
                "app".to_string(),
            )
        };

        // Without the person's name, nothing changes.
        assert_eq!(
            rename_area_as(&root, None, "A01", "x"),
            Err(Problem::HolderMissing)
        );
        let start = count();
        assert_eq!(
            rename_area_as(&root, will, "A01", "Tracking work")
                .unwrap()
                .name,
            "Tracking work"
        );
        assert_eq!((count(), last()), (start + 1, by("rename area", "A01")));

        // The iteration form sends every field; an empty release clears it.
        let change: allomos_core::ops::IterationChange = serde_json::from_value(
            serde_json::json!({ "name": "I01", "release": "", "start": "", "end": "2026-10-20" }),
        )
        .unwrap();
        let changed = change_iteration_as(&root, will, "I01", &change).unwrap();
        assert_eq!(changed.end.as_deref(), Some("2026-10-20"));
        assert_eq!(
            (count(), last()),
            (start + 2, by("change iteration", "I01"))
        );
        let bad: allomos_core::ops::IterationChange =
            serde_json::from_value(serde_json::json!({ "end": "20/10/2026" })).unwrap();
        assert_eq!(
            change_iteration_as(&root, will, "I01", &bad),
            Err(Problem::InvalidDate {
                value: "20/10/2026".into()
            })
        );

        // Not empty: refused, nothing deleted.
        assert_eq!(
            delete_area_as(&root, will, "A01"),
            Err(Problem::AreaNotEmpty {
                id: "A01".into(),
                count: "1".into()
            })
        );
        assert_eq!(
            delete_iteration_as(&root, will, "I01"),
            Err(Problem::IterationNotEmpty {
                id: "I01".into(),
                count: "1".into()
            })
        );
        assert_eq!(count(), start + 2);
        delete_iteration_as(&root, will, "I02").unwrap();
        assert_eq!(
            (count(), last()),
            (start + 3, by("delete iteration", "I02"))
        );
        delete_area_as(&root, will, "A02").unwrap();
        assert_eq!((count(), last()), (start + 4, by("delete area", "A02")));
        let bag = load(&root).taskbag.unwrap();
        assert_eq!(
            (
                bag.areas.len(),
                bag.iterations.len(),
                bag.areas[0].name.as_str()
            ),
            (1, 1, "Tracking work")
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn approve_selected_checks_and_reviews_multiple_tasks() {
        let dir = std::env::temp_dir()
            .join(format!("allomos-desktop-approve-selected-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join(allomos_store::DEFAULT_DIR);
        allomos_store::init(&root, "p", "P").unwrap();
        let will = Some("Will");
        let area = add_area_as(&root, will, None, "Core").unwrap();
        let t1 = add_task_as(
            &root,
            will,
            allomos_store::NewTask {
                id: None,
                area: area.id.clone(),
                iteration: None,
                title: "Task 1".into(),
                description: None,
                difficulty: None,
                base: None,
                branch: None,
                folders: Vec::new(),
            },
        )
        .unwrap();
        let t2 = add_task_as(
            &root,
            will,
            allomos_store::NewTask {
                id: None,
                area: area.id,
                iteration: None,
                title: "Task 2".into(),
                description: None,
                difficulty: None,
                base: None,
                branch: None,
                folders: Vec::new(),
            },
        )
        .unwrap();

        // Mark t1 done so it can be checked
        edit_task_as(
            &root,
            will,
            &t1.id,
            &allomos_core::ops::Edit::State {
                state: allomos_core::State::Done,
                waiting_on: None,
                who: None,
            },
        )
        .unwrap();

        // Record an AI decision on t2
        allomos_store::record_ai_decision(
            &root,
            allomos_store::Actor::alm(Some("agent")),
            LOCK_WAIT,
            &t2.id,
            "Used Rust",
            None,
        )
        .unwrap();

        approve_selected_as(&root, will, &[t1.id.clone()], &[t2.id.clone()]).unwrap();

        let bag = load(&root).taskbag.unwrap();
        let checked_t1 = bag.tasks.iter().find(|t| t.id == t1.id).unwrap();
        assert!(checked_t1.has_label(allomos_core::labels::LOCALLY_BY_HUMAN));

        let reviewed_t2 = bag.tasks.iter().find(|t| t.id == t2.id).unwrap();
        assert!(!reviewed_t2.has_label(allomos_core::labels::AI_DECISIONS));

        std::fs::remove_dir_all(dir).unwrap();
    }
}
