//! Events: the history of changes (R7, Task-A01-17).
//!
//! Every change Allomos makes to a task, an area, an iteration or the project is also written
//! down as one event: what changed, from what to what, who did it and when. History, and later
//! the burndown and the timeline, are computed from events instead of guessed.
//!
//! This module only builds and checks events; `crates/store` names their files and writes them.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// What an event is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Task,
    Area,
    Iteration,
    Project,
    /// A kind this Allomos does not know, perhaps from a newer one (Task-A01-piano403); the value
    /// is kept in the event's `extra`.
    #[serde(other)]
    Unknown,
}

impl Kind {
    pub fn is_unknown(&self) -> bool {
        *self == Kind::Unknown
    }
}

/// One field's change. `from` is left out when the field was empty (or the item new), `to` when
/// it was emptied. Fields it does not know are kept, as for [`Event`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Change {
    pub field: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<Value>,
    /// Kept as they are, so packing a month writes them back (Task-A01-piano403).
    #[serde(flatten)]
    pub extra: crate::model::Extra,
}

/// One change to one item, as stored in `events/<name>.json`.
///
/// Fields this Allomos does not know are ignored rather than refused (Task-A01-55, approver
/// 2026-10-19): events are written only by `alm` and the app, never by hand, so an unknown field
/// means a newer Allomos added it, and refusing it would only drop that change from the history.
/// Since Task-A01-piano403 they are kept in `extra`, so packing a month writes them back.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    /// When, in UTC: `2026-10-13T14:02:05Z`.
    pub at: String,
    /// Who: the `--as` name of an agent session or person. Left out when nobody said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    /// Which writer made the change: `alm` or `app`.
    pub via: String,
    /// The command or action, for example `claim` or `add task`.
    pub command: String,
    #[serde(skip_serializing_if = "Kind::is_unknown")]
    pub kind: Kind,
    /// The item's ID; for the project, the project's ID (left out when unknown).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Which AI agent ran `alm` (Task-A03-27, approver 2026-10-18): `claude` or `codex`, judged
    /// from the environment `alm` runs in ([`agent_from_env`]). Left out when unsure, for the
    /// app's changes, and in events recorded before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// The agent's tool, when the environment tells it: `claude_cli` (Claude Code in a terminal)
    /// or `claude_app` (the Claude app), as the app names its agent tools.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    /// Who ran it, beyond `agent` and `tool` (Task-A01-black004, decision 0021 § 1.1): for an
    /// agent, its session ID and model; for a person, their pseudonym ID. Left out in events
    /// recorded before, and when nothing about it was known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<EventActor>,
    /// On whose behalf an agent made the change: the pseudonym ID of the person it works for
    /// (decision 0021 § 2). Left out for a person's own change and in events recorded before.
    #[serde(default, rename = "for", skip_serializing_if = "Option::is_none")]
    pub on_behalf_of: Option<String>,
    /// The approval ID the change relies on, when it needed one and the writer named it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval: Option<String>,
    pub changes: Vec<Change>,
    /// Fields this Allomos does not know (and an unknown `kind`'s value), kept as they are.
    #[serde(flatten)]
    pub extra: crate::model::Extra,
}

/// Who ran a change, beyond the event's `agent` and `tool` (Task-A01-black004, decision 0021
/// § 1.1). Every field is optional: what the writer could not learn is left out, never guessed.
/// Fields this Allomos does not know (later tasks add more, such as the effort) are kept.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EventActor {
    /// The agent's own session ID (Claude Code's `CLAUDE_CODE_SESSION_ID`, Codex's
    /// `CODEX_THREAD_ID`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    /// The exact model ID the agent ran on, as the agent named it (`--model`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// A person's pseudonym ID (`p-7k3q2f`), for a change a person made: in the app, or with
    /// `alm` where no AI agent was seen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub person: Option<String>,
    #[serde(flatten)]
    pub extra: crate::model::Extra,
}

impl EventActor {
    pub fn is_empty(&self) -> bool {
        self.session.is_none()
            && self.model.is_none()
            && self.person.is_none()
            && self.extra.is_empty()
    }
}

/// The session ID of the agent `agent_from_env` found, from the same environment: Claude Code's
/// `CLAUDE_CODE_SESSION_ID`, Codex's `CODEX_THREAD_ID`.
pub fn session_from_env(agent: &str, var: impl Fn(&str) -> Option<String>) -> Option<String> {
    let name = match agent {
        "claude" => "CLAUDE_CODE_SESSION_ID",
        "codex" => "CODEX_THREAD_ID",
        _ => return None,
    };
    var(name)
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// The agent an `alm` command runs under, judged from its environment (`var` reads one variable):
/// Claude Code sets `CLAUDECODE=1`, and `CLAUDE_CODE_ENTRYPOINT` says whether it runs in a
/// terminal (`cli`) or in the Claude app (`claude-desktop`); Codex sets `CODEX_THREAD_ID` (or
/// `CODEX_SANDBOX` / `CODEX_SANDBOX_NETWORK_DISABLED` in its sandbox) in the commands it runs.
/// Signs of both, or of neither: `None` (unsure, nothing recorded).
pub fn agent_from_env(var: impl Fn(&str) -> Option<String>) -> Option<(String, Option<String>)> {
    let set = |name: &str| var(name).is_some_and(|v| !v.is_empty());
    let claude = var("CLAUDECODE").as_deref() == Some("1");
    let codex = [
        "CODEX_THREAD_ID",
        "CODEX_SANDBOX",
        "CODEX_SANDBOX_NETWORK_DISABLED",
    ]
    .into_iter()
    .any(set);
    match (claude, codex) {
        (true, false) => {
            let tool = match var("CLAUDE_CODE_ENTRYPOINT").as_deref() {
                Some("cli") => Some("claude_cli".to_string()),
                Some("claude-desktop") => Some("claude_app".to_string()),
                _ => None,
            };
            Some(("claude".to_string(), tool))
        }
        (false, true) => Some(("codex".to_string(), None)),
        _ => None,
    }
}

/// The order a task's fields are listed in an event, as the task pane shows them.
pub const TASK_FIELDS: &[&str] = &[
    "title",
    "area",
    "iteration",
    "folders",
    "state",
    "labels",
    "holder",
    "session",
    "waiting_on",
    "who",
    "blocked_by",
    "next_dependency",
    "handoff_note",
    "without_human_check",
    "human_check_steps",
    // The note's name before allomos/4: events recorded then still name it.
    "finished_without_owner",
    "description",
];

fn object<T: Serialize>(value: &T) -> Map<String, Value> {
    match serde_json::to_value(value) {
        Ok(Value::Object(map)) => map,
        _ => Map::new(),
    }
}

fn present(value: Option<&Value>) -> Option<Value> {
    match value {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) if s.is_empty() => None,
        Some(Value::Array(a)) if a.is_empty() => None,
        Some(v) => Some(v.clone()),
    }
}

/// The fields that differ between `before` (`None`: the item is new) and `after`, listed in
/// `order` first and then by name. A new item's `id` is not listed: the event names the item.
/// A changed `id` (the project's, or an iteration's with `alm rename iteration`) is.
pub fn changes<T: Serialize>(before: Option<&T>, after: &T, order: &[&str]) -> Vec<Change> {
    let old = before.map(object).unwrap_or_default();
    let new = object(after);
    let mut fields: Vec<&str> = order.to_vec();
    let mut rest: Vec<&str> = old
        .keys()
        .chain(new.keys())
        .map(String::as_str)
        .filter(|k| !order.contains(k))
        .collect();
    rest.sort_unstable();
    rest.dedup();
    fields.extend(rest);
    fields
        .into_iter()
        .filter(|f| before.is_some() || *f != "id")
        .filter_map(|field| {
            let from = present(old.get(field));
            let to = present(new.get(field));
            (from != to).then(|| Change {
                extra: Default::default(),
                field: field.to_string(),
                from,
                to,
            })
        })
        .collect()
}

/// The fields of an item that is deleted (Task-A01-29): every field it had, with only its `from`,
/// in `order` first and then by name. Like a new item, its `id` is not listed: the event names it.
pub fn removed<T: Serialize>(before: &T, order: &[&str]) -> Vec<Change> {
    let old = object(before);
    let mut fields: Vec<&str> = order.to_vec();
    let mut rest: Vec<&str> = old
        .keys()
        .map(String::as_str)
        .filter(|k| !order.contains(k))
        .collect();
    rest.sort_unstable();
    fields.extend(rest);
    fields
        .into_iter()
        .filter(|f| *f != "id")
        .filter_map(|field| {
            present(old.get(field)).map(|from| Change {
                extra: Default::default(),
                field: field.to_string(),
                from: Some(from),
                to: None,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{State, Task};

    fn task() -> Task {
        Task {
            folders: Default::default(),
            session: None,
            extra: Default::default(),
            id: "A01-01".into(),
            title: "First".into(),
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
            description: String::new(),
        }
    }

    #[test]
    fn lists_only_what_changed_in_task_order() {
        let before = task();
        let mut after = task();
        after.holder = Some("claude/x".into());
        after.state = State::InProgress;
        let c = changes(Some(&before), &after, TASK_FIELDS);
        assert_eq!(
            c,
            vec![
                Change {
                    extra: Default::default(),
                    field: "state".into(),
                    from: Some("ready".into()),
                    to: Some("in_progress".into()),
                },
                Change {
                    extra: Default::default(),
                    field: "holder".into(),
                    from: None,
                    to: Some("claude/x".into()),
                },
            ]
        );
        assert!(changes(Some(&before), &before, TASK_FIELDS).is_empty());
    }

    #[test]
    fn a_new_item_lists_its_fields_but_not_its_id() {
        let c = changes(None, &task(), TASK_FIELDS);
        let fields: Vec<&str> = c.iter().map(|c| c.field.as_str()).collect();
        assert_eq!(fields, ["title", "area", "state"]);
        assert!(c.iter().all(|c| c.from.is_none()));
    }

    #[test]
    fn a_deleted_item_lists_what_it_had() {
        let c = removed(&task(), TASK_FIELDS);
        let fields: Vec<&str> = c.iter().map(|c| c.field.as_str()).collect();
        assert_eq!(fields, ["title", "area", "state"]);
        assert!(c.iter().all(|c| c.from.is_some() && c.to.is_none()));
    }

    #[test]
    fn stores_without_empty_parts() {
        let e = Event {
            extra: Default::default(),
            at: "2026-10-13T14:02:05Z".into(),
            by: None,
            via: "alm".into(),
            command: "release".into(),
            kind: Kind::Task,
            id: Some("A01-01".into()),
            agent: None,
            tool: None,
            actor: None,
            on_behalf_of: None,
            approval: None,
            changes: vec![Change {
                extra: Default::default(),
                field: "holder".into(),
                from: Some("claude/x".into()),
                to: None,
            }],
        };
        let v = serde_json::to_value(&e).unwrap();
        assert_eq!(
            v,
            serde_json::json!({
                "at": "2026-10-13T14:02:05Z", "via": "alm", "command": "release",
                "kind": "task", "id": "A01-01",
                "changes": [{ "field": "holder", "from": "claude/x" }]
            })
        );
        let back: Event = serde_json::from_value(v).unwrap();
        assert_eq!(back, e);
    }

    #[test]
    fn an_event_records_the_agent_and_its_tool_when_known() {
        let e: Event = serde_json::from_value(serde_json::json!({
            "at": "2026-10-18T09:00:00Z", "via": "alm", "command": "claim", "kind": "task",
            "id": "A01-01", "agent": "claude", "tool": "claude_cli", "changes": []
        }))
        .unwrap();
        assert_eq!(e.agent.as_deref(), Some("claude"));
        assert_eq!(e.tool.as_deref(), Some("claude_cli"));
        // An older event has neither, and is stored back without them.
        let old: Event = serde_json::from_value(serde_json::json!({
            "at": "2026-10-13T14:02:05Z", "via": "alm", "command": "claim", "kind": "task",
            "changes": []
        }))
        .unwrap();
        assert_eq!((old.agent.as_deref(), old.tool.as_deref()), (None, None));
        assert!(serde_json::to_value(&old).unwrap().get("agent").is_none());
    }

    #[test]
    fn an_event_reads_with_fields_it_does_not_know() {
        let text = r#"{"at":"2026-10-19T14:00:00Z","by":"s","via":"alm","command":"label",
            "kind":"task","id":"A01-01","later":{"x":1},
            "changes":[{"field":"label","to":"implemented","note":"?"}]}"#;
        let e = crate::parse_event("events/e.json", text).unwrap();
        assert_eq!(e.command, "label");
        assert_eq!(e.changes[0].to.as_ref().unwrap(), "implemented");
        // A required field missing is still a problem, naming the field.
        let err = crate::parse_event(
            "events/e.json",
            r#"{"at":"2026-10-19T14:00:00Z","command":"label","kind":"task","changes":[]}"#,
        )
        .unwrap_err();
        assert_eq!(err.field.as_deref(), Some("via"));
        assert_eq!(err.problem, crate::Problem::FieldMissing);
        // So is a wrong kind of value, or text that is not JSON.
        assert!(crate::parse_event("events/e.json", r#"{"at":1}"#).is_err());
        assert!(crate::parse_event("events/e.json", "not json").is_err());
    }

    fn env<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| {
            vars.iter()
                .find(|(n, _)| *n == name)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn the_agent_is_judged_from_the_environment() {
        let seen = |vars: &[(&str, &str)]| agent_from_env(env(vars));
        let claude = |tool: Option<&str>| Some(("claude".to_string(), tool.map(String::from)));
        assert_eq!(
            seen(&[("CLAUDECODE", "1"), ("CLAUDE_CODE_ENTRYPOINT", "cli")]),
            claude(Some("claude_cli"))
        );
        assert_eq!(
            seen(&[
                ("CLAUDECODE", "1"),
                ("CLAUDE_CODE_ENTRYPOINT", "claude-desktop")
            ]),
            claude(Some("claude_app"))
        );
        assert_eq!(
            seen(&[("CLAUDECODE", "1"), ("CLAUDE_CODE_ENTRYPOINT", "sdk-ts")]),
            claude(None)
        );
        assert_eq!(seen(&[("CLAUDECODE", "1")]), claude(None));
        let codex = Some(("codex".to_string(), None));
        assert_eq!(seen(&[("CODEX_THREAD_ID", "t1")]), codex);
        assert_eq!(seen(&[("CODEX_SANDBOX", "seatbelt")]), codex);
        assert_eq!(seen(&[("CODEX_SANDBOX_NETWORK_DISABLED", "1")]), codex);
        // Unsure: nothing recorded.
        assert_eq!(seen(&[]), None);
        assert_eq!(seen(&[("CLAUDECODE", "0")]), None);
        assert_eq!(seen(&[("CODEX_THREAD_ID", "")]), None);
        assert_eq!(
            seen(&[("CLAUDECODE", "1"), ("CODEX_THREAD_ID", "t1")]),
            None
        );
    }

    /// Task-A01-black004: the new fields are optional; an event without them reads as before,
    /// and one with them keeps them (with `for` under that name) when written back.
    #[test]
    fn who_fields_are_optional_and_round_trip() {
        let old: Event = serde_json::from_str(
            r#"{"at":"2026-10-13T14:02:05Z","via":"alm","command":"claim","kind":"task","id":"A01-01","changes":[]}"#,
        )
        .unwrap();
        assert!(old.actor.is_none() && old.on_behalf_of.is_none() && old.approval.is_none());
        assert!(!serde_json::to_string(&old).unwrap().contains("actor"));
        let text = r#"{"at":"2026-11-08T10:00:00Z","via":"alm","command":"label","kind":"task","id":"A01-01","agent":"claude","actor":{"session":"s","model":"m","effort":"high"},"for":"p-abc123","approval":"ap-1","changes":[]}"#;
        let new: Event = serde_json::from_str(text).unwrap();
        let actor = new.actor.clone().unwrap();
        assert_eq!(
            (actor.session.as_deref(), actor.model.as_deref()),
            (Some("s"), Some("m"))
        );
        assert_eq!(
            actor.extra.get("effort"),
            Some(&Value::from("high")),
            "kept"
        );
        assert_eq!(new.on_behalf_of.as_deref(), Some("p-abc123"));
        let back: Value = serde_json::to_value(&new).unwrap();
        assert_eq!(back, serde_json::from_str::<Value>(text).unwrap());
    }

    #[test]
    fn the_session_comes_from_the_agents_own_variable() {
        let env = |name: &str| match name {
            "CLAUDE_CODE_SESSION_ID" => Some("c1".to_string()),
            "CODEX_THREAD_ID" => Some(" ".to_string()),
            _ => None,
        };
        assert_eq!(session_from_env("claude", env).as_deref(), Some("c1"));
        assert_eq!(session_from_env("codex", env), None);
        assert_eq!(session_from_env("other", env), None);
    }
}
