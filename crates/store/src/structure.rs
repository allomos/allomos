//! Areas, iterations and labels: adding, renaming, changing and deleting them.

use super::*;

/// Something added by [`add_area`], [`add_iteration`] or [`add_task`] (or renamed, changed or
/// deleted by [`rename_area`], [`change_iteration`], [`delete_area`], [`delete_iteration`]), with
/// the problems found in the taskbag's files while loading it.
#[derive(Debug, Clone, PartialEq)]
pub struct Added<T> {
    pub item: T,
    pub issues: Vec<Issue>,
}

/// A new area, the same way for `alm add area` and the app (Task-A01-19): under the lock, by the
/// `crates/core` rule, recorded as one `add area` event. Without an ID it takes the next free one.
pub fn add_area(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: Option<&str>,
    name: &str,
) -> Result<Added<Area>, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let id = id.map_or_else(|| ops::next_area_id(&bag), String::from);
    let area = ops::new_area(&bag, &id, name)?;
    let event = actor.event(
        "add area",
        Kind::Area,
        Some(&area.id),
        event::changes(None, &area, &[]),
    );
    lock.record(&event, |l| l.create_area(&area))?;
    Ok(Added { item: area, issues })
}

/// A new iteration, the same way for `alm add iteration` and the app (Task-A01-19): under the lock,
/// by the `crates/core` rule, recorded as one `add iteration` event. Without an ID it takes the
/// next free one.
pub fn add_iteration(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: Option<&str>,
    name: &str,
    details: ops::IterationDetails,
) -> Result<Added<Iteration>, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let id = id.map_or_else(|| ops::next_iteration_id(&bag), String::from);
    let iteration = ops::new_iteration(&bag, &id, name, details)?;
    let event = actor.event(
        "add iteration",
        Kind::Iteration,
        Some(&iteration.id),
        event::changes(None, &iteration, &[]),
    );
    lock.record(&event, |l| l.create_iteration(&iteration))?;
    Ok(Added {
        item: iteration,
        issues,
    })
}

/// The task pane's "New…" iteration (Task-A01-39): a new iteration, by the rule `alm add
/// iteration` uses (without an ID, the next free one), then task `task` planned in it by the rule `alm plan` uses,
/// under one lock, recorded as two events (`add iteration`, `plan`): the same as typing both
/// commands. Both rules are checked before anything is written, so a refusal of either (an empty
/// name, a bad date, an unknown task) writes nothing.
pub fn add_iteration_and_plan(
    root: &Path,
    actor: Actor,
    wait: Duration,
    task: &str,
    id: Option<&str>,
    name: &str,
    details: ops::IterationDetails,
) -> Result<(Iteration, Changed), Problem> {
    let (lock, mut bag, issues) = lock_for_change(root, wait)?;
    let id = id.map_or_else(|| ops::next_iteration_id(&bag), String::from);
    let iteration = ops::new_iteration(&bag, &id, name, details)?;
    bag.iterations.push(iteration.clone());
    let planned = ops::plan(&bag, task, Some(&iteration.id))?;
    let before = bag.tasks.iter().find(|t| t.id == planned.task.id).cloned();
    let added = actor.event(
        "add iteration",
        Kind::Iteration,
        Some(&iteration.id),
        event::changes(None, &iteration, &[]),
    );
    lock.record(&added, |l| l.create_iteration(&iteration))?;
    let plan = actor.event(
        "plan",
        Kind::Task,
        Some(&planned.task.id),
        event::changes(before.as_ref(), &planned.task, event::TASK_FIELDS),
    );
    lock.record(&plan, |l| l.save_task(&planned.task))?;
    Ok((
        iteration,
        Changed {
            task: planned.task,
            warnings: planned.warnings,
            issues,
            unblocked: Vec::new(),
        },
    ))
}

/// An area renamed, the same way for `alm rename area` and the app (Task-A01-29): under the lock,
/// by the `crates/core` rule, recorded as one `rename area` event. Its ID stays.
pub fn rename_area(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    name: &str,
) -> Result<Added<Area>, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let area = ops::rename_area(&bag, id, name)?;
    let before = bag.areas.iter().find(|a| a.id == area.id);
    let event = actor.event(
        "rename area",
        Kind::Area,
        Some(&area.id),
        event::changes(before, &area, &[]),
    );
    lock.record(&event, |l| l.save_area(&area))?;
    Ok(Added { item: area, issues })
}

/// What [`rename_iteration`] changed: the iteration under its new ID, and the tasks moved along.
pub type RenamedIteration = (Iteration, Vec<Task>);

/// An iteration given a new ID (`alm rename iteration`, Task-A00-rope803): under the lock, by the
/// `crates/core` rule, its file written under the new ID and the old one removed, recorded as one
/// `rename iteration` event for it (its `id` from the old to the new) and one for each task
/// planned in it. Renaming to the same ID writes nothing.
pub fn rename_iteration(
    root: &Path,
    actor: Actor,
    wait: Duration,
    old: &str,
    new: &str,
) -> Result<Added<RenamedIteration>, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let (iteration, tasks) = ops::rename_iteration(&bag, old, new)?;
    if iteration.id != old {
        let before = bag.iterations.iter().find(|i| i.id == old);
        let event = actor.event(
            "rename iteration",
            Kind::Iteration,
            Some(&iteration.id),
            event::changes(before, &iteration, &[]),
        );
        lock.record(&event, |l| {
            l.save_iteration(&iteration)?;
            l.remove_item("iterations", old)
        })?;
        for task in &tasks {
            let before = bag.tasks.iter().find(|t| t.id == task.id);
            let event = actor.event(
                "rename iteration",
                Kind::Task,
                Some(&task.id),
                event::changes(before, task, event::TASK_FIELDS),
            );
            lock.record(&event, |l| l.save_task(task))?;
        }
    }
    Ok(Added {
        item: (iteration, tasks),
        issues,
    })
}

/// What [`rename_label`] changed: the new ladder, and the tasks whose label changed.
pub type RenamedLabel = (Vec<String>, Vec<Task>);

/// A label renamed on the ladder together with every task that carries it (`alm rename label`,
/// Task-A01-32): under the lock, by the `crates/core` rule, recorded as one `rename label` event
/// for the project and one for each task changed. Returns the new ladder and the tasks changed.
pub fn rename_label(
    root: &Path,
    actor: Actor,
    wait: Duration,
    old: &str,
    new: &str,
) -> Result<Added<RenamedLabel>, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let (project, tasks) = ops::rename_label(&bag, old, new)?;
    let event = actor.event(
        "rename label",
        Kind::Project,
        Some(&project.id),
        event::changes(Some(&bag.project), &project, &[]),
    );
    lock.record(&event, |l| l.save_project(&project))?;
    for task in &tasks {
        let before = bag.tasks.iter().find(|t| t.id == task.id);
        let event = actor.event(
            "rename label",
            Kind::Task,
            Some(&task.id),
            event::changes(before, task, event::TASK_FIELDS),
        );
        lock.record(&event, |l| l.save_task(task))?;
    }
    Ok(Added {
        item: (project.label_names(), tasks),
        issues,
    })
}

/// An iteration changed (name, release, dates), the same way for `alm change iteration` and the
/// app (Task-A01-29): under the lock, by the `crates/core` rule, recorded as one
/// `change iteration` event. A change that changes nothing writes no event.
pub fn change_iteration(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    change: &ops::IterationChange,
) -> Result<Added<Iteration>, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let iteration = ops::change_iteration(&bag, id, change)?;
    let before = bag.iterations.iter().find(|i| i.id == iteration.id);
    let event = actor.event(
        "change iteration",
        Kind::Iteration,
        Some(&iteration.id),
        event::changes(before, &iteration, &[]),
    );
    lock.record(&event, |l| l.save_iteration(&iteration))?;
    Ok(Added {
        item: iteration,
        issues,
    })
}

/// An iteration moved before or after another one (`alm move iteration`, Task-A00-skate868): under
/// the lock, by the `crates/core` rule, recorded as one `move iteration` event for each iteration
/// whose order changed. Returns the iterations changed; a move that changes nothing writes none.
pub fn move_iteration(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    place: ops::Place<'_>,
) -> Result<Added<Vec<Iteration>>, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let moved = ops::move_iteration(&bag, id, place)?;
    for iteration in &moved {
        let before = bag.iterations.iter().find(|i| i.id == iteration.id);
        let event = actor.event(
            "move iteration",
            Kind::Iteration,
            Some(&iteration.id),
            event::changes(before, iteration, &[]),
        );
        lock.record(&event, |l| l.save_iteration(iteration))?;
    }
    Ok(Added {
        item: moved,
        issues,
    })
}

/// An area deleted, the same way for `alm delete area` and the app (Task-A01-29): refused while
/// any task is in it (done and dropped ones too); otherwise its file is removed under the lock,
/// recorded as one `delete area` event listing what it had. Returns the area as it was.
pub fn delete_area(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
) -> Result<Added<Area>, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let area = ops::delete_area(&bag, id)?;
    let event = actor.event(
        "delete area",
        Kind::Area,
        Some(&area.id),
        event::removed(&area, &[]),
    );
    lock.record(&event, |l| l.remove_item("areas", &area.id))?;
    Ok(Added { item: area, issues })
}

/// An iteration deleted, the same way for `alm delete iteration` and the app (Task-A01-29):
/// refused while any task is planned in it (done and dropped ones too); otherwise its file is
/// removed under the lock, recorded as one `delete iteration` event. Returns it as it was.
pub fn delete_iteration(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
) -> Result<Added<Iteration>, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let iteration = ops::delete_iteration(&bag, id)?;
    let event = actor.event(
        "delete iteration",
        Kind::Iteration,
        Some(&iteration.id),
        event::removed(&iteration, &[]),
    );
    lock.record(&event, |l| l.remove_item("iterations", &iteration.id))?;
    Ok(Added {
        item: iteration,
        issues,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp(name: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!("allomos-test-{name}-{n}"));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn iterations_are_written_read_and_sorted() {
        use allomos_core::ops::{self, IterationDetails};
        let dir = tmp("iterations");
        let root = dir.join(DEFAULT_DIR);
        init(&root, "p", "P").unwrap();
        // A taskbag made before iterations existed has no `iterations/` folder: still valid.
        assert!(!root.join("iterations").exists());
        let loaded = load(&root);
        assert!(loaded.issues.is_empty() && loaded.taskbag.unwrap().iterations.is_empty());

        let before = stamp(&root);
        let lock = Lock::acquire(&root, Duration::ZERO).unwrap();
        let bag = lock.load().taskbag.unwrap();
        let two = ops::new_iteration(
            &bag,
            "I02",
            "v0.2",
            IterationDetails {
                release: Some("v0.2"),
                ..Default::default()
            },
        )
        .unwrap();
        lock.create_iteration(&two).unwrap();
        assert!(matches!(
            lock.create_iteration(&two),
            Err(Problem::DuplicateIterationId { .. })
        ));
        // Written by hand with a lower order: listed first.
        fs::write(
            root.join("iterations/I01.json"),
            r#"{"id":"I01","name":"v0.1","order":0}"#,
        )
        .unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.create_area(&ops::new_area(&bag, "A01", "First").unwrap())
            .unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.create_task(&ops::new_task(&bag, "A01-01", "A01", "Do it", None).unwrap())
            .unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.save_task(&ops::plan(&bag, "A01-01", Some("I02")).unwrap().task)
            .unwrap();
        drop(lock);

        let json = fs::read_to_string(root.join("iterations/I02.json")).unwrap();
        assert!(!json.contains("null") && !json.contains("start"), "{json}");
        let loaded = load(&root);
        assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
        let bag = loaded.taskbag.unwrap();
        let ids: Vec<_> = bag.iterations.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(ids, ["I01", "I02"]);
        assert_eq!(bag.tasks[0].iteration.as_deref(), Some("I02"));
        assert_ne!(stamp(&root), before);

        // A task planned in a missing iteration is reported, and the task still shows.
        fs::remove_file(root.join("iterations/I02.json")).unwrap();
        let loaded = load(&root);
        assert_eq!(loaded.issues[0].problem.code(), "unknown_iteration");
        assert_eq!(loaded.taskbag.unwrap().tasks.len(), 1);
        fs::remove_dir_all(dir).unwrap();
    }
}
