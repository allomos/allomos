//! Labels (decision 0020, Task-A01-moth111): five built-in plain labels, a label table per
//! project, several labels per task, and how an older taskbag's ladder and labels are read.
//!
//! Every label is plain: no kind, no order, none excludes another. A task has a label or not.
//! "To check" is not a label: it is a done task with neither "by human" label
//! ([`crate::Task::to_check`]).

use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::State;
use crate::model::{LabelEntry, Project, Task, Taskbag};
use crate::ops::{Outcome, check_holder, check_open, find_task, non_empty};
use crate::problem::Problem;

pub const LOCALLY_BY_AGENT: &str = "locally verified by agent";
pub const DEPLOYED_BY_AGENT: &str = "deployed and verified by agent";
pub const LOCALLY_BY_HUMAN: &str = "locally verified by human";
pub const DEPLOYED_BY_HUMAN: &str = "deployed and verified by human";
pub const AI_DECISIONS: &str = "AI decisions to review";

/// The built-in labels with their meanings, in the order the table lists them.
pub const BUILT_IN: &[(&str, &str)] = &[
    (
        LOCALLY_BY_AGENT,
        "The agent built it and its checks passed on the computer where it worked.",
    ),
    (
        DEPLOYED_BY_AGENT,
        "The agent deployed it (with the approver's approval) and checked it live.",
    ),
    (
        LOCALLY_BY_HUMAN,
        "A person checked it by hand on their own computer (the human check).",
    ),
    (
        DEPLOYED_BY_HUMAN,
        "A person checked it by hand in the deployed app (the human check).",
    ),
    (
        AI_DECISIONS,
        "An agent decided things here that wait for review.",
    ),
];

/// Either one is the human check.
pub const BY_HUMAN: &[&str] = &[LOCALLY_BY_HUMAN, DEPLOYED_BY_HUMAN];

pub fn is_built_in(label: &str) -> bool {
    BUILT_IN.iter().any(|(n, _)| *n == label)
}

/// An older taskbag's ladder step or task label as `allomos/7` names it (decision 0020, In detail
/// 8): `None` for the dropped `drafted` and `implemented`; `human verified` (and the older
/// `owner verified`) becomes `deployed and verified by human` when `deployed_first`, else
/// `locally verified by human`. Any other step stays as a custom label.
pub fn upgraded_label(old: &str, deployed_first: bool) -> Option<String> {
    let old = old.trim();
    match old {
        "" | "drafted" | "implemented" => None,
        "locally verified" => Some(LOCALLY_BY_AGENT.into()),
        "deployed and verified" => Some(DEPLOYED_BY_AGENT.into()),
        "human verified" | "owner verified" if deployed_first => Some(DEPLOYED_BY_HUMAN.into()),
        "human verified" | "owner verified" => Some(LOCALLY_BY_HUMAN.into()),
        other => Some(other.to_string()),
    }
}

/// The table with every built-in label in it, first and in their own order, then the others as
/// they were. A built-in entry keeps a meaning the project wrote for it; one without a meaning
/// (for example an old ladder step the upgrade renamed into it) gets the built-in meaning.
pub fn with_built_ins(table: Vec<LabelEntry>) -> Vec<LabelEntry> {
    let mut out: Vec<LabelEntry> = BUILT_IN
        .iter()
        .map(|(name, meaning)| {
            let mut e = table
                .iter()
                .find(|e| e.name == *name)
                .cloned()
                .unwrap_or_else(|| LabelEntry::built_in(name, meaning));
            e.built_in = true;
            if e.meaning.as_deref().is_none_or(|m| m.trim().is_empty()) {
                e.meaning = Some(meaning.to_string());
            }
            e
        })
        .collect();
    for e in table {
        if !out.iter().any(|o| o.name == e.name) {
            out.push(LabelEntry {
                built_in: false,
                ..e
            });
        }
    }
    out
}

/// `project.json`'s `labels`: the `allomos/7` table, or an older taskbag's ladder of names, read
/// as the upgrade writes it. Either way the built-in labels are in it.
pub fn deserialize_table<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<LabelEntry>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Item {
        Old(String),
        Entry(LabelEntry),
    }
    let items = Vec::<Item>::deserialize(d)?;
    let table = items
        .into_iter()
        .filter_map(|i| match i {
            Item::Entry(e) => Some(e),
            Item::Old(name) => upgraded_label(&name, false).map(|n| LabelEntry::custom(&n, None)),
        })
        .collect();
    Ok(with_built_ins(table))
}

/// An older taskbag's task: its single `label` (kept among the unknown fields by the parser) read
/// into `labels` as the upgrade writes it, without the history (so `human verified` reads as
/// `locally verified by human`). Nothing changes for an `allomos/7` task.
pub fn read_old_label(task: &mut Task) {
    if let Some(Value::String(old)) = task.extra.remove("label") {
        if let Some(new) = upgraded_label(&old, false) {
            task.add_label(&new);
        }
    }
}

/// The labels of an older taskbag's task after the upgrade: its label renamed (or dropped), and
/// `AI decisions to review` when its history has unreviewed decisions (decision 0020, 8).
pub fn upgraded_labels(old: Option<&str>, deployed_first: bool, unreviewed: bool) -> Vec<String> {
    let mut labels: Vec<String> = old
        .and_then(|o| upgraded_label(o, deployed_first))
        .into_iter()
        .collect();
    if unreviewed {
        labels.push(AI_DECISIONS.into());
    }
    labels
}

/// The project's table with every label its tasks carry that it does not list yet, added at the
/// end (decision 0020, 5): the names added, in the order found.
pub fn add_found(project: &mut Project, labels: &[String]) -> Vec<String> {
    let mut added = Vec::new();
    for l in labels {
        if project.label(l).is_none() {
            project.labels.push(LabelEntry::custom(l, None));
            added.push(l.clone());
        }
    }
    added
}

/// What [`add`] did: the task, and the project when its table got a new label.
#[derive(Debug, Clone, PartialEq)]
pub struct Labelled {
    pub outcome: Outcome,
    /// The project with the new table entry, when the label was not in it yet.
    pub project: Option<Project>,
}

/// Add a label to task `id` (`alm label`), keeping its other labels (decision 0020, 3). A label
/// the table does not list is added to it (5); `alm` says so. A "by human" label on a done task
/// is the human check: its note and steps are cleared, as `alm checked` does.
pub fn add(
    taskbag: &Taskbag,
    id: &str,
    me: Option<&str>,
    label: &str,
) -> Result<Labelled, Problem> {
    let label = non_empty(Some(label)).ok_or(Problem::LabelNameMissing)?;
    let mut task = find_task(taskbag, id)?;
    check_holder(&task, me)?;
    check_open(&task, &[State::Dropped])?;
    task.add_label(&label);
    if task.state == State::Done && BY_HUMAN.contains(&label.as_str()) {
        task.without_human_check = None;
        task.human_check_steps = None;
    }
    let mut project = taskbag.project.clone();
    let project =
        (!add_found(&mut project, std::slice::from_ref(&label)).is_empty()).then_some(project);
    Ok(Labelled {
        outcome: Outcome::plain(task),
        project,
    })
}

/// Remove a label from task `id` on purpose (`alm label <id> --remove`). Refused when the task
/// does not carry it.
pub fn remove(
    taskbag: &Taskbag,
    id: &str,
    me: Option<&str>,
    label: &str,
) -> Result<Outcome, Problem> {
    let label = label.trim();
    let mut task = find_task(taskbag, id)?;
    check_holder(&task, me)?;
    check_open(&task, &[State::Dropped])?;
    if !task.remove_label(label) {
        return Err(Problem::LabelNotOnTask {
            id: task.id.clone(),
            label: label.to_string(),
        });
    }
    Ok(Outcome::plain(task))
}

/// The task's labels set to `labels`, as the app's Labels picker chose them (Task-A01-petal376):
/// each label left out is removed on purpose and each new one added, by the rules of
/// `alm label --remove` and `alm label` (a "by human" label added to a done task is the human
/// check: its note and steps are cleared). The labels it keeps stay in their order; new ones
/// follow in the order given. A label the table does not list is added there by the caller.
pub fn set(
    taskbag: &Taskbag,
    id: &str,
    me: Option<&str>,
    labels: &[String],
) -> Result<Outcome, Problem> {
    let want: Vec<String> = labels
        .iter()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    let mut task = find_task(taskbag, id)?;
    check_holder(&task, me)?;
    check_open(&task, &[State::Dropped])?;
    task.labels.retain(|l| want.contains(l));
    for l in &want {
        if task.add_label(l) && task.state == State::Done && BY_HUMAN.contains(&l.as_str()) {
            task.without_human_check = None;
            task.human_check_steps = None;
        }
    }
    Ok(Outcome::plain(task))
}

/// A label added to the table, or its meaning changed (`alm labels add`). A built-in label keeps
/// its name; only its meaning can change.
pub fn table_add(taskbag: &Taskbag, name: &str, meaning: Option<&str>) -> Result<Project, Problem> {
    let name = non_empty(Some(name)).ok_or(Problem::LabelNameMissing)?;
    let mut project = taskbag.project.clone();
    match project.labels.iter_mut().find(|e| e.name == name) {
        Some(e) => {
            if let Some(m) = meaning {
                e.meaning = non_empty(Some(m));
            }
        }
        None => project.labels.push(LabelEntry::custom(&name, meaning)),
    }
    Ok(project)
}

/// A label removed from the table on purpose (`alm labels remove`): refused for a built-in
/// label, and while a task still carries it (decision 0020, 6).
pub fn table_remove(taskbag: &Taskbag, name: &str) -> Result<Project, Problem> {
    let name = name.trim();
    if is_built_in(name) {
        return Err(Problem::LabelBuiltIn {
            label: name.to_string(),
        });
    }
    if taskbag.project.label(name).is_none() {
        return Err(Problem::LabelNotInTable {
            label: name.to_string(),
            labels: taskbag.project.label_names().join(", "),
        });
    }
    let tasks: Vec<&str> = taskbag
        .tasks
        .iter()
        .filter(|t| t.has_label(name))
        .map(|t| t.id.as_str())
        .collect();
    if !tasks.is_empty() {
        return Err(Problem::LabelInUse {
            label: name.to_string(),
            tasks: tasks.join(", "),
        });
    }
    let mut project = taskbag.project.clone();
    project.labels.retain(|e| e.name != name);
    Ok(project)
}

/// The table with its custom labels replaced by `names`, in that order, the built-in ones kept
/// first (the app's Tasks section). A custom label left out is removed, refused while a task
/// carries it.
pub fn set_custom(taskbag: &Taskbag, names: &[String]) -> Result<Project, Problem> {
    let names: Vec<String> = names
        .iter()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !is_built_in(l))
        .collect();
    let mut project = taskbag.project.clone();
    for e in taskbag.project.labels.iter().filter(|e| !e.built_in) {
        if !names.contains(&e.name) {
            project = table_remove(
                &Taskbag {
                    project: project.clone(),
                    ..taskbag.clone()
                },
                &e.name,
            )?;
        }
    }
    let built: Vec<LabelEntry> = project
        .labels
        .iter()
        .filter(|e| e.built_in)
        .cloned()
        .collect();
    let mut table = built;
    for n in &names {
        let e = project
            .label(n)
            .cloned()
            .unwrap_or_else(|| LabelEntry::custom(n, None));
        if !table.iter().any(|t| t.name == e.name) {
            table.push(e);
        }
    }
    project.labels = table;
    Ok(project)
}

/// A label renamed in the table and on every task that carries it (`alm rename label`,
/// Task-A01-32): the project and the tasks whose labels changed. A built-in label cannot be
/// renamed; the new name must not be in the table yet.
pub fn rename(taskbag: &Taskbag, old: &str, new: &str) -> Result<(Project, Vec<Task>), Problem> {
    let old = old.trim();
    let new = non_empty(Some(new)).ok_or(Problem::LabelNameMissing)?;
    if is_built_in(old) {
        return Err(Problem::LabelBuiltIn {
            label: old.to_string(),
        });
    }
    let mut project = taskbag.project.clone();
    let Some(at) = project.labels.iter().position(|e| e.name == old) else {
        return Err(Problem::LabelNotInTable {
            label: old.to_string(),
            labels: project.label_names().join(", "),
        });
    };
    if project.label(&new).is_some() {
        return Err(Problem::LabelExists { label: new });
    }
    project.labels[at].name = new.clone();
    let tasks = taskbag
        .tasks
        .iter()
        .filter(|t| t.has_label(old))
        .map(|t| {
            let mut t = t.clone();
            for l in t.labels.iter_mut().filter(|l| *l == old) {
                *l = new.clone();
            }
            t
        })
        .collect();
    Ok((project, tasks))
}

/// A person checked done task `id` (`alm checked`, decision 0020, In detail 2): it gets
/// `deployed and verified by human` when it carries `deployed and verified by agent`, else
/// `locally verified by human`; `which` picks one of the two instead. The note and steps are
/// cleared and stay in its history. The holder rule does not apply.
pub fn checked(taskbag: &Taskbag, id: &str, which: Option<&str>) -> Result<Outcome, Problem> {
    let mut task = find_task(taskbag, id)?;
    if task.state != State::Done {
        return Err(Problem::NotDone {
            id: task.id.clone(),
            state: task.state.as_str().to_string(),
        });
    }
    let label = match which {
        Some(l) if BY_HUMAN.contains(&l) => l,
        Some(l) => {
            return Err(Problem::LabelNotByHuman {
                label: l.to_string(),
            });
        }
        None if task.has_label(DEPLOYED_BY_AGENT) => DEPLOYED_BY_HUMAN,
        None => LOCALLY_BY_HUMAN,
    };
    task.add_label(label);
    task.without_human_check = None;
    task.human_check_steps = None;
    Ok(Outcome::plain(task))
}

#[cfg(test)]
mod tests {
    /// A built-in entry without a meaning (an old ladder step renamed by the upgrade) gets the
    /// built-in meaning; a meaning the project wrote stays.
    #[test]
    fn built_in_entries_get_their_meaning_unless_the_project_wrote_one() {
        let table = with_built_ins(vec![
            LabelEntry::custom(LOCALLY_BY_AGENT, None),
            LabelEntry::custom(LOCALLY_BY_HUMAN, Some("Our own words")),
        ]);
        for e in table.iter().filter(|e| e.built_in) {
            assert!(e.meaning.is_some(), "{}", e.name);
        }
        let find = |n: &str| table.iter().find(|e| e.name == n).unwrap();
        let built: Vec<&str> = BUILT_IN.iter().map(|(_, m)| *m).collect();
        assert_eq!(find(LOCALLY_BY_AGENT).meaning.as_deref(), Some(built[0]));
        assert_eq!(
            find(LOCALLY_BY_HUMAN).meaning.as_deref(),
            Some("Our own words")
        );
    }

    use super::*;

    #[test]
    fn old_steps_map_as_decided() {
        assert_eq!(upgraded_label("implemented", false), None);
        assert_eq!(upgraded_label("drafted", true), None);
        assert_eq!(
            upgraded_label("locally verified", false).unwrap(),
            LOCALLY_BY_AGENT
        );
        assert_eq!(
            upgraded_label("deployed and verified", false).unwrap(),
            DEPLOYED_BY_AGENT
        );
        assert_eq!(
            upgraded_label("human verified", false).unwrap(),
            LOCALLY_BY_HUMAN
        );
        assert_eq!(
            upgraded_label("human verified", true).unwrap(),
            DEPLOYED_BY_HUMAN
        );
        assert_eq!(
            upgraded_label("owner verified", true).unwrap(),
            DEPLOYED_BY_HUMAN
        );
        assert_eq!(upgraded_label("shipped", false).unwrap(), "shipped");
        assert_eq!(
            upgraded_labels(Some("implemented"), false, true),
            vec![AI_DECISIONS.to_string()]
        );
    }

    #[test]
    fn an_old_ladder_reads_as_a_table_with_the_built_ins() {
        let table: Vec<LabelEntry> = serde_json::from_str::<Wrap>(
            r#"{"labels":["drafted","implemented","locally verified","shipped","human verified"]}"#,
        )
        .unwrap()
        .labels;
        let names: Vec<&str> = table.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(
            names,
            [
                LOCALLY_BY_AGENT,
                DEPLOYED_BY_AGENT,
                LOCALLY_BY_HUMAN,
                DEPLOYED_BY_HUMAN,
                AI_DECISIONS,
                "shipped"
            ]
        );
        assert!(table[..5].iter().all(|e| e.built_in) && !table[5].built_in);
    }

    #[derive(Deserialize)]
    struct Wrap {
        #[serde(deserialize_with = "deserialize_table")]
        labels: Vec<LabelEntry>,
    }
}
