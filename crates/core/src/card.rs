//! Card files kept beside their tasks (Task-A00-30, approver 2026-10-16).
//!
//! Some projects keep a task's detail in a card file of its own. In Allomos a card file lives in
//! the taskbag, beside the task's files, as `tasks/<id>/card.md`, and the `Card:` link in the
//! task's description points to it, so a task is kept in one place.
//!
//! Neither `alm` nor the app looks for card files. An AI agent, told by Allomos's instructions,
//! finds the ones a project still keeps in its own folder and reports them
//! (`alm cards found <id>=<path>`); the report lives only in the history, as a `cards found` event
//! whose change `card_outside` has the path as its `to`. Moving a card in (`alm card <id>`)
//! writes `card.md`, points the description's link to it, and records one `card` event that
//! empties `card_outside`. A task's card still outside is therefore worked out from the events
//! alone, and the taskbag's format does not change: older Allomos versions read only `task.json`
//! and `description.md` in a task's folder, and accept any field name in an event's changes.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::event::{self, Change, Event, Kind};
use crate::ops::{Outcome, check_holder, find_task};
use crate::{Problem, Taskbag};

/// The card file's name inside its task's folder.
pub const FILE_NAME: &str = "card.md";
/// The field a `cards found` event (and the `card` event that moves it in) changes: where the
/// task's card file is in the project folder.
pub const OUTSIDE: &str = "card_outside";
/// The field a `card` event sets: the card file in the task's folder.
pub const CARD: &str = "card";
/// The command that records card files found in the project folder.
pub const FOUND: &str = "cards found";
/// The command that moves a card file in.
pub const MOVED: &str = "card";

/// A card file an agent reported as still in the project folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Outside {
    pub task: String,
    /// Its place, relative to the project folder, with `/`.
    pub path: String,
    /// When it was reported, in UTC.
    pub at: String,
    /// Who reported it (`--as`), when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
}

/// A card file's place as reported: relative to the project folder, written with `/`, never
/// leaving it. Refused (`card_path_invalid`) when empty, absolute, holding a scheme or drive
/// (`:`), or stepping up with `..`.
pub fn clean_path(path: &str) -> Result<String, Problem> {
    let invalid = || Problem::CardPathInvalid {
        path: path.to_string(),
    };
    let unified = path.trim().replace('\\', "/");
    if unified.is_empty() || unified.starts_with('/') || unified.contains(':') {
        return Err(invalid());
    }
    let mut parts = Vec::new();
    for part in unified.split('/') {
        match part {
            "" | "." => {}
            ".." => return Err(invalid()),
            other => parts.push(other),
        }
    }
    if parts.is_empty() {
        return Err(invalid());
    }
    Ok(parts.join("/"))
}

/// The change that reports task `id`'s card file at `path` in the project folder; `before` is
/// what was reported for it until now. An empty `path` withdraws the report. Refused for an
/// unknown task and a bad path; no change when nothing differs. Any task, in any state, held by
/// anyone, may be reported: the report changes nothing in the task.
pub fn report(
    taskbag: &Taskbag,
    id: &str,
    path: &str,
    before: Option<&str>,
) -> Result<Vec<Change>, Problem> {
    find_task(taskbag, id)?;
    let to = if path.trim().is_empty() {
        None
    } else {
        Some(clean_path(path)?)
    };
    if to.as_deref() == before {
        return Ok(Vec::new());
    }
    Ok(vec![Change {
        field: OUTSIDE.to_string(),
        from: before.map(|b| Value::String(b.to_string())),
        to: to.map(Value::String),
    }])
}

/// Every task's card file still reported as outside the taskbag, by task ID, from `events`
/// (oldest first): the last change to `card_outside` wins, and one without a `to` clears it.
pub fn outside(events: &[Event]) -> BTreeMap<String, Outside> {
    let mut out = BTreeMap::new();
    for e in events {
        let (Kind::Task, Some(id)) = (e.kind, e.id.as_deref()) else {
            continue;
        };
        for c in e.changes.iter().filter(|c| c.field == OUTSIDE) {
            match &c.to {
                Some(Value::String(path)) => {
                    out.insert(
                        id.to_string(),
                        Outside {
                            task: id.to_string(),
                            path: path.clone(),
                            at: e.at.clone(),
                            by: e.by.clone(),
                        },
                    );
                }
                _ => {
                    out.remove(id);
                }
            }
        }
    }
    out
}

/// `description` with its `Card:` link pointing to the card file beside it: the first link on
/// the first line starting `Card:` is replaced (the rest of that line stays); without such a
/// line, one is added at the top.
pub fn link_card(description: &str) -> String {
    let link = format!("[{FILE_NAME}]({FILE_NAME})");
    let mut lines: Vec<String> = description.lines().map(String::from).collect();
    let mut found = false;
    for line in lines.iter_mut() {
        let indent = line.len() - line.trim_start().len();
        let Some(rest) = line[indent..].strip_prefix("Card:") else {
            continue;
        };
        let rest = match replace_first_link(rest, &link) {
            Some(r) => r.trim_start().to_string(),
            None => link.clone(),
        };
        *line = format!("{}Card: {rest}", &line[..indent]);
        found = true;
        break;
    }
    let text = if found {
        lines.join("\n")
    } else if description.trim().is_empty() {
        format!("Card: {link}")
    } else {
        format!("Card: {link}\n\n{description}")
    };
    format!("{}\n", text.trim_end())
}

/// `text` with its first Markdown link `[…](…)` replaced by `link`, or `None` when it has none.
fn replace_first_link(text: &str, link: &str) -> Option<String> {
    let open = text.find('[')?;
    let middle = open + text[open..].find("](")?;
    let close = middle + 2 + text[middle + 2..].find(')')?;
    Some(format!("{}{link}{}", &text[..open], &text[close + 1..]))
}

/// Move task `id`'s card file in: its description's `Card:` link points to `card.md` beside it
/// ([`link_card`]), and the changes to record are the description's, the report's (`reported`,
/// emptied) and `card`. Same holder rule as other changes (a finished or dropped task counts as
/// held by nobody).
pub fn move_in(
    taskbag: &Taskbag,
    id: &str,
    me: Option<&str>,
    reported: Option<&str>,
) -> Result<(Outcome, Vec<Change>), Problem> {
    let before = find_task(taskbag, id)?;
    check_holder(&before, me)?;
    let mut task = before.clone();
    task.description = link_card(&before.description);
    let mut changes = event::changes(Some(&before), &task, event::TASK_FIELDS);
    if let Some(path) = reported {
        changes.push(Change {
            field: OUTSIDE.to_string(),
            from: Some(Value::String(path.to_string())),
            to: None,
        });
    }
    changes.push(Change {
        field: CARD.to_string(),
        from: None,
        to: Some(Value::String(FILE_NAME.to_string())),
    });
    Ok((Outcome::plain(task), changes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Area, Project, State, Task};

    fn task(id: &str, state: State, holder: Option<&str>, description: &str) -> Task {
        Task {
            id: id.into(),
            title: id.into(),
            area: "A01".into(),
            state,
            label: None,
            holder: holder.map(String::from),
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

    fn bag(tasks: Vec<Task>) -> Taskbag {
        Taskbag {
            project: serde_json::from_value::<Project>(serde_json::json!({
                "format": crate::FORMAT, "id": "P", "name": "P", "labels": ["implemented", "human verified"]
            }))
            .unwrap(),
            areas: vec![Area {
                id: "A01".into(),
                name: "A".into(),
                order: 0,
            }],
            iterations: Vec::new(),
            tasks,
        }
    }

    fn event(command: &str, id: &str, changes: Vec<Change>) -> Event {
        Event {
            at: "2026-10-16T10:00:00Z".into(),
            by: Some("claude/x".into()),
            via: "alm".into(),
            command: command.into(),
            kind: Kind::Task,
            id: Some(id.into()),
            changes,
        }
    }

    #[test]
    fn paths_stay_inside_the_project_folder() {
        assert_eq!(
            clean_path("docs\\tasks/./A01-01.md").unwrap(),
            "docs/tasks/A01-01.md"
        );
        for bad in [
            "",
            "  ",
            "/etc/x.md",
            "../x.md",
            "docs/../../x.md",
            "C:/x.md",
            ".",
        ] {
            assert!(
                matches!(clean_path(bad), Err(Problem::CardPathInvalid { .. })),
                "{bad}"
            );
        }
    }

    #[test]
    fn reports_and_withdrawals_are_worked_out_from_the_history() {
        let b = bag(vec![task("A01-01", State::Done, Some("old"), "")]);
        let found = report(&b, "A01-01", "docs/tasks/A01-01.md", None).unwrap();
        assert_eq!(found[0].field, OUTSIDE);
        // The same report again changes nothing.
        assert!(
            report(
                &b,
                "A01-01",
                "docs/tasks/A01-01.md",
                Some("docs/tasks/A01-01.md")
            )
            .unwrap()
            .is_empty()
        );
        assert!(matches!(
            report(&b, "A09-09", "x.md", None),
            Err(Problem::TaskNotFound { .. })
        ));
        let mut events = vec![event(FOUND, "A01-01", found)];
        assert_eq!(outside(&events)["A01-01"].path, "docs/tasks/A01-01.md");
        let withdrawn = report(&b, "A01-01", "", Some("docs/tasks/A01-01.md")).unwrap();
        events.push(event(FOUND, "A01-01", withdrawn));
        assert!(outside(&events).is_empty());
    }

    #[test]
    fn the_card_link_points_beside_the_task() {
        assert_eq!(
            link_card("Card: [docs/tasks/A.md](../../../app/docs/tasks/A.md#x) (moved)\n\nMore.\n"),
            "Card: [card.md](card.md) (moved)\n\nMore.\n"
        );
        assert_eq!(link_card(""), "Card: [card.md](card.md)\n");
        assert_eq!(
            link_card("**Outcome.** Done.\n"),
            "Card: [card.md](card.md)\n\n**Outcome.** Done.\n"
        );
        assert_eq!(link_card("Card: none yet\n"), "Card: [card.md](card.md)\n");
        // Already pointing there: unchanged.
        assert_eq!(
            link_card("Card: [card.md](card.md)\n"),
            "Card: [card.md](card.md)\n"
        );
    }

    #[test]
    fn a_finished_task_held_by_an_old_session_can_get_its_card() {
        let b = bag(vec![
            task(
                "A01-01",
                State::Done,
                Some("old/session"),
                "Card: [x](../x.md)\n",
            ),
            task("A01-02", State::InProgress, Some("other"), ""),
        ]);
        let (o, changes) = move_in(&b, "A01-01", Some("me"), Some("docs/x.md")).unwrap();
        assert_eq!(o.task.description, "Card: [card.md](card.md)\n");
        let fields: Vec<&str> = changes.iter().map(|c| c.field.as_str()).collect();
        assert_eq!(fields, ["description", OUTSIDE, CARD]);
        assert!(matches!(
            move_in(&b, "A01-02", Some("me"), None),
            Err(Problem::TaskHeld { .. })
        ));
    }
}
