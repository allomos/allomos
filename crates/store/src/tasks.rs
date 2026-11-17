//! Adding a task and changing one, under the lock.

use super::*;

/// A task as changed by [`change_task`], with what is worth telling the person.
#[derive(Debug, Clone, PartialEq)]
pub struct Changed {
    pub task: Task,
    /// Things worth telling that did not refuse the change (for example `took_over`).
    pub warnings: Vec<Problem>,
    /// Problems found in the taskbag's files while loading it.
    pub issues: Vec<Issue>,
    /// The tasks this change freed: it finished their last open blocker, so they are no longer
    /// blocked. They are not changed: each keeps its state (decision 0026).
    pub unblocked: Vec<Task>,
}

/// What a new task is made of (`alm add task`, the app's New task form).
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewTask {
    /// Without one, the next free ID in the area.
    #[serde(default)]
    pub id: Option<String>,
    pub area: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    /// The app only: plan it there too, as `alm plan` would (owner, 2026-10-14).
    #[serde(default)]
    pub iteration: Option<String>,
    /// The branch the agent is on (`alm add task --branch`), which chooses the base (Task-A03-22).
    #[serde(default)]
    pub branch: Option<String>,
    /// The base given (`--base`, or the person's pick in the app): it wins over `branch`.
    #[serde(default)]
    pub base: Option<String>,
    /// The project folders it belongs to (`alm add task --folder`, decision 0019); empty: none in
    /// particular. Each must be one of the taskbag's folders.
    #[serde(default)]
    pub folders: Vec<String>,
    /// How hard it is, 1 to 5 (`alm add task --difficulty`, Task-A01-actor813); `None`: not given,
    /// it counts as 3.
    #[serde(default)]
    pub difficulty: Option<u8>,
}

/// What [`add_task_with_base`] did: the task, the base it got and why, and warnings.
#[derive(Debug, Clone, PartialEq)]
pub struct AddedTask {
    pub task: Task,
    /// The base the task got, and why; `None` when neither a branch nor a base was given.
    pub base: Option<branch::Choice>,
    /// `branch_missing`, or `base_differs` for each task the adding session holds on another base.
    pub warnings: Vec<Problem>,
    pub issues: Vec<Issue>,
}

/// A new task in backlog, the same way for `alm add task` and the app (Task-A01-19): under the
/// lock, by the `crates/core` rule, recorded as one `add task` event. With an iteration it is then
/// planned under the same lock, by the rule `alm plan` uses, as a second `plan` event: the same as
/// typing both commands (owner, 2026-10-14). Both rules are checked before anything is written, so
/// a refusal of either writes nothing.
pub fn add_task(
    root: &Path,
    actor: Actor,
    wait: Duration,
    new: &NewTask,
) -> Result<Added<Task>, Problem> {
    add_task_with_base(root, actor, wait, new).map(|a| Added {
        item: a.task,
        issues: a.issues,
    })
}

/// [`add_task`], telling also which base the task got and why (Task-A03-22). The base is chosen
/// by [`branch::choose_base`] under the same lock and recorded in the `add task` event as a
/// `base_branch` change; `task.json` does not change shape.
pub fn add_task_with_base(
    root: &Path,
    actor: Actor,
    wait: Duration,
    new: &NewTask,
) -> Result<AddedTask, Problem> {
    let (lock, mut bag, issues) = lock_for_change(root, wait)?;
    let id = match &new.id {
        Some(id) => id.clone(),
        None => ops::next_task_id(&bag, &new.area)?,
    };
    let mut task = ops::new_task(&bag, &id, &new.area, &new.title, new.description.as_deref())?;
    task.folders = allomos_core::folders::check(&bag.project, &new.folders)?;
    if let Some(n) = new.difficulty {
        task.difficulty = Some(i64::from(allomos_core::difficulty::level(&n.to_string())?));
    }
    let recorded = branches(root);
    let filled = |v: &Option<String>| v.clone().filter(|b| !b.trim().is_empty());
    let base = branch::choose_base(
        &bag,
        &recorded,
        filled(&new.branch).as_deref(),
        filled(&new.base).as_deref(),
    )?;
    let mut warnings = Vec::new();
    match &base {
        Some(c) => warnings.extend(branch::other_bases(
            &bag, &recorded, actor.by, &task.id, &c.base,
        )),
        // The app adds without a branch when no open task has a base: nothing to say.
        None if actor.via == "alm" => warnings.push(Problem::BranchMissing {
            id: task.id.clone(),
        }),
        None => {}
    }
    bag.tasks.push(task.clone());
    let planned = match new
        .iteration
        .as_deref()
        .map(str::trim)
        .filter(|i| !i.is_empty())
    {
        Some(iteration) => Some(ops::plan(&bag, &task.id, Some(iteration))?.task),
        None => None,
    };
    let mut changes = event::changes(None, &task, event::TASK_FIELDS);
    if let Some(c) = &base {
        changes.extend(branch::change(branch::BASE, None, &c.base));
    }
    let added = actor.event("add task", Kind::Task, Some(&task.id), changes);
    lock.record(&added, |l| l.create_task(&task))?;
    let Some(planned) = planned else {
        return Ok(AddedTask {
            task,
            base,
            warnings,
            issues,
        });
    };
    let plan = actor.event(
        "plan",
        Kind::Task,
        Some(&task.id),
        event::changes(Some(&task), &planned, event::TASK_FIELDS),
    );
    lock.record(&plan, |l| l.save_task(&planned))?;
    Ok(AddedTask {
        task: planned,
        base,
        warnings,
        issues,
    })
}

/// One change to one task, the same way for `alm` and the app (Task-A01-18): refuse a taskbag in
/// another format, take the lock (waiting up to `wait` for another writer), load, apply `rule`
/// (a `crates/core` operation, which also names its command), then write the task and its event.
/// A refused change writes nothing.
pub fn change_task(
    root: &Path,
    actor: Actor,
    wait: Duration,
    rule: impl FnOnce(&Taskbag) -> Result<(allomos_core::ops::Outcome, &'static str), Problem>,
) -> Result<Changed, Problem> {
    change_task_under_lock(root, actor, wait, rule, false).map(|alone| alone.changed)
}

/// What [`change_task_alone`] did: the change, and what a window needs to show it without reading
/// the whole taskbag again (Task-A01-parade485).
#[derive(Debug, Clone, PartialEq)]
pub struct ChangedAlone {
    pub changed: Changed,
    /// When the change was recorded (its event's `at`, UTC): the task's new "last changed". None
    /// when nothing changed, so no event was written.
    pub at: Option<String>,
    /// The change also wrote `project.json` (a label new to the label table): more than this one
    /// task changed.
    pub project_changed: bool,
    /// The taskbag's fingerprint ([`stamp`]) just before the change was written and just after,
    /// both taken under the lock, so no other writer's change falls between them: a window whose
    /// last fingerprint is `stamp_before` has seen everything but this change.
    pub stamp_before: u64,
    pub stamp_after: u64,
}

/// [`change_task`] for a window that shows the taskbag (Task-A01-parade485): the same change, the
/// same way, and with it what the window needs to update this one task in place.
pub fn change_task_alone(
    root: &Path,
    actor: Actor,
    wait: Duration,
    rule: impl FnOnce(&Taskbag) -> Result<(allomos_core::ops::Outcome, &'static str), Problem>,
) -> Result<ChangedAlone, Problem> {
    change_task_under_lock(root, actor, wait, rule, true)
}

fn change_task_under_lock(
    root: &Path,
    actor: Actor,
    wait: Duration,
    rule: impl FnOnce(&Taskbag) -> Result<(allomos_core::ops::Outcome, &'static str), Problem>,
    stamps: bool,
) -> Result<ChangedAlone, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let (mut outcome, command) = rule(&bag)?;
    // Only a window asks for the fingerprints: `alm` does not pay for them.
    let stamp_before = if stamps { stamp(root) } else { 0 };
    // A label the table does not list yet is added to it, and `alm` says so (decision 0020, 5).
    let mut project = bag.project.clone();
    let added = allomos_core::labels::add_found(&mut project, &outcome.task.labels);
    let project_changed = !added.is_empty();
    if !added.is_empty() {
        let event = actor.event(
            "labels",
            Kind::Project,
            Some(&project.id),
            event::changes(Some(&bag.project), &project, &[]),
        );
        lock.record(&event, |l| l.save_project(&project))?;
        outcome
            .warnings
            .extend(added.into_iter().map(|label| Problem::LabelAddedToTable {
                id: outcome.task.id.clone(),
                label,
            }));
    }
    let before = bag.tasks.iter().find(|t| t.id == outcome.task.id);
    let described = before.is_some_and(|b| b.description != outcome.task.description);
    let mut changes = event::changes(before, &outcome.task, event::TASK_FIELDS);
    changes.extend(outcome.recorded.iter().cloned());
    // The holder's change is a heartbeat too (decision 0022 § 5): no extra commit, no event. A
    // change that changes nothing writes none either.
    if !changes.is_empty() {
        heartbeat::along(&mut outcome.task, actor.by, now_secs());
    }
    let event = actor.event(command, Kind::Task, Some(&outcome.task.id), changes);
    lock.record(&event, |l| {
        l.save_task(&outcome.task)?;
        if described {
            l.save_description(&outcome.task)?;
        }
        Ok(())
    })?;
    let stamp_after = if stamps { stamp(root) } else { 0 };
    let unblocked = freed_by(&bag, &outcome.task);
    Ok(ChangedAlone {
        at: (!event.changes.is_empty()).then(|| event.at.clone()),
        project_changed,
        stamp_before,
        stamp_after,
        changed: Changed {
            task: outcome.task,
            warnings: outcome.warnings,
            issues,
            unblocked,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use allomos_core::ops;
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

    /// Decision 0026: finishing a task's last open blocker changes nothing in that task; the
    /// change only reports it as freed.
    #[test]
    fn finishing_the_last_blocker_frees_a_task_without_changing_it() {
        let dir = tmp("freed");
        let root = dir.join(DEFAULT_DIR);
        init(&root, "p", "P").unwrap();
        let lock = Lock::acquire(&root, Duration::ZERO).unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.create_area(&ops::new_area(&bag, "A01", "First").unwrap())
            .unwrap();
        for id in ["A01-01", "A01-02", "A01-03"] {
            let bag = lock.load().taskbag.unwrap();
            lock.create_task(&ops::new_task(&bag, id, "A01", id, None).unwrap())
                .unwrap();
        }
        drop(lock);
        let me = || Actor::alm(Some("s"));
        let by = |ids: &[&str]| ids.iter().map(|i| i.to_string()).collect::<Vec<_>>();
        // A01-03 (backlog) is blocked by A01-01 and A01-02.
        let c = change_task(&root, me(), Duration::ZERO, |bag| {
            ops::block(bag, "A01-03", Some("s"), &by(&["A01-01", "A01-02"])).map(|o| (o, "block"))
        })
        .unwrap();
        assert_eq!(c.task.state, allomos_core::State::Backlog);
        let done = |id: &'static str| {
            change_task(&root, me(), Duration::ZERO, move |bag| {
                ops::finish(bag, id, Some("s"), None, None).map(|o| (o, "done"))
            })
            .unwrap()
        };
        assert!(done("A01-01").unblocked.is_empty());
        let events = events::list(&root).0.len();
        let freed = done("A01-02").unblocked;
        assert_eq!(freed.len(), 1);
        assert_eq!(freed[0].id, "A01-03");
        // Only the `done` event: the freed task is as it was, blockers and all.
        assert_eq!(events::list(&root).0.len(), events + 1);
        let bag = load(&root).taskbag.unwrap();
        let t = bag.tasks.iter().find(|t| t.id == "A01-03").unwrap();
        assert_eq!(t.state, allomos_core::State::Backlog);
        assert_eq!(t.blocked_by, by(&["A01-01", "A01-02"]));
        assert!(!ops::is_blocked(&bag, t));
        fs::remove_dir_all(dir).unwrap();
    }
}
