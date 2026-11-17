//! AI decisions and approvals recorded in a task's history.

use super::*;

/// What [`record_ai_decision`] or [`review_ai_decisions`] did (Task-A03-19).
#[derive(Debug, Clone, PartialEq)]
pub struct Decided {
    pub task: Task,
    /// The decisions the change recorded or marked reviewed.
    pub decisions: Vec<decision::Decision>,
    /// The task's decisions since its last review after the change, details included (see
    /// [`decision::Decision::to_review`] for the ones that wait for review).
    pub pending: Vec<decision::Decision>,
    /// Problems found in the taskbag's files while loading it.
    pub issues: Vec<Issue>,
}

/// Every task's AI decisions since its last review, by task ID (Task-A03-19), from the history:
/// details too, which are kept but do not wait for review (decision 0025 § 3).
pub fn ai_decisions(root: &Path) -> std::collections::BTreeMap<String, Vec<decision::Decision>> {
    decision::pending(&events::list(root).0)
}

/// Record a decision an agent made on its own in task `id` (`alm ai-decided`, Task-A03-19): one
/// `ai-decided` event, under the lock. The task's files do not change.
pub fn record_ai_decision(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    text: &str,
    kind: Option<decision::DecisionKind>,
) -> Result<Decided, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let mut changes = decision::decide(&bag, id, text, kind)?;
    let before = ops::find_task(&bag, id)?;
    // The task carries `AI decisions to review` until `alm reviewed` (decision 0020); a detail
    // is kept but does not wait for review (decision 0025 § 3).
    let mut task = before.clone();
    if kind != Some(decision::DecisionKind::Detail) {
        task.add_label(allomos_core::labels::AI_DECISIONS);
    }
    heartbeat::along(&mut task, actor.by, now_secs());
    changes.extend(event::changes(Some(&before), &task, event::TASK_FIELDS));
    let event = actor.event(decision::DECIDED, Kind::Task, Some(&task.id), changes);
    lock.record(&event, |l| {
        if task != before {
            l.save_task(&task)?;
        }
        Ok(())
    })?;
    let pending = ai_decisions(root).remove(&task.id).unwrap_or_default();
    let decisions = pending.last().cloned().into_iter().collect();
    Ok(Decided {
        task,
        decisions,
        pending,
        issues,
    })
}

/// Record in task `id`'s history that the person chose Go ahead on a plan shown through
/// `alm confirm --task` (Task-A03-37): since Task-A01-coach782 an approval record
/// ([`approvals::approve`], channel `confirm`) whose scope holds the SHA-256 of exactly the plan
/// text shown (`plan`, from the file `plan_file`). A changed plan is a new question.
pub fn record_approval(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    title: &str,
    plan_file: &str,
    plan: &str,
) -> Result<approvals::Approved, Problem> {
    let tasks = [id.to_string()];
    approvals::approve(
        root,
        actor,
        wait,
        &approvals::Input {
            tasks: &tasks,
            approver: None,
            channel: "confirm",
            words: Some(&format!("Go ahead: {title}")),
            expires: None,
            plan: Some((plan_file, plan)),
            shown: None,
        },
    )
}

/// Mark task `id`'s AI decisions reviewed (`alm reviewed`, the app's Reviewed button,
/// Task-A03-19): one `reviewed` event listing each, under the lock. Refused with
/// `nothing_to_review` when it has none. Overturning one is done by changing the task as usual.
pub fn review_ai_decisions(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
) -> Result<Decided, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let before = ops::find_task(&bag, id)?;
    let decisions = ai_decisions(root).remove(&before.id).unwrap_or_default();
    let mut task = before.clone();
    let had_label = task.remove_label(allomos_core::labels::AI_DECISIONS);
    heartbeat::along(&mut task, actor.by, now_secs());
    // A task carrying the label with nothing left in its history is cleared too (decision 0020).
    // Details alone are nothing to review (decision 0025 § 3); a review lists them with the rest.
    if !had_label && !decisions.iter().any(|d| d.to_review()) {
        return Err(Problem::NothingToReview { id: before.id });
    }
    let mut changes = if decisions.is_empty() {
        Vec::new()
    } else {
        decision::review(&bag, id, &decisions)?
    };
    changes.extend(event::changes(Some(&before), &task, event::TASK_FIELDS));
    let event = actor.event(decision::REVIEWED, Kind::Task, Some(&task.id), changes);
    lock.record(&event, |l| {
        if task != before {
            l.save_task(&task)?;
        }
        Ok(())
    })?;
    Ok(Decided {
        task,
        decisions,
        pending: Vec::new(),
        issues,
    })
}
