//! The queue of what needs a person (Task-A01-muffin353, decision 0028): one row per task, in
//! the order a person should take them. `alm human` and the app both list it from here, so an
//! agent and a person see the same order.
//!
//! The order, top to bottom (decision 0028 § 5):
//! 1. **An agent is waiting**: a question to answer, a plan to approve, an emergency change to
//!    approve afterwards. Always first, whatever the risk. Inside the group: the task that
//!    holds back more unfinished tasks first, then the one waiting longer.
//! 2. **Finished, for the person to look at**: to check, AI decisions to review. By the risk
//!    the agent recorded: high, medium, low, not classified (decision 0027); inside one level,
//!    the one waiting longer first.
//! 3. When neither tells them apart: the task whose iteration comes earlier in the plan.
//!
//! Nothing is judged here: the risk is the one an agent recorded with `alm classify`, and "how
//! long" is read from the history (when the task was finished, began to wait, or got the label
//! AI decisions to review). No new field is stored.

use std::cmp::Reverse;
use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::classify::{Classification, Unapproved};
use crate::decision::Decision;
use crate::event::{Event, Kind};
use crate::{State, Task, Taskbag};

/// One thing a person has to do about a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Todo {
    /// The task waits on a person (`alm wait`): a question, or an approval parked as one.
    Answer,
    /// A plan waits for a Go ahead (`alm confirm --task`), while the question is open.
    Approve,
    /// An emergency change not approved afterwards yet (`alm classify --type emergency`).
    ApproveAfterwards,
    /// Finished without the human check, not checked since.
    Check,
    /// It carries the label AI decisions to review.
    Review,
}

impl Todo {
    /// An agent is waiting for it (the first group of the queue).
    pub fn agent_waiting(self) -> bool {
        matches!(
            self,
            Todo::Answer | Todo::Approve | Todo::ApproveAfterwards
        )
    }
}

/// When a task began to need a person, each read from the history (UTC, as events write it).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Times {
    /// When it began to wait on a person; none once it stopped waiting.
    pub waiting: Option<String>,
    /// When it became "to check": finished, or sent back after a check (`alm recheck`).
    pub check: Option<String>,
    /// When it got the label AI decisions to review; none once the label is gone.
    pub review: Option<String>,
}

/// Every task's [`Times`], by task ID, from `events` (oldest first).
pub fn times(events: &[Event]) -> BTreeMap<String, Times> {
    let mut out: BTreeMap<String, Times> = BTreeMap::new();
    let has = |v: &Option<Value>, wanted: &dyn Fn(&str) -> bool| {
        v.as_ref()
            .and_then(Value::as_array)
            .is_some_and(|a| a.iter().filter_map(Value::as_str).any(wanted))
    };
    for e in events {
        let (Kind::Task, Some(id)) = (e.kind, e.id.as_deref()) else {
            continue;
        };
        for c in &e.changes {
            match c.field.as_str() {
                "state" => {
                    let to = c.to.as_ref().and_then(Value::as_str);
                    let from = c.from.as_ref().and_then(Value::as_str);
                    let entry = out.entry(id.to_string()).or_default();
                    if to == Some("waiting") {
                        entry.waiting = Some(e.at.clone());
                    } else if from == Some("waiting") {
                        entry.waiting = None;
                    }
                    if to == Some("done") {
                        entry.check = Some(e.at.clone());
                    } else if from == Some("done") {
                        entry.check = None;
                    }
                }
                "labels" => {
                    let entry = out.entry(id.to_string()).or_default();
                    let review = |l: &str| l == crate::labels::AI_DECISIONS;
                    match (has(&c.from, &review), has(&c.to, &review)) {
                        (false, true) => entry.review = Some(e.at.clone()),
                        (true, false) => entry.review = None,
                        _ => {}
                    }
                    // The "by human" label was taken off: it is to check again, from now.
                    let by_human = |l: &str| crate::labels::BY_HUMAN.contains(&l);
                    if has(&c.from, &by_human) && !has(&c.to, &by_human) {
                        entry.check = Some(e.at.clone());
                    }
                }
                _ => {}
            }
        }
    }
    out
}

/// A plan waiting for a Go ahead on a task, and when it was asked (UTC).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asked {
    pub task: String,
    pub at: String,
}

/// What the queue is worked out from, besides the taskbag: all read from the history, except
/// `asked` (questions open right now, which only the app knows).
#[derive(Debug, Clone, Copy)]
pub struct Facts<'a> {
    pub times: &'a BTreeMap<String, Times>,
    /// Every task's classification ([`crate::classify::recorded`]).
    pub classes: &'a BTreeMap<String, Classification>,
    /// Every task's AI decisions to review ([`crate::decision::to_review`] of its decisions).
    pub decisions: &'a BTreeMap<String, Vec<Decision>>,
    /// The emergencies not approved afterwards ([`crate::classify::unapproved`]).
    pub unapproved: &'a [Unapproved],
    pub asked: &'a [Asked],
}

/// One row of the queue: one task, with everything a person has to do about it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Row {
    pub task: String,
    /// What to do, in a fixed order: answer, approve, approve afterwards, check, review.
    pub todo: Vec<Todo>,
    /// How many AI decisions wait for review (0 without [`Todo::Review`], or when the label is
    /// there and the history names none).
    pub decisions: usize,
    /// An agent is waiting for one of them: the row is in the first group.
    pub agent_waiting: bool,
    /// The risk as the agent recorded it; none when the task is not classified.
    pub risk: Option<String>,
    /// Since when it waits for the person: the earliest of what puts it in its group. None when
    /// the history does not say.
    pub waiting_since: Option<String>,
    /// How many unfinished tasks name it in their `blocked_by` (decision 0026).
    pub blocks: usize,
}

/// 0 for high, then medium, low, and anything else (not classified, or a value not in the list).
pub fn risk_rank(risk: Option<&str>) -> u8 {
    match risk {
        Some("high") => 0,
        Some("medium") => 1,
        Some("low") => 2,
        _ => 3,
    }
}

/// The queue for `tasks` (all of `taskbag`'s, or the ones a filter kept), in order. A task with
/// nothing for a person to do has no row.
pub fn rows(taskbag: &Taskbag, tasks: &[&Task], facts: Facts) -> Vec<Row> {
    let earliest = |a: Option<String>, b: Option<&str>| match (a, b) {
        (Some(a), Some(b)) => Some(if b < a.as_str() { b.to_string() } else { a }),
        (a, b) => a.or(b.map(str::to_string)),
    };
    let mut rows: Vec<(usize, Row)> = Vec::new();
    for t in tasks {
        let at = facts.times.get(&t.id);
        let mut todo = Vec::new();
        let (mut first, mut second): (Option<String>, Option<String>) = (None, None);
        if t.state == State::Waiting {
            todo.push(Todo::Answer);
            first = earliest(first, at.and_then(|a| a.waiting.as_deref()));
        }
        for a in facts.asked.iter().filter(|a| a.task == t.id) {
            if !todo.contains(&Todo::Approve) {
                todo.push(Todo::Approve);
            }
            first = earliest(first, Some(a.at.as_str()).filter(|s| !s.is_empty()));
        }
        if let Some(u) = facts.unapproved.iter().find(|u| u.task == t.id) {
            todo.push(Todo::ApproveAfterwards);
            first = earliest(first, Some(u.since.as_str()).filter(|s| !s.is_empty()));
        }
        if t.to_check() {
            todo.push(Todo::Check);
            second = earliest(second, at.and_then(|a| a.check.as_deref()));
        }
        let mut decisions = 0;
        if t.has_label(crate::labels::AI_DECISIONS) {
            todo.push(Todo::Review);
            decisions = facts.decisions.get(&t.id).map_or(0, Vec::len);
            second = earliest(second, at.and_then(|a| a.review.as_deref()));
        }
        if todo.is_empty() {
            continue;
        }
        let agent_waiting = todo.iter().any(|d| d.agent_waiting());
        let blocks = taskbag
            .tasks
            .iter()
            .filter(|o| {
                !matches!(o.state, State::Done | State::Dropped) && o.blocked_by.contains(&t.id)
            })
            .count();
        // Its iteration's place in the plan; a task in no iteration comes after them all.
        let place = t
            .iteration
            .as_deref()
            .and_then(|i| taskbag.iterations.iter().position(|x| x.id == i))
            .unwrap_or(usize::MAX);
        rows.push((
            place,
            Row {
                task: t.id.clone(),
                todo,
                decisions,
                agent_waiting,
                risk: facts.classes.get(&t.id).and_then(|c| c.risk.clone()),
                waiting_since: if agent_waiting { first } else { second },
                blocks,
            },
        ));
    }
    rows.sort_by(|(pa, a), (pb, b)| {
        // A time the history does not say comes after every known one.
        let since = |r: &Row| (r.waiting_since.is_none(), r.waiting_since.clone());
        b.agent_waiting
            .cmp(&a.agent_waiting)
            .then_with(|| {
                if a.agent_waiting {
                    Reverse(a.blocks).cmp(&Reverse(b.blocks))
                } else {
                    risk_rank(a.risk.as_deref()).cmp(&risk_rank(b.risk.as_deref()))
                }
            })
            .then_with(|| since(a).cmp(&since(b)))
            .then_with(|| pa.cmp(pb))
            .then_with(|| a.task.cmp(&b.task))
    });
    rows.into_iter().map(|(_, r)| r).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn task(id: &str, state: &str, extra: Value) -> Task {
        let mut v = json!({ "id": id, "title": id, "area": "A01", "state": state });
        for (k, x) in extra.as_object().unwrap() {
            v[k] = x.clone();
        }
        serde_json::from_value(v).unwrap()
    }

    fn bag(tasks: Vec<Task>) -> Taskbag {
        let iteration = |id: &str, order: u32| -> crate::Iteration {
            serde_json::from_value(json!({ "id": id, "name": id, "order": order })).unwrap()
        };
        Taskbag {
            project: serde_json::from_value(json!({ "format": crate::FORMAT, "id": "P", "name": "P" })).unwrap(),
            areas: Vec::new(),
            iterations: vec![iteration("I1", 1), iteration("I2", 2)],
            tasks,
        }
    }

    fn event(at: &str, id: &str, changes: Value) -> Event {
        serde_json::from_value(json!({
            "at": at, "via": "alm", "command": "x", "kind": "task", "id": id, "changes": changes
        }))
        .unwrap()
    }

    fn done_at(at: &str, id: &str) -> Event {
        event(
            at,
            id,
            json!([{ "field": "state", "from": "in_progress", "to": "done" }]),
        )
    }

    fn waiting_at(at: &str, id: &str) -> Event {
        event(
            at,
            id,
            json!([{ "field": "state", "from": "in_progress", "to": "waiting" }]),
        )
    }

    fn risk(at: &str, id: &str, level: &str) -> Event {
        event(at, id, json!([{ "field": "risk", "to": level }]))
    }

    /// The queue's task IDs for `bag` and its history, with no open question and no emergency.
    fn order(bag: &Taskbag, events: &[Event]) -> Vec<String> {
        queue(bag, events, &[], &[])
            .into_iter()
            .map(|r| r.task)
            .collect()
    }

    fn queue(bag: &Taskbag, events: &[Event], unapproved: &[Unapproved], asked: &[Asked]) -> Vec<Row> {
        let tasks: Vec<&Task> = bag.tasks.iter().collect();
        let decisions: BTreeMap<String, Vec<Decision>> = crate::decision::pending(events)
            .into_iter()
            .map(|(id, all)| (id, crate::decision::to_review(&all)))
            .collect();
        rows(
            bag,
            &tasks,
            Facts {
                times: &times(events),
                classes: &crate::classify::recorded(events),
                decisions: &decisions,
                unapproved,
                asked,
            },
        )
    }

    const REVIEW: &str = crate::labels::AI_DECISIONS;

    #[test]
    fn one_row_for_a_task_with_several_things_to_do() {
        let bag = bag(vec![
            task("A01-01", "done", json!({ "labels": [REVIEW] })),
            task("A01-02", "ready", json!({})),
        ]);
        let events = [
            done_at("2026-11-01T10:00:00Z", "A01-01"),
            event(
                "2026-11-01T09:00:00Z",
                "A01-01",
                json!([
                    { "field": "ai_decision", "to": "a name", "kind": "visible" },
                    { "field": "labels", "from": [], "to": [REVIEW] }
                ]),
            ),
            event(
                "2026-11-01T09:30:00Z",
                "A01-01",
                json!([{ "field": "ai_decision", "to": "a colour", "kind": "visible" }]),
            ),
        ];
        let rows = queue(&bag, &events, &[], &[]);
        assert_eq!(rows.len(), 1, "a task with nothing to do has no row");
        assert_eq!(rows[0].task, "A01-01");
        assert_eq!(rows[0].todo, [Todo::Check, Todo::Review]);
        assert_eq!(rows[0].decisions, 2);
        assert!(!rows[0].agent_waiting);
        // The earliest of the two: the label came before the task was finished.
        assert_eq!(rows[0].waiting_since.as_deref(), Some("2026-11-01T09:00:00Z"));
        assert_eq!(
            serde_json::to_value(&rows[0]).unwrap()["todo"],
            json!(["check", "review"])
        );
    }

    #[test]
    fn a_task_an_agent_waits_on_comes_first_whatever_the_risk() {
        let bag = bag(vec![
            task("A01-01", "done", json!({})),
            task("A01-02", "waiting", json!({})),
            task("A01-03", "in_progress", json!({})),
            task("A01-04", "in_progress", json!({})),
        ]);
        let events = [
            done_at("2026-11-01T10:00:00Z", "A01-01"),
            risk("2026-11-01T10:00:01Z", "A01-01", "high"),
            waiting_at("2026-11-05T10:00:00Z", "A01-02"),
            risk("2026-11-05T10:00:01Z", "A01-02", "low"),
        ];
        let unapproved = [Unapproved {
            task: "A01-03".into(),
            since: "2026-11-06T00:00:00Z".into(),
            due: None,
            overdue: false,
        }];
        let asked = [Asked {
            task: "A01-04".into(),
            at: "2026-11-07T00:00:00Z".into(),
        }];
        let rows = queue(&bag, &events, &unapproved, &asked);
        let ids: Vec<&str> = rows.iter().map(|r| r.task.as_str()).collect();
        assert_eq!(ids, ["A01-02", "A01-03", "A01-04", "A01-01"]);
        assert_eq!(rows[0].todo, [Todo::Answer]);
        assert_eq!(rows[1].todo, [Todo::ApproveAfterwards]);
        assert_eq!(rows[2].todo, [Todo::Approve]);
        assert_eq!(rows[2].waiting_since.as_deref(), Some("2026-11-07T00:00:00Z"));
        assert!(rows[..3].iter().all(|r| r.agent_waiting));
    }

    #[test]
    fn agents_waiting_the_one_holding_back_more_tasks_first_then_the_longer_wait() {
        let bag = bag(vec![
            task("A01-01", "waiting", json!({})),
            task("A01-02", "waiting", json!({})),
            task("A01-03", "waiting", json!({})),
            task("A01-04", "ready", json!({ "blocked_by": ["A01-03"] })),
            task("A01-05", "backlog", json!({ "blocked_by": ["A01-03", "A01-02"] })),
            // A finished task no longer counts as held back.
            task("A01-06", "dropped", json!({ "blocked_by": ["A01-01"] })),
        ]);
        let events = [
            waiting_at("2026-11-01T10:00:00Z", "A01-01"),
            waiting_at("2026-11-02T10:00:00Z", "A01-02"),
            waiting_at("2026-11-03T10:00:00Z", "A01-03"),
        ];
        let rows = queue(&bag, &events, &[], &[]);
        let seen: Vec<(&str, usize)> = rows.iter().map(|r| (r.task.as_str(), r.blocks)).collect();
        assert_eq!(seen, [("A01-03", 2), ("A01-02", 1), ("A01-01", 0)]);

        // The same number held back: the longer wait first.
        let bag = self::bag(vec![
            task("A01-01", "waiting", json!({})),
            task("A01-02", "waiting", json!({})),
        ]);
        let events = [
            waiting_at("2026-11-02T10:00:00Z", "A01-01"),
            waiting_at("2026-11-01T10:00:00Z", "A01-02"),
        ];
        assert_eq!(order(&bag, &events), ["A01-02", "A01-01"]);
    }

    #[test]
    fn finished_tasks_by_risk_then_the_longer_wait() {
        let bag = bag(vec![
            task("A01-01", "done", json!({})),
            task("A01-02", "done", json!({})),
            task("A01-03", "done", json!({})),
            task("A01-04", "done", json!({})),
            task("A01-05", "done", json!({})),
            task("A01-06", "done", json!({})),
        ]);
        let events = [
            done_at("2026-11-01T10:00:00Z", "A01-01"),
            done_at("2026-11-02T10:00:00Z", "A01-02"),
            risk("2026-11-02T10:00:01Z", "A01-02", "low"),
            done_at("2026-11-03T10:00:00Z", "A01-03"),
            risk("2026-11-03T10:00:01Z", "A01-03", "medium"),
            done_at("2026-11-05T10:00:00Z", "A01-04"),
            risk("2026-11-05T10:00:01Z", "A01-04", "high"),
            done_at("2026-11-04T10:00:00Z", "A01-05"),
            risk("2026-11-04T10:00:01Z", "A01-05", "high"),
            // A value not in the list counts as not classified; it is never guessed.
            done_at("2026-10-01T10:00:00Z", "A01-06"),
            risk("2026-10-01T10:00:01Z", "A01-06", "huge"),
        ];
        assert_eq!(
            order(&bag, &events),
            ["A01-05", "A01-04", "A01-03", "A01-02", "A01-06", "A01-01"]
        );
    }

    #[test]
    fn last_the_earlier_iteration_then_the_id() {
        // No history: nothing says how long, so the plan's order decides.
        let bag = bag(vec![
            task("A01-01", "done", json!({})),
            task("A01-02", "done", json!({ "iteration": "I2" })),
            task("A01-03", "done", json!({ "iteration": "I1" })),
            task("A01-04", "done", json!({ "iteration": "I1" })),
        ]);
        assert_eq!(order(&bag, &[]), ["A01-03", "A01-04", "A01-02", "A01-01"]);
        // A known time comes before one the history does not say.
        let events = [done_at("2026-11-01T10:00:00Z", "A01-01")];
        assert_eq!(order(&bag, &events)[0], "A01-01");
    }

    #[test]
    fn how_long_is_read_from_the_history() {
        let events = [
            waiting_at("2026-11-01T10:00:00Z", "A01-01"),
            event(
                "2026-11-02T10:00:00Z",
                "A01-01",
                json!([{ "field": "state", "from": "waiting", "to": "in_progress" }]),
            ),
            done_at("2026-11-03T10:00:00Z", "A01-01"),
            event(
                "2026-11-04T10:00:00Z",
                "A01-01",
                json!([{ "field": "labels", "from": [], "to": ["locally verified by human"] }]),
            ),
            // `alm recheck`: to check again, counted from now.
            event(
                "2026-11-05T10:00:00Z",
                "A01-01",
                json!([{ "field": "labels", "from": ["locally verified by human"], "to": [] }]),
            ),
            event(
                "2026-11-06T10:00:00Z",
                "A01-02",
                json!([{ "field": "labels", "from": [], "to": [REVIEW] }]),
            ),
            event(
                "2026-11-07T10:00:00Z",
                "A01-02",
                json!([{ "field": "labels", "from": [REVIEW], "to": [] }]),
            ),
            done_at("2026-11-08T10:00:00Z", "A01-03"),
            event(
                "2026-11-09T10:00:00Z",
                "A01-03",
                json!([{ "field": "state", "from": "done", "to": "ready" }]),
            ),
        ];
        let t = times(&events);
        assert_eq!(
            t["A01-01"],
            Times {
                waiting: None,
                check: Some("2026-11-05T10:00:00Z".into()),
                review: None
            }
        );
        assert_eq!(t["A01-02"], Times::default());
        assert_eq!(t["A01-03"], Times::default(), "reopened: no longer to check");
    }
}
