//! A session coming back (decision 0022 § 8, Task-A01-castle658): which tasks this session held
//! have since been taken over by another one. `alm` says so first, whatever the command, and
//! refuses the session's further changes to those tasks.
//!
//! The answer comes from the history: a claim that took a task over records the holder changing
//! from the old session to the new one (Task-A01-sign613). No field of its own is written.

use serde::Serialize;

use crate::event::Event;
use crate::model::{State, Task};
use crate::problem::Problem;

/// A task taken over from this session, and not given back to it since.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TakenOver {
    pub id: String,
    /// The session that took it over from this one.
    pub by: String,
    /// When, in UTC: the time of that claim.
    pub at: String,
    /// Who holds it now (the same as `by` unless it was taken over again). For a task finished
    /// since (done or dropped), the last session that held it: the one that finished it.
    pub holder: String,
    /// `done` or `dropped` when the taker has finished it since (Task-A01-heart358); absent while
    /// it is still being worked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished: Option<String>,
}

impl TakenOver {
    /// The notice `alm` gives first (a warning code, worded in every language).
    pub fn notice(&self) -> Problem {
        Problem::TakenOver {
            id: self.id.clone(),
            by: self.by.clone(),
            at: self.at.clone(),
        }
    }
}

/// The tasks `me` held that another session took over, in ID order. A task counts while someone
/// else holds it, or once the taker has finished it (done or dropped, Task-A01-heart358), and
/// `me` has not held it again since the takeover. A task released since, and so held by nobody
/// and open to anyone, does not count. `events`: the taskbag's history, oldest first.
pub fn taken_over(tasks: &[Task], events: &[Event], me: &str) -> Vec<TakenOver> {
    let mut out: Vec<TakenOver> = tasks
        .iter()
        .filter_map(|t| {
            let finished = matches!(t.state, State::Done | State::Dropped);
            if !finished && t.holder.as_deref().is_none_or(|h| h == me) {
                return None;
            }
            // `last`: who took it from `me`, and when; `latest`: who held it last since then.
            let mut last: Option<(String, String)> = None;
            let mut latest: Option<String> = None;
            for e in events
                .iter()
                .filter(|e| e.id.as_deref() == Some(t.id.as_str()))
            {
                for ch in e.changes.iter().filter(|c| c.field == "holder") {
                    let from = ch.from.as_ref().and_then(|v| v.as_str());
                    let to = ch.to.as_ref().and_then(|v| v.as_str());
                    if to == Some(me) {
                        last = None;
                        latest = None;
                    } else if from == Some(me) {
                        last = to.map(|to| (to.to_string(), e.at.clone()));
                        latest = to.map(str::to_string);
                    } else if let Some(to) = to.filter(|_| last.is_some()) {
                        latest = Some(to.to_string());
                    }
                }
            }
            let (by, at) = last?;
            let holder = match t.holder.as_deref() {
                Some(h) if h == me => return None,
                Some(h) => h.to_string(),
                None => latest.unwrap_or_else(|| by.clone()),
            };
            Some(TakenOver {
                id: t.id.clone(),
                by,
                at,
                holder,
                finished: finished.then(|| t.state.as_str().to_string()),
            })
        })
        .collect();
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn task(id: &str, state: &str, holder: Option<&str>) -> Task {
        serde_json::from_value(json!({
            "id": id, "title": "t", "area": "A01", "state": state, "holder": holder
        }))
        .unwrap()
    }

    fn moved(id: &str, at: &str, from: Option<&str>, to: Option<&str>) -> Event {
        let mut ch = serde_json::Map::new();
        ch.insert("field".into(), json!("holder"));
        if let Some(f) = from {
            ch.insert("from".into(), json!(f));
        }
        if let Some(t) = to {
            ch.insert("to".into(), json!(t));
        }
        serde_json::from_value(json!({
            "at": at, "via": "alm", "command": "claim", "kind": "task", "id": id,
            "changes": [ch]
        }))
        .unwrap()
    }

    #[test]
    fn a_task_taken_from_me_is_named_until_i_hold_it_again() {
        let tasks = [task("A01-01", "in_progress", Some("s2"))];
        let mut events = vec![
            moved("A01-01", "2026-11-09T10:00:00Z", None, Some("s1")),
            moved("A01-01", "2026-11-09T11:00:00Z", Some("s1"), Some("s2")),
        ];
        let got = taken_over(&tasks, &events, "s1");
        assert_eq!(
            got,
            vec![TakenOver {
                id: "A01-01".into(),
                by: "s2".into(),
                at: "2026-11-09T11:00:00Z".into(),
                holder: "s2".into(),
                finished: None,
            }]
        );
        assert_eq!(got[0].notice().code(), "taken_over");
        // The new holder, and a session that never held it, are told nothing.
        assert!(taken_over(&tasks, &events, "s2").is_empty());
        assert!(taken_over(&tasks, &events, "s9").is_empty());
        // A chain: s3 takes it from s2; s1 is still told who took it from s1, and who holds it.
        events.push(moved(
            "A01-01",
            "2026-11-09T12:00:00Z",
            Some("s2"),
            Some("s3"),
        ));
        let tasks = [task("A01-01", "in_progress", Some("s3"))];
        let got = taken_over(&tasks, &events, "s1");
        assert_eq!((got[0].by.as_str(), got[0].holder.as_str()), ("s2", "s3"));
        assert_eq!(taken_over(&tasks, &events, "s2")[0].by, "s3");
        // s1 takes it back later: no longer a notice for s1.
        events.push(moved(
            "A01-01",
            "2026-11-09T13:00:00Z",
            Some("s3"),
            Some("s1"),
        ));
        let tasks = [task("A01-01", "in_progress", Some("s1"))];
        assert!(taken_over(&tasks, &events, "s1").is_empty());
    }

    /// Task-A01-heart358: finishing the task does not end the takeover for the old holder.
    #[test]
    fn a_task_the_taker_finished_is_still_named() {
        let mut events = vec![
            moved("A01-01", "2026-11-09T10:00:00Z", None, Some("s1")),
            moved("A01-01", "2026-11-09T11:00:00Z", Some("s1"), Some("s2")),
        ];
        // Done keeps its holder.
        let got = taken_over(&[task("A01-01", "done", Some("s2"))], &events, "s1");
        assert_eq!(
            (
                got[0].by.as_str(),
                got[0].holder.as_str(),
                got[0].finished.as_deref()
            ),
            ("s2", "s2", Some("done"))
        );
        assert!(taken_over(&[task("A01-01", "done", Some("s2"))], &events, "s2").is_empty());
        // Dropped clears it: the last holder is named, here after a second takeover.
        events.push(moved(
            "A01-01",
            "2026-11-09T12:00:00Z",
            Some("s2"),
            Some("s3"),
        ));
        events.push(moved("A01-01", "2026-11-09T12:30:00Z", Some("s3"), None));
        let got = taken_over(&[task("A01-01", "dropped", None)], &events, "s1");
        assert_eq!(
            (
                got[0].by.as_str(),
                got[0].holder.as_str(),
                got[0].finished.as_deref()
            ),
            ("s2", "s3", Some("dropped"))
        );
        // A task s1 finished itself is no takeover.
        let own = vec![moved("A01-01", "2026-11-09T10:00:00Z", None, Some("s1"))];
        assert!(taken_over(&[task("A01-01", "done", Some("s1"))], &own, "s1").is_empty());
        assert!(taken_over(&[task("A01-01", "dropped", None)], &own, "s1").is_empty());
    }

    #[test]
    fn released_and_own_releases_are_no_takeover() {
        let events = vec![
            moved("A01-01", "2026-11-09T10:00:00Z", None, Some("s1")),
            moved("A01-01", "2026-11-09T11:00:00Z", Some("s1"), Some("s2")),
        ];
        // Released by the new holder: nobody holds it, anyone may claim it.
        assert!(taken_over(&[task("A01-01", "ready", None)], &events, "s1").is_empty());
        // s1 released it itself, then s2 claimed it as a ready task: no takeover.
        let events = vec![
            moved("A01-01", "2026-11-09T10:00:00Z", None, Some("s1")),
            moved("A01-01", "2026-11-09T10:30:00Z", Some("s1"), None),
            moved("A01-01", "2026-11-09T11:00:00Z", None, Some("s2")),
        ];
        let tasks = [task("A01-01", "in_progress", Some("s2"))];
        assert!(taken_over(&tasks, &events, "s1").is_empty());
    }
}
