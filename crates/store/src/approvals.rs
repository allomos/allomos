//! Approvals as records in the history (Task-A01-coach782, decision 0021 § 1.1): writing one
//! (`alm approve`, `alm confirm --task` on Go ahead) and withdrawing one (`alm withdraw`), under
//! the taskbag's lock. The rules are in [`allomos_core::approval`].

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use allomos_core::approval::{self, Approval, Recorded};
use allomos_core::event::Kind;
use allomos_core::{Issue, Problem, Task, ops};

use crate::events::{self, Actor};
use crate::lock_for_change;

/// What an approval is given (the rest, its ID, time and approver, `alm` fills in).
#[derive(Debug, Clone, Default)]
pub struct Input<'a> {
    pub tasks: &'a [String],
    /// The approver's pseudonym ID; `None`: this computer's person's pseudonym in this project.
    pub approver: Option<&'a str>,
    pub channel: &'a str,
    pub words: Option<&'a str>,
    pub expires: Option<&'a str>,
    /// The plan shown, as `(file name, its text)`; `None`: the tasks' descriptions.
    pub plan: Option<(&'a str, &'a str)>,
    /// The description the person read before approving it (the app's Approve, Task-A01-baker800):
    /// when the tasks' plan text is no longer exactly this, nothing is recorded
    /// (`task_changed`), so a person never approves a plan they did not see. `None`: not checked.
    pub shown: Option<&'a str>,
}

/// What [`approve`] or [`withdraw`] did.
#[derive(Debug, Clone, PartialEq)]
pub struct Approved {
    pub approval: Approval,
    /// The tasks it covers, as they are.
    pub tasks: Vec<Task>,
    /// Problems found in the taskbag's files while loading it.
    pub issues: Vec<Issue>,
}

/// Today in UTC, `YYYY-MM-DD` (`ALLOMOS_NOW` fixes it in tests).
fn today() -> String {
    events::now_utc().chars().take(10).collect()
}

/// Every task's approvals, by task ID, each with where it stands now.
pub fn approvals(root: &Path) -> BTreeMap<String, Vec<Recorded>> {
    let loaded = crate::load(root);
    let Some(bag) = loaded.taskbag else {
        return BTreeMap::new();
    };
    approval::by_task(&events::list(root).0, &bag, &today())
}

/// Every approval in the history, oldest first, each with where it stands now.
pub fn all(root: &Path) -> Vec<Recorded> {
    let loaded = crate::load(root);
    let Some(bag) = loaded.taskbag else {
        return Vec::new();
    };
    approval::all(&events::list(root).0, &bag, &today())
}

/// Record an approval: one `approve` event in each task it covers, under the lock. The tasks'
/// files do not change.
pub fn approve(
    root: &Path,
    actor: Actor,
    wait: Duration,
    input: &Input,
) -> Result<Approved, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    if let (Some(shown), None) = (input.shown, input.plan) {
        if approval::plan_text(&bag, input.tasks)? != shown {
            let task = ops::find_task(&bag, input.tasks.first().map_or("", String::as_str))?;
            return Err(Problem::TaskChanged {
                version: ops::task_version(&task),
                id: task.id,
            });
        }
    }
    let id = format!("ap-{}", events::random_suffix());
    let pseudonym = match input.approver {
        Some(a) => Some(a.trim().to_string()).filter(|a| !a.is_empty()),
        None => crate::identity::pseudonym(&bag.project.id),
    };
    let at = events::now_utc();
    let (approval, change) = approval::approve(
        &bag,
        &approval::Given {
            id: &id,
            tasks: input.tasks,
            approver: pseudonym.as_deref(),
            channel: input.channel,
            words: input.words,
            expires: input.expires,
            plan: input.plan,
            at: &at,
        },
    )?;
    let events: Vec<_> = approval
        .scope
        .tasks
        .iter()
        .map(|task_id| {
            let mut event = actor.event(
                approval::APPROVE,
                Kind::Task,
                Some(task_id),
                vec![change.clone()],
            );
            // The event relies on the approval it records.
            event.approval = Some(approval.id.clone());
            event
        })
        .collect();
    // Separation of duties (Task-A01-grotto036) is checked for every task first, so a refusal
    // for one leaves none of them approved.
    for event in &events {
        events::separate_duties(root, &events::as_written(root, event))?;
    }
    let mut tasks = Vec::new();
    for event in &events {
        lock.record(event, |_| Ok(()))?;
        tasks.push(ops::find_task(
            &bag,
            event.id.as_deref().unwrap_or_default(),
        )?);
    }
    Ok(Approved {
        approval,
        tasks,
        issues,
    })
}

/// Withdraw approval `id`: one `withdraw` event in each task it covers, under the lock, with
/// `reason` (the app's Withdraw asks for one; `alm withdraw --why` gives it through the change's
/// who) as a `reason` change. Refused for an ID no task records and for one already withdrawn.
pub fn withdraw(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    reason: Option<&str>,
) -> Result<Approved, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let id = id.trim();
    let found = approval::all(&events::list(root).0, &bag, &today())
        .into_iter()
        .find(|r| r.approval.id == id)
        .ok_or_else(|| Problem::ApprovalNotFound { id: id.to_string() })?;
    if found.withdrawn_at.is_some() {
        return Err(Problem::ApprovalAlreadyWithdrawn { id: id.to_string() });
    }
    let mut changes = vec![approval::withdraw(&found.approval)];
    if let Some(why) = reason.map(str::trim).filter(|r| !r.is_empty()) {
        changes.push(allomos_core::event::reason_change(why));
    }
    let mut tasks = Vec::new();
    for task_id in &found.approval.scope.tasks {
        // A task removed since keeps its history; the others get the event.
        let Ok(task) = ops::find_task(&bag, task_id) else {
            continue;
        };
        let mut event = actor.event(
            approval::WITHDRAW,
            Kind::Task,
            Some(&task.id),
            changes.clone(),
        );
        event.approval = Some(found.approval.id.clone());
        lock.record(&event, |_| Ok(()))?;
        tasks.push(task);
    }
    Ok(Approved {
        approval: found.approval,
        tasks,
        issues,
    })
}
