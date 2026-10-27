//! What a task is missing that the app or the next session relies on (Task-A03-ham538, approver
//! 2026-10-27: a check inside `alm`). After a command that changes a task, `alm` lists each one as
//! a Fact line with what goes wrong without it and the command that fixes it. It refuses nothing:
//! `alm` tells, the agent fixes.

use allomos_core::{State, Task};
use serde::Serialize;

use allomos_core::branch::Branches;

/// One thing the task is missing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Missing {
    /// `session`, `working_branch` or `handoff_note`.
    pub code: &'static str,
    /// What is missing and what goes wrong without it, in plain words.
    pub fact: String,
    /// The command that fixes it.
    pub fix: String,
}

/// What `task` is missing, in that order: the session details the app needs to reopen the
/// session, the working branch of a task that was claimed, and the handoff note of a task
/// finished without the human check. `claimed`: the task was claimed at some point (a claim in
/// its history, or a holder now).
pub fn missing(task: &Task, branches: Option<&Branches>, claimed: bool) -> Vec<Missing> {
    let id = &task.id;
    let mut out = Vec::new();
    // 1. Session details: only for a task an agent works or worked, never for an unclaimed one.
    if claimed {
        let session = task.session.as_ref();
        let has = |v: Option<&String>| v.is_some_and(|v| !v.trim().is_empty());
        let gap = match session {
            None => Some("no session is recorded"),
            Some(s) if !has(s.id.as_ref()) => Some("its session has no session ID"),
            Some(s) if s.kind.as_deref() == Some("desktop") && !has(s.app_session_id.as_ref()) => {
                Some("its desktop app session has no app session ID")
            }
            Some(_) => None,
        };
        if let Some(gap) = gap {
            out.push(Missing {
                code: "session",
                fact: format!(
                    "Task {id}: {gap}, so the Allomos app cannot reopen the session (no Open in Claude or Resume in terminal)."
                ),
                fix: format!(
                    "alm session {id} --session-agent <your agent's id> --session-id <your session's ID> [--session-kind desktop --app-session-id <its local_… ID>]"
                ),
            });
        }
    }
    // 2. The working branch of a task that was claimed.
    if claimed
        && branches
            .and_then(|b| b.working.as_deref())
            .is_none_or(|w| w.trim().is_empty())
    {
        out.push(Missing {
            code: "working_branch",
            fact: format!(
                "Task {id}: no working branch is recorded, so the next session cannot find the work."
            ),
            fix: format!("alm claim {id} --branch <the branch you are on> [--working <the task's branch>]"),
        });
    }
    // 3. Finished without the human check, with nothing for the person checking it to read.
    if task.state == State::Done
        && task.without_human_check.is_some()
        && task
            .handoff_note
            .as_deref()
            .is_none_or(|n| n.trim().is_empty())
    {
        out.push(Missing {
            code: "handoff_note",
            fact: format!(
                "Task {id}: it is done, to check, with no handoff note, so the person checking it has no summary."
            ),
            fix: format!("alm handoff {id} --text \"<what was done, where it stands, who it waits on>\""),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use allomos_core::AgentSession;

    fn task() -> Task {
        serde_json::from_str(
            r#"{ "id": "A01-01", "title": "t", "area": "A01", "state": "in_progress" }"#,
        )
        .unwrap()
    }

    fn branch(working: &str) -> Branches {
        Branches {
            base: Some("main".into()),
            working: Some(working.into()),
        }
    }

    #[test]
    fn a_complete_task_is_missing_nothing() {
        let mut t = task();
        t.session = Some(AgentSession {
            agent: Some("claude-code".into()),
            id: Some("s1".into()),
            kind: Some("desktop".into()),
            app_session_id: Some("local_1".into()),
            ..Default::default()
        });
        assert_eq!(missing(&t, Some(&branch("claude/x")), true), vec![]);
        // Never claimed: nothing is asked of it.
        assert_eq!(missing(&task(), None, false), vec![]);
    }

    #[test]
    fn each_gap_is_named_with_its_fix() {
        let mut t = task();
        let codes = |m: Vec<Missing>| m.into_iter().map(|m| m.code).collect::<Vec<_>>();
        assert_eq!(
            codes(missing(&t, None, true)),
            ["session", "working_branch"]
        );
        // An agent but no session ID; a desktop session without its app ID.
        t.session = Some(AgentSession {
            agent: Some("claude-code".into()),
            ..Default::default()
        });
        let m = missing(&t, Some(&branch("b")), true);
        assert_eq!(codes(m.clone()), ["session"]);
        assert!(m[0].fix.starts_with("alm session A01-01"), "{}", m[0].fix);
        t.session = Some(AgentSession {
            id: Some("s1".into()),
            kind: Some("desktop".into()),
            ..Default::default()
        });
        assert!(
            missing(&t, Some(&branch("b")), true)[0]
                .fact
                .contains("app session ID")
        );
        // Done to check without a handoff note.
        t.session = Some(AgentSession {
            id: Some("s1".into()),
            ..Default::default()
        });
        t.state = State::Done;
        t.without_human_check = Some("only the hands-on check is left".into());
        assert_eq!(
            codes(missing(&t, Some(&branch("b")), true)),
            ["handoff_note"]
        );
        t.handoff_note = Some("Merged; check steps attached.".into());
        assert_eq!(missing(&t, Some(&branch("b")), true), vec![]);
    }
}
