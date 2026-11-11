//! Hand a held task to another agent (Task-A01-minnow200): a person asks in the app, and the
//! session holding the task hands it over itself: it writes its handoff, shares its work and
//! releases the task. The request is one field of `task.json`; `alm` tells the session it names,
//! and checks nothing (decision 0017; "alm tells, the agent checks").
//!
//! A request covers one change of hands. Releasing, any claim, done and dropped remove it
//! ([`crate::ops`]), and so does Cancel in the app; one that still names a former holder is
//! ignored ([`pending`]), so the new session is never asked to hand over.

use serde::{Deserialize, Serialize};

use crate::model::{Extra, State, Task, Taskbag};
use crate::ops::{self, Outcome};
use crate::problem::Problem;

/// A person's request that the holding session hand the task over, in `task.json` as
/// `handover_request`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoverRequest {
    /// The holder when the person asked: the only session this request speaks to.
    pub holder: String,
    /// Who asked: the person's saved name; absent when the app does not know it yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    /// When, in UTC (`2026-11-11T06:10:02Z`).
    pub at: String,
    /// The agent the task is to go to, as the app's agent picker names it (`claude_cli`,
    /// `codex_app`, … or `manual`).
    pub to: String,
    /// Fields this Allomos does not know, kept as they are.
    #[serde(flatten)]
    pub extra: Extra,
}

/// Whether the task is finished: done or dropped.
fn finished(task: &Task) -> bool {
    matches!(task.state, State::Done | State::Dropped)
}

/// The request still in force on `task`: the task is not finished and its holder is the session
/// the request names. `None` for a request that names a former holder.
pub fn pending(task: &Task) -> Option<&HandoverRequest> {
    task.handover_request
        .as_ref()
        .filter(|r| !finished(task) && task.holder.as_deref() == Some(r.holder.as_str()))
}

/// Ask the session holding `id` to hand it over to `to`. It replaces an earlier request. Anyone
/// may ask: the request only tells the holder, and changes nothing else about the task. Refused
/// for a finished task and for a task nobody holds.
pub fn request(
    taskbag: &Taskbag,
    id: &str,
    by: Option<&str>,
    to: &str,
    at: &str,
) -> Result<Outcome, Problem> {
    let mut task = ops::find_task_to_move(taskbag, id)?;
    if finished(&task) {
        return Err(Problem::TaskClosed {
            id: task.id.clone(),
            state: task.state.as_str().to_string(),
        });
    }
    let Some(holder) = task.holder.clone() else {
        return Err(Problem::NotHeld {
            id: task.id.clone(),
            state: task.state.as_str().to_string(),
        });
    };
    let to = to.trim();
    if to.is_empty() {
        return Err(Problem::HandoverAgentMissing);
    }
    task.handover_request = Some(HandoverRequest {
        holder,
        by: by
            .map(str::trim)
            .filter(|b| !b.is_empty())
            .map(str::to_string),
        at: at.to_string(),
        to: to.to_string(),
        extra: Extra::default(),
    });
    Ok(Outcome::plain(task))
}

/// Take the request back (the dialog's Cancel). A task with no request stays as it is.
pub fn cancel(taskbag: &Taskbag, id: &str) -> Result<Outcome, Problem> {
    let mut task = ops::find_task_to_move(taskbag, id)?;
    task.handover_request = None;
    Ok(Outcome::plain(task))
}

/// A request addressed to one session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HandoverAsked {
    pub id: String,
    /// Who asked; absent when not known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    /// When, in UTC.
    pub at: String,
    /// The agent the task is to go to.
    pub to: String,
}

impl HandoverAsked {
    /// The notice `alm` gives first (a warning code, worded in every language).
    pub fn notice(&self) -> Problem {
        Problem::HandoverRequested {
            id: self.id.clone(),
            by: self.by.clone().unwrap_or_else(|| "a person".to_string()),
            at: self.at.clone(),
            to: self.to.clone(),
        }
    }
}

/// The requests in force that name `me`, in ID order.
pub fn asked_of(tasks: &[Task], me: &str) -> Vec<HandoverAsked> {
    let mut out: Vec<HandoverAsked> = tasks
        .iter()
        .filter_map(|t| {
            let r = pending(t).filter(|r| r.holder == me)?;
            Some(HandoverAsked {
                id: t.id.clone(),
                by: r.by.clone(),
                at: r.at.clone(),
                to: r.to.clone(),
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

    const AT: &str = "2026-11-11T06:10:02Z";

    fn task(state: &str, holder: Option<&str>) -> Task {
        serde_json::from_value(json!({
            "id": "A01-01", "title": "t", "area": "A01", "state": state, "holder": holder
        }))
        .unwrap()
    }

    fn bag(task: Task) -> Taskbag {
        Taskbag {
            project: serde_json::from_value(json!({
                "format": crate::model::FORMAT, "id": "p", "name": "p"
            }))
            .unwrap(),
            areas: vec![],
            iterations: vec![],
            tasks: vec![task],
        }
    }

    fn asked(holder: &str) -> Taskbag {
        let b = bag(task("in_progress", Some(holder)));
        let mut t = request(&b, "A01-01", Some("will"), "codex_cli", AT)
            .unwrap()
            .task;
        t.description = String::new();
        bag(t)
    }

    #[test]
    fn only_the_named_holder_is_told() {
        let b = asked("s1");
        let got = asked_of(&b.tasks, "s1");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].to, "codex_cli");
        assert_eq!(got[0].by.as_deref(), Some("will"));
        assert_eq!(got[0].notice().code(), "handover_requested");
        assert!(asked_of(&b.tasks, "s2").is_empty());
    }

    #[test]
    fn a_request_naming_a_former_holder_is_ignored() {
        let mut b = asked("s1");
        b.tasks[0].holder = Some("s2".into());
        assert!(pending(&b.tasks[0]).is_none());
        assert!(asked_of(&b.tasks, "s1").is_empty());
        assert!(asked_of(&b.tasks, "s2").is_empty());
    }

    #[test]
    fn a_finished_task_tells_nobody() {
        let mut b = asked("s1");
        b.tasks[0].state = State::Done;
        assert!(asked_of(&b.tasks, "s1").is_empty());
    }

    #[test]
    fn it_needs_a_holder_an_open_task_and_an_agent() {
        let free = bag(task("ready", None));
        assert_eq!(
            request(&free, "A01-01", None, "codex_cli", AT)
                .err()
                .unwrap()
                .code(),
            "not_held"
        );
        let done = bag(task("done", Some("s1")));
        assert_eq!(
            request(&done, "A01-01", None, "codex_cli", AT)
                .err()
                .unwrap()
                .code(),
            "task_closed"
        );
        let held = bag(task("in_progress", Some("s1")));
        assert_eq!(
            request(&held, "A01-01", None, "  ", AT)
                .err()
                .unwrap()
                .code(),
            "handover_agent_missing"
        );
    }

    #[test]
    fn a_waiting_or_blocked_holder_can_be_asked() {
        for state in ["waiting", "blocked"] {
            let b = bag(task(state, Some("s1")));
            assert!(
                request(&b, "A01-01", None, "codex_cli", AT).is_ok(),
                "{state}"
            );
        }
    }

    #[test]
    fn release_claim_done_drop_and_cancel_clear_it() {
        let b = asked("s1");
        let gone = |o: Outcome| assert!(o.task.handover_request.is_none());
        gone(ops::release(&b, "A01-01", Some("s1"), Some("note")).unwrap());
        gone(ops::claim(&b, "A01-01", Some("s2"), true).unwrap());
        gone(ops::finish(&b, "A01-01", Some("s1"), None, None).unwrap());
        gone(ops::drop_task(&b, "A01-01", Some("s1")).unwrap());
        gone(cancel(&b, "A01-01").unwrap());
    }

    #[test]
    fn it_leaves_the_task_version_alone() {
        let before = bag(task("in_progress", Some("s1")));
        let after = asked("s1");
        assert_eq!(
            ops::task_version(&before.tasks[0]),
            ops::task_version(&after.tasks[0])
        );
    }
}
