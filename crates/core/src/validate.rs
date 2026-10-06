use std::collections::HashSet;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::model::{Area, FORMAT, Project, Seabag, State, Task};
use crate::problem::Problem;

/// One problem found in a seabag, tied to the file (and field, when known) that caused it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Issue {
    /// Path of the file, relative to the seabag root.
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
    pub seabag: Option<Seabag>,
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
                    return Err(Issue::new(
                        file,
                        Some(field),
                        Problem::InvalidChoice {
                            value: value.to_string(),
                            allowed: allowed.join(", "),
                        },
                    ));
                }
            }
        }
    }
    serde_json::from_str(text).map_err(|e| {
        let msg = e.to_string();
        // Only "missing field `x`" and "unknown field `x`" name a field; other messages
        // (for example a wrong value type) do not, so no field is guessed for them.
        let named = msg.split('`').nth(1);
        match (
            msg.starts_with("missing field"),
            msg.starts_with("unknown field"),
            named,
        ) {
            (true, _, Some(f)) => Issue::new(file, Some(f), Problem::FieldMissing),
            (_, true, Some(f)) => Issue::new(file, Some(f), Problem::FieldUnknown),
            _ => Issue::new(file, None, Problem::NotJson { detail: msg }),
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
            Problem::InvalidId { id: id.to_string() },
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
        issues.push(Issue::new(file, Some("name"), Problem::AreaNameMissing));
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
        issues.push(Issue::new(file, Some("title"), Problem::TaskTitleMissing));
    }
    if issues.is_empty() {
        Ok(task)
    } else {
        Err(issues)
    }
}

/// Cross-file checks: unique IDs, every task's area exists, labels are on the project's ladder,
/// and a waiting task says what it waits on.
pub fn validate_seabag(
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
                Problem::DuplicateAreaId {
                    id: area.id.clone(),
                },
            ));
        }
    }
    let mut task_ids = HashSet::new();
    for (file, task) in tasks {
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
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROJECT: &str = r#"{"format":"fliphopper/1","id":"demo","name":"Demo"}"#;

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
            r#"{"format":"fliphopper/9","id":"d","name":"D"}"#,
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
        assert_eq!(err[0].problem, Problem::FieldMissing);
    }

    #[test]
    fn unknown_state_names_the_state_field() {
        let err = parse_task(
            "t.json",
            r#"{"id":"T1","title":"x","area":"A1","state":"finished"}"#,
        )
        .unwrap_err();
        assert_eq!(err[0].field.as_deref(), Some("state"));
        let Problem::InvalidChoice { value, allowed } = &err[0].problem else {
            panic!("expected invalid_choice, got {:?}", err[0].problem)
        };
        assert_eq!(value, "\"finished\"");
        assert!(allowed.contains("in_progress"));
    }

    #[test]
    fn wrong_type_does_not_guess_a_field() {
        let err = parse_area("a.json", r#"{"id":"A1","name":"X","order":"first"}"#).unwrap_err();
        assert_eq!(err[0].field, None);
        assert_eq!(err[0].problem.code(), "not_json");
    }

    #[test]
    fn unknown_field_names_the_field() {
        let err = parse_area("a.json", r#"{"id":"A1","name":"X","colour":"red"}"#).unwrap_err();
        assert_eq!(err[0].field.as_deref(), Some("colour"));
        assert_eq!(err[0].problem, Problem::FieldUnknown);
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
        let issues = validate_seabag(&project(), &areas, &tasks);
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
        assert!(validate_seabag(&project(), &areas, &tasks).is_empty());
        let _ = Seabag {
            project: project(),
            areas: vec![],
            tasks: vec![],
        };
    }
}
