//! One task changed in the window (Task-A01-parade485): what the window gets back, so it shows the
//! change by replacing that one task in its list instead of reading the whole taskbag again.
//!
//! Reading the whole taskbag reads every task file and the history several times over (seconds on
//! a taskbag with thousands of events); the change itself already holds everything about the one
//! task it changed.

use std::path::Path;

use allomos_core::ops::Edit;
use allomos_core::{Problem, Task};
use serde::Serialize;

/// A task as an edit left it, with the parts of [`crate::taskbag_commands::Opened`] that follow
/// one task.
#[derive(Debug, Serialize)]
pub(crate) struct TaskChange {
    pub(crate) task: Task,
    /// The task's new version (`Opened::versions`).
    version: String,
    /// When the change was recorded (`Opened::last_changed`); `None` when nothing changed.
    changed_at: Option<String>,
    /// The card file its `Card:` link points to outside the taskbag, if any
    /// (`Opened::cards_outside`, for a task no agent reported; a report stays as it is).
    card_outside: Option<allomos_core::card::Outside>,
    /// The change reached beyond this task (the label table, the outside witness's checkpoint):
    /// the window reads the whole taskbag again, as before.
    whole: bool,
    /// The taskbag's fingerprint just before the change was written and just after, taken under
    /// the lock; as text, like `taskbag_stamp`. A window whose last fingerprint is `stamp_before`
    /// has seen everything but this change, and goes on from `stamp_after`; any other window
    /// reads the whole taskbag again (another session changed it too).
    stamp_before: String,
    stamp_after: String,
}

/// Make `edit` to task `id` as the app (`actor`), the same change through the same rule, lock and
/// event as before, and tell the window what it changed.
pub(crate) fn edit(
    root: &Path,
    actor: allomos_store::Actor,
    wait: std::time::Duration,
    id: &str,
    edit: &Edit,
) -> Result<TaskChange, Problem> {
    let alone = allomos_store::change_task_alone(root, actor, wait, |bag| {
        allomos_core::ops::edit(bag, id, actor.by, edit)
    })?;
    let task = alone.changed.task;
    // The person's check: the outside witness may make a checkpoint (Task-A01-bake791), which
    // writes more files after the fingerprint was taken.
    let checked = matches!(edit, Edit::Checked { .. });
    if checked {
        crate::audit::after_approval(root);
    }
    Ok(TaskChange {
        version: allomos_core::ops::task_version(&task),
        changed_at: alone.at,
        card_outside: allomos_store::cards::linked_outside(root, &task),
        whole: alone.project_changed || checked,
        stamp_before: alone.stamp_before.to_string(),
        stamp_after: alone.stamp_after.to_string(),
        task,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};
    use std::path::Path;

    /// The taskbag as the window reads it whole, as JSON.
    fn read(root: &Path) -> Value {
        serde_json::to_value(crate::taskbag_commands::open_taskbag(root.to_path_buf())).unwrap()
    }

    fn task_of<'a>(opened: &'a Value, id: &str) -> &'a Value {
        opened["loaded"]["taskbag"]["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["id"] == id)
            .unwrap()
    }

    /// `opened` without what belongs to task `id` alone: what a change to that one task must
    /// leave exactly as it was.
    fn without(opened: &Value, id: &str) -> Value {
        let mut v = opened.clone();
        v["loaded"]["taskbag"]["tasks"]
            .as_array_mut()
            .unwrap()
            .retain(|t| t["id"] != id);
        v["cards_outside"]
            .as_array_mut()
            .unwrap()
            .retain(|c| c["task"] != id);
        for map in ["versions", "last_changed"] {
            v[map].as_object_mut().unwrap().remove(id);
        }
        v
    }

    /// Task-A01-parade485: for every kind of edit, what `edit_task` answers is exactly what
    /// reading the whole taskbag again shows for that task, and nothing else in the taskbag
    /// differs, so the window can replace the one task in place. Where more did change, the
    /// answer says so (`whole`). The fingerprints are the taskbag's just before and just after.
    #[test]
    fn what_an_edit_answers_is_what_reading_the_whole_taskbag_again_shows() {
        let dir = std::env::temp_dir().join(format!(
            "allomos-desktop-task-change-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join(allomos_store::DEFAULT_DIR);
        allomos_store::init(&root, "p", "P").unwrap();
        let wait = std::time::Duration::from_secs(5);
        let app = allomos_store::Actor::APP;
        allomos_store::add_area(&root, app, wait, None, "A").unwrap();
        let iteration: crate::task_commands::NewIteration =
            serde_json::from_value(json!({ "name": "One" })).unwrap();
        let iteration = crate::task_commands::add_iteration_as(&root, Some("Will"), iteration)
            .unwrap()
            .id;
        let mut ids = Vec::new();
        for title in ["First", "Second", "Third"] {
            let new: allomos_store::NewTask =
                serde_json::from_value(json!({ "area": "A01", "title": title })).unwrap();
            ids.push(
                allomos_store::add_task(&root, app, wait, &new)
                    .unwrap()
                    .item
                    .id,
            );
        }
        let (first, second) = (ids[0].as_str(), ids[1].as_str());

        // (the task, the edit as the interface sends it, whether more than the task changes)
        let edits = [
            (first, json!({ "field": "state", "state": "ready" }), false),
            (second, json!({ "field": "blocked_by", "ids": [first] }), false),
            (first, json!({ "field": "title", "title": "First, renamed" }), false),
            (
                first,
                json!({ "field": "description", "text": "Card: [card](../../../../docs/first.md)\n\nThe plan." }),
                false,
            ),
            (first, json!({ "field": "description", "text": "The plan, with no card." }), false),
            (first, json!({ "field": "iteration", "iteration": iteration }), false),
            (first, json!({ "field": "difficulty", "difficulty": 3 }), false),
            (first, json!({ "field": "handoff_note", "text": "Stopped here." }), false),
            (first, json!({ "field": "next_dependency", "text": "The review." }), false),
            (
                first,
                json!({ "field": "state", "state": "waiting", "waiting_on": "an answer", "who": null }),
                false,
            ),
            (first, json!({ "field": "waiting_on", "text": "another answer" }), false),
            (first, json!({ "field": "label", "label": "locally verified by agent" }), false),
            // The same label again: nothing changes, no event is written.
            (first, json!({ "field": "label", "label": "locally verified by agent" }), false),
            (first, json!({ "field": "state", "state": "done" }), false),
            // A label the table does not list yet is added to `project.json` too.
            (
                second,
                json!({ "field": "labels", "labels": ["a label new to the table"] }),
                true,
            ),
            // The person's check may write the outside witness's checkpoint.
            (first, json!({ "field": "checked" }), true),
            (first, json!({ "field": "reopen", "note": "One more thing." }), false),
        ];
        let mut cards_linked = 0;
        for (id, edit, whole) in edits {
            let before = read(&root);
            let stamp_before = crate::taskbag_commands::taskbag_stamp(root.clone());
            let parsed: allomos_core::ops::Edit = serde_json::from_value(edit.clone()).unwrap();
            let change = crate::task_commands::change_task_as(&root, Some("Will"), id, &parsed)
                .unwrap_or_else(|p| panic!("{edit}: {}", p.code()));
            let change = serde_json::to_value(change).unwrap();
            let after = read(&root);

            assert_eq!(change["whole"], whole, "{edit}");
            assert_eq!(change["stamp_before"], stamp_before, "{edit}");
            assert_eq!(change["stamp_after"], crate::taskbag_commands::taskbag_stamp(root.clone()), "{edit}");
            // The task, its version and when it last changed, as the whole read has them.
            assert_eq!(&change["task"], task_of(&after, id), "{edit}");
            assert_eq!(change["version"], after["versions"][id], "{edit}");
            if change["changed_at"].is_null() {
                assert_eq!(before["last_changed"][id], after["last_changed"][id], "{edit}");
                assert_eq!(task_of(&before, id), task_of(&after, id), "{edit}");
            } else {
                assert_eq!(change["changed_at"], after["last_changed"][id], "{edit}");
            }
            let card = after["cards_outside"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["task"] == id)
                .cloned()
                .unwrap_or(Value::Null);
            assert_eq!(change["card_outside"], card, "{edit}");
            cards_linked += usize::from(!card.is_null());
            // Nothing else differs, unless the answer says to read the whole taskbag again.
            if !whole {
                assert_eq!(without(&before, id), without(&after, id), "{edit}");
            }
        }
        // The description that links a card file outside the taskbag was listed, then not.
        assert_eq!(cards_linked, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
