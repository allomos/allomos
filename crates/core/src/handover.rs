//! A request to hand a held task over, as older Allomos wrote it into `task.json`
//! (Task-A01-minnow200's Hand over to another agent…). Since Task-A01-kind858 the app's
//! Prepare a handoff… writes nothing: the person pastes the prompt into the holding session
//! (approver, 2026-11-12). An old request is still read and kept as it is, nothing reads it any
//! more, and releasing, any claim, done and dropped remove it ([`crate::ops`]).

use serde::{Deserialize, Serialize};

use crate::model::Extra;

/// A person's request that the holding session hand the task over, in `task.json` as
/// `handover_request`. Read only: no Allomos since Task-A01-kind858 writes one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoverRequest {
    /// The holder when the person asked.
    pub holder: String,
    /// Who asked: the person's saved name; absent when the app did not know it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    /// When, in UTC (`2026-11-11T06:10:02Z`).
    pub at: String,
    /// The agent the task was to go to (`claude_cli`, `codex_app`, … or `manual`); absent from
    /// requests that name none (approver, 2026-11-12: old data still reads, new requests leave
    /// it out).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    /// Fields this Allomos does not know, kept as they are.
    #[serde(flatten)]
    pub extra: Extra,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{State, Task, Taskbag};
    use crate::ops::{self, Outcome};
    use serde_json::json;

    fn asked(holder: &str) -> Taskbag {
        let task: Task = serde_json::from_value(json!({
            "id": "A01-01", "title": "t", "area": "A01", "state": "in_progress", "holder": holder,
            "handover_request": {
                "holder": holder, "by": "will", "at": "2026-11-11T06:10:02Z", "to": "codex_cli"
            }
        }))
        .unwrap();
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

    #[test]
    fn an_old_request_still_reads_with_or_without_to() {
        let b = asked("s1");
        let r = b.tasks[0].handover_request.as_ref().expect("read");
        assert_eq!(r.to.as_deref(), Some("codex_cli"));
        let bare: HandoverRequest =
            serde_json::from_value(json!({ "holder": "s1", "at": "2026-11-11T06:10:02Z" }))
                .unwrap();
        assert_eq!(bare.to, None);
        assert!(serde_json::to_value(&bare).unwrap().get("to").is_none());
    }

    #[test]
    fn release_claim_done_and_drop_clear_it() {
        let b = asked("s1");
        let gone = |o: Outcome| assert!(o.task.handover_request.is_none());
        gone(ops::release(&b, "A01-01", Some("s1"), Some("note")).unwrap());
        gone(ops::claim(&b, "A01-01", Some("s2"), true).unwrap());
        gone(ops::finish(&b, "A01-01", Some("s1"), None, None).unwrap());
        gone(ops::drop_task(&b, "A01-01", Some("s1")).unwrap());
        assert_eq!(b.tasks[0].state, State::InProgress);
    }

    #[test]
    fn it_leaves_the_task_version_alone() {
        let with = asked("s1");
        let mut without = with.tasks[0].clone();
        without.handover_request = None;
        assert_eq!(
            ops::task_version(&with.tasks[0]),
            ops::task_version(&without)
        );
    }
}
