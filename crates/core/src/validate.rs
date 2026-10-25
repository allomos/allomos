use std::collections::HashSet;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::model::{
    Area, FORMAT, FORMAT_VERSION, Iteration, Project, State, Task, Taskbag, format_version,
};
use crate::problem::Problem;

/// One problem found in a taskbag, tied to the file (and field, when known) that caused it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Issue {
    /// Path of the file, relative to the taskbag root.
    pub file: String,
    pub field: Option<String>,
    /// What is wrong, as a code the interface words in the chosen language.
    pub problem: Problem,
}

impl Issue {
    pub fn new(file: &str, field: Option<&str>, problem: Problem) -> Self {
        Issue {
            file: file.to_string(),
            field: field.map(String::from),
            problem,
        }
    }
}

/// The result of loading: what could be read, plus every problem found.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Loaded {
    pub taskbag: Option<Taskbag>,
    pub issues: Vec<Issue>,
    /// The active iteration's ID, as `ops::active_iteration` finds it, so the app shows the same
    /// one `alm` uses (Task-A01-root343).
    pub active_iteration: Option<String>,
}

/// Fields whose value is one of a fixed list. A string this Allomos does not know is read as the
/// field's `Unknown` and kept in `extra` (Task-A01-piano403); only a value that is not a string at
/// all is refused.
const CHOICE_FIELDS: &[&str] = &["state"];

fn parse<T: DeserializeOwned>(file: &str, text: &str) -> Result<T, Issue> {
    parse_with(file, text, CHOICE_FIELDS)
}

fn parse_with<T: DeserializeOwned>(file: &str, text: &str, choices: &[&str]) -> Result<T, Issue> {
    // Check fixed-choice fields first, so the problem names the field (e.g. `state`)
    // rather than the bad value.
    if let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(text) {
        for field in choices {
            if let Some(value) = map.get(*field).filter(|v| !v.is_string()) {
                return Err(Issue::new(
                    file,
                    Some(field),
                    Problem::InvalidChoice {
                        value: value.to_string(),
                    },
                ));
            }
        }
    }
    serde_json::from_str(text).map_err(|e| {
        let msg = e.to_string();
        // Only "missing field `x`" names a field; other messages (for example a wrong value type)
        // do not, so no field is guessed for them.
        match (msg.starts_with("missing field"), msg.split('`').nth(1)) {
            (true, Some(f)) => Issue::new(file, Some(f), Problem::FieldMissing),
            _ => Issue::new(file, None, Problem::NotJson { detail: msg }),
        }
    })
}

/// The value of one top-level field of a JSON file, as written.
fn raw_field(text: &str, field: &str) -> Option<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()?
        .get(field)
        .cloned()
}

/// Reminders for what an item's file holds that this Allomos does not know (Task-A01-piano403):
/// each unknown field, and each value kept for a fixed-choice field this Allomos does not know
/// (`kept`: the names of those fields). `prefix` names where the item sits in its file, for
/// example `handoff.`.
pub fn unknown_notes(
    file: &str,
    prefix: &str,
    extra: &crate::model::Extra,
    kept: &[&str],
    issues: &mut Vec<Issue>,
) {
    for (name, value) in extra {
        let field = format!("{prefix}{name}");
        let problem = match value.as_str() {
            Some(v) if kept.contains(&name.as_str()) => Problem::ValueUnknown {
                value: v.to_string(),
            },
            _ => Problem::FieldUnknown,
        };
        issues.push(Issue::new(file, Some(&field), problem));
    }
}

fn check_id(file: &str, field: &str, id: &str, issues: &mut Vec<Issue>) {
    let ok = !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !ok {
        issues.push(Issue::new(
            file,
            Some(field),
            Problem::InvalidId { id: id.to_string() },
        ));
    }
}

pub fn parse_project(file: &str, text: &str) -> Result<Project, Vec<Issue>> {
    // The format first: a newer taskbag may have fields this Allomos does not know (Task-A01-25),
    // and an older one is not read at all: an AI agent moves its tasks into a new taskbag, like
    // another task system (owner, 2026-10-15; Task-A01-45).
    if let Some(found) = serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|v| v.get("format").and_then(|f| f.as_str()).map(String::from))
    {
        match format_version(&found) {
            Some(n) if n > FORMAT_VERSION => {
                return Err(vec![Issue::new(
                    file,
                    Some("format"),
                    Problem::TaskbagNewer {
                        found,
                        supported: FORMAT.to_string(),
                    },
                )]);
            }
            Some(n) if n < crate::model::READ_FROM => {
                return Err(vec![Issue::new(
                    file,
                    Some("format"),
                    Problem::TaskbagToMoveIn {
                        found,
                        current: FORMAT.to_string(),
                    },
                )]);
            }
            _ => {}
        }
    }
    let project: Project = parse(file, text).map_err(|i| vec![i])?;
    let mut issues = Vec::new();
    if format_version(&project.format).is_none() {
        issues.push(Issue::new(
            file,
            Some("format"),
            Problem::WrongFormat {
                expected: FORMAT.to_string(),
                found: project.format.clone(),
            },
        ));
    }
    check_id(file, "id", &project.id, &mut issues);
    if project.name.trim().is_empty() {
        issues.push(Issue::new(file, Some("name"), Problem::ProjectNameMissing));
    }
    if project.labels.is_empty() {
        issues.push(Issue::new(file, Some("labels"), Problem::LadderEmpty));
    }
    if let Some(rules) = &project.handoff {
        let mut gates = vec![
            ("handoff.quota_gate".to_string(), rules.quota_gate),
            ("handoff.context_gate".to_string(), rules.context_gate),
        ];
        for (name, g) in &rules.overrides {
            gates.push((format!("handoff.overrides.{name}.quota_gate"), g.quota_gate));
            gates.push((
                format!("handoff.overrides.{name}.context_gate"),
                g.context_gate,
            ));
        }
        for (field, value) in gates {
            if let Some(v) = value.filter(|v| *v > 100) {
                issues.push(Issue::new(
                    file,
                    Some(&field),
                    Problem::InvalidPercent {
                        value: v.to_string(),
                    },
                ));
            }
        }
    }
    if let Some(value) = &project.task_language {
        if value != crate::model::TASK_LANGUAGE_UI && !crate::model::valid_language(value) {
            issues.push(Issue::new(
                file,
                Some("task_language"),
                Problem::InvalidLanguage {
                    value: value.clone(),
                },
            ));
        }
    }
    if issues.is_empty() {
        Ok(project)
    } else {
        Err(issues)
    }
}

/// One event file (`events/<name>.json`, Task-A01-17). Unknown fields, and a `kind` this Allomos
/// does not know, are kept in `extra` (Task-A01-piano403); [`event_notes`] lists them.
pub fn parse_event(file: &str, text: &str) -> Result<crate::event::Event, Issue> {
    let mut event: crate::event::Event = parse_with(file, text, &["kind"])?;
    if event.kind.is_unknown() {
        if let Some(v) = raw_field(text, "kind") {
            event.extra.insert("kind".into(), v);
        }
    }
    Ok(event)
}

/// The reminder for one event: a `kind` this Allomos does not know. Its unknown fields are kept but
/// not listed (approver, 2026-10-19; Task-A01-55): events are written only by `alm` and the app.
pub fn event_notes(file: &str, event: &crate::event::Event) -> Vec<Issue> {
    let mut issues = Vec::new();
    if let Some(kind) = event.extra.get("kind").filter(|_| event.kind.is_unknown()) {
        let kind = crate::model::Extra::from([("kind".to_string(), kind.clone())]);
        unknown_notes(file, "", &kind, &["kind"], &mut issues);
    }
    issues
}

pub fn parse_area(file: &str, text: &str) -> Result<Area, Vec<Issue>> {
    let area: Area = parse(file, text).map_err(|i| vec![i])?;
    let mut issues = Vec::new();
    check_id(file, "id", &area.id, &mut issues);
    if area.name.trim().is_empty() {
        issues.push(Issue::new(file, Some("name"), Problem::AreaNameMissing));
    }
    if issues.is_empty() {
        Ok(area)
    } else {
        Err(issues)
    }
}

/// A calendar date written `YYYY-MM-DD`, for example `2026-10-08`.
pub fn valid_date(text: &str) -> bool {
    let b = text.as_bytes();
    let digits = |r: std::ops::Range<usize>| b[r].iter().all(u8::is_ascii_digit);
    if b.len() != 10
        || b[4] != b'-'
        || b[7] != b'-'
        || !digits(0..4)
        || !digits(5..7)
        || !digits(8..10)
    {
        return false;
    }
    let num = |r: std::ops::Range<usize>| text[r].parse::<u32>().unwrap_or(0);
    let (year, month, day) = (num(0..4), num(5..7), num(8..10));
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days).contains(&day)
}

pub fn parse_iteration(file: &str, text: &str) -> Result<Iteration, Vec<Issue>> {
    let mut iteration: Iteration = parse(file, text).map_err(|i| vec![i])?;
    if iteration.state == crate::IterationState::Unknown {
        if let Some(v) = raw_field(text, "state") {
            iteration.extra.insert("state".into(), v);
        }
    }
    let mut issues = Vec::new();
    check_id(file, "id", &iteration.id, &mut issues);
    if iteration.name.trim().is_empty() {
        issues.push(Issue::new(
            file,
            Some("name"),
            Problem::IterationNameMissing,
        ));
    }
    for (field, value) in [("start", &iteration.start), ("end", &iteration.end)] {
        if let Some(v) = value.as_deref().filter(|v| !valid_date(v)) {
            issues.push(Issue::new(
                file,
                Some(field),
                Problem::InvalidDate {
                    value: v.to_string(),
                },
            ));
        }
    }
    if issues.is_empty() {
        Ok(iteration)
    } else {
        Err(issues)
    }
}

pub fn parse_task(file: &str, text: &str) -> Result<Task, Vec<Issue>> {
    let mut task: Task = parse(file, text).map_err(|i| vec![i])?;
    if task.state.is_unknown() {
        if let Some(v) = raw_field(text, "state") {
            task.extra.insert("state".into(), v);
        }
    }
    let mut issues = Vec::new();
    check_id(file, "id", &task.id, &mut issues);
    if task.title.trim().is_empty() {
        issues.push(Issue::new(file, Some("title"), Problem::TaskTitleMissing));
    }
    if task
        .without_human_check
        .as_ref()
        .is_some_and(|t| t.trim().is_empty())
    {
        issues.push(Issue::new(
            file,
            Some("without_human_check"),
            Problem::WithoutHumanCheckMissing,
        ));
    }
    if issues.is_empty() {
        Ok(task)
    } else {
        Err(issues)
    }
}

/// Cross-file checks: unique IDs, every task's area (and iteration, when it has one) exists,
/// labels are on the project's ladder, a waiting task says what it waits on, and a blocked task
/// names at least one other task of this taskbag that blocks it (decision 0016).
pub fn validate_taskbag(
    project: &Project,
    areas: &[(String, Area)],
    iterations: &[(String, Iteration)],
    tasks: &[(String, Task)],
) -> Vec<Issue> {
    let mut issues = Vec::new();
    // Reminders, not errors: what this Allomos does not know is shown and kept (Task-A01-piano403).
    unknown_notes("project.json", "", &project.extra, &[], &mut issues);
    if let Some(rules) = &project.handoff {
        unknown_notes("project.json", "handoff.", &rules.extra, &[], &mut issues);
        for (name, g) in &rules.overrides {
            let prefix = format!("handoff.overrides.{name}.");
            unknown_notes("project.json", &prefix, &g.extra, &[], &mut issues);
        }
    }
    for (file, area) in areas {
        unknown_notes(file, "", &area.extra, &[], &mut issues);
    }
    for (file, iteration) in iterations {
        let kept: &[&str] = if iteration.state == crate::IterationState::Unknown {
            &["state"]
        } else {
            &[]
        };
        unknown_notes(file, "", &iteration.extra, kept, &mut issues);
    }
    for (file, task) in tasks {
        let kept: &[&str] = if task.state.is_unknown() {
            &["state"]
        } else {
            &[]
        };
        unknown_notes(file, "", &task.extra, kept, &mut issues);
    }
    let mut area_ids = HashSet::new();
    for (file, area) in areas {
        if !area_ids.insert(area.id.as_str()) {
            issues.push(Issue::new(
                file,
                Some("id"),
                Problem::DuplicateAreaId {
                    id: area.id.clone(),
                },
            ));
        }
    }
    let mut iteration_ids = HashSet::new();
    let mut active: Option<&str> = None;
    for (file, iteration) in iterations {
        if !iteration_ids.insert(iteration.id.as_str()) {
            issues.push(Issue::new(
                file,
                Some("id"),
                Problem::DuplicateIterationId {
                    id: iteration.id.clone(),
                },
            ));
        }
        // At most one active iteration (Task-A01-root343): a hand edit may leave two.
        if iteration.state == crate::IterationState::Active {
            match active {
                Some(first) => issues.push(Issue::new(
                    file,
                    Some("state"),
                    Problem::SecondActiveIteration {
                        id: iteration.id.clone(),
                        active: first.to_string(),
                    },
                )),
                None => active = Some(iteration.id.as_str()),
            }
        }
    }
    // Project folders (decision 0019): valid, unique names; every task's folders among them.
    let mut folder_names = HashSet::new();
    for f in &project.project_folders {
        if !crate::model::valid_folder_name(&f.name) {
            issues.push(Issue::new(
                "project.json",
                Some("project_folders"),
                Problem::FolderNameInvalid {
                    name: f.name.clone(),
                },
            ));
        } else if !folder_names.insert(f.name.as_str()) {
            issues.push(Issue::new(
                "project.json",
                Some("project_folders"),
                Problem::FolderNameTaken {
                    name: f.name.clone(),
                },
            ));
        }
    }
    let mut task_ids = HashSet::new();
    for (file, task) in tasks {
        for folder in task
            .folders
            .iter()
            .filter(|f| !folder_names.contains(f.as_str()))
        {
            issues.push(Issue::new(
                file,
                Some("folders"),
                Problem::FolderUnknown {
                    folder: folder.clone(),
                    folders: crate::folders::names(project),
                },
            ));
        }
        if !task_ids.insert(task.id.as_str()) {
            issues.push(Issue::new(
                file,
                Some("id"),
                Problem::DuplicateTaskId {
                    id: task.id.clone(),
                },
            ));
        }
        if !area_ids.contains(task.area.as_str()) {
            issues.push(Issue::new(
                file,
                Some("area"),
                Problem::UnknownArea {
                    area: task.area.clone(),
                },
            ));
        }
        if let Some(iteration) = &task.iteration {
            if !iteration_ids.contains(iteration.as_str()) {
                issues.push(Issue::new(
                    file,
                    Some("iteration"),
                    Problem::UnknownIteration {
                        iteration: iteration.clone(),
                    },
                ));
            }
        }
        if let Some(label) = &task.label {
            if !project.labels.contains(label) {
                issues.push(Issue::new(
                    file,
                    Some("label"),
                    Problem::LabelNotOnLadder {
                        label: label.clone(),
                        ladder: project.labels.join(" → "),
                    },
                ));
            }
        }
        if task.state == State::Waiting
            && task
                .waiting_on
                .as_deref()
                .is_none_or(|w| w.trim().is_empty())
        {
            issues.push(Issue::new(
                file,
                Some("waiting_on"),
                Problem::WaitingOnMissing,
            ));
        }
        if task.state == State::Blocked && task.blocked_by.is_empty() {
            issues.push(Issue::new(
                file,
                Some("blocked_by"),
                Problem::BlockedByMissing,
            ));
        }
        for b in &task.blocked_by {
            let problem = if b == &task.id {
                Problem::BlockedBySelf { id: b.clone() }
            } else if !tasks.iter().any(|(_, t)| &t.id == b) {
                Problem::BlockerUnknown { id: b.clone() }
            } else {
                continue;
            };
            issues.push(Issue::new(file, Some("blocked_by"), problem));
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROJECT: &str = r#"{"format":"allomos/5","id":"demo","name":"Demo"}"#;

    fn project() -> Project {
        parse_project("project.json", PROJECT).unwrap()
    }

    #[test]
    fn project_gets_default_ladder() {
        assert_eq!(project().labels.len(), 4);
    }

    #[test]
    fn a_newer_format_is_reported_before_unknown_fields() {
        let err = parse_project(
            "project.json",
            r#"{"format":"allomos/7","id":"d","name":"D","later":{"context":90}}"#,
        )
        .unwrap_err();
        assert_eq!(err.len(), 1);
        assert_eq!(err[0].field.as_deref(), Some("format"));
        assert_eq!(err[0].problem.code(), "taskbag_newer");
    }

    #[test]
    fn handoff_gates_must_be_percentages() {
        let ok = parse_project(
            "project.json",
            r#"{"format":"allomos/5","id":"d","name":"D","handoff":{"quota_gate":5,"overrides":{"codex":{"context_gate":80}}}}"#,
        )
        .unwrap();
        let rules = ok.handoff.unwrap();
        assert_eq!(rules.applied(None).quota_gate, 5);
        assert_eq!(rules.applied(None).context_gate, 90);
        assert_eq!(rules.applied(Some("codex/a01-03")).context_gate, 80);
        assert_eq!(
            rules.applied(Some("codex/a01-03")).override_for.as_deref(),
            Some("codex")
        );
        assert_eq!(rules.applied(Some("codexy")).context_gate, 90);
        let err = parse_project(
            "project.json",
            r#"{"format":"allomos/5","id":"d","name":"D","handoff":{"overrides":{"codex":{"quota_gate":150}}}}"#,
        )
        .unwrap_err();
        assert_eq!(err[0].problem.code(), "invalid_percent");
        assert_eq!(
            err[0].field.as_deref(),
            Some("handoff.overrides.codex.quota_gate")
        );
    }

    #[test]
    fn format_numbers() {
        assert_eq!(format_version("allomos/1"), Some(1));
        assert_eq!(format_version("allomos/12"), Some(12));
        for other in [
            "allomos/",
            "allomos/01",
            "allomos/x",
            "fliphopper/1",
            "allomos/0",
        ] {
            assert_eq!(format_version(other), None, "{other}");
        }
    }

    #[test]
    fn wrong_format_is_reported_on_its_field() {
        let err = parse_project(
            "project.json",
            r#"{"format":"glidebox/1","id":"d","name":"D"}"#,
        )
        .unwrap_err();
        assert_eq!(err[0].field.as_deref(), Some("format"));
        assert_eq!(err[0].problem.code(), "wrong_format");
    }

    /// Task-A01-45: every format below this Allomos's is to be moved in, none is read; a newer one
    /// is refused too.
    #[test]
    fn an_older_or_newer_format_is_not_read() {
        // `allomos/5` stays readable until a person upgrades it (Task-A01-dime494).
        for n in 1..crate::model::READ_FROM {
            let text = format!(r#"{{"format":"allomos/{n}","id":"d","name":"D","old_field":1}}"#);
            let err = parse_project("project.json", &text).unwrap_err();
            assert_eq!(err.len(), 1);
            assert_eq!(err[0].field.as_deref(), Some("format"));
            assert_eq!(
                err[0].problem,
                Problem::TaskbagToMoveIn {
                    found: format!("allomos/{n}"),
                    current: FORMAT.to_string(),
                }
            );
        }
        let newer = format!(
            r#"{{"format":"allomos/{}","id":"d","name":"D"}}"#,
            FORMAT_VERSION + 1
        );
        let err = parse_project("project.json", &newer).unwrap_err();
        assert_eq!(err[0].problem.code(), "taskbag_newer");
    }

    #[test]
    fn missing_field_names_the_field() {
        let err = parse_task(
            "tasks/T1/task.json",
            r#"{"id":"T1","area":"A1","state":"ready"}"#,
        )
        .unwrap_err();
        assert_eq!(err[0].file, "tasks/T1/task.json");
        assert_eq!(err[0].field.as_deref(), Some("title"));
        assert_eq!(err[0].problem, Problem::FieldMissing);
    }

    #[test]
    fn an_unknown_state_is_kept_and_noted() {
        let text = r#"{"id":"T1","title":"x","area":"A1","state":"finished","colour":"red"}"#;
        let task = parse_task("t.json", text).unwrap();
        assert_eq!(task.state, State::Unknown);
        assert_eq!(task.state_text(), "finished");
        let mut notes = Vec::new();
        unknown_notes("t.json", "", &task.extra, &["state"], &mut notes);
        let found: Vec<_> = notes
            .iter()
            .map(|i| (i.field.as_deref().unwrap(), i.problem.code()))
            .collect();
        assert_eq!(
            found,
            [("colour", "field_unknown"), ("state", "value_unknown")]
        );
        // Written back as it was read.
        let back = serde_json::to_value(&task).unwrap();
        assert_eq!(back["state"], "finished");
        assert_eq!(back["colour"], "red");
    }

    #[test]
    fn a_state_that_is_not_a_word_is_refused() {
        let err =
            parse_task("t.json", r#"{"id":"T1","title":"x","area":"A1","state":3}"#).unwrap_err();
        assert_eq!(err[0].field.as_deref(), Some("state"));
        assert_eq!(err[0].problem.code(), "invalid_choice");
    }

    #[test]
    fn unknown_fields_are_kept_in_every_file() {
        let p = parse_project(
            "project.json",
            r#"{"format":"allomos/5","id":"d","name":"D","later":1,"handoff":{"soon":true,"overrides":{"codex":{"new":2}}}}"#,
        )
        .unwrap();
        let back = serde_json::to_value(&p).unwrap();
        assert_eq!(back["later"], 1);
        assert_eq!(back["handoff"]["soon"], true);
        assert_eq!(back["handoff"]["overrides"]["codex"]["new"], 2);
        let a = parse_area("a.json", r#"{"id":"A1","name":"X","icon":"star"}"#).unwrap();
        assert_eq!(serde_json::to_value(&a).unwrap()["icon"], "star");
        let i = parse_iteration("i.json", r#"{"id":"I1","name":"X","state":"paused"}"#).unwrap();
        assert_eq!(i.state, crate::IterationState::Unknown);
        assert_eq!(serde_json::to_value(&i).unwrap()["state"], "paused");
        let issues = validate_taskbag(&p, &[("a.json".into(), a)], &[("i.json".into(), i)], &[]);
        let fields: Vec<_> = issues.iter().filter_map(|i| i.field.as_deref()).collect();
        assert_eq!(
            fields,
            [
                "later",
                "handoff.soon",
                "handoff.overrides.codex.new",
                "icon",
                "state"
            ]
        );
    }

    #[test]
    fn wrong_type_does_not_guess_a_field() {
        let err = parse_area("a.json", r#"{"id":"A1","name":"X","order":"first"}"#).unwrap_err();
        assert_eq!(err[0].field, None);
        assert_eq!(err[0].problem.code(), "not_json");
    }

    #[test]
    fn unknown_field_names_the_field() {
        let area = parse_area("a.json", r#"{"id":"A1","name":"X","colour":"red"}"#).unwrap();
        let mut notes = Vec::new();
        unknown_notes("a.json", "", &area.extra, &[], &mut notes);
        assert_eq!(notes[0].field.as_deref(), Some("colour"));
        assert_eq!(notes[0].problem, Problem::FieldUnknown);
    }

    #[test]
    fn a_task_finished_without_the_human_check_carries_its_note_and_steps() {
        let t = parse_task(
            "t.json",
            r#"{"id":"T1","title":"x","area":"A1","state":"done","without_human_check":"card run","human_check_steps":"1. Open it."}"#,
        )
        .unwrap();
        assert_eq!(t.without_human_check.as_deref(), Some("card run"));
        assert_eq!(t.human_check_steps.as_deref(), Some("1. Open it."));
        assert!(t.to_check());
        let err = parse_task(
            "t.json",
            r#"{"id":"T1","title":"x","area":"A1","state":"done","without_human_check":" "}"#,
        )
        .unwrap_err();
        assert_eq!(err[0].field.as_deref(), Some("without_human_check"));
        assert_eq!(err[0].problem, Problem::WithoutHumanCheckMissing);
        // The name before allomos/4 is not read as the note any more (Task-A01-45); since
        // Task-A01-piano403 it is kept as a field this Allomos does not know.
        let old = parse_task(
            "t.json",
            r#"{"id":"T1","title":"x","area":"A1","state":"done","finished_without_owner":"card run"}"#,
        )
        .unwrap();
        assert!(!old.to_check());
        assert!(old.extra.contains_key("finished_without_owner"));
    }

    #[test]
    fn bad_id_is_rejected() {
        let err = parse_area("areas/x.json", r#"{"id":"has space","name":"X"}"#).unwrap_err();
        assert_eq!(err[0].field.as_deref(), Some("id"));
    }

    #[test]
    fn cross_file_rules() {
        let areas = vec![(
            "areas/A1.json".to_string(),
            parse_area("a", r#"{"id":"A1","name":"Tracking"}"#).unwrap(),
        )];
        let tasks = vec![
            (
                "tasks/T1/task.json".to_string(),
                parse_task(
                    "t",
                    r#"{"id":"T1","title":"x","area":"A9","state":"ready"}"#,
                )
                .unwrap(),
            ),
            (
                "tasks/T2/task.json".to_string(),
                parse_task(
                    "t",
                    r#"{"id":"T1","title":"y","area":"A1","state":"waiting","label":"shipped"}"#,
                )
                .unwrap(),
            ),
        ];
        let issues = validate_taskbag(&project(), &areas, &[], &tasks);
        let fields: Vec<_> = issues
            .iter()
            .map(|i| (i.file.as_str(), i.field.as_deref().unwrap()))
            .collect();
        assert!(fields.contains(&("tasks/T1/task.json", "area")));
        assert!(fields.contains(&("tasks/T2/task.json", "id")));
        assert!(fields.contains(&("tasks/T2/task.json", "label")));
        assert!(fields.contains(&("tasks/T2/task.json", "waiting_on")));
    }

    /// Decision 0016: a blocked task names at least one other task of this taskbag.
    #[test]
    fn a_blocked_task_names_known_other_tasks() {
        let areas = vec![(
            "a".to_string(),
            parse_area("a", r#"{"id":"A1","name":"Tracking"}"#).unwrap(),
        )];
        let task = |file: &str, json: &str| (file.to_string(), parse_task("t", json).unwrap());
        let tasks = vec![
            task(
                "tasks/T1/task.json",
                r#"{"id":"T1","title":"x","area":"A1","state":"blocked"}"#,
            ),
            task(
                "tasks/T2/task.json",
                r#"{"id":"T2","title":"y","area":"A1","state":"blocked","blocked_by":["T2","T9"]}"#,
            ),
            task(
                "tasks/T3/task.json",
                r#"{"id":"T3","title":"z","area":"A1","state":"blocked","blocked_by":["T1"]}"#,
            ),
            task(
                "tasks/T4/task.json",
                r#"{"id":"T4","title":"w","area":"A1","state":"waiting","waiting_on":"approval","who":"Ann"}"#,
            ),
        ];
        let issues = validate_taskbag(&project(), &areas, &[], &tasks);
        let got: Vec<(&str, &str)> = issues
            .iter()
            .map(|i| (i.file.as_str(), i.problem.code()))
            .collect();
        assert_eq!(
            got,
            vec![
                ("tasks/T1/task.json", "blocked_by_missing"),
                ("tasks/T2/task.json", "blocked_by_self"),
                ("tasks/T2/task.json", "blocker_unknown"),
            ]
        );
        assert!(
            issues
                .iter()
                .all(|i| i.field.as_deref() == Some("blocked_by"))
        );
    }

    #[test]
    fn valid_box_has_no_issues() {
        let areas = vec![(
            "a".to_string(),
            parse_area("a", r#"{"id":"A1","name":"Tracking"}"#).unwrap(),
        )];
        let tasks = vec![("t".to_string(), parse_task("t", r#"{"id":"T1","title":"x","area":"A1","state":"in_progress","label":"implemented"}"#).unwrap())];
        assert!(validate_taskbag(&project(), &areas, &[], &tasks).is_empty());
        let _ = Taskbag {
            project: project(),
            areas: vec![],
            iterations: vec![],
            tasks: vec![],
        };
    }

    #[test]
    fn dates() {
        for good in ["2026-10-08", "2024-02-29", "2000-02-29", "2026-12-31"] {
            assert!(valid_date(good), "{good}");
        }
        for bad in [
            "2026-1-08",
            "2026-13-01",
            "2026-02-29",
            "1900-02-29",
            "2026-04-31",
            "2026/10/08",
            "today",
            "2026-10-08x",
        ] {
            assert!(!valid_date(bad), "{bad}");
        }
    }

    #[test]
    fn iterations() {
        let it = parse_iteration(
            "iterations/I02.json",
            r#"{"id":"I02","name":"v0.2","order":2,"release":"v0.2","start":"2026-10-08"}"#,
        )
        .unwrap();
        assert_eq!((it.release.as_deref(), it.end), (Some("v0.2"), None));
        let err = parse_iteration("i", r#"{"id":"I1","name":" ","end":"soon"}"#).unwrap_err();
        let problems: Vec<_> = err
            .iter()
            .map(|i| (i.field.as_deref(), i.problem.code()))
            .collect();
        assert_eq!(
            problems,
            vec![
                (Some("name"), "iteration_name_missing"),
                (Some("end"), "invalid_date")
            ]
        );

        let areas = vec![(
            "a".to_string(),
            parse_area("a", r#"{"id":"A1","name":"T"}"#).unwrap(),
        )];
        // The state (Task-A01-root343): missing is planned; a field it does not know is read past.
        let st = |text: &str| parse_iteration("i", text).map(|i| i.state);
        assert_eq!(
            st(r#"{"id":"I1","name":"One"}"#),
            Ok(crate::IterationState::Planned)
        );
        assert_eq!(
            st(r#"{"id":"I1","name":"One","state":"active"}"#),
            Ok(crate::IterationState::Active)
        );
        assert_eq!(
            st(r#"{"id":"I1","name":"One","state":"closed","later":1}"#),
            Ok(crate::IterationState::Closed)
        );
        assert_eq!(
            st(r#"{"id":"I1","name":"One","state":"doing"}"#),
            Ok(crate::IterationState::Unknown)
        );
        assert_eq!(
            serde_json::to_string(
                &parse_iteration("i", r#"{"id":"I1","name":"One","state":"planned"}"#).unwrap()
            )
            .unwrap(),
            r#"{"id":"I1","name":"One","order":0}"#
        );

        let iterations = vec![
            (
                "iterations/I1.json".to_string(),
                parse_iteration("i", r#"{"id":"I1","name":"One"}"#).unwrap(),
            ),
            (
                "iterations/x.json".to_string(),
                parse_iteration("i", r#"{"id":"I1","name":"Again"}"#).unwrap(),
            ),
        ];
        let tasks = vec![
            (
                "tasks/T1/task.json".to_string(),
                parse_task(
                    "t",
                    r#"{"id":"T1","title":"x","area":"A1","state":"ready","iteration":"I1"}"#,
                )
                .unwrap(),
            ),
            (
                "tasks/T2/task.json".to_string(),
                parse_task(
                    "t",
                    r#"{"id":"T2","title":"y","area":"A1","state":"ready","iteration":"I9"}"#,
                )
                .unwrap(),
            ),
            (
                "tasks/T3/task.json".to_string(),
                parse_task(
                    "t",
                    r#"{"id":"T3","title":"z","area":"A1","state":"ready"}"#,
                )
                .unwrap(),
            ),
        ];
        let issues = validate_taskbag(&project(), &areas, &iterations, &tasks);
        let found: Vec<_> = issues
            .iter()
            .map(|i| (i.file.as_str(), i.problem.code()))
            .collect();
        assert_eq!(
            found,
            vec![
                ("iterations/x.json", "duplicate_iteration_id"),
                ("tasks/T2/task.json", "unknown_iteration"),
            ]
        );
    }
}
