//! Task changes and their rules.
//!
//! Each function takes the taskbag as it is now and returns the changed item, or the
//! [`Problem`] that refuses the change. Nothing here touches the disk: the caller loads
//! the taskbag, applies one change and writes the result, all while holding the
//! taskbag's lock (see `crates/store`).

use crate::event::Change;
use crate::model::{Area, Iteration, Project, State, Task, Taskbag};
use crate::problem::Problem;
use crate::validate::valid_date;
use serde::Deserialize;

/// A changed task plus warnings: things worth telling the user that do not refuse the change.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub task: Task,
    pub warnings: Vec<Problem>,
    /// Changes recorded only in the task's history, not in its files, such as why a task was
    /// reopened (`reopen_note`, Task-A01-50). The writer adds them to the change's event.
    pub recorded: Vec<Change>,
}

impl Outcome {
    pub(crate) fn plain(task: Task) -> Self {
        Outcome {
            task,
            warnings: Vec::new(),
            recorded: Vec::new(),
        }
    }

    fn warned(task: Task, warnings: Vec<Problem>) -> Self {
        Outcome {
            task,
            warnings,
            recorded: Vec::new(),
        }
    }
}

/// 1–64 letters, digits, `-` or `_` (the same rule as the file checks).
pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn check_id(id: &str) -> Result<(), Problem> {
    if valid_id(id) {
        Ok(())
    } else {
        Err(Problem::InvalidId { id: id.to_string() })
    }
}

fn non_empty(text: Option<&str>) -> Option<String> {
    text.map(str::trim)
        .filter(|t| !t.is_empty())
        .map(String::from)
}

/// The task with this ID, cloned for changing.
pub fn find_task(taskbag: &Taskbag, id: &str) -> Result<Task, Problem> {
    taskbag
        .tasks
        .iter()
        .find(|t| t.id == id)
        .cloned()
        .ok_or_else(|| Problem::TaskNotFound { id: id.to_string() })
}

/// Refuse when someone other than `me` holds the task. An unheld task may be changed by anyone.
/// A finished or dropped task counts as held by nobody: the holder it keeps is history, not a
/// claim (approver, 2026-10-16, task A00-30).
pub(crate) fn check_holder(task: &Task, me: Option<&str>) -> Result<(), Problem> {
    if matches!(task.state, State::Done | State::Dropped) {
        return Ok(());
    }
    match &task.holder {
        Some(h) if Some(h.as_str()) != me => Err(Problem::TaskHeld {
            id: task.id.clone(),
            holder: h.clone(),
        }),
        _ => Ok(()),
    }
}

pub(crate) fn check_open(task: &Task, closed: &[State]) -> Result<(), Problem> {
    if closed.contains(&task.state) {
        Err(Problem::TaskClosed {
            id: task.id.clone(),
            state: task.state.as_str().to_string(),
        })
    } else {
        Ok(())
    }
}

/// A new area. Its ID must be valid and unused, and it needs a name.
pub fn new_area(taskbag: &Taskbag, id: &str, name: &str) -> Result<Area, Problem> {
    check_id(id)?;
    if taskbag.areas.iter().any(|a| a.id == id) {
        return Err(Problem::DuplicateAreaId { id: id.to_string() });
    }
    let name = non_empty(Some(name)).ok_or(Problem::AreaNameMissing)?;
    let order = taskbag.areas.iter().map(|a| a.order).max().unwrap_or(0) + 1;
    Ok(Area {
        id: id.to_string(),
        name,
        order,
    })
}

/// A new task ID in `area` (Task-A01-54): `<area>-<word><000-999>`, for example `A01-tiger042`,
/// drawn at random so tasks added on different branches do not clash ([`crate::ids`]); drawn
/// again if the ID is already in this taskbag. Refused when the area does not exist.
pub fn next_task_id(taskbag: &Taskbag, area: &str) -> Result<String, Problem> {
    if !taskbag.areas.iter().any(|a| a.id == area) {
        return Err(Problem::UnknownArea {
            area: area.to_string(),
        });
    }
    Ok(crate::ids::new_task_id(area, |id| {
        taskbag.tasks.iter().any(|t| t.id == id)
    }))
}

/// The next free ID after `ids` (in their order), following the project's own pattern
/// `<letters><number>`: the letters of the last such ID (`fallback` when there is none), one more
/// than the highest number used with them, as wide as the widest one (at least two digits). IDs of
/// another shape do not count. Shared by areas (`A08` after `A07`) and iterations (`I16`).
fn next_numbered_id<'a>(ids: impl Iterator<Item = &'a str> + Clone, fallback: &'a str) -> String {
    let split = |id: &'a str| -> Option<(&'a str, &'a str)> {
        let at = id.trim_end_matches(|c: char| c.is_ascii_digit()).len();
        (at < id.len()).then(|| id.split_at(at))
    };
    let prefix = ids
        .clone()
        .filter_map(split)
        .last()
        .map_or(fallback, |(p, _)| p);
    let (highest, width) = ids
        .clone()
        .filter_map(split)
        .filter(|(p, _)| *p == prefix)
        .filter_map(|(_, n)| n.parse::<u64>().ok().map(|v| (v, n.len())))
        .fold((0, 2), |(h, w), (v, len)| (h.max(v), w.max(len)));
    // Higher than every number used with these letters, so it is free in any spelling.
    format!("{prefix}{:0width$}", highest + 1)
}

/// The next free area ID (`alm add area` without one; the app's suggestion, Task-A01-19):
/// `A08` after `A00`–`A07`, `A01` in a taskbag with no areas.
pub fn next_area_id(taskbag: &Taskbag) -> String {
    let mut areas: Vec<&Area> = taskbag.areas.iter().collect();
    areas.sort_by_key(|a| a.order);
    next_numbered_id(areas.iter().map(|a| a.id.as_str()), "A")
}

/// The next free iteration ID (`alm add iteration` without one; the app's suggestion,
/// Task-A01-19): `I16` after `I01`–`I15`, `I01` in a taskbag with no iterations.
pub fn next_iteration_id(taskbag: &Taskbag) -> String {
    let mut iterations: Vec<&Iteration> = taskbag.iterations.iter().collect();
    iterations.sort_by_key(|i| i.order);
    next_numbered_id(iterations.iter().map(|i| i.id.as_str()), "I")
}

/// The project with a new task language (`alm language`): `ui` to follow the Allomos app's
/// language, or a language code. Refused when it is neither.
pub fn set_task_language(taskbag: &Taskbag, value: &str) -> Result<Project, Problem> {
    let value = value.trim();
    if value != crate::model::TASK_LANGUAGE_UI && !crate::model::valid_language(value) {
        return Err(Problem::InvalidLanguage {
            value: value.to_string(),
        });
    }
    Ok(Project {
        task_language: Some(value.to_string()),
        ..taskbag.project.clone()
    })
}

/// A change to the project's handoff rules (`alm handoff-rules`, the app's Handoff settings).
/// `agent`: the change is that agent's or account's override instead of the project's own rules.
/// `clear`: remove that override (with `agent`) or go back to the defaults (without).
#[derive(Debug, Clone, Default)]
pub struct HandoffChange<'a> {
    pub quota_gate: Option<&'a str>,
    pub context_gate: Option<&'a str>,
    pub write_when: Option<&'a str>,
    pub agent: Option<&'a str>,
    pub clear: bool,
}

fn percent(value: &str) -> Result<u8, Problem> {
    value
        .trim()
        .trim_end_matches('%')
        .parse::<u8>()
        .ok()
        .filter(|v| *v <= 100)
        .ok_or_else(|| Problem::InvalidPercent {
            value: value.to_string(),
        })
}

/// The project with changed handoff rules (Task-A03-06). Percentages must be whole numbers from 0
/// to 100; an override is named like a holder (`codex`, `claude/work`).
pub fn set_handoff_rules(taskbag: &Taskbag, change: &HandoffChange) -> Result<Project, Problem> {
    let quota = change.quota_gate.map(percent).transpose()?;
    let context = change.context_gate.map(percent).transpose()?;
    let mut rules = taskbag.project.handoff.clone().unwrap_or_default();
    match change.agent.map(str::trim) {
        Some("") => return Err(Problem::HolderMissing),
        Some(agent) => {
            if change.clear {
                rules.overrides.remove(agent);
            } else {
                let gates = rules.overrides.entry(agent.to_string()).or_default();
                if quota.is_some() {
                    gates.quota_gate = quota;
                }
                if context.is_some() {
                    gates.context_gate = context;
                }
            }
        }
        None => {
            if change.clear {
                rules = crate::model::HandoffRules {
                    overrides: rules.overrides,
                    ..Default::default()
                };
            }
            if quota.is_some() {
                rules.quota_gate = quota;
            }
            if context.is_some() {
                rules.context_gate = context;
            }
            if let Some(text) = change.write_when {
                let text = text.trim();
                rules.write_when = (!text.is_empty()).then(|| text.to_string());
            }
        }
    }
    let empty = rules == crate::model::HandoffRules::default();
    Ok(Project {
        handoff: (!empty).then_some(rules),
        ..taskbag.project.clone()
    })
}

/// The project with a new label ladder, lowest step first (`alm labels`). Refused when it is
/// empty, or when a task's label is not on it: changing the ladder never strands a task.
pub fn set_ladder(taskbag: &Taskbag, labels: &[String]) -> Result<Project, Problem> {
    let labels: Vec<String> = labels
        .iter()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    if labels.is_empty() {
        return Err(Problem::LadderEmpty);
    }
    if let Some(label) = taskbag
        .tasks
        .iter()
        .filter_map(|t| t.label.as_ref())
        .find(|l| !labels.contains(l))
    {
        return Err(Problem::LabelNotOnLadder {
            label: label.clone(),
            ladder: labels.join(" → "),
        });
    }
    Ok(Project {
        labels,
        ..taskbag.project.clone()
    })
}

/// A step of the ladder renamed, with every task that carries it (`alm rename label`,
/// Task-A01-32): the project with the new ladder (same order), and the tasks whose label changed,
/// done and dropped ones too. The old name must be on the ladder; the new one must not be.
pub fn rename_label(
    taskbag: &Taskbag,
    old: &str,
    new: &str,
) -> Result<(Project, Vec<Task>), Problem> {
    let old = old.trim();
    let new = non_empty(Some(new)).ok_or(Problem::LabelNameMissing)?;
    let ladder = &taskbag.project.labels;
    let Some(at) = ladder.iter().position(|l| l == old) else {
        return Err(Problem::LabelNotOnLadder {
            label: old.to_string(),
            ladder: ladder.join(" → "),
        });
    };
    if ladder.iter().any(|l| *l == new) {
        return Err(Problem::LabelExists { label: new });
    }
    let mut project = taskbag.project.clone();
    project.labels[at] = new.clone();
    let tasks = taskbag
        .tasks
        .iter()
        .filter(|t| t.label.as_deref() == Some(old))
        .map(|t| Task {
            label: Some(new.clone()),
            ..t.clone()
        })
        .collect();
    Ok((project, tasks))
}

/// The parts of a new iteration besides its ID and name; all optional.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IterationDetails<'a> {
    pub release: Option<&'a str>,
    pub start: Option<&'a str>,
    pub end: Option<&'a str>,
}

/// A new iteration, ordered after the existing ones. Its ID must be valid and unused, it needs a
/// name, and its dates must be `YYYY-MM-DD`.
pub fn new_iteration(
    taskbag: &Taskbag,
    id: &str,
    name: &str,
    details: IterationDetails,
) -> Result<Iteration, Problem> {
    check_id(id)?;
    if taskbag.iterations.iter().any(|i| i.id == id) {
        return Err(Problem::DuplicateIterationId { id: id.to_string() });
    }
    let name = non_empty(Some(name)).ok_or(Problem::IterationNameMissing)?;
    let date = optional_date;
    let order = taskbag
        .iterations
        .iter()
        .map(|i| i.order)
        .max()
        .unwrap_or(0)
        + 1;
    Ok(Iteration {
        id: id.to_string(),
        name,
        order,
        start: date(details.start)?,
        end: date(details.end)?,
        release: non_empty(details.release),
    })
}

/// `YYYY-MM-DD`, or empty; the same check as a new iteration's dates.
fn optional_date(d: Option<&str>) -> Result<Option<String>, Problem> {
    match non_empty(d) {
        Some(d) if !valid_date(&d) => Err(Problem::InvalidDate { value: d }),
        d => Ok(d),
    }
}

fn find_area(taskbag: &Taskbag, id: &str) -> Result<Area, Problem> {
    taskbag
        .areas
        .iter()
        .find(|a| a.id == id)
        .cloned()
        .ok_or_else(|| Problem::UnknownArea {
            area: id.to_string(),
        })
}

fn find_iteration(taskbag: &Taskbag, id: &str) -> Result<Iteration, Problem> {
    taskbag
        .iterations
        .iter()
        .find(|i| i.id == id)
        .cloned()
        .ok_or_else(|| Problem::UnknownIteration {
            iteration: id.to_string(),
        })
}

/// The area with a new name (`alm rename area`, Task-A01-29). Its ID and order stay; an empty
/// name is refused.
pub fn rename_area(taskbag: &Taskbag, id: &str, name: &str) -> Result<Area, Problem> {
    let mut area = find_area(taskbag, id)?;
    area.name = non_empty(Some(name)).ok_or(Problem::AreaNameMissing)?;
    Ok(area)
}

/// What `alm change iteration` changes (Task-A01-29). A field left out (`None`) stays as it is;
/// an empty release or date clears it. The name cannot be emptied.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IterationChange {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub release: Option<String>,
    #[serde(default)]
    pub start: Option<String>,
    #[serde(default)]
    pub end: Option<String>,
}

/// The iteration with `change` applied (`alm change iteration`, Task-A01-29). Its ID and order
/// stay; dates are checked as `alm add iteration` checks them.
pub fn change_iteration(
    taskbag: &Taskbag,
    id: &str,
    change: &IterationChange,
) -> Result<Iteration, Problem> {
    let mut iteration = find_iteration(taskbag, id)?;
    if let Some(name) = &change.name {
        iteration.name = non_empty(Some(name)).ok_or(Problem::IterationNameMissing)?;
    }
    if let Some(release) = &change.release {
        iteration.release = non_empty(Some(release));
    }
    if let Some(start) = &change.start {
        iteration.start = optional_date(Some(start))?;
    }
    if let Some(end) = &change.end {
        iteration.end = optional_date(Some(end))?;
    }
    Ok(iteration)
}

/// The area to delete (`alm delete area`, Task-A01-29). Refused while any task is in it, done
/// and dropped ones included, so history never points at an area that is gone (owner,
/// 2026-10-14).
pub fn delete_area(taskbag: &Taskbag, id: &str) -> Result<Area, Problem> {
    let area = find_area(taskbag, id)?;
    let count = taskbag.tasks.iter().filter(|t| t.area == id).count();
    if count > 0 {
        return Err(Problem::AreaNotEmpty {
            id: id.to_string(),
            count: count.to_string(),
        });
    }
    Ok(area)
}

/// The iteration to delete (`alm delete iteration`, Task-A01-29). Refused while any task is
/// planned in it, done and dropped ones included (owner, 2026-10-14).
pub fn delete_iteration(taskbag: &Taskbag, id: &str) -> Result<Iteration, Problem> {
    let iteration = find_iteration(taskbag, id)?;
    let count = taskbag
        .tasks
        .iter()
        .filter(|t| t.iteration.as_deref() == Some(id))
        .count();
    if count > 0 {
        return Err(Problem::IterationNotEmpty {
            id: id.to_string(),
            count: count.to_string(),
        });
    }
    Ok(iteration)
}

/// Plan the task in an iteration, or take it out of its iteration with `None`. Planning is not
/// progress, so anyone may do it, whoever holds the task and in any state (a finished task can
/// still be placed in the iteration it was finished in).
pub fn plan(taskbag: &Taskbag, id: &str, iteration: Option<&str>) -> Result<Outcome, Problem> {
    let mut task = find_task(taskbag, id)?;
    let iteration = non_empty(iteration);
    if let Some(it) = &iteration {
        if !taskbag.iterations.iter().any(|i| &i.id == it) {
            return Err(Problem::UnknownIteration {
                iteration: it.clone(),
            });
        }
    }
    task.iteration = iteration;
    Ok(Outcome::plain(task))
}

/// A new task in `backlog`. Its ID must be valid and unused, its area must exist, and it
/// needs a title.
pub fn new_task(
    taskbag: &Taskbag,
    id: &str,
    area: &str,
    title: &str,
    description: Option<&str>,
) -> Result<Task, Problem> {
    check_id(id)?;
    if taskbag.tasks.iter().any(|t| t.id == id) {
        return Err(Problem::DuplicateTaskId { id: id.to_string() });
    }
    if !taskbag.areas.iter().any(|a| a.id == area) {
        return Err(Problem::UnknownArea {
            area: area.to_string(),
        });
    }
    let title = non_empty(Some(title)).ok_or(Problem::TaskTitleMissing)?;
    let mut description = description.unwrap_or_default().trim_end().to_string();
    if !description.is_empty() {
        description.push('\n');
    }
    Ok(Task {
        id: id.to_string(),
        title,
        area: area.to_string(),
        state: State::Backlog,
        label: None,
        holder: None,
        waiting_on: None,
        who: None,
        blocked_by: Vec::new(),
        next_dependency: None,
        iteration: None,
        handoff_note: None,
        without_human_check: None,
        human_check_steps: None,
        description,
    })
}

/// `me` takes the task and starts it. Refused without a name, when someone else holds it
/// (unless `take` is set, which takes it over with a warning), or when it is done or dropped.
pub fn claim(
    taskbag: &Taskbag,
    id: &str,
    me: Option<&str>,
    take: bool,
) -> Result<Outcome, Problem> {
    let me = non_empty(me).ok_or(Problem::HolderMissing)?;
    let mut task = find_task(taskbag, id)?;
    check_open(&task, &[State::Done, State::Dropped])?;
    let mut warnings = Vec::new();
    if let Err(held) = check_holder(&task, Some(&me)) {
        if !take {
            return Err(held);
        }
        warnings.push(Problem::TookOver {
            id: task.id.clone(),
            previous: task.holder.clone().unwrap_or_default(),
        });
    }
    task.holder = Some(me);
    task.state = State::InProgress;
    clear_stop(&mut task);
    Ok(Outcome::warned(task, warnings))
}

/// Set the task's label. It must be a step on the project's ladder.
pub fn set_label(
    taskbag: &Taskbag,
    id: &str,
    me: Option<&str>,
    label: &str,
) -> Result<Outcome, Problem> {
    let mut task = find_task(taskbag, id)?;
    check_holder(&task, me)?;
    check_open(&task, &[State::Dropped])?;
    if !taskbag.project.labels.iter().any(|l| l == label) {
        return Err(Problem::LabelNotOnLadder {
            label: label.to_string(),
            ladder: taskbag.project.labels.join(" → "),
        });
    }
    task.label = Some(label.to_string());
    // The top label on a finished task is the human check (Task-A03-18).
    if task.state == State::Done && taskbag.project.labels.last().map(String::as_str) == Some(label)
    {
        clear_human_check(&mut task);
    }
    Ok(Outcome::plain(task))
}

/// Clear what a stopped task waited on or was blocked by: it is moving on.
fn clear_stop(task: &mut Task) {
    task.waiting_on = None;
    task.who = None;
    task.blocked_by.clear();
}

/// Park the task as `waiting`: a person (or an outside event) must act. `on` says what (required),
/// `who` optionally who; without it the owner is meant (decision 0016). Blocking tasks are cleared.
pub fn wait(
    taskbag: &Taskbag,
    id: &str,
    me: Option<&str>,
    on: &str,
    who: Option<&str>,
) -> Result<Outcome, Problem> {
    let mut task = find_task(taskbag, id)?;
    check_holder(&task, me)?;
    check_open(&task, &[State::Done, State::Dropped])?;
    let on = non_empty(Some(on)).ok_or(Problem::WaitingOnMissing)?;
    clear_stop(&mut task);
    task.waiting_on = Some(on);
    task.who = non_empty(who);
    task.state = State::Waiting;
    Ok(Outcome::plain(task))
}

/// One task that blocks another, with its state now (`None`: no such task any more).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Blocker {
    pub id: String,
    pub state: Option<State>,
}

impl Blocker {
    /// `A01-17 (in progress)`; a dropped blocker is flagged, as it will never be done.
    pub fn describe(&self) -> String {
        match self.state {
            Some(State::Dropped) => format!("{} (dropped; still blocked?)", self.id),
            Some(state) => format!("{} ({})", self.id, state.as_str().replace('_', " ")),
            None => format!("{} (no such task)", self.id),
        }
    }
}

/// The tasks blocking `task`, each with its current state, in the order `blocked_by` lists them.
pub fn blockers(taskbag: &Taskbag, task: &Task) -> Vec<Blocker> {
    task.blocked_by
        .iter()
        .map(|id| Blocker {
            id: id.clone(),
            state: taskbag.tasks.iter().find(|t| &t.id == id).map(|t| t.state),
        })
        .collect()
}

/// Whether a `blocked` task's blockers are all done. A dropped blocker never counts as done
/// (owner, 2026-10-14). `alm` only tells; it never moves the task by itself.
pub fn all_blockers_done(taskbag: &Taskbag, task: &Task) -> bool {
    task.state == State::Blocked
        && !task.blocked_by.is_empty()
        && blockers(taskbag, task)
            .iter()
            .all(|b| b.state == Some(State::Done))
}

/// Mark the task `blocked` by other tasks in this taskbag that must finish first (`alm block`,
/// decision 0016). `by` needs at least one ID; each must be a task here and not the task itself.
/// A dropped blocker is allowed but warned about: it will never be done. What it waited on is
/// cleared; a person part belongs in the task's next dependency.
pub fn block(
    taskbag: &Taskbag,
    id: &str,
    me: Option<&str>,
    by: &[String],
) -> Result<Outcome, Problem> {
    let mut task = find_task(taskbag, id)?;
    check_holder(&task, me)?;
    check_open(&task, &[State::Done, State::Dropped])?;
    let mut ids: Vec<String> = Vec::new();
    for b in by.iter().flat_map(|b| b.split(',')).map(str::trim) {
        if !b.is_empty() && !ids.iter().any(|x| x == b) {
            ids.push(b.to_string());
        }
    }
    if ids.is_empty() {
        return Err(Problem::BlockedByMissing);
    }
    let mut warnings = Vec::new();
    for b in &ids {
        if b == &task.id {
            return Err(Problem::BlockedBySelf {
                id: task.id.clone(),
            });
        }
        match taskbag.tasks.iter().find(|t| &t.id == b) {
            None => return Err(Problem::BlockerUnknown { id: b.clone() }),
            Some(t) if t.state == State::Dropped => warnings.push(Problem::BlockerDropped {
                id: task.id.clone(),
                blocker: b.clone(),
            }),
            Some(_) => {}
        }
    }
    clear_stop(&mut task);
    task.blocked_by = ids;
    task.state = State::Blocked;
    Ok(Outcome::warned(task, warnings))
}

/// Move a `blocked` task on (`alm unblock`): back to `in_progress` when someone holds it, else
/// `ready`. Its blocking tasks are cleared. `alm` does not check that they are done: the agent
/// does (owner, 2026-10-14).
pub fn unblock(taskbag: &Taskbag, id: &str, me: Option<&str>) -> Result<Outcome, Problem> {
    let mut task = find_task(taskbag, id)?;
    check_holder(&task, me)?;
    if task.state != State::Blocked {
        return Err(Problem::NotBlocked {
            id: task.id.clone(),
            state: task.state.as_str().to_string(),
        });
    }
    clear_stop(&mut task);
    task.state = if task.holder.is_some() {
        State::InProgress
    } else {
        State::Ready
    };
    Ok(Outcome::plain(task))
}

/// Finish the task. `alm` does not judge whether it is really finished: the agent is given the
/// rule before it finishes (Task-A03-15). `without_human_check` records why a task is finished
/// without the human check (an automatic or loop run); empty text is refused. `steps` are how a
/// person checks it (Markdown); given without a note, or empty, they are refused. A note without
/// steps is accepted with the warning `human_check_steps_missing`, which says how to write them
/// (Task-A03-18: `alm` asks, it does not refuse). What is not given stays as it was.
///
/// On a task that is already done, only its human check is recorded, so the holder rule does not
/// apply: a finished task's holder is history (Task-A03-18).
pub fn finish(
    taskbag: &Taskbag,
    id: &str,
    me: Option<&str>,
    without_human_check: Option<&str>,
    steps: Option<&str>,
) -> Result<Outcome, Problem> {
    let mut task = find_task(taskbag, id)?;
    if task.state != State::Done {
        check_holder(&task, me)?;
    }
    check_open(&task, &[State::Dropped])?;
    let mut warnings = Vec::new();
    if let Some(text) = without_human_check {
        let note = non_empty(Some(text)).ok_or(Problem::WithoutHumanCheckMissing)?;
        if let Some(steps) = steps {
            let steps = steps.trim();
            if steps.is_empty() {
                return Err(Problem::HumanCheckStepsEmpty);
            }
            task.human_check_steps = Some(format!("{steps}\n"));
        } else if task.human_check_steps.is_none() {
            warnings.push(Problem::HumanCheckStepsMissing {
                id: task.id.clone(),
            });
        }
        task.without_human_check = Some(note);
    } else if steps.is_some() {
        return Err(Problem::WithoutHumanCheckMissing);
    }
    task.state = State::Done;
    clear_stop(&mut task);
    Ok(Outcome::warned(task, warnings))
}

/// Clear what waits for the human check: the note and its steps. The task's history keeps them.
fn clear_human_check(task: &mut Task) {
    task.without_human_check = None;
    task.human_check_steps = None;
}

/// A person has checked a finished task (Task-A03-18): it gets the top label of the ladder and
/// leaves "To check"; the note and the steps are cleared, and stay in its history. Only a done
/// task can be checked. The holder rule does not apply: the person checking is rarely the agent
/// session that held it.
pub fn checked(taskbag: &Taskbag, id: &str) -> Result<Outcome, Problem> {
    let mut task = find_task(taskbag, id)?;
    if task.state != State::Done {
        return Err(Problem::NotDone {
            id: task.id.clone(),
            state: task.state.as_str().to_string(),
        });
    }
    if let Some(top) = taskbag.project.labels.last() {
        task.label = Some(top.clone());
    }
    clear_human_check(&mut task);
    Ok(Outcome::plain(task))
}

/// Send a finished task back to "To check" (Task-A03-37): its work changed after a person checked
/// it, so the earlier check no longer covers it. The AI agent decides when (approver,
/// 2026-10-20); `alm` does not look at the work. `reason` says what changed and is shown where the
/// task waits for its check; `steps` replace the steps for checking it (without them, the earlier
/// steps stay, or the warning `human_check_steps_missing` asks for them). A task at the top label
/// goes one step down, since the person's check no longer covers it. Only a done task can be sent
/// back; the holder rule does not apply, as for `checked`.
pub fn recheck(
    taskbag: &Taskbag,
    id: &str,
    reason: &str,
    steps: Option<&str>,
) -> Result<Outcome, Problem> {
    let mut task = find_task(taskbag, id)?;
    if task.state != State::Done {
        return Err(Problem::NotDone {
            id: task.id.clone(),
            state: task.state.as_str().to_string(),
        });
    }
    let note = non_empty(Some(reason)).ok_or(Problem::WithoutHumanCheckMissing)?;
    let mut warnings = Vec::new();
    match steps.map(str::trim) {
        Some("") => return Err(Problem::HumanCheckStepsEmpty),
        Some(steps) => task.human_check_steps = Some(format!("{steps}\n")),
        None if task.human_check_steps.is_none() => {
            warnings.push(Problem::HumanCheckStepsMissing {
                id: task.id.clone(),
            })
        }
        None => {}
    }
    let ladder = &taskbag.project.labels;
    if ladder.len() > 1 && task.label.as_ref() == ladder.last() {
        task.label = Some(ladder[ladder.len() - 2].clone());
    }
    task.without_human_check = Some(note);
    Ok(Outcome::warned(task, warnings))
}

/// Let go of the task. A task `in_progress` goes back to `ready`. The handoff note is
/// replaced by `note`, or removed when there is none, so it always describes the latest
/// release.
pub fn release(
    taskbag: &Taskbag,
    id: &str,
    me: Option<&str>,
    note: Option<&str>,
) -> Result<Outcome, Problem> {
    let mut task = find_task(taskbag, id)?;
    check_holder(&task, me)?;
    task.holder = None;
    if task.state == State::InProgress {
        task.state = State::Ready;
    }
    task.handoff_note = non_empty(note);
    Ok(Outcome::plain(task))
}

/// Set the task's next dependency: the one thing needed for its next label. `None` (or empty
/// text) clears it.
pub fn set_next(
    taskbag: &Taskbag,
    id: &str,
    me: Option<&str>,
    text: Option<&str>,
) -> Result<Outcome, Problem> {
    let mut task = find_task(taskbag, id)?;
    check_holder(&task, me)?;
    check_open(&task, &[State::Dropped])?;
    task.next_dependency = non_empty(text);
    Ok(Outcome::plain(task))
}

/// Give a task a new title (`alm title`). Same holder rule as other changes; an empty title is
/// refused. Allowed in any state, so an old name can be corrected on a finished task too.
pub fn set_title(
    taskbag: &Taskbag,
    id: &str,
    me: Option<&str>,
    title: &str,
) -> Result<Outcome, Problem> {
    let mut task = find_task(taskbag, id)?;
    check_holder(&task, me)?;
    task.title = non_empty(Some(title)).ok_or(Problem::TaskTitleMissing)?;
    Ok(Outcome::plain(task))
}

/// Set or clear the task's handoff note (the app's task pane, Task-A01-18). Same holder rule as
/// other changes; allowed in any state. `release --note` sets it too, together with letting go.
pub fn set_handoff_note(
    taskbag: &Taskbag,
    id: &str,
    me: Option<&str>,
    note: Option<&str>,
) -> Result<Outcome, Problem> {
    let mut task = find_task(taskbag, id)?;
    check_holder(&task, me)?;
    task.handoff_note = non_empty(note);
    Ok(Outcome::plain(task))
}

/// The version of a task as it is now (Task-A01-57): a short fingerprint of everything saved in
/// its `task.json` and `description.md`. It changes with every change to the task, so a writer
/// that read version `v` can tell whether the task changed since.
pub fn task_version(task: &crate::model::Task) -> String {
    // FNV-1a, 64 bits: the same on every computer and Rust version.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let json = serde_json::to_string(task).unwrap_or_default();
    for b in json
        .bytes()
        .chain(std::iter::once(0))
        .chain(task.description.bytes())
    {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// Refuse a change made from an out-of-date copy (Task-A01-57): when the writer says which
/// version it read (`expected`) and the task has changed since, nothing is written. Without a
/// version, nothing is checked, so existing agent prompts keep working.
pub fn check_version(taskbag: &Taskbag, id: &str, expected: Option<&str>) -> Result<(), Problem> {
    let Some(expected) = expected.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(());
    };
    let task = find_task(taskbag, id)?;
    let version = task_version(&task);
    if version == expected {
        Ok(())
    } else {
        Err(Problem::TaskChanged {
            id: task.id,
            version,
        })
    }
}

/// Give the task a new description, the text of its `description.md` (the app's task pane,
/// Task-A01-18). Same holder rule as other changes; allowed in any state. Empty text removes it.
pub fn set_description(
    taskbag: &Taskbag,
    id: &str,
    me: Option<&str>,
    text: &str,
) -> Result<Outcome, Problem> {
    let mut task = find_task(taskbag, id)?;
    check_holder(&task, me)?;
    let mut description = text.trim_end().to_string();
    if !description.is_empty() {
        description.push('\n');
    }
    task.description = description;
    Ok(Outcome::plain(task))
}

/// One change to one field of a task, as the app's task pane asks for it (Task-A01-18). Each
/// goes through the same rule as its `alm` command; [`edit`] says which.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "field", rename_all = "snake_case", deny_unknown_fields)]
pub enum Edit {
    /// `ready`, `waiting` (with `waiting_on`, optionally `who`), `blocked` (with `blocked_by`),
    /// `done` or `dropped`. `in_progress` comes only from claiming the task (or unblocking a held
    /// one), and nothing moves a task back to `backlog`.
    State {
        state: State,
        #[serde(default)]
        waiting_on: Option<String>,
        #[serde(default)]
        who: Option<String>,
        #[serde(default)]
        blocked_by: Vec<String>,
    },
    Label {
        label: String,
    },
    /// What the task waits on; like `alm wait`, this also makes it `waiting` (its `who` stays).
    WaitingOn {
        text: String,
    },
    /// The tasks that block it; like `alm block`, this also makes it `blocked`.
    BlockedBy {
        ids: Vec<String>,
    },
    NextDependency {
        #[serde(default)]
        text: Option<String>,
    },
    HandoffNote {
        #[serde(default)]
        text: Option<String>,
    },
    Iteration {
        #[serde(default)]
        iteration: Option<String>,
    },
    /// `expected_version`: the version the app showed (Task-A01-57); a newer task refuses it.
    Title {
        title: String,
        #[serde(default)]
        expected_version: Option<String>,
    },
    Description {
        text: String,
        #[serde(default)]
        expected_version: Option<String>,
    },
    /// A person checked the finished task: like `alm checked` (Task-A03-18).
    Checked,
    /// Reopen a done or dropped task, saying why: like `alm reopen` (Task-A01-50).
    Reopen {
        note: String,
    },
}

/// Apply one [`Edit`] by `me`, through the rule `alm` uses for it. Returns the changed task and
/// the name of that rule's command (`ready`, `wait`, `block`, `done`, `drop`, `label`, `needs`, `plan`,
/// `title`, `checked`; `note` and `describe` for the two changes `alm` has no command of their own for), so
/// the change is recorded the same way.
pub fn edit(
    taskbag: &Taskbag,
    id: &str,
    me: Option<&str>,
    edit: &Edit,
) -> Result<(Outcome, &'static str), Problem> {
    match edit {
        Edit::State {
            state,
            waiting_on,
            who,
            blocked_by,
        } => match state {
            State::Ready => make_ready(taskbag, id, me).map(|o| (o, "ready")),
            State::Waiting => wait(
                taskbag,
                id,
                me,
                waiting_on.as_deref().unwrap_or_default(),
                who.as_deref(),
            )
            .map(|o| (o, "wait")),
            State::Blocked => block(taskbag, id, me, blocked_by).map(|o| (o, "block")),
            State::Done => finish(taskbag, id, me, None, None).map(|o| (o, "done")),
            State::Dropped => drop_task(taskbag, id, me).map(|o| (o, "drop")),
            State::Backlog | State::InProgress => Err(Problem::CannotSetState {
                id: id.to_string(),
                state: state.as_str().to_string(),
            }),
        },
        Edit::Label { label } => set_label(taskbag, id, me, label).map(|o| (o, "label")),
        Edit::WaitingOn { text } => {
            let who = find_task(taskbag, id)?.who;
            wait(taskbag, id, me, text, who.as_deref()).map(|o| (o, "wait"))
        }
        Edit::BlockedBy { ids } => block(taskbag, id, me, ids).map(|o| (o, "block")),
        Edit::NextDependency { text } => {
            set_next(taskbag, id, me, text.as_deref()).map(|o| (o, "needs"))
        }
        Edit::HandoffNote { text } => {
            set_handoff_note(taskbag, id, me, text.as_deref()).map(|o| (o, "note"))
        }
        Edit::Iteration { iteration } => {
            plan(taskbag, id, iteration.as_deref()).map(|o| (o, "plan"))
        }
        Edit::Title {
            title,
            expected_version,
        } => {
            check_version(taskbag, id, expected_version.as_deref())?;
            set_title(taskbag, id, me, title).map(|o| (o, "title"))
        }
        Edit::Description {
            text,
            expected_version,
        } => {
            check_version(taskbag, id, expected_version.as_deref())?;
            set_description(taskbag, id, me, text).map(|o| (o, "describe"))
        }
        Edit::Checked => checked(taskbag, id).map(|o| (o, "checked")),
        Edit::Reopen { note } => reopen(taskbag, id, note).map(|o| (o, "reopen")),
    }
}

/// The project with a new name and/or ID (`alm project`). An empty name or an invalid ID is
/// refused; what is not given stays as it is.
pub fn rename_project(
    taskbag: &Taskbag,
    name: Option<&str>,
    id: Option<&str>,
) -> Result<Project, Problem> {
    let mut project = taskbag.project.clone();
    if let Some(name) = name {
        project.name = non_empty(Some(name)).ok_or(Problem::ProjectNameMissing)?;
    }
    if let Some(id) = id {
        check_id(id)?;
        project.id = id.to_string();
    }
    Ok(project)
}

/// Mark an unheld `backlog`, `waiting` or `blocked` task `ready` (for example, the owner approved
/// it). What it waited on or was blocked by is cleared.
pub fn make_ready(taskbag: &Taskbag, id: &str, me: Option<&str>) -> Result<Outcome, Problem> {
    let mut task = find_task(taskbag, id)?;
    check_holder(&task, me)?;
    if task.holder.is_some()
        || !matches!(task.state, State::Backlog | State::Waiting | State::Blocked)
    {
        return Err(Problem::CannotMakeReady {
            id: task.id.clone(),
            state: task.state.as_str().to_string(),
        });
    }
    task.state = State::Ready;
    clear_stop(&mut task);
    Ok(Outcome::plain(task))
}

/// Drop the task: it will not be done. Its holder, *waiting on* and blockers are cleared.
pub fn drop_task(taskbag: &Taskbag, id: &str, me: Option<&str>) -> Result<Outcome, Problem> {
    let mut task = find_task(taskbag, id)?;
    check_holder(&task, me)?;
    check_open(&task, &[State::Done, State::Dropped])?;
    task.state = State::Dropped;
    task.holder = None;
    clear_stop(&mut task);
    Ok(Outcome::plain(task))
}

/// The history-only field that says why a task was reopened (Task-A01-50): its `to` is the note.
pub const REOPEN_NOTE: &str = "reopen_note";

/// Reopen a finished task (`alm reopen`, the app's **Reopen…**, Task-A01-50): a `done` or
/// `dropped` task becomes `ready`, held by nobody, so more work on the same outcome stays on the
/// same task. `note` says why (required); it is recorded in the task's history only
/// ([`REOPEN_NOTE`]), not in its files. The new work has proven nothing yet, so the label is
/// cleared, and so is the human-check record (the note and its steps); both stay in the history.
/// The description, handoff note and next dependency are kept. Any other state is refused
/// (`cannot_reopen`). The holder rule does not apply: a finished task is held by nobody (A00-30).
pub fn reopen(taskbag: &Taskbag, id: &str, note: &str) -> Result<Outcome, Problem> {
    let mut task = find_task(taskbag, id)?;
    if !matches!(task.state, State::Done | State::Dropped) {
        return Err(Problem::CannotReopen {
            id: task.id.clone(),
            state: task.state.as_str().to_string(),
        });
    }
    let note = non_empty(Some(note)).ok_or(Problem::ReopenNoteMissing)?;
    task.state = State::Ready;
    task.holder = None;
    task.label = None;
    clear_stop(&mut task);
    clear_human_check(&mut task);
    let mut outcome = Outcome::plain(task);
    outcome.recorded.push(Change {
        field: REOPEN_NOTE.to_string(),
        from: None,
        to: Some(serde_json::Value::String(note)),
    });
    Ok(outcome)
}

/// Every task that is `ready` and held by no one, for the agent to choose from (Task-A03-14). A
/// `blocked` or `waiting` task is never among them: skipped by its state, not by its words:
/// earliest iteration first (unplanned tasks last), then area order, then ID. With an
/// iteration, only the tasks planned in it; an iteration that does not exist is refused.
pub fn next<'a>(taskbag: &'a Taskbag, iteration: Option<&str>) -> Result<Vec<&'a Task>, Problem> {
    if let Some(i) = iteration {
        if !taskbag.iterations.iter().any(|x| x.id == i) {
            return Err(Problem::UnknownIteration {
                iteration: i.to_string(),
            });
        }
    }
    let area_rank = |area: &str| {
        taskbag
            .areas
            .iter()
            .position(|a| a.id == area)
            .unwrap_or(usize::MAX)
    };
    let iteration_rank = |t: &Task| {
        t.iteration
            .as_ref()
            .and_then(|i| taskbag.iterations.iter().position(|x| &x.id == i))
            .unwrap_or(usize::MAX)
    };
    let mut tasks: Vec<&Task> = taskbag
        .tasks
        .iter()
        .filter(|t| t.state == State::Ready && t.holder.is_none())
        .filter(|t| iteration.is_none_or(|i| t.iteration.as_deref() == Some(i)))
        .collect();
    tasks.sort_by(|a, b| {
        iteration_rank(a)
            .cmp(&iteration_rank(b))
            .then(area_rank(&a.area).cmp(&area_rank(&b.area)))
            .then(a.id.cmp(&b.id))
    });
    Ok(tasks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::default_labels;

    fn bag() -> Taskbag {
        let mut s = Taskbag {
            project: Project {
                format: crate::FORMAT.into(),
                id: "p".into(),
                name: "P".into(),
                labels: default_labels(),
                project_folder: None,
                task_language: None,
                handoff: None,
            },
            areas: vec![],
            iterations: vec![],
            tasks: vec![],
        };
        s.areas.push(new_area(&s, "A02", "Second").unwrap());
        s.areas.push(new_area(&s, "A01", "First").unwrap());
        // Areas are kept sorted by order, as `load` does: A02 was added first.
        for (id, area) in [("A01-02", "A01"), ("A01-01", "A01"), ("A02-01", "A02")] {
            let t = new_task(&s, id, area, "Do it", Some("Why.")).unwrap();
            s.tasks.push(t);
        }
        s
    }

    /// Store a changed task back into the taskbag, as writing and reloading would.
    macro_rules! put {
        ($s:ident, $task:expr $(,)?) => {{
            let t: Task = $task;
            let i = $s.tasks.iter().position(|x| x.id == t.id).unwrap();
            $s.tasks[i] = t;
        }};
    }

    /// Decision 0016: `alm block` refuses an empty list, the task itself and an unknown ID; a
    /// dropped blocker is allowed but warned about; unblocking moves it on by its holder.
    #[test]
    fn block_and_unblock_rules() {
        let mut s = bag();
        let by = |ids: &[&str]| ids.iter().map(|i| i.to_string()).collect::<Vec<_>>();
        assert_eq!(
            block(&s, "A01-01", None, &by(&[])),
            Err(Problem::BlockedByMissing)
        );
        assert_eq!(
            block(&s, "A01-01", None, &by(&[" ", ""])),
            Err(Problem::BlockedByMissing)
        );
        assert_eq!(
            block(&s, "A01-01", None, &by(&["A01-01"])),
            Err(Problem::BlockedBySelf {
                id: "A01-01".into()
            })
        );
        assert_eq!(
            block(&s, "A01-01", None, &by(&["A09-01"])),
            Err(Problem::BlockerUnknown {
                id: "A09-01".into()
            })
        );
        // Waiting, then blocked: what it waited on is cleared; IDs are de-duplicated, commas split.
        put!(
            s,
            wait(&s, "A01-01", None, "approval", Some("Ann"))
                .unwrap()
                .task
        );
        let o = block(&s, "A01-01", None, &by(&["A01-02,A02-01", "A01-02"])).unwrap();
        assert!(o.warnings.is_empty());
        assert_eq!(o.task.state, State::Blocked);
        assert_eq!(o.task.blocked_by, by(&["A01-02", "A02-01"]));
        assert_eq!(
            (o.task.waiting_on.clone(), o.task.who.clone()),
            (None, None)
        );
        put!(s, o.task);
        // Blockers with their states; not all done.
        let t = find_task(&s, "A01-01").unwrap();
        assert_eq!(
            blockers(&s, &t)
                .iter()
                .map(Blocker::describe)
                .collect::<Vec<_>>(),
            vec!["A01-02 (backlog)", "A02-01 (backlog)"]
        );
        assert!(!all_blockers_done(&s, &t));
        // One done, one dropped: a dropped blocker never counts as done, and is flagged.
        put!(s, finish(&s, "A01-02", None, None, None).unwrap().task);
        put!(s, drop_task(&s, "A02-01", None).unwrap().task);
        let t = find_task(&s, "A01-01").unwrap();
        assert!(!all_blockers_done(&s, &t));
        assert_eq!(
            blockers(&s, &t)[1].describe(),
            "A02-01 (dropped; still blocked?)"
        );
        // Blocking on a dropped task is allowed, with a warning.
        let o = block(&s, "A01-01", None, &by(&["A02-01"])).unwrap();
        assert_eq!(
            o.warnings,
            vec![Problem::BlockerDropped {
                id: "A01-01".into(),
                blocker: "A02-01".into()
            }]
        );
        // All done: alm only tells; the task stays blocked until someone unblocks it.
        put!(s, block(&s, "A01-01", None, &by(&["A01-02"])).unwrap().task);
        let t = find_task(&s, "A01-01").unwrap();
        assert!(all_blockers_done(&s, &t));
        assert_eq!(t.state, State::Blocked);
        // Unblock: ready when unheld, in progress when held; only a blocked task.
        let u = unblock(&s, "A01-01", None).unwrap().task;
        assert_eq!((u.state, u.blocked_by.len()), (State::Ready, 0));
        assert_eq!(
            unblock(&s, "A01-02", None),
            Err(Problem::NotBlocked {
                id: "A01-02".into(),
                state: "done".into()
            })
        );
        let mut held = s.clone();
        put!(held, claim(&held, "A01-01", Some("a"), true).unwrap().task);
        put!(
            held,
            block(&held, "A01-01", Some("a"), &by(&["A01-02"]))
                .unwrap()
                .task
        );
        assert_eq!(
            unblock(&held, "A01-01", Some("a")).unwrap().task.state,
            State::InProgress
        );
        // `alm ready` also moves an unheld blocked task on; a blocked task is never next.
        assert_eq!(
            make_ready(&s, "A01-01", None).unwrap().task.state,
            State::Ready
        );
        assert!(next(&s, None).unwrap().iter().all(|t| t.id != "A01-01"));
        // Done or dropped tasks cannot be blocked.
        assert_eq!(
            block(&s, "A01-02", None, &by(&["A01-01"]))
                .unwrap_err()
                .code(),
            "task_closed"
        );
    }

    /// Task-A01-50: reopening a done or dropped task.
    #[test]
    fn reopen_rules() {
        let mut s = bag();
        assert_eq!(
            reopen(&s, "A01-01", "why"),
            Err(Problem::CannotReopen {
                id: "A01-01".into(),
                state: "backlog".into()
            })
        );
        put!(s, claim(&s, "A01-01", Some("a"), false).unwrap().task);
        put!(
            s,
            set_label(&s, "A01-01", Some("a"), "implemented")
                .unwrap()
                .task
        );
        put!(
            s,
            set_next(&s, "A01-01", Some("a"), Some("a look"))
                .unwrap()
                .task
        );
        put!(
            s,
            finish(&s, "A01-01", Some("a"), Some("loop"), Some("steps"))
                .unwrap()
                .task
        );
        assert_eq!(reopen(&s, "A01-01", "  "), Err(Problem::ReopenNoteMissing));
        let o = reopen(&s, "A01-01", " failed its check ").unwrap();
        let t = &o.task;
        assert_eq!((t.state, &t.holder, &t.label), (State::Ready, &None, &None));
        assert_eq!(
            (&t.without_human_check, &t.human_check_steps),
            (&None, &None)
        );
        assert_eq!(t.next_dependency.as_deref(), Some("a look"));
        assert_eq!(t.description, find_task(&s, "A01-01").unwrap().description);
        assert_eq!(
            o.recorded,
            vec![Change {
                field: REOPEN_NOTE.into(),
                from: None,
                to: Some("failed its check".into()),
            }]
        );
        // The app's Reopen… goes through the same rule.
        let (e, command) = edit(
            &s,
            "A01-01",
            Some("Will"),
            &Edit::Reopen {
                note: "failed its check".into(),
            },
        )
        .unwrap();
        assert_eq!((e, command), (o, "reopen"));
        // Dropped tasks too; a held task is not reopened.
        put!(s, drop_task(&s, "A02-01", None).unwrap().task);
        assert_eq!(
            reopen(&s, "A02-01", "wanted").unwrap().task.state,
            State::Ready
        );
        put!(s, claim(&s, "A01-02", Some("a"), false).unwrap().task);
        assert_eq!(
            reopen(&s, "A01-02", "x").unwrap_err().code(),
            "cannot_reopen"
        );
    }

    #[test]
    fn renaming_a_label_renames_it_on_the_tasks_too() {
        let mut s = bag();
        s.tasks[0].label = Some("human verified".into());
        s.tasks[1].label = Some("human verified".into());
        s.tasks[1].state = State::Done;
        s.tasks[2].label = Some("implemented".into());
        let (p, tasks) = rename_label(&s, "human verified", " human checked ").unwrap();
        assert_eq!(p.labels[3], "human checked");
        assert_eq!(p.labels.len(), 4);
        assert_eq!(tasks.len(), 2, "the done task too");
        assert!(
            tasks
                .iter()
                .all(|t| t.label.as_deref() == Some("human checked"))
        );
        assert_eq!(
            rename_label(&s, "shipped", "x").unwrap_err().code(),
            "label_not_on_ladder"
        );
        assert_eq!(
            rename_label(&s, "human verified", "implemented"),
            Err(Problem::LabelExists {
                label: "implemented".into()
            })
        );
        assert_eq!(
            rename_label(&s, "human verified", " "),
            Err(Problem::LabelNameMissing)
        );
    }

    #[test]
    fn a_new_task_id_is_a_word_and_a_number_in_its_area() {
        let s = bag();
        // bag() has A01-01, A01-02 and A02-01: numbered IDs stay; new ones are words (A01-54).
        for area in ["A01", "A02"] {
            let id = next_task_id(&s, area).unwrap();
            assert!(id.starts_with(&format!("{area}-")), "{id}");
            assert!(!crate::ids::numbered(&id), "{id}");
            assert!(find_task(&s, &id).is_err(), "{id}");
            // It can be added as any ID can.
            assert!(new_task(&s, &id, area, "t", None).is_ok());
        }
        assert_eq!(
            next_task_id(&s, "A09"),
            Err(Problem::UnknownArea { area: "A09".into() })
        );
    }

    /// `alm add area` / `alm add iteration` without an ID, and the app's suggestions (Task-A01-19).
    #[test]
    fn the_next_area_and_iteration_ids_follow_the_pattern() {
        let mut s = bag();
        s.areas.clear();
        assert_eq!(next_area_id(&s), "A01");
        assert_eq!(next_iteration_id(&s), "I01");
        for id in ["A00", "A01", "A07"] {
            s.areas.push(new_area(&s, id, "x").unwrap());
        }
        assert_eq!(next_area_id(&s), "A08");
        for n in 1..=15 {
            let id = format!("I{n:02}");
            s.iterations
                .push(new_iteration(&s, &id, "x", IterationDetails::default()).unwrap());
        }
        assert_eq!(next_iteration_id(&s), "I16");
        // Other shapes do not count; the last area's letters are followed; width is kept.
        s.areas.push(new_area(&s, "misc", "x").unwrap());
        assert_eq!(next_area_id(&s), "A08");
        s.areas.push(new_area(&s, "T009", "x").unwrap());
        assert_eq!(next_area_id(&s), "T010");
        s.areas.push(new_area(&s, "T0010", "x").unwrap());
        assert_eq!(next_area_id(&s), "T0011");
    }

    /// Two writers read the same task; the first saves; the second, still holding the old
    /// version, is refused and nothing it sent is kept (Task-A01-57).
    #[test]
    fn a_save_from_an_out_of_date_copy_is_refused() {
        let mut s = bag();
        let id = s.tasks[0].id.clone();
        let read = task_version(&s.tasks[0]);
        // The same task gives the same version; the version is short and stable.
        assert_eq!(read, task_version(&s.tasks[0].clone()));
        assert_eq!(read.len(), 16);
        // The first writer (an agent's `alm describe`) saves.
        let first = edit(
            &s,
            &id,
            None,
            &Edit::Description {
                text: "The agent's text".into(),
                expected_version: Some(read.clone()),
            },
        )
        .unwrap()
        .0;
        s.tasks[0] = first.task.clone();
        let now = task_version(&s.tasks[0]);
        assert_ne!(now, read);
        // The second writer (the person in the app) still holds the old version: refused.
        let stale = Edit::Description {
            text: "The person's text".into(),
            expected_version: Some(read.clone()),
        };
        assert_eq!(
            edit(&s, &id, None, &stale),
            Err(Problem::TaskChanged {
                id: id.clone(),
                version: now.clone()
            })
        );
        let stale_title = Edit::Title {
            title: "A new title".into(),
            expected_version: Some(read),
        };
        assert!(matches!(
            edit(&s, &id, None, &stale_title),
            Err(Problem::TaskChanged { .. })
        ));
        // With the current version, or with none (existing prompts), the save goes through.
        let fresh = Edit::Description {
            text: "The person's text".into(),
            expected_version: Some(now),
        };
        assert_eq!(
            edit(&s, &id, None, &fresh).unwrap().0.task.description,
            "The person's text\n"
        );
        assert!(check_version(&s, &id, None).is_ok());
        assert!(check_version(&s, &id, Some("  ")).is_ok());
    }

    #[test]
    fn the_task_language_is_ui_or_a_language_code() {
        let s = bag();
        for ok in ["ui", "en", "zh-CN", "ja", "pt-BR", "yue", " fr "] {
            let p = set_task_language(&s, ok).unwrap();
            assert_eq!(p.task_language.as_deref(), Some(ok.trim()));
        }
        for bad in ["", "chinese!", "z", "zh_CN", "12", "en-"] {
            assert_eq!(
                set_task_language(&s, bad),
                Err(Problem::InvalidLanguage {
                    value: bad.trim().into()
                }),
                "{bad}"
            );
        }
    }

    #[test]
    fn titles_and_the_project_name_can_change() {
        let mut s = bag();
        let t = set_title(&s, "A01-01", None, "  A clearer outcome ").unwrap();
        assert_eq!(t.task.title, "A clearer outcome");
        assert_eq!(
            set_title(&s, "A01-01", None, " "),
            Err(Problem::TaskTitleMissing)
        );
        s.tasks[1].holder = Some("other".into());
        let held = s.tasks[1].id.clone();
        assert!(matches!(
            set_title(&s, &held, Some("me"), "x"),
            Err(Problem::TaskHeld { .. })
        ));
        let p = rename_project(&s, Some("Allomos"), Some("allomos")).unwrap();
        assert_eq!((p.name.as_str(), p.id.as_str()), ("Allomos", "allomos"));
        assert_eq!(
            rename_project(&s, Some(" "), None),
            Err(Problem::ProjectNameMissing)
        );
        assert!(matches!(
            rename_project(&s, None, Some("bad id")),
            Err(Problem::InvalidId { .. })
        ));
        assert_eq!(rename_project(&s, None, None).unwrap(), s.project);
    }

    #[test]
    fn the_ladder_changes_only_when_no_task_is_stranded() {
        let mut s = bag();
        let ladder = |l: &[&str]| l.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        let p = set_ladder(&s, &ladder(&["implemented", " connected ", ""])).unwrap();
        assert_eq!(p.labels, ["implemented", "connected"]);
        assert_eq!((p.id.as_str(), p.name.as_str()), ("p", "P"));
        assert_eq!(set_ladder(&s, &ladder(&[" "])), Err(Problem::LadderEmpty));
        s.tasks[0].label = Some("human verified".into());
        assert_eq!(
            set_ladder(&s, &ladder(&["implemented"])),
            Err(Problem::LabelNotOnLadder {
                label: "human verified".into(),
                ladder: "implemented".into()
            })
        );
    }

    #[test]
    fn new_items_follow_the_rules() {
        let s = bag();
        assert_eq!(s.areas[1].order, 2);
        assert_eq!(s.tasks[0].state, State::Backlog);
        assert_eq!(s.tasks[0].description, "Why.\n");
        assert_eq!(
            new_area(&s, "A01", "x"),
            Err(Problem::DuplicateAreaId { id: "A01".into() })
        );
        assert_eq!(new_area(&s, "A03", " "), Err(Problem::AreaNameMissing));
        assert_eq!(
            new_area(&s, "bad id", "x"),
            Err(Problem::InvalidId {
                id: "bad id".into()
            })
        );
        assert_eq!(
            new_task(&s, "A01-01", "A01", "t", None),
            Err(Problem::DuplicateTaskId {
                id: "A01-01".into()
            })
        );
        assert_eq!(
            new_task(&s, "X-1", "A09", "t", None),
            Err(Problem::UnknownArea { area: "A09".into() })
        );
        assert_eq!(
            new_task(&s, "X-1", "A01", "", None),
            Err(Problem::TaskTitleMissing)
        );
        assert_eq!(
            new_task(&s, "X-1", "A01", "t", None).unwrap().description,
            ""
        );
    }

    #[test]
    fn claim_rules() {
        let mut s = bag();
        assert_eq!(
            claim(&s, "A01-01", None, false),
            Err(Problem::HolderMissing)
        );
        assert_eq!(
            claim(&s, "nope", Some("a"), false),
            Err(Problem::TaskNotFound { id: "nope".into() })
        );
        let o = claim(&s, "A01-01", Some("a"), false).unwrap();
        assert_eq!(o.task.state, State::InProgress);
        assert_eq!(o.task.holder.as_deref(), Some("a"));
        put!(s, o.task);
        assert_eq!(
            claim(&s, "A01-01", Some("b"), false),
            Err(Problem::TaskHeld {
                id: "A01-01".into(),
                holder: "a".into()
            })
        );
        // Claiming again as the same holder is fine.
        assert!(claim(&s, "A01-01", Some("a"), false).is_ok());
        let took = claim(&s, "A01-01", Some("b"), true).unwrap();
        assert_eq!(took.task.holder.as_deref(), Some("b"));
        assert_eq!(
            took.warnings,
            vec![Problem::TookOver {
                id: "A01-01".into(),
                previous: "a".into()
            }]
        );
        let done = finish(&s, "A01-01", Some("a"), None, None).unwrap().task;
        put!(s, done);
        assert_eq!(
            claim(&s, "A01-01", Some("a"), false),
            Err(Problem::TaskClosed {
                id: "A01-01".into(),
                state: "done".into()
            })
        );
    }

    #[test]
    fn needs_ready_and_drop() {
        let mut s = bag();
        // Next dependency: set by anyone on an unheld task, cleared by empty text.
        let t = set_next(&s, "A01-01", None, Some(" owner review "))
            .unwrap()
            .task;
        assert_eq!(t.next_dependency.as_deref(), Some("owner review"));
        put!(s, t);
        assert_eq!(
            set_next(&s, "A01-01", None, Some(""))
                .unwrap()
                .task
                .next_dependency,
            None
        );
        assert_eq!(
            set_next(&s, "A01-01", None, None)
                .unwrap()
                .task
                .next_dependency,
            None
        );

        // Ready: from backlog, and from waiting (clearing waiting on).
        let r = make_ready(&s, "A01-01", None).unwrap().task;
        assert_eq!(r.state, State::Ready);
        put!(s, r);
        assert_eq!(
            make_ready(&s, "A01-01", None),
            Err(Problem::CannotMakeReady {
                id: "A01-01".into(),
                state: "ready".into()
            })
        );
        put!(s, claim(&s, "A01-02", Some("a"), false).unwrap().task);
        put!(
            s,
            wait(&s, "A01-02", Some("a"), "owner", None).unwrap().task
        );
        // Held by someone else: refused as held; held by me: refused, release first.
        assert!(matches!(
            make_ready(&s, "A01-02", Some("b")),
            Err(Problem::TaskHeld { .. })
        ));
        assert!(matches!(
            make_ready(&s, "A01-02", Some("a")),
            Err(Problem::CannotMakeReady { .. })
        ));
        put!(s, release(&s, "A01-02", Some("a"), None).unwrap().task);
        let r = make_ready(&s, "A01-02", None).unwrap().task;
        assert_eq!((r.state, r.waiting_on.clone()), (State::Ready, None));
        put!(s, r);

        // Drop: clears holder and waiting on; done or dropped tasks are closed.
        put!(s, claim(&s, "A02-01", Some("a"), false).unwrap().task);
        assert!(matches!(
            drop_task(&s, "A02-01", Some("b")),
            Err(Problem::TaskHeld { .. })
        ));
        let d = drop_task(&s, "A02-01", Some("a")).unwrap().task;
        assert_eq!((d.state, d.holder.clone()), (State::Dropped, None));
        put!(s, d);
        assert!(matches!(
            drop_task(&s, "A02-01", None),
            Err(Problem::TaskClosed { .. })
        ));
        assert!(matches!(
            set_next(&s, "A02-01", None, Some("x")),
            Err(Problem::TaskClosed { .. })
        ));
        assert!(matches!(
            make_ready(&s, "A02-01", None),
            Err(Problem::CannotMakeReady { .. })
        ));
        assert_eq!(
            set_next(&s, "Z", None, None).map(|_| ()),
            Err(Problem::TaskNotFound { id: "Z".into() })
        );
    }

    #[test]
    fn label_wait_done_release() {
        let mut s = bag();
        put!(s, claim(&s, "A01-01", Some("a"), false).unwrap().task);
        assert!(matches!(
            set_label(&s, "A01-01", Some("a"), "shipped"),
            Err(Problem::LabelNotOnLadder { .. })
        ));
        assert!(matches!(
            set_label(&s, "A01-01", Some("b"), "implemented"),
            Err(Problem::TaskHeld { .. })
        ));
        put!(
            s,
            set_label(&s, "A01-01", Some("a"), "implemented")
                .unwrap()
                .task,
        );
        assert_eq!(
            wait(&s, "A01-01", Some("a"), "  ", None),
            Err(Problem::WaitingOnMissing)
        );
        let w = wait(&s, "A01-01", Some("a"), "owner review", None)
            .unwrap()
            .task;
        assert_eq!(w.state, State::Waiting);
        assert_eq!(w.waiting_on.as_deref(), Some("owner review"));
        put!(s, w);

        // Released while waiting: stays waiting, holder cleared, note kept for the next one.
        let r = release(&s, "A01-01", Some("a"), Some("tests pass; docs left"))
            .unwrap()
            .task;
        assert_eq!(r.state, State::Waiting);
        assert_eq!(r.holder, None);
        assert_eq!(r.handoff_note.as_deref(), Some("tests pass; docs left"));
        put!(s, r);

        // Claimed again, then released without a note: back to ready, note removed.
        put!(s, claim(&s, "A01-01", Some("b"), false).unwrap().task);
        let r = release(&s, "A01-01", Some("b"), None).unwrap().task;
        assert_eq!(r.state, State::Ready);
        assert_eq!(r.handoff_note, None);

        // Below the top step: finished with no warning; `alm` does not judge (Task-A03-15).
        let d = finish(&s, "A01-01", Some("b"), None, None).unwrap();
        assert_eq!(d.task.state, State::Done);
        assert!(d.warnings.is_empty());
        assert_eq!(d.task.without_human_check, None);
        // Finished without the human check: the note is written; empty text is refused, and so
        // are steps without a note, or empty steps (Task-A03-18).
        assert_eq!(
            finish(&s, "A01-01", Some("b"), Some("  "), None).unwrap_err(),
            Problem::WithoutHumanCheckMissing
        );
        assert_eq!(
            finish(&s, "A01-01", Some("b"), None, Some("1. Open it.")).unwrap_err(),
            Problem::WithoutHumanCheckMissing
        );
        assert_eq!(
            finish(&s, "A01-01", Some("b"), Some("card run"), Some(" \n ")).unwrap_err(),
            Problem::HumanCheckStepsEmpty
        );
        // Without steps: finished, with the warning that asks for them.
        let d = finish(
            &s,
            "A01-01",
            Some("b"),
            Some("card run, approver away"),
            None,
        )
        .unwrap();
        assert_eq!(
            d.warnings,
            vec![Problem::HumanCheckStepsMissing {
                id: "A01-01".into()
            }]
        );
        assert_eq!(
            d.task.without_human_check.as_deref(),
            Some("card run, approver away")
        );
        let d = finish(
            &s,
            "A01-01",
            Some("b"),
            Some("card run, approver away"),
            Some("1. Open the app.\n2. Click Save."),
        )
        .unwrap();
        assert!(d.warnings.is_empty());
        assert_eq!(
            d.task.human_check_steps.as_deref(),
            Some("1. Open the app.\n2. Click Save.\n")
        );
        assert!(d.task.to_check());
        put!(s, d.task);
        // Already done: anyone may record its human check again, whoever held it; steps stay.
        let again = finish(&s, "A01-01", Some("c"), Some("loop run"), None).unwrap();
        assert!(again.warnings.is_empty());
        assert_eq!(again.task.without_human_check.as_deref(), Some("loop run"));
        assert!(again.task.human_check_steps.is_some());
        // A person checks it: top label, note and steps cleared; only a done task (Task-A03-18).
        let c = checked(&s, "A01-01").unwrap().task;
        assert_eq!(c.label.as_deref(), Some("human verified"));
        assert_eq!(
            (&c.without_human_check, &c.human_check_steps),
            (&None, &None)
        );
        assert!(!c.to_check());
        assert!(matches!(
            checked(&s, "A01-02"),
            Err(Problem::NotDone { .. })
        ));
        // The top label set on a finished task is the human check too.
        let l = set_label(&s, "A01-01", Some("b"), "human verified")
            .unwrap()
            .task;
        assert_eq!(l.without_human_check, None);
        // A lower label leaves it to check.
        let l = set_label(&s, "A01-01", Some("b"), "implemented")
            .unwrap()
            .task;
        assert!(l.to_check());
        // `done` on a finished task records only its human check.
        let mut old = find_task(&s, "A01-01").unwrap();
        old.without_human_check = None;
        old.human_check_steps = None;
        put!(s, old);
        assert!(!find_task(&s, "A01-01").unwrap().to_check());
        let moved = finish(&s, "A01-01", None, Some("loop run"), None)
            .unwrap()
            .task;
        assert_eq!(moved.without_human_check.as_deref(), Some("loop run"));
        assert!(
            checked(&s, "A01-01")
                .unwrap()
                .task
                .without_human_check
                .is_none()
        );
        // Through the app's edit: the same rule, recorded as `checked`.
        let (o, command) = edit(&s, "A01-01", Some("z"), &Edit::Checked).unwrap();
        assert_eq!(command, "checked");
        assert_eq!(o.task.label.as_deref(), Some("human verified"));
        let e: Edit = serde_json::from_str(r#"{"field":"checked"}"#).unwrap();
        assert_eq!(e, Edit::Checked);
        // Its work changed after the check: the agent sends it back to To check (Task-A03-37).
        put!(s, o.task);
        let back = recheck(&s, "A01-01", " The save button moved ", None).unwrap();
        assert_eq!(
            back.task.without_human_check.as_deref(),
            Some("The save button moved")
        );
        assert_eq!(back.task.label.as_deref(), Some("deployed and verified"));
        assert!(back.task.to_check());
        assert!(matches!(
            back.warnings[..],
            [Problem::HumanCheckStepsMissing { .. }]
        ));
        let with = recheck(&s, "A01-01", "moved", Some("1. Click Save.\n")).unwrap();
        assert!(with.warnings.is_empty());
        assert_eq!(
            with.task.human_check_steps.as_deref(),
            Some("1. Click Save.\n")
        );
        assert!(matches!(
            recheck(&s, "A01-01", " ", None),
            Err(Problem::WithoutHumanCheckMissing)
        ));
        assert!(matches!(
            recheck(&s, "A01-01", "x", Some(" ")),
            Err(Problem::HumanCheckStepsEmpty)
        ));
        assert!(matches!(
            recheck(&s, "A01-02", "x", None),
            Err(Problem::NotDone { .. })
        ));
    }

    #[test]
    fn iterations_and_planning() {
        let mut s = bag();
        let one = new_iteration(&s, "I01", "v0.1", IterationDetails::default()).unwrap();
        assert_eq!((one.order, one.release.clone()), (1, None));
        s.iterations.push(one);
        let two = new_iteration(
            &s,
            "I02",
            " v0.2 ",
            IterationDetails {
                release: Some("v0.2"),
                start: Some("2026-10-08"),
                end: Some(" "),
            },
        )
        .unwrap();
        assert_eq!(
            (
                two.order,
                two.name.as_str(),
                two.start.as_deref(),
                two.end.clone()
            ),
            (2, "v0.2", Some("2026-10-08"), None)
        );
        s.iterations.push(two);
        assert_eq!(
            new_iteration(&s, "I01", "x", IterationDetails::default()),
            Err(Problem::DuplicateIterationId { id: "I01".into() })
        );
        assert_eq!(
            new_iteration(&s, "I03", "", IterationDetails::default()),
            Err(Problem::IterationNameMissing)
        );
        assert_eq!(
            new_iteration(
                &s,
                "I03",
                "x",
                IterationDetails {
                    end: Some("next week"),
                    ..Default::default()
                }
            ),
            Err(Problem::InvalidDate {
                value: "next week".into()
            })
        );

        // Planning works whoever holds the task, and on finished tasks too.
        put!(s, claim(&s, "A01-01", Some("a"), false).unwrap().task);
        put!(s, finish(&s, "A01-01", Some("a"), None, None).unwrap().task);
        let t = plan(&s, "A01-01", Some("I01")).unwrap().task;
        assert_eq!(t.iteration.as_deref(), Some("I01"));
        put!(s, t);
        assert_eq!(
            plan(&s, "A01-01", Some("I09")),
            Err(Problem::UnknownIteration {
                iteration: "I09".into()
            })
        );
        assert_eq!(plan(&s, "A01-01", None).unwrap().task.iteration, None);
        assert_eq!(plan(&s, "A01-01", Some("")).unwrap().task.iteration, None);
        assert!(matches!(
            plan(&s, "nope", None),
            Err(Problem::TaskNotFound { .. })
        ));
    }

    #[test]
    fn areas_and_iterations_are_renamed_changed_and_deleted_only_when_empty() {
        let mut s = bag();
        let a = rename_area(&s, "A01", "  Tracking ").unwrap();
        assert_eq!(
            (a.id.as_str(), a.name.as_str(), a.order),
            ("A01", "Tracking", 2)
        );
        assert_eq!(rename_area(&s, "A01", " "), Err(Problem::AreaNameMissing));
        assert_eq!(
            rename_area(&s, "A09", "x"),
            Err(Problem::UnknownArea { area: "A09".into() })
        );

        let details = IterationDetails {
            release: Some("v0.1"),
            start: Some("2026-10-01"),
            end: Some("2026-10-10"),
        };
        s.iterations
            .push(new_iteration(&s, "I01", "One", details).unwrap());
        // Only what is given changes; an empty release or date clears it.
        let it = change_iteration(
            &s,
            "I01",
            &IterationChange {
                end: Some("2026-10-20".into()),
                release: Some(String::new()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            (
                it.name.as_str(),
                it.start.as_deref(),
                it.end.as_deref(),
                it.release.as_deref(),
                it.order
            ),
            ("One", Some("2026-10-01"), Some("2026-10-20"), None, 1)
        );
        let bad = |c: IterationChange| change_iteration(&s, "I01", &c);
        assert_eq!(
            bad(IterationChange {
                start: Some("1/10/2026".into()),
                ..Default::default()
            }),
            Err(Problem::InvalidDate {
                value: "1/10/2026".into()
            })
        );
        assert_eq!(
            bad(IterationChange {
                name: Some("".into()),
                ..Default::default()
            }),
            Err(Problem::IterationNameMissing)
        );
        assert_eq!(
            change_iteration(&s, "I09", &IterationChange::default()),
            Err(Problem::UnknownIteration {
                iteration: "I09".into()
            })
        );

        // Deleting: refused while any task is in it, a dropped one included.
        assert_eq!(
            delete_area(&s, "A01"),
            Err(Problem::AreaNotEmpty {
                id: "A01".into(),
                count: "2".into()
            })
        );
        put!(s, drop_task(&s, "A02-01", None).unwrap().task);
        assert!(matches!(
            delete_area(&s, "A02"),
            Err(Problem::AreaNotEmpty { .. })
        ));
        assert_eq!(delete_iteration(&s, "I01").unwrap().id, "I01");
        put!(s, plan(&s, "A02-01", Some("I01")).unwrap().task);
        assert_eq!(
            delete_iteration(&s, "I01"),
            Err(Problem::IterationNotEmpty {
                id: "I01".into(),
                count: "1".into()
            })
        );
        s.tasks.retain(|t| t.area != "A02");
        assert_eq!(delete_area(&s, "A02").unwrap().id, "A02");
        assert_eq!(delete_iteration(&s, "I01").unwrap().id, "I01");
        assert!(matches!(
            delete_area(&s, "A09"),
            Err(Problem::UnknownArea { .. })
        ));
    }

    #[test]
    fn next_lists_free_ready_tasks_by_iteration_area_then_id() {
        let mut s = bag();
        assert!(next(&s, None).unwrap().is_empty());
        for id in ["A01-02", "A01-01", "A02-01"] {
            let mut t = find_task(&s, id).unwrap();
            t.state = State::Ready;
            put!(s, t);
        }
        let ids = |s: &Taskbag, i: Option<&str>| -> Vec<String> {
            next(s, i).unwrap().iter().map(|t| t.id.clone()).collect()
        };
        // A02 has the lower order, so it comes first.
        assert_eq!(ids(&s, None), ["A02-01", "A01-01", "A01-02"]);
        put!(s, claim(&s, "A02-01", Some("a"), false).unwrap().task);
        assert_eq!(ids(&s, None), ["A01-01", "A01-02"]);
        assert!(matches!(
            next(&s, Some("I09")),
            Err(Problem::UnknownIteration { .. })
        ));
    }

    /// Every edit the app's task pane makes goes through the same rule as its `alm` command, and
    /// is refused the same way (Task-A01-18).
    #[test]
    fn each_app_edit_is_its_alm_rule() {
        let mut s = bag();
        s.iterations
            .push(new_iteration(&s, "I01", "One", IterationDetails::default()).unwrap());
        let me = Some("will");
        let ed = |s: &Taskbag, e: Edit| edit(s, "A01-01", me, &e);
        let same = |got: Result<(Outcome, &str), Problem>,
                    want: Result<Outcome, Problem>,
                    command: &str| {
            match (got, want) {
                (Ok((g, c)), Ok(w)) => {
                    assert_eq!(g, w);
                    assert_eq!(c, command);
                }
                (Err(g), Err(w)) => assert_eq!(g, w),
                (g, w) => panic!("{g:?} vs {w:?}"),
            }
        };
        let state = |st: State, on: Option<&str>| Edit::State {
            state: st,
            waiting_on: on.map(String::from),
            who: None,
            blocked_by: vec![],
        };
        same(
            ed(&s, state(State::Ready, None)),
            make_ready(&s, "A01-01", me),
            "ready",
        );
        same(
            ed(&s, state(State::Waiting, Some("the owner"))),
            wait(&s, "A01-01", me, "the owner", None),
            "wait",
        );
        same(
            ed(&s, state(State::Waiting, None)),
            wait(&s, "A01-01", me, "", None),
            "wait",
        );
        same(
            ed(&s, state(State::Done, None)),
            finish(&s, "A01-01", me, None, None),
            "done",
        );
        same(
            ed(&s, state(State::Dropped, None)),
            drop_task(&s, "A01-01", me),
            "drop",
        );
        assert_eq!(
            ed(&s, state(State::InProgress, None)),
            Err(Problem::CannotSetState {
                id: "A01-01".into(),
                state: "in_progress".into()
            })
        );
        assert!(ed(&s, state(State::Backlog, None)).is_err());
        for label in ["implemented", "nonsense"] {
            same(
                ed(
                    &s,
                    Edit::Label {
                        label: label.into(),
                    },
                ),
                set_label(&s, "A01-01", me, label),
                "label",
            );
        }
        same(
            ed(&s, Edit::WaitingOn { text: "x".into() }),
            wait(&s, "A01-01", me, "x", None),
            "wait",
        );
        same(
            ed(
                &s,
                Edit::NextDependency {
                    text: Some("y".into()),
                },
            ),
            set_next(&s, "A01-01", me, Some("y")),
            "needs",
        );
        for it in [Some("I01"), Some("I09"), None] {
            same(
                ed(
                    &s,
                    Edit::Iteration {
                        iteration: it.map(String::from),
                    },
                ),
                plan(&s, "A01-01", it),
                "plan",
            );
        }
        same(
            ed(
                &s,
                Edit::Title {
                    title: " ".into(),
                    expected_version: None,
                },
            ),
            set_title(&s, "A01-01", me, " "),
            "title",
        );
        let (o, c) = ed(
            &s,
            Edit::HandoffNote {
                text: Some(" n ".into()),
            },
        )
        .unwrap();
        assert_eq!((o.task.handoff_note.as_deref(), c), (Some("n"), "note"));
        let (o, c) = ed(
            &s,
            Edit::Description {
                text: "New\n\n".into(),
                expected_version: None,
            },
        )
        .unwrap();
        assert_eq!((o.task.description.as_str(), c), ("New\n", "describe"));
        let (o, _) = ed(
            &s,
            Edit::Description {
                text: " ".into(),
                expected_version: None,
            },
        )
        .unwrap();
        assert_eq!(o.task.description, "");

        // A task an agent holds: the app's person is refused by the holder rule, like `alm`;
        // planning is not progress, so it is still allowed.
        put!(
            s,
            claim(&s, "A01-01", Some("claude/x"), false).unwrap().task
        );
        let held = Err(Problem::TaskHeld {
            id: "A01-01".into(),
            holder: "claude/x".into(),
        });
        for e in [
            state(State::Ready, None),
            state(State::Done, None),
            Edit::Label {
                label: "implemented".into(),
            },
            Edit::NextDependency { text: None },
            Edit::HandoffNote { text: None },
            Edit::Title {
                title: "t".into(),
                expected_version: None,
            },
            Edit::Description {
                text: "d".into(),
                expected_version: None,
            },
        ] {
            assert_eq!(ed(&s, e).map(|(o, _)| o), held.clone());
        }
        assert!(
            ed(
                &s,
                Edit::Iteration {
                    iteration: Some("I01".into())
                }
            )
            .is_ok()
        );
    }

    #[test]
    fn edits_arrive_as_json_from_the_app() {
        let e: Edit =
            serde_json::from_str(r#"{"field":"state","state":"waiting","waiting_on":"x"}"#)
                .unwrap();
        assert_eq!(
            e,
            Edit::State {
                state: State::Waiting,
                waiting_on: Some("x".into()),
                who: None,
                blocked_by: vec![]
            }
        );
        let e: Edit = serde_json::from_str(r#"{"field":"iteration","iteration":null}"#).unwrap();
        assert_eq!(e, Edit::Iteration { iteration: None });
        assert!(serde_json::from_str::<Edit>(r#"{"field":"holder","holder":"x"}"#).is_err());
    }

    #[test]
    fn finished_and_dropped_tasks_count_as_held_by_nobody() {
        // Approver, 2026-10-16 (Task-A00-30): the holder a finished task keeps is history.
        let mut s = bag();
        s.tasks[0].holder = Some("old/session".into());
        s.tasks[0].state = State::Done;
        assert!(set_description(&s, &s.tasks[0].id.clone(), Some("me"), "New").is_ok());
        s.tasks[0].state = State::Dropped;
        assert!(set_title(&s, &s.tasks[0].id.clone(), Some("me"), "New").is_ok());
        s.tasks[0].state = State::InProgress;
        assert!(matches!(
            set_description(&s, &s.tasks[0].id.clone(), Some("me"), "New"),
            Err(Problem::TaskHeld { .. })
        ));
    }
}
