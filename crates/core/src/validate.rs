use std::collections::HashSet;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::model::{Area, FORMAT, Project, State, Task, TaskBox};

/// One problem found in a task box, tied to the file (and field, when known) that caused it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Issue {
    /// Path of the file, relative to the task box root.
    pub file: String,
    pub field: Option<String>,
    /// Plain-language explanation of what is wrong and how to fix it.
    pub message: String,
}

impl Issue {
    fn new(file: &str, field: Option<&str>, message: impl Into<String>) -> Self {
        Issue {
            file: file.to_string(),
            field: field.map(String::from),
            message: message.into(),
        }
    }
}

/// The result of loading: what could be read, plus every problem found.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Loaded {
    pub task_box: Option<TaskBox>,
    pub issues: Vec<Issue>,
}

/// Fields whose value must be one of a fixed list: (field, allowed values).
const CHOICE_FIELDS: &[(&str, &[&str])] = &[(
    "state",
    &[
        "backlog",
        "ready",
        "in_progress",
        "waiting",
        "done",
        "dropped",
    ],
)];

fn parse<T: DeserializeOwned>(file: &str, text: &str) -> Result<T, Issue> {
    // Check fixed-choice fields first, so the problem names the field (e.g. `state`)
    // rather than the bad value.
    if let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(text) {
        for (field, allowed) in CHOICE_FIELDS {
            if let Some(value) = map.get(*field) {
                let ok = value.as_str().is_some_and(|v| allowed.contains(&v));
                if !ok {
                    return Err(Issue {
                        file: file.to_string(),
                        field: Some((*field).to_string()),
                        message: format!(
                            "{value} is not a valid {field}. Use one of: {}.",
                            allowed.join(", ")
                        ),
                    });
                }
            }
        }
    }
    serde_json::from_str(text).map_err(|e| {
        let msg = e.to_string();
        // Only "missing field `x`" and "unknown field `x`" name a field; other messages
        // (for example a wrong value type) do not, so no field is guessed for them.
        let field = (msg.starts_with("missing field") || msg.starts_with("unknown field"))
            .then(|| msg.split('`').nth(1).map(String::from))
            .flatten();
        Issue {
            file: file.to_string(),
            field,
            message: format!("This file could not be read as JSON for glidebox: {msg}."),
        }
    })
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
            format!("\"{id}\" is not a valid ID. Use 1–64 letters, digits, '-' or '_'."),
        ));
    }
}

pub fn parse_project(file: &str, text: &str) -> Result<Project, Vec<Issue>> {
    let project: Project = parse(file, text).map_err(|i| vec![i])?;
    let mut issues = Vec::new();
    if project.format != FORMAT {
        issues.push(Issue::new(
            file,
            Some("format"),
            format!("Expected \"{FORMAT}\" but found \"{}\". This version of glidebox reads only \"{FORMAT}\".", project.format),
        ));
    }
    check_id(file, "id", &project.id, &mut issues);
    if project.name.trim().is_empty() {
        issues.push(Issue::new(file, Some("name"), "The project needs a name."));
    }
    if project.labels.is_empty() {
        issues.push(Issue::new(
            file,
            Some("labels"),
            "The label ladder needs at least one step.",
        ));
    }
    if issues.is_empty() {
        Ok(project)
    } else {
        Err(issues)
    }
}

pub fn parse_area(file: &str, text: &str) -> Result<Area, Vec<Issue>> {
    let area: Area = parse(file, text).map_err(|i| vec![i])?;
    let mut issues = Vec::new();
    check_id(file, "id", &area.id, &mut issues);
    if area.name.trim().is_empty() {
        issues.push(Issue::new(file, Some("name"), "The area needs a name."));
    }
    if issues.is_empty() {
        Ok(area)
    } else {
        Err(issues)
    }
}

pub fn parse_task(file: &str, text: &str) -> Result<Task, Vec<Issue>> {
    let task: Task = parse(file, text).map_err(|i| vec![i])?;
    let mut issues = Vec::new();
    check_id(file, "id", &task.id, &mut issues);
    if task.title.trim().is_empty() {
        issues.push(Issue::new(file, Some("title"), "The task needs a title."));
    }
    if issues.is_empty() {
        Ok(task)
    } else {
        Err(issues)
    }
}

/// Cross-file checks: unique IDs, every task's area exists, labels are on the project's ladder,
/// and a waiting task says what it waits on.
pub fn validate_task_box(
    project: &Project,
    areas: &[(String, Area)],
    tasks: &[(String, Task)],
) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut area_ids = HashSet::new();
    for (file, area) in areas {
        if !area_ids.insert(area.id.as_str()) {
            issues.push(Issue::new(
                file,
                Some("id"),
                format!("Another area already uses the ID \"{}\".", area.id),
            ));
        }
    }
    let mut task_ids = HashSet::new();
    for (file, task) in tasks {
        if !task_ids.insert(task.id.as_str()) {
            issues.push(Issue::new(
                file,
                Some("id"),
                format!("Another task already uses the ID \"{}\".", task.id),
            ));
        }
        if !area_ids.contains(task.area.as_str()) {
            issues.push(Issue::new(
                file,
                Some("area"),
                format!(
                    "The area \"{}\" does not exist. Add areas/{}.json or use an existing area ID.",
                    task.area, task.area
                ),
            ));
        }
        if let Some(label) = &task.label {
            if !project.labels.contains(label) {
                issues.push(Issue::new(
                    file,
                    Some("label"),
                    format!(
                        "\"{label}\" is not a step of this project's ladder ({}).",
                        project.labels.join(" → ")
                    ),
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
                "A waiting task must say what it is waiting on.",
            ));
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROJECT: &str = r#"{"format":"glidebox/1","id":"demo","name":"Demo"}"#;

    fn project() -> Project {
        parse_project("project.json", PROJECT).unwrap()
    }

    #[test]
    fn project_gets_default_ladder() {
        assert_eq!(project().labels.len(), 4);
    }

    #[test]
    fn wrong_format_is_reported_on_its_field() {
        let err = parse_project(
            "project.json",
            r#"{"format":"glidebox/9","id":"d","name":"D"}"#,
        )
        .unwrap_err();
        assert_eq!(err[0].field.as_deref(), Some("format"));
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
    }

    #[test]
    fn unknown_state_names_the_state_field() {
        let err = parse_task(
            "t.json",
            r#"{"id":"T1","title":"x","area":"A1","state":"finished"}"#,
        )
        .unwrap_err();
        assert_eq!(err[0].field.as_deref(), Some("state"));
        assert!(err[0].message.contains("\"finished\" is not a valid state"));
        assert!(err[0].message.contains("in_progress"));
    }

    #[test]
    fn wrong_type_does_not_guess_a_field() {
        let err = parse_area("a.json", r#"{"id":"A1","name":"X","order":"first"}"#).unwrap_err();
        assert_eq!(err[0].field, None);
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
        let issues = validate_task_box(&project(), &areas, &tasks);
        let fields: Vec<_> = issues
            .iter()
            .map(|i| (i.file.as_str(), i.field.as_deref().unwrap()))
            .collect();
        assert!(fields.contains(&("tasks/T1/task.json", "area")));
        assert!(fields.contains(&("tasks/T2/task.json", "id")));
        assert!(fields.contains(&("tasks/T2/task.json", "label")));
        assert!(fields.contains(&("tasks/T2/task.json", "waiting_on")));
    }

    #[test]
    fn valid_box_has_no_issues() {
        let areas = vec![(
            "a".to_string(),
            parse_area("a", r#"{"id":"A1","name":"Tracking"}"#).unwrap(),
        )];
        let tasks = vec![("t".to_string(), parse_task("t", r#"{"id":"T1","title":"x","area":"A1","state":"in_progress","label":"implemented"}"#).unwrap())];
        assert!(validate_task_box(&project(), &areas, &tasks).is_empty());
        let _ = TaskBox {
            project: project(),
            areas: vec![],
            tasks: vec![],
        };
    }
}
