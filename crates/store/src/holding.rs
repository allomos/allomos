//! Holding a task: claiming it, heartbeats, its branches and traces.

use super::*;

/// Now, in seconds since 1970 (UTC); `ALLOMOS_NOW` fixes it in tests, as for events.
pub fn now_secs() -> i64 {
    heartbeat::parse_utc(&events::now_utc()).unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    })
}

/// What `alm beat` did ([`beat`]).
#[derive(Debug, Clone, PartialEq)]
pub struct Beaten {
    /// One entry for each task beaten for, in ID order: written, or too soon to write again.
    pub beats: Vec<heartbeat::Beat>,
    /// The project's heartbeat interval that applied, in seconds (Task-A01-zero387).
    pub interval_secs: i64,
    /// Whether the project asks for work in progress to be pushed at each heartbeat
    /// (Task-A01-butter895).
    pub push_work: bool,
    pub issues: Vec<Issue>,
}

/// `alm beat` (decision 0022, Task-A01-nurse164): write the heartbeat of task `id`, or with no
/// `id` of every task the session `actor.by` holds (in progress), in one change under the lock.
/// `busy`: a busy window of that many seconds. No event is written, and a task whose heartbeat is
/// younger than half the interval is left as it is. Refused without a session name
/// (`holder_missing`), for a task the session does not hold (`task_held`, `not_held`).
pub fn beat(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: Option<&str>,
    busy: Option<i64>,
    doing: Option<&str>,
) -> Result<Beaten, Problem> {
    let me = actor.by.ok_or(Problem::HolderMissing)?;
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let now = now_secs();
    let applied = heartbeat::applied(&bag.project);
    let interval = applied.interval_secs;
    let tasks: Vec<&Task> = match id {
        Some(id) => vec![
            bag.tasks
                .iter()
                .find(|t| t.id == id)
                .ok_or_else(|| Problem::TaskNotFound { id: id.to_string() })?,
        ],
        None => {
            let mut held: Vec<&Task> = bag
                .tasks
                .iter()
                .filter(|t| {
                    t.state == allomos_core::State::InProgress && t.holder.as_deref() == Some(me)
                })
                .collect();
            held.sort_by(|a, b| a.id.cmp(&b.id));
            held
        }
    };
    let beats = tasks
        .into_iter()
        .map(|t| heartbeat::beat(t, me, now, busy, doing, interval))
        .collect::<Result<Vec<_>, _>>()?;
    for b in &beats {
        if let heartbeat::Beat::Written(task) = b {
            lock.save_task(task)?;
        }
    }
    Ok(Beaten {
        beats,
        interval_secs: interval,
        push_work: applied.push_work,
        issues,
    })
}

/// After a change that leaves `task` finished (done or dropped): the tasks it was the last open
/// blocker of, which are no longer blocked. Nothing is written: being blocked is worked out from
/// `blocked_by`, and each task keeps its state (decision 0026, replacing Task-A01-tuna668's
/// automatic move to ready). The caller only tells.
pub(crate) fn freed_by(bag: &Taskbag, task: &Task) -> Vec<Task> {
    if !matches!(
        task.state,
        allomos_core::State::Done | allomos_core::State::Dropped
    ) {
        return Vec::new();
    }
    let mut after = bag.clone();
    match after.tasks.iter_mut().find(|t| t.id == task.id) {
        Some(t) => *t = task.clone(),
        None => after.tasks.push(task.clone()),
    }
    bag.tasks
        .iter()
        .filter(|t| t.id != task.id && t.blocked_by.contains(&task.id))
        .filter(|t| ops::is_blocked(bag, t) && !ops::is_blocked(&after, t))
        .cloned()
        .collect()
}

/// Every task's branches (base and working), by task ID (Task-A03-22), from the history.
pub fn branches(root: &Path) -> std::collections::BTreeMap<String, branch::Branches> {
    branch::recorded(&events::list(root).0)
}

/// When each task last changed, by task ID (Task-A01-52): the newest `at` among its events, in
/// UTC. The Iterations tab sorts by it. A task with no recorded change is left out.
pub fn last_changed(root: &Path) -> std::collections::BTreeMap<String, String> {
    let mut out: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    for e in events::list(root).0 {
        let (Kind::Task, Some(id)) = (e.kind, e.id) else {
            continue;
        };
        let at = out.entry(id).or_default();
        if e.at > *at {
            *at = e.at;
        }
    }
    out
}

/// When each task was made, by task ID (Task-A01-54): the oldest `at` among its events, in UTC.
/// Lists order word IDs by it. A task with no recorded change is left out.
pub fn created(root: &Path) -> std::collections::BTreeMap<String, String> {
    let mut out: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    for e in events::list(root).0 {
        let (Kind::Task, Some(id)) = (e.kind, e.id) else {
            continue;
        };
        let at = out.entry(id).or_insert_with(|| e.at.clone());
        if e.at < *at {
            *at = e.at;
        }
    }
    out
}

/// What [`claim_task`] did: the change, and the task's branches after it.
#[derive(Debug, Clone, PartialEq)]
pub struct Claimed {
    pub changed: Changed,
    /// What the claim recorded, or found, about the task's branches.
    pub branches: branch::AtClaim,
    /// Set when the claim took the task over from another holder (Task-A01-sign613).
    pub took_over: Option<TookOver>,
}

/// A claim that took a task over from another holder (decision 0022 § 6, Task-A01-sign613): from
/// whom, whether it was lost (and since when), and where the former holder worked, when its claim
/// recorded that. The claim's event records the change of holder; this is what `alm claim` tells.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TookOver {
    /// The former holder.
    pub from: String,
    /// Set when the task was lost; `None` when it was taken with `--take` while not lost.
    pub lost: Option<heartbeat::Lost>,
    /// The folder the former holder's claim ran in, relative to the folder holding `.allomos/`,
    /// when it recorded one. It may still hold uncommitted changes.
    pub folder: Option<String>,
    /// The working branch the task had before the claim.
    pub working: Option<String>,
    /// What the former holder last said it was doing (`alm beat --doing`), and when, in UTC.
    pub doing: Option<String>,
    pub doing_at: Option<String>,
}

/// Claim task `id` (`alm claim`, Task-A03-22): the claim rule, plus its branches. `current` is the
/// branch the agent is on, `working` the branch the work is done on (default `current`). A task
/// without a base gets `current`. `session` is the agent's session as it passed it (Task-A03-hive049):
/// it replaces the task's last one, and nothing passed clears it. One `claim` event holds both the
/// task's changes and the branch changes.
#[allow(clippy::too_many_arguments)]
pub fn claim_task(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    take: bool,
    current: Option<&str>,
    working: Option<&str>,
    session: Option<allomos_core::AgentSession>,
) -> Result<Claimed, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    // A lost task may be claimed: claiming it takes it over (decision 0022 § 6, Task-A01-sign613).
    // One held by someone else and not lost is refused as before, unless `--take`.
    let now = now_secs();
    let found = bag.tasks.iter().find(|t| t.id == id);
    let former = found
        .filter(|t| !matches!(t.state, State::Done | State::Dropped))
        .and_then(|t| t.holder.clone())
        .filter(|h| Some(h.as_str()) != actor.by.map(str::trim));
    let lost = match (found, &former) {
        (Some(t), Some(_)) => {
            let applied = heartbeat::applied(&bag.project);
            let last = last_changed(root);
            heartbeat::lost(
                t,
                now,
                applied.lost_after_secs,
                last.get(&t.id).map(String::as_str),
            )
        }
        _ => None,
    };
    let mut outcome = ops::claim(&bag, id, actor.by, take || lost.is_some())?;
    if let Some(l) = &lost {
        for w in outcome.warnings.iter_mut() {
            if let Problem::TookOver { id, previous } = w {
                *w = Problem::TookOverLost {
                    id: id.clone(),
                    previous: previous.clone(),
                    since: l.since.clone(),
                };
            }
        }
    }
    let before = branches(root).remove(id).unwrap_or_default();
    let took_over = former.map(|from| TookOver {
        from,
        lost: lost.clone(),
        folder: found
            .and_then(|t| t.session.as_ref())
            .and_then(|s| s.folder.clone()),
        working: before.working.clone(),
        doing: found
            .and_then(heartbeat::doing)
            .map(|(words, _)| words.to_string()),
        doing_at: found
            .and_then(heartbeat::doing)
            .and_then(|(_, at)| at.map(str::to_string)),
    });
    // Work goes on on the task's working branch (decision 0022 § 6): a takeover keeps it unless
    // `--working` names another.
    let working = match (&took_over, working) {
        (_, Some(w)) => Some(w),
        (Some(t), None) => t.working.as_deref().or(current),
        (None, None) => None,
    };
    outcome.task.session = session
        .filter(|s| !s.is_empty())
        .map(|s| allomos_core::AgentSession {
            by: outcome.task.holder.clone(),
            ..s
        });
    let at = branch::at_claim(id, &before, current, working)?;
    let old = bag.tasks.iter().find(|t| t.id == outcome.task.id);
    let mut changes = event::changes(old, &outcome.task, event::TASK_FIELDS);
    changes.extend(at.changes.iter().cloned());
    if !changes.is_empty() {
        heartbeat::along(&mut outcome.task, actor.by, now);
    }
    let event = actor.event("claim", Kind::Task, Some(&outcome.task.id), changes);
    lock.record(&event, |l| l.save_task(&outcome.task))?;
    let mut warnings = outcome.warnings;
    warnings.extend(at.warnings.iter().cloned());
    Ok(Claimed {
        changed: Changed {
            task: outcome.task,
            warnings,
            issues,
            unblocked: Vec::new(),
        },
        branches: at,
        took_over,
    })
}

/// Change task `id`'s base to `name` (`alm base`, Task-A03-22): one `base` event, under the lock;
/// the task's files do not change. Returns the task and its branches after it.
pub fn set_base(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    name: &str,
) -> Result<(Task, branch::Branches, Vec<Issue>), Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let recorded = branches(root);
    let changes = branch::set_base(&bag, &recorded, id, actor.by, name)?;
    let task = ops::find_task(&bag, id)?;
    let event = actor.event(branch::SET_BASE, Kind::Task, Some(&task.id), changes);
    lock.record(&event, |_| Ok(()))?;
    let after = branches(root).remove(&task.id).unwrap_or_default();
    Ok((task, after, issues))
}

/// Every task's traces (the requirements and decisions it serves), by task ID
/// (Task-A01-fennel617), from the history.
pub fn traces(root: &Path) -> std::collections::BTreeMap<String, Vec<String>> {
    allomos_core::traces::recorded(&events::list(root).0)
}

/// Add and remove task `id`'s traces (`alm trace`, Task-A01-fennel617): one `trace` event, under
/// the lock, when the list changes; the task's files do not change. Returns the task and its
/// traces after it.
pub fn set_traces(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    add: &[String],
    remove: &[String],
) -> Result<(Task, Vec<String>, Vec<Issue>), Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let recorded = traces(root);
    let (changes, after) = allomos_core::traces::set(&bag, &recorded, id, actor.by, add, remove)?;
    let task = ops::find_task(&bag, id)?;
    if !changes.is_empty() {
        let event = actor.event(
            allomos_core::traces::TRACE,
            Kind::Task,
            Some(&task.id),
            changes,
        );
        lock.record(&event, |_| Ok(()))?;
    }
    Ok((task, after, issues))
}
