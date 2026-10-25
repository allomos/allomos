//! Which Git branch a task starts from and merges into (Task-A03-22, agreed with the approver
//! 2026-10-16).
//!
//! Every task records its **base branch** by its real name (`main`, `master`, `storage-v2`, any
//! name): the task starts from it and merges back into it. There is no "trunk" setting and no
//! empty-means-main. A claimed task also records its **working branch**, the branch the work is
//! done on (the base itself when the agent works on it directly).
//!
//! Allomos never runs or reads Git (decision 0017): the agent passes the branch it is on
//! (`--branch`). The branches live only in the task's history, as event changes named
//! `base_branch` and `working_branch` (like `ai_decision`, Task-A03-19), so `task.json` gains no
//! field, older Allomos builds read the events like any other, and the format stays `allomos/4`.
//! A task's branches are the last value each field was given.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::event::{Change, Event, Kind};
use crate::ops::{check_holder, check_open, find_task};
use crate::{Problem, State, Taskbag};

/// The field that records a task's base branch.
pub const BASE: &str = "base_branch";
/// The field that records a task's working branch.
pub const WORKING: &str = "working_branch";
/// The command that changes a task's base (`alm base`).
pub const SET_BASE: &str = "base";

/// A task's branches, as its history last recorded them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Branches {
    /// The branch the task starts from and merges into.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    /// The branch the work is done on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working: Option<String>,
}

/// Every task's branches, by task ID, from `events` (oldest first). The last change of each field
/// wins; one with only a `from` empties it. Tasks with neither are left out.
pub fn recorded(events: &[Event]) -> BTreeMap<String, Branches> {
    let mut out: BTreeMap<String, Branches> = BTreeMap::new();
    for e in events {
        let (Kind::Task, Some(id)) = (e.kind, e.id.as_deref()) else {
            continue;
        };
        for c in &e.changes {
            let value = c.to.as_ref().map(|v| match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            });
            match c.field.as_str() {
                BASE => out.entry(id.to_string()).or_default().base = value,
                WORKING => out.entry(id.to_string()).or_default().working = value,
                _ => {}
            }
        }
    }
    out.retain(|_, b| b.base.is_some() || b.working.is_some());
    out
}

/// A branch name as given, trimmed. Refused (`branch_invalid`) when empty, with a space or a
/// control character, or starting with `-`: Git refuses those too. Anything else is the agent's
/// to judge, since alm reads no Git.
pub fn branch_name(name: &str) -> Result<String, Problem> {
    let name = name.trim();
    let bad = name.is_empty()
        || name.starts_with('-')
        || name.chars().any(|c| c.is_whitespace() || c.is_control());
    if bad {
        Err(Problem::BranchInvalid {
            name: name.to_string(),
        })
    } else {
        Ok(name.to_string())
    }
}

/// Why a task got its base.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "why", rename_all = "snake_case")]
pub enum Why {
    /// `--base` named it (or the person picked it in the app).
    Given,
    /// The branch the agent is on is `task`'s working branch, so the new task shares its base.
    WorkingBranchOf { task: String, branch: String },
    /// The branch the agent is on is no task's working branch, so it is the base itself.
    CurrentBranch { branch: String },
    /// At claim: the task had no base, so it gets the branch the agent is on.
    NoneBefore { branch: String },
    /// At claim: the task already had this base.
    Recorded,
}

impl Why {
    /// In plain words, for `alm`.
    pub fn english(&self, base: &str) -> String {
        match self {
            Why::Given => "given with --base".to_string(),
            Why::WorkingBranchOf { task, branch } => format!(
                "{branch}, the branch you are on, is the working branch of {task}, which starts from and merges into {base}"
            ),
            Why::CurrentBranch { branch } => {
                format!("{branch} is the branch you are on, and it is no task's working branch")
            }
            Why::NoneBefore { branch } => {
                format!("the task had no base, so it gets {branch}, the branch you are on")
            }
            Why::Recorded => "recorded earlier for this task".to_string(),
        }
    }
}

/// The base chosen for a task, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Choice {
    pub base: String,
    #[serde(flatten)]
    pub why: Why,
}

impl Choice {
    /// `main (why)`, for `alm`.
    pub fn english(&self) -> String {
        format!("{} ({})", self.base, self.why.english(&self.base))
    }
}

fn open(state: State) -> bool {
    !matches!(state, State::Done | State::Dropped)
}

/// The base for a new task (`alm add task`, the app's New task form). `given` (`--base`, or the
/// person's pick) wins. Otherwise, with `current` (the branch the agent is on, `--branch`): when
/// it is a task's working branch, that task's base (an open task first, then by ID); else
/// `current` itself. Neither: no base (`None`). A name given is checked ([`branch_name`]).
pub fn choose_base(
    taskbag: &Taskbag,
    recorded: &BTreeMap<String, Branches>,
    current: Option<&str>,
    given: Option<&str>,
) -> Result<Option<Choice>, Problem> {
    if let Some(given) = given {
        return Ok(Some(Choice {
            base: branch_name(given)?,
            why: Why::Given,
        }));
    }
    let Some(current) = current else {
        return Ok(None);
    };
    let current = branch_name(current)?;
    let mut sharing: Vec<(&str, bool, &str)> = taskbag
        .tasks
        .iter()
        .filter_map(|t| {
            let b = recorded.get(&t.id)?;
            (b.working.as_deref() == Some(current.as_str()))
                .then_some(())
                .and(b.base.as_deref())
                .map(|base| (t.id.as_str(), open(t.state), base))
        })
        .collect();
    // Open tasks first, then by ID.
    sharing.sort_by_key(|(id, open, _)| (!*open, *id));
    Ok(Some(match sharing.first() {
        Some((task, _, base)) => Choice {
            base: base.to_string(),
            why: Why::WorkingBranchOf {
                task: task.to_string(),
                branch: current.clone(),
            },
        },
        None => Choice {
            base: current.clone(),
            why: Why::CurrentBranch { branch: current },
        },
    }))
}

/// Warnings (`base_differs`) for each open task `me` holds whose base is not `base`, other than
/// `except` (the new task): the agent decides whether the new task should share that base.
pub fn other_bases(
    taskbag: &Taskbag,
    recorded: &BTreeMap<String, Branches>,
    me: Option<&str>,
    id: &str,
    base: &str,
) -> Vec<Problem> {
    let Some(me) = me else {
        return Vec::new();
    };
    taskbag
        .tasks
        .iter()
        .filter(|t| t.id != id && open(t.state) && t.holder.as_deref() == Some(me))
        .filter_map(|t| {
            let held_base = recorded.get(&t.id)?.base.as_deref()?;
            (held_base != base).then(|| Problem::BaseDiffers {
                held: t.id.clone(),
                held_base: held_base.to_string(),
                base: base.to_string(),
            })
        })
        .collect()
}

/// The change that sets `field` from `before` to `after`, or none when it stays the same.
pub fn change(field: &str, before: Option<&str>, after: &str) -> Option<Change> {
    (before != Some(after)).then(|| Change {
        extra: Default::default(),
        field: field.to_string(),
        from: before.map(|b| Value::String(b.to_string())),
        to: Some(Value::String(after.to_string())),
    })
}

/// What a claim records about branches.
#[derive(Debug, Clone, PartialEq)]
pub struct AtClaim {
    /// The task's branches after the claim.
    pub branches: Branches,
    /// Why it has its base, when it has one.
    pub why: Option<Why>,
    /// The branch the agent is on, as given.
    pub current: Option<String>,
    /// The event changes to record (none when nothing is new).
    pub changes: Vec<Change>,
    /// `branch_missing` when the agent did not say which branch it is on and something is unknown.
    pub warnings: Vec<Problem>,
}

/// The branches a claim of task `id` records. `current` is the branch the agent is on
/// (`--branch`); `working` the branch the work will be done on (`--working`, default `current`).
/// A task without a base gets `current` (approver, 2026-10-16). Without `current` nothing new is
/// recorded for what is still unknown, and `branch_missing` says so.
pub fn at_claim(
    id: &str,
    before: &Branches,
    current: Option<&str>,
    working: Option<&str>,
) -> Result<AtClaim, Problem> {
    let current = current.map(branch_name).transpose()?;
    let working = working.map(branch_name).transpose()?.or(current.clone());
    let mut after = before.clone();
    let mut changes = Vec::new();
    let mut warnings = Vec::new();
    let why = match (&before.base, &current) {
        (Some(_), _) => Some(Why::Recorded),
        (None, Some(cur)) => {
            after.base = Some(cur.clone());
            changes.extend(change(BASE, None, cur));
            Some(Why::NoneBefore {
                branch: cur.clone(),
            })
        }
        (None, None) => None,
    };
    match &working {
        Some(w) => {
            changes.extend(change(WORKING, before.working.as_deref(), w));
            after.working = Some(w.clone());
        }
        None => warnings.push(Problem::BranchMissing { id: id.to_string() }),
    }
    Ok(AtClaim {
        branches: after,
        why,
        current,
        changes,
        warnings,
    })
}

/// The change that sets task `id`'s base to `name` (`alm base`). Refused for an unknown task, one
/// that is done or dropped, one someone else holds, and a name that is not a branch name. No
/// change when it already has that base.
pub fn set_base(
    taskbag: &Taskbag,
    recorded: &BTreeMap<String, Branches>,
    id: &str,
    me: Option<&str>,
    name: &str,
) -> Result<Vec<Change>, Problem> {
    let task = find_task(taskbag, id)?;
    check_open(&task, &[State::Done, State::Dropped])?;
    check_holder(&task, me)?;
    let name = branch_name(name)?;
    let before = recorded.get(id).and_then(|b| b.base.as_deref());
    Ok(change(BASE, before, &name).into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Area, Project, Task};

    fn task(id: &str, state: State, holder: Option<&str>) -> Task {
        Task {
            extra: Default::default(),
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
            description: String::new(),
        }
    }

    fn bag(tasks: Vec<Task>) -> Taskbag {
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
            tasks,
        }
    }

    fn event(id: &str, changes: Vec<Change>) -> Event {
        Event {
            extra: Default::default(),
            at: "2026-10-16T10:00:00Z".into(),
            by: Some("claude/x".into()),
            via: "alm".into(),
            command: "claim".into(),
            kind: Kind::Task,
            id: Some(id.into()),
            agent: None,
            tool: None,
            changes,
        }
    }

    fn branches(base: &str, working: &str) -> Branches {
        Branches {
            base: Some(base.into()),
            working: Some(working.into()),
        }
    }

    #[test]
    fn the_last_recorded_branch_wins() {
        let events = vec![
            event("A01-01", change(BASE, None, "main").into_iter().collect()),
            event(
                "A01-01",
                vec![
                    change(WORKING, None, "claude/a").unwrap(),
                    Change {
                        extra: Default::default(),
                        field: "state".into(),
                        from: None,
                        to: Some("in_progress".into()),
                    },
                ],
            ),
            event(
                "A01-01",
                change(BASE, Some("main"), "storage-v2")
                    .into_iter()
                    .collect(),
            ),
            event("A01-02", vec![]),
        ];
        let r = recorded(&events);
        assert_eq!(r["A01-01"], branches("storage-v2", "claude/a"));
        assert!(!r.contains_key("A01-02"));
        // A change with only a `from` empties the field.
        let mut more = events.clone();
        more.push(event(
            "A01-01",
            vec![Change {
                extra: Default::default(),
                field: WORKING.into(),
                from: Some("claude/a".into()),
                to: None,
            }],
        ));
        assert_eq!(recorded(&more)["A01-01"].working, None);
    }

    #[test]
    fn a_branch_name_is_checked() {
        assert_eq!(branch_name(" storage-v2 ").unwrap(), "storage-v2");
        assert_eq!(branch_name("claude/a03-22").unwrap(), "claude/a03-22");
        for bad in ["", "  ", "my branch", "-x", "a\tb"] {
            assert!(
                matches!(branch_name(bad), Err(Problem::BranchInvalid { .. })),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn a_new_task_follows_the_branch_the_agent_is_on() {
        let bag = bag(vec![
            task("A01-01", State::InProgress, Some("claude/a")),
            task("A01-02", State::Done, None),
        ]);
        let mut r = BTreeMap::new();
        r.insert("A01-01".to_string(), branches("storage-v2", "claude/a"));
        r.insert("A01-02".to_string(), branches("main", "claude/old"));

        // On a task's working branch: that task's base.
        let c = choose_base(&bag, &r, Some("claude/a"), None)
            .unwrap()
            .unwrap();
        assert_eq!(c.base, "storage-v2");
        assert_eq!(
            c.why,
            Why::WorkingBranchOf {
                task: "A01-01".into(),
                branch: "claude/a".into()
            }
        );
        assert!(c.english().contains("working branch of A01-01"));

        // A done task's working branch counts too, when no open task shares it.
        let c = choose_base(&bag, &r, Some("claude/old"), None)
            .unwrap()
            .unwrap();
        assert_eq!(c.base, "main");

        // On any other branch: that branch itself, by its real name.
        let c = choose_base(&bag, &r, Some("master"), None)
            .unwrap()
            .unwrap();
        assert_eq!(
            (c.base.as_str(), &c.why),
            (
                "master",
                &Why::CurrentBranch {
                    branch: "master".into()
                }
            )
        );

        // --base overrides; nothing given: no base.
        let c = choose_base(&bag, &r, Some("claude/a"), Some("main"))
            .unwrap()
            .unwrap();
        assert_eq!((c.base.as_str(), &c.why), ("main", &Why::Given));
        assert_eq!(choose_base(&bag, &r, None, None).unwrap(), None);
        assert!(matches!(
            choose_base(&bag, &r, Some("a b"), None),
            Err(Problem::BranchInvalid { .. })
        ));
    }

    #[test]
    fn an_open_task_wins_over_a_done_one_on_the_same_working_branch() {
        let bag = bag(vec![
            task("A01-01", State::Done, None),
            task("A01-02", State::Ready, None),
        ]);
        let mut r = BTreeMap::new();
        r.insert("A01-01".to_string(), branches("main", "shared"));
        r.insert("A01-02".to_string(), branches("storage-v2", "shared"));
        let c = choose_base(&bag, &r, Some("shared"), None)
            .unwrap()
            .unwrap();
        assert_eq!(c.base, "storage-v2");
    }

    #[test]
    fn the_adding_session_is_told_when_it_holds_a_task_on_another_base() {
        let bag = bag(vec![
            task("A01-01", State::InProgress, Some("claude/a")),
            task("A01-02", State::Done, Some("claude/a")),
            task("A01-03", State::InProgress, Some("claude/b")),
            task("A01-04", State::Backlog, None),
        ]);
        let mut r = BTreeMap::new();
        r.insert("A01-01".to_string(), branches("storage-v2", "claude/a"));
        r.insert("A01-02".to_string(), branches("old", "claude/a"));
        r.insert("A01-03".to_string(), branches("other", "claude/b"));
        let w = other_bases(&bag, &r, Some("claude/a"), "A01-04", "main");
        assert_eq!(
            w,
            [Problem::BaseDiffers {
                held: "A01-01".into(),
                held_base: "storage-v2".into(),
                base: "main".into()
            }]
        );
        assert!(other_bases(&bag, &r, Some("claude/a"), "A01-04", "storage-v2").is_empty());
        assert!(other_bases(&bag, &r, None, "A01-04", "main").is_empty());
    }

    #[test]
    fn a_claim_records_the_working_branch_and_a_missing_base() {
        // No base yet: the branch the agent is on becomes it, and the working branch.
        let c = at_claim("A01-01", &Branches::default(), Some("main"), None).unwrap();
        assert_eq!(c.branches, branches("main", "main"));
        assert_eq!(
            c.why,
            Some(Why::NoneBefore {
                branch: "main".into()
            })
        );
        let fields: Vec<&str> = c.changes.iter().map(|c| c.field.as_str()).collect();
        assert_eq!(fields, [BASE, WORKING]);
        assert!(c.warnings.is_empty());

        // A base already: kept; --working names the branch the work is done on.
        let before = Branches {
            base: Some("storage-v2".into()),
            working: None,
        };
        let c = at_claim("A01-01", &before, Some("main"), Some("claude/a")).unwrap();
        assert_eq!(c.branches, branches("storage-v2", "claude/a"));
        assert_eq!(c.why, Some(Why::Recorded));
        assert_eq!(c.changes.len(), 1);

        // Claimed again on the same branch: nothing new.
        let c = at_claim(
            "A01-01",
            &branches("storage-v2", "claude/a"),
            Some("claude/a"),
            None,
        )
        .unwrap();
        assert!(c.changes.is_empty());

        // Not told: nothing recorded, and a warning.
        let c = at_claim("A01-01", &Branches::default(), None, None).unwrap();
        assert!(c.changes.is_empty());
        assert_eq!(c.why, None);
        assert_eq!(
            c.warnings,
            [Problem::BranchMissing {
                id: "A01-01".into()
            }]
        );
        assert!(at_claim("A01-01", &Branches::default(), Some("a b"), None).is_err());
    }

    #[test]
    fn the_base_is_changed_by_its_holder_on_an_open_task() {
        let bag = bag(vec![
            task("A01-01", State::InProgress, Some("claude/a")),
            task("A01-02", State::Done, None),
        ]);
        let mut r = BTreeMap::new();
        r.insert("A01-01".to_string(), branches("main", "claude/a"));
        let c = set_base(&bag, &r, "A01-01", Some("claude/a"), "storage-v2").unwrap();
        assert_eq!(
            c,
            change(BASE, Some("main"), "storage-v2")
                .into_iter()
                .collect::<Vec<_>>()
        );
        assert!(
            set_base(&bag, &r, "A01-01", Some("claude/a"), "main")
                .unwrap()
                .is_empty()
        );
        assert!(matches!(
            set_base(&bag, &r, "A01-01", Some("claude/b"), "x"),
            Err(Problem::TaskHeld { .. })
        ));
        assert!(matches!(
            set_base(&bag, &r, "A01-02", None, "x"),
            Err(Problem::TaskClosed { .. })
        ));
    }
}
