//! One-time numbers for changes (Task-A01-58, approved 2026-10-20): an agent may attach a
//! number of its own (`alm --request-id <id>`) to a change. If its session crashes and it runs
//! the same command again with the same number, Allomos finds the first run in the history and
//! returns its result instead of doing the change twice; the same number with a different input
//! is refused.
//!
//! Like the branches (Task-A03-22), the number lives only in the history: each event the change
//! wrote carries one more change named `request_id`, whose `to` holds the number and a
//! fingerprint of the command's input. `task.json` gains no field, older Allomos builds read the
//! events like any other, and the format stays `allomos/4`.

use serde_json::{Value, json};

use crate::event::{Change, Event};

/// The field that records a change's one-time number.
pub const FIELD: &str = "request_id";

/// A short fingerprint of `input` (FNV-1a, 64 bits): the same on every computer.
pub fn fingerprint(input: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in input.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// The change that marks an event as made under the number `id`, for the input `input`
/// (a [`fingerprint`]).
pub fn change(id: &str, input: &str) -> Change {
    Change {
        extra: Default::default(),
        field: FIELD.to_string(),
        from: None,
        to: Some(json!({ "id": id, "input": input })),
    }
}

/// The number and input fingerprint an event was made under, if any.
pub fn of(event: &Event) -> Option<(&str, &str)> {
    event
        .changes
        .iter()
        .find(|c| c.field == FIELD)
        .and_then(|c| {
            let to = c.to.as_ref()?;
            Some((to.get("id")?.as_str()?, to.get("input")?.as_str()?))
        })
}

/// The events made under the number `id`, oldest first.
pub fn made_under<'a>(events: &'a [Event], id: &str) -> Vec<&'a Event> {
    events
        .iter()
        .filter(|e| of(e).is_some_and(|(n, _)| n == id))
        .collect()
}

/// A value for JSON results: what the first run did.
pub fn summary(events: &[&Event]) -> Value {
    json!(
        events
            .iter()
            .map(|e| json!({ "at": e.at, "command": e.command, "kind": e.kind, "id": e.id }))
            .collect::<Vec<_>>()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Kind;

    fn event(changes: Vec<Change>) -> Event {
        Event {
            extra: Default::default(),
            at: "2026-10-20T10:00:00Z".into(),
            by: Some("s/one".into()),
            via: "alm".into(),
            command: "add task".into(),
            kind: Kind::Task,
            id: Some("A01-30".into()),
            agent: None,
            tool: None,
            changes,
        }
    }

    #[test]
    fn a_number_is_found_with_its_input() {
        let input = fingerprint("add task A01 \"Sign-in\"");
        assert_eq!(input, fingerprint("add task A01 \"Sign-in\""));
        assert_ne!(input, fingerprint("add task A01 \"Sign-up\""));
        let marked = event(vec![change("n-1", &input)]);
        let plain = event(vec![]);
        assert_eq!(of(&marked), Some(("n-1", input.as_str())));
        assert_eq!(of(&plain), None);
        let events = [plain.clone(), marked.clone()];
        assert_eq!(made_under(&events, "n-1"), vec![&marked]);
        assert!(made_under(&events, "n-2").is_empty());
        assert_eq!(summary(&[&marked])[0]["id"], "A01-30");
    }
}
