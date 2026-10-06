//! Task changes and their rules.
//!
//! Each function takes the seabag as it is now and returns the changed item, or the
//! [`Problem`] that refuses the change. Nothing here touches the disk: the caller loads
//! the seabag, applies one change and writes the result, all while holding the
//! seabag's lock (see `crates/store`).

use crate::model::{Area, Seabag, State, Task};
use crate::problem::Problem;

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
pub fn find_task(seabag: &Seabag, id: &str) -> Result<Task, Problem> {
    seabag
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
pub fn new_area(seabag: &Seabag, id: &str, name: &str) -> Result<Area, Problem> {
    check_id(id)?;
    if seabag.areas.iter().any(|a| a.id == id) {
        return Err(Problem::DuplicateAreaId { id: id.to_string() });
    }
    let name = non_empty(Some(name)).ok_or(Problem::AreaNameMissing)?;
    let order = seabag.areas.iter().map(|a| a.order).max().unwrap_or(0) + 1;
    Ok(Area {
        id: id.to_string(),
        name,
        order,
    })
}

/// A new task in `backlog`. Its ID must be valid and unused, its area must exist, and it
/// needs a title.
pub fn new_task(
    seabag: &Seabag,
    id: &str,
    area: &str,
    title: &str,
    description: Option<&str>,
) -> Result<Task, Problem> {
    check_id(id)?;
    if seabag.tasks.iter().any(|t| t.id == id) {
        return Err(Problem::DuplicateTaskId { id: id.to_string() });
    }
    if !seabag.areas.iter().any(|a| a.id == area) {
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
        handoff_note: None,
        description,
    })
}

/// `me` takes the task and starts it. Refused without a name, when someone else holds it
/// (unless `take` is set, which takes it over with a warning), or when it is done or dropped.
pub fn claim(seabag: &Seabag, id: &str, me: Option<&str>, take: bool) -> Result<Outcome, Problem> {
    let me = non_empty(me).ok_or(Problem::HolderMissing)?;
    let mut task = find_task(seabag, id)?;
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
    seabag: &Seabag,
    id: &str,
    me: Option<&str>,
    label: &str,
) -> Result<Outcome, Problem> {
    let mut task = find_task(seabag, id)?;
    check_holder(&task, me)?;
    check_open(&task, &[State::Dropped])?;
    if !seabag.project.labels.iter().any(|l| l == label) {
        return Err(Problem::LabelNotOnLadder {
            label: label.to_string(),
            ladder: seabag.project.labels.join(" → "),
        });
    }
    task.label = Some(label.to_string());
    Ok(Outcome::plain(task))
}

/// Park the task as `waiting`, saying what it waits on.
pub fn wait(seabag: &Seabag, id: &str, me: Option<&str>, on: &str) -> Result<Outcome, Problem> {
    let mut task = find_task(seabag, id)?;
    check_holder(&task, me)?;
    check_open(&task, &[State::Done, State::Dropped])?;
    task.waiting_on = Some(non_empty(Some(on)).ok_or(Problem::WaitingOnMissing)?);
    task.state = State::Waiting;
    Ok(Outcome::plain(task))
}

/// Finish the task. Allowed below the top label, with a warning (owner, 2026-10-06).
pub fn finish(seabag: &Seabag, id: &str, me: Option<&str>) -> Result<Outcome, Problem> {
    let mut task = find_task(seabag, id)?;
    check_holder(&task, me)?;
    check_open(&task, &[State::Dropped])?;
    let mut warnings = Vec::new();
    if let Some(top) = seabag.project.labels.last() {
        if task.label.as_ref() != Some(top) {
            warnings.push(Problem::LabelBelowTop {
                label: task.label.clone().unwrap_or_else(|| "none".to_string()),
                top: top.clone(),
            });
        }
    }
    task.state = State::Done;
    task.waiting_on = None;
    Ok(Outcome { task, warnings })
}

/// Let go of the task. A task `in_progress` goes back to `ready`. The handoff note is
/// replaced by `note`, or removed when there is none, so it always describes the latest
/// release.
pub fn release(
    seabag: &Seabag,
    id: &str,
    me: Option<&str>,
    note: Option<&str>,
) -> Result<Outcome, Problem> {
    let mut task = find_task(seabag, id)?;
    check_holder(&task, me)?;
    task.holder = None;
    if task.state == State::InProgress {
        task.state = State::Ready;
    }
    task.handoff_note = non_empty(note);
    Ok(Outcome::plain(task))
}

/// The first task that is `ready` and held by no one: by area order, then area ID, then task ID.
pub fn next(seabag: &Seabag) -> Option<&Task> {
    let area_rank = |area: &str| {
        seabag
            .areas
            .iter()
            .position(|a| a.id == area)
            .unwrap_or(usize::MAX)
    };
    seabag
        .tasks
        .iter()
        .filter(|t| t.state == State::Ready && t.holder.is_none())
        .min_by(|a, b| {
            area_rank(&a.area)
                .cmp(&area_rank(&b.area))
                .then(a.id.cmp(&b.id))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Project, default_labels};

    fn bag() -> Seabag {
        let mut s = Seabag {
            project: Project {
                format: crate::FORMAT.into(),
                id: "p".into(),
                name: "P".into(),
                labels: default_labels(),
            },
            areas: vec![],
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

    /// Store a changed task back into the seabag, as writing and reloading would.
    macro_rules! put {
        ($s:ident, $task:expr $(,)?) => {{
            let t: Task = $task;
            let i = $s.tasks.iter().position(|x| x.id == t.id).unwrap();
            $s.tasks[i] = t;
        }};
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
        let done = finish(&s, "A01-01", Some("a")).unwrap().task;
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

        let d = finish(&s, "A01-01", Some("b")).unwrap();
        assert_eq!(d.task.state, State::Done);
        assert_eq!(
            d.warnings,
            vec![Problem::LabelBelowTop {
                label: "implemented".into(),
                top: "owner verified".into()
            }]
        );
        // At the top step, and unheld: anyone may finish it, with no warning.
        let mut top = find_task(&s, "A01-01").unwrap();
        top.label = Some("owner verified".into());
        top.holder = None;
        put!(s, top);
        assert!(finish(&s, "A01-01", None).unwrap().warnings.is_empty());
    }

    #[test]
    fn next_picks_first_free_ready_task_by_area_then_id() {
        let mut s = bag();
        assert_eq!(next(&s), None);
        for id in ["A01-02", "A01-01", "A02-01"] {
            let mut t = find_task(&s, id).unwrap();
            t.state = State::Ready;
            put!(s, t);
        }
        // A02 has the lower order, so it comes first.
        assert_eq!(next(&s).unwrap().id, "A02-01");
        put!(s, claim(&s, "A02-01", Some("a"), false).unwrap().task);
        assert_eq!(next(&s).unwrap().id, "A01-01");
    }
}
