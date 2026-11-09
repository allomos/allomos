//! Approvals as records (Task-A01-coach782, decision 0021 § 1.1 "Approvals").
//!
//! A person's approval used to be a line of prose in a task's description. Now `alm approve` (and
//! `alm confirm --task` on Go ahead) writes an event, command `approve`, for each task the approval
//! covers, with one change, `approval`, whose `to` is the record: who approved (a pseudonym ID),
//! the scope (the tasks and the SHA-256 fingerprint of the exact plan text approved), when, through
//! which channel (`chat`, `app`, `confirm`), the person's own words, and when it expires.
//! Withdrawing it (`alm withdraw`) is an event, command `withdraw`, whose `approval` change has
//! only a `from`: the record withdrawn.
//!
//! Whether an approval still applies is worked out from the events alone, so `task.json` gains no
//! field and the format does not change: withdrawn, expired, or a plan that is no longer the one
//! approved (the tasks' descriptions changed since) each make it no longer apply. `alm` lays these
//! facts out; deciding what to do about them is the agent's.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::event::{Change, Event, Kind};
use crate::ops::find_task;
use crate::{Problem, State, Task, Taskbag};

/// The field an `approve` or `withdraw` event changes.
pub const FIELD: &str = "approval";
/// The command that records an approval.
pub const APPROVE: &str = "approve";
/// The command that withdraws one.
pub const WITHDRAW: &str = "withdraw";
/// The plan is the covered tasks' descriptions (the default).
pub const PLAN_DESCRIPTION: &str = "description";

/// What an approval covers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scope {
    /// The task IDs it covers, in the order given.
    pub tasks: Vec<String>,
    /// The SHA-256 of the exact plan text approved, in lowercase hex.
    pub plan_sha256: String,
    /// Where the plan text came from: `description` (the tasks' descriptions, see [`plan_text`])
    /// or the name of the file shown (`alm approve --plan`, `alm confirm`). Only a description
    /// plan can be compared again later.
    #[serde(default = "description_plan")]
    pub plan: String,
    #[serde(flatten)]
    pub extra: crate::model::Extra,
}

fn description_plan() -> String {
    PLAN_DESCRIPTION.to_string()
}

/// One approval as recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Approval {
    /// `ap-` and six letters or digits.
    pub id: String,
    /// The pseudonym ID of the person who approved (never a name or an email address).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approver: Option<String>,
    pub scope: Scope,
    /// When it was given (recorded), in UTC.
    pub at: String,
    /// `chat` (the person's words in the conversation), `app` (a button in the Allomos app) or
    /// `confirm` (Go ahead on `alm confirm`).
    pub channel: String,
    /// The person's own words, as they wrote them (or the button they chose).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub words: Option<String>,
    /// The last day it holds (`YYYY-MM-DD`, through the end of that day in UTC), when it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires: Option<String>,
    #[serde(flatten)]
    pub extra: crate::model::Extra,
}

/// Where an approval stands now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Not withdrawn, not expired, and the plan is still the one approved (or a file plan alm
    /// cannot compare).
    Applies,
    /// The tasks' descriptions changed since: the plan approved is no longer the plan.
    PlanChanged,
    /// Past its `expires` day.
    Expired,
    /// Withdrawn.
    Withdrawn,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Applies => "applies",
            Status::PlanChanged => "plan_changed",
            Status::Expired => "expired",
            Status::Withdrawn => "withdrawn",
        }
    }
}

/// An approval with what happened to it since.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Recorded {
    #[serde(flatten)]
    pub approval: Approval,
    /// Who recorded it (`--as`), when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded_by: Option<String>,
    /// When it was withdrawn, if it was.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub withdrawn_at: Option<String>,
    /// Who withdrew it (`--as`), when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub withdrawn_by: Option<String>,
    pub status: Status,
}

/// The plan text of `tasks` for an approval: one task's description as it is; several tasks'
/// descriptions each under a `## <id>` line, joined by blank lines.
pub fn plan_text(taskbag: &Taskbag, tasks: &[String]) -> Result<String, Problem> {
    if let [one] = tasks {
        return Ok(find_task(taskbag, one)?.description);
    }
    let mut parts = Vec::new();
    for id in tasks {
        parts.push(format!(
            "## {id}\n\n{}",
            find_task(taskbag, id)?.description
        ));
    }
    Ok(parts.join("\n\n"))
}

/// The SHA-256 of `text`, in lowercase hex.
pub fn sha256(text: &str) -> String {
    use sha2::Digest;
    use std::fmt::Write;
    sha2::Sha256::digest(text.as_bytes())
        .iter()
        .fold(String::new(), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
}

/// What `alm approve` is given.
#[derive(Debug, Clone, Default)]
pub struct Given<'a> {
    pub id: &'a str,
    pub tasks: &'a [String],
    pub approver: Option<&'a str>,
    pub channel: &'a str,
    pub words: Option<&'a str>,
    pub expires: Option<&'a str>,
    /// The plan shown, as `(file name, its text)`; `None`: the tasks' descriptions.
    pub plan: Option<(&'a str, &'a str)>,
    pub at: &'a str,
}

/// The approval `given` describes, and the change recording it. Refused for no task or an
/// unknown one, an unknown channel, an expiry that is not a date, and an approval in the
/// conversation without the person's words (`approval_words_missing`).
pub fn approve(taskbag: &Taskbag, given: &Given) -> Result<(Approval, Change), Problem> {
    if given.tasks.is_empty() {
        return Err(Problem::BadUsage {
            detail: "name at least one task the approval covers".into(),
        });
    }
    let mut tasks: Vec<String> = Vec::new();
    for id in given.tasks {
        let id = find_task(taskbag, id)?.id;
        if !tasks.contains(&id) {
            tasks.push(id);
        }
    }
    if !matches!(given.channel, "chat" | "app" | "confirm") {
        return Err(Problem::BadUsage {
            detail: format!(
                "the channel is chat, app or confirm, not \"{}\"",
                given.channel
            ),
        });
    }
    let words = given.words.map(str::trim).filter(|w| !w.is_empty());
    if given.channel == "chat" && words.is_none() {
        return Err(Problem::ApprovalWordsMissing);
    }
    let expires = given.expires.map(str::trim).filter(|e| !e.is_empty());
    if let Some(e) = expires.filter(|e| !crate::validate::valid_date(e)) {
        return Err(Problem::InvalidDate {
            value: e.to_string(),
        });
    }
    let (plan, text) = match given.plan {
        Some((name, text)) => (name.to_string(), text.to_string()),
        None => (PLAN_DESCRIPTION.to_string(), plan_text(taskbag, &tasks)?),
    };
    let approval = Approval {
        id: given.id.to_string(),
        approver: given.approver.map(String::from),
        scope: Scope {
            tasks,
            plan_sha256: sha256(&text),
            plan,
            extra: Default::default(),
        },
        at: given.at.to_string(),
        channel: given.channel.to_string(),
        words: words.map(String::from),
        expires: expires.map(String::from),
        extra: Default::default(),
    };
    let change = Change {
        extra: Default::default(),
        field: FIELD.to_string(),
        from: None,
        to: Some(serde_json::to_value(&approval).expect("approvals serialize")),
    };
    Ok((approval, change))
}

/// The change withdrawing `approval`: only a `from`.
pub fn withdraw(approval: &Approval) -> Change {
    Change {
        extra: Default::default(),
        field: FIELD.to_string(),
        from: Some(serde_json::to_value(approval).expect("approvals serialize")),
        to: None,
    }
}

/// Every approval in `events` (oldest first), by ID, in the order given, with where it stands
/// on `today` (`YYYY-MM-DD`, UTC) against the taskbag as it is now. An `approval` change whose
/// value is not a record (the older `approved` events of `alm confirm`, a plain text) is left out.
pub fn all(events: &[Event], taskbag: &Taskbag, today: &str) -> Vec<Recorded> {
    let mut out: Vec<Recorded> = Vec::new();
    for e in events {
        if e.kind != Kind::Task {
            continue;
        }
        for c in e.changes.iter().filter(|c| c.field == FIELD) {
            match (&c.to, &c.from) {
                (Some(to), _) => {
                    let Ok(a) = serde_json::from_value::<Approval>(to.clone()) else {
                        continue;
                    };
                    if out.iter().all(|r| r.approval.id != a.id) {
                        out.push(Recorded {
                            approval: a,
                            recorded_by: e.by.clone(),
                            withdrawn_at: None,
                            withdrawn_by: None,
                            status: Status::Applies,
                        });
                    }
                }
                (None, Some(from)) => {
                    let id = match from {
                        Value::String(s) => Some(s.clone()),
                        other => other.get("id").and_then(Value::as_str).map(String::from),
                    };
                    if let Some(r) = out
                        .iter_mut()
                        .find(|r| Some(&r.approval.id) == id.as_ref() && r.withdrawn_at.is_none())
                    {
                        r.withdrawn_at = Some(e.at.clone());
                        r.withdrawn_by = e.by.clone();
                    }
                }
                _ => {}
            }
        }
    }
    for r in &mut out {
        r.status = status(r, taskbag, today);
    }
    out
}

fn status(r: &Recorded, taskbag: &Taskbag, today: &str) -> Status {
    if r.withdrawn_at.is_some() {
        return Status::Withdrawn;
    }
    if r.approval.expires.as_deref().is_some_and(|e| today > e) {
        return Status::Expired;
    }
    let scope = &r.approval.scope;
    if scope.plan == PLAN_DESCRIPTION {
        match plan_text(taskbag, &scope.tasks) {
            Ok(text) if sha256(&text) == scope.plan_sha256 => {}
            _ => return Status::PlanChanged,
        }
    }
    Status::Applies
}

/// The withdrawn approval to point out on `task` (approver, 2026-11-08, Task-A01-baker800): while
/// the task is in progress, its latest approval (of `approvals`, oldest first), when that one was
/// withdrawn. Withdrawing stops nothing: `alm` and the app say so, and the task's holder decides
/// whether to stop.
pub fn withdrawn_notice<'a>(task: &Task, approvals: &'a [Recorded]) -> Option<&'a Recorded> {
    if task.state != State::InProgress {
        return None;
    }
    approvals.last().filter(|r| r.status == Status::Withdrawn)
}

/// [`all`], by task ID: each approval under every task it covers. Tasks with none are left out.
pub fn by_task(
    events: &[Event],
    taskbag: &Taskbag,
    today: &str,
) -> BTreeMap<String, Vec<Recorded>> {
    let mut out: BTreeMap<String, Vec<Recorded>> = BTreeMap::new();
    for r in all(events, taskbag, today) {
        for id in &r.approval.scope.tasks {
            out.entry(id.clone()).or_default().push(r.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Area, Project, State, Task};

    fn task(id: &str, description: &str) -> Task {
        Task {
            folders: Default::default(),
            session: None,
            heartbeat: None,
            extra: Default::default(),
            id: id.into(),
            title: "T".into(),
            area: "A01".into(),
            state: State::Ready,
            labels: Vec::new(),
            holder: None,
            waiting_on: None,
            who: None,
            blocked_by: Vec::new(),
            next_dependency: None,
            iteration: None,
            handoff_note: None,
            without_human_check: None,
            human_check_steps: None,
            description: description.into(),
        }
    }

    fn bag(description: &str) -> Taskbag {
        Taskbag {
            project: serde_json::from_value::<Project>(serde_json::json!({
                "format": crate::FORMAT, "id": "P", "name": "P", "labels": ["implemented"]
            }))
            .unwrap(),
            areas: vec![Area {
                extra: Default::default(),
                id: "A01".into(),
                name: "A".into(),
                order: 0,
            }],
            iterations: Vec::new(),
            tasks: vec![task("A01-01", description), task("A01-02", "other")],
        }
    }

    fn event(command: &str, id: &str, at: &str, changes: Vec<Change>) -> Event {
        Event {
            extra: Default::default(),
            at: at.into(),
            by: Some("claude/x".into()),
            via: "alm".into(),
            command: command.into(),
            kind: Kind::Task,
            id: Some(id.into()),
            agent: None,
            tool: None,
            actor: None,
            on_behalf_of: None,
            approval: None,
            changes,
        }
    }

    fn given(tasks: &[String]) -> Given<'_> {
        Given {
            id: "ap-abc123",
            tasks,
            approver: Some("p-7k3q2f"),
            channel: "chat",
            words: Some(" go ahead "),
            expires: None,
            plan: None,
            at: "2026-11-08T10:00:00Z",
        }
    }

    #[test]
    fn sha256_is_the_standard_one() {
        assert_eq!(
            sha256("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn an_approval_records_who_what_when_how_and_until() {
        let tasks = vec!["A01-01".to_string()];
        let (a, c) = approve(&bag("the plan"), &given(&tasks)).unwrap();
        assert_eq!(a.scope.plan_sha256, sha256("the plan"));
        assert_eq!(a.scope.plan, "description");
        assert_eq!(a.words.as_deref(), Some("go ahead"));
        let to = c.to.unwrap();
        assert_eq!(to["approver"], "p-7k3q2f");
        assert_eq!(to["channel"], "chat");
        assert_eq!(to["scope"]["tasks"][0], "A01-01");
        assert!(c.from.is_none());

        let mut g = given(&tasks);
        g.words = Some("  ");
        assert_eq!(approve(&bag("p"), &g), Err(Problem::ApprovalWordsMissing));
        g.channel = "app";
        assert!(approve(&bag("p"), &g).is_ok(), "a button needs no words");
        g.channel = "email";
        assert!(matches!(
            approve(&bag("p"), &g),
            Err(Problem::BadUsage { .. })
        ));
        let mut g = given(&tasks);
        g.expires = Some("next week");
        assert!(matches!(
            approve(&bag("p"), &g),
            Err(Problem::InvalidDate { .. })
        ));
        let unknown = vec!["A09-09".to_string()];
        assert!(matches!(
            approve(&bag("p"), &given(&unknown)),
            Err(Problem::TaskNotFound { .. })
        ));
    }

    #[test]
    fn a_changed_plan_expiry_and_withdrawal_end_it() {
        let tasks = vec!["A01-01".to_string(), "A01-02".to_string()];
        let mut g = given(&tasks);
        g.expires = Some("2026-11-30");
        let (a, c) = approve(&bag("the plan"), &g).unwrap();
        let mut events = vec![
            event(APPROVE, "A01-01", "2026-11-08T10:00:00Z", vec![c.clone()]),
            event(APPROVE, "A01-02", "2026-11-08T10:00:00Z", vec![c]),
        ];
        let now = |bag: &Taskbag, events: &[Event], today: &str| {
            all(events, bag, today)
                .iter()
                .map(|r| r.status)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            now(&bag("the plan"), &events, "2026-11-08"),
            [Status::Applies]
        );
        assert_eq!(by_task(&events, &bag("the plan"), "2026-11-08").len(), 2);
        assert_eq!(
            now(&bag("the plan, changed"), &events, "2026-11-08"),
            [Status::PlanChanged]
        );
        assert_eq!(
            now(&bag("the plan"), &events, "2026-11-30"),
            [Status::Applies]
        );
        assert_eq!(
            now(&bag("the plan"), &events, "2026-12-01"),
            [Status::Expired]
        );

        events.push(event(
            WITHDRAW,
            "A01-01",
            "2026-11-09T09:00:00Z",
            vec![withdraw(&a)],
        ));
        let r = &all(&events, &bag("the plan"), "2026-11-09")[0];
        assert_eq!(r.status, Status::Withdrawn);
        assert_eq!(r.withdrawn_at.as_deref(), Some("2026-11-09T09:00:00Z"));
    }

    #[test]
    fn a_file_plan_is_not_compared_and_older_approval_texts_are_left_out() {
        let tasks = vec!["A01-01".to_string()];
        let mut g = given(&tasks);
        g.plan = Some(("plan.md", "shown text"));
        let (a, c) = approve(&bag("x"), &g).unwrap();
        assert_eq!(a.scope.plan_sha256, sha256("shown text"));
        let old = Change {
            extra: Default::default(),
            field: FIELD.into(),
            from: None,
            to: Some(Value::String("Deploy (plan 0123abcd)".into())),
        };
        let events = vec![
            event("approved", "A01-01", "2026-10-01T00:00:00Z", vec![old]),
            event(APPROVE, "A01-01", "2026-11-08T10:00:00Z", vec![c]),
        ];
        let all = all(&events, &bag("changed"), "2026-11-08");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].status, Status::Applies);
    }
}
