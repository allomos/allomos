//! Task changes and their rules.
//!
//! Each function takes the taskbag as it is now and returns the changed item, or the
//! [`Problem`] that refuses the change. Nothing here touches the disk: the caller loads
//! the taskbag, applies one change and writes the result, all while holding the
//! taskbag's lock (see `crates/store`).

use crate::model::{Area, Iteration, Project, State, Task, Taskbag};
use crate::problem::Problem;
use crate::validate::valid_date;
use serde::Deserialize;

/// A changed task plus warnings: things worth telling the user that do not refuse the change.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub task: Task,
    pub warnings: Vec<Problem>,
}

impl Outcome {
    fn plain(task: Task) -> Self {
        Outcome {
            task,
            warnings: Vec::new(),
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
fn check_holder(task: &Task, me: Option<&str>) -> Result<(), Problem> {
    match &task.holder {
        Some(h) if Some(h.as_str()) != me => Err(Problem::TaskHeld {
            id: task.id.clone(),
            holder: h.clone(),
        }),
        _ => Ok(()),
    }
}

fn check_open(task: &Task, closed: &[State]) -> Result<(), Problem> {
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

/// The next free task ID in `area`, following the project's own pattern `<area>-<number>`:
/// one more than the highest number used, as wide as the widest one (at least two digits).
/// Child tasks (`A01-03-01`) and other IDs do not count. Refused when the area does not exist.
pub fn next_task_id(taskbag: &Taskbag, area: &str) -> Result<String, Problem> {
    if !taskbag.areas.iter().any(|a| a.id == area) {
        return Err(Problem::UnknownArea {
            area: area.to_string(),
        });
    }
    let prefix = format!("{area}-");
    let (highest, width) = taskbag
        .tasks
        .iter()
        .filter_map(|t| t.id.strip_prefix(&prefix))
        .filter(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
        .filter_map(|n| n.parse::<u64>().ok().map(|v| (v, n.len())))
        .fold((0, 2), |(h, w), (v, len)| (h.max(v), w.max(len)));
    Ok(format!("{prefix}{:0width$}", highest + 1))
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
        next_dependency: None,
        iteration: None,
        handoff_note: None,
        finished_without_owner: None,
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
    task.waiting_on = None;
    Ok(Outcome { task, warnings })
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
    Ok(Outcome::plain(task))
}

/// Park the task as `waiting`, saying what it waits on.
pub fn wait(taskbag: &Taskbag, id: &str, me: Option<&str>, on: &str) -> Result<Outcome, Problem> {
    let mut task = find_task(taskbag, id)?;
    check_holder(&task, me)?;
    check_open(&task, &[State::Done, State::Dropped])?;
    task.waiting_on = Some(non_empty(Some(on)).ok_or(Problem::WaitingOnMissing)?);
    task.state = State::Waiting;
    Ok(Outcome::plain(task))
}

/// Finish the task. `alm` does not judge whether it is really finished: the agent is given the
/// rule before it finishes (Task-A03-15). `without_owner` records why a task is finished without
/// the owner's check (an automatic or loop run); empty text is refused. A note already on the task
/// is kept when none is given.
pub fn finish(
    taskbag: &Taskbag,
    id: &str,
    me: Option<&str>,
    without_owner: Option<&str>,
) -> Result<Outcome, Problem> {
    let mut task = find_task(taskbag, id)?;
    check_holder(&task, me)?;
    check_open(&task, &[State::Dropped])?;
    if let Some(text) = without_owner {
        task.finished_without_owner =
            Some(non_empty(Some(text)).ok_or(Problem::WithoutOwnerMissing)?);
    }
    task.state = State::Done;
    task.waiting_on = None;
    Ok(Outcome::plain(task))
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
    /// `ready`, `waiting` (with `waiting_on`), `done` or `dropped`. `in_progress` comes only from
    /// claiming the task, and nothing moves a task back to `backlog`.
    State {
        state: State,
        #[serde(default)]
        waiting_on: Option<String>,
    },
    Label {
        label: String,
    },
    /// What the task waits on; like `alm wait`, this also makes it `waiting`.
    WaitingOn {
        text: String,
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
    Title {
        title: String,
    },
    Description {
        text: String,
    },
}

/// Apply one [`Edit`] by `me`, through the rule `alm` uses for it. Returns the changed task and
/// the name of that rule's command (`ready`, `wait`, `done`, `drop`, `label`, `needs`, `plan`,
/// `title`; `note` and `describe` for the two changes `alm` has no command of their own for), so
/// the change is recorded the same way.
pub fn edit(
    taskbag: &Taskbag,
    id: &str,
    me: Option<&str>,
    edit: &Edit,
) -> Result<(Outcome, &'static str), Problem> {
    match edit {
        Edit::State { state, waiting_on } => match state {
            State::Ready => make_ready(taskbag, id, me).map(|o| (o, "ready")),
            State::Waiting => wait(taskbag, id, me, waiting_on.as_deref().unwrap_or_default())
                .map(|o| (o, "wait")),
            State::Done => finish(taskbag, id, me, None).map(|o| (o, "done")),
            State::Dropped => drop_task(taskbag, id, me).map(|o| (o, "drop")),
            State::Backlog | State::InProgress => Err(Problem::CannotSetState {
                id: id.to_string(),
                state: state.as_str().to_string(),
            }),
        },
        Edit::Label { label } => set_label(taskbag, id, me, label).map(|o| (o, "label")),
        Edit::WaitingOn { text } => wait(taskbag, id, me, text).map(|o| (o, "wait")),
        Edit::NextDependency { text } => {
            set_next(taskbag, id, me, text.as_deref()).map(|o| (o, "needs"))
        }
        Edit::HandoffNote { text } => {
            set_handoff_note(taskbag, id, me, text.as_deref()).map(|o| (o, "note"))
        }
        Edit::Iteration { iteration } => {
            plan(taskbag, id, iteration.as_deref()).map(|o| (o, "plan"))
        }
        Edit::Title { title } => set_title(taskbag, id, me, title).map(|o| (o, "title")),
        Edit::Description { text } => {
            set_description(taskbag, id, me, text).map(|o| (o, "describe"))
        }
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

/// Mark an unheld `backlog` or `waiting` task `ready` (for example, the owner approved it).
/// Its *waiting on* is cleared.
pub fn make_ready(taskbag: &Taskbag, id: &str, me: Option<&str>) -> Result<Outcome, Problem> {
    let mut task = find_task(taskbag, id)?;
    check_holder(&task, me)?;
    if task.holder.is_some() || !matches!(task.state, State::Backlog | State::Waiting) {
        return Err(Problem::CannotMakeReady {
            id: task.id.clone(),
            state: task.state.as_str().to_string(),
        });
    }
    task.state = State::Ready;
    task.waiting_on = None;
    Ok(Outcome::plain(task))
}

/// Drop the task: it will not be done. Its holder and *waiting on* are cleared.
pub fn drop_task(taskbag: &Taskbag, id: &str, me: Option<&str>) -> Result<Outcome, Problem> {
    let mut task = find_task(taskbag, id)?;
    check_holder(&task, me)?;
    check_open(&task, &[State::Done, State::Dropped])?;
    task.state = State::Dropped;
    task.holder = None;
    task.waiting_on = None;
    Ok(Outcome::plain(task))
}

/// Every task that is `ready` and held by no one, for the agent to choose from (Task-A03-14):
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

    #[test]
    fn the_next_task_id_follows_the_area_pattern() {
        let mut s = bag();
        // bag() has A01-01, A01-02 and A02-01.
        assert_eq!(next_task_id(&s, "A01").unwrap(), "A01-03");
        assert_eq!(next_task_id(&s, "A02").unwrap(), "A02-02");
        // Children and other IDs do not count; a wider number keeps its width.
        for id in ["A01-02-07", "A01-x", "A02-009"] {
            let area = &id[..3];
            s.tasks.push(new_task(&s, id, area, "t", None).unwrap());
        }
        assert_eq!(next_task_id(&s, "A01").unwrap(), "A01-03");
        assert_eq!(next_task_id(&s, "A02").unwrap(), "A02-010");
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
        s.tasks[0].label = Some("owner verified".into());
        assert_eq!(
            set_ladder(&s, &ladder(&["implemented"])),
            Err(Problem::LabelNotOnLadder {
                label: "owner verified".into(),
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
        let done = finish(&s, "A01-01", Some("a"), None).unwrap().task;
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
        put!(s, wait(&s, "A01-02", Some("a"), "owner").unwrap().task);
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
            wait(&s, "A01-01", Some("a"), "  "),
            Err(Problem::WaitingOnMissing)
        );
        let w = wait(&s, "A01-01", Some("a"), "owner review").unwrap().task;
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
        let d = finish(&s, "A01-01", Some("b"), None).unwrap();
        assert_eq!(d.task.state, State::Done);
        assert!(d.warnings.is_empty());
        assert_eq!(d.task.finished_without_owner, None);
        // Finished without the owner's check: the note is written; empty text is refused.
        assert_eq!(
            finish(&s, "A01-01", Some("b"), Some("  ")).unwrap_err(),
            Problem::WithoutOwnerMissing
        );
        let d = finish(&s, "A01-01", Some("b"), Some("card run, owner away"))
            .unwrap()
            .task;
        assert_eq!(
            d.finished_without_owner.as_deref(),
            Some("card run, owner away")
        );
        put!(s, d);
        // Labelled owner verified later and unheld: anyone may finish it, the note is kept.
        let mut top = find_task(&s, "A01-01").unwrap();
        top.label = Some("owner verified".into());
        top.holder = None;
        put!(s, top);
        let d = finish(&s, "A01-01", None, None).unwrap();
        assert!(d.warnings.is_empty());
        assert_eq!(
            d.task.finished_without_owner.as_deref(),
            Some("card run, owner away")
        );
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
        put!(s, finish(&s, "A01-01", Some("a"), None).unwrap().task);
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
        };
        same(
            ed(&s, state(State::Ready, None)),
            make_ready(&s, "A01-01", me),
            "ready",
        );
        same(
            ed(&s, state(State::Waiting, Some("the owner"))),
            wait(&s, "A01-01", me, "the owner"),
            "wait",
        );
        same(
            ed(&s, state(State::Waiting, None)),
            wait(&s, "A01-01", me, ""),
            "wait",
        );
        same(
            ed(&s, state(State::Done, None)),
            finish(&s, "A01-01", me, None),
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
            wait(&s, "A01-01", me, "x"),
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
            ed(&s, Edit::Title { title: " ".into() }),
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
            },
        )
        .unwrap();
        assert_eq!((o.task.description.as_str(), c), ("New\n", "describe"));
        let (o, _) = ed(&s, Edit::Description { text: " ".into() }).unwrap();
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
            Edit::Title { title: "t".into() },
            Edit::Description { text: "d".into() },
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
                waiting_on: Some("x".into())
            }
        );
        let e: Edit = serde_json::from_str(r#"{"field":"iteration","iteration":null}"#).unwrap();
        assert_eq!(e, Edit::Iteration { iteration: None });
        assert!(serde_json::from_str::<Edit>(r#"{"field":"holder","holder":"x"}"#).is_err());
    }
}
