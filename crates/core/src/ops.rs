//! Task changes and their rules.
//!
//! Each function takes the seabag as it is now and returns the changed item, or the
//! [`Problem`] that refuses the change. Nothing here touches the disk: the caller loads
//! the seabag, applies one change and writes the result, all while holding the
//! seabag's lock (see `crates/store`).

use crate::model::{Area, Iteration, Project, Seabag, State, Task};
use crate::problem::Problem;
use crate::validate::valid_date;

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

/// The next free task ID in `area`, following the project's own pattern `<area>-<number>`:
/// one more than the highest number used, as wide as the widest one (at least two digits).
/// Child tasks (`A01-03-01`) and other IDs do not count. Refused when the area does not exist.
pub fn next_task_id(seabag: &Seabag, area: &str) -> Result<String, Problem> {
    if !seabag.areas.iter().any(|a| a.id == area) {
        return Err(Problem::UnknownArea {
            area: area.to_string(),
        });
    }
    let prefix = format!("{area}-");
    let (highest, width) = seabag
        .tasks
        .iter()
        .filter_map(|t| t.id.strip_prefix(&prefix))
        .filter(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
        .filter_map(|n| n.parse::<u64>().ok().map(|v| (v, n.len())))
        .fold((0, 2), |(h, w), (v, len)| (h.max(v), w.max(len)));
    Ok(format!("{prefix}{:0width$}", highest + 1))
}

/// The project with a new task language (`fh language`): `ui` to follow the FlipHopper app's
/// language, or a language code. Refused when it is neither.
pub fn set_task_language(seabag: &Seabag, value: &str) -> Result<Project, Problem> {
    let value = value.trim();
    if value != crate::model::TASK_LANGUAGE_UI && !crate::model::valid_language(value) {
        return Err(Problem::InvalidLanguage {
            value: value.to_string(),
        });
    }
    Ok(Project {
        task_language: Some(value.to_string()),
        ..seabag.project.clone()
    })
}

/// The project with a new label ladder, lowest step first (`fh labels`). Refused when it is
/// empty, or when a task's label is not on it: changing the ladder never strands a task.
pub fn set_ladder(seabag: &Seabag, labels: &[String]) -> Result<Project, Problem> {
    let labels: Vec<String> = labels
        .iter()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    if labels.is_empty() {
        return Err(Problem::LadderEmpty);
    }
    if let Some(label) = seabag
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
        ..seabag.project.clone()
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
    seabag: &Seabag,
    id: &str,
    name: &str,
    details: IterationDetails,
) -> Result<Iteration, Problem> {
    check_id(id)?;
    if seabag.iterations.iter().any(|i| i.id == id) {
        return Err(Problem::DuplicateIterationId { id: id.to_string() });
    }
    let name = non_empty(Some(name)).ok_or(Problem::IterationNameMissing)?;
    let date = |d: Option<&str>| match non_empty(d) {
        Some(d) if !valid_date(&d) => Err(Problem::InvalidDate { value: d }),
        d => Ok(d),
    };
    let order = seabag.iterations.iter().map(|i| i.order).max().unwrap_or(0) + 1;
    Ok(Iteration {
        id: id.to_string(),
        name,
        order,
        start: date(details.start)?,
        end: date(details.end)?,
        release: non_empty(details.release),
    })
}

/// Plan the task in an iteration, or take it out of its iteration with `None`. Planning is not
/// progress, so anyone may do it, whoever holds the task and in any state (a finished task can
/// still be placed in the iteration it was finished in).
pub fn plan(seabag: &Seabag, id: &str, iteration: Option<&str>) -> Result<Outcome, Problem> {
    let mut task = find_task(seabag, id)?;
    let iteration = non_empty(iteration);
    if let Some(it) = &iteration {
        if !seabag.iterations.iter().any(|i| &i.id == it) {
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
        iteration: None,
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

/// Set the task's next dependency: the one thing needed for its next label. `None` (or empty
/// text) clears it.
pub fn set_next(
    seabag: &Seabag,
    id: &str,
    me: Option<&str>,
    text: Option<&str>,
) -> Result<Outcome, Problem> {
    let mut task = find_task(seabag, id)?;
    check_holder(&task, me)?;
    check_open(&task, &[State::Dropped])?;
    task.next_dependency = non_empty(text);
    Ok(Outcome::plain(task))
}

/// Mark an unheld `backlog` or `waiting` task `ready` (for example, the owner approved it).
/// Its *waiting on* is cleared.
pub fn make_ready(seabag: &Seabag, id: &str, me: Option<&str>) -> Result<Outcome, Problem> {
    let mut task = find_task(seabag, id)?;
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
pub fn drop_task(seabag: &Seabag, id: &str, me: Option<&str>) -> Result<Outcome, Problem> {
    let mut task = find_task(seabag, id)?;
    check_holder(&task, me)?;
    check_open(&task, &[State::Done, State::Dropped])?;
    task.state = State::Dropped;
    task.holder = None;
    task.waiting_on = None;
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
    use crate::model::default_labels;

    fn bag() -> Seabag {
        let mut s = Seabag {
            project: Project {
                format: crate::FORMAT.into(),
                id: "p".into(),
                name: "P".into(),
                labels: default_labels(),
                project_folder: None,
                task_language: None,
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

    /// Store a changed task back into the seabag, as writing and reloading would.
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
        put!(s, finish(&s, "A01-01", Some("a")).unwrap().task);
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
